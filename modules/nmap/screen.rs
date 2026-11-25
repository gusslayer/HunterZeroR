//! Módulo Nmap en Rust para HunterZeroR.
//! 
//! Versión mejorada con opciones de checkboxes concatenables.
//! 
//! Implementa el trait ToolScreen para la arquitectura Rust.
//! El contenido se dibuja en el área usable del viewport virtual 320x240.
//! El header y footer se dibujan automáticamente por la aplicación principal.
//! 
//! Este módulo ahora incluye una serie de opciones nmap que se pueden
//! seleccionar mediante checkboxes para construir comandos de escaneo
//! completos y personalizables.

use core::input::Action;

use ggez::graphics;
use ggez::Context;

// Tema y colores (consistentes con MODULARIZACION.md)
use crate::theme::{BG_COLOR, TEXT_COLOR, ACCENT_COLOR, HEADER_HEIGHT, FOOTER_HEIGHT, CONTENT_AREA_HEIGHT};

// Fase de configuración del módulo Nmap
#[derive(Debug, Clone, PartialEq)]
enum NmapPhase {
    /// Seleccionando modo de escaneo
    SelectingMode,
    /// Modo seleccionando, configurando opciones checkbox
    ConfiguringOptions,
    /// Listo para ejecutar escaneo
    Ready,
}

/// Opción de escaneo Nmap con descripción
#[derive(Debug, Clone, PartialEq)]
enum NmapOption {
    /// Escaneo TCP connect (sT)
    TcpConnect,
    /// Escaneo SYN (sS) - requiere permisos
    SynScan,
    /// Escaneo UDP (sU)
    UdpScan,
    /// Escaneo rápido (-F)
    FastScan,
    /// Detección de versión (-sV)
    VersionDetection,
    /// Detección de SO (-O)
    OSDetection,
    /// Escaneo de puertos específicos (-p)
    PortSpecification,
    /// Escaneo agresivo (-A)
    AggressiveScan,
    /// Plantilla de tiempo - T4 (rápido)
    TimingTemplateT4,
    /// Plantilla de tiempo - T5 (muy rápido)
    TimingTemplateT5,
}

// Estado de una casilla de verificación
#[derive(Debug, Clone, PartialEq)]
struct Checkbox {
    /// Nombre de la opción
    name: &'static str,
    /// Descripción corta
    description: &'static str,
    /// Si está marcada
    checked: bool,
}

/// Estructura de estado del módulo Nmap (migrado a Rust).
/// 
/// Almacena la configuración actual para escaneo de puertos con
/// opciones seleccionables mediante checkboxes.
pub struct NmapScreen {
    /// Modo de escaneo seleccionado
    mode: NmapMode,
    /// Puerto objetivo o rango
    target_port: String,
    /// Dirección IP objetivo
    target_ip: String,
    /// Fase actual de configuración
    phase: NmapPhase,
    /// Resultados acumulados
    results: String,
    /// Opciones de nmap seleccionadas por checkbox
    options: [Checkbox; 10],
    /// Opción actualmente navegada (índice en el array options)
    option_cursor: usize,
}

impl NmapScreen {
    /// Crear una nueva instancia de NmapScreen con valores por defecto.
    /// 
    /// Valores por defecto: Modo TCP connect, puerto vacío, IP vacía,
    /// todas las opciones desmarcadas.
    pub fn new() -> Self {
        Self {
            mode: NmapMode::TcpConnect,
            target_port: String::new(),
            target_ip: String::new(),
            phase: NmapPhase::SelectingMode,
            results: String::new(),
            // Inicializar checkboxes con descripciones de opciones nmap
            options: [
                Checkbox {
                    name: "sT TCP Connect",
                    description: "Escaneo TCP conectar - más seguro, funciona en cualquier sistema",
                    checked: false,
                },
                Checkbox {
                    name: "sS SYN Scan",
                    description: "Escaneo SYN - stealth, requiere permisos de root",
                    checked: false,
                },
                Checkbox {
                    name: "sU UDP Scan",
                    description: "Escaneo UDP - detección puertos abiertos UDP",
                    checked: false,
                },
                Checkbox {
                    name: "-F Fast Scan",
                    description: "Escaneo rápido - fewer ports, veloz pero menos completo",
                    checked: false,
                },
                Checkbox {
                    name: "-sV Version",
                    description: "Detección de versión de servicio - qué software hay en los puertos",
                    checked: false,
                },
                Checkbox {
                    name: "-O OS Detect",
                    description: "Detección del sistema operativo remoto",
                    checked: false,
                },
                Checkbox {
                    name: "-p Puerto",
                    description: "Especificar puertos específicos (ej. -p 80,443 o -p 1-1000)",
                    checked: false,
                },
                Checkbox {
                    name: "-A Aggressive",
                    description: "Escaneo agresivo - version + OS + script traceroute",
                    checked: false,
                },
                Checkbox {
                    name: "-T4 Timing",
                    description: "Plantilla T4 - más rápido, menos sospechoso que T5",
                    checked: false,
                },
                Checkbox {
                    name: "-T5 Aggressive",
                    description: "Plantilla T5 - muy rápido, más detectable",
                    checked: false,
                },
            ],
            option_cursor: 0,
        }
    }
}

