//! Module screens and the trait they implement.

use std::path::Path;

use crate::core::input::Action;
use crate::core::process_manager::ProcessManager;
use crate::core::system_stats::SystemStats;
use crate::gfx::Canvas;

pub mod bleeding;
pub mod logs;
pub mod nmap;
pub mod nslookup;
pub mod parameterized;
pub mod ping;
pub mod power;
pub mod proc_monitor;
pub mod system_monitor;
pub mod update;
pub mod wifi_survey;

/// What the application should do after a screen handles an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Stay,
    Close,
}

/// A full-screen module. Screens own their state; shared services are passed in
/// so a module cannot accidentally create a second process manager or HUD.
pub trait ToolScreen {
    fn title(&self) -> &str;

    fn handle(&mut self, action: Action, processes: &mut ProcessManager) -> Outcome;

    /// Called once per frame. Screens poll background work here.
    fn sync(&mut self, processes: &mut ProcessManager) {
        let _ = processes;
    }

    fn draw(&mut self, canvas: &mut Canvas, processes: &ProcessManager, stats: &SystemStats);

    /// Stop background work when the user leaves the module.
    fn close(&mut self, processes: &mut ProcessManager) {
        let _ = processes;
    }
}

/// Build the screen registered under `id`.
pub fn create(id: &str, root: &Path) -> Option<Box<dyn ToolScreen>> {
    match id {
        "bleeding" => Some(Box::new(bleeding::BleedingScreen::new())),
        "logs" => Some(Box::new(logs::LogsScreen::new(root))),
        "nmap" => Some(Box::new(nmap::NmapScreen::new())),
        "nslookup" => Some(Box::new(nslookup::NsLookupScreen::new())),
        "ping" => Some(Box::new(ping::PingScreen::new())),
        "power" => Some(Box::new(power::PowerScreen::new())),
        "proc_monitor" => Some(Box::new(proc_monitor::ProcMonitorScreen::new())),
        "system_monitor" => Some(Box::new(system_monitor::SystemMonitorScreen::new())),
        "update" => Some(Box::new(update::UpdateScreen::new(root))),
        "wifi_survey" => Some(Box::new(wifi_survey::WifiSurveyScreen::new(root))),
        _ => None,
    }
}
