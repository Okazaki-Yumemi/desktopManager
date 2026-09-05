//! shell_probe — standalone Windows desktop-shell probe for DesktopManager.
//!
//! Verifies, on the real machine, whether documented shell interfaces can
//! read and write Explorer desktop icon positions without touching files.
//! See docs/WINDOWS_SHELL_PROBE.md for the protocol and results.
//!
//! Two routes are attempted, in order:
//!   1. COM: ShellWindows → FindWindowSW(SWC_DESKTOP) → IServiceProvider
//!      → IShellBrowser → IFolderView (documented, no cross-process memory).
//!   2. LVM: Progman/WorkerW → SHELLDLL_DefView → SysListView32 window chain,
//!      then LVM_* messages with remote buffers (VirtualAllocEx / Read /
//!      WriteProcessMemory into explorer.exe).
//!
//! Subcommands:
//!   info                      enumerate desktop items + positions
//!   snapshot <out.json>       save all positions to a JSON file
//!   move <name> <x> <y>       move one item (file name), screen px
//!   restore <snapshot.json>   restore positions by name
//!   verify <snapshot.json>    compare current positions against a snapshot
//!   prepare                   create dm-probe-1.txt / dm-probe-2.txt on desktop
//!   cleanup                   delete those two probe files
//!   diag                      FindWindowSW variant diagnostics
//!   diag2                     window-chain + ShellWindows activation diagnostics

#![cfg(windows)]

use std::mem::size_of;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use windows::core::{Interface, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::MapWindowPoints;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, CLSCTX_LOCAL_SERVER,
    COINIT_APARTMENTTHREADED, IDispatch, IServiceProvider,
};
use windows::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
use windows::Win32::System::Memory::{
    VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
};
use windows::Win32::System::Variant::{VARIANT, VT_I4, VT_UI4};
use windows::Win32::UI::Controls::{
    LVIF_TEXT, LVITEMW, LVM_GETITEMCOUNT, LVM_GETITEMPOSITION, LVM_GETITEMTEXTW,
    LVM_SETITEMPOSITION32,
};
use windows::Win32::UI::Shell::Common::{STRRET, STRRET_WSTR};
use windows::Win32::UI::Shell::{
    IShellFolder, IShellWindows, SHGDN_FORPARSING, SHGetKnownFolderPath, SID_STopLevelBrowser,
    SWC_DESKTOP, SWFO_NEEDDISPATCH, SVSI_DESELECTOTHERS, SVSI_POSITIONITEM, SVSI_SELECT,
    SVGIO_ALLVIEW, ShellWindowFindWindowOptions,
};
use windows::Win32::UI::Shell::{FOLDERID_Desktop, KNOWN_FOLDER_FLAG};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, GA_PARENT, GetAncestor, GetClassNameW, GetWindowThreadProcessId, SendMessageW,
};

const PROBE_FILES: [&str; 2] = ["dm-probe-1.txt", "dm-probe-2.txt"];

#[derive(Serialize, Deserialize, Clone, Copy)]
struct ItemPos {
    x: i32,
    y: i32,
}

#[derive(Serialize, Deserialize, Clone)]
struct SnapshotItem {
    name: String,
    index: usize,
    /// Position as returned by the shell (ListView client coordinates).
    pos: ItemPos,
    /// Same point converted to screen coordinates, for reference.
    screen: ItemPos,
}

#[derive(Serialize, Deserialize)]
struct Snapshot {
    captured_at: u64,
    method: String,
    desktop_path: String,
    items: Vec<SnapshotItem>,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

mod err {
    // Tiny error type so the probe stays dependency-light.
    #[derive(Debug)]
    pub struct Error(pub String);
    impl From<windows::core::Error> for Error {
        fn from(e: windows::core::Error) -> Self {
            Error(format!("HRESULT {:#010x}: {e}", e.code().0))
        }
    }
    impl From<std::io::Error> for Error {
        fn from(e: std::io::Error) -> Self {
            Error(e.to_string())
        }
    }
    impl From<std::string::FromUtf16Error> for Error {
        fn from(e: std::string::FromUtf16Error) -> Self {
            Error(e.to_string())
        }
    }
    impl From<serde_json::Error> for Error {
        fn from(e: serde_json::Error) -> Self {
            Error(e.to_string())
        }
    }
    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&self.0)
        }
    }
    pub type Result<T> = std::result::Result<T, Error>;
}
use err::{Error, Result};

