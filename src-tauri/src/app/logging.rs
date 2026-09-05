use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use tracing_appender::non_blocking::WorkerGuard;

use super::error::AppResult;

/// Keeps the async log writer alive. Managed as Tauri state so it is dropped
/// (and flushed) on application shutdown. The inner guard is intentionally
/// never read — holding it IS the feature.
#[allow(dead_code)]
pub struct LogGuard(WorkerGuard);

/// Current unix time in milliseconds; falls back to 0 rather than panicking.
pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Initialize `tracing` with a daily-rotated file sink plus stdout in debug
/// builds. Old log files are pruned so retention stays bounded.
pub fn init<M: tauri::Manager<R>, R: tauri::Runtime>(manager: &M) -> AppResult<LogGuard> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{EnvFilter, Layer};

    let log_dir = manager.path().app_data_dir()?.join("logs");
    fs::create_dir_all(&log_dir)?;
    if let Err(err) = prune_old_logs(&log_dir, KEEP_LOG_DAYS) {
        eprintln!("log retention cleanup failed: {err}");
    }

    let app_filter = if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    };
    let file_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("info,desktop_manager_lib={app_filter}")));

    let (writer, guard) = tracing_appender::non_blocking(tracing_appender::rolling::daily(
        &log_dir,
        "desktopmanager.log",
    ));
    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(writer)
        .with_filter(file_filter);

    let stdout_layer = if cfg!(debug_assertions) {
        Some(
            tracing_subscriber::fmt::layer().with_filter(EnvFilter::new(format!(
                "info,desktop_manager_lib={app_filter}"
            ))),
        )
    } else {
        None
    };

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .init();

    tracing::debug!(log_dir = %log_dir.display(), "logging initialized");
    Ok(LogGuard(guard))
}

const KEEP_LOG_DAYS: u32 = 14;

/// Delete rotated log files older than `keep_days`. Best effort: failures to
/// remove individual files are ignored.
pub fn prune_old_logs(log_dir: &Path, keep_days: u32) -> std::io::Result<()> {
    let cutoff = now_millis() - (i64::from(keep_days) * 24 * 60 * 60 * 1000);
    for entry in fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let modified = entry
            .metadata()?
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64);
        if let Some(m) = modified {
            if m < cutoff {
                let _ = fs::remove_file(&path);
            }
        }
    }
    Ok(())
}
