/**
 * HunterZeroR - Bucle principal con UI simplificado
 * - Header: SÓLO íconos de sistema (sin texto)
 * - Footer: UN Solo conjunto fijo de hints (sin cambios de estado)
 * - Sin títulos de módulo, sin texto de recursos
 * - Contenido solo en el área usable (Y=20 a Y=219)
 */

use ggez;
use ggez::{Context, GameResult};
use ggez::graphics;
use ggez::input::keyboard;
use std::path::Path;

// === DISEÑO UI FIXTO ===
// Usar constantes del tema para consistencia
use crate::theme::{BG_COLOR, TEXT_COLOR, ACCENT_COLOR, SCREEN_VIRTUAL_WIDTH, SCREEN_VIRTUAL_HEIGHT, HEADER_HEIGHT, FOOTER_HEIGHT, CONTENT_AREA_HEIGHT};

// Modos y fases del módulo Nmap (versión simplificada para main.rs)
#[derive(Debug, Clone)]
enum NmapMode {
    TcpConnect,
    SynScan,
    UdpScan,
    FastScan,
    Custom,
}

// Fase simplificada del módulo Nmap
#[derive(Debug, Clone, PartialEq)]
enum NmapPhaseSimple {
    SelectingMode,
    ConfiguringOptions,
    Ready,
}

// Opciones nmap para la versión simplificada
#[derive(Debug, Clone)]
enum NmapOption {
    TcpConnect,
    SynScan,
    UdpScan,
    FastScan,
    VersionDetect,
    OsDetection,
    PortSpec,
    Aggressive,
    TimingT4,
    TimingT5,
}

// Configuración de pantalla GPi Case 2W
// Se lee del archivo /boot/firmware/config.txt al iniciar la aplicación
// Si no existe, continúa con valores por defecto y emite advertencia

// Alturas fijas (del tema: HEADER_HEIGHT=20, FOOTER_HEIGHT=20, CONTENT_AREA_HEIGHT=200)
// Derivadas
const CONTENT_START_Y: f32 = HEADER_HEIGHT; // 20.0
const CONTENT_END_Y: f32 = SCREEN_VIRTUAL_HEIGHT as f32 - FOOTER_HEIGHT; // 220.0
const FOOTER_Y: f32 = SCREEN_VIRTUAL_HEIGHT as f32 - FOOTER_HEIGHT; // 220.0
const HEADER_Y: f32 = 0.0;

// === Configuración GPi Case 2W ===

/// Estructura de configuración de pantalla GPi Case 2W
#[derive(Debug, Clone)]
pub struct GpiCase2WConfig {
    pub dpi_overlay: bool,
    pub hactive: u32,
    pub vactive: u32,
    pub clock_frequency: u32,
    pub audio_pwm_mode: u32,
}

static mut GPi_CASE_2W_CONFIG: Option<GpiCase2WConfig> = None;

/// Ruta default de configuración de pantalla
const SCREEN_CONFIG_PATH: &str = "/boot/firmware/config.txt";