fn desktop_dir() -> Result<PathBuf> {
    unsafe {
        let pw = SHGetKnownFolderPath(&FOLDERID_Desktop, KNOWN_FOLDER_FLAG(0), None)?;
        let s = PCWSTR::from_raw(pw.as_ptr()).to_string()?;
        CoTaskMemFree(Some(pw.as_ptr() as *const _));
        Ok(PathBuf::from(s))
    }
}

/// VARIANT holding a shell-window class constant (SWC_*).
unsafe fn shell_window_variant() -> VARIANT {
    let mut loc = VARIANT::default();
    // VARIANT nests ManuallyDrop'd unions in windows 0.61; explicit derefs required.
    (*loc.Anonymous.Anonymous).vt = VT_UI4;
    (*loc.Anonymous.Anonymous).Anonymous.ulVal = SWC_DESKTOP.0 as u32;
    loc
}

unsafe fn find_listview() -> Result<HWND> {
    let progman_class = windows::core::w!("Progman");
    let workerw_class = windows::core::w!("WorkerW");
    let defview_class = windows::core::w!("SHELLDLL_DefView");
    let listview_class = windows::core::w!("SysListView32");

    let progman = FindWindowExW(None, None, progman_class, PCWSTR::null())
        .map_err(|e| Error(format!("Progman window not found: {e}")))?;
    // SHELLDLL_DefView is usually a direct child of Progman, but wallpaper
    // tools (and some drivers) reparent it into a WorkerW sibling.
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
    let defview = defview
        .map_err(|_| Error("SHELLDLL_DefView not found under Progman or any WorkerW".into()))?;
    let listview = FindWindowExW(Some(defview), None, listview_class, PCWSTR::null())
        .map_err(|_| Error("SysListView32 not found under SHELLDLL_DefView".into()))?;
    Ok(listview)
}

fn window_class(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let n = unsafe { GetClassNameW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

fn window_owner(hwnd: HWND) -> (u32, u32) {
    let mut pid = 0u32;
    let tid = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    (tid, pid)
}

// ---------------------------------------------------------------------------
// LVM fallback backend
// ---------------------------------------------------------------------------

/// A page of memory allocated inside the listview owner (explorer.exe).
struct RemoteMem {
    process: HANDLE,
    ptr: *mut std::ffi::c_void,
    size: usize,
}

impl RemoteMem {
    unsafe fn alloc(process: HANDLE, size: usize) -> Result<Self> {
        let ptr = VirtualAllocEx(process, None, size, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
        if ptr.is_null() {
            return Err(Error(format!(
                "VirtualAllocEx({size} bytes) in explorer.exe returned NULL"
            )));
        }
        Ok(RemoteMem { process, ptr, size })
    }

    unsafe fn write_at(&self, offset: usize, bytes: &[u8]) -> Result<()> {
        if offset + bytes.len() > self.size {
            return Err(Error("remote write out of bounds".into()));
        }
        WriteProcessMemory(
            self.process,
            (self.ptr as *const u8).add(offset) as *const _,
            bytes.as_ptr() as *const _,
            bytes.len(),
            None,
        )
        .map_err(|e| Error(format!("WriteProcessMemory: {e}")))
    }

    unsafe fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<()> {
        if offset + buf.len() > self.size {
            return Err(Error("remote read out of bounds".into()));
        }
        ReadProcessMemory(
            self.process,
            (self.ptr as *const u8).add(offset) as *const _,
            buf.as_mut_ptr() as *mut _,
            buf.len(),
            None,
        )
        .map_err(|e| Error(format!("ReadProcessMemory: {e}")))
    }
}

impl Drop for RemoteMem {
    fn drop(&mut self) {
        unsafe {
            let _ = VirtualFreeEx(self.process, self.ptr, 0, MEM_RELEASE);
        }
    }
}

/// Desktop icon list operated through SysListView32 messages. Works across
/// processes: message arguments that are pointers must live in the target
/// process, hence the RemoteMem dance.
struct LvmView {
    listview: HWND,
    process: HANDLE,
}

impl Drop for LvmView {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.process);
        }
    }
}

