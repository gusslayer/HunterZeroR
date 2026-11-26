//! Launch the small, explicitly allowlisted set of background diagnostics.
//!
//! Only `ping`, `nmap` and `nslookup` may be started by a module. The update
//! flow is a separate, fixed entry point. Output is captured to a file and the
//! UI reads a bounded tail for display.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::core::logging;

const ALLOWED_COMMANDS: &[&str] = &["ping", "nmap", "nslookup"];
pub const UPDATE_LABEL: &str = "Actualizacion de HunterZero";
pub const UPDATE_DEPENDENCY_HINT: &str =
    "Si falta una dependencia, ejecuta: sudo bash deploy/deps.sh.";
pub const MAX_COMMAND_TIMEOUT_SECONDS: u64 = 180;
const MAX_DISPLAY_OUTPUT: usize = 8192;
const MAX_DISPLAY_LINES: usize = 24;

struct RunningCommand {
    label: String,
    child: Child,
    started: Instant,
    timeout: Duration,
    output_path: PathBuf,
    last_output_read: Instant,
}

pub struct ProcessManager {
    root: PathBuf,
    running: Option<RunningCommand>,
    pub message: String,
    pub output: String,
}

impl ProcessManager {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            running: None,
            message: String::new(),
            output: String::new(),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.is_some()
    }

    pub fn running_label(&self) -> Option<&str> {
        self.running.as_ref().map(|task| task.label.as_str())
    }

    /// Start a validated ping, Nmap or DNS lookup argv with captured output.
    pub fn start_command(&mut self, label: &str, command: &[String], timeout_seconds: u64) -> bool {
        let Some(program) = command.first() else {
            self.message = "Comando no permitido.".to_string();
            return false;
        };
        if !ALLOWED_COMMANDS.contains(&program.as_str()) {
            self.message = "Comando no permitido.".to_string();
            logging::log("Se bloqueo el intento de lanzar un comando no permitido.");
            return false;
        }
        self.launch(
            label,
            command,
            timeout_seconds,
            format!("No esta instalado {program}; instala ping, nmap o dnsutils."),
            None,
        )
    }

    /// Run `deploy/update.sh` with the same guarded launcher as tool commands.
    pub fn start_update(&mut self) -> bool {
        let script = self.root.join("deploy").join("update.sh");
        if !script.is_file() {
            self.message = format!("No existe {}.", script.display());
            return false;
        }
        let command = vec!["bash".to_string(), script.to_string_lossy().to_string()];
        self.launch(
            UPDATE_LABEL,
            &command,
            MAX_COMMAND_TIMEOUT_SECONDS,
            "No esta instalado bash; no se puede actualizar ahora.".to_string(),
            Some(("HUNTERZERO_UPDATE_GUI", "1")),
        )
    }

    fn launch(
        &mut self,
        label: &str,
        command: &[String],
        timeout_seconds: u64,
        missing_message: String,
        environment: Option<(&str, &str)>,
    ) -> bool {
        if self.is_running() {
            self.message = "Ya hay una herramienta en ejecucion.".to_string();
            return false;
        }
        if !(1..=MAX_COMMAND_TIMEOUT_SECONDS).contains(&timeout_seconds) {
            self.message = "Tiempo limite fuera del rango permitido.".to_string();
            return false;
        }
        if command
            .iter()
            .any(|argument| argument.is_empty() || argument.contains('\0'))
        {
            self.message = "Argumentos de comando invalidos.".to_string();
            return false;
        }
        let Some(executable) = find_executable(&command[0]) else {
            self.message = missing_message;
            logging::log(&format!(
                "No se encontro el ejecutable permitido {}.",
                command[0]
            ));
            return false;
        };

        let output_path = self.root.join("logs").join("commands").join("latest.log");
        if let Some(parent) = output_path.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                self.message = format!("No se pudo crear el log de comandos: {error}");
                return false;
            }
        }
        let output_file = match File::create(&output_path) {
            Ok(file) => file,
            Err(error) => {
                self.message = format!("No se pudo abrir el log de comandos: {error}");
                return false;
            }
        };
        let stderr_file = match output_file.try_clone() {
            Ok(file) => file,
            Err(error) => {
                self.message = format!("No se pudo duplicar el log de comandos: {error}");
                return false;
            }
        };

        let mut process = Command::new(&executable);
        process
            .args(&command[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::from(output_file))
            .stderr(Stdio::from(stderr_file));
        if let Some((key, value)) = environment {
            process.env(key, value);
        }
        match process.spawn() {
            Ok(child) => {
                self.output.clear();
                self.message = format!("Ejecutando {label}...");
                logging::log(&format!("Se inicio {label} (pid {}).", child.id()));
                self.running = Some(RunningCommand {
                    label: label.to_string(),
                    child,
                    started: Instant::now(),
                    timeout: Duration::from_secs(timeout_seconds),
                    output_path,
                    last_output_read: Instant::now(),
                });
                true
            }
            Err(error) => {
                self.message = format!("No se pudo iniciar {label}: {error}");
                logging::log(&format!("No se pudo iniciar {label}: {error}"));
                false
            }
        }
    }

    /// Collect completed output or stop the command when its deadline expires.
    pub fn update(&mut self) {
        let Some(task) = self.running.as_mut() else {
            return;
        };

        if task.started.elapsed() >= task.timeout {
            let label = task.label.clone();
            let output_path = task.output_path.clone();
            kill_child(task);
            self.output = read_output(&output_path);
            self.message = format!("Tiempo agotado: {label}.");
            if label == UPDATE_LABEL {
                self.message.push(' ');
                self.message.push_str(UPDATE_DEPENDENCY_HINT);
            }
            logging::log(&format!("Se agoto el tiempo de {label}."));
            self.running = None;
            return;
        }

        match task.child.try_wait() {
            Ok(None) => {
                if task.last_output_read.elapsed() >= Duration::from_millis(250) {
                    self.output = read_output(&task.output_path);
                    task.last_output_read = Instant::now();
                }
            }
            Ok(Some(status)) => {
                let label = task.label.clone();
                self.output = read_output(&task.output_path);
                if status.success() {
                    self.message = format!("{label}: finalizado correctamente.");
                    logging::log(&format!("{label} finalizo correctamente."));
                } else {
                    self.message = format!(
                        "{label}: termino con error ({}).",
                        status.code().unwrap_or(-1)
                    );
                    if label == UPDATE_LABEL {
                        self.message.push(' ');
                        self.message.push_str(UPDATE_DEPENDENCY_HINT);
                    }
                    logging::log(&self.message.clone());
                }
                self.running = None;
            }
            Err(error) => {
                self.message = format!("No se pudo consultar el proceso: {error}");
                self.running = None;
            }
        }
    }

    /// Terminate the active command when the user leaves or cancels its screen.
    pub fn stop(&mut self) {
        let Some(task) = self.running.as_mut() else {
            return;
        };
        let label = task.label.clone();
        let output_path = task.output_path.clone();
        kill_child(task);
        self.output = read_output(&output_path);
        self.message = format!("Cancelado: {label}.");
        self.running = None;
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        self.stop();
    }
}

