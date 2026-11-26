//! HunterZeroR - software-rendered kiosk UI for the GPi Case 2W.
//!
//! Rendering targets a 320x240 virtual canvas; a platform backend scales it to
//! the physical display (2x nearest neighbour on the 640x480 framebuffer).
//! No SDL, no OpenGL/GLES, no Python. See `DESIGN.md` and `AGENTS.md`.

mod core;
mod gfx;
mod modules;
mod platform;
mod theme;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::core::input::Action;
use crate::core::plugins::MODULES;
use crate::core::process_manager::ProcessManager;
use crate::core::system_stats::{StatsReader, SystemStats};
use crate::core::ui::{draw_row, ScrollState};
use crate::gfx::Canvas;
use crate::modules::{Outcome, ToolScreen};
use crate::theme::{
    ACCENT_COLOR, BG_COLOR, CONTENT_BOTTOM, CONTENT_TOP, FOOTER_TOP, FPS, GRID_COLOR, MUTED_COLOR,
    SCREEN_HEIGHT, SCREEN_WIDTH, TEXT_COLOR,
};

/// Fixed footer hints shown in the main menu (see `DESIGN.md`).
const FOOTER_HINTS: [(&str, i32); 4] = [
    ("D-PAD Mover", 4),
    ("A Abrir", 116),
    ("START Ejecutar", 188),
    ("B Volver", 272),
];

const MENU_ROW_HEIGHT: i32 = 19;
const MENU_TOP: i32 = CONTENT_TOP + 18;

enum Screen {
    Menu,
    Module(Box<dyn ToolScreen>),
}

fn main() {
    let root = app_root();
    crate::core::logging::init(&root);
    crate::core::logging::event(
        "INFO",
        "app",
        &format!(
            "SESSION START hunterzero-rust v{}",
            env!("CARGO_PKG_VERSION")
        ),
    );

    if let Err(error) = run(&root) {
        crate::core::logging::event("ERROR", "app", &format!("fatal: {error}"));
        std::process::exit(1);
    }

    crate::core::logging::event("INFO", "app", "SESSION END");
}

fn run(root: &Path) -> Result<(), String> {
    let mut backend = platform::create()?;
    crate::core::logging::event(
        "INFO",
        "platform",
        &format!("backend {}", backend.description()),
    );

    let mut canvas = Canvas::new(SCREEN_WIDTH, SCREEN_HEIGHT);
    let mut processes = ProcessManager::new(root);
    let mut stats_reader = StatsReader::new();
    let mut stats: SystemStats = stats_reader.read();

    let mut screen = Screen::Menu;
    let mut menu = ScrollState::new(menu_capacity());
    menu.update(MODULES.len(), None);

    let frame = Duration::from_nanos(1_000_000_000 / u64::from(FPS.max(1)));
    let mut next_frame = Instant::now();
    let mut next_stats = Instant::now();
    let mut running = true;

    while running {
        processes.update();

        if next_stats.elapsed() >= Duration::from_secs(1) {
            stats = stats_reader.read();
            next_stats = Instant::now();
        }

        for action in backend.poll() {
            if action == Action::Quit {
                running = false;
                break;
            }

            if matches!(screen, Screen::Menu) {
                handle_menu(action.as_open(), &mut menu, &mut screen, root);
            } else {
                let mut close_module = false;
                if let Screen::Module(module) = &mut screen {
                    match module.handle(action.as_open(), &mut processes) {
                        Outcome::Stay => {}
                        Outcome::Close => close_module = true,
                    }
                }
                if close_module {
                    if let Screen::Module(module) = &mut screen {
                        module.close(&mut processes);
                    }
                    screen = Screen::Menu;
                }
            }
        }

        if let Screen::Module(module) = &mut screen {
            module.sync(&mut processes);
        }

        render(&mut canvas, &mut screen, &menu, &processes, &stats);

        if let Err(error) = backend.present(&canvas) {
            crate::core::logging::event("WARN", "platform", &format!("present: {error}"));
        }

        next_frame += frame;
        let now = Instant::now();
        if next_frame > now {
            std::thread::sleep(next_frame - now);
        } else {
            next_frame = now;
        }
    }

    if let Screen::Module(module) = &mut screen {
        module.close(&mut processes);
    }
    Ok(())
}

fn handle_menu(action: Action, menu: &mut ScrollState, screen: &mut Screen, root: &Path) {
    match action {
        Action::Up => menu.move_by(-1),
        Action::Down => menu.move_by(1),
        Action::Open | Action::Start => {
            if let Some(info) = MODULES.get(menu.selected) {
                if let Some(module) = modules::create(info.id, root) {
                    *screen = Screen::Module(module);
                }
            }
        }
        _ => {}
    }
}

fn render(
    canvas: &mut Canvas,
    screen: &mut Screen,
    menu: &ScrollState,
    processes: &ProcessManager,
    stats: &SystemStats,
) {
    canvas.clear(BG_COLOR);

    match screen {
        Screen::Menu => draw_menu(canvas, menu),
        Screen::Module(module) => module.draw(canvas, processes, stats),
    }

    draw_footer(canvas, &*screen);
}

fn draw_menu(canvas: &mut Canvas, menu: &ScrollState) {
    canvas.draw_text("HUNTERZERO", 10, CONTENT_TOP + 2, ACCENT_COLOR, 1);
    let capacity = menu_capacity();
    let first = menu.offset;
    let last = (first + capacity).min(MODULES.len());
    for (row, index) in (first..last).enumerate() {
        let info = &MODULES[index];
        let y = MENU_TOP + row as i32 * MENU_ROW_HEIGHT;
        draw_row(
            canvas,
            10,
            y,
            SCREEN_WIDTH as i32 - 20,
            MENU_ROW_HEIGHT - 2,
            info.name,
            info.description,
            index == menu.selected,
            Some(info.icon),
        );
    }
    canvas.draw_text(
        &format!("{}/{}", menu.selected + 1, MODULES.len().max(1)),
        SCREEN_WIDTH as i32 - 48,
        CONTENT_TOP + 2,
        MUTED_COLOR,
        1,
    );
}

fn draw_footer(canvas: &mut Canvas, screen: &Screen) {
    canvas.fill_rect(
        0,
        FOOTER_TOP,
        SCREEN_WIDTH as i32,
        SCREEN_HEIGHT as i32 - FOOTER_TOP,
        BG_COLOR,
    );
    canvas.hline(0, SCREEN_WIDTH as i32 - 1, FOOTER_TOP, GRID_COLOR);
    let hints: Vec<(&str, i32)> = match screen {
        Screen::Menu => FOOTER_HINTS.iter().map(|(text, x)| (*text, *x)).collect(),
        Screen::Module(module) => module.footer_hints(),
    };
    for (text, x) in hints {
        canvas.draw_text_clipped(
            text,
            x,
            FOOTER_TOP + 11,
            SCREEN_WIDTH as i32 - x - 2,
            TEXT_COLOR,
            1,
        );
    }
}

fn menu_capacity() -> usize {
    (((CONTENT_BOTTOM - MENU_TOP) / MENU_ROW_HEIGHT).max(1)) as usize
}

fn app_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