impl LvmView {
    unsafe fn attach(listview: HWND) -> Result<Self> {
        let (tid, pid) = window_owner(listview);
        if pid == 0 {
            return Err(Error(format!(
                "SysListView32 {:#x} has no owner process",
                listview.0 as usize
            )));
        }
        let process = OpenProcess(
            PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
            false,
            pid,
        )
        .map_err(|e| Error(format!("OpenProcess(explorer pid={pid}, tid={tid}): {e}")))?;
        Ok(LvmView { listview, process })
    }

    fn count(&self) -> usize {
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

    unsafe fn item_position(&self, index: i32) -> Result<POINT> {
        let mem = RemoteMem::alloc(self.process, size_of::<POINT>())?;
        let sent = SendMessageW(
            self.listview,
            LVM_GETITEMPOSITION,
            Some(WPARAM(index as usize)),
            Some(LPARAM(mem.ptr as isize)),
        );
        if sent.0 == 0 {
            return Err(Error(format!("LVM_GETITEMPOSITION(index={index}) returned 0")));
        }
        let mut buf = [0u8; 8];
        mem.read_at(0, &mut buf)?;
        Ok(POINT {
            x: i32::from_le_bytes(buf[0..4].try_into().unwrap()),
            y: i32::from_le_bytes(buf[4..8].try_into().unwrap()),
        })
    }

    unsafe fn item_text(&self, index: i32) -> Result<String> {
        const TEXT_WCHARS: usize = 260;
        let item_size = size_of::<LVITEMW>();
        let mem = RemoteMem::alloc(self.process, item_size + TEXT_WCHARS * 2)?;
        let mut item = LVITEMW::default();
        item.mask = LVIF_TEXT;
        item.iItem = index;
        item.iSubItem = 0;
        item.cchTextMax = TEXT_WCHARS as i32;
        // pszText must point at the remote buffer, not ours.
        item.pszText = PWSTR((mem.ptr as *mut u8).add(item_size) as *mut u16);
        let bytes = std::slice::from_raw_parts(&item as *const LVITEMW as *const u8, item_size);
        mem.write_at(0, bytes)?;
        let sent = SendMessageW(
            self.listview,
            LVM_GETITEMTEXTW,
            Some(WPARAM(index as usize)),
            Some(LPARAM(mem.ptr as isize)),
        );
        if sent.0 == 0 {
            return Err(Error(format!("LVM_GETITEMTEXTW(index={index}) returned 0")));
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

    unsafe fn set_item_position(&self, index: i32, client: POINT) -> Result<()> {
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

// ---------------------------------------------------------------------------
// Unified desktop view: COM route with LVM fallback
// ---------------------------------------------------------------------------

/// COM-route access to the desktop's folder view.
struct ComFolder {
    view: windows::Win32::UI::Shell::IFolderView,
    folder: IShellFolder,
}

struct DesktopView {
    /// Backend identifier recorded into snapshots ("IFolderView" / "LVM").
    method: &'static str,
    com: Option<ComFolder>,
    lvm: Option<LvmView>,
    listview_hwnd: HWND,
}

unsafe fn try_open_com_view() -> Result<(ComFolder, HWND)> {
    let shell_windows: IShellWindows = CoCreateInstance(
        &windows::Win32::UI::Shell::ShellWindows,
        None,
        CLSCTX_LOCAL_SERVER,
    )
    .map_err(|e| Error(format!("CoCreateInstance(ShellWindows): {e}")))?;
    let loc = shell_window_variant();
    let mut loc_root = VARIANT::default();
    let mut hwnd_i32 = 0i32;
    let dispatch = shell_windows
        .FindWindowSW(&loc, &mut loc_root, SWC_DESKTOP, &mut hwnd_i32, SWFO_NEEDDISPATCH)
        .map_err(|e| Error(format!("FindWindowSW(SWC_DESKTOP): {e}")))?;
    if dispatch.as_raw() as usize == 0 {
        return Err(Error(
            "FindWindowSW returned S_OK but a NULL dispatch (desktop not registered?)".into(),
        ));
    }

    let provider: IServiceProvider = dispatch
        .cast()
        .map_err(|e| Error(format!("cast(IServiceProvider): {e}")))?;
    let browser = provider
        .QueryService::<windows::Win32::UI::Shell::IShellBrowser>(&SID_STopLevelBrowser)
        .map_err(|e| Error(format!("QueryService(SID_STopLevelBrowser): {e}")))?;
    let folder_view: windows::Win32::UI::Shell::IFolderView = browser
        .cast()
        .map_err(|e| Error(format!("cast(IFolderView): {e}")))?;
    let shell_folder: IShellFolder = folder_view
        .GetFolder()
        .map_err(|e| Error(format!("GetFolder(IShellFolder): {e}")))?;

    let listview_hwnd = find_listview().unwrap_or(HWND(hwnd_i32 as usize as *mut _));
    Ok((
        ComFolder {
            view: folder_view,
            folder: shell_folder,
        },
        listview_hwnd,
    ))
}

unsafe fn open_desktop_view() -> Result<DesktopView> {
    match try_open_com_view() {
        Ok((com, listview)) => Ok(DesktopView {
            method: "IFolderView",
            com: Some(com),
            lvm: None,
            listview_hwnd: listview,
        }),
        Err(com_err) => {
            println!("[route] IFolderView unavailable ({com_err}); falling back to LVM");
            let listview = find_listview()?;
            let lvm = LvmView::attach(listview)?;
            Ok(DesktopView {
                method: "LVM",
                com: None,
                lvm: Some(lvm),
                listview_hwnd: listview,
            })
        }
    }
}

impl DesktopView {
    unsafe fn item_count(&self) -> Result<usize> {
        if let Some(com) = &self.com {
            return Ok(com.view.ItemCount(SVGIO_ALLVIEW)? as usize);
        }
        if let Some(lvm) = &self.lvm {
            return Ok(lvm.count());
        }
        Err(Error("no backend".into()))
    }

    unsafe fn client_pos_at(&self, index: i32) -> Result<ItemPos> {
        if let Some(com) = &self.com {
            let pidl = com.view.Item(index)?;
            let pt = com.view.GetItemPosition(pidl)?;
            return Ok(ItemPos { x: pt.x, y: pt.y });
        }
        if let Some(lvm) = &self.lvm {
            let pt = lvm.item_position(index)?;
            return Ok(ItemPos { x: pt.x, y: pt.y });
        }
        Err(Error("no backend".into()))
    }

    unsafe fn place_at(&self, index: i32, screen: ItemPos) -> Result<()> {
        if let Some(com) = &self.com {
            let pidl = com.view.Item(index)?;
            let client = self.from_screen(screen);
            let pts = [POINT {
                x: client.x,
                y: client.y,
            }];
            let pidls: [*const windows::Win32::UI::Shell::Common::ITEMIDLIST; 1] = [pidl];
            com.view.SelectAndPositionItems(
                1,
                pidls.as_ptr(),
                Some(pts.as_ptr()),
                (SVSI_POSITIONITEM.0 | SVSI_SELECT.0 | SVSI_DESELECTOTHERS.0) as u32,
            )?;
            return Ok(());
        }
        if let Some(lvm) = &self.lvm {
            let client = self.from_screen(screen);
            return lvm.set_item_position(index, POINT { x: client.x, y: client.y });
        }
        Err(Error("no backend".into()))
    }

    /// Item name. COM route: full parsing path (unique). LVM route: the
    /// listview caption (file name). Matching goes through `find_item`, which
    /// accepts either form.
    unsafe fn name_at(&self, index: i32) -> Result<String> {
        if let Some(com) = &self.com {
            let pidl = com.view.Item(index)?;
            let mut strret = STRRET::default();
            com.folder
                .GetDisplayNameOf(pidl, SHGDN_FORPARSING, &mut strret)?;
            if strret.uType != STRRET_WSTR.0 as u32 {
                return Err(Error(format!("unexpected STRRET type {}", strret.uType)));
            }
            let pw = strret.Anonymous.pOleStr;
            let s = PCWSTR::from_raw(pw.as_ptr()).to_string()?;
            CoTaskMemFree(Some(pw.as_ptr() as *const _));
            return Ok(s);
        }
        if let Some(lvm) = &self.lvm {
            return lvm.item_text(index);
        }
        Err(Error("no backend".into()))
    }

    unsafe fn to_screen(&self, pt: ItemPos) -> ItemPos {
        let p = POINT { x: pt.x, y: pt.y };
        MapWindowPoints(Some(self.listview_hwnd), None, &mut [p][..]);
        ItemPos { x: p.x, y: p.y }
    }

    unsafe fn from_screen(&self, pt: ItemPos) -> ItemPos {
        let p = POINT { x: pt.x, y: pt.y };
        MapWindowPoints(None, Some(self.listview_hwnd), &mut [p][..]);
        ItemPos { x: p.x, y: p.y }
    }

    unsafe fn enumerate(&self) -> Result<Vec<SnapshotItem>> {
        let count = self.item_count()?;
        let mut items = Vec::with_capacity(count);
        for i in 0..count as i32 {
            let name = self.name_at(i)?;
            let pos = self.client_pos_at(i)?;
            let screen = self.to_screen(pos);
            items.push(SnapshotItem {
                name,
                index: i as usize,
                pos,
                screen,
            });
        }
        Ok(items)
    }
}

/// Match a requested item name against an enumerated name. Enumerated names
/// are full paths on the COM route and bare captions on the LVM route.
fn find_item<'a>(items: &'a [SnapshotItem], name: &str) -> Option<&'a SnapshotItem> {
    items
        .iter()
        .find(|it| it.name == name || it.name.rsplit('\\').next() == Some(name))
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn cmd_info() -> Result<()> {
    unsafe {
        let view = open_desktop_view()?;
        println!("desktop dir  : {}", desktop_dir()?.display());
        println!("route        : {}", view.method);
        println!("listview hwnd: {:?}", view.listview_hwnd);
        let items = view.enumerate()?;
        println!("items: {}", items.len());
        for it in &items {
            println!(
                "  [{:>3}] {:<40} client=({:>5},{:>5}) screen=({:>5},{:>5})",
                it.index, it.name, it.pos.x, it.pos.y, it.screen.x, it.screen.y
            );
        }
    }
    Ok(())
}

fn cmd_snapshot(out_path: &str) -> Result<()> {
    unsafe {
        let view = open_desktop_view()?;
        let items = view.enumerate()?;
        let snap = Snapshot {
            captured_at: now_secs(),
            method: view.method.to_string(),
            desktop_path: desktop_dir()?.to_string_lossy().into_owned(),
            items,
        };
        std::fs::write(out_path, serde_json::to_vec_pretty(&snap)?)?;
        println!("snapshot: {} items -> {} (route {})", snap.items.len(), out_path, snap.method);
    }
    Ok(())
}

fn load_snapshot(path: &str) -> Result<Snapshot> {
    let data = std::fs::read(path)?;
    Ok(serde_json::from_slice(&data)?)
}

fn cmd_restore(path: &str) -> Result<()> {
    unsafe {
        let snap = load_snapshot(path)?;
        let view = open_desktop_view()?;
        let items = view.enumerate()?;
        let mut restored = 0usize;
        let mut missing = 0usize;
        for want in &snap.items {
            if let Some(cur) = find_item(&items, &want.name) {
                view.place_at(cur.index as i32, want.screen)?;
                restored += 1;
            } else {
                missing += 1;
                println!("  missing: {}", want.name);
            }
        }
        println!("restore: {restored} positioned, {missing} missing");
    }
    Ok(())
}

fn cmd_verify(path: &str) -> Result<()> {
    unsafe {
        let snap = load_snapshot(path)?;
        let view = open_desktop_view()?;
        let items = view.enumerate()?;
        let mut ok = 0usize;
        let mut diff = 0usize;
        let mut missing = 0usize;
        for want in &snap.items {
            match find_item(&items, &want.name) {
                None => {
                    missing += 1;
                    println!("  MISSING  {}", want.name);
                }
                Some(cur) => {
                    if (cur.screen.x - want.screen.x).abs() <= 1
                        && (cur.screen.y - want.screen.y).abs() <= 1
                    {
                        ok += 1;
                    } else {
                        diff += 1;
                        println!(
                            "  DIFF     {}: want ({},{}) got ({},{})",
                            want.name, want.screen.x, want.screen.y, cur.screen.x, cur.screen.y
                        );
                    }
                }
            }
        }
        println!("verify: {ok} match, {diff} differ, {missing} missing");
        if diff + missing > 0 {
            std::process::exit(1);
        }
    }
    Ok(())
}

fn cmd_move(name: &str, x: i32, y: i32) -> Result<()> {
    unsafe {
        let view = open_desktop_view()?;
        let items = view.enumerate()?;
        let target = find_item(&items, name)
            .ok_or_else(|| Error(format!("item '{name}' not found on desktop")))?;
        view.place_at(target.index as i32, ItemPos { x, y })?;
        let after = view.client_pos_at(target.index as i32)?;
        let after_screen = view.to_screen(after);
        println!(
            "moved '{name}' -> client=({},{}) screen=({},{})",
            after.x, after.y, after_screen.x, after_screen.y
        );
    }
    Ok(())
}

fn cmd_prepare() -> Result<()> {
    let dir = desktop_dir()?;
    for f in PROBE_FILES {
        let p = dir.join(f);
        std::fs::write(&p, "DesktopManager shell probe test file\r\n")?;
        println!("created {}", p.display());
    }
    std::thread::sleep(std::time::Duration::from_millis(1200));
    Ok(())
}

fn cmd_cleanup() -> Result<()> {
    let dir = desktop_dir()?;
    for f in PROBE_FILES {
        let p = dir.join(f);
        match std::fs::remove_file(&p) {
            Ok(()) => println!("removed {}", p.display()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => println!("(absent) {f}"),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// Diagnostic: try FindWindowSW variants and report which (if any) yields the
/// desktop dispatch. Used to document behavior on this OS build.
fn cmd_diag() -> Result<()> {
    unsafe {
        let sw: IShellWindows = CoCreateInstance(
            &windows::Win32::UI::Shell::ShellWindows,
            None,
            CLSCTX_LOCAL_SERVER,
        )
        .map_err(|e| Error(format!("create: {e}")))?;
        let count = sw.Count().map_err(|e| Error(format!("count: {e}")))?;
        println!("ShellWindows count: {count}");
        for (vt_name, use_i4) in [("VT_UI4", false), ("VT_I4", true)] {
            for flags in [0i32, 1i32] {
                let mut loc = VARIANT::default();
                let mut root = VARIANT::default();
                if use_i4 {
                    (*loc.Anonymous.Anonymous).vt = VT_I4;
                    (*loc.Anonymous.Anonymous).Anonymous.lVal = SWC_DESKTOP.0;
                } else {
                    (*loc.Anonymous.Anonymous).vt = VT_UI4;
                    (*loc.Anonymous.Anonymous).Anonymous.ulVal = SWC_DESKTOP.0 as u32;
                }
                let mut hwnd = 0i32;
                let r: std::result::Result<IDispatch, _> = sw.FindWindowSW(
                    &loc,
                    &mut root,
                    SWC_DESKTOP,
                    &mut hwnd,
                    ShellWindowFindWindowOptions(flags),
                );
                match r {
                    Ok(d) => println!(
                        "{vt_name} flags={flags}: OK hwnd={hwnd:#x} disp_null={}",
                        d.as_raw() as usize == 0
                    ),
                    Err(e) => println!("{vt_name} flags={flags}: ERR {e} hwnd={hwnd:#x}"),
                }
            }
        }
    }
    Ok(())
}

/// Diagnostic 2: raw window chain (Progman/WorkerW → SHELLDLL_DefView →
/// SysListView32) and ShellWindows activation-class comparison. Decides
/// whether the LVM-message fallback is viable when the COM route is blocked.
fn cmd_diag2() -> Result<()> {
    unsafe {
        println!("--- ShellWindows activation ---");
        for (name, ctx) in [
            ("CLSCTX_ALL          ", CLSCTX_ALL),
            ("CLSCTX_LOCAL_SERVER  ", CLSCTX_LOCAL_SERVER),
        ] {
            let created: windows::core::Result<IShellWindows> =
                CoCreateInstance(&windows::Win32::UI::Shell::ShellWindows, None, ctx);
            match created {
                Ok(sw) => match sw.Count() {
                    Ok(c) => println!("{name}: created, Count()={c}"),
                    Err(e) => println!("{name}: created, Count() ERR {e}"),
                },
                Err(e) => println!("{name}: create ERR {e}"),
            }
        }

        println!("--- window chain ---");
        let progman_class = windows::core::w!("Progman");
        let defview_class = windows::core::w!("SHELLDLL_DefView");
        let workerw_class = windows::core::w!("WorkerW");

        // Any top-level SHELLDLL_DefView, whatever its actual parent is.
        let defview_hwnd = match FindWindowExW(None, None, defview_class, PCWSTR::null()) {
            Ok(dv) => {
                println!("top-level SHELLDLL_DefView: {:#x}", dv.0 as usize);
                let parent = GetAncestor(dv, GA_PARENT);
                if !parent.0.is_null() {
                    println!("  parent: {:#x} class={}", parent.0 as usize, window_class(parent));
                }
                let (tid, pid) = window_owner(dv);
                println!("  owner tid={tid} pid={pid}");
                Some(dv)
            }
            Err(e) => {
                println!("top-level SHELLDLL_DefView: ERR {e}");
                None
            }
        };

        let progman_hwnd = FindWindowExW(None, None, progman_class, PCWSTR::null()).ok();
        println!("Progman: {:?}", progman_hwnd.map(|h| h.0 as usize));
        if let Some(pm) = progman_hwnd {
            let direct = FindWindowExW(Some(pm), None, defview_class, PCWSTR::null());
            println!(
                "Progman > SHELLDLL_DefView (direct): {:?}",
                direct.map(|h| h.0 as usize)
            );
        }

        let mut worker = FindWindowExW(None, None, workerw_class, PCWSTR::null());
        let mut n = 0usize;
        while let Ok(w) = worker {
            let child = FindWindowExW(Some(w), None, defview_class, PCWSTR::null());
            println!(
                "WorkerW[{n}] {:#x} > SHELLDLL_DefView: {:?}",
                w.0 as usize,
                child.map(|h| h.0 as usize)
            );
            worker = FindWindowExW(None, Some(w), workerw_class, PCWSTR::null());
            n += 1;
            if n >= 16 {
                println!("(workerw iteration capped at 16)");
                break;
            }
        }
        let _ = defview_hwnd;

        // The listview decides the fallback route. find_listview() prefers the
        // Progman direct child, which is where Win11 keeps it.
        match find_listview() {
            Ok(lv) => {
                let (tid, pid) = window_owner(lv);
                let count = SendMessageW(lv, LVM_GETITEMCOUNT, Some(WPARAM(0)), Some(LPARAM(0)));
                println!(
                    "SysListView32: {:#x} tid={tid} pid={pid} LVM_GETITEMCOUNT={}",
                    lv.0 as usize,
                    count.0
                );
            }
            Err(e) => println!("SysListView32: unreachable: {e}"),
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = run(&args);
    if let Err(e) = result {
        eprintln!("FAILED: {e}");
        std::process::exit(1);
    }
}

fn run(args: &[String]) -> Result<()> {
    let cmd = args.first().map(String::as_str).unwrap_or("");
    let mut rest = args.iter().skip(1);
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let _ = hr; // RPC_E_CHANGED_MODE is tolerable: COM is available either way.
        match cmd {
            "info" => cmd_info(),
            "snapshot" => cmd_snapshot(arg(rest.next(), "snapshot <out.json>")?),
            "restore" => cmd_restore(arg(rest.next(), "restore <snapshot.json>")?),
            "verify" => cmd_verify(arg(rest.next(), "verify <snapshot.json>")?),
            "move" => {
                let name = arg(rest.next(), "move <name> <x> <y>")?;
                let x: i32 = arg(rest.next(), "move <name> <x> <y>")?
                    .parse()
                    .map_err(|e: std::num::ParseIntError| Error(e.to_string()))?;
                let y: i32 = arg(rest.next(), "move <name> <x> <y>")?
                    .parse()
                    .map_err(|e: std::num::ParseIntError| Error(e.to_string()))?;
                cmd_move(&name, x, y)
            }
            "prepare" => cmd_prepare(),
            "cleanup" => cmd_cleanup(),
            "diag" => cmd_diag(),
            "diag2" => cmd_diag2(),
            _ => {
                eprintln!(
                    "usage: shell_probe <info|snapshot|restore|verify|move|prepare|cleanup|diag|diag2> [args]"
                );
                std::process::exit(2);
            }
        }
    }
}

fn arg<'a>(value: Option<&'a String>, usage: &str) -> Result<&'a str> {
    value
        .map(String::as_str)
        .ok_or_else(|| Error(format!("usage: {usage}")))
}