/// Leer y parsear la configuración de GPi Case 2W desde /boot/firmware/config.txt
///
/// Esta función se llama al inicio de la aplicación y verifica que la
/// configuración de pantalla requerida esté presente. Si no encuentra
/// los parámetros esperados, emite una advertencia pero continúa (la
/// aplicación seguirá funcionando con configuración por defecto).
fn leer_config_gpi_case2w() -> Option<GpiCase2WConfig> {
    // Solo en Raspberry Pi con /boot/firmware
    if !std::path::Path::new(SCREEN_CONFIG_PATH).exists() {
        println!("WARNING: No se encontró {} - usando configuración por defecto", SCREEN_CONFIG_PATH);
        return None;
    }
    
    let Ok(contenido) = std::fs::read_to_string(SCREEN_CONFIG_PATH) else {
        println!("WARNING: No se pudo leer {} - usando configuración por defecto", SCREEN_CONFIG_PATH);
        return None;
    };
    
    let mut dpi_overlay = false;
    let mut hactive: Option<u32> = None;
    let mut vactive: Option<u32> = None;
    let mut clock_frequency: Option<u32> = None;
    let mut audio_pwm_mode: Option<u32> = None;
    
    for linea in contenido.lines() {
        let linea = linea.trim();
        
        // Verificar dtoverlay=vc4-kms-dpi-generic
        if linea.starts_with("dtoverlay=vc4-kms-dpi-generic") || 
           linea.contains("dtoverlay=vc4-kms-dpi-generic") {
            dpi_overlay = true;
        }
        
        // Extraer hactive=
        if let Some(rest) = linea.strip_prefix("hactive=") {
            if let Ok(val) = rest.split_whitespace().next().parse::<u32>() {
                hactive = Some(val);
            }
        }
        // También apoyar el formato con signo igual sin espacios
        if hactive.is_none() {
            if let Some(rest) = linea.strip_prefix("hactive=") {
                // Tomar solo el primer valor numérico
                let primera_palabra = rest.split_whitespace().next().unwrap_or("");
                if let Ok(val) = primera_palabra.parse::<u32>() {
                    hactive = Some(val);
                }
            }
        }
        
        // Extraer vactive=
        if let Some(rest) = linea.strip_prefix("vactive=") {
            if let Ok(val) = rest.split_whitespace().next().parse::<u32>() {
                vactive = Some(val);
            }
        }
        
        // Extraer clock-frequency=
        if let Some(rest) = linea.strip_prefix("clock-frequency=") {
            if let Ok(val) = rest.split_whitespace().next().parse::<u32>() {
                clock_frequency = Some(val);
            }
        }
        
        // Extraer audio_pwm_mode=
        if let Some(rest) = linea.strip_prefix("audio_pwm_mode=") {
            if let Ok(val) = rest.split_whitespace().next().parse::<u32>() {
                audio_pwm_mode = Some(val);
            }
        }
    }
    
    let config = GpiCase2WConfig {
        dpi_overlay: dpi_overlay,
        hactive: hactive.unwrap_or(640),
        vactive: vactive.unwrap_or(480),
        clock_frequency: clock_frequency.unwrap_or(24000000),
        audio_pwm_mode: audio_pwm_mode.unwrap_or(2),
    };
    
    Some(config)
}

/// Aplicar la configuración leída a la memoria estática
fn aplicar_config_gpi_case2w() {
    unsafe {
        GPi_CASE_2W_CONFIG = leer_config_gpi_case2w();
    }
    
    if let Some(config) = unsafe { GPi_CASE_2W_CONFIG } {
        println!("Configuración GPi Case 2W cargada:");
        println!("  - hactive: {}", config.hactive);
        println!("  - vactive: {}", config.vactive);
        println!("  - clock-frequency: {} Hz", config.clock_frequency);
        println!("  - audio_pwm_mode: {}", config.audio_pwm_mode);
        if config.dpi_overlay {
            println!("  - dtoverlay: vc4-kms-dpi-generic (activado)");
        }
    } else {
        println!("Usando configuración por defecto de pantalla GPi Case 2W");
    }
}

// === MÓDULOS DEL SISTEMA ===

mod core;
mod core::input;
mod core::plugins;
mod core::state;
mod core::system_stats;
mod core::ui::hud;
mod modules::ping::screen::PingScreen;
mod modules::nmap::screen::NmapScreen;
mod modules::nslookup::screen::NsLookupScreen;
mod modules::bleeding::screen::BleedingScreen;
mod modules::system_monitor::screen::SystemMonitorScreen;
mod modules::proc_monitor::screen::ProcMonitorScreen;

// === Trait para pantallas de módulos ===

