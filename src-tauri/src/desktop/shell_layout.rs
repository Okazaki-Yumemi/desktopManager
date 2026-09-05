//! Read/write desktop icon positions through the shell's SysListView32 via
//! LVM messages — the route verified in probe/shell_probe and
//! docs/WINDOWS_SHELL_PROBE.md (27/27 read/write/restore round-trip on this
//! machine, Windows 11 26200). Files on disk are never touched. Positions are
//! ListView client coordinates, stored verbatim: cross-process
//! MapWindowPoints proved unreliable (probe coordinate-space notes), and a
//! snapshot restored on an unchanged monitor layout needs no conversion.

use std::mem::size_of;

use serde::{Deserialize, Serialize};
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, POINT, WPARAM};
use windows::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
use windows::Win32::System::Memory::{
    VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
};
use windows::Win32::UI::Controls::{
    LVIF_TEXT, LVITEMW, LVM_GETITEMCOUNT, LVM_GETITEMPOSITION, LVM_GETITEMTEXTW,
    LVM_SETITEMPOSITION32,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, GetWindowThreadProcessId, SendMessageW,
};

use crate::app::error::{AppError, AppResult};

/// One icon: listview caption + position in listview client pixels.
#[derive(Serialize, Deserialize, Clone)]
pub struct LayoutItem {
    pub name: String,
    pub x: i32,
    pub y: i32,
}

/// JSON shape stored in `layout_snapshots.payload`.
#[derive(Serialize, Deserialize)]
pub struct LayoutPayload {
    pub items: Vec<LayoutItem>,
}

/// Outcome of a restore, in counts.
#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct ApplyReport {
    pub applied: usize,
    pub missing: usize,
    pub diverged: usize,
}

fn other<T>(msg: impl Into<String>) -> AppResult<T> {
    Err(AppError::Other(msg.into()))
}

fn win(step: &str, e: windows::core::Error) -> AppError {
    AppError::Other(format!("{step}: {e}"))
}

/// Progman/WorkerW → SHELLDLL_DefView → SysListView32. SHELLDLL_DefView is
/// usually a direct child of Progman; wallpaper tools and some drivers
/// reparent it into a WorkerW sibling, so search those too (probe diag2).
fn find_listview() -> AppResult<HWND> {
    unsafe {
        let progman_class = windows::core::w!("Progman");
        let workerw_class = windows::core::w!("WorkerW");
        let defview_class = windows::core::w!("SHELLDLL_DefView");
        let listview_class = windows::core::w!("SysListView32");

        let progman = FindWindowExW(None, None, progman_class, PCWSTR::null())
            .map_err(|e| win("Progman 窗口未找到", e))?;
        let mut defview = FindWindowExW(Some(progman), None, defview_class, PCWSTR::null());
        if defview.is_err() {
            let mut worker = FindWindowExW(None, None, workerw_class, PCWSTR::null());
            while let Ok(w) = worker {
                if let Ok(dv) = FindWindowExW(Some(w), None, defview_class, PCWSTR::null()) {
                    defview = Ok(dv);
                    break;
                }
                worker = FindWindowExW(None, Some(w), workerw_class, PCWSTR::null());
            }
        }
        let defview =
            defview.map_err(|_| AppError::Other("未找到桌面视图窗口（SHELLDLL_DefView）".into()))?;
        FindWindowExW(Some(defview), None, listview_class, PCWSTR::null())
            .map_err(|_| AppError::Other("未找到桌面图标列表（SysListView32）".into()))
    }
}

/// A page of memory inside the listview owner (explorer.exe): LVM message
/// arguments that are pointers must live in the target process.
struct RemoteMem {
    process: windows::Win32::Foundation::HANDLE,
    ptr: *mut std::ffi::c_void,
    size: usize,
}

