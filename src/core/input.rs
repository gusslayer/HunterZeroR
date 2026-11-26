//! Platform-neutral input actions.
//!
//! The GPi Case 2W exposes its buttons through different kernel interfaces
//! (evdev gamepad, GPIO keyboard, or both at once). Each platform backend
//! translates its native events into these actions, and the debouncer removes
//! the duplicate that arrives when one physical press is reported twice.

use std::time::{Duration, Instant};

/// One action understood by the menu and by every module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    /// A / Enter / Space: open, confirm or activate.
    Open,
    /// B / Escape / Backspace: cancel or go back.
    Back,
    /// START / Tab: run the current operation (treated as Open inside modules).
    Start,
    /// A real exit request (window close, F10, Ctrl+Q). B never exits.
    Quit,
}

impl Action {
    /// START behaves like A inside tool screens, exactly as in the Python app.
    pub fn as_open(self) -> Action {
        match self {
            Action::Start => Action::Open,
            other => other,
        }
    }
}

/// Suppress the same action arriving from two input families at once.
pub struct Debouncer {
    last: Option<(Action, Instant)>,
    window: Duration,
}

impl Debouncer {
    pub fn new() -> Self {
        Self {
            last: None,
            window: Duration::from_millis(80),
        }
    }

    /// Return `true` when the action should be delivered to the UI.
    pub fn accept(&mut self, action: Action) -> bool {
        let now = Instant::now();
        if let Some((last_action, last_time)) = self.last {
            if last_action == action && now.duration_since(last_time) < self.window {
                return false;
            }
        }
        self.last = Some((action, now));
        true
    }
}

impl Default for Debouncer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_is_treated_as_open_in_modules() {
        assert_eq!(Action::Start.as_open(), Action::Open);
        assert_eq!(Action::Back.as_open(), Action::Back);
    }

    #[test]
    fn duplicate_actions_are_suppressed() {
        let mut debouncer = Debouncer::new();
        assert!(debouncer.accept(Action::Down));
        assert!(!debouncer.accept(Action::Down));
        assert!(debouncer.accept(Action::Open));
    }
}
