//! Native display and input backends.
//!
//! The application renders into a 320x240 software canvas. A backend only has
//! to present that canvas and translate native events into `Action`s. This
//! keeps the UI independent of OpenGL/GLES versions, which is essential on the
//! Pi Zero W (VideoCore IV, GLES 2.0 only).

use crate::core::input::Action;
use crate::gfx::Canvas;

pub trait Backend {
    /// Blit the virtual canvas to the physical display.
    fn present(&mut self, canvas: &Canvas) -> Result<(), String>;
    /// Drain pending input. May contain `Action::Quit`.
    fn poll(&mut self) -> Vec<Action>;
    /// Human-readable backend name for logs.
    fn description(&self) -> String;
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub fn create() -> Result<Box<dyn Backend>, String> {
    Ok(Box::new(linux::FbBackend::new()?))
}

#[cfg(windows)]
pub fn create() -> Result<Box<dyn Backend>, String> {
    Ok(Box::new(windows::GdiBackend::new()?))
}

#[cfg(not(any(target_os = "linux", windows)))]
pub fn create() -> Result<Box<dyn Backend>, String> {
    Err("Esta plataforma no tiene backend de pantalla.".to_string())
}
