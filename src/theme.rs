//! Colours and fixed layout shared by every screen.
//!
//! The interface is authored on a 320x240 virtual canvas and scaled to
//! 640x480. All drawing code uses these constants so a module cannot drift
//! away from the GPi Case 2W layout.

/// A packed 32-bit colour in `0xAARRGGBB` order.
pub type Color = u32;

pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
    0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

pub const BG_COLOR: Color = rgb(0, 0, 0);
pub const TEXT_COLOR: Color = rgb(255, 255, 255);
pub const ACCENT_COLOR: Color = rgb(255, 85, 0);
pub const MUTED_COLOR: Color = rgb(102, 102, 102);
pub const PANEL_COLOR: Color = rgb(12, 12, 12);
pub const SELECTED_BG: Color = rgb(58, 20, 0);
pub const GRID_COLOR: Color = rgb(38, 38, 38);
pub const SUCCESS_COLOR: Color = rgb(120, 200, 120);
pub const ERROR_COLOR: Color = rgb(255, 110, 90);
pub const THERMAL_STABLE_COLOR: Color = rgb(120, 220, 200);
pub const THERMAL_WEAK_COLOR: Color = rgb(230, 190, 60);

/// Width of the virtual canvas.
pub const SCREEN_WIDTH: u32 = 320;
/// Height of the virtual canvas.
pub const SCREEN_HEIGHT: u32 = 240;
/// Width of the scaled window on a desktop machine.
#[allow(dead_code)]
pub const WINDOW_WIDTH: u32 = 640;
/// Height of the scaled window on a desktop machine.
#[allow(dead_code)]
pub const WINDOW_HEIGHT: u32 = 480;
/// Refresh rate. The Python original uses 15 FPS, which is comfortable on a
/// Pi Zero W and keeps the software renderer cheap.
pub const FPS: u32 = 15;

/// Height of the live telemetry strip (HUD).
pub const HEADER_HEIGHT: i32 = 27;
/// First usable content row (below the HUD).
pub const CONTENT_TOP: i32 = 28;
/// First row of the footer.
pub const FOOTER_TOP: i32 = 211;
/// Height of the footer.
pub const FOOTER_HEIGHT: i32 = SCREEN_HEIGHT as i32 - FOOTER_TOP;
/// Exclusive last usable content row (above the footer).
pub const CONTENT_BOTTOM: i32 = FOOTER_TOP;
/// Height of the usable content area.
pub const CONTENT_HEIGHT: i32 = CONTENT_BOTTOM - CONTENT_TOP;