impl RemoteMem {
    // SAFETY: caller guarantees `process` is a live handle with VM access.
    unsafe fn alloc(process: windows::Win32::Foundation::HANDLE, size: usize) -> AppResult<Self> {
        let ptr =
            VirtualAllocEx(process, None, size, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
        if ptr.is_null() {
            return other(format!("VirtualAllocEx({size} 字节) 失败"));
        }
        Ok(RemoteMem {
            process,
            ptr,
            size,
        })
    }

    // SAFETY: bounds are checked; `process`/`ptr` come from a live alloc.
    unsafe fn write_at(&self, offset: usize, bytes: &[u8]) -> AppResult<()> {
        if offset + bytes.len() > self.size {
            return other("远程写入越界");
        }
        WriteProcessMemory(
            self.process,
            (self.ptr as *const u8).add(offset) as *const _,
            bytes.as_ptr() as *const _,
            bytes.len(),
            None,
        )
        .map_err(|e| win("WriteProcessMemory", e))
    }

    // SAFETY: bounds are checked; `process`/`ptr` come from a live alloc.
    unsafe fn read_at(&self, offset: usize, buf: &mut [u8]) -> AppResult<()> {
        if offset + buf.len() > self.size {
            return other("远程读取越界");
        }
        ReadProcessMemory(
            self.process,
            (self.ptr as *const u8).add(offset) as *const _,
            buf.as_mut_ptr() as *mut _,
            buf.len(),
            None,
        )
        .map_err(|e| win("ReadProcessMemory", e))
    }
}

impl Drop for RemoteMem {
    fn drop(&mut self) {
        // SAFETY: matching alloc above; failures leak one page at worst.
        unsafe {
            let _ = VirtualFreeEx(self.process, self.ptr, 0, MEM_RELEASE);
        }
    }
}

/// The desktop icon list, operated through LVM messages across processes.
struct LvmView {
    listview: HWND,
    process: windows::Win32::Foundation::HANDLE,
}

impl Drop for LvmView {
    fn drop(&mut self) {
        // SAFETY: handle was opened by OpenProcess in attach().
        unsafe {
            let _ = CloseHandle(self.process);
        }
    }
}

impl LvmView {
    fn attach() -> AppResult<Self> {
        unsafe {
            let listview = find_listview()?;
            let mut pid = 0u32;
            let tid = GetWindowThreadProcessId(listview, Some(&mut pid));
            if pid == 0 {
                return other("SysListView32 没有属主进程");
            }
            let process = OpenProcess(
                PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
                false,
                pid,
            )
            .map_err(|e| {
                AppError::Other(format!("OpenProcess(explorer pid={pid}, tid={tid}): {e}"))
            })?;
            Ok(LvmView { listview, process })
        }
    }

    fn count(&self) -> usize {
        // SAFETY: plain message; no pointer arguments.
        unsafe {
            SendMessageW(
                self.listview,
                LVM_GETITEMCOUNT,
                Some(WPARAM(0)),
                Some(LPARAM(0)),
            )
            .0 as usize
        }
    }

    fn item_position(&self, index: i32) -> AppResult<POINT> {
        unsafe {
            let mem = RemoteMem::alloc(self.process, size_of::<POINT>())?;
            let sent = SendMessageW(
                self.listview,
                LVM_GETITEMPOSITION,
                Some(WPARAM(index as usize)),
                Some(LPARAM(mem.ptr as isize)),
            );
            if sent.0 == 0 {
                return other(format!("LVM_GETITEMPOSITION(index={index}) 返回 0"));
            }
            let mut buf = [0u8; 8];
            mem.read_at(0, &mut buf)?;
            Ok(POINT {
                x: i32::from_le_bytes(buf[0..4].try_into().unwrap()),
                y: i32::from_le_bytes(buf[4..8].try_into().unwrap()),
            })
        }
    }

    fn item_text(&self, index: i32) -> AppResult<String> {
        const TEXT_WCHARS: usize = 260;
        unsafe {
            let item_size = size_of::<LVITEMW>();
            let mem = RemoteMem::alloc(self.process, item_size + TEXT_WCHARS * 2)?;
            // pszText must point at the remote buffer, not ours.
            let item = LVITEMW {
                mask: LVIF_TEXT,
                iItem: index,
                iSubItem: 0,
                cchTextMax: TEXT_WCHARS as i32,
                pszText: PWSTR((mem.ptr as *mut u8).add(item_size) as *mut u16),
                ..Default::default()
            };
            let bytes =
                std::slice::from_raw_parts(&item as *const LVITEMW as *const u8, item_size);
            mem.write_at(0, bytes)?;
            let sent = SendMessageW(
                self.listview,
                LVM_GETITEMTEXTW,
                Some(WPARAM(index as usize)),
                Some(LPARAM(mem.ptr as isize)),
            );
            if sent.0 == 0 {
                return other(format!("LVM_GETITEMTEXTW(index={index}) 返回 0"));
            }
            let mut raw = vec![0u8; TEXT_WCHARS * 2];
            mem.read_at(item_size, &mut raw)?;
            let wide: Vec<u16> = raw
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
            Ok(String::from_utf16_lossy(&wide[..len]))
        }
    }

