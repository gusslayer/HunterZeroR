//! Persisted UI state.
//!
//! Only the menu selection is essential. The file is deliberately forgiving: a
//! corrupt or partially written state file must never prevent the kiosk from
//! starting.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct UIState {
    pub selected: usize,
}

impl UIState {
    /// Load the state or return the default when the file is missing/invalid.
    pub fn load(path: &Path, module_count: usize) -> Self {
        let mut state = std::fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str::<UIState>(&content).ok())
            .unwrap_or_default();
        if module_count > 0 {
            state.selected = state.selected.min(module_count - 1);
        } else {
            state.selected = 0;
        }
        state
    }

    /// Atomically save the state so a power cut cannot leave a partial file.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let temporary = path.with_extension("tmp");
        std::fs::write(&temporary, json)?;
        match std::fs::rename(&temporary, path) {
            Ok(()) => Ok(()),
            Err(_) if path.exists() => {
                // Windows does not replace an existing file with rename().
                std::fs::remove_file(path)?;
                std::fs::rename(&temporary, path)
            }
            Err(error) => Err(error),
        }
    }
}

/// Default location used by the application.
pub fn default_state_path(root: &Path) -> PathBuf {
    root.join("state").join("ui_state.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_index_is_clamped() {
        let path = std::env::temp_dir().join("hz_state_test.json");
        std::fs::write(&path, r#"{"selected": 99}"#).unwrap_or_default();
        let state = UIState::load(&path, 7);
        assert_eq!(state.selected, 6);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn corrupt_state_falls_back_to_default() {
        let path = std::env::temp_dir().join("hz_state_corrupt.json");
        std::fs::write(&path, "{not json").unwrap_or_default();
        let state = UIState::load(&path, 7);
        assert_eq!(state.selected, 0);
        let _ = std::fs::remove_file(path);
    }
}
