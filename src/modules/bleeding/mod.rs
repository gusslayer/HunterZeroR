//! Gamepad screen for BLEeding: discovery, enumeration, and bounded DeAuth.

use crate::core::input::Action;
use crate::core::process_manager::ProcessManager;
use crate::core::system_stats::SystemStats;
use crate::core::ui::{
    draw_frame, draw_help, draw_row, draw_scroll_indicator, help_capacity, EditResult, HelpSection,
    MacEditor, ScrollState, DEFAULT_MAC,
};
use crate::gfx::{wrap_text, Canvas};
use crate::theme::*;

use super::engine::{
    port_for_protocol, BluetoothEngine, Device, Kind, Mode, Service, ATTACK_DURATIONS,
    PACKET_SIZES, PROTOCOLS, THREAD_COUNTS,
};
use super::{Outcome, ToolScreen};

const SETTINGS_ROWS: [&str; 7] = [
    "MODO", "OBJETIVO", "ESCANEAR", "ENUMERAR", "DEAUTH", "AYUDA", "VOLVER",
];

const PREPARE_ROWS: [&str; 6] = [
    "PAQUETE",
    "HILOS",
    "DURACION",
    "PROTOCOLO",
    "INICIAR",
    "CANCELAR",
];
const LIST_CAPACITY: usize = 8;
const RESULT_TOP: i32 = 38;
const RESULT_BOTTOM: i32 = 208;
const RESULT_ROW: i32 = 16;
const RESULT_CAPACITY: usize = ((RESULT_BOTTOM - RESULT_TOP) / RESULT_ROW) as usize;

