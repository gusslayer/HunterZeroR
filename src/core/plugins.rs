//! Plugin catalog for HunterZeroR.
//! 
//! Mirrors core/plugins.py from the Python version.
//! Gestiona el registro y descubrimiento de módulos (tools).

use std::path::Path;
use std::collections::HashMap;

use crate::core::state::UIState;
use crate::core::system_stats::SystemStats;

/// Identificador de un módulo/plugin.
/// 
/// Igual que `plugin.identifier` en Python.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleIdentifier {
    pub id: String,
    pub name: String,
}

/// Metadatos de un módulo registrado.
/// 
/// Igual que `Plugin` en Python.
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Identificador único (debe coincidir con nombre de carpeta)
    pub identifier: ModuleIdentifier,
    /// Nombre mostrado en el menú
    pub name: String,
    /// Descripción corta
    pub description: String,
    /// Nombre de archivo de icono
    pub icon: String,
}

/// Catálogo de plugins/modules registrados.
/// 
/// Igual que `PluginCatalog` en Python. Descubre módulos escaneando
/// las carpetas bajo un directorio raíz.
pub struct PluginCatalog {
    /// Módulos registrados por identificador
    plugins: HashMap<String, ModuleInfo>,
    /// Errores de módulos no registrados o con pantallas faltantes
    errors: Vec<String>,
}

impl PluginCatalog {
    /// Crear un nuevo catálogo escaneando directorios de módulos.
    /// 
    /// Igual que `PluginCatalog(ROOT / "modules")` en Python.
    pub fn new(modules_dir: &Path) -> Self {
        let mut catalog = Self {
            plugins: HashMap::new(),
            errors: Vec::new(),
        };
        
        // Escanear directorios de módulos
        if let Ok(entries) = std::fs::read_dir(modules_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Buscar manifest.yaml en el directorio
                    let manifest_path = path.join("manifest.yaml");
                    if manifest_path.exists() {
                        if let Ok(manifest_content) = std::fs::read_to_string(&manifest_path) {
                            if let Ok(manifest) = Self::parse_manifest(&manifest_content) {
                                catalog.plugins.insert(manifest.identifier.id.clone(), manifest.clone());
                            } else {
                                catalog.errors.push format!("Error parsing manifest in {:?}", path);
                            }
                        }
                    }
                }
            }
        }
        
        catalog
    }
    
    /// Parsear el contenido YAML del manifiesto.
    /// 
    /// Igual que la lógica en core/app.py que usa yaml.safe_load.
    fn parse_manifest(content: &str) -> Result<ModuleInfo, String> {
        // YAML simple sin dependencia externa
        // Espera formato: id: ..., name: ..., description: ..., icon: ...
        let lines: Vec<&str> = content.lines().filter(|l| !l.starts_with('#')).collect();
        
        let mut id = String::new();
        let mut name = String::new();
        let mut description = String::new();
        let mut icon = String::new();
        
        for line in lines {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("id:") {
                id = rest.trim().to_string();
            } else if let Some(rest) = trimmed.strip_prefix("name:") {
                name = rest.trim().to_string();
            } else if let Some(rest) = trimmed.strip_prefix("description:") {
                description = rest.trim().to_string();
            } else if let Some(rest) = trimmed.strip_prefix("icon:") {
                icon = rest.trim().to_string();
            }
        }
        
        if id.is_empty() {
            return Err("Manifest without id".to_string());
        }
        
        let identifier = ModuleIdentifier {
            id,
            name: name.clone(),
        };
        
        Ok(ModuleInfo {
            identifier,
            name,
            description,
            icon,
        })
    }
    
    /// Obtener todos los módulos registrados.
    pub fn plugins(&self) -> &HashMap<String, ModuleInfo> {
        &self.plugins
    }
    
    /// Obtener errores de módulos no registrados.
    pub fn errors(&self) -> &[String] {
        &self.errors
    }
    
    /// Verificar si un módulo tiene pantalla registrada.
    pub fn has_screen(&self, identifier: &str) -> bool {
        self.plugins.contains_key(identifier)
    }
    
    /// Obtener un módulo por su identificador.
    pub fn get(&self, identifier: &str) -> Option<&ModuleInfo> {
        self.plugins.get(identifier)
    }
}