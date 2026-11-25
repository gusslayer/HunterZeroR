//! State management for HunterZeroR.
//! 
//! Mirrors core/state.py from the Python version.
//! Gestiona el estado UI (menú, módulo en ejecución, resultado) y persiste la selección.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Estado de la interfaz de usuario.
/// 
/// Igual que `UIState` en Python, pero con serialización JSON para persistencia.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIState {
    /// Índice del módulo seleccionado en el menú
    pub selected: usize,
    /// Última acción o resultado guardado
    pub last_action: String,
    /// Configuración de preferencias del usuario
    pub preferences: serde_json::Value,
}

impl UIState {
    /// Crear un nuevo estado UI.
    pub fn new() -> Self {
        Self {
            selected: 0,
            last_action: String::new(),
            preferences: serde_json::json!({}),
        }
    }

    /// Cargar el estado desde un archivo JSON.
    pub fn load_from_file(path: &Path) -> std::io::Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let state: Self = serde_json::from_str(&content)?;
            Ok(state)
        } else {
            Ok(Self::new())
        }
    }

    /// Guardar el estado en un archivo JSON.
    pub fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        path.parent().map(|p| std::fs::create_dir_all(p)).transpose()?;
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }
}

/// Cargar el estado desde la ruta por defecto.
/// 
/// Igual que `UIState(ROOT / "state" / "ui_state.json").load()` en Python.
pub fn load_default_state(root: &Path) -> std::io::Result<Self> {
    let path = root.join("state").join("ui_state.json");
    UIState::load_from_file(&path)
}

/// Guardar el estado en la ruta por defecto.
pub fn save_default_state(state: &UIState, root: &Path) -> std::io::Result<()> {
    let path = root.join("state").join("ui_state.json");
    state.save_to_file(&path)
}