fn kill_child(task: &mut RunningCommand) {
    if task.child.try_wait().ok().flatten().is_some() {
        return;
    }
    let _ = task.child.kill();
    let _ = task.child.wait();
}

fn read_output(path: &Path) -> String {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) => return format!("No se pudo leer el resultado: {error}"),
    };
    let size = file.metadata().map(|metadata| metadata.len()).unwrap_or(0);
    let start = size.saturating_sub(MAX_DISPLAY_OUTPUT as u64);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return String::new();
    }
    let mut buffer = Vec::new();
    if file.read_to_end(&mut buffer).is_err() {
        return String::new();
    }
    let text = String::from_utf8_lossy(&buffer);
    let lines: Vec<&str> = text.lines().collect();
    let first = lines.len().saturating_sub(MAX_DISPLAY_LINES);
    lines[first..]
        .iter()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Locate an executable on PATH without spawning a shell.
pub fn find_executable(name: &str) -> Option<PathBuf> {
    let candidate = Path::new(name);
    if candidate.components().count() > 1 {
        return candidate.is_file().then(|| candidate.to_path_buf());
    }
    let path = std::env::var_os("PATH")?;
    let extensions: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT".to_string())
            .split(';')
            .map(|extension| extension.to_ascii_lowercase())
            .collect()
    } else {
        vec![String::new()]
    };
    for directory in std::env::split_paths(&path) {
        for extension in &extensions {
            let file = directory.join(format!("{name}{extension}"));
            if file.is_file() {
                return Some(file);
            }
            if cfg!(windows) && extension.is_empty() {
                let file = directory.join(name);
                if file.is_file() {
                    return Some(file);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_tail_is_limited() {
        let path = std::env::temp_dir().join("hz_output_test.log");
        let content = (1..=40)
            .map(|line| format!("line {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, content).unwrap_or_default();
        let output = read_output(&path);
        assert_eq!(output.lines().count(), 24);
        assert!(output.starts_with("line 17"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn allowlist_rejects_arbitrary_programs() {
        let mut manager = ProcessManager::new(Path::new("."));
        assert!(!manager.start_command("rm", &["rm".to_string(), "-rf".to_string()], 10));
        assert_eq!(manager.message, "Comando no permitido.");
    }
}
