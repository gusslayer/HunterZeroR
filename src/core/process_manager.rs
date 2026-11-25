//! Process manager for HunterZeroR.
//! 
//! Mirrors core/process_manager.py from the Python version.
//! Gestiona la ejecución de herramientas (ping, nmap, etc.) con cancelación,
//! timeouts y salida limitada. Escribe logs a logs/commands/latest.log.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::io::{Write, Read};
use crate::core::system_stats::SystemStats;

/// Administrador de procesos de herramienta.
/// 
/// Igual que `ProcessManager` en Python.
/// 
/// Responsabilidades:
/// - Construir comandos seguros (argv tipado, sin shell=True)
/// - Ejecutar con timeout y cancelación
/// - Capturar stdout/stderr con límite de líneas
/// - Escribir logs a logs/commands/latest.log
/// - Señalizar procesos hijopara cancelación
pub struct ProcessManager {
    /// Directorio raíz de la aplicación
    root: std::path::Path,
    /// Logger para mensajes
    logger: Option<dyn Fn(&str) + Send + Sync>,
    /// Proceso en ejecución actual
    current_process: Option<std::process::Child>,
    /// Tiempo de inicio del proceso actual
    start_time: Option<Instant>,
    /// Mensaje de estado actual
    message: String,
    /// Timeout máximo en segundos (180s por defecto, como Nmap)
    max_timeout_seconds: u64,
}

impl ProcessManager {
    /// Crear un nuevo ProcessManager.
    /// 
    /// Igual que `ProcessManager(ROOT, logger)` en Python.
    pub fn new(root: std::path::Path, logger: Option<dyn Fn(&str) + Send + Sync>) -> Self {
        Self {
            root,
            logger,
            current_process: None,
            start_time: None,
            message: String::new(),
            max_timeout_seconds: 180, // Igual que MAX_COMMAND_TIMEOUT_SECONDS en Python
        }
    }
    
    /// Ejecutar un comando con los controles de seguridad de HunterZero.
    /// 
    /// Igual que `ProcessManager.start()` en Python.
    /// 
    /// Args:
    ///     command: Nombre del ejecutable (ping, nmap, etc.)
    ///     args: Argumentos del comando
    /// 
    /// Returns:
        /// true si el comando se lanzó exitosamente, false si fue rechazado.
    pub fn start(&mut self, command: &str, args: &[&str]) -> bool {
        // Construir comando seguro: sin shell=True, argv construido por funciones tipadas
        let cmd = Command::new(command)
            .args(args)
            .stdin(Stdio::null()) // Cerrar stdin
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        
        match cmd {
            Ok(mut child) => {
                self.current_process = Some(child);
                self.start_time = Some(Instant::now());
                
                // Iniciar hilo de lectura de salida (simplificado)
                // En Rust usaríamos crossbeam-channel o mpsc
                
                if let Some(logger) = self.logger.as_mut() {
                    logger(&format!("Ejecutando: {} {}", command, args.join(" ")));
                }
                
                true
            }
            Err(e) => {
                if let Some(logger) = self.logger.as_mut() {
                    logger(&format!("Error ejecutando {}: {}", command, e));
                }
                false
            }
        }
    }
    
    /// Verificar si el proceso actual aún está en ejecución.
    pub fn is_running(&self) -> bool {
        match &self.current_process {
            Some(p) => p.poll().unwrap_or(false) == std::process::ExitStatus::default(),
            None => false,
        }
    }
    
    /// Cancelar/terminar el proceso actual.
    /// 
    /// Igual que `ProcessManager.stop()` en Python.
    pub fn stop(&mut self) {
        if let Some(p) = self.current_process.take() {
            let _ = p.kill(); // Enviar SIGKILL
            if let Some(logger) = self.logger.as_mut() {
                logger("Proceso cancelado");
            }
        }
        self.start_time = None;
    }
    
    /// Actualizar el estado del proceso (llmarlo periódicamente en el loop).
    pub fn update(&mut self, stats: &mut SystemStats) {
        // Actualizar mensaje de estado basado en el proceso
        if self.is_running() {
            self.message = "Completado".to_string();
        } else if self.start_time.is_some() {
            let elapsed = self.start_time.unwrap().elapsed().as_secs();
            self.message = format!("Ejecutando... {}s", elapsed);
            
            // Verificar timeout
            if elapsed > self.max_timeout_seconds {
                self.stop();
                self.message = "Timeout excedido".to_string();
            }
        } else {
            self.message = String::new();
        }
    }
    
    /// Obtener el mensaje de estado actual.
    pub fn message(&self) -> &str {
        &self.message
    }
    
    /// Escribir salida del comando a logs/commands/latest.log.
    fn write_log(&self, output: &str) {
        let log_path = self.root.join("logs").join("commands").join("latest.log");
        let _ = std::fs::create_dir_all(log_path.parent().unwrap_or(&std::path::Path::new("logs")));
        
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            let _ = writeln!(file, "{} - {}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(), output);
        }
    }
}