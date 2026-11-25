//! Módulo Bleeding en Rust para HunterZeroR.
//! 
//! Versión migrada del módulo Python `modules/bleeding/screen.py`.
//! Implementa el trait ToolScreen para la arquitectura Rust.
//! El contenido se dibuja en el área usable del viewport virtual 320x240.
//! El header y footer se dibujan automáticamente por la aplicación principal.
//! 
//! Este módulo maneja Bluetooth scanning y deauth operations.
//! Pantalla diseñada para Raspberry Pi Zero W con adaptador Bluetooth.

// Módulo std y dependencias
use core::input::Action;

use ggez::graphics;
use ggez::Context;

// Tema y colores (consistentes con DISEÑO_INTERFAZ.md)
use crate::theme::{BG_COLOR, TEXT_COLOR, ACCENT_COLOR, HEADER_HEIGHT, FOOTER_HEIGHT, CONTENT_AREA_HEIGHT};

// Fase de configuración del módulo Bleeding
#[derive(Debug, Clone, PartialEq)]
enum BleedingPhase {
    /// Escaneando dispositivos Bluetooth
    Scanning,
    /// Modo de ataque seleccionado
    AttackMode,
    /// Configuración de objetivo
    TargetConfig,
    /// Listo para ejecutar
    Ready,
}

/// Estructura de estado del módulo Bleeding (migrado a Rust).
/// 
/// Almacena la configuración actual para operaciones Bluetooth:
/// - Dispositivo objetivo seleccionado
/// - Modo de operación actual
/// - Parámetros de tiempo
pub struct BleedingScreen {
    /// Nombre del dispositivo objetivo seleccionado
    target_device: String,
    /// Modo de operación actual
    mode: BleedingMode,
    /// Contador de tiempo/paquetes
    packet_count: u32,
    /// Fase actual de configuración
    phase: BleedingPhase,
}

/// Modo de operación para el módulo Bleeding
#[derive(Debug, Clone, PartialEq)]
enum BleedingMode {
    /// Escaneo pasivo de dispositivos Bluetooth
    Scan,
    /// Desautenticación de dispositivo objetivo
    Deauth,
    /// Fuerza bruta de handshake
    HandshakeCapture,
    /// Modo monitor puro
    Monitor,
}

impl BleedingScreen {
    /// Crear una nueva instancia de BleedingScreen con valores por defecto.
    /// 
    /// Valores por defecto: Modo Scan, sin dispositivo objetivo seleccionado.
    pub fn new() -> Self {
        Self {
            target_device: String::new(),
            mode: BleedingMode::Scan,
            packet_count: 1,
            phase: BleedingPhase::Scanning,
        }
    }
}

