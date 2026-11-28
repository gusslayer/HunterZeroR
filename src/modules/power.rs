//! Power menu: safe shutdown/reboot from the UI.

use std::process::{Command, Stdio};

use crate::core::input::Action;
use crate::core::logging;
use crate::core::process_manager::ProcessManager;
use crate::core::system_stats::SystemStats;
use crate::core::ui::{draw_help, draw_row, help_capacity, ScrollState};
use crate::gfx::Canvas;
use crate::theme::*;

use super::{Outcome, ToolScreen};

static HELP: &[crate::core::ui::HelpSection] = &[
    crate::core::ui::HelpSection {
        heading: "APAGADO",
        paragraphs: &[
            "Apaga o reinicia el equipo desde la interfaz.",
            "Usa systemctl/shutdown del sistema; requiere permisos (servicio root).",
        ],
    },
    crate::core::ui::HelpSection {
        heading: "CONTROLES",
        paragraphs: &[
            "A/START en una opcion ejecuta la accion.",
            "B vuelve al menu principal.",
        ],
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Menu,
    Help,
    ConfirmShutdown,
    ConfirmReboot,
    Done,
}

pub struct PowerScreen {
    phase: Phase,
    selected: usize,
    help_scroll: ScrollState,
    message: String,
}

impl PowerScreen {
    pub fn new() -> Self {
        Self {
            phase: Phase::Menu,
            selected: 0,
            help_scroll: ScrollState::new(help_capacity()),
            message: String::new(),
        }
    }

    fn run_command(args: &[&str]) -> Result<(), String> {
        let Some(program) = args.first() else {
            return Err("Comando vacío.".to_string());
        };
        let Some(executable) = crate::core::process_manager::find_executable(program) else {
            return Err(format!("No se encontró {program}."));
        };
        Command::new(executable)
            .args(&args[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("No se pudo ejecutar {program}: {e}"))
    }

    fn shutdown(&mut self) {
        logging::event("INFO", "power", "Shutdown requested from UI");
        let result = Self::run_command(&["systemctl", "poweroff"])
            .or_else(|_| Self::run_command(&["shutdown", "-h", "now"]));
        match result {
            Ok(()) => {
                self.message = "Apagando...".to_string();
                self.phase = Phase::Done;
            }
            Err(err) => {
                logging::event("ERROR", "power", &err);
                self.message = err;
                self.phase = Phase::Done;
            }
        }
    }

    fn reboot(&mut self) {
        logging::event("INFO", "power", "Reboot requested from UI");
        let result = Self::run_command(&["systemctl", "reboot"])
            .or_else(|_| Self::run_command(&["shutdown", "-r", "now"]));
        match result {
            Ok(()) => {
                self.message = "Reiniciando...".to_string();
                self.phase = Phase::Done;
            }
            Err(err) => {
                logging::event("ERROR", "power", &err);
                self.message = err;
                self.phase = Phase::Done;
            }
        }
    }
}

impl ToolScreen for PowerScreen {
    fn title(&self) -> &str {
        "APAGADO"
    }

    fn handle(&mut self, action: Action, _processes: &mut ProcessManager) -> Outcome {
        match self.phase {
            Phase::Menu => match action {
                Action::Up => {
                    self.selected = (self.selected + 3) % 4;
                    Outcome::Stay
                }
                Action::Down => {
                    self.selected = (self.selected + 1) % 4;
                    Outcome::Stay
                }
                Action::Start => {
                    match self.selected {
                        0 => self.phase = Phase::ConfirmShutdown,
                        1 => self.phase = Phase::ConfirmReboot,
                        2 => {
                            self.help_scroll.reset(help_capacity());
                            self.phase = Phase::Help;
                        }
                        _ => return Outcome::Close,
                    }
                    Outcome::Stay
                }
                Action::Open => match self.selected {
                    0 => {
                        self.phase = Phase::ConfirmShutdown;
                        Outcome::Stay
                    }
                    1 => {
                        self.phase = Phase::ConfirmReboot;
                        Outcome::Stay
                    }
                    2 => {
                        self.help_scroll.reset(help_capacity());
                        self.phase = Phase::Help;
                        Outcome::Stay
                    }
                    _ => Outcome::Close,
                },
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
                Action::Open | Action::Start | Action::Back => {
                    self.phase = Phase::Menu;
                    Outcome::Stay
                }
                _ => Outcome::Stay,
            },
            Phase::ConfirmShutdown => match action {
                Action::Open | Action::Start => {
                    self.shutdown();
                    Outcome::Stay
                }
                Action::Back => {
                    self.phase = Phase::Menu;
                    Outcome::Stay
                }
                _ => Outcome::Stay,
            },
            Phase::ConfirmReboot => match action {
                Action::Open | Action::Start => {
                    self.reboot();
                    Outcome::Stay
                }
                Action::Back => {
                    self.phase = Phase::Menu;
                    Outcome::Stay
                }
                _ => Outcome::Stay,
            },
            Phase::Done => match action {
                Action::Open | Action::Start | Action::Back => {
                    self.phase = Phase::Menu;
                    Outcome::Stay
                }
                _ => Outcome::Stay,
            },
        }
    }

    fn draw(&mut self, canvas: &mut Canvas, _processes: &ProcessManager, _stats: &SystemStats) {
        canvas.fill_rect(
            0,
            CONTENT_TOP,
            SCREEN_WIDTH as i32,
            CONTENT_HEIGHT,
            BG_COLOR,
        );
        match self.phase {
            Phase::Menu => {
                let rows = ["APAGAR SISTEMA", "REINICIAR", "AYUDA", "VOLVER"];
                for (index, label) in rows.iter().enumerate() {
                    draw_row(
                        canvas,
                        12,
                        36 + index as i32 * 20,
                        296,
                        18,
                        label,
                        "",
                        index == self.selected,
                        None,
                    );
                }
                canvas.draw_text("A: confirmar  B: volver", 16, 150, MUTED_COLOR, 1);
            }
            Phase::Help => draw_help(canvas, HELP, &mut self.help_scroll),
            Phase::ConfirmShutdown => {
                canvas.draw_text("¿Apagar el sistema?", 16, 60, TEXT_COLOR, 1);
                canvas.draw_text("A/START: confirmar", 16, 90, ACCENT_COLOR, 1);
                canvas.draw_text("B: cancelar", 16, 106, MUTED_COLOR, 1);
            }
            Phase::ConfirmReboot => {
                canvas.draw_text("¿Reiniciar el sistema?", 16, 60, TEXT_COLOR, 1);
                canvas.draw_text("A/START: confirmar", 16, 90, ACCENT_COLOR, 1);
                canvas.draw_text("B: cancelar", 16, 106, MUTED_COLOR, 1);
            }
            Phase::Done => {
                for (index, line) in crate::gfx::wrap_text(&self.message, 34)
                    .iter()
                    .take(4)
                    .enumerate()
                {
                    canvas.draw_text(line, 16, 60 + index as i32 * 14, TEXT_COLOR, 1);
                }
                canvas.draw_text("A/B: volver", 16, 150, MUTED_COLOR, 1);
            }
        }
    }
}
