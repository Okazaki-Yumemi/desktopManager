//! Top-level desktop folder scanner.
//!
//! Only the first level is indexed — that is exactly what the desktop view
//! shows. No recursion, no following into folders, and files are only read
//! for metadata, never moved or modified.

use std::path::Path;

/// One scanned desktop entry, ready for the repository layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedItem {
    /// Absolute path (display form, backslashes) — natural key in the index.
    pub path: String,
    /// `user_desktop` | `public_desktop`
    pub source: &'static str,
    /// What the desktop effectively shows: shortcuts without their `.lnk`
    /// suffix, everything else with its real file name (see DECISIONS D10).
    pub display_name: String,
    /// `file` | `folder` | `shortcut`
    pub kind: &'static str,
    /// Lowercased extension without the dot; `None` for folders.
    pub ext: Option<String>,
    pub size_bytes: Option<i64>,
    /// Epoch milliseconds.
    pub modified_at: Option<i64>,
}

/// Scan one desktop folder (top level). Missing/unreadable dirs yield an
/// empty result rather than an error — the index simply reflects reality.
pub fn scan_desktop_dir(dir: &Path, source: &'static str) -> Vec<ScannedItem> {
    let mut items = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            tracing::warn!(dir = %dir.display(), %err, "cannot read desktop folder");
            return items;
        }
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(meta) = entry.metadata() else {
            // Vanished between read_dir and metadata — skip silently, the
            // next scan will converge.
            continue;
        };
        if is_hidden_or_system(&meta) {
            continue;
        }

        let path = entry.path();
        let ext = Path::new(&name)
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase());
        let kind = classify(&meta, ext.as_deref());
        let display_name = match kind {
            "shortcut" => Path::new(&name)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or(name.clone()),
            _ => name,
        };
        let size_bytes = if meta.is_file() {
            Some(i64::try_from(meta.len()).unwrap_or(i64::MAX))
        } else {
            None
        };
        let modified_at = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));

        items.push(ScannedItem {
            path: path.to_string_lossy().into_owned(),
            source,
            display_name,
            kind,
            ext: ext.filter(|e| !e.is_empty() && kind != "folder"),
            size_bytes,
            modified_at,
        });
    }
    items
}

/// Explorer-classification: `.lnk`/`.url` are shortcuts, directories are
/// folders, everything else is a file.
fn classify(meta: &std::fs::Metadata, ext: Option<&str>) -> &'static str {
    if meta.is_dir() {
        "folder"
    } else if matches!(ext, Some("lnk" | "url")) {
        "shortcut"
    } else {
        "file"
    }
}

/// Skip what the desktop does not normally show (desktop.ini, Thumbs.db, …).
#[cfg(windows)]
fn is_hidden_or_system(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_SYSTEM};
    let attrs = meta.file_attributes();
    attrs & (FILE_ATTRIBUTE_HIDDEN.0 | FILE_ATTRIBUTE_SYSTEM.0) != 0
}

#[cfg(not(windows))]
fn is_hidden_or_system(meta: &std::fs::Metadata) -> bool {
    meta.file_name().starts_with('.') // ignore test-only platforms
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::discovery::USER_DESKTOP;
    use std::fs;

    #[test]
    fn classifies_files_folders_and_shortcuts() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("资料夹")).unwrap();
        fs::write(tmp.path().join("笔记.TXT"), "hi").unwrap();
        fs::write(tmp.path().join("程序.lnk"), "fake").unwrap();

        let items = scan_desktop_dir(tmp.path(), USER_DESKTOP);
        let by_name = |n: &str| items.iter().find(|i| i.path.ends_with(n)).unwrap();

        let folder = by_name("资料夹");
        assert_eq!(folder.kind, "folder");
        assert_eq!(folder.ext, None);
        assert_eq!(folder.display_name, "资料夹");

        let file = by_name("笔记.TXT");
        assert_eq!(file.kind, "file");
        assert_eq!(file.ext.as_deref(), Some("txt"));
        assert_eq!(file.display_name, "笔记.TXT");
        assert_eq!(file.size_bytes, Some(2));

        let shortcut = by_name("程序.lnk");
        assert_eq!(shortcut.kind, "shortcut");
        assert_eq!(shortcut.display_name, "程序");
        assert_eq!(shortcut.ext.as_deref(), Some("lnk"));
        assert_eq!(shortcut.source, USER_DESKTOP);
    }

    #[cfg(windows)]
    #[test]
    fn skips_hidden_and_system_entries() {
        use std::os::windows::fs::OpenOptionsExt;
        use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN;

        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("普通.txt"), "x").unwrap();
        fs::OpenOptions::new()
            .write(true)
            .create(true)
            .custom_flags(FILE_ATTRIBUTE_HIDDEN.0)
            .open(tmp.path().join("secret.ini"))
            .unwrap();

        let items = scan_desktop_dir(tmp.path(), USER_DESKTOP);
        assert_eq!(items.len(), 1);
        assert!(items[0].path.ends_with("普通.txt"));
    }

    #[test]
    fn missing_dir_yields_empty_result() {
        assert!(scan_desktop_dir(Path::new("Z:\\definitely\\missing"), USER_DESKTOP).is_empty());
    }
}
