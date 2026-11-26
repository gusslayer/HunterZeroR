//! Minimal append-only application log.
//!
//! Keeps actionable diagnostics reachable over SSH/file while systemd captures
//! stderr/journal. Designed for low I/O on microSD.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::Utc;

static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
const MAX_LOG_BYTES: u64 = 256 * 1024;
const LOG_KEEP: usize = 3;

/// Point the logger at `<root>/logs/app.log`.
pub fn init(root: &Path) {
    let path = root.join("logs").join("app.log");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut guard) = LOG_PATH.lock() {
        *guard = Some(path);
    }
}

/// Compatibility wrapper: logs as INFO/app.
pub fn log(message: &str) {
    event("INFO", "app", message);
}

/// Append one actionable log line. Logging must never crash the UI.
pub fn event(level: &str, module: &str, message: &str) {
    let ts = Utc::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{ts}] [{level}] [{module}] {}", sanitize(message));
    eprintln!("{line}");

    let path = match LOG_PATH.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => None,
    };
    let Some(path) = path else { return };
    rotate_if_needed(&path);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

fn sanitize(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    for ch in message.chars() {
        match ch {
            '\n' | '\r' | '\t' => out.push(' '),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out.trim().to_string()
}

fn rotate_if_needed(path: &Path) {
    let Ok(meta) = fs::metadata(path) else { return };
    if meta.len() < MAX_LOG_BYTES {
        return;
    }
    for idx in (1..=LOG_KEEP).rev() {
        let from = rotated_path(path, idx);
        let to = rotated_path(path, idx + 1);
        if idx == LOG_KEEP {
            let _ = fs::remove_file(&from);
        } else if from.exists() {
            let _ = fs::rename(&from, &to);
        }
    }
    let _ = fs::rename(path, rotated_path(path, 1));
}

fn rotated_path(path: &Path, idx: usize) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "app.log".to_string());
    path.with_file_name(format!("{name}.{idx}"))
}
