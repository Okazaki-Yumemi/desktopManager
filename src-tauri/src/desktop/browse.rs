//! Read-only folder browsing for collection references.
//!
//! Expanding a folder shows the immediate children of a real directory.
//! This never writes anything — it is a plain directory listing — and the
//! allowed roots are gated by the same allow-list as opening (indexed
//! desktop paths or collection-held paths).

use serde::Serialize;

use crate::app::error::{AppError, AppResult};

/// Safety cap so a huge directory cannot stall the UI; the UI says "…"
/// beyond this.
const MAX_CHILDREN: usize = 500;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub ext: Option<String>,
    pub size_bytes: Option<i64>,
}

/// Immediate children of `dir`, directories first, then case-insensitive
/// by name. Hidden entries (leading dot) and unreadable entries are skipped.
pub fn list_children(dir: &str) -> AppResult<Vec<PathEntry>> {
    let path = std::path::Path::new(dir);
    if !path.is_absolute() {
        return Err(AppError::Other("路径必须是绝对路径".into()));
    }
    if !path.is_dir() {
        return Err(AppError::Other(format!("不是文件夹：{dir}")));
    }

    let mut entries: Vec<PathEntry> = Vec::new();
    let mut truncated = false;
    let read = std::fs::read_dir(path)
        .map_err(|e| AppError::Other(format!("无法读取文件夹：{e}")))?;
    for entry in read {
        let Ok(entry) = entry else { continue };
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let child_path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        let is_dir = meta.is_dir();
        let ext = child_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());
        let size_bytes = (!is_dir).then_some(meta.len() as i64);
        entries.push(PathEntry {
            path: child_path.to_string_lossy().into_owned(),
            name,
            is_dir,
            ext,
            size_bytes,
        });
        if entries.len() >= MAX_CHILDREN {
            truncated = true;
            break;
        }
    }
    if truncated {
        // A marker entry keeps the cap visible instead of silently hiding
        // content.
        entries.push(PathEntry {
            name: "…（仅显示前 500 项）".into(),
            path: format!("\0truncated:{dir}"),
            is_dir: false,
            ext: None,
            size_bytes: None,
        });
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_dirs_first_sorted_and_skips_hidden() {
        let dir = std::env::temp_dir().join(format!("dm-browse-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("b.txt"), "12345").unwrap();
        std::fs::write(dir.join("A.TXT"), "x").unwrap();
        std::fs::create_dir_all(dir.join("子目录")).unwrap();
        std::fs::create_dir_all(dir.join(".hidden")).unwrap();

        let entries = list_children(&dir.to_string_lossy()).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].name, "子目录");
        assert_eq!(entries[1].name, "A.TXT"); // case-insensitive, dirs first
        assert!(!entries[1].is_dir);
        assert_eq!(entries[1].ext.as_deref(), Some("txt"));
        assert_eq!(entries[1].size_bytes, Some(1));
        assert_eq!(entries[2].size_bytes, Some(5));
        assert!(entries.iter().all(|e| !e.name.starts_with('.')));

        assert!(list_children("relative/path").is_err());
        assert!(list_children(dir.join("b.txt").to_string_lossy().as_ref()).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
