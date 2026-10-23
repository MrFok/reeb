use super::KeyState;
use crate::mixer::Shared;
use std::{
    cell::Cell,
    mem::{size_of, zeroed},
    ptr::{null, null_mut},
    sync::{Arc, mpsc},
    thread,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::{
            GetRawInputData, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RID_INPUT, RIDEV_DEVNOTIFY,
            RIDEV_INPUTSINK, RIM_TYPEKEYBOARD, RegisterRawInputDevices,
        },
        WindowsAndMessaging::*,
    },
};

pub struct GlobalInput {
    window: usize,
    thread: Option<thread::JoinHandle<()>>,
}

struct State {
    keys: KeyState,
    shared: Arc<Shared>,
}

thread_local! {
    static HOOK_STATE: Cell<*mut State> = const { Cell::new(null_mut()) };
}

struct TabHook(HHOOK);

impl TabHook {
    unsafe fn install(state: *mut State) -> Result<Self, String> {
        HOOK_STATE.with(|slot| slot.set(state));
        let hook = unsafe {
            SetWindowsHookExW(WH_KEYBOARD_LL, Some(tab_hook), GetModuleHandleW(null()), 0)
        };
        if hook.is_null() {
            HOOK_STATE.with(|slot| slot.set(null_mut()));
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(Self(hook))
    }
}

impl Drop for TabHook {
    fn drop(&mut self) {
        unsafe {
            UnhookWindowsHookEx(self.0);
        }
        HOOK_STATE.with(|slot| slot.set(null_mut()));
    }
}

fn tab_transition(
    keys: &mut KeyState,
    message: u32,
    key: &KBDLLHOOKSTRUCT,
) -> Option<crate::keys::KeyEvent> {
    if key.vkCode != 9 || key.scanCode != 0x0f {
        return None;
    }
    let pressed = match message {
        WM_KEYDOWN | WM_SYSKEYDOWN => true,
        WM_KEYUP | WM_SYSKEYUP => false,
        _ => return None,
    };
    keys.transition(usize::MAX, 0x0f, pressed)
}

unsafe extern "system" fn tab_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let key = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
        HOOK_STATE.with(|slot| {
            let pointer = slot.get();
            if !pointer.is_null() {
                let state = unsafe { &mut *pointer };
                if let Some(event) = tab_transition(&mut state.keys, wparam as u32, key) {
                    state.shared.push(event);
                }
            }
        });
    }
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}

impl GlobalInput {
    pub fn start(shared: Arc<Shared>) -> Result<Self, String> {
        let (sender, receiver) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("keyboard-input".to_owned())
            .spawn(move || {
                // The window, callback state and message pump are owned by this thread.
                unsafe {
                    let class: Vec<u16> = "ReebInput\0".encode_utf16().collect();
                    let instance = GetModuleHandleW(null());
                    let wc = WNDCLASSW {
                        lpfnWndProc: Some(window_proc),
                        hInstance: instance,
                        lpszClassName: class.as_ptr(),
                        ..zeroed()
                    };
                    if RegisterClassW(&wc) == 0 {
                        let _ = sender.send(Err(std::io::Error::last_os_error().to_string()));
                        return;
                    }
                    let window = CreateWindowExW(
                        0,
                        class.as_ptr(),
                        class.as_ptr(),
                        0,
                        0,
                        0,
                        0,
                        0,
                        HWND_MESSAGE,
                        null_mut(),
                        instance,
                        null(),
                    );
                    if window.is_null() {
                        let _ = sender.send(Err(std::io::Error::last_os_error().to_string()));
                        UnregisterClassW(class.as_ptr(), instance);
                        return;
                    }
                    let mut state = Box::new(State {
                        keys: KeyState::default(),
                        shared,
                    });
                    SetWindowLongPtrW(window, GWLP_USERDATA, (&mut *state as *mut State) as isize);
                    let device = RAWINPUTDEVICE {
                        usUsagePage: 1,
                        usUsage: 6,
                        dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
                        hwndTarget: window,
                    };
                    if RegisterRawInputDevices(&device, 1, size_of::<RAWINPUTDEVICE>() as u32) == 0
                    {
                        let _ = sender.send(Err(std::io::Error::last_os_error().to_string()));
                        DestroyWindow(window);
                        UnregisterClassW(class.as_ptr(), instance);
                        return;
                    }
                    let hook = match TabHook::install(&mut *state) {
                        Ok(hook) => hook,
                        Err(error) => {
                            let _ = sender.send(Err(format!("Tab capture unavailable: {error}")));
                            DestroyWindow(window);
                            UnregisterClassW(class.as_ptr(), instance);
                            return;
                        }
                    };
                    if sender.send(Ok(window as usize)).is_err() {
                        DestroyWindow(window);
                        UnregisterClassW(class.as_ptr(), instance);
                        return;
                    }
                    let mut message = zeroed();
                    while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                    drop(hook);
                    SetWindowLongPtrW(window, GWLP_USERDATA, 0);
                    if IsWindow(window) != 0 {
                        DestroyWindow(window);
                    }
                    UnregisterClassW(class.as_ptr(), instance);
                }
            })
            .map_err(|error| error.to_string())?;
        match receiver.recv().map_err(|error| error.to_string())? {
            Ok(window) => Ok(Self {
                window,
                thread: Some(thread),
            }),
            Err(error) => {
                let _ = thread.join();
                Err(error)
            }
        }
    }
}

