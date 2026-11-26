//! Nmap module: conservative TCP-connect scan with optional NSE scripts.

use crate::core::input::Action;
use crate::core::process_manager::{ProcessManager, MAX_COMMAND_TIMEOUT_SECONDS};
use crate::core::system_stats::SystemStats;
use crate::core::ui::HelpSection;
use crate::core::validation::parse_ipv4;
use crate::gfx::Canvas;

use super::parameterized::{ParameterizedScreen, ToolSpec};
use super::{Outcome, ToolScreen};

const PORT_COUNTS: &[u32] = &[20, 50, 100];
const SCRIPT_OPTIONS: &[&str] = &["vuln", "ninguno"];

static HELP: &[HelpSection] = &[
    HelpSection {
        heading: "CONFIGURAR",
        paragraphs: &[
            "IP DESTINO: D-pad mueve entre digitos y cambia su valor.",
            "En PUERTOS TCP elige 20, 50 o 100 puertos comunes.",
        ],
    },
    HelpSection {
        heading: "EJECUTAR",
        paragraphs: &[
            "Selecciona INICIAR NMAP y confirma con A; B cancela.",
            "TCP connect lento y scripts NSE vuln; usa solo equipos autorizados.",
        ],
    },
];

pub struct NmapScreen {
    inner: ParameterizedScreen,
}

impl NmapScreen {
    pub fn new() -> Self {
        Self {
            inner: ParameterizedScreen::new(ToolSpec {
                title: "NMAP TCP",
                parameter_label: "PUERTOS TCP",
                parameter_values: PORT_COUNTS,
                initial_parameter: 20,
                operation_label: "INICIAR NMAP",
                option_label: "SCRIPTS NSE",
                option_values: SCRIPT_OPTIONS,
                help: HELP,
                command_builder: build_nmap_command,
                timeout_for: |_| MAX_COMMAND_TIMEOUT_SECONDS,
            }),
        }
    }
}

impl Default for NmapScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolScreen for NmapScreen {
    fn title(&self) -> &str {
        self.inner.title()
    }

    fn handle(&mut self, action: Action, processes: &mut ProcessManager) -> Outcome {
        self.inner.handle(action, processes)
    }

    fn sync(&mut self, processes: &mut ProcessManager) {
        self.inner.sync(processes);
    }

    fn draw(&mut self, canvas: &mut Canvas, processes: &ProcessManager, stats: &SystemStats) {
        self.inner.draw(canvas, processes, stats);
    }

    fn footer_hints(&self) -> Vec<(&'static str, i32)> {
        self.inner.footer_hints()
    }

    fn close(&mut self, processes: &mut ProcessManager) {
        self.inner.close(processes);
    }
}

/// Build a slow, unprivileged TCP-connect scan with the chosen NSE scripts.
pub fn build_nmap_command(
    target: &str,
    port_count: u32,
    script: Option<&str>,
) -> Result<Vec<String>, String> {
    let octets = parse_ipv4(target)?;
    let address = format!("{}.{}.{}.{}", octets[0], octets[1], octets[2], octets[3]);
    if !PORT_COUNTS.contains(&port_count) {
        return Err(format!("Los puertos deben ser uno de {:?}.", PORT_COUNTS));
    }
    let mut command = vec![
        "nmap".to_string(),
        "-n".to_string(),
        "-Pn".to_string(),
        "-sT".to_string(),
        "-T2".to_string(),
        "--top-ports".to_string(),
        port_count.to_string(),
        "--open".to_string(),
        "--host-timeout".to_string(),
        "90s".to_string(),
    ];
    if script == Some("ninguno") {
        command.push(address);
        return Ok(command);
    }
    command.push("--script".to_string());
    command.push("vuln".to_string());
    command.push(address);
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nmap_command_matches_the_python_flags() {
        let command = build_nmap_command("10.0.0.5", 20, Some("vuln")).unwrap_or_default();
        assert_eq!(command.first(), Some(&"nmap".to_string()));
        assert!(command.contains(&"-sT".to_string()));
        assert!(command.contains(&"vuln".to_string()));
        assert!(command.contains(&"10.0.0.5".to_string()));
    }

    #[test]
    fn nmap_can_disable_scripts() {
        let command = build_nmap_command("10.0.0.5", 50, Some("ninguno")).unwrap_or_default();
        assert!(!command.contains(&"vuln".to_string()));
    }

    #[test]
    fn nmap_rejects_invalid_targets() {
        assert!(build_nmap_command("not-an-ip", 20, None).is_err());
        assert!(build_nmap_command("10.0.0.5", 21, None).is_err());
    }
}
