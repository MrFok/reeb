use crate::mixer::Shared;
use eframe::egui::IconData;
use std::{
    cell::Cell,
    mem::{size_of, zeroed},
    ptr::{null, null_mut},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};
use windows_sys::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{Shell::*, WindowsAndMessaging::*},
    },
    core::w,
};

const CALLBACK: u32 = WM_APP + 1;
const UPDATE: u32 = WM_APP + 2;
const POPUP: u32 = WM_APP + 3;
const OPEN: usize = 1;
const SETTINGS: usize = 2;
const TOGGLE: usize = 3;
const QUIT: usize = 4;
const VOLUME: usize = 5;
const TONE: usize = 6;
const ENTER: usize = 7;
const SWITCHES: usize = 10;
const NIN_KEYSELECT: u32 = NIN_SELECT | 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayAction {
    Open,
    Settings,
    Switches,
    Volume,
    Tone,
    Enter,
    Toggle,
    Quit,
    Available,
    Unavailable,
    Menu { x: i32, y: i32 },
}

pub struct Tray {
    window: usize,
    thread: Option<thread::JoinHandle<()>>,
    actions: mpsc::Receiver<TrayAction>,
    available: Arc<AtomicBool>,
}

impl Tray {
    pub fn open_existing() -> bool {
        unsafe {
            let window = FindWindowW(w!("ReebTray"), null());
            if window.is_null() {
                return false;
            }
            let mut process = 0;
            GetWindowThreadProcessId(window, &mut process);
            AllowSetForegroundWindow(process);
            PostMessageW(window, WM_COMMAND, OPEN, 0) != 0
        }
    }

