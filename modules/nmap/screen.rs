//! Módulo Nmap en Rust para HunterZeroR.
//! 
//! Versión mejorada con opciones de checkboxes concatenables.
//! 
//! Implementa el trait ToolScreen para la arquitectura Rust.
//! El contenido se dibuja en el área usable del viewport virtual 320x240.
//! El header y footer se dibujan automáticamente por la aplicación principal.
//! 
//! Este módulo ahora incluye una serie de opciones nmap que se pueden
//! seleccionar mediante checkboxes para construir comandos de escaneo
//! completos y personalizables, y ejecutarlos directamente en Rust.

use core::input::Action;

use ggez::graphics;
use ggez::Context;

// Tema y colores (consistentes con MODULARIZACION.md)
use crate::theme::{BG_COLOR, TEXT_COLOR, ACCENT_COLOR, HEADER_HEIGHT, FOOTER_HEIGHT, CONTENT_AREA_HEIGHT};

// Fase de configuración del módulo Nmap
#[derive(Debug, Clone, PartialEq)]
enum NmapPhase {
    /// Seleccionando modo de escaneo
    SelectingMode,
    /// Modo seleccionando, configurando opciones checkbox
    ConfiguringOptions,
    /// Listo para ejecutar escaneo
    Ready,
}

/// Opción de escaneo Nmap con descripción
#[derive(Debug, Clone, PartialEq)]
enum NmapOption {
    /// Escaneo TCP connect (sT)
    TcpConnect,
    /// Escaneo SYN (sS) - requiere permisos
    SynScan,
    /// Escaneo UDP (sU)
    UdpScan,
    /// Escaneo rápido (-F)
    FastScan,
    /// Detección de versión (-sV)
    VersionDetection,
    /// Detección de SO (-O)
    OSDetection,
    /// Escaneo de puertos específicos (-p)
    PortSpecification,
    /// Escaneo agresivo (-A)
    AggressiveScan,
    /// Plantilla de tiempo - T4 (rápido)
    TimingTemplateT4,
    /// Plantilla de tiempo - T5 (muy rápido)
    TimingTemplateT5,
}

// Estado de una casilla de verificación
#[derive(Debug, Clone, PartialEq)]
struct Checkbox {
    /// Nombre de la opción
    name: &'static str,
    /// Descripción corta
    description: &'static str,
    /// Si está marcada
    checked: bool,
}

/// Estructura de estado del módulo Nmap (migrado a Rust).
/// 
/// Almacena la configuración actual para escaneo de puertos con
/// opciones seleccionables mediante checkboxes y ejecución en Rust.
pub struct NmapScreen {
    /// Modo de escaneo seleccionado
    mode: NmapMode,
    /// Puerto objetivo o rango
    target_port: String,
    /// Dirección IP objetivo
    target_ip: String,
    /// Fase actual de configuración
    phase: NmapPhase,
    /// Resultados acumulados
    results: String,
    /// Opciones de nmap seleccionadas por checkbox
    options: [Checkbox; 10],
    /// Opción actualmente navegada (índice en el array options)
    option_cursor: usize,
}

impl NmapScreen {
    /// Crear una nueva instancia de NmapScreen con valores por defecto.
    /// 
    /// Valores por defecto: Modo TCP connect, puerto vacío, IP vacía,
    /// todas las opciones desmarcadas.
    pub fn new() -> Self {
        Self {
            mode: NmapMode::TcpConnect,
            target_port: String::new(),
            target_ip: String::new(),
            phase: NmapPhase::SelectingMode,
            results: String::new(),
            // Inicializar checkboxes con descripciones de opciones nmap
            options: [
                Checkbox {
                    name: "sT TCP Connect",
                    description: "Escaneo TCP conectar - más seguro, funciona en cualquier sistema",
                    checked: false,
                },
                Checkbox {
                    name: "sS SYN Scan",
                    description: "Escaneo SYN - stealth, requiere permisos de root",
                    checked: false,
                },
                Checkbox {
                    name: "sU UDP Scan",
                    description: "Escaneo UDP - detección puertos abiertos UDP",
                    checked: false,
                },
                Checkbox {
                    name: "-F Fast Scan",
                    description: "Escaneo rápido - fewer ports, veloz pero menos completo",
                    checked: false,
                },
                Checkbox {
                    name: "-sV Version",
                    description: "Detección de versión de servicio - qué software hay en los puertos",
                    checked: false,
                },
                Checkbox {
                    name: "-O OS Detect",
                    description: "Detección del sistema operativo remoto",
                    checked: false,
                },
                Checkbox {
                    name: "-p Puerto",
                    description: "Especificar puertos específicos (ej. -p 80,443 o -p 1-1000)",
                    checked: false,
                },
                Checkbox {
                    name: "-A Aggressive",
                    description: "Escaneo agresivo - version + OS + script traceroute",
                    checked: false,
                },
                Checkbox {
                    name: "-T4 Timing",
                    description: "Plantilla T4 - más rápido, menos sospechoso que T5",
                    checked: false,
                },
                Checkbox {
                    name: "-T5 Aggressive",
                    description: "Plantilla T5 - muy rápido, más detectable",
                    checked: false,
                },
            ],
            option_cursor: 0,
        }
    }