impl ToolScreen for NmapScreen {
    /// Dibujar el contenido del módulo Nmap SOLO en el área usable.
    /// 
    /// ¡NO dibuja header ni footer! Eso se hace en main.rs para consistencia.
    /// 
    /// El área de dibujo está en Y desde CONTENT_START_Y (20.0) hasta CONTENT_END_Y (220.0).
    fn draw_content(&self, ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult<()> {
        canvas.set_color(BG_COLOR);
        canvas.clear();

        // --- Contenido en el área usable de 200px (Y desde 20) ---
        
        // Título dentro del área usable
        let title_text = graphics::Text::new(ctx, "NMAP SCAN", 8)?;
        title_text.draw(
            ctx,
            graphics::Point::new(SCREEN_VIRTUAL_WIDTH as f32 / 2.0 - title_text.width(ctx)? / 2.0, 25.0),
            ACCENT_COLOR,
        )?;

        // Mostrar modo actual
        let mode_text = match self.mode {
            NmapMode::TcpConnect => "Modo: TCP connect (-sT)",
            NmapMode::SynScan => "Modo: SYN scan (-sS)",
            NmapMode::UdpScan => "Modo: UDP scan (-sU)",
            NmapMode::FastScan => "Modo: Escaneo rápido",
            NmapMode::Custom => "Modo: Personalizado",
        };
        let mode_instr = graphics::Text::new(ctx, mode_text, 8)?;
        mode_instr.draw(ctx, graphics::Point::new(10.0, 45.0), TEXT_COLOR)?;

        // Mostrar IP objetivo
        let ip_text = graphics::Text::new(ctx, &format!("IP objetivo: {}", self.target_ip), 8)?;
        ip_text.draw(ctx, graphics::Point::new(10.0, 65.0), TEXT_COLOR)?;

        // Mostrar puerto objetivo
        let port_text = graphics::Text::new(ctx, &format!("Puerto objetivo: {}", self.target_port), 8)?;
        port_text.draw(ctx, graphics::Point::new(10.0, 85.0), TEXT_COLOR)?;

        // Mostrar fase actual
        let phase_text = match self.phase {
            NmapPhase::SelectingMode => "Fase: Seleccionando modo de escaneo",
            NmapPhase::ConfiguringOptions => "Fase: Configurando opciones de escaneo",
            NmapPhase::Ready => "Fase: Listo para escanear",
        };
        let phase_instr = graphics::Text::new(ctx, phase_text, 7)?;
        phase_instr.draw(ctx, graphics::Point::new(10.0, 105.0), TEXT_COLOR)?;

        // --- Sección de checkboxes nmap ---
        // Empieza en Y=115, cada checkbox ocupa ~18px (14px texto + 4px espacio)
        let mut y_pos = 115f32;
        for (i, checkbox) in self.options.iter().enumerate() {
            // Resaltar la opción cursor
            let is_cursor = i == self.option_cursor;
            let color = if is_cursor { ACCENT_COLOR } else { TEXT_COLOR };
            
            // Mostrar nombre de la opción con posible marca
            let marker = if checkbox.checked { "[x]" } else { "[ ]" };
            let option_name = format!("{} {}", marker, checkbox.name);
            
            let option_text = graphics::Text::new(ctx, &option_name, 7)?;
            option_text.draw(ctx, graphics::Point::new(10.0, y_pos), color)?;
            
            // Mostrar descripción al lado derecho o abajo
            // Usamos las últimas líneas del área usable
            let desc_text = graphics::Text::new(ctx, checkbox.description, 6)?;
            // Descripción a la derecha de la casilla - aproximadamente en x=100
            // Calculamos posición: el texto tiene un ancho, lo ponemos centrado o a la derecha
            let desc_width = desc_text.width(ctx)? as f32;
            let desc_y = y_pos + 1.0; // un poco de padding vertical
            
            // Si hay cursor, desplazamos un poco la descripción
            let desc_x = if is_cursor { 130.0 } else { 110.0 };
            desc_text.draw(ctx, graphics::Point::new(desc_x, desc_y), TEXT_COLOR)?;
            
            y_pos += 18.0;
        }

        // Instrucciones dentro del área usable (hasta Y=220)
        let instr_text = graphics::Text::new(
            ctx, 
            "D-pad: navegar  Arriba/Abajo: toggle  A: seleccionar/confirmar  START: escanear  B: cancelar", 
            7
        )?;
        instr_text.draw(ctx, graphics::Point::new(10.0, 210.0), TEXT_COLOR)?;

        Ok(())
    }

    /// Manejar acciones de entrada del D-pad, A, B, START.
    /// 
    /// Devuelve true si la acción fue consumida y no se necesita más procesamiento.
    fn handle_action(&mut self, action: &Action) -> bool {
        match action {
            // Izquierda/Derecha: cambiar modo de escaneo
            Action::Left => {
                // Cambiar modo anterior (wrap-around)
                self.mode = match self.mode {
                    NmapMode::TcpConnect => NmapMode::UdpScan,
                    NmapMode::UdpScan => NmapMode::FastScan,
                    NmapMode::FastScan => NmapMode::SynScan,
                    NmapMode::SynScan => NmapMode::Custom,
                    NmapMode::Custom => NmapMode::TcpConnect,
                };
                true
            }
            Action::Right => {
                // Cambiar modo siguiente (wrap-around)
                self.mode = match self.mode {
                    NmapMode::TcpConnect => NmapMode::SynScan,
                    NmapMode::SynScan => NmapMode::FastScan,
                    NmapMode::FastScan => NmapMode::UdpScan,
                    NmapMode::UdpScan => NmapMode::Custom,
                    NmapMode::Custom => NmapMode::TcpConnect,
                };
                true
            }
            // Arriba/Abajo: navegar entre checkboxes o cambiar fase
            Action::Up => {
                // Mover cursor anterior (wrap-around)
                if self.option_cursor > 0 {
                    self.option_cursor -= 1;
                }
                true
            }
            Action::Down => {
                // Mover cursor siguiente (wrap-around)
                if self.option_cursor < self.options.len() - 1 {
                    self.option_cursor += 1;
                }
                true
            }
            // Action::Left/Right también podrían toggle, pero usaremos Arriba/Abajo para eso
            // Para toggle usaremos: la tecla A siempre togglea la opción cursor
            
            // Arriba/Abajo también pueden toggle si lo deseamos, pero mantengamos simple:
            // Action::Up y Action::Down solo mueven el cursor
            
            // A - Toggle la opción cursor o avanzar fase
            Action::Open => {
                match self.phase {
                    NmapPhase::SelectingMode => {
                        self.phase = NmapPhase::ConfiguringOptions;
                    }
                    NmapPhase::ConfiguringOptions => {
                        // Toggle la opción actualmente navegada
                        let idx = self.option_cursor;
                        self.options[idx].checked = !self.options[idx].checked;
                        // Si todas las opciones están marcadas, ir a Ready phase
                        let all_checked = self.options.iter().all(|c| c.checked);
                        if all_checked {
                            self.phase = NmapPhase::Ready;
                        }
                    }
                    NmapPhase::Ready => {
                        // Reiniciar después de escaneo
                        self.phase = NmapPhase::SelectingMode;
                    }
                }
                true
            }
            // B - Cancelar configuración, regresar al menú
            Action::Back => {
                // Reiniciar estado y volver al menú
                self.target_ip = String::new();
                self.target_port = String::new();
                self.phase = NmapPhase::SelectingMode;
                self.results = String::new();
                // Resetear todas las opciones
                for opt in self.options.iter_mut() {
                    opt.checked = false;
                }
                self.option_cursor = 0;
                true
            }
            _ => true,
        }
    }

    /// Obtener hints para el footer (4 strings fijos consistentes en todas las vistas).
    fn footer_hints(&self) -> [&'static str; 4] {
        ["D-PAD Mover", "A Seleccionar", "START Escanear", "B Cancelar"]
    }

    /// Limpiar recursos al cerrar esta pantalla.
    fn close(&mut self) {
        // Limpiar estado del módulo Nmap
        self.target_ip = String::new();
        self.target_port = String::new();
        self.phase = NmapPhase::SelectingMode;
        self.results = String::new();
        // Resetear todas las opciones
        for opt in self.options.iter_mut() {
            opt.checked = false;
        }
        self.option_cursor = 0;
    }
}