//! Módulo Proc Monitor para HunterZeroR.
//! 
//! Lista de procesos activos.
//! Contenido en área usable, sin títulos internos.

use core::input::Action;

use ggez::graphics;
use ggez::Context;

use crate::theme::{BG_COLOR, TEXT_COLOR, ACCENT_COLOR};

struct ProcMonitorScreen {
    processes: Vec<String>,
    selected: usize,
}

impl ProcMonitorScreen {
    pub fn new() -> Self {
        Self {
            processes: vec!["---".to_string(); 8], // Placeholder
            selected: 0,
        }
    }
}

impl ToolScreen for ProcMonitorScreen {
    fn draw_content(&self, ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult<()> {
        canvas.set_color(BG_COLOR);
        canvas.clear();

        let text = graphics::Text::new(ctx, "PROCESOS ACTIVOS", 8)?;
        text.draw(ctx, graphics::Point::new(10.0, 10.0), ACCENT_COLOR)?;

        for (i, proc) in self.processes.iter().enumerate() {
            let color = if i as usize == self.selected { ACCENT_COLOR } else { TEXT_COLOR };
            let instr = graphics::Text::new(ctx, proc, 8)?;
            instr.draw(ctx, graphics::Point::new(10.0, (i + 1) as f32 * 15.0 + 5.0), color)?;
        }

        let instr = graphics::Text::new(ctx, "B: volver arriba/abajo seleccionar", 7)?;
        instr.draw(ctx, graphics::Point::new(10.0, 120.0), TEXT_COLOR)?;

        Ok(())
    }

    fn handle_action(&mut self, action: &Action) -> bool {
        match action {
            core::input::Action::Up => {
                self.selected = if self.selected > 0 { self.selected - 1 } else { self.processes.len() - 1 };
                true
            }
            core::input::Action::Down => {
                self.selected = (self.selected + 1) % self.processes.len();
                true
            }
            core::input::Action::Back => {
                // Volver al menú
                true
            }
            _ => true,
        }
    }

    fn footer_hints(&self) -> [&'static str; 4] {
        ["D-PAD Mover", "A Abrir", "START Ejecutar", "B Volver"]
    }

    fn close(&mut self) {
        // Reiniciar
    }
}