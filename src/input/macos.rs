use super::{KeyState, mac_keycodes};
use crate::mixer::Shared;
use std::{
    ffi::c_void,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};

unsafe extern "C" {
    fn reeb_input_create(
        callback: extern "C" fn(*mut c_void, u16, u32, u64),
        context: *mut c_void,
    ) -> *mut c_void;
    fn reeb_input_run(input: *mut c_void);
    fn reeb_input_stop(input: *mut c_void);
    fn reeb_input_destroy(input: *mut c_void);
}

struct Events {
    shared: Arc<Shared>,
    keys: KeyState,
}

impl Events {
    fn receive(&mut self, code: u16, kind: u32, flags: u64) {
        if kind >= 0xffff_fffe {
            self.keys.remove_device(0);
            self.shared.reset_visualization();
            return;
        }
        let (scan, pressed) = if kind == 14 {
            // The native bridge sends auxiliary data1 in the flags argument.
            let Some(event) = mac_keycodes::media_key(flags as u32) else {
                return;
            };
            event
        } else {
            let Some(scan) = mac_keycodes::scan(code) else {
                return;
            };
            let pressed = match kind {
                10 => true,
                11 => false,
                12 => match mac_keycodes::modifier_pressed(code, flags) {
                    Some(pressed) => pressed,
                    None => return,
                },
                _ => return,
            };
            (scan, pressed)
        };
        if let Some(event) = self.keys.transition(0, scan, pressed) {
            self.shared.push(event);
        }
    }
}

extern "C" fn key_event(context: *mut c_void, code: u16, kind: u32, flags: u64) {
    // The worker owns Events for the entire run-loop lifetime; callbacks are serial.
    unsafe { &mut *context.cast::<Events>() }.receive(code, kind, flags);
}

pub struct GlobalInput {
    native: usize,
    thread: Option<thread::JoinHandle<()>>,
    active: Arc<AtomicBool>,
}

impl GlobalInput {
    pub fn start(shared: Arc<Shared>) -> Result<Self, String> {
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let active = Arc::new(AtomicBool::new(true));
        let worker_active = active.clone();
        let thread = thread::Builder::new()
            .name("mac-keyboard".into())
            .spawn(move || {
                let mut events = Box::new(Events {
                    shared,
                    keys: KeyState::default(),
                });
                let native =
                    unsafe { reeb_input_create(key_event, (&mut *events as *mut Events).cast()) };
                if native.is_null() {
                    worker_active.store(false, Ordering::Release);
                    let error = if crate::macos::input_allowed() {
                        "Input Monitoring is allowed, but the keyboard listener could not start. Quit and reopen reeb."
                    } else {
                        "macOS has not granted Input Monitoring to this copy of reeb."
                    };
                    let _ = ready_tx.send(Err(error.to_owned()));
                    return;
                }
                if ready_tx.send(Ok(native as usize)).is_err() {
                    unsafe { reeb_input_destroy(native) };
                    return;
                }
                unsafe { reeb_input_run(native) };
                events.shared.reset_visualization();
                worker_active.store(false, Ordering::Release);
            })
            .map_err(|error| error.to_string())?;
        match ready_rx.recv().map_err(|error| error.to_string())? {
            Ok(native) => Ok(Self {
                native,
                thread: Some(thread),
                active,
            }),
            Err(error) => {
                let _ = thread.join();
                Err(error)
            }
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }
}

impl Drop for GlobalInput {
    fn drop(&mut self) {
        unsafe { reeb_input_stop(self.native as *mut c_void) };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        unsafe { reeb_input_destroy(self.native as *mut c_void) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixer::Settings;

    #[test]
    fn media_keys_reach_the_mixer_once_per_press_and_release() {
        let shared = Shared::new(&Settings::default());
        shared.set_visualize(true);
        let mut events = Events {
            shared: shared.clone(),
            keys: KeyState::default(),
        };
        // Volume up: repeated downs and stray ups must not duplicate sounds.
        for data in [0x0a00, 0x0a01, 0x0b00, 0x0b00] {
            events.receive(0, 14, data);
        }
        // A normal F1 press still takes the existing keyboard path.
        events.receive(122, 10, 0);
        events.receive(122, 11, 0);
        let mut actual = Vec::new();
        shared.take_visual_events(&mut actual);
        assert_eq!(
            actual
                .iter()
                .map(|e| (e.scan, e.pressed))
                .collect::<Vec<_>>(),
            [(0x58, true), (0x58, false), (0x3b, true), (0x3b, false)]
        );
        assert_eq!(shared.events.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn mac_events_suppress_repeats_handle_modifiers_and_reset_held_state() {
        let shared = Shared::new(&Settings::default());
        shared.set_visualize(true);
        let mut events = Events {
            shared: shared.clone(),
            keys: KeyState::default(),
        };
        events.receive(0, 10, 0);
        events.receive(0, 10, 0);
        events.receive(0, 11, 0);
        events.receive(56, 12, 0x02);
        events.receive(56, 12, 0);
        let expected = [(0x1e, true), (0x1e, false), (0x2a, true), (0x2a, false)];
        let mut actual = Vec::new();
        shared.take_visual_events(&mut actual);
        assert_eq!(actual.len(), expected.len());
        for ((scan, pressed), event) in expected.into_iter().zip(actual) {
            assert_eq!((event.scan, event.pressed), (scan, pressed));
        }
        assert_eq!(shared.events.load(Ordering::Relaxed), 4);
        events.receive(0, 10, 0);
        events.receive(0, 0xffff_fffe, 0);
        events.receive(0, 10, 0);
        assert_eq!(shared.events.load(Ordering::Relaxed), 6);
    }
}