    fn set_item_position(&self, index: i32, client: POINT) -> AppResult<()> {
        unsafe {
            let mem = RemoteMem::alloc(self.process, size_of::<POINT>())?;
            let mut buf = [0u8; 8];
            buf[0..4].copy_from_slice(&client.x.to_le_bytes());
            buf[4..8].copy_from_slice(&client.y.to_le_bytes());
            mem.write_at(0, &buf)?;
            SendMessageW(
                self.listview,
                LVM_SETITEMPOSITION32,
                Some(WPARAM(index as usize)),
                Some(LPARAM(mem.ptr as isize)),
            );
            Ok(())
        }
    }
}

/// Read every icon's caption + position. Read-only.
pub fn read_layout() -> AppResult<LayoutPayload> {
    let view = LvmView::attach()?;
    let count = view.count();
    let mut items = Vec::with_capacity(count);
    for i in 0..count as i32 {
        let name = view.item_text(i)?;
        let p = view.item_position(i)?;
        items.push(LayoutItem {
            name,
            x: p.x,
            y: p.y,
        });
    }
    Ok(LayoutPayload { items })
}

/// Refuse batch restores when the shell overrides position writes (自动排列).
/// Canary: move icon 0 by a clear delta, read back, always try to put it back.
/// Align-to-grid may snap the probe position to a grid corner — that still
/// counts as "write accepted"; only a no-op read-back means writes are being
/// overridden (probe: writes land within ~4/20 px of the request when the
/// grid is on).
pub fn canary_check() -> AppResult<()> {
    let view = LvmView::attach()?;
    if view.count() == 0 {
        return other("桌面列表为空，无法校验");
    }
    let orig = view.item_position(0)?;
    view.set_item_position(0, POINT {
        x: orig.x + 150,
        y: orig.y + 150,
    })?;
    let back = view.item_position(0)?;
    // Restore first, judge afterwards — data safety before correctness.
    view.set_item_position(0, orig)?;
    if (back.x - orig.x).abs() < 8 && (back.y - orig.y).abs() < 8 {
        return other(
            "检测到桌面图标位置写入被系统忽略（可能开启了“自动排列图标”），已取消恢复。\
             请在桌面右键菜单关闭自动排列后重试。",
        );
    }
    let final_pos = view.item_position(0)?;
    if (final_pos.x - orig.x).abs() > 40 || (final_pos.y - orig.y).abs() > 40 {
        view.set_item_position(0, orig)?;
        return other("金丝雀图标未能回到原位，已再次复位；请确认桌面无误后重试。");
    }
    Ok(())
}

/// Restore saved positions, matching by listview caption. Duplicate captions
/// are consumed in order. Always restores only positions; files untouched.
pub fn apply_layout(payload: &LayoutPayload) -> AppResult<ApplyReport> {
    if payload.items.is_empty() {
        return other("该布局没有保存任何图标位置");
    }
    let view = LvmView::attach()?;
    let count = view.count();
    if count == 0 {
        return other("桌面列表为空");
    }
    let mut current: Vec<(i32, String)> = Vec::with_capacity(count);
    for i in 0..count as i32 {
        current.push((i, view.item_text(i)?));
    }
    let mut applied = 0usize;
    let mut missing = 0usize;
    let mut written: Vec<(i32, &LayoutItem)> = Vec::new();
    for want in &payload.items {
        if let Some(pos) = current.iter().position(|(_, name)| name == &want.name) {
            let (idx, _) = current.remove(pos);
            view.set_item_position(idx, POINT {
                x: want.x,
                y: want.y,
            })?;
            written.push((idx, want));
            applied += 1;
        } else {
            missing += 1;
        }
    }
    let mut diverged = 0usize;
    for (idx, want) in &written {
        if let Ok(p) = view.item_position(*idx) {
            if (p.x - want.x).abs() > 1 || (p.y - want.y).abs() > 1 {
                diverged += 1;
            }
        }
    }
    Ok(ApplyReport {
        applied,
        missing,
        diverged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LIVE smoke: reads the real desktop (explorer must be running).
    /// Non-mutating; safe to run any time.
    #[test]
    #[ignore = "requires a live interactive desktop session"]
    fn live_read_layout() {
        let payload = read_layout().expect("read layout");
        assert!(!payload.items.is_empty(), "desktop should have icons");
        assert!(payload.items.iter().all(|it| !it.name.is_empty()));
        println!("live layout: {} icons", payload.items.len());
    }

    /// LIVE smoke: the canary moves icon 0 briefly and puts it back.
    #[test]
    #[ignore = "requires a live interactive desktop session; moves one icon briefly"]
    fn live_canary_roundtrip() {
        canary_check().expect("canary roundtrip");
    }
}
