//! Reusable settings, confirmation, help and result flow for one tool module.
//!
//! Ping and Nmap only differ in their parameter range, timeout and command
//! builder; the D-pad navigation is identical, so it lives here.

use crate::core::input::Action;
use crate::core::process_manager::ProcessManager;
use crate::core::system_stats::SystemStats;
use crate::core::ui::{
    draw_chevron, draw_frame, draw_help, draw_row, help_capacity, EditResult, HelpSection,
    Ipv4Editor, ScrollState,
};
use crate::gfx::{wrap_text, Canvas};
use crate::theme::*;

use super::{Outcome, ToolScreen};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Settings,
    EditTarget,
    Confirm,
    Running,
    Result,
    Help,
}

pub struct ToolSpec {
    pub title: &'static str,
    pub parameter_label: &'static str,
    pub parameter_values: &'static [u32],
    pub initial_parameter: u32,
    pub operation_label: &'static str,
    pub option_label: &'static str,
    pub option_values: &'static [&'static str],
    pub option_multi: bool,
    pub help: &'static [HelpSection],
    pub command_builder: fn(&str, u32, Option<&str>) -> Result<Vec<String>, String>,
    pub timeout_for: fn(u32) -> u64,
}

pub struct ParameterizedScreen {
    spec: ToolSpec,
    rows: Vec<&'static str>,
    target: Ipv4Editor,
    parameter: u32,
    option: usize,
    option_toggles: Vec<bool>,
    phase: Phase,
    selected: usize,
    help_scroll: ScrollState,
}

impl ParameterizedScreen {
    pub fn new(spec: ToolSpec) -> Self {
        let mut rows = vec!["IP DESTINO", spec.parameter_label, spec.operation_label];
        if !spec.option_values.is_empty() {
            rows.push(spec.option_label);
        }
        rows.push("AYUDA");
        rows.push("VOLVER");
        let parameter = if spec.parameter_values.contains(&spec.initial_parameter) {
            spec.initial_parameter
        } else {
            spec.parameter_values.first().copied().unwrap_or(0)
        };
        let option_toggles = if spec.option_multi {
            vec![false; spec.option_values.len()]
        } else {
            Vec::new()
        };
        Self {
            spec,
            rows,
            target: Ipv4Editor::new("192.168.1.1"),
            parameter,
            option: 0,
            option_toggles,
            phase: Phase::Settings,
            selected: 0,
            help_scroll: ScrollState::new(help_capacity()),
        }
    }

    fn option_value(&self) -> Option<String> {
        if self.spec.option_values.is_empty() {
            return None;
        }
        if self.spec.option_multi {
            let selected: Vec<&str> = self
                .spec
                .option_values
                .iter()
                .zip(self.option_toggles.iter())
                .filter_map(|(name, on)| if *on { Some(*name) } else { None })
                .collect();
            if selected.is_empty() {
                Some("ninguno".to_string())
            } else {
                Some(selected.join(","))
            }
        } else {
            self.spec
                .option_values
                .get(self.option)
                .map(|v| (*v).to_string())
        }
    }

    fn help_row(&self) -> usize {
        self.rows.len() - 2
    }

    fn back_row(&self) -> usize {
        self.rows.len() - 1
    }

    fn handle_settings(&mut self, action: Action, processes: &mut ProcessManager) -> Outcome {
        let option_row = if self.spec.option_values.is_empty() {
            None
        } else {
            Some(3)
        };
        match action {
            Action::Up => {
                self.selected = (self.selected + self.rows.len() - 1) % self.rows.len();
            }
            Action::Down => {
                self.selected = (self.selected + 1) % self.rows.len();
            }
            Action::Left | Action::Right => {
                let step = if action == Action::Right { 1 } else { -1 };
                if self.selected == 1 {
                    let index = self
                        .spec
                        .parameter_values
                        .iter()
                        .position(|value| *value == self.parameter)
                        .unwrap_or(0) as i32;
                    let count = self.spec.parameter_values.len() as i32;
                    let next = ((index + step) % count + count) % count;
                    self.parameter = self.spec.parameter_values[next as usize];
                } else if option_row == Some(self.selected) {
                    let count = self.spec.option_values.len() as i32;
                    self.option = (((self.option as i32 + step) % count + count) % count) as usize;
                }
            }
            Action::Open | Action::Start => {
                if self.selected == 0 {
                    self.target.begin();
                    self.phase = Phase::EditTarget;
                } else if option_row == Some(self.selected) && self.spec.option_multi {
                    if let Some(slot) = self.option_toggles.get_mut(self.option) {
                        *slot = !*slot;
                    }
                } else if self.selected == 2 {
                    if action == Action::Start {
                        self.start_command(processes);
                    } else {
                        self.phase = Phase::Confirm;
                    }
                } else if self.selected == self.help_row() {
                    self.phase = Phase::Help;
                } else if self.selected == self.back_row() {
                    return Outcome::Close;
                }
            }
            Action::Back => return Outcome::Close,
            _ => {}
        }
        Outcome::Stay
    }

    fn start_command(&mut self, processes: &mut ProcessManager) {
        let address = self.target.address();
        let option = self.option_value();
        let command = match (self.spec.command_builder)(&address, self.parameter, option.as_deref())
        {
            Ok(command) => command,
            Err(error) => {
                processes.message = error;
                self.phase = Phase::Result;
                return;
            }
        };
        let label = format!("{} {}", self.spec.operation_label, address);
        let timeout = (self.spec.timeout_for)(self.parameter);
        if processes.start_command(&label, &command, timeout) {
            self.phase = Phase::Running;
        } else {
            self.phase = Phase::Result;
        }
    }
}