    /// Ejecutar comando nmap con las opciones seleccionadas.
    /// 
    /// Construye y ejecuta el comando nmap directamente en Rust usando
    /// std::process::Command. Captura la salida estándar y de error.
    /// 
    /// NOTA: nmap requiere permisos de root/sudo en la mayoría de los escaneos.
    /// En Raspberry Pi Zero, ejecútalo como root o configura permisos adecuados.
    fn execute_nmap_scan(&mut self) {
        // Construir argumentos de nmap basados en checkboxes marcados
        let mut args: Vec<&str> = Vec::new();
        
        // Agregar modo de escaneo (siempre seleccionado uno)
        let mode_flag = match self.mode {
            NmapMode::TcpConnect => "-sT",
            NmapMode::SynScan => "-sS",
            NmapMode::UdpScan => "-sU",
            NmapMode::FastScan => "-F",
            NmapMode::Custom => "-sT",
        };
        args.push(mode_flag);
        
        // Agregar opciones adicionales marcadas
        for checkbox in self.options.iter() {
            if checkbox.checked {
                match checkbox.name {
                    "sS SYN Scan" => args.push("-sS"),
                    "sU UDP Scan" => args.push("-sU"),
                    "-F Fast Scan" => args.push("-F"),
                    "-sV Version" => args.push("-sV"),
                    "-O OS Detect" => args.push("-O"),
                    "-p Puerto" => {
                        // Puerto objetivo si está definido
                        if !self.target_port.is_empty() {
                            args.push(&format!("-p {}", self.target_port));
                        }
                    }
                    "-A Aggressive" => args.push("-A"),
                    "-T4 Timing" => args.push("-T4"),
                    "-T5 Aggressive" => args.push("-T5"),
                    _ => {}
                }
            }
        }
        
        // Agregar IP objetivo como argumento final
        if !self.target_ip.is_empty() {
            args.push(&self.target_ip);
        }
        
        // Ejecutar nmap como subprocess en Rust
        use std::process::{Command, Stdio};
        
        let output = Command::new("nmap")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();
        
        match output {
            Ok(result) => {
                // Construir mensaje de resultados
                let output_str = String::from_utf8_lossy(&result.stdout);
                let err_str = String::from_utf8_lossy(&result.stderr);
                
                // Limitar tamaño de salida para UI
                let mut results = String::new();
                if !output_str.is_empty() {
                    results.push_str(&format!("STDOUT:\n{}\n", output_str.lines().take(10).collect::<Vec<&str>>().join("\n")));
                }
                if !err_str.is_empty() {
                    // No mostramos el mensaje de "Root privileges required" como error
                    if err_str.trim() != "nmap: Root privileges required for many scans." {
                        results.push_str(&format!("STDERR:\n{}\n", err_str.lines().take(5).collect::<Vec<&str>>().join("\n")));
                    }
                }
                
                // Agregar información de estado
                results.push_str(&format!("\nEscaneo nmap completado con {} argumentos", args.len()));
                results.push_str(&format!("\nModo: {:?}", self.mode));
                if !self.target_ip.is_empty() {
                    results.push_str(&format!("\nIP objetivo: {}", self.target_ip));
                }
                if !self.target_port.is_empty() {
                    results.push_str(&format!("\nPuertos: {}", self.target_port));
                }
                
                self.results = results;
            }
            Err(e) => {
                self.results = format!("Error ejecutando nmap: {}", e);
            }
        }
        
        // Regresa a fase de selección
        self.phase = NmapPhase::SelectingMode;
    }
    
    /// Limpiar recursos al cerrar esta pantalla.
    fn close(&mut self) {
        // Limpiar estado del módulo Nmap
        self.target_ip = String::new();
        self.target_port = String::new();
        self.phase = NmapPhase::SelectingMode;
        self.results = String::new();
        // Resetear todas las opciones
        for opt in self.options.iter_mut() {
            opt.checked = false;
        }
        self.option_cursor = 0;
    }
}