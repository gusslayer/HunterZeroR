//! Módulo NSLookup para HunterZeroR.
//! 
//! Resolución DNS interactiva.
//! Contenido en área usable, sin títulos internos.

use core::input::Action;

use ggez::graphics;
use ggez::Context;

use crate::theme::{BG_COLOR, TEXT_COLOR, ACCENT_COLOR};

struct NsLookupScreen {
    target_domain: String,
}

impl NsLookupScreen {
    pub fn new() -> Self {
        Self {
            target_domain: String::new(),
        }
    }
}

impl ToolScreen for NsLookupScreen {
    fn draw_content(&self, ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult<()> {
        canvas.set_color(BG_COLOR);
        canvas.clear();

        let text = graphics::Text::new(ctx, "NSLOOKUP DNS", 8)?;
        text.draw(ctx, graphics::Point::new(10.0, 10.0), ACCENT_COLOR)?;

        let domain = graphics::Text::new(ctx, &format!("Dominio: {}", self.target_domain), 8)?;
        domain.draw(ctx, graphics::Point::new(10.0, 25.0), TEXT_COLOR)?;

        let instr = graphics::Text::new(ctx, "D-pad: mover cursor A: seleccionar DEL: borrar B: cancelar START: validar", 7)?;
        instr.draw(ctx, graphics::Point::new(10.0, 40.0), TEXT_COLOR)?;

        Ok(())
    }

    fn handle_action(&mut self, action: &Action) -> bool {
        match action {
            core::input::Action::Left | core::input::Action::Right => true,
            core::input::Action::Up | core::input::Action::Down => true,
            core::input::Action::Open => { false } // A - seleccionar/validar
            core::input::Action::Back => { 
                self.target_domain = String::new(); 
                true 
            }
            _ => true,
        }
    }

    fn footer_hints(&self) -> [&'static str; 4] {
        ["D-PAD Mover", "A Abrir", "START Ejecutar", "B Volver"]
    }

    fn close(&mut self) {
        self.target_domain.clear();
    }
}