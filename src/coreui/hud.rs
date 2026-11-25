//! HUD (Heads-Up Display) for HunterZeroR.
//!
//! UI simplificado: header SOLAMENTE iconos, footer SOLAMENTE hints fijos.
//! Sin títulos de módulo, sin texto de recursos, sin cambios de estado.

use ggez::graphics;
use ggez::Context;

// ============================================================
// DISEÑO UI: Sin títulos, solo iconos, hints fijos
// ============================================================

// Usar constantes del tema para consistencia
use crate::theme::{BG_COLOR, TEXT_COLOR, ACCENT_COLOR, SCREEN_VIRTUAL_WIDTH, SCREEN_VIRTUAL_HEIGHT, HEADER_HEIGHT, FOOTER_HEIGHT, CONTENT_AREA_HEIGHT};

// Alturas fijas de la UI (SIN títulos, maximizar espacio)
// Estos valores vienen del tema y deben coincidir:
// HEADER_HEIGHT = 20.0, FOOTER_HEIGHT = 20.0, CONTENT_AREA_HEIGHT = 200.0

const HEADER_Y: f32 = 0.0;
const FOOTER_Y: f32 = SCREEN_VIRTUAL_HEIGHT as f32 - FOOTER_HEIGHT; // 220.0
const CONTENT_START_Y: f32 = HEADER_HEIGHT; // 20.0
const CONTENT_END_Y: f32 = FOOTER_Y; // 220.0

// Colores oficiales (importados del tema)

// --- Header: SÓLO ÍCONOS SISTEMA (sin texto) ---
//
// Tres iconos pequeños en la parte superior:
// 1. Ícono de red/conectividad (izquierda)
// 2. Ícono de estado (centro)
// 3. Ícono de información (derecha)
//
// Sin texto "HUNTERZERO", sin "CPU: MEM: WiFi:"
// Solo gráficos pixel art de 16x16 o 20x20 píxeles cada uno

/// Dibuja el header SOLAMENTE con iconos de sistema.
///
/// NO dibuja texto de recursos (CPU, MEM, WiFi) ni título "HUNTERZERO".
/// Solo 3-4 iconos pequeños en la fila superior.
pub fn draw_icon_header(
    ctx: &mut Context,
    canvas: &mut graphics::Canvas,
) -> graphics::GameResult {
    // Fondo header oscuro
    canvas.set_color(*BG_COLOR);
    canvas.draw_rect(
        graphics::Rect::new(0.0, HEADER_Y, SCREEN_VIRTUAL_WIDTH as f32, HEADER_HEIGHT),
        graphics::Color::BLACK,
    )?;

    // Ícono 1: Conectividad (izquierda - x=4)
    // Representa señal de red/IP
    let icon1 = graphics::Text::new(ctx, "⎋", 12)?; // glifo pequeño
    icon1.draw(ctx, graphics::Point::new(4.0, 4.0), *ACCENT_COLOR)?;

    // Ícono 2: Estado (centro)
    // Representa métricas/sistema
    let icon2 = graphics::Text::new(ctx, "⎊", 12)?; // glifo de estado
    icon2.draw(
        ctx,
        graphics::Point::new(SCREEN_VIRTUAL_WIDTH as f32 / 2.0 - 8.0, 4.0),
        *ACCENT_COLOR,
    )?;

    // Ícono 3: Información (derecha)
    // Representa configuración/ayuda
    let icon3 = graphics::Text::new(ctx, "ℹ", 12)?;
    icon3.draw(
        ctx,
        graphics::Point::new(SCREEN_VIRTUAL_WIDTH as f32 - 20.0, 4.0),
        *ACCENT_COLOR,
    )?;

    Ok(())
}

/// Dibuja el footer fijo SOLAMENTE con hints de navegación.
///
/// Siempre el mismo conjunto de 4 hints, NUNCA cambia de estado.
/// Evita "dobles opciones" o hints conflictivos.
pub fn draw_fixed_footer(
    ctx: &mut Context,
    canvas: &mut graphics::Canvas,
) -> graphics::GameResult {
    // Fondo footer oscuro
    canvas.set_color(*BG_COLOR);
    canvas.draw_rect(
        graphics::Rect::new(0.0, FOOTER_Y, SCREEN_VIRTUAL_WIDTH as f32, FOOTER_HEIGHT),
        graphics::Color::BLACK,
    )?;

    // Un ÚNICO conjunto de hints fijos - consistente en TODAS las vistas
    // Sin cambios dependiendo del estado del módulo
    let hints = [
        // Izquierda: Navegación D-Pad
        ("D-PAD Mover", 4),
        // Centro-izquierda: Confirmar
        ("A Abrir", 116),
        // Centro-derecha: Ejecutar/Start
        ("START Ejecutar", 188),
        // Derecha: Retroceso
        ("B Volver", 272),
    ];

    // Dibujar cada hint exactamente UNA VEZ en su posición X fija
    for (hint_text, x) in hints.iter() {
        let text = graphics::Text::new(ctx, hint_text, 8)?;
        text.draw(
            ctx,
            graphics::Point::new(*x as f32, FOOTER_Y + 1.0),
            *TEXT_COLOR,
        )?;
    }

    Ok(())
}

/// Prepara el área de contenido usable entre header y footer.
pub fn draw_content_area(
    ctx: &mut Context,
    canvas: &mut graphics::Canvas,
) -> graphics::GameResult {
    // Limpiar el área usable (entre header y footer)
    canvas.set_color(*BG_COLOR);
    canvas.draw_rect(
        graphics::Rect::new(0.0, CONTENT_START_Y, SCREEN_VIRTUAL_WIDTH as f32, CONTENT_END_Y - CONTENT_START_Y),
        graphics::Color::BLACK,
    )?;

    Ok(())
}