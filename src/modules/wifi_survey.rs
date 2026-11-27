//! Wi-Fi survey: real-time signal, gradient, and heatmap-style history.
//!
//! Runtime requirements:
//! - Linux: `iw` must be installed and the process needs permission to scan
//!   (usually root, which is how HunterZero runs under systemd).
//! - The Wi-Fi interface is auto-detected from `iw dev` (preferring `wlan0`
//!   when present); if no interface is found, the UI reports a clear error.
//! - Windows/non-Linux: simulated scan data is used for development.

use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::core::input::Action;
use crate::core::logging;
use crate::core::process_manager::ProcessManager;
use crate::core::system_stats::SystemStats;
use crate::gfx::Canvas;
use crate::theme::*;

use super::{Outcome, ToolScreen};

const HISTORY_CAP: usize = 240;
const DISCOVER_POLL_MS: u64 = 4000;
const SURVEY_POLL_MS: u64 = 2000;
const GRADIENT_WINDOW: usize = 5; // ~10s at the 2s survey cadence
const SAVE_FLASH_MS: u64 = 1500;
const GRAPH_LEFT: i32 = 34;
const GRAPH_RIGHT: i32 = 314;
const GRAPH_TOP: i32 = 118;
const GRAPH_BOTTOM: i32 = 206;
const GRAPH_W: i32 = GRAPH_RIGHT - GRAPH_LEFT;
const GRAPH_H: i32 = GRAPH_BOTTOM - GRAPH_TOP;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Discover,
    Survey,
}