trait ToolScreen: Send + 'static {
    fn draw_content(&self, ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult<()>;
    fn handle_action(&mut self, action: &core::input::Action) -> bool;
    fn footer_hints(&self) -> [&'static str; 4];
    fn close(&mut self);
}

// === Implementaciones de pantallas (simplificadas) ===

struct PingScreen;

impl ToolScreen for PingScreen {
    fn draw_content(&self, ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult<()> {
        canvas.set_color(BG_COLOR);
        canvas.clear();

        // --- Contenido en el área usable de 200px (Y desde 20) ---
        // Título dentro del área usable (empezando en Y=20, no Y=15 que está en header)
        let title_text = graphics::Text::new(ctx, "PING MANUAL", 8)?;
        title_text.draw(
            ctx,
            graphics::Point::new(SCREEN_VIRTUAL_WIDTH as f32 / 2.0 - title_text.width(ctx)? / 2.0, 25.0),
            ACCENT_COLOR,
        )?;

        // Mostrar IP actual (aunque esté siendo editada)
        let ip_text = graphics::Text::new(ctx, "IP: ", 8)?;
        ip_text.draw(ctx, graphics::Point::new(10.0, 45.0), TEXT_COLOR)?;

        // Mostrar cantidad de paquetes
        let packets_text = graphics::Text::new(ctx, "Paquetes: 2", 8)?;
        packets_text.draw(ctx, graphics::Point::new(10.0, 65.0), TEXT_COLOR)?;

        // Instrucciones dentro del área usable (hasta Y=220)
        let instr_text = graphics::Text::new(
            ctx, 
            "D-pad: cambia octeto  A: guarda  START: envía  B: cancela", 
            7
        )?;
        instr_text.draw(ctx, graphics::Point::new(10.0, 85.0), TEXT_COLOR)?;

        // Mostrar fase actual
        let phase_text = "Seleccionando octeto IP";
        let phase_instr = graphics::Text::new(ctx, phase_text, 7)?;
        phase_instr.draw(ctx, graphics::Point::new(10.0, 100.0), TEXT_COLOR)?;

        Ok(())
    }

    fn handle_action(&mut self, action: &core::input::Action) -> bool {
        match action {
            // Navegar entre octetos de IP (izquierda/derecha) o cambiar valores (arriba/abajo)
            core::input::Action::Left => {
                // Mover al octeto anterior (o hacer wrap-around)
                true
            }
            core::input::Action::Right => {
                // Mover al siguiente octeto o aumentar paquetes
                true
            }
            core::input::Action::Up => {
                // Aumentar valor actual (o aumentar paquetes si estamos al final)
                true
            }
            core::input::Action::Down => {
                // Disminuir valor actual
                true
            }
            // A - Guardar configuración y pasar a estado de ejecución
            core::input::Action::Open => {
                // Marcar que la acción fue consumida
                false
            }
            // B - Cancelar configuración, regresar al menú
            core::input::Action::Back => {
                // Reiniciar estado y volver al menú
                true
            }
            _ => true,
        }
    }

    fn footer_hints(&self) -> [&'static str; 4] {
        ["D-PAD Mover", "A Abrir", "START Ejecutar", "B Volver"]
    }

    fn close(&mut self) {}
}

// Módulo Nmap mejorado con opciones de checkboxes.
// Versión simplificada para compatibilidad hacia atrás; la versión completa
// con checkboxes está en modules/nmap/screen.rs y se carga por el PluginCatalog.

struct NmapScreen {
    mode: NmapMode,
    target_port: String,
    target_ip: String,
    phase: NmapPhaseSimple,
    options: Vec<NmapOption>,
    option_selected: usize,
}

impl ToolScreen for NmapScreen {
    fn draw_content(&self, ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult<()> {
        canvas.set_color(BG_COLOR);
        canvas.clear();

        let title = graphics::Text::new(ctx, "ESCANEO NMAP", 8)?;
        title.draw(ctx, graphics::Point::new(10.0, 25.0), ACCENT_COLOR)?;

        let mode_text = "Modo: TCP connect (-sT)";
        let mode_instr = graphics::Text::new(ctx, mode_text, 7)?;
        mode_instr.draw(ctx, graphics::Point::new(10.0, 40.0), TEXT_COLOR)?;

        let instr = graphics::Text::new(ctx, "D-pad: cambiar modo  A: seleccionar  B: volver  START: escanear", 7)?;
        instr.draw(ctx, graphics::Point::new(10.0, 55.0), TEXT_COLOR)?;

        Ok(())
    }

    fn handle_action(&mut self, action: &core::input::Action) -> bool {
        match action {
            // Left | Right: Cambiar modo de escaneo
            core::input::Action::Left | core::input::Action::Right => {
                self.mode = match self.mode {
                    NmapMode::TcpConnect => NmapMode::UdpScan,
                    NmapMode::UdpScan => NmapMode::FastScan,
                    NmapMode::FastScan => NmapMode::SynScan,
                    NmapMode::SynScan => NmapMode::Custom,
                    NmapMode::Custom => NmapMode::TcpConnect,
                };
                true
            }
            // Up | Down: mover selector de opciones o cambiar fase
            core::input::Action::Up => {
                if self.option_selected > 0 {
                    self.option_selected -= 1;
                }
                true
            }
            core::input::Action::Down => {
                if self.option_selected < self.options.len().saturating_sub(1) {
                    self.option_selected += 1;
                }
                true
            }
            // A - Toggle opción o avanzar fase
            core::input::Action::Open => {
                match self.phase {
                    NmapPhaseSimple::SelectingMode => {
                        self.phase = NmapPhaseSimple::ConfiguringOptions;
                    }
                    NmapPhaseSimple::ConfiguringOptions => {
                        // En implementación completa, aquí se togglearía la opción seleccionada
                    }
                    NmapPhaseSimple::Ready => {
                        self.phase = NmapPhaseSimple::SelectingMode;
                    }
                }
                true
            }
            // B - Cancelar, volver al menú
            core::input::Action::Back => {
                self.target_ip = String::new();
                self.target_port = String::new();
                self.phase = NmapPhaseSimple::SelectingMode;
                self.options.clear();
                self.option_selected = 0;
                true
            }
            _ => true,
        }
    }

    fn footer_hints(&self) -> [&'static str; 4] {
        ["D-PAD Mover", "A Seleccionar", "START Escanear", "B Cancelar"]
    }

    fn close(&mut self) {
        // Limpiar estado del módulo Nmap
        self.target_ip = String::new();
        self.target_port = String::new();
        self.phase = NmapPhaseSimple::SelectingMode;
        self.options.clear();
        self.option_selected = 0;
    }
}

// ========================================================
// PUNTO DE ENTRADA
// ========================================================

fn main() -> GameResult {
    // Aplicar configuración de pantalla GPi Case 2W al iniciar
    aplicar_config_gpi_case2w();
    
    let cb = ggez::Conf::new()
        .title("HunterZeroR")
        .window_mode(ggez::graphics::Rect::new(0.0, 0.0, SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32))
        .window_setup(ggez::conf::WindowSetup::new().resizable(false).sample_count(0));

    // Desactivar antialiasing para mantener píxeles nítidos (según DISENO_INTERFAZ.md)
    let (ctx, events_loop) = ggez::Context::builder(cb)
        .frame_time_per_frame(std::time::Duration::from_secs(1) / 60u32)
        .build()?;

    // Montar fuente
    ctx.filesystem.mount(
        &Path::new("assets").join("font").join("PressStart2P-Regular.ttf"),
        "assets/fonts/PressStart2P-Regular.ttf",
    )?;

    let mut app = HunterZeroApp::new(&mut ctx)?;

    ggez::event::poll(&ctx, |event| {
        use ggez::event::Event;
        if let Event::KeyDown { key_code: Some(key), .. } = event {
            use ggez::input::keyboard::KeyCode;
            match key {
                KeyCode::Escape => {
                    app.running = false;
                    return false;
                }
                KeyCode::Space | KeyCode::Return => {
                    // A/START - confirmar
                    if app.tool_screen.is_some() {
                        if let Some(screen) = app.tool_screen.as_mut() {
                            let consumed = screen.handle_action(&core::input::Action::Open);
                            if consumed {
                                screen.close();
                                app.tool_screen = None;
                            }
                        }
                    }
                    true
                }
                KeyCode::B => {
                    // B - cancelar
                    if app.tool_screen.is_some() {
                        if let Some(screen) = app.tool_screen.take() {
                            screen.close();
                        }
                    }
                    true
                }
                _ => true,
            }
        } else {
            true
        }
    });

    Ok(())
}

// Estructura HunterZeroApp simplificada
struct HunterZeroApp {
    font_8: graphics::Font,
    state: core::state::AppState,
    selected_module: Option<String>,
    frame_counter: u32,
    catalog: core::plugins::PluginCatalog,
    ui_state: core::state::UIState,
    process_manager: core::process_manager::ProcessManager,
    stats_reader: SystemStats,
    last_stats: f64,
    last_heartbeat: f64,
    running: bool,
    tool_screen: Option<Box<dyn ToolScreen>>,
}

impl HunterZeroApp {
    fn new(ctx: &mut Context) -> GameResult<Self> {
        let font_8 = graphics::Font::new(ctx, "assets/fonts/PressStart2P-Regular.ttf", 8)?;
        let catalog = core::plugins::PluginCatalog::new(Path::new("modules"));
        let ui_state = core::state::UIState::new();
        let stats_reader = SystemStats::read();

        Ok(Self {
            font_8,
            state: core::state::AppState::Menu(vec![]),
            selected_module: None,
            frame_counter: 0,
            catalog,
            ui_state,
            process_manager: core::process_manager::ProcessManager::new(Path::new(".").to_path_buf(), None),
            stats_reader,
            last_stats: 0.0,
            last_heartbeat: 0.0,
            running: true,
            tool_screen: None,
        })
    }
}