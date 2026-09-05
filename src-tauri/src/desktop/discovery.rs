//! Desktop folder discovery via the Windows known-folder API.
//!
//! `FOLDERID_DESKTOP` is the user's (possibly redirected) desktop — on this
//! machine `D:\Desktop`; `FOLDERID_PUBLICDESKTOP` is the shared desktop. The
//! desktop view the user sees is the union of both, so both are indexed and
//! tagged with their origin.

use std::path::{Path, PathBuf};

use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_PublicDesktop, SHGetKnownFolderPath, KNOWN_FOLDER_FLAG,
};

/// `desktop_items.source` value for items in the user's own desktop folder.
pub const USER_DESKTOP: &str = "user_desktop";
/// `desktop_items.source` value for items in the all-users (public) desktop.
pub const PUBLIC_DESKTOP: &str = "public_desktop";

/// One folder to index. `source` is stored verbatim in `desktop_items.source`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopSource {
    pub root: PathBuf,
    pub source: &'static str,
}

/// Resolve the folders to index. Never fails: if the shell API cannot answer,
/// environment/default paths are used so the app stays useful (fallback-first).
pub fn discover_desktop_sources() -> Vec<DesktopSource> {
    let mut sources = Vec::new();

    let user = known_folder(&FOLDERID_Desktop)
        .or_else(fallback_user_desktop)
        .map(|root| DesktopSource {
            root,
            source: USER_DESKTOP,
        });
    let public = known_folder(&FOLDERID_PublicDesktop).map(|root| DesktopSource {
        root,
        source: PUBLIC_DESKTOP,
    });

    for candidate in [user, public].into_iter().flatten() {
        // Public items may already be covered when the user desktop is the
        // merged view; keep the user source on conflicts.
        if sources
            .iter()
            .any(|s: &DesktopSource| same_dir(&s.root, &candidate.root))
        {
            tracing::debug!(
                dir = %candidate.root.display(),
                "public desktop equals another source, skipping"
            );
            continue;
        }
        tracing::info!(dir = %candidate.root.display(), source = candidate.source, "desktop source discovered");
        sources.push(candidate);
    }
    sources
}

/// Call SHGetKnownFolderPath and free the returned CoTaskMem buffer.
fn known_folder(folder: &windows::core::GUID) -> Option<PathBuf> {
    unsafe {
        // flags 0: current-user value, no redirection games, no default-on-fail
        let path: windows::core::PWSTR =
            SHGetKnownFolderPath(folder, KNOWN_FOLDER_FLAG(0), None).ok()?;
        let parsed = path.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(path.0.cast()));
        parsed
    }
}

/// Last-resort user desktop if the shell API is unavailable.
fn fallback_user_desktop() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(|profile| Path::new(&profile).join("Desktop"))
}

/// True when both paths point at the same existing directory (best effort).
fn same_dir(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn discovers_at_least_the_user_desktop_and_it_exists() {
        let sources = discover_desktop_sources();
        assert!(!sources.is_empty(), "no desktop source discovered");
        assert_eq!(sources[0].source, USER_DESKTOP);
        for s in &sources {
            assert!(
                s.root.is_dir(),
                "discovered source is not a dir: {:?}",
                s.root
            );
        }
    }
}