    pub fn start(
        shared: Arc<Shared>,
        icon: IconData,
        parent_window: usize,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Self, String> {
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (actions_tx, actions) = mpsc::channel();
        let available = Arc::new(AtomicBool::new(false));
        let worker_available = available.clone();
        let thread = thread::Builder::new()
            .name("tray".to_owned())
            .spawn(move || {
                let window = match TrayWindow::new() {
                    Ok(window) => window,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                        return;
                    }
                };
                let state = match State::new(
                    window.0,
                    shared,
                    icon,
                    parent_window as HWND,
                    actions_tx,
                    worker_available,
                    Box::new(wake),
                ) {
                    Ok(state) => Box::new(state),
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                        return;
                    }
                };
                if let Err(error) = state.add_icon() {
                    let _ = ready_tx.send(Err(error));
                    return;
                }
                unsafe {
                    SetWindowLongPtrW(window.0, GWLP_USERDATA, (&*state as *const State) as isize);
                    if ready_tx.send(Ok(window.0 as usize)).is_ok() {
                        let mut message = zeroed();
                        while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
                            TranslateMessage(&message);
                            DispatchMessageW(&message);
                        }
                    }
                    SetWindowLongPtrW(window.0, GWLP_USERDATA, 0);
                }
            })
            .map_err(|error| error.to_string())?;
        match ready_rx.recv().map_err(|error| error.to_string())? {
            Ok(window) => Ok(Self {
                window,
                thread: Some(thread),
                actions,
                available,
            }),
            Err(error) => {
                let _ = thread.join();
                Err(error)
            }
        }
    }

    pub fn is_available(&self) -> bool {
        self.available.load(Ordering::Relaxed)
    }

    pub fn next_action(&self) -> Option<TrayAction> {
        self.actions.try_recv().ok()
    }

    pub fn set_popup_open(&self, open: bool) {
        unsafe {
            PostMessageW(self.window as HWND, POPUP, usize::from(open), 0);
        }
    }

    pub fn update(&self) {
        unsafe {
            PostMessageW(self.window as HWND, UPDATE, 0, 0);
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        unsafe {
            PostMessageW(self.window as HWND, WM_CLOSE, 0, 0);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct TrayWindow(HWND);

impl TrayWindow {
    fn new() -> Result<Self, String> {
        unsafe {
            let instance = GetModuleHandleW(null());
            let class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance,
                lpszClassName: w!("ReebTray"),
                ..zeroed()
            };
            if RegisterClassW(&class) == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            // A hidden top-level window receives Explorer's TaskbarCreated broadcast.
            let window = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("ReebTray"),
                w!("ReebTray"),
                WS_POPUP,
                0,
                0,
                0,
                0,
                null_mut(),
                null_mut(),
                instance,
                null(),
            );
            if window.is_null() {
                let error = std::io::Error::last_os_error().to_string();
                UnregisterClassW(w!("ReebTray"), instance);
                return Err(error);
            }
            let window = Self(window);
            if SetTimer(window.0, 1, 1000, None) == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            Ok(window)
        }
    }
}

impl Drop for TrayWindow {
    fn drop(&mut self) {
        unsafe {
            SetWindowLongPtrW(self.0, GWLP_USERDATA, 0);
            if IsWindow(self.0) != 0 {
                DestroyWindow(self.0);
            }
            UnregisterClassW(w!("ReebTray"), GetModuleHandleW(null()));
        }
    }
}

fn create_icon(icon: &IconData) -> Result<HICON, String> {
    if icon.width != 32 || icon.height != 32 || icon.rgba.len() != 4096 {
        return Err("Tray icon must be 32 x 32 RGBA".to_owned());
    }
    let mut bgra = Vec::with_capacity(icon.rgba.len());
    for row in icon.rgba.chunks_exact(128) {
        for pixel in row.chunks_exact(4) {
            let alpha = u16::from(pixel[3]);
            let premultiply = |channel: u8| (u16::from(channel) * alpha / 255) as u8;
            bgra.extend_from_slice(&[
                premultiply(pixel[2]),
                premultiply(pixel[1]),
                premultiply(pixel[0]),
                pixel[3],
            ]);
        }
    }
    unsafe {
        let handle = CreateIcon(
            GetModuleHandleW(null()),
            32,
            32,
            1,
            32,
            [0_u8; 128].as_ptr(),
            bgra.as_ptr(),
        );
        if handle.is_null() {
            Err(std::io::Error::last_os_error().to_string())
        } else {
            Ok(handle)
        }
    }
}

struct State {
    data: Cell<NOTIFYICONDATAW>,
    parent_window: HWND,
    taskbar_created: u32,
    shared: Arc<Shared>,
    actions: mpsc::Sender<TrayAction>,
    available: Arc<AtomicBool>,
    wake: Box<dyn Fn() + Send>,
}

impl State {
    fn new(
        window: HWND,
        shared: Arc<Shared>,
        icon: IconData,
        parent_window: HWND,
        actions: mpsc::Sender<TrayAction>,
        available: Arc<AtomicBool>,
        wake: Box<dyn Fn() + Send>,
    ) -> Result<Self, String> {
        let handle = create_icon(&icon)?;
        unsafe {
            let taskbar_created = RegisterWindowMessageW(w!("TaskbarCreated"));
            if taskbar_created == 0 {
                let error = std::io::Error::last_os_error().to_string();
                DestroyIcon(handle);
                return Err(error);
            }
            Ok(Self {
                data: Cell::new(NOTIFYICONDATAW {
                    cbSize: size_of::<NOTIFYICONDATAW>() as u32,
                    hWnd: window,
                    uID: 1,
                    uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_SHOWTIP,
                    uCallbackMessage: CALLBACK,
                    hIcon: handle,
                    ..zeroed()
                }),
                taskbar_created,
                parent_window,
                shared,
                actions,
                available,
                wake,
            })
        }
    }

    fn tooltip(&self) {
        let text = if self.shared.enabled.load(Ordering::Relaxed) {
            "Reeb — enabled"
        } else {
            "Reeb — disabled"
        };
        let mut data = self.data.get();
        data.szTip.fill(0);
        for (slot, value) in data.szTip.iter_mut().zip(text.encode_utf16()) {
            *slot = value;
        }
        self.data.set(data);
    }

    fn add_icon(&self) -> Result<(), String> {
        self.tooltip();
        let mut data = self.data.get();
        unsafe {
            if Shell_NotifyIconW(NIM_ADD, &data) == 0 {
                return Err("Windows could not add the Reeb tray icon".to_owned());
            }
            data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            if Shell_NotifyIconW(NIM_SETVERSION, &data) == 0 {
                Shell_NotifyIconW(NIM_DELETE, &data);
                return Err("Windows could not configure the Reeb tray icon".to_owned());
            }
        }
        self.data.set(data);
        self.available.store(true, Ordering::Relaxed);
        Ok(())
    }

    fn notify(&self, action: TrayAction) {
        let _ = self.actions.send(action);
        self.wake();
    }

    fn wake(&self) {
        (self.wake)();
        // winit 0.30's RDW_INTERNALPAINT request does not repaint hidden Windows windows.
        unsafe {
            PostMessageW(self.parent_window, WM_PAINT, 0, 0);
        }
    }

    fn unavailable(&self) {
        self.available.store(false, Ordering::Relaxed);
        self.notify(TrayAction::Unavailable);
    }

    fn command(&self, id: usize) {
        let action = match id {
            OPEN => TrayAction::Open,
            SETTINGS => TrayAction::Settings,
            SWITCHES => TrayAction::Switches,
            VOLUME => TrayAction::Volume,
            TONE => TrayAction::Tone,
            ENTER => TrayAction::Enter,
            TOGGLE => TrayAction::Toggle,
            QUIT => TrayAction::Quit,
            _ => return,
        };
        self.notify(action);
    }

    fn menu(&self) {
        let mut point = windows_sys::Win32::Foundation::POINT::default();
        unsafe {
            GetCursorPos(&mut point);
        }
        self.notify(TrayAction::Menu {
            x: point.x,
            y: point.y,
        });
    }
}

impl Drop for State {
    fn drop(&mut self) {
        self.unavailable();
        let data = self.data.get();
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &data);
            DestroyIcon(data.hIcon);
        }
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state = unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) as *const State };
    if let Some(state) = unsafe { state.as_ref() } {
        if message == POPUP {
            if unsafe { SetTimer(window, 1, if wparam == 0 { 1000 } else { 16 }, None) } == 0 {
                state.unavailable();
            }
            return 0;
        }
        if message == WM_TIMER && unsafe { IsWindowVisible(state.parent_window) } == 0 {
            state.wake();
            return 0;
        }
        if message == CALLBACK {
            match lparam as u32 & 0xffff {
                NIN_SELECT | NIN_KEYSELECT => state.notify(TrayAction::Open),
                WM_CONTEXTMENU => state.menu(),
                _ => {}
            }
            return 0;
        }
        if message == WM_COMMAND {
            state.command(wparam & 0xffff);
            return 0;
        }
        if message == UPDATE {
            state.tooltip();
            if unsafe { Shell_NotifyIconW(NIM_MODIFY, &state.data.get()) } == 0 {
                state.unavailable();
            }
            return 0;
        }
        if message == state.taskbar_created {
            state.available.store(false, Ordering::Relaxed);
            if state.add_icon().is_err() {
                state.unavailable();
            } else {
                state.notify(TrayAction::Available);
            }
            return 0;
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
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::Graphics::Gdi::*;

    #[test]
    #[ignore = "requires an interactive Windows shell"]
    fn popup_wakes_the_hidden_parent_promptly_and_restores_idle_interval() {
        use std::{sync::atomic::AtomicUsize, time::Duration};
        let wakes = Arc::new(AtomicUsize::new(0));
        let counter = wakes.clone();
        let tray = Tray::start(
            Shared::new(&crate::mixer::Settings::default()),
            IconData {
                width: 32,
                height: 32,
                rgba: vec![0; 4096],
            },
            0,
            move || {
                counter.fetch_add(1, Ordering::Relaxed);
            },
        )
        .unwrap();
        tray.set_popup_open(true);
        thread::sleep(Duration::from_millis(200));
        assert!(wakes.load(Ordering::Relaxed) >= 3);
        tray.set_popup_open(false);
        thread::sleep(Duration::from_millis(50));
        let before = wakes.load(Ordering::Relaxed);
        thread::sleep(Duration::from_millis(150));
        assert_eq!(wakes.load(Ordering::Relaxed), before);
    }

    #[test]
    fn windows_icon_preserves_top_down_rows_and_premultiplied_alpha() {
        let mut rgba = vec![0; 4096];
        for pixel in rgba[..128].chunks_exact_mut(4) {
            pixel.copy_from_slice(&[240, 80, 20, 128]);
        }
        for pixel in rgba[3968..].chunks_exact_mut(4) {
            pixel.copy_from_slice(&[10, 30, 250, 255]);
        }
        let icon = create_icon(&IconData {
            width: 32,
            height: 32,
            rgba,
        })
        .unwrap();
        unsafe {
            let mut info: ICONINFO = zeroed();
            assert_ne!(GetIconInfo(icon, &mut info), 0);
            let dc = CreateCompatibleDC(null_mut());
            let mut bitmap = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: 32,
                    biHeight: -32,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    ..zeroed()
                },
                ..zeroed()
            };
            let mut pixels = vec![0_u8; 4096];
            let rows = GetDIBits(
                dc,
                info.hbmColor,
                0,
                32,
                pixels.as_mut_ptr().cast(),
                &mut bitmap,
                DIB_RGB_COLORS,
            );
            DeleteDC(dc);
            DeleteObject(info.hbmColor);
            DeleteObject(info.hbmMask);
            DestroyIcon(icon);
            assert_eq!(rows, 32);
            assert_eq!(&pixels[..4], &[10, 40, 120, 128]);
            assert_eq!(&pixels[3968..3972], &[250, 30, 10, 255]);
        }
    }
}