impl ToolScreen for BleedingScreen {
    /// Dibujar el contenido del módulo Bleeding SOLO en el área usable.
    /// 
    /// ¡NO dibuja header ni footer! Eso se hace en main.rs para consistencia.
    /// 
    /// El área de dibujo está en Y desde CONTENT_START_Y (20.0) hasta CONTENT_END_Y (220.0).
    fn draw_content(&self, ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult<()> {
        canvas.set_color(BG_COLOR);
        canvas.clear();

        // --- Contenido en el área usable de 200px (Y desde 20) ---
        
        // Título dentro del área usable
        let title_text = graphics::Text::new(ctx, "BLEEDING BT", 8)?;
        title_text.draw(
            ctx,
            graphics::Point::new(SCREEN_VIRTUAL_WIDTH as f32 / 2.0 - title_text.width(ctx)? / 2.0, 25.0),
            ACCENT_COLOR,
        )?;

        // Mostrar modo actual
        let mode_text = match self.mode {
            BleedingMode::Scan => "Modo: Escaneo BT",
            BleedingMode::Deauth => "Modo: Deauth",
            BleedingMode::HandshakeCapture => "Modo: Handshake",
            BleedingMode::Monitor => "Modo: Monitor",
        };
        let mode_instr = graphics::Text::new(ctx, mode_text, 8)?;
        mode_instr.draw(ctx, graphics::Point::new(10.0, 45.0), TEXT_COLOR)?;

        // Mostrar dispositivo objetivo
        let device_text = if self.target_device.is_empty() {
            "Dispositivo: Ninguno seleccionado"
        } else {
            &format!("Dispositivo: {}", self.target_device)
        };
        let device_instr = graphics::Text::new(ctx, device_text, 8)?;
        device_instr.draw(ctx, graphics::Point::new(10.0, 65.0), TEXT_COLOR)?;

        // Mostrar contador de paquetes / tiempo
        let packet_text = graphics::Text::new(ctx, &format!("Intentos: {}", self.packet_count), 8)?;
        packet_text.draw(ctx, graphics::Point::new(10.0, 85.0), TEXT_COLOR)?;

        // Instrucciones dentro del área usable (hasta Y=220)
        let instr_text = graphics::Text::new(
            ctx, 
            "D-pad: cambiar modo  A: seleccionar  START: ejecutar  B: cancelar", 
            7
        )?;
        instr_text.draw(ctx, graphics::Point::new(10.0, 105.0), TEXT_COLOR)?;

        // Mostrar fase actual
        let phase_text = match self.phase {
            BleedingPhase::Scanning => "Fase: Escaneando dispositivos BT",
            BleedingPhase::AttackMode => "Fase: Modo ataque seleccionado",
            BleedingPhase::TargetConfig => "Fase: Configurando objetivo",
            BleedingPhase::Ready => "Fase: Listo para ejecutar",
        };
        let phase_instr = graphics::Text::new(ctx, phase_text, 7)?;
        phase_instr.draw(ctx, graphics::Point::new(10.0, 125.0), TEXT_COLOR)?;

        Ok(())
    }

    /// Manejar acciones de entrada del D-pad, A, B, START.
    /// 
    /// Devuelve true si la acción fue consumida y no se necesita más procesamiento.
    fn handle_action(&mut self, action: &Action) -> bool {
        match action {
            // Izquierda/Derecha: cambiar modo de operación
            Action::Left => {
                // Cambiar modo anterior (wrap-around)
                self.mode = match self.mode {
                    BleedingMode::Scan => BleedingMode::Deauth,
                    BleedingMode::Deauth => BleedingMode::HandshakeCapture,
                    BleedingMode::HandshakeCapture => BleedingMode::Monitor,
                    BleedingMode::Monitor => BleedingMode::Scan,
                };
                true
            }
            Action::Right => {
                // Cambiar modo siguiente (wrap-around)
                self.mode = match self.mode {
                    BleedingMode::Scan => BleedingMode::Monitor,
                    BleedingMode::Monitor => BleedingMode::HandshakeCapture,
                    BleedingMode::HandshakeCapture => BleedingMode::Deauth,
                    BleedingMode::Deauth => BleedingMode::Scan,
                };
                true
            }
            // Arriba/Abajo: cambiar valor o fase
            Action::Up => {
                // Aumentar contador de intentos
                self.packet_count = self.packet_count.saturating_add(1);
                true
            }
            Action::Down => {
                // Disminuir contador de intentos (mínimo 1)
                if self.packet_count > 1 {
                    self.packet_count -= 1;
                }
                true
            }
            // A - Seleccionar modo o iniciar operación
            Action::Open => {
                // Cambiar a fase siguiente o iniciar según estado actual
                match self.phase {
                    BleedingPhase::Scanning => {
                        self.phase = BleedingPhase::AttackMode;
                    }
                    BleedingPhase::AttackMode => {
                        self.phase = BleedingPhase::TargetConfig;
                    }
                    BleedingPhase::TargetConfig => {
                        self.phase = BleedingPhase::Ready;
                    }
                    BleedingPhase::Ready => {
                        // Aquí se ejecutaría el comando bluetooth real
                        // Ejemplo: bluetoothctl scan on o hciconfig
                        self.phase = BleedingPhase::Scanning; // Reiniciar después de ejecutar
                    }
                }
                true
            }
            // B - Cancelar configuración, regresar al menú
            Action::Back => {
                // Reiniciar estado y volver al menú
                self.target_device = String::new();
                self.mode = BleedingMode::Scan;
                self.packet_count = 1;
                self.phase = BleedingPhase::Scanning;
                true
            }
            _ => true,
        }
    }

    /// Obtener hints para el footer (4 strings fijos consistentes en todas las vistas).
    fn footer_hints(&self) -> [&'static str; 4] {
        ["D-PAD Mover", "A MODO", "START Ejecutar", "B Volver"]
    }

    /// Limpiar recursos al cerrar esta pantalla.
    fn close(&mut self) {
        // Limpiar estado del módulo Bleeding
        self.target_device = String::new();
        self.mode = BleedingMode::Scan;
        self.packet_count = 1;
        self.phase = BleedingPhase::Scanning;
    }
}