impl Mode {
    fn label(self) -> &'static str {
        match self {
            Mode::Discover => "SCAN",
            Mode::Survey => "SURVEY",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Zoom {
    X1,
    X2,
    X4,
}

impl Zoom {
    fn samples(self) -> usize {
        match self {
            Zoom::X1 => 240,
            Zoom::X2 => 120,
            Zoom::X4 => 60,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Zoom::X1 => "1X",
            Zoom::X2 => "2X",
            Zoom::X4 => "4X",
        }
    }

    fn next(self) -> Self {
        match self {
            Zoom::X1 => Zoom::X2,
            Zoom::X2 => Zoom::X4,
            Zoom::X4 => Zoom::X1,
        }
    }

    fn prev(self) -> Self {
        match self {
            Zoom::X1 => Zoom::X4,
            Zoom::X2 => Zoom::X1,
            Zoom::X4 => Zoom::X2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkInfo {
    pub ssid: String,
    pub bssid: String,
    pub channel: Option<u16>,
    pub freq_mhz: Option<u32>,
    pub bitrate: String,
    pub signal_dbm: Option<i32>,
    pub connected: bool,
}

impl Default for LinkInfo {
    fn default() -> Self {
        Self {
            ssid: "N/D".to_string(),
            bssid: "N/D".to_string(),
            channel: None,
            freq_mhz: None,
            bitrate: "N/D".to_string(),
            signal_dbm: None,
            connected: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanEntry {
    pub ssid: String,
    pub bssid: String,
    pub channel: Option<u16>,
    pub signal_dbm: Option<i32>,
}

#[derive(Debug, Clone)]
enum WorkerMsg {
    // Produced by the Linux `iw link` fallback; kept for every platform so the
    // UI state machine has a single message type.
    #[allow(dead_code)]
    Link(LinkInfo),
    Scan(Vec<ScanEntry>),
    Error(String),
}

#[derive(Debug)]
struct WorkerCtl {
    mode: Mode,
    force_scan: bool,
    iface: String,
}

pub struct WifiSurveyScreen {
    root: PathBuf,
    mode: Mode,
    zoom: Zoom,
    v_min: i32,
    v_max: i32,
    history: VecDeque<i32>,
    link: LinkInfo,
    scan_entries: Vec<ScanEntry>,
    scan_selected: usize,
    survey_bssid: Option<String>,
    saved_count: u32,
    save_flash: Option<Instant>,
    status_msg: String,
    last_error: Option<String>,
    ifaces: Vec<String>,
    iface_idx: usize,
    rx: Receiver<WorkerMsg>,
    stop: Arc<AtomicBool>,
    ctl: Arc<Mutex<WorkerCtl>>,
    worker: Option<JoinHandle<()>>,
}

impl WifiSurveyScreen {
    pub fn new(root: &Path) -> Self {
        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let ifaces = detect_wifi_ifaces();
        let iface_idx = 0usize;
        let iface = ifaces
            .get(iface_idx)
            .cloned()
            .unwrap_or_else(|| "wlan0".to_string());
        let ctl = Arc::new(Mutex::new(WorkerCtl {
            mode: Mode::Discover,
            force_scan: true,
            iface: iface.clone(),
        }));
        let worker_stop = Arc::clone(&stop);
        let worker_ctl = Arc::clone(&ctl);
        let worker = thread::Builder::new()
            .name("wifi-survey".to_string())
            .spawn(move || {
                let _ = catch_unwind(AssertUnwindSafe(|| {
                    worker_loop(worker_stop, worker_ctl, tx)
                }));
            })
            .ok();
        Self {
            root: root.to_path_buf(),
            mode: Mode::Discover,
            zoom: Zoom::X1,
            v_min: -90,
            v_max: -30,
            history: VecDeque::with_capacity(HISTORY_CAP),
            link: LinkInfo::default(),
            scan_entries: Vec::new(),
            scan_selected: 0,
            survey_bssid: None,
            saved_count: 0,
            save_flash: None,
            status_msg: String::new(),
            last_error: None,
            ifaces,
            iface_idx,
            rx,
            stop,
            ctl,
            worker,
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        if let Ok(mut ctl) = self.ctl.lock() {
            ctl.mode = mode;
            ctl.force_scan = true;
        }
        match mode {
            Mode::Discover => logging::event("INFO", "wifi_survey", "Modo DISCOVER"),
            Mode::Survey => logging::event("INFO", "wifi_survey", "Modo SURVEY"),
        }
    }

    fn selected_scan_bssid(&self) -> Option<String> {
        self.scan_entries
            .get(self.scan_selected)
            .map(|entry| entry.bssid.clone())
    }

    fn update_link_from_scan(&mut self, entry: &ScanEntry, push: bool) {
        self.link.ssid = entry.ssid.clone();
        self.link.bssid = entry.bssid.clone();
        self.link.channel = entry.channel;
        self.link.signal_dbm = entry.signal_dbm;
        self.link.connected = entry.signal_dbm.is_some();
        let dbm = if push { entry.signal_dbm } else { None };
        if let Some(dbm) = dbm {
            self.push_signal(dbm);
        }
    }

    fn apply_link(&mut self, info: LinkInfo) {
        // Only push history in Survey mode; Discover uses Link as a fallback
        // status update and should not pollute the survey graph.
        if self.mode == Mode::Survey {
            if let Some(dbm) = info.signal_dbm {
                self.push_signal(dbm);
            }
        }
        if info.connected || info.signal_dbm.is_some() {
            self.status_msg.clear();
        } else if self.status_msg.is_empty() {
            self.status_msg = "SIN CONEXION / SIN DATOS".to_string();
        }
        self.link = info;
    }

    fn handle_scan(&mut self, entries: Vec<ScanEntry>) {
        let prev_bssid = self.selected_scan_bssid();
        self.scan_entries = entries;
        let prev_index = prev_bssid.as_deref().and_then(|prev| {
            self.scan_entries
                .iter()
                .position(|e| e.bssid.eq_ignore_ascii_case(prev))
        });
        if let Some(idx) = prev_index {
            self.scan_selected = idx;
        }
        if self.scan_selected >= self.scan_entries.len() {
            self.scan_selected = self.scan_entries.len().saturating_sub(1);
        }

        if self.mode == Mode::Survey {
            let target = self.survey_bssid.clone();
            match target.as_deref() {
                Some(target) => {
                    if let Some(idx) = self
                        .scan_entries
                        .iter()
                        .position(|e| e.bssid.eq_ignore_ascii_case(target))
                    {
                        self.scan_selected = idx;
                        if let Some(entry) = self.scan_entries.get(idx).cloned() {
                            self.update_link_from_scan(&entry, true);
                            self.status_msg.clear();
                        }
                    } else {
                        self.status_msg = "AP NO VISIBLE".to_string();
                    }
                }
                None => {
                    if let Some(entry) = self.scan_entries.get(self.scan_selected).cloned() {
                        self.update_link_from_scan(&entry, true);
                    }
                }
            }
            return;
        }

        if let Some(entry) = self.scan_entries.get(self.scan_selected).cloned() {
            self.update_link_from_scan(&entry, false);
            self.status_msg.clear();
        } else if self.scan_entries.is_empty() {
            self.status_msg = "SCAN: SIN REDES (revisa root/rfkill)".to_string();
        }
    }

    fn push_signal(&mut self, dbm: i32) {
        let clamped = dbm.clamp(-100, -10);
        if self.history.len() == HISTORY_CAP {
            self.history.pop_front();
        }
        self.history.push_back(clamped);
    }

    fn gradient_db(&self) -> i32 {
        signal_gradient(&self.history, GRADIENT_WINDOW)
    }

    fn visible_history(&self) -> Vec<i32> {
        let n = self.zoom.samples().min(self.history.len());
        self.history.iter().rev().take(n).rev().copied().collect()
    }

    fn save_snapshot(&mut self) {
        if self.survey_bssid.is_none() || self.link.signal_dbm.is_none() {
            self.status_msg = "SIN DATOS PARA GUARDAR".to_string();
            return;
        }
        let delta = self.gradient_db();
        let signal = self.link.signal_dbm.unwrap_or(-100);
        let level = thermal_level(signal).label();
        let path = self.root.join("logs").join("wifi_survey.csv");
        let next_count = self.saved_count.saturating_add(1);
        let result = (|| -> std::io::Result<()> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let needs_header = !path.exists()
                || std::fs::metadata(&path)
                    .map(|m| m.len() == 0)
                    .unwrap_or(true);
            let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
            if needs_header {
                writeln!(
                    file,
                    "punto,timestamp,modo,ssid,bssid,canal,dbm,delta_db,nivel_termico"
                )?;
            }
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            let channel = self
                .link
                .channel
                .map(|c| c.to_string())
                .unwrap_or_else(|| "N/D".to_string());
            let ssid = csv_escape(&self.link.ssid);
            let bssid = csv_escape(&self.link.bssid);
            writeln!(
                file,
                "{},{},{},{},{},{},{},{},{}",
                next_count,
                timestamp,
                self.mode.label(),
                ssid,
                bssid,
                channel,
                signal,
                delta,
                level
            )?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.saved_count = next_count;
                self.save_flash = Some(Instant::now());
            }
            Err(error) => {
                logging::event(
                    "WARN",
                    "wifi_survey",
                    &format!("No se pudo guardar CSV: {error}"),
                );
            }
        }
    }

    fn draw_header(&self, canvas: &mut Canvas) {
        let mode = self.mode.label();
        let iface = self
            .ifaces
            .get(self.iface_idx)
            .map(String::as_str)
            .unwrap_or("wlan0");
        let zoom = self.zoom.label();
        let left = if self.mode == Mode::Survey {
            format!("{mode} Z:{zoom}")
        } else {
            format!("{mode} {iface}")
        };
        canvas.draw_text_clipped(&left, 4, 30, 120, ACCENT_COLOR, 1);
        let ch = self
            .link
            .channel
            .map(|c| format!("CH{c}"))
            .unwrap_or_else(|| "CH?".to_string());
        canvas.draw_text_clipped(&ch, 126, 30, 40, MUTED_COLOR, 1);
        canvas.draw_text_clipped(&self.link.ssid, 168, 30, 148, TEXT_COLOR, 1);
    }

    fn draw_signal_bar(&self, canvas: &mut Canvas) {
        let dbm = self.link.signal_dbm.unwrap_or(-90);
        let ratio = signal_ratio(dbm);
        let bar_x = 4;
        let bar_y = 42;
        let bar_w = 240;
        let bar_h = 12;
        canvas.fill_rect(bar_x, bar_y, bar_w, bar_h, PANEL_COLOR);
        canvas.stroke_rect(bar_x, bar_y, bar_w, bar_h, GRID_COLOR);
        let fill_w = ((bar_w - 2) as f32 * ratio) as i32;
        let color = thermal_level(dbm).color();
        if fill_w > 0 {
            canvas.fill_rect(bar_x + 1, bar_y + 1, fill_w, bar_h - 2, color);
        }
        let label = self
            .link
            .signal_dbm
            .map(|v| format!("{v}dBm"))
            .unwrap_or_else(|| "N/D".to_string());
        canvas.draw_text(&label, 250, 44, color, 1);
    }

    fn draw_gradient(&self, canvas: &mut Canvas) {
        let delta = self.gradient_db();
        let (arrow, color) = if delta > 0 {
            (">>>", SUCCESS_COLOR)
        } else if delta < 0 {
            ("<<<", ERROR_COLOR)
        } else {
            ("---", MUTED_COLOR)
        };
        canvas.draw_text("GRAD", 4, 58, MUTED_COLOR, 1);
        canvas.draw_text(arrow, 44, 58, color, 1);
        let sign = if delta > 0 { "+" } else { "" };
        canvas.draw_text(&format!("{sign}{delta}dB"), 80, 58, color, 1);
        if self.mode == Mode::Discover && !self.scan_entries.is_empty() {
            let info = format!("{}/{}", self.scan_selected + 1, self.scan_entries.len());
            canvas.draw_text_clipped(&info, 140, 58, 60, MUTED_COLOR, 1);
        }
        let bssid = if self.link.bssid.is_empty() {
            "N/D"
        } else {
            &self.link.bssid
        };
        canvas.draw_text_clipped(bssid, 190, 58, 126, MUTED_COLOR, 1);
    }

    fn draw_scan_list(&self, canvas: &mut Canvas) {
        if self.mode != Mode::Discover {
            return;
        }
        let y0 = 70;
        let max_rows: usize = 3;
        if self.scan_entries.is_empty() {
            canvas.draw_text_clipped(
                "SCAN: SIN REDES  START=REESCANEAR",
                4,
                y0,
                312,
                MUTED_COLOR,
                1,
            );
            return;
        }
        let start = self
            .scan_selected
            .saturating_sub(max_rows.saturating_sub(1));
        let end = (start + max_rows).min(self.scan_entries.len());
        for (row, idx) in (start..end).enumerate() {
            let entry = &self.scan_entries[idx];
            let y = y0 + row as i32 * 10;
            let selected = idx == self.scan_selected;
            let color = if selected { ACCENT_COLOR } else { TEXT_COLOR };
            let dbm = entry
                .signal_dbm
                .map(|v| format!("{v}"))
                .unwrap_or_else(|| "?".to_string());
            let line = format!("{} {}dBm", entry.ssid, dbm);
            if selected {
                canvas.fill_rect(2, y - 1, 316, 10, SELECTED_BG);
            }
            canvas.draw_text_clipped(&line, 4, y, 220, color, 1);
            canvas.draw_text_clipped(&entry.bssid, 226, y, 92, MUTED_COLOR, 1);
        }
        canvas.draw_text_clipped(
            "A=SELECCIONAR  START=REESCANEAR",
            4,
            102,
            312,
            ACCENT_COLOR,
            1,
        );
    }

    fn dbm_to_graph_y(&self, dbm: i32) -> i32 {
        dbm_to_y_range(dbm, self.v_min, self.v_max)
    }

    fn draw_graph(&self, canvas: &mut Canvas) {
        canvas.fill_rect(
            GRAPH_LEFT - 2,
            GRAPH_TOP - 2,
            GRAPH_W + 6,
            GRAPH_H + 6,
            PANEL_COLOR,
        );
        canvas.stroke_rect(
            GRAPH_LEFT - 2,
            GRAPH_TOP - 2,
            GRAPH_W + 6,
            GRAPH_H + 6,
            GRID_COLOR,
        );

        // Thermal level bands and labels.
        let levels = [
            (-55i32, "55", thermal_level(-50).color()),
            (-67i32, "67", thermal_level(-60).color()),
            (-78i32, "78", thermal_level(-72).color()),
        ];
        for (dbm, label, color) in levels {
            if dbm < self.v_min || dbm > self.v_max {
                continue;
            }
            let y = self.dbm_to_graph_y(dbm);
            // dotted guide
            let mut x = GRAPH_LEFT;
            while x < GRAPH_RIGHT {
                canvas.fill_rect(x, y, 2, 1, GRID_COLOR);
                x += 4;
            }
            // compact label box
            let lw = 20;
            let lx = 6;
            let ly = (y - 4).clamp(GRAPH_TOP - 1, GRAPH_BOTTOM - 10);
            canvas.stroke_rect(lx, ly, lw, 9, color);
            canvas.draw_text(label, lx + 2, ly + 1, color, 1);
        }

        let samples = self.visible_history();
        if samples.len() < 2 {
            let msg = if self.mode == Mode::Discover {
                "SELECCIONA SSID + A PARA MEDIR"
            } else if self.status_msg.is_empty() {
                "SIN DATOS"
            } else {
                self.status_msg.as_str()
            };
            canvas.draw_text_clipped(msg, GRAPH_LEFT + 8, GRAPH_TOP + 28, 270, MUTED_COLOR, 1);
            let tip = if self.mode == Mode::Discover {
                "START=REESCANEAR"
            } else {
                "TIP: ejecutar como root"
            };
            canvas.draw_text_clipped(tip, GRAPH_LEFT + 8, GRAPH_TOP + 42, 270, MUTED_COLOR, 1);
            return;
        }

        let max_x = GRAPH_W - 1;
        let n = samples.len();
        for i in 1..n {
            let x0 = GRAPH_LEFT + ((i - 1) as i32 * max_x) / (n as i32 - 1);
            let x1 = GRAPH_LEFT + (i as i32 * max_x) / (n as i32 - 1);
            let s0 = samples[i - 1];
            let s1 = samples[i];
            let y0 = self.dbm_to_graph_y(s0);
            let y1 = self.dbm_to_graph_y(s1);
            let color = thermal_level(s1).color();
            canvas.line(x0, y0, x1, y1, color);
            // soft fill under curve
            let y_base = GRAPH_BOTTOM - 1;
            let mid_y = ((y0 + y1) / 2).clamp(GRAPH_TOP, y_base);
            canvas.vline(x1, mid_y, y_base, blend(color, BG_COLOR));
        }
    }

    fn draw_save_flash(&self, canvas: &mut Canvas) {
        let Some(when) = self.save_flash else {
            return;
        };
        if when.elapsed() > Duration::from_millis(SAVE_FLASH_MS) {
            return;
        }
        let msg = format!("SAVED #{}", self.saved_count);
        canvas.fill_rect(232, 100, 84, 12, PANEL_COLOR);
        canvas.stroke_rect(232, 100, 84, 12, SUCCESS_COLOR);
        canvas.draw_text(&msg, 236, 102, SUCCESS_COLOR, 1);
    }
}

impl Drop for WifiSurveyScreen {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Detach worker: joining could block UI if iw scan is in progress.
        let _ = self.worker.take();
    }
}

impl ToolScreen for WifiSurveyScreen {
    fn title(&self) -> &str {
        "WIFI SURVEY"
    }

    fn handle(&mut self, action: Action, _processes: &mut ProcessManager) -> Outcome {
        match action {
            Action::Back => {
                if self.mode == Mode::Survey {
                    self.survey_bssid = None;
                    self.last_error = None;
                    self.set_mode(Mode::Discover);
                    self.status_msg = "SELECCIONA UN SSID Y PULSA A".to_string();
                    return Outcome::Stay;
                }
                return Outcome::Close;
            }
            Action::Left => {
                if self.mode == Mode::Survey {
                    self.zoom = self.zoom.prev();
                } else if self.ifaces.len() > 1 {
                    self.iface_idx = if self.iface_idx == 0 {
                        self.ifaces.len() - 1
                    } else {
                        self.iface_idx - 1
                    };
                    if let Some(iface) = self.ifaces.get(self.iface_idx).cloned() {
                        self.status_msg = format!("IFACE: {iface}");
                        if let Ok(mut ctl) = self.ctl.lock() {
                            ctl.iface = iface;
                            ctl.force_scan = true;
                        }
                    }
                }
            }
            Action::Right => {
                if self.mode == Mode::Survey {
                    self.zoom = self.zoom.next();
                } else if self.ifaces.len() > 1 {
                    self.iface_idx = (self.iface_idx + 1) % self.ifaces.len();
                    if let Some(iface) = self.ifaces.get(self.iface_idx).cloned() {
                        self.status_msg = format!("IFACE: {iface}");
                        if let Ok(mut ctl) = self.ctl.lock() {
                            ctl.iface = iface;
                            ctl.force_scan = true;
                        }
                    }
                }
            }
            Action::Up => {
                if self.mode == Mode::Discover && !self.scan_entries.is_empty() {
                    self.scan_selected = self.scan_selected.saturating_sub(1);
                } else if self.mode == Mode::Survey {
                    self.v_min = -90;
                    self.v_max = -30;
                }
            }
            Action::Down => {
                if self.mode == Mode::Discover && !self.scan_entries.is_empty() {
                    self.scan_selected =
                        (self.scan_selected + 1).min(self.scan_entries.len().saturating_sub(1));
                } else if self.mode == Mode::Survey {
                    self.v_min = -80;
                    self.v_max = -40;
                }
            }
            Action::Open => {
                if self.mode == Mode::Discover {
                    if let Some(entry) = self.scan_entries.get(self.scan_selected).cloned() {
                        self.history.clear();
                        self.save_flash = None;
                        self.last_error = None;
                        self.survey_bssid = Some(entry.bssid.clone());
                        if let Ok(mut ctl) = self.ctl.lock() {
                            ctl.force_scan = true;
                            ctl.mode = Mode::Survey;
                        }
                        self.mode = Mode::Survey;
                        self.status_msg = format!("MIDIENDO {}", entry.ssid);
                        logging::event("INFO", "wifi_survey", "Survey iniciado");
                    } else {
                        self.status_msg = "NO HAY SSID SELECCIONADO".to_string();
                    }
                } else {
                    self.save_snapshot();
                }
            }
            Action::Start => {
                if self.mode == Mode::Discover {
                    let prev = self
                        .ifaces
                        .get(self.iface_idx)
                        .cloned()
                        .unwrap_or_else(|| "wlan0".to_string());
                    let refreshed = detect_wifi_ifaces();
                    if !refreshed.is_empty() {
                        self.ifaces = refreshed;
                        self.iface_idx = self.ifaces.iter().position(|i| i == &prev).unwrap_or(0);
                    }
                }
                let iface = self
                    .ifaces
                    .get(self.iface_idx)
                    .cloned()
                    .unwrap_or_else(|| "wlan0".to_string());
                if let Ok(mut ctl) = self.ctl.lock() {
                    ctl.iface = iface;
                    ctl.force_scan = true;
                }
                self.status_msg = if self.mode == Mode::Discover {
                    "ESCANEANDO...".to_string()
                } else {
                    "MIDIENDO...".to_string()
                };
            }
            _ => {}
        }
        Outcome::Stay
    }

    fn sync(&mut self, _processes: &mut ProcessManager) {
        loop {
            match self.rx.try_recv() {
                Ok(WorkerMsg::Link(info)) => {
                    let wrong_target = self.mode == Mode::Survey
                        && self
                            .survey_bssid
                            .as_deref()
                            .is_some_and(|target| !info.bssid.eq_ignore_ascii_case(target));
                    if wrong_target {
                        if self.status_msg.is_empty() {
                            self.status_msg = "AP NO VISIBLE".to_string();
                        }
                        continue;
                    }
                    self.last_error = None;
                    self.apply_link(info);
                }
                Ok(WorkerMsg::Scan(entries)) => {
                    self.last_error = None;
                    self.handle_scan(entries);
                }
                Ok(WorkerMsg::Error(msg)) => {
                    if self.last_error.as_deref() != Some(msg.as_str()) {
                        logging::event("WARN", "wifi_survey", &msg);
                        self.last_error = Some(msg.clone());
                    }
                    self.status_msg = msg;
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
        if self
            .save_flash
            .is_some_and(|when| when.elapsed() > Duration::from_millis(SAVE_FLASH_MS))
        {
            self.save_flash = None;
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
        self.draw_header(canvas);
        self.draw_signal_bar(canvas);
        self.draw_gradient(canvas);
        self.draw_scan_list(canvas);
        self.draw_graph(canvas);
        self.draw_save_flash(canvas);
    }

    fn close(&mut self, _processes: &mut ProcessManager) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

// ---------------- Worker ----------------

fn worker_loop(stop: Arc<AtomicBool>, ctl: Arc<Mutex<WorkerCtl>>, tx: mpsc::Sender<WorkerMsg>) {
    let mut last_scan = Instant::now() - Duration::from_secs(60);
    let mut sim_tick: i32 = 0;
    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let (mode, force_scan, iface) = match ctl.lock() {
            Ok(mut c) => {
                let fs = c.force_scan;
                c.force_scan = false;
                (c.mode, fs, c.iface.clone())
            }
            Err(_) => (Mode::Discover, false, "wlan0".to_string()),
        };

        let interval = if force_scan {
            Duration::from_millis(0)
        } else {
            Duration::from_millis(match mode {
                Mode::Discover => DISCOVER_POLL_MS,
                Mode::Survey => SURVEY_POLL_MS,
            })
        };
        if force_scan || last_scan.elapsed() >= interval {
            last_scan = Instant::now();
            let msg = collect_scan(sim_tick, &iface);
            sim_tick = sim_tick.wrapping_add(1);
            if tx.send(msg).is_err() {
                break;
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn collect_scan(sim_tick: i32, iface: &str) -> WorkerMsg {
    #[cfg(windows)]
    {
        let _ = iface;
        WorkerMsg::Scan(simulated_scan(sim_tick))
    }
    #[cfg(not(windows))]
    {
        let _ = sim_tick;
        if iface.trim().is_empty() {
            return WorkerMsg::Error(
                "SCAN: no se detectó interfaz Wi-Fi. Revisa adaptador/rfkill.".to_string(),
            );
        }
        match run_iw_scan(iface) {
            Ok(output) => WorkerMsg::Scan(parse_iw_scan(&output)),
            Err(error) => {
                #[cfg(target_os = "linux")]
                {
                    if let Ok(link_output) = run_iw_link(iface) {
                        let info = parse_iw_link(&link_output);
                        if info.connected {
                            return WorkerMsg::Link(info);
                        }
                    }
                    WorkerMsg::Error(format!("SCAN[{iface}]: {}", short_error(&error)))
                }
                #[cfg(not(target_os = "linux"))]
                {
                    let _ = error;
                    WorkerMsg::Scan(simulated_scan(sim_tick))
                }
            }
        }
    }
}

#[cfg(not(windows))]
fn run_iw_scan(iface: &str) -> Result<String, String> {
    // Scanning requires root or CAP_NET_ADMIN; errors are surfaced to the UI.
    run_command("iw", &["dev", iface, "scan"])
}

#[cfg(target_os = "linux")]
fn run_iw_link(iface: &str) -> Result<String, String> {
    // Fallback when scan fails (e.g. busy/rfkill/permissions): read link state.
    run_command("iw", &["dev", iface, "link"])
}

#[cfg(target_os = "linux")]
fn detect_wifi_ifaces() -> Vec<String> {
    let mut ifaces = Vec::new();
    let Ok(output) = run_command("iw", &["dev"]) else {
        return ifaces;
    };
    for line in output.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix("Interface ") {
            let name = name.trim();
            if !name.is_empty() && !ifaces.iter().any(|i: &String| i == name) {
                ifaces.push(name.to_string());
            }
        }
    }
    // Prefer wlan0 first for predictable default on GPi Case 2W.
    if let Some(pos) = ifaces.iter().position(|i| i == "wlan0") {
        if pos != 0 {
            let iface = ifaces.remove(pos);
            ifaces.insert(0, iface);
        }
    }
    ifaces
}

#[cfg(target_os = "linux")]
fn detect_wifi_iface() -> Option<String> {
    detect_wifi_ifaces().into_iter().next()
}

#[cfg(not(target_os = "linux"))]
fn detect_wifi_ifaces() -> Vec<String> {
    #[cfg(windows)]
    {
        vec!["sim0".to_string()]
    }
    #[cfg(not(windows))]
    {
        vec!["wlan0".to_string()]
    }
}

#[cfg(not(windows))]
fn run_command(program: &str, args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("{program}: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        let detail = if detail.is_empty() {
            "fallo sin detalle".to_string()
        } else {
            short_error(detail)
        };
        return Err(format!("{program}: {detail}"));
    }
    let mut text = stdout.to_string();
    let has_stderr = !stderr.trim().is_empty();
    if has_stderr && !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    if has_stderr {
        text.push_str(&stderr);
    }
    Ok(text)
}

#[cfg(not(windows))]
fn short_error(text: &str) -> String {
    let cleaned = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.chars().count() > 90 {
        let mut clipped: String = cleaned.chars().take(90).collect();
        clipped.push_str("...");
        clipped
    } else {
        cleaned
    }
}

// ---------------- Parsing & logic ----------------

pub fn parse_iw_link(output: &str) -> LinkInfo {
    let mut info = LinkInfo::default();
    let lower = output.to_ascii_lowercase();
    if lower.contains("not connected") {
        info.connected = false;
        info.ssid = "N/D".to_string();
        info.bssid = "N/D".to_string();
        return info;
    }
    for raw in output.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("Connected to ") {
            // "Connected to aa:bb:cc:dd:ee:ff (on wlan0)"
            let bssid = rest.split_whitespace().next().unwrap_or("N/D");
            info.bssid = bssid.to_string();
            info.connected = true;
        } else if let Some(rest) = line.strip_prefix("SSID:") {
            info.ssid = rest.trim().to_string();
            info.connected = true;
        } else if let Some(rest) = line.strip_prefix("freq:") {
            if let Ok(freq) = rest.trim().parse::<u32>() {
                info.freq_mhz = Some(freq);
                info.channel = freq_to_channel(freq);
            }
        } else if let Some(rest) = line.strip_prefix("signal:") {
            // "-45 dBm"
            let dbm_part = rest.trim().split_whitespace().next().unwrap_or("");
            if let Ok(dbm) = dbm_part.parse::<i32>() {
                info.signal_dbm = Some(dbm);
            }
        } else if let Some(rest) = line.strip_prefix("tx bitrate:") {
            info.bitrate = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("rx bitrate:") {
            if info.bitrate == "N/D" {
                info.bitrate = rest.trim().to_string();
            }
        }
    }
    if info.connected && info.ssid.is_empty() {
        info.ssid = "N/D".to_string();
    }
    info
}

pub fn parse_iw_scan(output: &str) -> Vec<ScanEntry> {
    let mut entries: Vec<ScanEntry> = Vec::new();
    let mut current_bssid: Option<String> = None;
    let mut ssid = String::new();
    let mut freq: Option<u32> = None;
    let mut signal: Option<i32> = None;

    let flush = |entries: &mut Vec<ScanEntry>,
                 bssid: &mut Option<String>,
                 ssid: &mut String,
                 freq: &mut Option<u32>,
                 signal: &mut Option<i32>| {
        if let Some(b) = bssid.take() {
            entries.push(ScanEntry {
                ssid: if ssid.is_empty() {
                    "N/D".to_string()
                } else {
                    ssid.clone()
                },
                bssid: b,
                channel: freq.and_then(|f| freq_to_channel(f)),
                signal_dbm: *signal,
            });
        }
        ssid.clear();
        *freq = None;
        *signal = None;
    };

    for raw in output.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("BSS ") {
            flush(
                &mut entries,
                &mut current_bssid,
                &mut ssid,
                &mut freq,
                &mut signal,
            );
            let bssid = rest.split('(').next().unwrap_or("").trim().to_string();
            if !bssid.is_empty() {
                current_bssid = Some(bssid);
            }
        } else if let Some(rest) = line.strip_prefix("SSID:") {
            ssid = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("freq:") {
            freq = rest.trim().parse::<u32>().ok();
        } else if let Some(rest) = line.strip_prefix("signal:") {
            // "-45.00 dBm"
            let token = rest.trim().split_whitespace().next().unwrap_or("");
            if let Ok(v) = token.parse::<f32>() {
                signal = Some(v.round() as i32);
            } else if let Ok(v) = token.parse::<i32>() {
                signal = Some(v);
            }
        }
    }
    flush(
        &mut entries,
        &mut current_bssid,
        &mut ssid,
        &mut freq,
        &mut signal,
    );
    // dedupe by bssid keeping strongest
    entries.sort_by(|a, b| {
        b.signal_dbm
            .unwrap_or(-100)
            .cmp(&a.signal_dbm.unwrap_or(-100))
    });
    let mut seen: Vec<String> = Vec::new();
    entries.retain(|e| {
        if seen.iter().any(|s| s.eq_ignore_ascii_case(&e.bssid)) {
            false
        } else {
            seen.push(e.bssid.clone());
            true
        }
    });
    entries.truncate(32);
    entries
}

pub fn freq_to_channel(freq_mhz: u32) -> Option<u16> {
    match freq_mhz {
        2412..=2472 => Some(((freq_mhz - 2407) / 5) as u16),
        2484 => Some(14),
        5000..=5900 => Some(((freq_mhz - 5000) / 5) as u16),
        _ => None,
    }
}

pub fn parse_proc_net_wireless(output: &str, iface: &str) -> Option<i32> {
    for raw in output.lines() {
        let line = raw.trim();
        let Some(rest) = line.strip_prefix(iface) else {
            continue;
        };
        let rest = rest.trim_start_matches(':').trim();
        // fields: status link level noise ...
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let level_raw = parts[2].trim_end_matches('.');
        if let Ok(value) = level_raw.parse::<f32>() {
            let dbm = value.round() as i32;
            // /proc/net/wireless may report dBm (negative) or quality; assume dBm if negative
            if (-120..=0).contains(&dbm) {
                return Some(dbm);
            }
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalLevel {
    Optimal,
    Stable,
    Weak,
    Critical,
}

impl ThermalLevel {
    pub fn label(self) -> &'static str {
        match self {
            ThermalLevel::Optimal => "OPTIMO",
            ThermalLevel::Stable => "ESTABLE",
            ThermalLevel::Weak => "DEBIL",
            ThermalLevel::Critical => "CRITICO",
        }
    }

    pub fn color(self) -> Color {
        match self {
            ThermalLevel::Optimal => SUCCESS_COLOR,
            ThermalLevel::Stable => THERMAL_STABLE_COLOR,
            ThermalLevel::Weak => THERMAL_WEAK_COLOR,
            ThermalLevel::Critical => ERROR_COLOR,
        }
    }
}

pub fn thermal_level(dbm: i32) -> ThermalLevel {
    if dbm > -55 {
        ThermalLevel::Optimal
    } else if dbm >= -67 {
        ThermalLevel::Stable
    } else if dbm >= -78 {
        ThermalLevel::Weak
    } else {
        ThermalLevel::Critical
    }
}

pub fn signal_ratio(dbm: i32) -> f32 {
    let clamped = dbm.clamp(-90, -30) as f32;
    (clamped + 90.0) / 60.0
}

pub fn dbm_to_y(dbm: i32) -> i32 {
    dbm_to_y_range(dbm, -90, -30)
}

pub fn dbm_to_y_range(dbm: i32, v_min: i32, v_max: i32) -> i32 {
    let lo = v_min.min(v_max - 1);
    let hi = v_max;
    let clamped = dbm.clamp(lo, hi);
    let span = (hi - lo) as f32;
    let ratio = (clamped - lo) as f32 / span;
    let y = GRAPH_BOTTOM - 1 - (ratio * (GRAPH_H - 2) as f32) as i32;
    y.clamp(GRAPH_TOP, GRAPH_BOTTOM - 1)
}

pub fn signal_gradient(history: &VecDeque<i32>, window: usize) -> i32 {
    let w = window.max(1);
    let len = history.len();
    if len < w * 2 {
        return 0;
    }
    let recent_start = len - w;
    let prev_start = len - w * 2;
    let recent_avg = avg_range(history, recent_start, len);
    let prev_avg = avg_range(history, prev_start, recent_start);
    recent_avg - prev_avg
}

fn avg_range(history: &VecDeque<i32>, start: usize, end: usize) -> i32 {
    let mut sum: i32 = 0;
    let mut count: i32 = 0;
    for i in start..end {
        if let Some(v) = history.get(i) {
            sum += *v;
            count += 1;
        }
    }
    if count == 0 {
        0
    } else {
        (sum as f32 / count as f32).round() as i32
    }
}

fn blend(fg: Color, bg: Color) -> Color {
    // simple 1/4 tint for under-curve fill
    let fr = ((fg >> 16) & 0xFF) as u32;
    let fg_g = ((fg >> 8) & 0xFF) as u32;
    let fb = (fg & 0xFF) as u32;
    let br = ((bg >> 16) & 0xFF) as u32;
    let bg_g = ((bg >> 8) & 0xFF) as u32;
    let bb = (bg & 0xFF) as u32;
    let mix = |f: u32, b: u32| -> u8 { ((f + b * 3) / 4) as u8 };
    rgb(mix(fr, br), mix(fg_g, bg_g), mix(fb, bb))
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

// ---------------- Simulation (Windows/dev) ----------------

#[cfg(any(windows, not(target_os = "linux")))]
fn simulated_scan(tick: i32) -> Vec<ScanEntry> {
    let base = -55 - (tick % 25);
    vec![
        ScanEntry {
            ssid: "HZ-DEMO".to_string(),
            bssid: "AA:BB:CC:DD:EE:FF".to_string(),
            channel: Some(6),
            signal_dbm: Some(base.clamp(-90, -30)),
        },
        ScanEntry {
            ssid: "VECINA".to_string(),
            bssid: "11:22:33:44:55:66".to_string(),
            channel: Some(11),
            signal_dbm: Some((-72 + (tick % 7)).clamp(-90, -30)),
        },
        ScanEntry {
            ssid: "IOT".to_string(),
            bssid: "77:88:99:AA:BB:CC".to_string(),
            channel: Some(1),
            signal_dbm: Some((-84 + (tick % 5)).clamp(-90, -30)),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_link_extracts_fields() {
        let out = "Connected to aa:bb:cc:dd:ee:ff (on wlan0)\n\tSSID: MyNet\n\tfreq: 2437\n\tsignal: -45 dBm\n\ttx bitrate: 72.2 MBit/s\n";
        let info = parse_iw_link(out);
        assert!(info.connected);
        assert_eq!(info.bssid, "aa:bb:cc:dd:ee:ff");
        assert_eq!(info.ssid, "MyNet");
        assert_eq!(info.freq_mhz, Some(2437));
        assert_eq!(info.channel, Some(6));
        assert_eq!(info.signal_dbm, Some(-45));
        assert!(info.bitrate.contains("72.2"));
    }

    #[test]
    fn parse_link_not_connected() {
        let info = parse_iw_link("Not connected.");
        assert!(!info.connected);
        assert_eq!(info.ssid, "N/D");
    }

    #[test]
    fn parse_scan_extracts_bss() {
        let out = "BSS aa:bb:cc:dd:ee:ff(on wlan0)\n\tSSID: Net1\n\tfreq: 2412\n\tsignal: -50.00 dBm\nBSS 11:22:33:44:55:66(on wlan0)\n\tSSID: Net2\n\tfreq: 5180\n\tsignal: -70.00 dBm\n";
        let entries = parse_iw_scan(out);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].ssid, "Net1");
        assert_eq!(entries[0].channel, Some(1));
        assert_eq!(entries[0].signal_dbm, Some(-50));
        assert_eq!(entries[1].channel, Some(36));
    }

    #[test]
    fn gradient_detects_warmer_and_colder() {
        let mut h: VecDeque<i32> = VecDeque::new();
        for v in [-70, -70, -70, -60, -60, -60] {
            h.push_back(v);
        }
        assert_eq!(signal_gradient(&h, 3), 10);
        let mut h2: VecDeque<i32> = VecDeque::new();
        for v in [-60, -60, -60, -70, -70, -70] {
            h2.push_back(v);
        }
        assert_eq!(signal_gradient(&h2, 3), -10);
    }

    #[test]
    fn thermal_levels_classify() {
        assert_eq!(thermal_level(-50), ThermalLevel::Optimal);
        assert_eq!(thermal_level(-60), ThermalLevel::Stable);
        assert_eq!(thermal_level(-72), ThermalLevel::Weak);
        assert_eq!(thermal_level(-85), ThermalLevel::Critical);
    }

    #[test]
    fn zoom_samples() {
        assert_eq!(Zoom::X1.samples(), 240);
        assert_eq!(Zoom::X2.samples(), 120);
        assert_eq!(Zoom::X4.samples(), 60);
    }

    #[test]
    fn zoom_clips_history_to_latest_samples() {
        let mut h: VecDeque<i32> = (1..=240).collect();
        let take_n = Zoom::X4.samples().min(h.len());
        let visible: Vec<i32> = h.iter().rev().take(take_n).rev().copied().collect();
        assert_eq!(visible.len(), 60);
        assert_eq!(visible.first().copied(), Some(181));
        assert_eq!(visible.last().copied(), Some(240));
        h.push_back(241);
        let take_n = Zoom::X2.samples().min(h.len());
        let visible: Vec<i32> = h.iter().rev().take(take_n).rev().copied().collect();
        assert_eq!(visible.len(), 120);
        assert_eq!(visible.last().copied(), Some(241));
    }

    #[test]
    fn parse_proc_net_wireless_level() {
        let out = "Inter-| sta-|   Quality        |   Discarded packets\n face | tus | link level noise |\n wlan0: 0000   70.  -42.  -256        0\n";
        assert_eq!(parse_proc_net_wireless(out, "wlan0"), Some(-42));
    }
}
