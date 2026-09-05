//! Lazy, bounded desktop icon extraction.
//!
//! Icons are pulled from the Windows shell (the same imagery Explorer shows)
//! only on demand, cached in a small LRU keyed by path, and shipped to the
//! webview as raw RGBA (the frontend encodes PNG via canvas — no image crate
//! in the backend). Extraction failure is data, not an error: the UI falls
//! back to a generic glyph.

use std::collections::{HashMap, VecDeque};

use base64::Engine as _;
use windows::core::HSTRING;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
};
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

use crate::app::error::AppResult;

unsafe fn delete_bitmap(bitmap: HBITMAP) {
    let _ = DeleteObject(HGDIOBJ(bitmap.0));
}

/// Cached icons per process; 256 covers any realistic desktop several times.
const CACHE_CAPACITY: usize = 256;

/// RGBA pixel payload ready for the frontend (base64-encoded).
pub struct IconPixels {
    pub width: i32,
    pub height: i32,
    pub rgba: Vec<u8>,
}

/// Minimal FIFO-evicting LRU: refresh on hit, evict oldest on overflow.
/// The workload (a few hundred small blobs) never needs anything fancier.
pub struct LruCache<V> {
    map: HashMap<String, V>,
    order: VecDeque<String>,
    capacity: usize,
}

impl<V> LruCache<V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            capacity,
        }
    }

    pub fn get(&mut self, key: &str) -> Option<&V> {
        if self.map.contains_key(key) {
            self.touch(key);
        }
        self.map.get(key)
    }

    pub fn put(&mut self, key: String, value: V) {
        if self.map.contains_key(&key) {
            self.touch(&key);
        } else {
            self.order.push_back(key.clone());
        }
        self.map.insert(key, value);
        while self.map.len() > self.capacity {
            let oldest = self.order.pop_front();
            match oldest {
                Some(k) => {
                    self.map.remove(&k);
                }
                None => break,
            }
        }
    }

    fn touch(&mut self, key: &str) {
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            self.order.remove(pos);
            self.order.push_back(key.to_string());
        }
    }

    #[allow(dead_code)] // exercised by tests
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

fn cache() -> std::sync::MutexGuard<'static, LruCache<IconPixels>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<LruCache<IconPixels>>> =
        std::sync::OnceLock::new();
    CACHE
        .get_or_init(|| std::sync::Mutex::new(LruCache::new(CACHE_CAPACITY)))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Extract the shell icon for a path (base64 RGBA), using the cache.
pub fn extract_cached(path: &str) -> AppResult<Option<IconPayload>> {
    if let Some(hit) = cache().get(path) {
        return Ok(Some(encode(hit)));
    }
    let Some(pixels) = extract_rgba(path) else {
        return Ok(None);
    };
    let payload = encode(&pixels);
    cache().put(path.to_string(), pixels);
    Ok(Some(payload))
}

/// Wire format for the frontend: dimensions + base64 RGBA.
pub struct IconPayload {
    pub width: i32,
    pub height: i32,
    pub rgba: String,
}

fn encode(pixels: &IconPixels) -> IconPayload {
    IconPayload {
        width: pixels.width,
        height: pixels.height,
        rgba: base64::engine::general_purpose::STANDARD.encode(&pixels.rgba),
    }
}

/// Ask the shell for the icon and read its pixels. Any failure along the way
/// yields `None` — callers show a generic glyph instead.
fn extract_rgba(path: &str) -> Option<IconPixels> {
    unsafe {
        let wide = HSTRING::from(path);
        let mut shfi = SHFILEINFOW::default();
        let ok = SHGetFileInfoW(
            windows::core::PCWSTR(wide.as_ptr()),
            Default::default(),
            Some(&mut shfi),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        );
        if ok == 0 || shfi.hIcon.is_invalid() {
            return None;
        }
        let hicon = shfi.hIcon;
        let result = read_hicon_rgba(hicon);
        let _ = DestroyIcon(hicon);
        result
    }
}

