//! M9 measurement harness — not part of the app, not run in CI.
//!
//! Run with:
//! `cargo test m9_ -- --ignored --nocapture` (from src-tauri)
//!
//! Numbers come from a **debug** build, so they are upper bounds used to
//! spot order-of-magnitude problems, not a release performance claim.
//! docs/PERFORMANCE.md transcribes the latest run.

#![cfg(test)]

use std::fs;
use std::time::Instant;

use crate::desktop::scanner::scan_desktop_dir;
use crate::desktop::discovery::USER_DESKTOP;
use crate::storage::desktop_repo::DesktopRepo;
use crate::storage::Database;

const SIZES: &[usize] = &[0, 50, 200, 500];

fn build_files(dir: &std::path::Path, n: usize) {
    for i in 0..n {
        let ext = match i % 3 {
            0 => "txt",
            1 => "lnk", // realistic mix: shortcuts resolve to bare display names
            _ => "png",
        };
        fs::write(dir.join(format!("文件_{i:03}.{ext}")), "x").unwrap();
    }
}

/// Best-of-N wall time in milliseconds.
fn time_min<F: FnMut()>(runs: usize, mut f: F) -> f64 {
    let mut best = f64::MAX;
    for _ in 0..runs {
        let t = Instant::now();
        f();
        best = best.min(t.elapsed().as_secs_f64() * 1000.0);
    }
    best
}

#[test]
#[ignore = "measurement, not CI: cargo test m9_ -- --ignored --nocapture"]
fn m9_scan_and_db_queries_at_scale() {
    eprintln!(
        "{:>6} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9}",
        "items", "scan", "sync1", "resync", "list", "search", "open"
    );
    eprintln!("{:>6} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9}", "", "ms", "ms", "ms", "ms", "ms", "ms");

    for &n in SIZES {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("perf.db");
        build_files(tmp.path(), n);

        // Full open path the app pays at startup: connection, pragmas,
        // PRAGMA quick_check (D22) and pending migrations.
        let open_ms = time_min(3, || {
            let _db = Database::open_with_recovery(&db_path).unwrap();
        });

        let scan_ms = time_min(3, || {
            let _ = scan_desktop_dir(tmp.path(), USER_DESKTOP);
        });
        let items = scan_desktop_dir(tmp.path(), USER_DESKTOP);

        let mut db = Database::open_with_recovery(&db_path).unwrap().0;
        let sync1_ms = {
            let t = Instant::now();
            DesktopRepo::new(db.conn()).sync_scan(&items).unwrap();
            t.elapsed().as_secs_f64() * 1000.0
        };
        let resync_ms = time_min(3, || {
            DesktopRepo::new(db.conn()).sync_scan(&items).unwrap();
        });
        let list_ms = time_min(5, || {
            let _ = DesktopRepo::new(db.conn()).list_visible().unwrap();
        });
        // Substring hit ~1/10 of items, like a real typed query.
        let search_ms = time_min(5, || {
            let _ = DesktopRepo::new(db.conn()).search("件_2").unwrap();
        });

        eprintln!(
            "{n:>6} {scan_ms:>9.2} {sync1_ms:>9.2} {resync_ms:>9.2} \
             {list_ms:>9.2} {search_ms:>9.2} {open_ms:>9.2}"
        );
    }
}
