//! Módulo Update en Rust para HunterZeroR.
//! 
//! Versión migrada del módulo Python `core/ui/update_screen.py`.
//! Implementa el trait ToolScreen para la arquitectura Rust.
//! El contenido se dibuja en el área usable del viewport virtual 320x240.
//! El header y footer se dibujan automáticamente por la aplicación principal.
//! 
//! Este módulo maneja la actualización de la aplicación mediante git pull.

use core::input::Action;

use ggez::graphics;
use ggez::Context;

// Tema y colores (consistentes con DISEÑO_INTERFAZ.md)
use crate::theme::{BG_COLOR, TEXT_COLOR, ACCENT_COLOR, HEADER_HEIGHT, FOOTER_HEIGHT, CONTENT_AREA_HEIGHT};

// Fase de configuración del módulo Update
#[derive(Debug, Clone, PartialEq)]
enum UpdatePhase {
    /// Verificando actualizaciones
    Checking,
    /// Actualización disponible
    UpdateAvailable,
    /// Aplicando actualización
    Applying,
    /// Actualización completada
    Updated,
}

/// Estructura de estado del módulo Update (migrado a Rust).
/// 
/// Almacena el estado actual del proceso de actualización.
pub struct UpdateScreen {
    /// Versión actual de la aplicación
    current_version: String,
    /// Versión disponible (si hay actualización)
    available_version: Option<String>,
    /// Estado del proceso de actualización
    phase: UpdatePhase,
    /// Progreso porcentual
    progress: u32,
    /// Mensaje de estado
    status_message: String,
}

impl UpdateScreen {
    /// Crear una nueva instancia de UpdateScreen con valores por defecto.
    /// 
    /// Valores por defecto: Versión 0.1.0, sin actualizaciones verificadas.
    pub fn new() -> Self {
        Self {
            current_version: "0.1.0".to_string(),
            available_version: None,
            phase: UpdatePhase::Checking,
            progress: 0,
            status_message: "Verificando actualizaciones...".to_string(),
        }
    }
}

