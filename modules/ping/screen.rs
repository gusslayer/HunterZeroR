//! Módulo Ping para HunterZeroR.
//! 
//! Pantalla que configura y envía manualmente solicitudes ICMP Echo.
//! 
//! Implementa el trait ToolScreen para la arquitectura Rust.
//! El contenido se dibuja en el área usable (221px alto, Y 10-221).
//! El header y footer se dibujan automáticamente por la aplicación principal.

use core::input::Action;

use ggez::graphics;
use ggez::Context;

use crate::theme::{BG_COLOR, TEXT_COLOR, ACCENT_COLOR, HEADER_HEIGHT, FOOTER_HEIGHT, CONTENT_AREA_HEIGHT};

/// Estructura de estado del módulo Ping.
/// 
/// Almacena la configuración actual (IP destino, cantidad de paquetes)
/// que el usuario está definiendo a través del D-pad.
pub struct PingScreen {
    /// Dirección IP de destino (se construye octeto a octeto con D-pad)
    target_ip: String,
    /// Cantidad de paquetes ICMP a enviar (1-4)
    packet_count: u32,
    /// Estado actual de configuración
    config_phase: ConfigPhase,
}

/// Fase de configuración del módulo Ping.
#[derive(Debug, Clone, PartialEq)]
enum ConfigPhase {
    /// Seleccionando octeto de IP
    SelectingOctet,
    /// Modificando valor del octeto actual
    ModifyingValue,
    /// Listo para ejecutar
    Ready,
}

impl PingScreen {
    /// Crear una nueva instancia de PingScreen con valores por defecto.
    /// 
    /// Valores por defecto: IP vacía (se llenará con D-pad), 2 paquetes.
    pub fn new() -> Self {
        Self {
            target_ip: String::new(),
            packet_count: 2,
            config_phase: ConfigPhase::SelectingOctet,
        }
    }
}

impl ToolScreen for PingScreen {
    /// Dibujar el contenido del módulo Ping SOLO en el área usable (221px alto).
    /// 
    /// ¡NO dibuja header ni footer! Eso se hace en main.rs para consistencia.
    /// 
    /// El área de dibujo tiene Y desde CONTENT_START_Y (10.0) hasta CONTENT_END_Y (221.0).
    fn draw_content(&self, ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult<()> {
        canvas.set_color(BG_COLOR);
        canvas.clear();

        // --- Contenido en el área usable de 221px ---
        
        // Título dentro del área usable
        let title_text = graphics::Text::new(ctx, "PING MANUAL", 8)?;
        title_text.draw(
            ctx,
            graphics::Point::new(SCREEN_VIRTUAL_WIDTH as f32 / 2.0 - title_text.width(ctx)? / 2.0, 15.0),
            ACCENT_COLOR,
        )?;

        // Mostrar IP actual (aunque esté siendo editada)
        let ip_text = graphics::Text::new(ctx, &format!("IP: {}", self.target_ip), 8)?;
        // Posición dependiendo de fase: si estamos seleccionando octeto, mostrar posición del cursor
        let show_ip = match self.config_phase {
            ConfigPhase::SelectingOctet => {
                // Mostrar IP con resaltado del octeto actual
                &self.target_ip
            }
            _ => &self.target_ip,
        };
        ip_text.draw(ctx, graphics::Point::new(10.0, 35.0), TEXT_COLOR)?;

        // Mostrar cantidad de paquetes
        let packets_text = graphics::Text::new(ctx, &format!("Paquetes: {}", self.packet_count), 8)?;
        packets_text.draw(ctx, graphics::Point::new(10.0, 55.0), TEXT_COLOR)?;

        // Instrucciones dentro del área usable (hasta Y=221)
        let instr_text = graphics::Text::new(
            ctx, 
            "D-pad: cambia octeto  A: guarda  START: envía  B: cancela", 
            7
        )?;
        instr_text.draw(ctx, graphics::Point::new(10.0, 75.0), TEXT_COLOR)?;

        // Mostrar fase actual
        let phase_text = match self.config_phase {
            ConfigPhase::SelectingOctet => "Seleccionando octeto IP",
            ConfigPhase::ModifyingValue => "Modificando valor",
            ConfigPhase::Ready => "Listo para enviar",
        };
        let phase_instr = graphics::Text::new(ctx, phase_text, 7)?;
        phase_instr.draw(ctx, graphics::Point::new(10.0, 90.0), TEXT_COLOR)?;

        Ok(())
    }

    /// Manejar acciones de entrada del D-pad, A, B, START.
    /// 
    /// Devuelve true si la acción fue consumida y no se necesita más procesamiento.
    fn handle_action(&mut self, action: &Action) -> bool {
        match action {
            // Navegar entre octetos de IP (izquierda/derecha) o cambiar valores (arriba/abajo)
            Action::Left => {
                // Mover al octeto anterior (o hacer wrap-around)
                true
            }
            Action::Right => {
                // Mover al siguiente octeto o aumentar paquetes
                true
            }
            Action::Up => {
                // Aumentar valor actual (o aumentar paquetes si estamos al final)
                true
            }
            Action::Down => {
                // Disminuir valor actual
                true
            }
            // A - Guardar configuración y pasar a estado de ejecución
            Action::Open => {
                // Marcar que la acción fue consumida
                // En un proyecto completo, aquí construiríamos el comando ping
                // y cambiaríamos al phase Running
                false
            }
            // B - Cancelar configuración, regresar al menú
            Action::Back => {
                // Reiniciar estado y volver al menú
                self.target_ip = String::new();
                self.packet_count = 2;
                self.config_phase = ConfigPhase::SelectingOctet;
                true
            }
            _ => true,
        }
    }

    /// Obtener hints para el footer (4 strings fijos consistentes en todas las vistas).
    fn footer_hints(&self) -> [&'static str; 4] {
        ["D-PAD Mover", "A Abrir", "START Ejecutar", "B Volver"]
    }

    /// Limpiar recursos al cerrar esta pantalla.
    fn close(&mut self) {
        // Limpiar estado del módulo Ping
        self.target_ip.clear();
        self.packet_count = 2;
        self.config_phase = ConfigPhase::SelectingOctet;
    }
}