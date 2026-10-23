#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::GlobalInput;

use crate::keys::KeyEvent;
use std::collections::HashSet;

#[derive(Default)]
pub struct KeyState {
    held: HashSet<(usize, u16)>,
}

impl KeyState {
    pub fn transition(&mut self, device: usize, scan: u16, pressed: bool) -> Option<KeyEvent> {
        let changed = if pressed {
            self.held.insert((device, scan))
        } else {
            self.held.remove(&(device, scan))
        };
        changed.then_some(KeyEvent { scan, pressed })
    }

    pub fn remove_device(&mut self, device: usize) {
        self.held.retain(|&(id, _)| id != device);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeats_are_suppressed_per_keyboard_and_unplug_clears_state() {
        let mut state = KeyState::default();
        assert!(state.transition(1, 30, true).is_some());
        assert!(state.transition(1, 30, true).is_none());
        assert!(state.transition(2, 30, true).is_some());
        assert!(state.transition(1, 30, false).is_some());
        assert!(state.transition(1, 30, false).is_none());
        state.remove_device(2);
        assert!(state.transition(2, 30, true).is_some());
    }
}