/// GetIconInfo → read the color bitmap as 32bpp top-down BGRA → fix alpha
/// from the AND mask when the icon carries none (classic 4-bit/8-bit icons).
unsafe fn read_hicon_rgba(
    hicon: windows::Win32::UI::WindowsAndMessaging::HICON,
) -> Option<IconPixels> {
    let mut info = ICONINFO::default();
    GetIconInfo(hicon, &mut info).ok()?;

    let color = info.hbmColor;
    let mask = info.hbmMask;
    if color.is_invalid() || mask.is_invalid() {
        if !color.is_invalid() {
            delete_bitmap(color);
        }
        if !mask.is_invalid() {
            delete_bitmap(mask);
        }
        return None;
    }

    // Dimensions via GetObjectW(BITMAP).
    let mut bm = BITMAP::default();
    if GetObjectW(
        HGDIOBJ(color.0),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bm as *mut _ as *mut _),
    ) == 0
    {
        delete_bitmap(color);
        delete_bitmap(mask);
        return None;
    }
    let (width, height) = (bm.bmWidth, bm.bmHeight.abs());
    if width <= 0 || height <= 0 || width > 512 || height > 512 {
        delete_bitmap(color);
        delete_bitmap(mask);
        return None;
    }

    let hdc = CreateCompatibleDC(None);
    // Negative height → top-down rows, no bottom-up flipping later.
    let mut bmi = top_down_info(width, height, 32);

    let mut bytes = vec![0u8; (width as usize) * (height as usize) * 4];
    let lines = GetDIBits(
        hdc,
        color,
        0,
        height as u32,
        Some(bytes.as_mut_ptr().cast()),
        &mut bmi,
        DIB_RGB_COLORS,
    );
    let mask_bits = read_mask_bits(hdc, mask, width, height);

    delete_bitmap(color);
    delete_bitmap(mask);
    let _ = DeleteDC(hdc);
    if lines == 0 {
        return None;
    }

    // BGRA → RGBA, and alpha fixup from the AND mask for legacy icons.
    let mask_row = (width as usize).div_ceil(32) * 4;
    for i in 0..(width as usize) * (height as usize) {
        let p = i * 4;
        bytes.swap(p, p + 2);
        if bytes[p + 3] == 0 {
            let opaque = mask_bits.as_ref().is_none_or(|m| {
                let x = i % width as usize;
                let y = i / width as usize;
                (m[y * mask_row + x / 8] >> (7 - (x % 8))) & 1 == 0
            });
            if opaque {
                bytes[p + 3] = 255;
            }
        }
    }

    Some(IconPixels {
        width,
        height,
        rgba: bytes,
    })
}

/// Top-down (negative height) uncompressed BI_RGB header for GetDIBits.
fn top_down_info(width: i32, height: i32, bit_count: u16) -> BITMAPINFO {
    BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: bit_count,
            biCompression: DIB_RGB_COLORS.0, // BI_RGB: no fancy compression
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Read the 1bpp AND mask top-down (only the half over the color bitmap).
unsafe fn read_mask_bits(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    mask: windows::Win32::Graphics::Gdi::HBITMAP,
    width: i32,
    height: i32,
) -> Option<Vec<u8>> {
    let mut bmi = top_down_info(width, height, 1);
    let row = (width as usize).div_ceil(32) * 4;
    let mut bits = vec![0u8; row * height as usize];
    let lines = GetDIBits(
        hdc,
        mask,
        0,
        height as u32,
        Some(bits.as_mut_ptr().cast()),
        &mut bmi,
        DIB_RGB_COLORS,
    );
    (lines != 0).then_some(bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lru_evicts_oldest_beyond_capacity() {
        let mut cache = LruCache::new(2);
        cache.put("a".into(), 1);
        cache.put("b".into(), 2);
        assert_eq!(cache.get("a"), Some(&1)); // a is now newest
        cache.put("c".into(), 3); // evicts b
        assert_eq!(cache.get("b"), None);
        assert_eq!(cache.get("a"), Some(&1));
        assert_eq!(cache.get("c"), Some(&3));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    #[cfg(windows)]
    fn extracts_a_real_system_icon() {
        let payload = extract_cached("C:\\Windows\\System32\\notepad.exe")
            .unwrap()
            .expect("notepad.exe must have an icon");
        assert!(payload.width >= 16);
        // The payload is base64-encoded RGBA; decoding must restore exactly
        // width × height × 4 bytes.
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&payload.rgba)
            .unwrap();
        assert_eq!(
            decoded.len(),
            payload.width as usize * payload.height as usize * 4
        );
        // Second call must hit the cache (same payload object semantics).
        let again = extract_cached("C:\\Windows\\System32\\notepad.exe")
            .unwrap()
            .expect("cached entry must still resolve");
        assert_eq!(again.rgba, payload.rgba);
    }

    #[test]
    #[cfg(windows)]
    fn missing_path_yields_none_not_error() {
        assert!(extract_rgba("Z:\\definitely\\missing\\thing.xyz").is_none());
    }
}
