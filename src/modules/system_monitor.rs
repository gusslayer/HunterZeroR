//! System monitor: local telemetry without launching external commands.

use std::time::Instant;

use crate::core::input::Action;
use crate::core::process_manager::ProcessManager;
use crate::core::system_stats::{disk_usage_percent, SystemStats};
use crate::core::ui::{draw_frame, draw_help, help_capacity, HelpSection, ScrollState};
use crate::gfx::Canvas;
use crate::theme::*;

use super::{Outcome, ToolScreen};

static HELP: &[HelpSection] = &[
    HelpSection {
        heading: "LECTURAS LOCALES",
        paragraphs: &[
            "CPU, RAM, temperatura, IP, SSID, Wi-Fi, bateria, disco y hora.",
            "Temperatura y bateria dependen de lo que exponga el sistema.",
        ],
    },
    HelpSection {
        heading: "CONTROLES",
        paragraphs: &["A alterna esta ayuda; B vuelve al menu."],
    },
];

pub struct SystemMonitorScreen {
    show_help: bool,
    help_scroll: ScrollState,
    disk_percent: u8,
    last_disk_refresh: Instant,
}

impl SystemMonitorScreen {
    pub fn new() -> Self {
        Self {
            show_help: false,
            help_scroll: ScrollState::new(help_capacity()),
            disk_percent: disk_usage_percent(),
            last_disk_refresh: Instant::now(),
        }
    }
}

impl Default for SystemMonitorScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolScreen for SystemMonitorScreen {
    fn title(&self) -> &str {
        "SISTEMA"
    }

    fn handle(&mut self, action: Action, _processes: &mut ProcessManager) -> Outcome {
        if action == Action::Back {
            return Outcome::Close;
        }
        if action == Action::Open || action == Action::Start {
            self.show_help = !self.show_help;
            return Outcome::Stay;
        }
        if self.show_help {
            match action {
                Action::Up => self.help_scroll.move_by(-1),
                Action::Down => self.help_scroll.move_by(1),
                _ => {}
            }
        }
        Outcome::Stay
    }

    fn sync(&mut self, _processes: &mut ProcessManager) {
        if self.last_disk_refresh.elapsed().as_secs_f32() >= 2.0 {
            self.last_disk_refresh = Instant::now();
            self.disk_percent = disk_usage_percent();
        }
    }

    fn draw(&mut self, canvas: &mut Canvas, _processes: &ProcessManager, stats: &SystemStats) {
        if self.show_help {
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
        draw_frame(canvas, 10, 32, 300, 136, GRID_COLOR, Some(PANEL_COLOR));
        let temperature = stats
            .temperature_c
            .map(|value| format!("{value} C"))
            .unwrap_or_else(|| "N/D".to_string());
        let battery = stats
            .battery_percent
            .map(|value| format!("{value}%"))
            .unwrap_or_else(|| "N/D".to_string());
        let values = [
            format!("CPU: {}%", stats.cpu_percent),
            format!("RAM: {}%", stats.ram_percent),
            format!("DISCO: {}%", self.disk_percent),
            format!("TEMP: {temperature}"),
            format!("IP: {}", stats.local_ip),
            format!("SSID: {}", stats.ssid.as_deref().unwrap_or("N/D")),
            format!(
                "WIFI: {}",
                if stats.wifi_connected {
                    "CONECTADO"
                } else {
                    "NO DISPONIBLE"
                }
            ),
            format!("BATERIA: {battery}"),
            format!("HORA: {}", stats.clock),
        ];
        for (index, value) in values.iter().enumerate() {
            let color = if index < 3 { ACCENT_COLOR } else { TEXT_COLOR };
            canvas.draw_text_clipped(value, 18, 38 + index as i32 * 15, 284, color, 1);
        }
    }
}
