//! Módulo System Monitor en Rust para HunterZeroR.
//! 
//! Versión migrada del módulo Python `modules/system_monitor/screen.py`.
//! Implementa el trait ToolScreen para la arquitectura Rust.
//! El contenido se dibuja en el área usable del viewport virtual 320x240.
//! El header y footer se dibujan automáticamente por la aplicación principal.
//! 
//! Este módulo muestra métricas del sistema: CPU, memoria, temperatura y IP.

use core::input::Action;

use ggez::graphics;
use ggez::Context;

// Tema y colores (consistentes con DISEÑO_INTERFAZ.md)
use crate::theme::{BG_COLOR, TEXT_COLOR, ACCENT_COLOR, HEADER_HEIGHT, FOOTER_HEIGHT, CONTENT_AREA_HEIGHT};

// Fase de configuración del módulo System Monitor
#[derive(Debug, Clone, PartialEq)]
enum SysMonitorPhase {
    /// Mostrando métricas en tiempo real
    ShowingStats,
    /// Configurando qué métricas mostrar
    Configuring,
    /// Lista de procesos (para proc_monitor)
    ShowingProcesses,
}

/// Estructura de estado del módulo System Monitor (migrado a Rust).
/// 
/// Almacena las métricas del sistema actuales.
pub struct SystemMonitorScreen {
    /// Porcentaje de CPU actual
    cpu_percent: f32,
    /// Porcentaje de memoria actual
    mem_percent: f32,
    /// Temperatura actual (si disponible)
    temperature: Option<f32>,
    /// IP local
    local_ip: String,
    /// Fase actual de visualización
    phase: SysMonitorPhase,
    /// Actualizar contador
    update_count: u32,
}

impl SystemMonitorScreen {
    /// Crear una nueva instancia de SystemMonitorScreen con valores por defecto.
    /// 
    /// Valores por defecto: Todas las métricas en 0, fase ShowingStats.
    pub fn new() -> Self {
        Self {
            cpu_percent: 0.0,
            mem_percent: 0.0,
            temperature: None,
            local_ip: "127.0.0.1".to_string(),
            phase: SysMonitorPhase::ShowingStats,
            update_count: 0,
        }
    }
}

impl SystemMonitorScreen {
    /// Actualizar las métricas del sistema.
    /// 
    /// Esta función se llamaría periódicamente desde el main loop
    /// para actualizar los valores leídos de /proc y /sys.
    pub fn update_stats(&mut self, stats: &crate::core::system_stats::SystemStats) {
        self.cpu_percent = stats.cpu_percent;
        self.mem_percent = stats.mem_percent;
        self.temperature = stats.temperature;
        self.local_ip = stats.local_ip.clone();
        self.update_count += 1;
    }
}

