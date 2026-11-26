//! On-device log viewer for app and command logs.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::core::input::Action;
use crate::core::process_manager::ProcessManager;
use crate::core::system_stats::SystemStats;
use crate::core::ui::{draw_help, draw_scroll_indicator, help_capacity, HelpSection, ScrollState};
use crate::gfx::{wrap_text, Canvas};
use crate::theme::*;

use super::{Outcome, ToolScreen};

const VIEW_TOP: i32 = 46;
const VIEW_BOTTOM: i32 = CONTENT_BOTTOM - 4;
const LINE_H: i32 = 12;

static HELP: &[HelpSection] = &[
    HelpSection {
        heading: "VISOR LOGS",
        paragraphs: &[
            "Muestra las ultimas lineas de logs/app.log.",
            "Pulsa A para alternar con logs/commands/latest.log.",
        ],
    },
    HelpSection {
        heading: "CONTROLES",
        paragraphs: &[
            "D-PAD arriba/abajo hace scroll. START vacia el log actual.",
            "B vuelve al menu principal.",
        ],
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    App,
    Command,
}

pub struct LogsScreen {
    root: PathBuf,
    source: Source,
    show_help: bool,
    help_scroll: ScrollState,
    view: ScrollState,
    lines: Vec<String>,
}

impl LogsScreen {
    pub fn new(root: &Path) -> Self {
        let mut screen = Self {
            root: root.to_path_buf(),
            source: Source::App,
            show_help: false,
            help_scroll: ScrollState::new(help_capacity()),
            view: ScrollState::new(((VIEW_BOTTOM - VIEW_TOP) / LINE_H).max(1) as usize),
            lines: Vec::new(),
        };
        screen.reload();
        screen
    }

    fn path(&self) -> PathBuf {
        match self.source {
            Source::App => self.root.join("logs").join("app.log"),
            Source::Command => self.root.join("logs").join("commands").join("latest.log"),
        }
    }

    fn source_label(&self) -> &'static str {
        match self.source {
            Source::App => "APP.LOG",
            Source::Command => "LATEST.LOG",
        }
    }

    fn reload(&mut self) {
        let path = self.path();
        let raw = fs::read_to_string(&path).unwrap_or_else(|_| "SIN LOG DISPONIBLE".to_string());
        let mut wrapped = Vec::new();
        let lines: Vec<&str> = raw.lines().collect();
        let start = lines.len().saturating_sub(200);
        for line in &lines[start..] {
            if line.is_empty() {
                wrapped.push(String::new());
            } else {
                wrapped.extend(wrap_text(line, 38));
            }
        }
        if wrapped.is_empty() {
            wrapped.push("SIN LOG DISPONIBLE".to_string());
        }
        self.lines = wrapped;
        let cap = ((VIEW_BOTTOM - VIEW_TOP) / LINE_H).max(1) as usize;
        self.view.update(self.lines.len(), Some(cap));
        // Jump to end so latest errors are visible.
        if self.lines.len() > cap {
            self.view.selected = self.lines.len() - 1;
            self.view.update(self.lines.len(), Some(cap));
        } else {
            self.view.selected = 0;
            self.view.offset = 0;
            self.view.update(self.lines.len(), Some(cap));
        }
    }

    fn clear_current(&mut self) {
        let path = self.path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let ts = Utc::now().format("%Y-%m-%d %H:%M:%S");
        let line = format!("[{ts}] [INFO] [logs] log reiniciado\n");
        let _ = fs::write(path, line);
        self.reload();
    }
}

impl ToolScreen for LogsScreen {
    fn title(&self) -> &str {
        "LOGS"
    }

    fn handle(&mut self, action: Action, _processes: &mut ProcessManager) -> Outcome {
        if self.show_help {
            match action {
                Action::Up => self.help_scroll.move_by(-1),
                Action::Down => self.help_scroll.move_by(1),
                Action::Open | Action::Start => self.show_help = false,
                Action::Back => return Outcome::Close,
                _ => {}
            }
            return Outcome::Stay;
        }

        match action {
            Action::Up => self.view.move_by(-1),
            Action::Down => self.view.move_by(1),
            Action::Open => {
                self.source = match self.source {
                    Source::App => Source::Command,
                    Source::Command => Source::App,
                };
                self.reload();
            }
            Action::Start => self.clear_current(),
            Action::Back => return Outcome::Close,
            _ => {}
        }
        Outcome::Stay
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

        canvas.draw_text(self.source_label(), 12, CONTENT_TOP + 4, ACCENT_COLOR, 1);
        canvas.draw_text_clipped(
            "A:CAMBIAR START:VACIAR B:VOLVER",
            108,
            CONTENT_TOP + 4,
            200,
            MUTED_COLOR,
            1,
        );

        let cap = ((VIEW_BOTTOM - VIEW_TOP) / LINE_H).max(1) as usize;
        self.view.update(self.lines.len(), Some(cap));
        let first = self.view.first();
        let last = self.view.last();
        for (row, idx) in (first..last).enumerate() {
            let y = VIEW_TOP + row as i32 * LINE_H;
            let line = &self.lines[idx];
            let color = if line.contains("[ERROR]") {
                ERROR_COLOR
            } else if line.contains("[WARN]") {
                ACCENT_COLOR
            } else {
                TEXT_COLOR
            };
            canvas.draw_text_clipped(line, 12, y, 292, color, 1);
        }
        draw_scroll_indicator(
            canvas,
            8,
            VIEW_TOP,
            304,
            VIEW_BOTTOM - VIEW_TOP,
            self.view.has_above(),
            self.view.has_below(),
            ACCENT_COLOR,
        );
    }

    fn close(&mut self, _processes: &mut ProcessManager) {}
}
