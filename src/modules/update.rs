//! Update screen: run `deploy/update.sh` and show its captured output.
//!
//! Runtime requirements:
//! - `deploy/update.sh` must exist in the app root and be executable by bash.
//! - The app normally runs as root under systemd; the update script uses sudo
//!   for system operations when not root. Network/git/cargo/systemd permissions
//!   can block execution; output is shown in the UI log.

use std::path::Path;
use std::time::Instant;

use crate::core::input::Action;
use crate::core::process_manager::ProcessManager;
use crate::core::system_stats::SystemStats;
use crate::core::ui::{
    draw_help, draw_row, draw_scroll_indicator, help_capacity, HelpSection, ScrollState,
};
use crate::gfx::{wrap_text, Canvas};
use crate::theme::*;

use super::{Outcome, ToolScreen};

const RESULT_TOP: i32 = 38;
const RESULT_BOTTOM: i32 = 208;
const RESULT_ROW: i32 = 16;

static HELP: &[HelpSection] = &[
    HelpSection {
        heading: "ACTUALIZACION",
        paragraphs: &[
            "Ejecuta deploy/update.sh: pull, build, install y restart.",
            "Muestra salida en vivo; puede tardar varios minutos en Pi Zero.",
        ],
    },
    HelpSection {
        heading: "CONTROLES",
        paragraphs: &[
            "START o A en INICIAR lanzan la actualizacion.",
            "B cancela durante la ejecucion o vuelve al menu.",
        ],
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Menu,
    Help,
    Running,
    Result,
}

pub struct UpdateScreen {
    phase: Phase,
    selected: usize,
    help_scroll: ScrollState,
    started_at: Instant,
    last_sync: Instant,
    activity: f32,
    result_view: ScrollState,
    _root: std::path::PathBuf,
}

impl UpdateScreen {
    pub fn new(root: &Path) -> Self {
        Self {
            phase: Phase::Menu,
            selected: 0,
            help_scroll: ScrollState::new(help_capacity()),
            started_at: Instant::now(),
            last_sync: Instant::now(),
            activity: 0.0,
            result_view: ScrollState::new(((RESULT_BOTTOM - RESULT_TOP) / RESULT_ROW) as usize),
            _root: root.to_path_buf(),
        }
    }

    fn result_lines(&self, processes: &ProcessManager) -> Vec<String> {
        let mut lines = wrap_text(&processes.message, 34);
        for line in processes.output.lines() {
            lines.extend(wrap_text(line, 34));
        }
        lines
    }

    fn start(&mut self, processes: &mut ProcessManager) {
        if processes.start_update() {
            self.started_at = Instant::now();
            self.last_sync = self.started_at;
            self.activity = 0.0;
            self.phase = Phase::Running;
        } else {
            self.result_view.reset(self.result_lines(processes).len());
            self.phase = Phase::Result;
        }
    }
}

impl ToolScreen for UpdateScreen {
    fn title(&self) -> &str {
        "ACTUALIZAR"
    }

    fn handle(&mut self, action: Action, processes: &mut ProcessManager) -> Outcome {
        match self.phase {
            Phase::Menu => match action {
                Action::Up => {
                    self.selected = (self.selected + 2) % 3;
                    Outcome::Stay
                }
                Action::Down => {
                    self.selected = (self.selected + 1) % 3;
                    Outcome::Stay
                }
                Action::Start => {
                    self.start(processes);
                    Outcome::Stay
                }
                Action::Open => match self.selected {
                    0 => {
                        self.start(processes);
                        Outcome::Stay
                    }
                    1 => {
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
                Action::Open | Action::Start => {
                    self.phase = Phase::Menu;
                    Outcome::Stay
                }
                Action::Back => Outcome::Close,
                _ => Outcome::Stay,
            },
            Phase::Running => {
                if action == Action::Back {
                    processes.stop();
                    self.result_view.reset(self.result_lines(processes).len());
                    self.phase = Phase::Result;
                }
                Outcome::Stay
            }
            Phase::Result => {
                let lines = self.result_lines(processes);
                self.result_view.update(lines.len(), None);
                match action {
                    Action::Up => {
                        self.result_view.move_by(-1);
                        Outcome::Stay
                    }
                    Action::Down => {
                        self.result_view.move_by(1);
                        Outcome::Stay
                    }
                    Action::Open | Action::Start => {
                        self.phase = Phase::Menu;
                        Outcome::Stay
                    }
                    Action::Back => Outcome::Close,
                    _ => Outcome::Stay,
                }
            }
        }
    }

    fn sync(&mut self, processes: &mut ProcessManager) {
        let now = Instant::now();
        if self.phase == Phase::Running {
            self.activity += now.duration_since(self.last_sync).as_secs_f32();
            if !processes.is_running() {
                self.result_view.reset(self.result_lines(processes).len());
                self.phase = Phase::Result;
            }
        }
        self.last_sync = now;
    }

    fn draw(&mut self, canvas: &mut Canvas, processes: &ProcessManager, _stats: &SystemStats) {
        canvas.fill_rect(
            0,
            CONTENT_TOP,
            SCREEN_WIDTH as i32,
            CONTENT_HEIGHT,
            BG_COLOR,
        );
        match self.phase {
            Phase::Menu => {
                let rows = ["INICIAR ACTUALIZACION", "AYUDA", "VOLVER"];
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
                let tip = "Update: git pull, build release, instala y reinicia el servicio.";
                for (index, line) in wrap_text(tip, 34).iter().take(2).enumerate() {
                    canvas.draw_text(line, 16, 120 + index as i32 * 12, MUTED_COLOR, 1);
                }
            }
            Phase::Help => draw_help(canvas, HELP, &mut self.help_scroll),
            Phase::Running => {
                let elapsed = self.started_at.elapsed().as_secs();
                let dots = ".".repeat((elapsed % 4) as usize);
                canvas.draw_text(&format!("ACTUALIZANDO{dots}"), 16, 44, TEXT_COLOR, 1);
                canvas.draw_text_clipped(
                    &format!("{elapsed}s / 180s"),
                    250,
                    44,
                    60,
                    MUTED_COLOR,
                    1,
                );

                let (progress, step) = update_progress(&processes.output, &processes.message);
                let detail = last_meaningful_line(&processes.output)
                    .unwrap_or_else(|| processes.message.clone());
                for (index, line) in wrap_text(&detail, 34).iter().take(2).enumerate() {
                    canvas.draw_text_clipped(line, 16, 74 + index as i32 * 14, 288, TEXT_COLOR, 1);
                }
                canvas.draw_text_clipped(step, 16, 112, 288, MUTED_COLOR, 1);

                let bar_x = 16;
                let bar_y = 132;
                let bar_w = 288;
                let bar_h = 10;
                canvas.fill_rect(bar_x, bar_y, bar_w, bar_h, MUTED_COLOR);
                let fill_w = ((bar_w as f32) * progress).round() as i32;
                if fill_w > 0 {
                    canvas.fill_rect(bar_x, bar_y, fill_w.clamp(0, bar_w), bar_h, ACCENT_COLOR);
                }
                canvas.draw_text(
                    &format!("{}%", (progress * 100.0).round() as i32),
                    16,
                    150,
                    ACCENT_COLOR,
                    1,
                );
                canvas.draw_text_clipped("B: cancelar", 220, 150, 84, MUTED_COLOR, 1);
            }
            Phase::Result => {
                let lines = self.result_lines(processes);
                self.result_view.update(lines.len(), None);
                for (position, index) in
                    (self.result_view.first()..self.result_view.last()).enumerate()
                {
                    let y = RESULT_TOP + position as i32 * RESULT_ROW;
                    let selected = index == self.result_view.selected;
                    if selected {
                        canvas.fill_rect(12, y - 1, 296, RESULT_ROW, SELECTED_BG);
                    }
                    canvas.draw_text_clipped(
                        &lines[index],
                        16,
                        y,
                        288,
                        if selected { ACCENT_COLOR } else { TEXT_COLOR },
                        1,
                    );
                }
                draw_scroll_indicator(
                    canvas,
                    12,
                    RESULT_TOP,
                    296,
                    RESULT_BOTTOM - RESULT_TOP,
                    self.result_view.has_above(),
                    self.result_view.has_below(),
                    ACCENT_COLOR,
                );
            }
        }
    }

    fn close(&mut self, processes: &mut ProcessManager) {
        if self.phase == Phase::Running {
            processes.stop();
        }
    }
}

fn last_meaningful_line(output: &str) -> Option<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .last()
        .map(|line| line.to_string())
}

fn update_progress(output: &str, message: &str) -> (f32, &'static str) {
    let haystack = format!("{output}\n{message}").to_ascii_lowercase();
    let (progress, step) = if haystack.contains("actualización completa")
        || haystack.contains("actualizacion completa")
    {
        (1.0, "Completado")
    } else if haystack.contains("reinici") || haystack.contains("instalando binario") {
        (0.90, "Instalando / reiniciando")
    } else if haystack.contains("compilando") || haystack.contains("cargo build") {
        (0.60, "Compilando")
    } else if haystack.contains("cambios detectados") || haystack.contains("git pull") {
        (0.30, "Descargando cambios")
    } else if haystack.contains("actualizando repositorio") {
        (0.15, "Buscando actualizaciones")
    } else if haystack.contains("ejecutando") {
        (0.05, "Iniciando")
    } else {
        (0.05, "Preparando")
    };
    (progress, step)
}
