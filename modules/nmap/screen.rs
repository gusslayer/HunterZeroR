//! Módulo Nmap para HunterZeroR.
//! 
//! Pantalla de escaneo de puertos TCP.
//! 
//! Implementa el trait ToolScreen. Contenido en área usable 221px.

use core::input::Action;

use ggez::graphics;
use ggez::Context;

use crate::theme::{BG_COLOR, TEXT_COLOR, ACCENT_COLOR};

/// Estructura de estado del módulo Nmap.
struct NmapScreen {
    /// Modo de escaneo actual
    scan_mode: ScanMode,
    /// Resultado del último escaneo (líneas seleccionadas)
    last_result: String,
}

/// Modo de escaneo disponible.
#[derive(Debug, Clone, PartialEq)]
enum ScanMode {
    /// Escaneo TCP connect simple
    TcpConnect,
    /// Escaneo de puertos top
    TopPorts,
    /// Escaneo con scripts vuln
    VulnScripts,
}

impl NmapScreen {
    pub fn new() -> Self {
        Self {
            scan_mode: ScanMode::TcpConnect,
            last_result: String::new(),
        }
    }
}

impl ToolScreen for NmapScreen {
    fn draw_content(&self, ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult<()> {
        canvas.set_color(BG_COLOR);
        canvas.clear();

        let title = graphics::Text::new(ctx, "ESCANEO NMAP", 8)?;
        title.draw(ctx, graphics::Point::new(10.0, 15.0), ACCENT_COLOR)?;

        let mode_text = match self.scan_mode {
            ScanMode::TcpConnect => "Modo: TCP connect (-sT)",
            ScanMode::TopPorts => "Modo: Top ports (-T2)",
            ScanMode::VulnScripts => "Modo: Scripts vuln",
        };
        let mode_instr = graphics::Text::new(ctx, mode_text, 7)?;
        mode_instr.draw(ctx, graphics::Point::new(10.0, 30.0), TEXT_COLOR)?;

        let instr = graphics::Text::new(ctx, "D-pad: cambiar modo  A: seleccionar  B: volver  START: escanear", 7)?;
        instr.draw(ctx, graphics::Point::new(10.0, 45.0), TEXT_COLOR)?;

        Ok(())
    }

    fn handle_action(&mut self, action: &Action) -> bool {
        match action {
            Action::Left | Action::Right => {
                // Cambiar modo de escaneo
                true
            }
            Action::Up | Action::Down => {
                // Navegar opciones dentro del modo
                true
            }
            Action::Open => {
                // A - Seleccionar modo o iniciar escaneo
                false
            }
            Action::Back => {
                // B - Cancelar, volver al menú
                true
            }
            _ => true,
        }
    }

    fn footer_hints(&self) -> [&'static str; 4] {
        ["D-PAD Mover", "A Abrir", "START Ejecutar", "B Volver"]
    }

    fn close(&mut self) {
        // Limpiar estado
    }
}