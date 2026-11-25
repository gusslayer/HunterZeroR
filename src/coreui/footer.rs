//! Footer con hints de botones para HunterZeroR.
//!
//! Mirrors core/ui/footer.py from the Python version.
//! Panel inferior con indicaciones de D-PAD, A, B, START.

use ggez::graphics;
use ggez::Context;

use crate::theme::{BG_COLOR, TEXT_COLOR, SCREEN_VIRTUAL_WIDTH, SCREEN_VIRTUAL_HEIGHT};

/// Footer hints mostrados en la parte inferior de cada pantalla.
///
/// Formato consistente con Python:
/// - D-PAD MOVER (arriba/abajo)
/// - A ABRIR / Confirmar
/// - START ABRIR / Dentro de módulo
/// - B VOLVER / Cancelar
pub fn draw_footer(
    ctx: &mut Context,
    canvas: &mut graphics::Canvas,
) -> graphics::GameResult {
    // Panel inferior negro
    canvas.set_color(*BG_COLOR);
    canvas.draw_rect(
        graphics::Rect::new(0.0, SCREEN_VIRTUAL_HEIGHT as f32 - FOOTER_HEIGHT, SCREEN_VIRTUAL_WIDTH as f32, FOOTER_HEIGHT),
        graphics::Color::BLACK,
    )?;

    // Mismos hints que en Python
    let hints = [
        ("D-PAD Mover", 4),
        ("A Abrir", 116),
        ("START Ejecutar", 188),
        ("B Volver", 272),
    ];

    // Usar la fuente de 8px cargada en el contexto
    // La fuente PressStart2P-Regular.ttf debe estar montada en el contexto
    let font = graphics::Font::new(ctx, "assets/fonts/PressStart2P-Regular.ttf", 8)?;

    for (hint_text, x) in hints.iter() {
        let text = graphics::Text::new(&font, hint_text);
        text.draw(ctx, graphics::Point::new(*x as f32, SCREEN_VIRTUAL_HEIGHT as f32 - 15.0), *TEXT_COLOR)?;
    }

    Ok(())
}