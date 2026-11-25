/**
 * HunterZeroR - Bucle principal con UI simplificado
 * - Header: SÓLO íconos de sistema (sin texto)
 * - Footer: UN Solo conjunto fijo de hints (sin cambios de estado)
 * - Sin títulos de módulo, sin texto de recursos
 */

use ggez;
use ggez::{Context, GameResult};
use ggez::graphics;
use ggez::input::keyboard;
use std::path::Path;

// === DISEÑO UI FIXTO ===
// Usar constantes del tema para consistencia
use crate::theme::{BG_COLOR, TEXT_COLOR, ACCENT_COLOR, SCREEN_VIRTUAL_WIDTH, SCREEN_VIRTUAL_HEIGHT, HEADER_HEIGHT, FOOTER_HEIGHT, CONTENT_AREA_HEIGHT};

// Alturas fijas (del tema: HEADER_HEIGHT=20, FOOTER_HEIGHT=20, CONTENT_AREA_HEIGHT=200)
// Derivadas
const CONTENT_START_Y: f32 = HEADER_HEIGHT; // 20.0
const CONTENT_END_Y: f32 = SCREEN_VIRTUAL_HEIGHT as f32 - FOOTER_HEIGHT; // 220.0
const FOOTER_Y: f32 = SCREEN_VIRTUAL_HEIGHT as f32 - FOOTER_HEIGHT; // 220.0
const HEADER_Y: f32 = 0.0;

// === MÓDULOS DEL SISTEMA ===

mod core;
mod core::input;
mod core::plugins;
mod core::state;
mod core::system_stats;
mod core::process_manager;
mod core::ui::hud;
mod core::ui::footer;
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

        // --- Contenido en el área usable de 200px ---
        
        // Título dentro del área usable
        let title_text = graphics::Text::new(ctx, "PING MANUAL", 8)?;
        title_text.draw(
            ctx,
            graphics::Point::new(SCREEN_VIRTUAL_WIDTH as f32 / 2.0 - title_text.width(ctx)? / 2.0, 15.0),
            ACCENT_COLOR,
        )?;

        // Mostrar IP actual (aunque esté siendo editada)
        let ip_text = graphics::Text::new(ctx, "IP: ", 8)?;
        ip_text.draw(ctx, graphics::Point::new(10.0, 35.0), TEXT_COLOR)?;

        // Mostrar cantidad de paquetes
        let packets_text = graphics::Text::new(ctx, "Paquetes: 2", 8)?;
        packets_text.draw(ctx, graphics::Point::new(10.0, 55.0), TEXT_COLOR)?;

        // Instrucciones dentro del área usable (hasta Y=220)
        let instr_text = graphics::Text::new(
            ctx, 
            "D-pad: cambia octeto A: guarda START: envía B: cancela", 
            7
        )?;
        instr_text.draw(ctx, graphics::Point::new(10.0, 75.0), TEXT_COLOR)?;

        // Mostrar fase actual
        let phase_text = "Seleccionando octeto IP";
        let phase_instr = graphics::Text::new(ctx, phase_text, 7)?;
        phase_instr.draw(ctx, graphics::Point::new(10.0, 90.0), TEXT_COLOR)?;

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

// Iguales simplificadas para los demás módulos...

struct NmapScreen;

impl ToolScreen for NmapScreen {
    fn draw_content(&self, ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult<()> {
        canvas.set_color(BG_COLOR);
        canvas.clear();

        let title = graphics::Text::new(ctx, "ESCANEO NMAP", 8)?;
        title.draw(ctx, graphics::Point::new(10.0, 15.0), ACCENT_COLOR)?;

        let mode_text = "Modo: TCP connect (-sT)";
        let mode_instr = graphics::Text::new(ctx, mode_text, 7)?;
        mode_instr.draw(ctx, graphics::Point::new(10.0, 30.0), TEXT_COLOR)?;

        let instr = graphics::Text::new(ctx, "D-pad: cambiar modo A: seleccionar B: volver START: escanear", 7)?;
        instr.draw(ctx, graphics::Point::new(10.0, 45.0), TEXT_COLOR)?;

        Ok(())
    }

    fn handle_action(&mut self, action: &core::input::Action) -> bool {
        match action {
            // Left | Right: Cambiar modo de escaneo
            core::input::Action::Left | core::input::Action::Right => {
                true
            }
            // Up | Down: Navegar opciones dentro del modo
            core::input::Action::Up | core::input::Action::Down => {
                true
            }
            // A - Seleccionar modo o iniciar escaneo
            core::input::Action::Open => {
                false
            }
            // B - Cancelar, volver al menú
            core::input::Action::Back => {
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

// ========================================================
// PUNTO DE ENTRADA
// ========================================================

fn main() -> GameResult {
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
        let modules = vec![
            // Módulos registrados
        ];
        let catalog = PluginCatalog::new(Path::new("modules"));
        let ui_state = core::state::UIState::new();
        let stats_reader = SystemStats::read();

        Ok(Self {
            font_8,
            state: core::state::AppState::Menu(modules),
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