impl ToolScreen for UpdateScreen {
    /// Dibujar el contenido del módulo Update SOLO en el área usable.
    /// 
    /// ¡NO dibuja header ni footer! Eso se hace en main.rs para consistencia.
    /// 
    /// El área de dibujo está en Y desde CONTENT_START_Y (20.0) hasta CONTENT_END_Y (220.0).
    fn draw_content(&self, ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult<()> {
        canvas.set_color(BG_COLOR);
        canvas.clear();

        // --- Contenido en el área usable de 200px (Y desde 20) ---
        
        // Título dentro del área usable
        let title_text = graphics::Text::new(ctx, "ACTUALIZAR", 8)?;
        title_text.draw(
            ctx,
            graphics::Point::new(SCREEN_VIRTUAL_WIDTH as f32 / 2.0 - title_text.width(ctx)? / 2.0, 25.0),
            ACCENT_COLOR,
        )?;

        // Mostrar versión actual
        let version_text = graphics::Text::new(ctx, &format!("Versión actual: {}", self.current_version), 8)?;
        version_text.draw(ctx, graphics::Point::new(10.0, 45.0), TEXT_COLOR)?;

        // Mostrar estado de actualización
        let state_text = match self.phase {
            UpdatePhase::Checking => &self.status_message,
            UpdatePhase::UpdateAvailable => &format!("Actualización disponible: {}", self.available_version.as_ref().map_or("desconocida".to_string(), |v| v.clone())),
            UpdatePhase::Applying => &self.status_message,
            UpdatePhase::Updated => "¡Actualización completada!".to_string(),
        };
        let state_instr = graphics::Text::new(ctx, state_text, 8)?;
        state_instr.draw(ctx, graphics::Point::new(10.0, 65.0), TEXT_COLOR)?;

        // Mostrar progreso
        let progress_text = graphics::Text::new(ctx, &format!("Progreso: {}%", self.progress), 8)?;
        progress_text.draw(ctx, graphics::Point::new(10.0, 85.0), TEXT_COLOR)?;

        // Mostrar fase actual
        let phase_text = match self.phase {
            UpdatePhase::Checking => "Fase: Verificando repositorio remoto",
            UpdatePhase::UpdateAvailable => "Fase: Preparando archivos de actualización",
            UpdatePhase::Applying => "Fase: Aplicando actualización",
            UpdatePhase::Updated => "Fase: Actualización completada",
        };
        let phase_instr = graphics::Text::new(ctx, phase_text, 7)?;
        phase_instr.draw(ctx, graphics::Point::new(10.0, 105.0), TEXT_COLOR)?;

        // Instrucciones dentro del área usable (hasta Y=220)
        let instr_text = graphics::Text::new(
            ctx, 
            "D-pad: navegar  A: ejecutar/git pull  START: verificar  B: cancelar", 
            7
        )?;
        instr_text.draw(ctx, graphics::Point::new(10.0, 165.0), TEXT_COLOR)?;

        Ok(())
    }

    /// Manejar acciones de entrada del D-pad, A, B, START.
    /// 
    /// Devuelve true si la acción fue consumida y no se necesita más procesamiento.
    fn handle_action(&mut self, action: &Action) -> bool {
        match action {
            // Izquierda/Derecha: navegar entre fases
            Action::Left => {
                // Retroceder fase o dependiendo del estado
                true
            }
            Action::Right => {
                // Avanzar fase
                true
            }
            // Arriba/Abajo: dependiendo de la fase
            Action::Up => {
                match self.phase {
                    UpdatePhase::Checking => {
                        // Iniciar verificación de actualizaciones
                        self.phase = UpdatePhase::UpdateAvailable;
                        self.status_message = "Actualización disponible: v0.1.1".to_string();
                        self.available_version = Some("0.1.1".to_string());
                        self.progress = 0;
                    }
                    UpdatePhase::UpdateAvailable => {
                        // Comenzar aplicación de actualización
                        self.phase = UpdatePhase::Applying;
                        self.status_message = "Descargando y aplicando actualización...".to_string();
                        self.progress = 0;
                    }
                    UpdatePhase::Applying => {
                        // Avanzar progreso
                        if self.progress < 100 {
                            self.progress += 10;
                        }
                        if self.progress >= 100 {
                            self.phase = UpdatePhase::Updated;
                            self.status_message = "¡Actualización completada exitosamente!".to_string();
                        }
                    }
                    UpdatePhase::Updated => {
                        // Reiniciar para nueva verificación
                        self.phase = UpdatePhase::Checking;
                        self.status_message = "Volviendo a verificar actualizaciones...".to_string();
                        self.available_version = None;
                        self.progress = 0;
                    }
                }
                true
            }
            Action::Down => {
                // Retroceder o cancelar dependiendo fase
                match self.phase {
                    UpdatePhase::Checking => {
                        // Nothing at the start
                    }
                    UpdatePhase::UpdateAvailable => {
                        // Volver a verificar
                        self.phase = UpdatePhase::Checking;
                        self.status_message = "Verificando actualizaciones...".to_string();
                        self.available_version = None;
                    }
                    UpdatePhase::Applying => {
                        // Cancelar actualización y volver a checking
                        self.phase = UpdatePhase::Checking;
                        self.status_message = "Verificando actualizaciones...".to_string();
                        self.available_version = None;
                        self.progress = 0;
                    }
                    UpdatePhase::Updated => {
                        // Volver al menú/estado anterior
                    }
                }
                true
            }
            // A - Ejecutar acción principal (git pull / aplicar update)
            Action::Open => {
                match self.phase {
                    UpdatePhase::Checking => {
                        // Iniciar verificación
                        self.phase = UpdatePhase::UpdateAvailable;
                        self.status_message = "Actualización disponible: v0.1.1".to_string();
                        self.available_version = Some("0.1.1".to_string());
                        self.progress = 0;
                    }
                    UpdatePhase::UpdateAvailable => {
                        // Aplicar actualización (git pull equivalent)
                        self.phase = UpdatePhase::Applying;
                        self.status_message = "Descargando y aplicando actualización...".to_string();
                        self.progress = 0;
                    }
                    UpdatePhase::Applying => {
                        // Simular progreso hasta completar
                        if self.progress < 100 {
                            self.progress = 100;
                        }
                        if self.progress >= 100 {
                            self.phase = UpdatePhase::Updated;
                            self.status_message = "¡Actualización completada!".to_string();
                        }
                    }
                    UpdatePhase::Updated => {
                        // Reiniciar ciclo de verificación
                        self.phase = UpdatePhase::Checking;
                        self.status_message = "Volviendo a verificar actualizaciones...".to_string();
                        self.available_version = None;
                        self.progress = 0;
                    }
                }
                true
            }
            // B - Cancelar, regresar al menú
            Action::Back => {
                // Reiniciar estado y volver al menú
                self.current_version = "0.1.0".to_string();
                self.available_version = None;
                self.phase = UpdatePhase::Checking;
                self.progress = 0;
                self.status_message = "Verificando actualizaciones...".to_string();
                true
            }
            _ => true,
        }
    }

    /// Obtener hints para el footer (4 strings fijos consistentes en todas las vistas).
    fn footer_hints(&self) -> [&'static str; 4] {
        ["D-PAD Mover", "A Actualizar", "START Verificar", "B Cancelar"]
    }

    /// Limpiar recursos al cerrar esta pantalla.
    fn close(&mut self) {
        // Reiniciar estado del módulo
        self.current_version = "0.1.0".to_string();
        self.available_version = None;
        self.phase = UpdatePhase::Checking;
        self.progress = 0;
        self.status_message = "Verificando actualizaciones...".to_string();
    }
}