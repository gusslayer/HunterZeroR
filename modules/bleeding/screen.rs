//! Módulo Bleeding para HunterZeroR.
//! 
//! Bluetooth LE / BR-EDR scanning.
//! Contenido en área usable, sin títulos internos.

use core::input::Action;

use ggez::graphics;
use ggez::Context;

use crate::theme::{BG_COLOR, TEXT_COLOR, ACCENT_COLOR};

struct BleedingScreen {
    scan_type: String, // BR-EDR o BLE
    duration: u32,
}

impl BleedingScreen {
    pub fn new() -> Self {
        Self {
            scan_type: "BR-EDR".to_string(),
            duration: 60,
        }
    }
}

impl ToolScreen for BleedingScreen {
    fn draw_content(&self, ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult<()> {
        canvas.set_color(BG_COLOR);
        canvas.clear();

        let text = graphics::Text::new(ctx, "BLUETOOTH SCAN", 8)?;
        text.draw(ctx, graphics::Point::new(10.0, 10.0), ACCENT_COLOR)?;

        let type_text = graphics::Text::new(ctx, &format!("Tipo: {}", self.scan_type), 8)?;
        type_text.draw(ctx, graphics::Point::new(10.0, 25.0), TEXT_COLOR)?;

        let dur_text = graphics::Text::new(ctx, &format!("Duración: {}s", self.duration), 8)?;
        dur_text.draw(ctx, graphics::Point::new(10.0, 40.0), TEXT_COLOR)?;

        let instr = graphics::Text::new(ctx, "D-pad: cambiar tipo A: iniciar B: cancelar START: configurar", 7)?;
        instr.draw(ctx, graphics::Point::new(10.0, 55.0), TEXT_COLOR)?;

        Ok(())
    }

    fn handle_action(&mut self, action: &Action) -> bool {
        match action {
            core::input::Action::Left | core::input::Action::Right => {
                // Cambiar tipo BR-EDR/BLE
                true
            }
            core::input::Action::Up | core::input::Action::Down => {
                // Ajustar duración
                true
            }
            core::input::Action::Open => { false } // A - iniciar o configurar
            core::input::Action::Back => { 
                self.scan_type = "BR-EDR".to_string(); 
                self.duration = 60; 
                true 
            }
            _ => true,
        }
    }

    fn footer_hints(&self) -> [&'static str; 4] {
        ["D-PAD Mover", "A Abrir", "START Ejecutar", "B Volver"]
    }

    fn close(&mut self) {
        // Limpiar
    }
}