//! Process monitor: a top-style list of the busiest processes.

use std::time::Instant;

use crate::core::input::Action;
use crate::core::process_manager::ProcessManager;
use crate::core::system_stats::{ProcessRow, ProcessSampler, SystemStats};
use crate::core::ui::{draw_help, draw_scroll_indicator, help_capacity, HelpSection, ScrollState};
use crate::gfx::Canvas;
use crate::theme::*;

use super::{Outcome, ToolScreen};

const LIST_TOP: i32 = 48;
const LIST_BOTTOM: i32 = 208;
const ROW_HEIGHT: i32 = 15;
const LIST_CAPACITY: usize = ((LIST_BOTTOM - LIST_TOP) / ROW_HEIGHT) as usize;

static HELP: &[HelpSection] = &[
    HelpSection {
        heading: "PROCESOS ACTIVOS",
        paragraphs: &[
            "Top por CPU; PID, %CPU, %MEM y tiempo transcurrido.",
            "Refresca cada segundo; no lanza procesos.",
        ],
    },
    HelpSection {
        heading: "CONTROLES",
        paragraphs: &["A alterna esta ayuda; B vuelve al menu."],
    },
];

pub struct ProcMonitorScreen {
    show_help: bool,
    help_scroll: ScrollState,
    sampler: ProcessSampler,
    processes: Vec<ProcessRow>,
    list_view: ScrollState,
    last_refresh: Instant,
}

impl ProcMonitorScreen {
    pub fn new() -> Self {
        let mut sampler = ProcessSampler::new();
        let processes = sampler.snapshot();
        let mut list_view = ScrollState::new(LIST_CAPACITY);
        list_view.update(processes.len(), None);
        Self {
            show_help: false,
            help_scroll: ScrollState::new(help_capacity()),
            sampler,
            processes,
            list_view,
            last_refresh: Instant::now(),
        }
    }
}

impl ToolScreen for ProcMonitorScreen {
    fn title(&self) -> &str {
        "PROCESOS"
    }

    fn handle(&mut self, action: Action, _processes: &mut ProcessManager) -> Outcome {
        if self.show_help {
            match action {
                Action::Open | Action::Start => self.show_help = false,
                Action::Back => return Outcome::Close,
                Action::Up => self.help_scroll.move_by(-1),
                Action::Down => self.help_scroll.move_by(1),
                _ => {}
            }
            return Outcome::Stay;
        }
        match action {
            Action::Back => Outcome::Close,
            Action::Open | Action::Start => {
                self.show_help = true;
                Outcome::Stay
            }
            Action::Up => {
                self.list_view.move_by(-1);
                Outcome::Stay
            }
            Action::Down => {
                self.list_view.move_by(1);
                Outcome::Stay
            }
            _ => Outcome::Stay,
        }
    }

    fn sync(&mut self, _processes: &mut ProcessManager) {
        if self.last_refresh.elapsed().as_secs_f32() < 1.0 {
            return;
        }
        self.last_refresh = Instant::now();
        self.processes = self.sampler.snapshot();
        self.list_view.update(self.processes.len(), None);
    }

    fn draw(&mut self, canvas: &mut Canvas, _processes: &ProcessManager, _stats: &SystemStats) {
        canvas.fill_rect(
            0,
            CONTENT_TOP,
            SCREEN_WIDTH as i32,
            CONTENT_HEIGHT,
            BG_COLOR,
        );
        if self.show_help {
            draw_help(canvas, HELP, &mut self.help_scroll);
            return;
        }
        if self.processes.is_empty() {
            canvas.draw_text("SIN PROCESOS DISPONIBLES.", 16, 48, TEXT_COLOR, 1);
            return;
        }
        canvas.draw_text("PROC   PID  %CPU %MEM HRA", 16, 34, MUTED_COLOR, 1);
        self.list_view.update(self.processes.len(), None);
        for (position, index) in (self.list_view.first()..self.list_view.last()).enumerate() {
            let snapshot = &self.processes[index];
            let name = truncate_chars(&snapshot.name, 7);
            let line = format!(
                "{:<7} {:>5} {:3.0}% {:3.0}% {}",
                name,
                snapshot.pid,
                snapshot.cpu_percent,
                snapshot.memory_percent,
                truncate_chars(&snapshot.elapsed, 4)
            );
            canvas.draw_text(
                &line,
                16,
                LIST_TOP + position as i32 * ROW_HEIGHT,
                TEXT_COLOR,
                1,
            );
        }
        draw_scroll_indicator(
            canvas,
            12,
            LIST_TOP,
            296,
            LIST_BOTTOM - LIST_TOP,
            self.list_view.has_above(),
            self.list_view.has_below(),
            ACCENT_COLOR,
        );
    }
}

fn truncate_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_capacity_matches_the_visible_area() {
        assert_eq!(LIST_CAPACITY, 10);
    }
}