impl ToolScreen for SystemMonitorScreen {
    /// Dibujar el contenido del módulo System Monitor SOLO en el área usable.
    /// 
    /// ¡NO dibuja header ni footer! Eso se hace en main.rs para consistencia.
    /// 
    /// El área de dibujo está en Y desde CONTENT_START_Y (20.0) hasta CONTENT_END_Y (220.0).
    fn draw_content(&self, ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult<()> {
        canvas.set_color(BG_COLOR);
        canvas.clear();

        // --- Contenido en el área usable de 200px (Y desde 20) ---
        
        // Título dentro del área usable
        let title_text = graphics::Text::new(ctx, "SYS MONITOR", 8)?;
        title_text.draw(
            ctx,
            graphics::Point::new(SCREEN_VIRTUAL_WIDTH as f32 / 2.0 - title_text.width(ctx)? / 2.0, 25.0),
            ACCENT_COLOR,
        )?;

        // Mostrar CPU
        let cpu_text = graphics::Text::new(ctx, &format!("CPU: {:.1}%", self.cpu_percent), 8)?;
        cpu_text.draw(ctx, graphics::Point::new(10.0, 45.0), TEXT_COLOR)?;

        // Mostrar Memoria
        let mem_text = graphics::Text::new(ctx, &format!("MEM: {:.1}%", self.mem_percent), 8)?;
        mem_text.draw(ctx, graphics::Point::new(10.0, 65.0), TEXT_COLOR)?;

        // Mostrar Temperatura (si disponible)
        let temp_text = if let Some(temp) = self.temperature {
            &format!("TEMP: {:.1}°C", temp)
        } else {
            "TEMP: N/A"
        };
        let temp_instr = graphics::Text::new(ctx, temp_text, 8)?;
        temp_instr.draw(ctx, graphics::Point::new(10.0, 85.0), TEXT_COLOR)?;

        // Mostrar IP local
        let ip_text = graphics::Text::new(ctx, &format!("IP: {}", self.local_ip), 8)?;
        ip_text.draw(ctx, graphics::Point::new(10.0, 105.0), TEXT_COLOR)?;

        // Mostrar fase actual
        let phase_text = match self.phase {
            SysMonitorPhase::ShowingStats => "Fase: Mostrando métricas en tiempo real",
            SysMonitorPhase::Configuring => "Fase: Configurando qué mostrar",
            SysMonitorPhase::ShowingProcesses => "Fase: Lista de procesos",
        };
        let phase_instr = graphics::Text::new(ctx, phase_text, 7)?;
        phase_instr.draw(ctx, graphics::Point::new(10.0, 125.0), TEXT_COLOR)?;

        // Contador de actualizaciones
        let count_text = graphics::Text::new(ctx, &format!("Actualizaciones: {}", self.update_count), 7)?;
        count_text.draw(ctx, graphics::Point::new(10.0, 145.0), TEXT_COLOR)?;

        // Instrucciones dentro del área usable (hasta Y=220)
        let instr_text = graphics::Text::new(
            ctx, 
            "D-pad: siguiente métrica  A: actualizar  START: ver procesos  B: cancelar", 
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
            // Izquierda/Derecha: siguiente/anterior métrica
            Action::Left => {
                // Mover a métrica anterior o configurar
                true
            }
            Action::Right => {
                // Mover a siguiente métrica o configurar
                true
            }
            // Arriba/Abajo: navegar fases
            Action::Up => {
                match self.phase {
                    SysMonitorPhase::ShowingStats => {
                        self.phase = SysMonitorPhase::Configuring;
                    }
                    SysMonitorPhase::Configuring => {
                        self.phase = SysMonitorPhase::ShowingProcesses;
                    }
                    SysMonitorPhase::ShowingProcesses => {
                        self.phase = SysMonitorPhase::ShowingStats;
                    }
                }
                true
            }
            Action::Down => {
                match self.phase {
                    SysMonitorPhase::ShowingStats => {
                        self.phase = SysMonitorPhase::Configuring;
                    }
                    SysMonitorPhase::Configuring => {
                        self.phase = SysMonitorPhase::ShowingProcesses;
                    }
                    SysMonitorPhase::ShowingProcesses => {
                        self.phase = SysMonitorPhase::ShowingStats;
                    }
                }
                true
            }
            // A - Actualizar métricas o aceptar configuración
            Action::Open => {
                match self.phase {
                    SysMonitorPhase::ShowingStats => {
                        // Actualizar valores desde el system stats
                        // En implementación real, leeríamos de /proc/stat, /proc/meminfo, etc.
                    }
                    SysMonitorPhase::Configuring => {
                        // Aplicar configuración y volver a mostrar
                        self.phase = SysMonitorPhase::ShowingStats;
                    }
                    SysMonitorPhase::ShowingProcesses => {
                        // Volver al modo de métricas
                        self.phase = SysMonitorPhase::ShowingStats;
                    }
                }
                true
            }
            // B - Cancelar/configurar, regresar al menú
            Action::Back => {
                // Reiniciar estado y volver al menú
                self.cpu_percent = 0.0;
                self.mem_percent = 0.0;
                self.temperature = None;
                self.local_ip = "127.0.0.1".to_string();
                self.phase = SysMonitorPhase::ShowingStats;
                self.update_count = 0;
                true
            }
            _ => true,
        }
    }

    /// Obtener hints para el footer (4 strings fijos consistentes en todas las vistas).
    fn footer_hints(&self) -> [&'static str; 4] {
        ["D-PAD Mover", "A Actualizar", "START Procesos", "B Cancelar"]
    }

    /// Limpiar recursos al cerrar esta pantalla.
    fn close(&mut self) {
        // Reiniciar estado del módulo
        self.cpu_percent = 0.0;
        self.mem_percent = 0.0;
        self.temperature = None;
        self.local_ip = "127.0.0.1".to_string();
        self.phase = SysMonitorPhase::ShowingStats;
        self.update_count = 0;
    }
}