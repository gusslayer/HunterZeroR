//! Input handling for HunterZeroR.
//! 
//! Mirrors core/input.py from the Python version.
//! Traduce eventos de SDL/gamepad a acciones: UP, DOWN, LEFT, RIGHT, OPEN, BACK, START, QUIT.

use ggez::event;
use ggez::input::keyboard;
use ggez::input::mouse;

use crate::state::UIState;

/// Acciones normalizadas que usan los módulos y el menú.
/// 
/// Igual que en Python: UP/DOWN/LEFT/RIGHT/OPEN/BACK/START/QUIT
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Open,   // A (abrir/confirmar)
    Back,   // B (cancelar/volver)
    Start,  // START
    Quit,   // SDL_QUIT / Escape
    None,   // Sin acción
}

/// Traducir un evento de SDL a una Action normalizada.
/// 
/// Igual que `action_from_event(event)` en Python.
pub fn action_from_event(event: &event::Event) -> Option<Action> {
    use event::Event::*;
    
    match event {
        // Teclado
        KeyDown { key_code: Some(key), .. } => {
            use keyboard::KeyCode;
            match key {
                KeyCode::Up => Some(Action::Up),
                KeyCode::Down => Some(Action::Down),
                KeyCode::Left => Some(Action::Left),
                KeyCode::Right => Some(Action::Right),
                KeyCode::Return | KeyCode::Space => Some(Action::Open),
                KeyCode::Backspace | KeyCode::Escape => Some(Action::Back),
                KeyCode::S => Some(Action::Start), // Mapeo temporal
                _ => None,
            }
        }
        // Gamepad / Joystick
        GamepadButtonPressed { button, .. } => {
            // Mapear botones del gamepad a acciones
            // Botón superior derecho = START, Botón derecho = B, etc.
            None // Simplificado: se mapearía según la configuración
        }
        // Ratón
        MouseButtonDown { button, .. } => {
            match button {
                mouse::MouseButton::Left => Some(Action::Open),
                mouse::MouseButton::Right => Some(Action::Back),
                _ => None,
            }
        }
        // SDL QUIT
        Quit => Some(Action::Quit),
        _ => None,
    }
}

/// Inicializar joysticks/gamepads.
/// 
/// Igual que `initialize_joysticks(logger)` en Python.
pub fn initialize_joysticks(logger: &mut dyn std::fmt::Display) -> Vec<ggez::input::joystick::Joystick> {
    let joysticks = ggez::input::Joysticks::new();
    let count = joysticks.len();
    logger.log(&format!("Joysticks detectados: {}", count));
    
    (0..count)
        .map(|i| joysticks.at(i).expect("Joystick error"))
        .collect()
}

/// Verificar si se presionó QUIT (Escape o SDL_QUIT).
pub fn is_quit_action(action: &Option<Action>) -> bool {
    match action {
        Some(Action::Quit) => true,
        _ => false,
    }
}

/// Verificar si es acción de abrir/confirmar.
pub fn is_open_action(action: &Option<Action>) -> bool {
    match action {
        Some(Action::Open) | Some(Action::Start) => true,
        _ => false,
    }
}

/// Verificar si es acción de retroceso/cancelar.
pub fn is_back_action(action: &Option<Action>) -> bool {
    match action {
        Some(Action::Back) => true,
        _ => false,
    }
}