impl Drop for GlobalInput {
    fn drop(&mut self) {
        // The callback runs only on the worker; state outlives window destruction.
        unsafe {
            PostMessageW(self.window as HWND, WM_CLOSE, 0, 0);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // GWLP_USERDATA is installed before raw input registration and remains valid
    // until this worker finishes dispatching messages.
    let state = unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) as *mut State };
    if !state.is_null() {
        let state = unsafe { &mut *state };
        if message == WM_INPUT {
            let mut raw: RAWINPUT = unsafe { zeroed() };
            let mut bytes = size_of::<RAWINPUT>() as u32;
            let copied = unsafe {
                GetRawInputData(
                    lparam as _,
                    RID_INPUT,
                    (&mut raw as *mut RAWINPUT).cast(),
                    &mut bytes,
                    size_of::<RAWINPUTHEADER>() as u32,
                )
            };
            if copied != u32::MAX
                && copied >= size_of::<RAWINPUTHEADER>() as u32
                && raw.header.dwType == RIM_TYPEKEYBOARD
            {
                let key = unsafe { raw.data.keyboard };
                // 0xff is an overrun marker. E1 sequences include synthetic bytes.
                if key.VKey != 0xff
                    && key.MakeCode != 0
                    && key.Flags & 4 == 0
                    && key.MakeCode != 0x0f
                {
                    let scan = key.MakeCode | if key.Flags & 2 != 0 { 0x100 } else { 0 };
                    if let Some(event) =
                        state
                            .keys
                            .transition(raw.header.hDevice as usize, scan, key.Flags & 1 == 0)
                    {
                        state.shared.push(event);
                    }
                }
            }
        } else if message == WM_INPUT_DEVICE_CHANGE && wparam == 2 {
            state.keys.remove_device(lparam as usize);
            state.shared.reset_visualization();
        }
    }
    if message == WM_CLOSE {
        unsafe {
            DestroyWindow(window);
        }
        return 0;
    }
    if message == WM_DESTROY {
        unsafe {
            PostQuitMessage(0);
        }
        return 0;
    }
    // Required for foreground WM_INPUT cleanup; legacy input remains enabled.
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_taps_survive_system_key_messages_and_autorepeat_is_suppressed() {
        let mut keys = KeyState::default();
        let key = KBDLLHOOKSTRUCT {
            vkCode: 9,
            scanCode: 0x0f,
            flags: LLKHF_ALTDOWN,
            ..unsafe { zeroed() }
        };
        let mut events = Vec::new();
        for (down, up) in [
            (WM_SYSKEYDOWN, WM_SYSKEYUP),
            (WM_SYSKEYDOWN, WM_KEYUP),
            (WM_KEYDOWN, WM_KEYUP),
        ] {
            events.extend(tab_transition(&mut keys, down, &key));
            assert!(tab_transition(&mut keys, down, &key).is_none());
            events.extend(tab_transition(&mut keys, up, &key));
        }
        assert_eq!(events.len(), 6);
        assert!(events.iter().all(|event| event.scan == 0x0f));
        assert_eq!(
            events.iter().map(|event| event.pressed).collect::<Vec<_>>(),
            [true, false, true, false, true, false]
        );
        let other = KBDLLHOOKSTRUCT {
            vkCode: 65,
            scanCode: 0x1e,
            ..unsafe { zeroed() }
        };
        assert!(tab_transition(&mut keys, WM_SYSKEYDOWN, &other).is_none());
    }
}
