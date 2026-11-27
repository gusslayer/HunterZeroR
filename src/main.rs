//! HunterZeroR - software-rendered kiosk UI for the GPi Case 2W.
//!
//! Rendering targets a 320x240 virtual canvas; a platform backend scales it to
//! the physical display (2x nearest neighbour on the 640x480 framebuffer).

mod core;
mod gfx;
mod modules;
mod platform;
mod theme;

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::core::input::Action;
use crate::core::plugins::MODULES;
use crate::core::process_manager::ProcessManager;
use crate::core::system_stats::{StatsReader, SystemStats};
use crate::core::ui::{draw_global_footer, draw_global_header, draw_row, ScrollState};
use crate::gfx::Canvas;
use crate::modules::{Outcome, ToolScreen};
use crate::theme::{
    ACCENT_COLOR, BG_COLOR, CONTENT_BOTTOM, CONTENT_HEIGHT, CONTENT_TOP, ERROR_COLOR, FPS,
    MUTED_COLOR, SCREEN_HEIGHT, SCREEN_WIDTH,
};

const MENU_ROW_HEIGHT: i32 = 19;
const MENU_TOP: i32 = CONTENT_TOP + 18;
const ALERT_DURATION: Duration = Duration::from_secs(3);

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
    let mut alert: Option<(String, Instant)> = None;

    let frame = Duration::from_nanos(1_000_000_000 / u64::from(FPS.max(1)));
    let mut next_frame = Instant::now();
    let mut next_stats = Instant::now();
    let mut running = true;
    let mut dirty = true;

    while running {
        processes.update();

        if next_stats.elapsed() >= Duration::from_secs(1) {
            stats = stats_reader.read();
            next_stats = Instant::now();
            dirty = true;
        }

        for action in backend.poll() {
            dirty = true;
            if action == Action::Quit {
                running = false;
                break;
            }

            if matches!(screen, Screen::Menu) {
                handle_menu(action.as_open(), &mut menu, &mut screen, root);
            } else {
                let mut close_module = false;
                let mut panic_error = None;
                if let Screen::Module(module) = &mut screen {
                    let result =
                        catch_unwind(AssertUnwindSafe(|| module.handle(action, &mut processes)));
                    match result {
                        Ok(Outcome::Stay) => {}
                        Ok(Outcome::Close) => close_module = true,
                        Err(_) => panic_error = Some("handle"),
                    }
                }
                if let Some(stage) = panic_error {
                    recover_module(&mut screen, &mut processes, &mut alert, stage);
                } else if close_module {
                    if let Screen::Module(module) = &mut screen {
                        let _ = catch_unwind(AssertUnwindSafe(|| {
                            module.close(&mut processes);
                        }));
                    }
                    screen = Screen::Menu;
                }
            }
        }

        if let Screen::Module(module) = &mut screen {
            let result = catch_unwind(AssertUnwindSafe(|| {
                module.sync(&mut processes);
            }));
            dirty = true;
            if result.is_err() {
                recover_module(&mut screen, &mut processes, &mut alert, "sync");
            }
        } else if alert.is_some() {
            dirty = true;
        }

        if dirty {
            render(
                &mut canvas,
                &mut screen,
                &menu,
                &mut processes,
                &stats,
                &mut alert,
            );

            if let Err(error) = backend.present(&canvas) {
                crate::core::logging::event("WARN", "platform", &format!("present: {error}"));
            }
            dirty = false;
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
        let _ = catch_unwind(AssertUnwindSafe(|| {
            module.close(&mut processes);
        }));
    }
    Ok(())
}

fn recover_module(
    screen: &mut Screen,
    processes: &mut ProcessManager,
    alert: &mut Option<(String, Instant)>,
    stage: &str,
) {
    processes.stop();
    crate::core::logging::event("ERROR", "app", &format!("module panic during {stage}"));
    if let Screen::Module(module) = screen {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            module.close(processes);
        }));
    }
    *screen = Screen::Menu;
    *alert = Some((
        "ERROR DE MODULO. VOLVIENDO AL MENU.".to_string(),
        Instant::now(),
    ));
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
    processes: &mut ProcessManager,
    stats: &SystemStats,
    alert: &mut Option<(String, Instant)>,
) {
    canvas.clear(BG_COLOR);

    let mut draw_panic = false;
    match screen {
        Screen::Menu => draw_menu(canvas, menu),
        Screen::Module(module) => {
            let result = catch_unwind(AssertUnwindSafe(|| {
                module.draw(canvas, processes, stats);
            }));
            if result.is_err() {
                draw_panic = true;
            }
        }
    }

    if draw_panic {
        recover_module(screen, processes, alert, "draw");
        canvas.fill_rect(
            0,
            CONTENT_TOP,
            SCREEN_WIDTH as i32,
            CONTENT_HEIGHT,
            BG_COLOR,
        );
        draw_menu(canvas, menu);
    }

    draw_alert(canvas, alert);
    // Global HUD always drawn last to prevent corruption/overwrite.
    draw_global_header(canvas, stats);
    draw_global_footer(canvas);
}

fn draw_alert(canvas: &mut Canvas, alert: &mut Option<(String, Instant)>) {
    let Some((message, at)) = alert.as_ref() else {
        return;
    };
    if at.elapsed() > ALERT_DURATION {
        *alert = None;
        return;
    }
    canvas.draw_text_clipped(message, 16, CONTENT_BOTTOM - 14, 288, ERROR_COLOR, 1);
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

fn menu_capacity() -> usize {
    (((CONTENT_BOTTOM - MENU_TOP) / MENU_ROW_HEIGHT).max(1)) as usize
}

fn app_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
