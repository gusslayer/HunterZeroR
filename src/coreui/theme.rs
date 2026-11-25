//! Theme colors and font loading for HunterZeroR.
//! 
//! Color scheme: negro, blanco, naranja (accent)
//! Fixed layout: header and footer always visible, no window titles

use ggez::graphics;
use ggez::Context;

/// Negro de fondo - usado para header y footer
pub const BG_COLOR: graphics::Color = graphics::Color::from_rgb(0x00, 0x00, 0x00);
/// Blanco para texto - usado en contenido y stats
pub const TEXT_COLOR: graphics::Color = graphics::Color::from_rgb(0xFF, 0xFF, 0xFF);
/// Naranja acento - usado para resaltados y headers
pub const ACCENT_COLOR: graphics::Color = graphics::Color::from_rgb(0xFF, 0xA5, 0x00);

// Screen dimensions - virtual 320x240 scaled to 640x480
pub const SCREEN_VIRTUAL_WIDTH: u32 = 320;
pub const SCREEN_VIRTUAL_HEIGHT: u32 = 240;
pub const SCREEN_WIDTH: u32 = 640;
pub const SCREEN_HEIGHT: u32 = 480;
pub const FPS: u32 = 60;

// Layout constants - fixed positions for header/footer
/// Altura del header (20 píxeles fijos, solo para 3-4 iconos pequeños)
pub const HEADER_HEIGHT: f32 = 20.0;
/// Altura del footer (20 píxeles fijos, siempre visible)
pub const FOOTER_HEIGHT: f32 = 20.0;
/// Ancho disponible entre header y footer
pub const CONTENT_AREA_HEIGHT: f32 = SCREEN_VIRTUAL_HEIGHT - HEADER_HEIGHT - FOOTER_HEIGHT; // 200.0

/// Posición Y del header (siempre en la cima)
pub const HEADER_Y: f32 = 0.0;
/// Posición Y del footer (siempre en la base)
pub const FOOTER_Y: f32 = SCREEN_VIRTUAL_HEIGHT - FOOTER_HEIGHT; // 220.0