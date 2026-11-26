//! Update screen: run `deploy/update.sh` and show its captured output.

use std::path::Path;
use std::time::Instant;

use crate::core::input::Action;
use crate::core::process_manager::ProcessManager;
use crate::core::system_stats::SystemStats;
use crate::core::ui::{draw_scroll_indicator, ScrollState};
use crate::gfx::{wrap_text, Canvas};
use crate::theme::*;

use super::{Outcome, ToolScreen};

const RESULT_TOP: i32 = 38;
const RESULT_BOTTOM: i32 = 208;
const RESULT_ROW: i32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Confirm,
    Running,
    Result,
}

pub struct UpdateScreen {
    phase: Phase,
    started_at: Instant,
    last_sync: Instant,
    activity: f32,
    result_view: ScrollState,
    _root: std::path::PathBuf,
}

impl UpdateScreen {
    pub fn new(root: &Path) -> Self {
        Self {
            phase: Phase::Confirm,
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
}

impl ToolScreen for UpdateScreen {
    fn title(&self) -> &str {
        "ACTUALIZAR"
    }

    fn handle(&mut self, action: Action, processes: &mut ProcessManager) -> Outcome {
        match self.phase {
            Phase::Confirm => match action {
                Action::Open => {
                    if processes.start_update() {
                        self.started_at = Instant::now();
                        self.last_sync = self.started_at;
                        self.activity = 0.0;
                        self.phase = Phase::Running;
                    } else {
                        self.phase = Phase::Result;
                    }
                    Outcome::Stay
                }
                Action::Back => Outcome::Close,
                _ => Outcome::Stay,
            },
            Phase::Running => {
                if action == Action::Back {
                    processes.stop();
                    self.phase = Phase::Result;
                }
                Outcome::Stay
            }
            Phase::Result => {
                self.result_view
                    .update(self.result_lines(processes).len(), None);
                match action {
                    Action::Up => {
                        self.result_view.move_by(-1);
                        Outcome::Stay
                    }
                    Action::Down => {
                        self.result_view.move_by(1);
                        Outcome::Stay
                    }
                    _ => Outcome::Close,
                }
            }
        }
    }

    fn sync(&mut self, processes: &mut ProcessManager) {
        let now = Instant::now();
        if self.phase == Phase::Running {
            self.activity += now.duration_since(self.last_sync).as_secs_f32();
        }
        self.last_sync = now;
        if self.phase == Phase::Running && !processes.is_running() {
            self.phase = Phase::Result;
        }
    }

    fn draw(&mut self, canvas: &mut Canvas, processes: &ProcessManager, _stats: &SystemStats) {
        canvas.fill_rect(0, CONTENT_TOP, SCREEN_WIDTH as i32, 184, BG_COLOR);
        match self.phase {
            Phase::Confirm => {
                let mut lines = wrap_text("Descarga el codigo con git pull --ff-only.", 34);
                lines.extend(wrap_text("Los pasos con sudo se indican al terminar.", 34));
                lines.push("A: ACTUALIZAR   B: VOLVER".to_string());
                for (index, line) in lines.iter().enumerate() {
                    let color = if index == lines.len() - 1 {
                        ACCENT_COLOR
                    } else {
                        TEXT_COLOR
                    };
                    canvas.draw_text(line, 16, 36 + index as i32 * 13, color, 1);
                }
            }
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

    fn footer_hints(&self) -> Vec<(&'static str, i32)> {
        match self.phase {
            Phase::Confirm => vec![("A ACTUALIZAR", 12), ("B VOLVER", 168)],
            Phase::Running => vec![("B CANCELAR", 12)],
            Phase::Result => vec![("ARR/ABA TEXTO", 8), ("A/B VOLVER", 150)],
        }
    }

    fn close(&mut self, processes: &mut ProcessManager) {
        if self.phase == Phase::Running {
            processes.stop();
        }
    }
}
