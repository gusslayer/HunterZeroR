//! Ping module: manually addressed ICMP Echo requests.

use crate::core::input::Action;
use crate::core::process_manager::ProcessManager;
use crate::core::system_stats::SystemStats;
use crate::core::ui::HelpSection;
use crate::core::validation::parse_ipv4;
use crate::gfx::Canvas;

use super::parameterized::{ParameterizedScreen, ToolSpec};
use super::{Outcome, ToolScreen};

const PACKET_COUNTS: &[u32] = &[1, 2, 3, 4];

static HELP: &[HelpSection] = &[
    HelpSection {
        heading: "CONFIGURAR",
        paragraphs: &[
            "IP DESTINO: D-pad mueve entre digitos y cambia su valor.",
            "En PAQUETES ICMP usa izquierda/derecha para elegir de 1 a 4.",
        ],
    },
    HelpSection {
        heading: "EJECUTAR",
        paragraphs: &[
            "Selecciona ENVIAR PING y confirma con A; B cancela.",
            "Se admite cualquier IPv4; cada octeto debe estar entre 0 y 255.",
        ],
    },
];

pub struct PingScreen {
    inner: ParameterizedScreen,
}

impl PingScreen {
    pub fn new() -> Self {
        Self {
            inner: ParameterizedScreen::new(ToolSpec {
                title: "PING MANUAL",
                parameter_label: "PAQUETES ICMP",
                parameter_values: PACKET_COUNTS,
                initial_parameter: 2,
                operation_label: "ENVIAR PING",
                option_label: "",
                option_values: &[],
                option_multi: false,
                help: HELP,
                command_builder: build_ping_command,
                timeout_for: |count| 3 + count as u64 * 2,
            }),
        }
    }
}

impl Default for PingScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolScreen for PingScreen {
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

    fn close(&mut self, processes: &mut ProcessManager) {
        self.inner.close(processes);
    }
}

/// Build a bounded ping command. Linux keeps the original flags; Windows uses
/// its native equivalents so the development build is also usable.
pub fn build_ping_command(
    target: &str,
    packet_count: u32,
    _script: Option<&str>,
) -> Result<Vec<String>, String> {
    let octets = parse_ipv4(target)?;
    let address = format!("{}.{}.{}.{}", octets[0], octets[1], octets[2], octets[3]);
    if !PACKET_COUNTS.contains(&packet_count) {
        return Err(format!(
            "El numero de paquetes debe ser uno de {:?}.",
            PACKET_COUNTS
        ));
    }
    let count = packet_count.to_string();
    if cfg!(windows) {
        Ok(vec![
            "ping".to_string(),
            "-n".to_string(),
            count,
            "-w".to_string(),
            "2000".to_string(),
            address,
        ])
    } else {
        Ok(vec![
            "ping".to_string(),
            "-n".to_string(),
            "-c".to_string(),
            count,
            "-W".to_string(),
            "2".to_string(),
            address,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_command_is_bounded() {
        let command = build_ping_command("192.168.1.10", 2, None).unwrap_or_default();
        assert_eq!(command.first(), Some(&"ping".to_string()));
        assert!(command.contains(&"192.168.1.10".to_string()));
        assert!(build_ping_command("999.1.1.1", 2, None).is_err());
        assert!(build_ping_command("192.168.1.10", 99, None).is_err());
    }

    #[test]
    fn screen_starts_without_a_process() {
        let _ = PingScreen::new();
    }
}
