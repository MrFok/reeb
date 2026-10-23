//! Small AppKit bridge. Menu operations run on eframe's main thread.
use eframe::egui;
use std::{ffi::c_void, marker::PhantomData, rc::Rc, sync::mpsc};

unsafe extern "C" {
    fn reeb_input_allowed() -> bool;
    fn reeb_input_request();
    fn reeb_menu_create(
        callback: extern "C" fn(*mut c_void, u32),
        context: *mut c_void,
    ) -> *mut c_void;
    fn reeb_menu_update(menu: *mut c_void, enabled: bool);
    fn reeb_menu_available(menu: *mut c_void) -> bool;
    fn reeb_menu_destroy(menu: *mut c_void);
    fn reeb_window_show();
}

pub fn input_allowed() -> bool {
    unsafe { reeb_input_allowed() }
}
pub fn request_input_access() {
    unsafe { reeb_input_request() }
}

#[derive(Clone, Copy)]
pub enum Action {
    Open,
    Settings,
    Toggle,
    Quit,
    Close,
}

struct Callback {
    sender: mpsc::Sender<Action>,
    ctx: egui::Context,
}

extern "C" fn menu_action(context: *mut c_void, command: u32) {
    let action = match command {
        0 => Action::Open,
        1 => Action::Settings,
        2 => Action::Toggle,
        3 => Action::Quit,
        4 => Action::Close,
        _ => return,
    };
    let callback = unsafe { &*context.cast::<Callback>() };
    let _ = callback.sender.send(action);
    callback.ctx.request_repaint();
}

pub struct Menu {
    native: *mut c_void,
    _callback: Box<Callback>,
    actions: mpsc::Receiver<Action>,
    // AppKit objects must only be accessed/dropped on their creating main thread.
    _main_thread: PhantomData<Rc<()>>,
}

impl Menu {
    pub fn new(ctx: &egui::Context, enabled: bool) -> Self {
        let (sender, actions) = mpsc::channel();
        let mut callback = Box::new(Callback {
            sender,
            ctx: ctx.clone(),
        });
        let native =
            unsafe { reeb_menu_create(menu_action, (&mut *callback as *mut Callback).cast()) };
        let menu = Self {
            native,
            _callback: callback,
            actions,
            _main_thread: PhantomData,
        };
        menu.update(enabled);
        menu
    }
    pub fn next_action(&self) -> Option<Action> {
        self.actions.try_recv().ok()
    }
    pub fn update(&self, enabled: bool) {
        unsafe { reeb_menu_update(self.native, enabled) }
    }
    pub fn is_available(&self) -> bool {
        unsafe { reeb_menu_available(self.native) }
    }
    pub fn show_window(&self) {
        unsafe { reeb_window_show() }
    }
}

impl Drop for Menu {
    fn drop(&mut self) {
        unsafe { reeb_menu_destroy(self.native) }
    }
}
