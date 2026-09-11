//! Custom background image: stored once in the app data dir and served to
//! the webview through the `bg://` custom protocol (see lib.rs).

use base64::Engine;
use std::path::PathBuf;
use tauri::State;

use crate::app::error::{AppError, AppResult};
use crate::app::logging::now_millis;
use crate::app::state::{lock_db, AppState};

/// Fixed file name in the app data dir; mime is sniffed when serving.
const BG_FILE: &str = "background.img";
/// 15 MB decoded — far above any sane wallpaper, catches accidental dumps.
const MAX_BYTES: usize = 15 * 1024 * 1024;

const ALLOWED_MIMES: &[&str] = &["image/png", "image/jpeg", "image/webp"];

/// Store a background image (base64) and make sure the settings key exists
/// with a default opacity. Replaces any previous image.
#[tauri::command]
pub fn background_set(state: State<'_, AppState>, data_b64: String, mime: String) -> AppResult<()> {
    let bytes = decode_image(&data_b64, &mime)?;
    std::fs::write(state.data_dir.join(BG_FILE), &bytes)?;
    tracing::info!(bytes = bytes.len(), mime, "background image stored");
    ensure_background_setting(&state)

}

#[tauri::command]
pub fn background_clear(state: State<'_, AppState>) -> AppResult<()> {
    let path = state.data_dir.join(BG_FILE);
    match std::fs::remove_file(&path) {
        Ok(()) => tracing::info!(path = %path.display(), "background image removed"),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err.into()),
    }
    let mut db = lock_db(&state)?;
    crate::storage::settings_repo::SettingsRepo::new(db.conn()).delete("ui.background")?;
    Ok(())
}


// --- Wallpaper library + auto-switch (M14) --------------------------------

/// Directory (under the app data dir) holding the wallpaper library.
const LIB_DIR: &str = "wallpapers";

/// Library files are named `<millis>.img` by us; accepting only digits and
/// one dots suffix blocks path traversal through the `name` parameter and
/// the `bg://library/<name>` protocol route.
pub(crate) fn valid_lib_name(name: &str) -> bool {
    match name.strip_suffix(".img") {
        // `<millis>.img` — the stem must be non-empty digits only, which
        // also rejects traversal (`..img`, `a/b.img`, `x.img.exe`).
        Some(stem) => {
            !stem.is_empty() && stem.len() <= 22 && stem.bytes().all(|b| b.is_ascii_digit())
        }
        None => false,
    }
}

fn lib_dir(state: &AppState) -> PathBuf {
    state.data_dir.join(LIB_DIR)
}

fn decode_image(data_b64: &str, mime: &str) -> AppResult<Vec<u8>> {
    if !ALLOWED_MIMES.contains(&mime) {
        return Err(AppError::Other(format!(
            "不支持的图片格式：{mime}（仅 PNG/JPEG/WebP）"
        )));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_b64.as_bytes())
        .map_err(|_| AppError::Other("图片数据不是有效的 base64".into()))?;
    if bytes.is_empty() {
        return Err(AppError::Other("图片内容为空".into()));
    }
    if bytes.len() > MAX_BYTES {
        return Err(AppError::Other("图片过大（解码后超过 15MB）".into()));
    }
    Ok(bytes)
}

/// Ensure `ui.background` exists with a default opacity (shared by set/apply).
fn ensure_background_setting(state: &State<'_, AppState>) -> AppResult<()> {
    let mut db = lock_db(state)?;
    let repo = crate::storage::settings_repo::SettingsRepo::new(db.conn());
    if repo.get("ui.background")?.is_none() {
        repo.set("ui.background", &serde_json::json!({ "opacity": 0.35 }))?;
    }
    Ok(())
}

/// Add an image to the wallpaper library (does NOT change the current
/// background). Returns the generated library file name.
#[tauri::command]
pub fn background_library_add(
    state: State<'_, AppState>,
    data_b64: String,
    mime: String,
) -> AppResult<String> {
    let bytes = decode_image(&data_b64, &mime)?;
    let dir = lib_dir(&state);
    std::fs::create_dir_all(&dir)?;
    let name = format!("{}.img", now_millis());
    std::fs::write(dir.join(&name), &bytes)?;
    tracing::info!(name, bytes = bytes.len(), "wallpaper added to library");
    Ok(name)
}

/// List the wallpaper library (sorted file names, oldest first).
#[tauri::command]
pub fn background_library_list(state: State<'_, AppState>) -> AppResult<Vec<String>> {
    let mut names: Vec<String> = std::fs::read_dir(lib_dir(&state))
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .filter(|n| valid_lib_name(n))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    Ok(names)
}

/// Make a library image the current background (copies over background.img;
/// the library entry itself is never touched).
#[tauri::command]
pub fn background_apply(state: State<'_, AppState>, name: String) -> AppResult<()> {
    if !valid_lib_name(&name) {
        return Err(AppError::Other("无效的壁纸文件名".into()));
    }
    let bytes = std::fs::read(lib_dir(&state).join(&name))
        .map_err(|_| AppError::Other("壁纸文件不存在".into()))?;
    std::fs::write(state.data_dir.join(BG_FILE), &bytes)?;
    tracing::info!(name, bytes = bytes.len(), "wallpaper applied from library");
    ensure_background_setting(&state)
}

/// Remove one image from the library. Removing the currently applied image
/// keeps the background as-is (a copy lives in background.img).
#[tauri::command]
pub fn background_library_remove(state: State<'_, AppState>, name: String) -> AppResult<()> {
    if !valid_lib_name(&name) {
        return Err(AppError::Other("无效的壁纸文件名".into()));
    }
    let path = lib_dir(&state).join(&name);
    match std::fs::remove_file(&path) {
        Ok(()) => tracing::info!(name, "wallpaper removed from library"),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err.into()),
    }
    Ok(())
}

/// Mime sniffed from magic bytes (the stored file keeps a neutral name).
pub fn sniff_mime(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        "image/png"
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}

/// App data maintenance: destructive purges, each preceded by a DB backup.
/// `kind` is "collections" or "all" (all also clears settings + index +
/// background; the index rebuilds itself on the next scan).
#[tauri::command]
pub fn appdata_purge(state: State<'_, AppState>, kind: String) -> AppResult<()> {
    let db_path = state.data_dir.join(crate::app::state::DB_FILE_NAME);
    if kind != "collections" && kind != "all" {
        return Err(AppError::Other(format!("未知的清理范围：{kind}")));
    }
    {
        let db = lock_db(&state)?;
        db.checkpoint()?;
        let backup_dir = state.data_dir.join("backups");
        std::fs::create_dir_all(&backup_dir)?;
        let target = backup_dir.join(format!("desktopmanager-prepurge-{}.db", now_millis()));
        std::fs::copy(&db_path, &target)?;
        tracing::info!(backup = %target.display(), "database backed up before purge");
        if kind == "collections" {
            db.purge_collections()?;
        } else {
            db.purge_all()?;
        }
    }
    if kind == "all" {
        let bg = state.data_dir.join(BG_FILE);
        let _ = std::fs::remove_file(bg);
    }
    tracing::info!(kind, "app data purged");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::valid_lib_name;

    #[test]
    fn lib_name_accepts_only_generated_names() {
        assert!(valid_lib_name("1730000000000.img"));
        assert!(!valid_lib_name(""));
        assert!(!valid_lib_name("payload.img/../../secret"));
        assert!(!valid_lib_name("..img"));
        assert!(!valid_lib_name("a.png"));
        assert!(!valid_lib_name("1730000000000.img.exe"));
        assert!(!valid_lib_name("工具.img"));
    }
}
