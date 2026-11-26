//! DNS lookup screen with a gamepad hostname editor and an IP result view.

use crate::core::input::Action;
use crate::core::process_manager::ProcessManager;
use crate::core::system_stats::SystemStats;
use crate::core::ui::{
    draw_frame, draw_help, draw_row, help_capacity, EditResult, HelpSection, HostnameEditor,
    ScrollState,
};
use crate::core::validation::normalize_hostname;
use crate::gfx::wrap_text;
use crate::gfx::Canvas;
use crate::theme::*;

use super::{Outcome, ToolScreen};

const RECORD_TYPES: &[&str] = &["A", "AAAA"];
const MAX_LOOKUP_TIMEOUT_SECONDS: u64 = 20;

static HELP: &[HelpSection] = &[
    HelpSection {
        heading: "CONFIGURAR",
        paragraphs: &[
            "HOST: izquierda/derecha mueve el cursor.",
            "Arriba/abajo cambia el caracter; DEL borra y OK guarda.",
        ],
    },
    HelpSection {
        heading: "CONSULTAR",
        paragraphs: &[
            "TIPO REGISTRO elige A (IPv4) o AAAA (IPv6).",
            "Confirma con A; B cancela la consulta en curso.",
        ],
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Settings,
    EditHost,
    Confirm,
    Running,
    Result,
    Help,
}

pub struct NsLookupScreen {
    editor: HostnameEditor,
    record_type: usize,
    rows: [&'static str; 5],
    selected: usize,
    phase: Phase,
    addresses: Vec<String>,
    help_scroll: ScrollState,
}

impl NsLookupScreen {
    pub fn new() -> Self {
        Self {
            editor: HostnameEditor::new("example.com"),
            record_type: 0,
            rows: ["HOST", "TIPO REGISTRO", "CONSULTAR DNS", "AYUDA", "VOLVER"],
            selected: 0,
            phase: Phase::Settings,
            addresses: Vec::new(),
            help_scroll: ScrollState::new(help_capacity()),
        }
    }

    fn handle_settings(&mut self, action: Action) -> Outcome {
        match action {
            Action::Up => {
                self.selected = (self.selected + self.rows.len() - 1) % self.rows.len();
            }
            Action::Down => {
                self.selected = (self.selected + 1) % self.rows.len();
            }
            Action::Left | Action::Right if self.selected == 1 => {
                let count = RECORD_TYPES.len() as i32;
                let step = if action == Action::Right { 1 } else { -1 };
                self.record_type =
                    (((self.record_type as i32 + step) % count + count) % count) as usize;
            }
            Action::Open | Action::Start => match self.selected {
                0 => {
                    self.editor.begin();
                    self.phase = Phase::EditHost;
                }
                2 => {
                    if action == Action::Start {
                        self.start_command(processes);
                    } else {
                        self.phase = Phase::Confirm;
                    }
                }
                3 => self.phase = Phase::Help,
                4 => return Outcome::Close,
                _ => {}
            },
            Action::Back => return Outcome::Close,
            _ => {}
        }
        Outcome::Stay
    }

    fn start_command(&mut self, processes: &mut ProcessManager) {
        let hostname = self.editor.value();
        let command = match build_nslookup_command(&hostname, RECORD_TYPES[self.record_type]) {
            Ok(command) => command,
            Err(error) => {
                processes.message = error;
                self.addresses.clear();
                self.phase = Phase::Result;
                return;
            }
        };
        let label = format!("NSLOOKUP {hostname}");
        if processes.start_command(&label, &command, MAX_LOOKUP_TIMEOUT_SECONDS) {
            self.phase = Phase::Running;
        } else {
            self.phase = Phase::Result;
        }
    }
}

impl Default for NsLookupScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolScreen for NsLookupScreen {
    fn title(&self) -> &str {
        "NSLOOKUP"
    }

    fn handle(&mut self, action: Action, processes: &mut ProcessManager) -> Outcome {
        match self.phase {
            Phase::EditHost => {
                if let Some(EditResult::Saved | EditResult::Cancelled) =
                    self.editor.handle(action.as_open())
                {
                    self.phase = Phase::Settings;
                }
                Outcome::Stay
            }
            Phase::Confirm => {
                match action {
                    Action::Back => self.phase = Phase::Settings,
                    Action::Open | Action::Start => self.start_command(processes),
                    _ => {}
                }
                Outcome::Stay
            }
            Phase::Running => {
                if action == Action::Back {
                    processes.stop();
                    self.phase = Phase::Result;
                }
                Outcome::Stay
            }
            Phase::Result => match action {
                Action::Open | Action::Start => {
                    self.phase = Phase::Settings;
                    Outcome::Stay
                }
                Action::Back => Outcome::Close,
                _ => Outcome::Stay,
            },
            Phase::Help => match action {
                Action::Up => {
                    self.help_scroll.move_by(-1);
                    Outcome::Stay
                }
                Action::Down => {
                    self.help_scroll.move_by(1);
                    Outcome::Stay
                }
                Action::Open | Action::Start => {
                    self.phase = Phase::Settings;
                    Outcome::Stay
                }
                Action::Back => Outcome::Close,
                _ => Outcome::Stay,
            },
            Phase::Settings => self.handle_settings(action),
        }
    }

    fn sync(&mut self, processes: &mut ProcessManager) {
        if self.phase == Phase::Running && !processes.is_running() {
            self.addresses = parse_addresses(&processes.output);
            self.phase = Phase::Result;
        }
    }

    fn draw(&mut self, canvas: &mut Canvas, processes: &ProcessManager, _stats: &SystemStats) {
        if self.phase == Phase::EditHost {
            self.editor.draw(canvas);
            return;
        }
        if self.phase == Phase::Help {
            draw_help(canvas, HELP, &mut self.help_scroll);
            return;
        }
        canvas.fill_rect(
            0,
            CONTENT_TOP,
            SCREEN_WIDTH as i32,
            CONTENT_HEIGHT,
            BG_COLOR,
        );
        match self.phase {
            Phase::Settings => {
                let values = [
                    self.editor.value(),
                    RECORD_TYPES[self.record_type].to_string(),
                    String::new(),
                    String::new(),
                    String::new(),
                ];
                for (index, label) in self.rows.iter().enumerate() {
                    draw_row(
                        canvas,
                        12,
                        32 + index as i32 * 19,
                        296,
                        17,
                        label,
                        &values[index],
                        index == self.selected,
                        None,
                    );
                }
                draw_frame(canvas, 10, 150, 300, 59, GRID_COLOR, Some(PANEL_COLOR));
                canvas.draw_text("AYUDA RAPIDA", 16, 154, ACCENT_COLOR, 1);
                for (index, line) in wrap_text("ESCRIBE EL DOMINIO Y PULSA A PARA RESOLVER.", 34)
                    .iter()
                    .enumerate()
                {
                    canvas.draw_text(line, 16, 165 + index as i32 * 11, TEXT_COLOR, 1);
                }
            }
            Phase::Confirm => {
                let lines = [
                    format!("HOST: {}", self.editor.value()),
                    format!("TIPO: {}", RECORD_TYPES[self.record_type]),
                    "A: EJECUTAR  B: CANCELAR".to_string(),
                ];
                for (index, line) in lines.iter().enumerate() {
                    let color = if index == lines.len() - 1 {
                        ACCENT_COLOR
                    } else {
                        TEXT_COLOR
                    };
                    canvas.draw_text_clipped(line, 16, 36 + index as i32 * 16, 288, color, 1);
                }
            }
            Phase::Running => {
                canvas.draw_text("CONSULTANDO DNS...", 16, 38, TEXT_COLOR, 1);
                canvas.draw_text_clipped(
                    &format!("HOST: {}", self.editor.value()),
                    16,
                    60,
                    288,
                    ACCENT_COLOR,
                    1,
                );
                canvas.draw_text(
                    &format!("TIPO: {}", RECORD_TYPES[self.record_type]),
                    16,
                    77,
                    MUTED_COLOR,
                    1,
                );
            }
            Phase::Result => {
                canvas.draw_text_clipped(&processes.message, 16, 34, 288, ACCENT_COLOR, 1);
                let output_y = if self.addresses.is_empty() {
                    canvas.draw_text("SIN IP EN LA RESPUESTA", 16, 54, MUTED_COLOR, 1);
                    74
                } else {
                    canvas.draw_text("IP ENCONTRADA", 16, 54, ACCENT_COLOR, 1);
                    for (index, address) in self.addresses.iter().take(4).enumerate() {
                        canvas.draw_text_clipped(
                            address,
                            16,
                            68 + index as i32 * 14,
                            288,
                            TEXT_COLOR,
                            1,
                        );
                    }
                    68 + self.addresses.len().min(4) as i32 * 14 + 6
                };
                let lines: Vec<&str> = processes.output.lines().collect();
                let first = lines.len().saturating_sub(3);
                for (index, line) in lines[first..].iter().enumerate() {
                    let y = output_y + index as i32 * 14;
                    if y > 190 {
                        break;
                    }
                    canvas.draw_text_clipped(line, 16, y, 288, MUTED_COLOR, 1);
                }
            }
            Phase::EditHost | Phase::Help => {}
        }
    }

    fn close(&mut self, processes: &mut ProcessManager) {
        if self.phase == Phase::Running {
            processes.stop();
        }
    }
}

/// Build a bounded nslookup argv for a validated hostname and record type.
pub fn build_nslookup_command(hostname: &str, record_type: &str) -> Result<Vec<String>, String> {
    let target = normalize_hostname(hostname)?;
    if !RECORD_TYPES.contains(&record_type) {
        return Err("Tipo de registro no admitido.".to_string());
    }
    Ok(vec![
        "nslookup".to_string(),
        format!("-type={record_type}"),
        target,
    ])
}

/// Collect unique IP addresses from the answer section of nslookup output.
pub fn parse_addresses(output: &str) -> Vec<String> {
    let mut addresses = Vec::new();
    let mut in_answer = false;
    for raw_line in output.lines() {
        let line = raw_line.trim();
        let lowered = line.to_ascii_lowercase();
        if lowered.starts_with("name:") || lowered.contains("answer:") {
            in_answer = true;
            continue;
        }
        if !in_answer || !lowered.starts_with("address") {
            continue;
        }
        let Some((_, value)) = line.split_once(':') else {
            continue;
        };
        let candidate = value.split('#').next().unwrap_or("").trim();
        let Ok(address) = candidate.parse::<std::net::IpAddr>() else {
            continue;
        };
        let address = address.to_string();
        if !addresses.contains(&address) {
            addresses.push(address);
        }
    }
    addresses
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nslookup_command_is_bounded() {
        let command = build_nslookup_command("Example.COM", "A").unwrap_or_default();
        assert_eq!(command, vec!["nslookup", "-type=A", "example.com"]);
        assert!(build_nslookup_command("bad host", "A").is_err());
        assert!(build_nslookup_command("example.com", "MX").is_err());
    }

    #[test]
    fn addresses_are_extracted_from_the_answer_section() {
        let output = "\nName: example.com\nAddress: 93.184.216.34\n\nName: example.com\nAddress: 2606:2800::1\n";
        let addresses = parse_addresses(output);
        assert!(addresses.contains(&"93.184.216.34".to_string()));
        assert!(addresses.contains(&"2606:2800::1".to_string()));
    }

    #[test]
    fn server_address_before_the_name_is_ignored() {
        let output = "Server: 192.168.1.1\nAddress: 192.168.1.1#53\n\nName: example.com\nAddress: 93.184.216.34\n";
        let addresses = parse_addresses(output);
        assert_eq!(addresses, vec!["93.184.216.34".to_string()]);
    }
}
