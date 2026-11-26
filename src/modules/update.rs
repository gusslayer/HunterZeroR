//! Update screen: run `deploy/update.sh` and show its captured output.

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
            "Ejecuta deploy/update.sh con git pull --ff-only.",
            "Requiere red y repositorio git configurado.",
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
                let tip = "Usa git pull --ff-only desde deploy/update.sh.";
                for (index, line) in wrap_text(tip, 34).iter().take(2).enumerate() {
                    canvas.draw_text(line, 16, 120 + index as i32 * 12, MUTED_COLOR, 1);
                }
            }
            Phase::Help => draw_help(canvas, HELP, &mut self.help_scroll),
            Phase::Running => {
                let elapsed = self.started_at.elapsed().as_secs();
                let dots = ".".repeat((elapsed % 4) as usize);
                canvas.draw_text(&format!("ACTUALIZANDO{dots}"), 16, 36, TEXT_COLOR, 1);
                canvas.draw_text(
                    &format!("Esperando git: {elapsed}s (max. 180s)"),
                    16,
                    56,
                    ACCENT_COLOR,
                    1,
                );
                let lines: Vec<&str> = processes.output.lines().collect();
                let first = lines.len().saturating_sub(3);
                for (index, line) in lines[first..].iter().enumerate() {
                    canvas.draw_text_clipped(line, 16, 76 + index as i32 * 16, 288, TEXT_COLOR, 1);
                }
                canvas.fill_rect(15, 145, 290, 7, MUTED_COLOR);
                let position = 15 + ((self.activity * 70.0) as i32 % 270);
                canvas.fill_rect(position, 145, 20, 7, ACCENT_COLOR);
                canvas.draw_text("B CANCELA LA DESCARGA.", 16, 194, ACCENT_COLOR, 1);
            }
            Phase::Result => {
                let lines = self.result_lines(processes);
                self.result_view.update(lines.len(), None);
                for (position, index) in
                    (self.result_view.first()..self.result_view.last()).enumerate()
                {
                    let color = if index == 0 { ACCENT_COLOR } else { TEXT_COLOR };
                    canvas.draw_text(
                        &lines[index],
                        16,
                        RESULT_TOP + position as i32 * RESULT_ROW,
                        color,
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