static HELP: &[HelpSection] = &[
    HelpSection {
        heading: "ANALISIS",
        paragraphs: &[
            "MODO alterna BLE y BR/EDR con izquierda/derecha.",
            "ESCANEAR elige un dispositivo; ENUMERAR lee sus servicios.",
        ],
    },
    HelpSection {
        heading: "DEAUTH",
        paragraphs: &[
            "Requiere confirmacion; B cancela en cualquier momento.",
            "Duracion maxima 180s. Usa solo equipos propios o autorizados.",
        ],
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Settings,
    EditTarget,
    Devices,
    Services,
    Prepare,
    Confirm,
    Running,
    Result,
    Help,
}

pub struct BleedingScreen {
    engine: BluetoothEngine,
    editor: MacEditor,
    mode: usize,
    selected: usize,
    phase: Phase,
    job_kind: Kind,
    result_message: String,
    devices: Vec<Device>,
    services: Vec<Service>,
    device_view: ScrollState,
    service_view: ScrollState,
    result_view: ScrollState,
    help_scroll: ScrollState,
    packet_index: usize,
    thread_index: usize,
    duration_index: usize,
    protocol_index: usize,
    prepare_selected: usize,
}

impl BleedingScreen {
    pub fn new() -> Self {
        let packet_index = PACKET_SIZES
            .iter()
            .position(|value| *value == 512)
            .unwrap_or(0);
        let thread_index = THREAD_COUNTS
            .iter()
            .position(|value| *value == 2)
            .unwrap_or(0);
        let duration_index = ATTACK_DURATIONS
            .iter()
            .position(|value| *value == 30)
            .unwrap_or(0);
        Self {
            engine: BluetoothEngine::new(),
            editor: MacEditor::new(DEFAULT_MAC),
            mode: 0,
            selected: 0,
            phase: Phase::Settings,
            job_kind: Kind::Scan,
            result_message: String::new(),
            devices: Vec::new(),
            services: Vec::new(),
            device_view: ScrollState::new(LIST_CAPACITY),
            service_view: ScrollState::new(LIST_CAPACITY),
            result_view: ScrollState::new(RESULT_CAPACITY),
            help_scroll: ScrollState::new(help_capacity()),
            packet_index,
            thread_index,
            duration_index,
            protocol_index: 0,
            prepare_selected: 0,
        }
    }

    fn mode(&self) -> Mode {
        Mode::from_index(self.mode)
    }

    fn protocol(&self) -> &'static str {
        if self.mode() == Mode::Ble {
            "GATT"
        } else {
            PROTOCOLS[self.protocol_index]
        }
    }

    fn start_action(
        &mut self,
        kind: Kind,
        result: Result<(), String>,
        processes: &mut ProcessManager,
    ) {
        match result {
            Ok(()) => {
                self.job_kind = kind;
                self.phase = Phase::Running;
                processes.message = format!("Bluetooth: {}", kind.label());
            }
            Err(error) => {
                self.result_message = error;
                self.phase = Phase::Result;
                processes.message = self.result_message.clone();
            }
        }
    }

    fn start_scan(&mut self, processes: &mut ProcessManager) {
        let result = self.engine.start_scan(self.mode());
        self.start_action(Kind::Scan, result, processes);
    }

    fn start_enumeration(&mut self, processes: &mut ProcessManager) {
        let address = self.editor.value();
        let result = self.engine.start_enumeration(&address, self.mode());
        self.start_action(Kind::Enum, result, processes);
    }

    fn start_deauth(&mut self, processes: &mut ProcessManager) {
        let address = self.editor.value();
        let mode = self.mode();
        let protocol = PROTOCOLS[self.protocol_index];
        let port = port_for_protocol(protocol);
        let packet_size = PACKET_SIZES[self.packet_index];
        let workers = THREAD_COUNTS[self.thread_index];
        let duration = ATTACK_DURATIONS[self.duration_index];
        let result = self.engine.start_deauth(
            &address,
            mode,
            protocol,
            port,
            packet_size,
            workers,
            duration,
        );
        self.start_action(Kind::Deauth, result, processes);
    }

    fn result_lines(&self) -> Vec<String> {
        let state = self.engine.snapshot();
        let base = if self.result_message.is_empty() {
            state.message
        } else {
            self.result_message.clone()
        };
        let mut lines = wrap_text(&base, 34);
        for event in state.events {
            lines.extend(wrap_text(&event, 34));
        }
        lines
    }

    fn handle_settings(&mut self, action: Action, processes: &mut ProcessManager) -> Outcome {
        match action {
            Action::Up => {
                self.selected = (self.selected + SETTINGS_ROWS.len() - 1) % SETTINGS_ROWS.len();
            }
            Action::Down => {
                self.selected = (self.selected + 1) % SETTINGS_ROWS.len();
            }
            Action::Left | Action::Right if self.selected == 0 => {
                let step = if action == Action::Right { 1 } else { -1 };
                self.mode = (((self.mode as i32 + step) % 2 + 2) % 2) as usize;
            }
            Action::Open => match self.selected {
                0 => self.mode = (self.mode + 1) % 2,
                1 => {
                    self.editor.begin();
                    self.phase = Phase::EditTarget;
                }
                2 => self.start_scan(processes),
                3 => self.start_enumeration(processes),
                4 => {
                    self.prepare_selected = 0;
                    self.phase = Phase::Prepare;
                }
                5 => self.phase = Phase::Help,
                _ => {
                    self.engine.cancel();
                    return Outcome::Close;
                }
            },
            Action::Back => {
                self.engine.cancel();
                return Outcome::Close;
            }
            _ => {}
        }
        Outcome::Stay
    }

    fn handle_devices(&mut self, action: Action) -> Outcome {
        if !self.devices.is_empty() {
            self.device_view.update(self.devices.len(), None);
            match action {
                Action::Up => self.device_view.move_by(-1),
                Action::Down => self.device_view.move_by(1),
                Action::Open => {
                    if let Some(device) = self.devices.get(self.device_view.selected) {
                        self.editor = MacEditor::new(&device.address);
                        self.selected = 1;
                        self.result_message = "OBJETIVO SELECCIONADO".to_string();
                        self.phase = Phase::Settings;
                    }
                }
                _ => {}
            }
        }
        if action == Action::Back {
            self.phase = Phase::Settings;
        }
        Outcome::Stay
    }

    fn handle_services(&mut self, action: Action) -> Outcome {
        if !self.services.is_empty() {
            self.service_view.update(self.services.len(), None);
            match action {
                Action::Up => self.service_view.move_by(-1),
                Action::Down => self.service_view.move_by(1),
                _ => {}
            }
        }
        if action == Action::Open || action == Action::Back {
            self.phase = Phase::Settings;
        }
        Outcome::Stay
    }

    fn handle_prepare(&mut self, action: Action, processes: &mut ProcessManager) -> Outcome {
        match action {
            Action::Up => {
                self.prepare_selected =
                    (self.prepare_selected + PREPARE_ROWS.len() - 1) % PREPARE_ROWS.len();
            }
            Action::Down => {
                self.prepare_selected = (self.prepare_selected + 1) % PREPARE_ROWS.len();
            }
            Action::Left | Action::Right => {
                let step = if action == Action::Right { 1 } else { -1 };
                self.change_parameter(step);
            }
            Action::Open => match self.prepare_selected {
                4 => self.phase = Phase::Confirm,
                5 => self.phase = Phase::Settings,
                _ => {}
            },
            Action::Back => self.phase = Phase::Settings,
            _ => {}
        }
        let _ = processes;
        Outcome::Stay
    }

    fn change_parameter(&mut self, step: i32) {
        match self.prepare_selected {
            0 => self.packet_index = cycle(self.packet_index, PACKET_SIZES.len(), step),
            1 => self.thread_index = cycle(self.thread_index, THREAD_COUNTS.len(), step),
            2 => self.duration_index = cycle(self.duration_index, ATTACK_DURATIONS.len(), step),
            3 if self.mode() == Mode::Bredr => {
                self.protocol_index = cycle(self.protocol_index, PROTOCOLS.len(), step)
            }
            _ => {}
        }
    }

    fn draw_settings(&self, canvas: &mut Canvas) {
        for (index, label) in SETTINGS_ROWS.iter().enumerate() {
            draw_row(
                canvas,
                12,
                30 + index as i32 * 18,
                296,
                16,
                label,
                "",
                index == self.selected,
                None,
            );
        }
        canvas.draw_text(self.mode().label(), 240, 34, ACCENT_COLOR, 1);
        canvas.draw_text_clipped(&self.editor.value(), 152, 52, 140, ACCENT_COLOR, 1);

        draw_frame(canvas, 10, 160, 300, 46, GRID_COLOR, Some(PANEL_COLOR));
        let state = self.engine.snapshot();
        let bredr_status = if self.engine.bredr_discovery_available {
            "ON"
        } else if self.engine.bredr_available {
            "ATK"
        } else {
            "OFF"
        };
        let message = if state.message.is_empty() {
            "LISTO PARA ANALIZAR".to_string()
        } else {
            state.message
        };
        let mut lines = wrap_text(&message, 36);
        lines.truncate(2);
        lines.push(format!(
            "BLE {}  BR/EDR {bredr_status}",
            if self.engine.ble_available {
                "ON"
            } else {
                "OFF"
            }
        ));
        for (index, line) in lines.iter().take(3).enumerate() {
            let color = if index == 0 {
                ACCENT_COLOR
            } else {
                MUTED_COLOR
            };
            canvas.draw_text_clipped(line, 16, 164 + index as i32 * 13, 288, color, 1);
        }
    }

    fn draw_prepare(&self, canvas: &mut Canvas) {
        let values = [
            format!("{}B", PACKET_SIZES[self.packet_index]),
            THREAD_COUNTS[self.thread_index].to_string(),
            format!("{}S", ATTACK_DURATIONS[self.duration_index]),
            self.protocol().to_string(),
            String::new(),
            String::new(),
        ];
        for (index, label) in PREPARE_ROWS.iter().enumerate() {
            draw_row(
                canvas,
                12,
                30 + index as i32 * 18,
                296,
                16,
                label,
                &values[index],
                index == self.prepare_selected,
                None,
            );
        }
        canvas.draw_text_clipped(
            &format!("OBJETIVO {}", self.editor.value()),
            16,
            168,
            288,
            ACCENT_COLOR,
            1,
        );
    }

    fn draw_confirmation(&self, canvas: &mut Canvas) {
        let lines = [
            format!("OBJETIVO: {}", self.editor.value()),
            format!(
                "MODO: {}  PROTOCOLO: {}",
                self.mode().label(),
                self.protocol()
            ),
            format!(
                "PAQUETE: {}B  HILOS: {}",
                PACKET_SIZES[self.packet_index], THREAD_COUNTS[self.thread_index]
            ),
            format!("DURACION: {}S", ATTACK_DURATIONS[self.duration_index]),
            "SOLO EQUIPOS PROPIOS O AUTORIZADOS".to_string(),
            "A INICIAR  B CANCELAR".to_string(),
        ];
        for (index, line) in lines.iter().enumerate() {
            let color = if index >= lines.len() - 2 {
                ACCENT_COLOR
            } else {
                TEXT_COLOR
            };
            canvas.draw_text_clipped(line, 16, 38 + index as i32 * 18, 288, color, 1);
        }
    }

    fn draw_running(&self, canvas: &mut Canvas) {
        let state = self.engine.snapshot();
        let elapsed = state
            .started_at
            .map(|started| started.elapsed().as_secs())
            .unwrap_or(0);
        let mut lines = vec![
            format!("{} EN CURSO  {elapsed}S", state.kind.label()),
            format!("OBJETIVO: {}", self.editor.value()),
        ];
        lines.extend(wrap_text(&state.message, 34).into_iter().take(2));
        for (index, line) in lines.iter().enumerate() {
            canvas.draw_text_clipped(line, 16, 38 + index as i32 * 16, 288, ACCENT_COLOR, 1);
        }
        let first = state.events.len().saturating_sub(3);
        for (index, event) in state.events[first..].iter().enumerate() {
            canvas.draw_text_clipped(event, 16, 100 + index as i32 * 16, 288, MUTED_COLOR, 1);
        }
    }

    fn draw_devices(&mut self, canvas: &mut Canvas) {
        canvas.draw_text("DISPOSITIVOS", 16, 34, ACCENT_COLOR, 1);
        if self.devices.is_empty() {
            canvas.draw_text("SIN RESULTADOS", 16, 60, MUTED_COLOR, 1);
            return;
        }
        self.device_view.update(self.devices.len(), None);
        let first = self.device_view.first();
        let last = self.device_view.last();
        for (position, index) in (first..last).enumerate() {
            let device = &self.devices[index];
            let row_y = 52 + position as i32 * 17;
            draw_row(
                canvas,
                12,
                row_y,
                296,
                15,
                &truncate_chars(&device.name, 14),
                "",
                index == self.device_view.selected,
                None,
            );
            canvas.draw_text_clipped(
                &truncate_chars(&device.address, 17),
                150,
                row_y + 3,
                140,
                ACCENT_COLOR,
                1,
            );
        }
        draw_scroll_indicator(
            canvas,
            12,
            52,
            296,
            LIST_CAPACITY as i32 * 17,
            self.device_view.has_above(),
            self.device_view.has_below(),
            ACCENT_COLOR,
        );
    }

    fn draw_services(&mut self, canvas: &mut Canvas) {
        canvas.draw_text("SERVICIOS", 16, 34, ACCENT_COLOR, 1);
        if self.services.is_empty() {
            canvas.draw_text("SIN RESULTADOS", 16, 60, MUTED_COLOR, 1);
            return;
        }
        self.service_view.update(self.services.len(), None);
        let first = self.service_view.first();
        let last = self.service_view.last();
        for (position, index) in (first..last).enumerate() {
            let service = &self.services[index];
            let short_uuid: String = service
                .uuid
                .chars()
                .rev()
                .take(12)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            draw_row(
                canvas,
                12,
                52 + position as i32 * 17,
                296,
                15,
                &truncate_chars(&service.name, 18),
                &short_uuid,
                index == self.service_view.selected,
                None,
            );
        }
        draw_scroll_indicator(
            canvas,
            12,
            52,
            296,
            LIST_CAPACITY as i32 * 17,
            self.service_view.has_above(),
            self.service_view.has_below(),
            ACCENT_COLOR,
        );
    }

    fn draw_result(&mut self, canvas: &mut Canvas) {
        let lines = self.result_lines();
        self.result_view.update(lines.len(), None);
        let first = self.result_view.first();
        let last = self.result_view.last();
        for (position, index) in (first..last).enumerate() {
            let color = if index == 0 {
                ACCENT_COLOR
            } else {
                MUTED_COLOR
            };
            canvas.draw_text_clipped(
                &lines[index],
                16,
                RESULT_TOP + position as i32 * RESULT_ROW,
                288,
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

impl Default for BleedingScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolScreen for BleedingScreen {
    fn title(&self) -> &str {
        "BLEEDING"
    }

    fn handle(&mut self, action: Action, processes: &mut ProcessManager) -> Outcome {
        match self.phase {
            Phase::EditTarget => {
                if let Some(EditResult::Saved | EditResult::Cancelled) = self.editor.handle(action)
                {
                    self.phase = Phase::Settings;
                }
                Outcome::Stay
            }
            Phase::Devices => self.handle_devices(action),
            Phase::Services => self.handle_services(action),
            Phase::Prepare => self.handle_prepare(action, processes),
            Phase::Confirm => {
                match action {
                    Action::Back => self.phase = Phase::Prepare,
                    Action::Open => self.start_deauth(processes),
                    _ => {}
                }
                Outcome::Stay
            }
            Phase::Running => {
                if action == Action::Back {
                    self.engine.cancel();
                    self.result_message = if self.job_kind == Kind::Deauth {
                        "Ataque cancelado".to_string()
                    } else {
                        "Tarea cancelada".to_string()
                    };
                    self.phase = Phase::Result;
                }
                Outcome::Stay
            }
            Phase::Result => {
                self.result_view.update(self.result_lines().len(), None);
                match action {
                    Action::Up => {
                        self.result_view.move_by(-1);
                        Outcome::Stay
                    }
                    Action::Down => {
                        self.result_view.move_by(1);
                        Outcome::Stay
                    }
                    Action::Open => {
                        self.phase = Phase::Settings;
                        Outcome::Stay
                    }
                    Action::Back => {
                        self.engine.cancel();
                        Outcome::Close
                    }
                    _ => Outcome::Stay,
                }
            }
            Phase::Help => match action {
                Action::Up => {
                    self.help_scroll.move_by(-1);
                    Outcome::Stay
                }
                Action::Down => {
                    self.help_scroll.move_by(1);
                    Outcome::Stay
                }
                Action::Open => {
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
        let state = self.engine.snapshot();
        if self.phase != Phase::Running {
            return;
        }
        processes.message = truncate_chars(&state.message, 34);
        if state.running {
            return;
        }
        if state.message.starts_with("Fallo") {
            self.result_message = state.message;
            self.phase = Phase::Result;
            return;
        }
        match state.kind {
            Kind::Scan => {
                self.devices = state.devices;
                self.device_view.reset(self.devices.len());
                self.phase = Phase::Devices;
            }
            Kind::Enum => {
                self.services = state.services;
                self.service_view.reset(self.services.len());
                self.phase = Phase::Services;
            }
            Kind::Deauth => {
                self.result_message = state.message;
                self.phase = Phase::Result;
            }
        }
    }

    fn draw(&mut self, canvas: &mut Canvas, _processes: &ProcessManager, _stats: &SystemStats) {
        canvas.fill_rect(0, CONTENT_TOP, SCREEN_WIDTH as i32, 183, BG_COLOR);
        match self.phase {
            Phase::EditTarget => self.editor.draw(canvas),
            Phase::Help => draw_help(canvas, HELP, &mut self.help_scroll),
            Phase::Settings => self.draw_settings(canvas),
            Phase::Prepare => self.draw_prepare(canvas),
            Phase::Confirm => self.draw_confirmation(canvas),
            Phase::Running => self.draw_running(canvas),
            Phase::Devices => self.draw_devices(canvas),
            Phase::Services => self.draw_services(canvas),
            Phase::Result => self.draw_result(canvas),
        }
    }

    fn footer_hints(&self) -> Vec<(&'static str, i32)> {
        match self.phase {
            Phase::EditTarget => vec![("ARR/ABA HEX", 8), ("IZQ/DER POS", 118), ("A GUARDAR", 240)],
            Phase::Devices | Phase::Services => vec![
                ("ARR/ABA LISTA", 8),
                ("A SELECCIONAR", 150),
                ("B VOLVER", 240),
            ],
            Phase::Prepare => vec![("IZQ/DER VALOR", 8), ("A INICIAR", 168), ("B VOLVER", 240)],
            Phase::Confirm => vec![("A ATACAR", 8), ("B CANCELAR", 120)],
            Phase::Running => vec![("B CANCELAR", 8)],
            Phase::Result => vec![("ARR/ABA TEXTO", 8), ("A OPCIONES", 150), ("B VOLVER", 240)],
            Phase::Help => vec![("ARR/ABA TEXTO", 8), ("A OPCIONES", 168), ("B VOLVER", 240)],
            Phase::Settings => vec![
                ("ARR/ABA MOVER", 8),
                ("IZQ/DER MODO", 136),
                ("A ABRIR", 240),
            ],
        }
    }

    fn close(&mut self, _processes: &mut ProcessManager) {
        self.engine.cancel();
    }
}

fn cycle(index: usize, len: usize, step: i32) -> usize {
    if len == 0 {
        return 0;
    }
    (((index as i32 + step) % len as i32 + len as i32) % len as i32) as usize
}

fn truncate_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_wraps_in_both_directions() {
        assert_eq!(cycle(0, 4, -1), 3);
        assert_eq!(cycle(3, 4, 1), 0);
        assert_eq!(cycle(2, 4, 1), 3);
    }

    #[test]
    fn settings_and_prepare_tables_match_the_python_ui() {
        assert_eq!(SETTINGS_ROWS.len(), 7);
        assert_eq!(PREPARE_ROWS.len(), 6);
        assert_eq!(RESULT_CAPACITY, 10);
    }
}