impl ToolScreen for ParameterizedScreen {
    fn title(&self) -> &str {
        self.spec.title
    }

    fn handle(&mut self, action: Action, processes: &mut ProcessManager) -> Outcome {
        match self.phase {
            Phase::EditTarget => {
                if let Some(EditResult::Saved | EditResult::Cancelled) =
                    self.target.handle(action.as_open())
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
            Phase::Settings => self.handle_settings(action, processes),
        }
    }

    fn sync(&mut self, processes: &mut ProcessManager) {
        if self.phase == Phase::Running && !processes.is_running() {
            self.phase = Phase::Result;
        }
    }

    fn draw(&mut self, canvas: &mut Canvas, processes: &ProcessManager, _stats: &SystemStats) {
        if self.phase == Phase::EditTarget {
            self.target.draw(canvas);
            return;
        }
        if self.phase == Phase::Help {
            draw_help(canvas, self.spec.help, &mut self.help_scroll);
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
                let mut y = 32;
                let mut multi_block_bottom = 0i32;
                for (index, label) in self.rows.iter().enumerate() {
                    let is_multi_option =
                        self.spec.option_multi && !self.spec.option_values.is_empty() && index == 3;
                    if is_multi_option {
                        let selected = index == self.selected;
                        let option_count = self.spec.option_values.len() as i32;
                        let block_h = 17 + option_count * 13;
                        if selected {
                            canvas.fill_rect(12, y, 296, block_h, SELECTED_BG);
                            canvas.stroke_rect(12, y, 296, block_h, ACCENT_COLOR);
                            canvas.fill_rect(12, y, 3, block_h, ACCENT_COLOR);
                        }
                        canvas.draw_text_clipped(
                            &crate::core::font::plain_text(label).to_ascii_uppercase(),
                            19,
                            y + 4,
                            140,
                            TEXT_COLOR,
                            1,
                        );
                        for (i, name) in self.spec.option_values.iter().enumerate() {
                            let oy = y + 17 + i as i32 * 13;
                            let on = self.option_toggles.get(i).copied().unwrap_or(false);
                            let focused = i == self.option;
                            let marker = if on { "[X]" } else { "[ ]" };
                            let color = if focused && selected {
                                ACCENT_COLOR
                            } else if on {
                                TEXT_COLOR
                            } else {
                                MUTED_COLOR
                            };
                            let prefix = if focused && selected { "> " } else { "  " };
                            let item = format!("{prefix}{marker} {name}");
                            canvas.draw_text_clipped(&item, 24, oy, 270, color, 1);
                        }
                        if selected {
                            draw_chevron(canvas, 295, y + 3, 9, 11, ACCENT_COLOR);
                        }
                        y += block_h + 2;
                        multi_block_bottom = y;
                    } else {
                        let value = match index {
                            0 => self.target.address(),
                            1 => self.parameter.to_string(),
                            2 => String::new(),
                            _ if !self.spec.option_values.is_empty() && index == 3 => {
                                self.option_value().unwrap_or_default()
                            }
                            _ => String::new(),
                        };
                        draw_row(
                            canvas,
                            12,
                            y,
                            296,
                            17,
                            label,
                            &value,
                            index == self.selected,
                            None,
                        );
                        y += 19;
                    }
                }
                let help_y = if multi_block_bottom > 0 { y + 1 } else { 150 };
                let available_h = CONTENT_BOTTOM - 2 - help_y;
                if available_h >= 28 {
                    let help_h = available_h.clamp(28, 59);
                    draw_frame(canvas, 10, help_y, 300, help_h, GRID_COLOR, Some(PANEL_COLOR));
                    canvas.draw_text("AYUDA RAPIDA", 16, help_y + 4, ACCENT_COLOR, 1);
                    let tip = self
                        .spec
                        .help
                        .first()
                        .and_then(|section| section.paragraphs.first())
                        .copied()
                        .unwrap_or("");
                    let max_lines = ((help_h - 16) / 11).max(1) as usize;
                    for (index, line) in wrap_text(tip, 34).iter().take(max_lines).enumerate() {
                        canvas.draw_text(line, 16, help_y + 15 + index as i32 * 11, TEXT_COLOR, 1);
                    }
                }
            }
            Phase::Confirm => {
                let mut lines = vec![
                    format!("Destino: {}", self.target.address()),
                    format!("{}: {}", self.spec.parameter_label, self.parameter),
                ];
                if let Some(option) = self.option_value() {
                    lines.push(format!("{}: {option}", self.spec.option_label));
                }
                lines.push("Solo equipos propios o autorizados.".to_string());
                lines.push("A: ejecutar    B: cancelar".to_string());
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
                let label = format!("{} EN CURSO", self.spec.operation_label);
                canvas.draw_text_clipped(&label, 16, 38, 288, TEXT_COLOR, 1);
                canvas.draw_text_clipped(
                    &format!("OBJETIVO: {}", self.target.address()),
                    16,
                    58,
                    288,
                    ACCENT_COLOR,
                    1,
                );
                canvas.draw_text_clipped(&processes.message, 16, 78, 288, MUTED_COLOR, 1);
            }
            Phase::Result => {
                canvas.draw_text_clipped(&processes.message, 16, 36, 288, ACCENT_COLOR, 1);
                let lines: Vec<&str> = processes.output.lines().collect();
                let first = lines.len().saturating_sub(6);
                for (index, line) in lines[first..].iter().enumerate() {
                    canvas.draw_text_clipped(line, 16, 56 + index as i32 * 14, 288, TEXT_COLOR, 1);
                }
            }
            Phase::EditTarget | Phase::Help => {}
        }
    }

    fn close(&mut self, processes: &mut ProcessManager) {
        if self.phase == Phase::Running {
            processes.stop();
        }
    }
}
