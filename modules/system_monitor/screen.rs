//! Módulo System Monitor para HunterZeroR.
//! 
//! Métricas CPU, Memoria, Temperatura.
//! Contenido en área usable, sin títulos internos.

use core::input::Action;

use ggez::graphics;
use ggez::Context;

use crate::theme::{BG_COLOR, TEXT_COLOR, ACCENT_COLOR};

struct SystemMonitorScreen {
    cpu_val: f32,
    mem_val: f32,
    temp_val: f32,
}

impl SystemMonitorScreen {
    pub fn new() -> Self {
        Self {
            cpu_val: 0.0,
            mem_val: 0.0,
            temp_val: 0.0,
        }
    }
}

impl ToolScreen for SystemMonitorScreen {
    fn draw_content(&self, ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult<()> {
        canvas.set_color(BG_COLOR);
        canvas.clear();

        let text = graphics::Text::new(ctx, "MONITOR SISTEMA", 8)?;
        text.draw(ctx, graphics::Point::new(10.0, 10.0), ACCENT_COLOR)?;

        let cpu = graphics::Text::new(ctx, &format!("CPU: {:.0}%", self.cpu_val), 8)?;
        cpu.draw(ctx, graphics::Point::new(10.0, 25.0), TEXT_COLOR)?;

        let mem = graphics::Text::new(ctx, &format!("MEM: {:.0}MB", self.mem_val), 8)?;
        mem.draw(ctx, graphics::Point::new(10.0, 40.0), TEXT_COLOR)?;

        let temp = graphics::Text::new(ctx, &format!("TEMP: {:.1}°C", self.temp_val), 8)?;
        temp.draw(ctx, graphics::Point::new(10.0, 55.0), TEXT_COLOR)?;

        let instr = graphics::Text::new(ctx, "Actualizando... B: volver", 7)?;
        instr.draw(ctx, graphics::Point::new(10.0, 70.0), TEXT_COLOR)?;

        Ok(())
    }

    fn handle_action(&mut self, action: &Action) -> bool {
        match action {
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
        // Reiniciar valores
    }
}