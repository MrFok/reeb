use crate::{
    audition::Audition,
    config::Saved,
    icon,
    preview::Preview,
    theme,
    ui::{self, Page},
};
use eframe::egui;
#[cfg(windows)]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use reeb::{
    audio::{self, Audio},
    keys::KeyEvent,
    mixer::Shared,
    samples::SampleBank,
};
use std::{
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};

pub struct App {
    saved: Saved,
    bank: Arc<SampleBank>,
    shared: Arc<Shared>,
    audio: Option<Audio>,
    audio_error: Option<String>,
    devices: Vec<String>,
    input_status: String,
    preview: Preview,
    audition: Audition,
    image: egui::TextureHandle,
    visual_events: Vec<KeyEvent>,
    #[cfg(any(windows, target_os = "macos"))]
    input: Option<reeb::input::GlobalInput>,
    #[cfg(windows)]
    tray: Option<reeb::tray::Tray>,
    tray_error: Option<String>,
    #[cfg(target_os = "macos")]
    menu: reeb::macos::Menu,
    #[cfg(target_os = "macos")]
    input_retry: Instant,
    last_retry: Instant,
    page: Page,
    exiting: bool,
    hidden: bool,
    #[cfg(windows)]
    popup: Option<crate::popup::Popup>,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        theme::apply(&cc.egui_ctx);
        let mut saved: Saved = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();
        saved.settings.sanitize();
        theme::select(&cc.egui_ctx, saved.theme);
        let bank = Arc::new(SampleBank::load(saved.switch)?);
        let shared = Shared::new(&saved.settings);
        shared.set_enabled(saved.enabled);
        #[cfg(windows)]
        let (input, input_status) = match reeb::input::GlobalInput::start(shared.clone()) {
            Ok(input) => (Some(input), "Listening across Windows".to_owned()),
            Err(error) => (None, format!("Background input unavailable: {error}")),
        };
        #[cfg(target_os = "macos")]
        let (input, input_status) = match reeb::input::GlobalInput::start(shared.clone()) {
            Ok(input) => (Some(input), "Listening across macOS".to_owned()),
            Err(error) => (
                None,
                format!("{error} Focused typing is available meanwhile."),
            ),
        };
        #[cfg(not(any(windows, target_os = "macos")))]
        let input_status = "Mac preview: type while this window is focused".to_owned();
        #[cfg(windows)]
        let (tray, tray_error) = {
            let ctx = cc.egui_ctx.clone();
            let parent_window = match cc.window_handle()?.as_raw() {
                RawWindowHandle::Win32(window) => window.hwnd.get() as usize,
                _ => return Err("The Windows window handle is unavailable".into()),
            };
            reeb::window::set_application_icons(parent_window);
            reeb::window::round_corners(parent_window);
            match reeb::tray::Tray::start(shared.clone(), icon::tray(), parent_window, move || {
                ctx.request_repaint()
            }) {
                Ok(tray) => (Some(tray), None),
                Err(error) => (
                    None,
                    Some(format!(
                        "Tray unavailable: {error}. Closing the window will quit Reeb."
                    )),
                ),
            }
        };
        #[cfg(target_os = "macos")]
        let menu = reeb::macos::Menu::new(&cc.egui_ctx, saved.enabled);
        let mut app = Self {
            saved,
            bank,
            shared,
            audio: None,
            audio_error: None,
            devices: audio::output_devices().unwrap_or_default(),
            input_status,
            visual_events: Vec::with_capacity(reeb::mixer::QUEUE_SIZE),
            preview: Preview::default(),
            audition: Audition::default(),
            image: icon::image(&cc.egui_ctx),
            #[cfg(any(windows, target_os = "macos"))]
            input,
            #[cfg(windows)]
            tray,
            #[cfg(windows)]
            tray_error,
            #[cfg(not(windows))]
            tray_error: None,
            #[cfg(target_os = "macos")]
            menu,
            #[cfg(target_os = "macos")]
            input_retry: Instant::now(),
            last_retry: Instant::now(),
            page: Page::default(),
            exiting: false,
            hidden: false,
            #[cfg(windows)]
            popup: None,
        };
        app.restart_audio();
        Ok(app)
    }

    fn restart_audio(&mut self) {
        self.audio = None;
        self.preview.cancel();
        self.audition.cancel();
        while self.shared.queue.pop().is_some() {}
        self.last_retry = Instant::now();
        match Audio::start(
            self.bank.clone(),
            self.shared.clone(),
            self.saved.device.as_deref(),
        ) {
            Ok(audio) => {
                self.audio = Some(audio);
                self.audio_error = None;
            }
            Err(error) => self.audio_error = Some(error),
        }
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.saved.enabled = enabled;
        self.shared.set_enabled(enabled);
        if !enabled {
            self.audition.cancel();
            self.preview.cancel();
        }
        #[cfg(windows)]
        if let Some(tray) = &self.tray {
            tray.update();
        }
        #[cfg(target_os = "macos")]
        self.menu.update(enabled);
    }

    fn tray_available(&self) -> bool {
        #[cfg(windows)]
        {
            self.tray
                .as_ref()
                .is_some_and(reeb::tray::Tray::is_available)
        }
        #[cfg(target_os = "macos")]
        {
            self.menu.is_available()
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        {
            false
        }
    }

    #[cfg(any(windows, target_os = "macos"))]
    fn restore(&mut self, ctx: &egui::Context) {
        self.hidden = false;
        #[cfg(target_os = "macos")]
        self.menu.show_window();
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(840.0, 660.0)));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    #[cfg(any(windows, target_os = "macos"))]
    fn open_page(&mut self, ctx: &egui::Context, page: Page) {
        self.preview.cancel();
        self.page = page;
        self.restore(ctx);
    }

    fn quit(&mut self, ctx: &egui::Context) {
        self.exiting = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    #[cfg(windows)]
    fn tray_actions(&mut self, ctx: &egui::Context) {
        while let Some(action) = self.tray.as_ref().and_then(reeb::tray::Tray::next_action) {
            self.tray_action(ctx, action);
        }
    }

    #[cfg(windows)]
    fn tray_action(&mut self, ctx: &egui::Context, action: reeb::tray::TrayAction) {
        use reeb::tray::TrayAction;
        match action {
            TrayAction::Menu { x, y } => {
                self.popup = Some(crate::popup::Popup::new(x, y, true));
                if let Some(tray) = &self.tray {
                    tray.set_popup_open(true);
                }
            }
            TrayAction::Open => self.restore(ctx),
            TrayAction::Settings => self.open_page(ctx, Page::Settings),
            TrayAction::Switches => self.open_page(ctx, Page::Switches),
            TrayAction::Volume => self.open_page(ctx, Page::Volume),
            TrayAction::Tone => self.open_page(ctx, Page::Tone),
            TrayAction::Enter => self.open_page(ctx, Page::Enter),
            TrayAction::Toggle => self.set_enabled(!self.saved.enabled),
            TrayAction::Quit => self.quit(ctx),
            TrayAction::Available => self.tray_error = None,
            TrayAction::Unavailable => {
                self.tray_error =
                    Some("Tray unavailable. Closing the window will quit Reeb.".to_owned());
                self.restore(ctx);
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn mac_actions(&mut self, ctx: &egui::Context) {
        while let Some(action) = self.menu.next_action() {
            match action {
                reeb::macos::Action::Open => self.restore(ctx),
                reeb::macos::Action::Settings => self.open_page(ctx, Page::Settings),
                reeb::macos::Action::Toggle => self.set_enabled(!self.saved.enabled),
                reeb::macos::Action::Quit => self.quit(ctx),
                reeb::macos::Action::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            }
        }
        if self.input_retry.elapsed() >= Duration::from_secs(3) {
            self.input_retry = Instant::now();
            if self.input.as_ref().is_some_and(|input| !input.is_active())
                || !reeb::macos::input_allowed()
            {
                self.input = None;
                self.shared.reset_visualization();
            }
            if self.input.is_none() {
                match reeb::input::GlobalInput::start(self.shared.clone()) {
                    Ok(input) => {
                        self.input = Some(input);
                        self.input_status = "Listening across macOS".into();
                    }
                    Err(error) => {
                        self.input_status =
                            format!("{error} Focused typing is available meanwhile.")
                    }
                }
            }
        }
    }

    fn focused_keys(&self, ctx: &egui::Context) {
        #[cfg(any(windows, target_os = "macos"))]
        if self.input.is_some() {
            return;
        }
        ctx.input(|input| {
            for event in &input.events {
                if let egui::Event::Key {
                    key,
                    physical_key,
                    pressed,
                    repeat,
                    ..
                } = event
                    && !repeat
                    && let Some(scan) = crate::scan_for_key(physical_key.unwrap_or(*key))
                {
                    self.shared.push(KeyEvent {
                        scan,
                        pressed: *pressed,
                    });
                }
            }
        });
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use eframe::App as _;
    use reeb::{mixer::Settings, tray::TrayAction};

    fn hidden_app(ctx: &egui::Context) -> App {
        theme::apply(ctx);
        App {
            saved: Saved::default(),
            bank: Arc::new(SampleBank::bundled().unwrap()),
            shared: Shared::new(&Settings::default()),
            audio: None,
            audio_error: None,
            devices: vec![],
            input_status: String::new(),
            visual_events: Vec::with_capacity(reeb::mixer::QUEUE_SIZE),
            preview: Preview::default(),
            audition: Audition::default(),
            image: icon::image(ctx),
            input: None,
            tray: None,
            tray_error: None,
            last_retry: Instant::now(),
            page: Page::Switches,
            exiting: false,
            hidden: true,
            popup: None,
        }
    }

    #[test]
    fn tray_open_and_settings_restore_the_hidden_window() {
        for (action, page) in [
            (TrayAction::Open, Page::Switches),
            (TrayAction::Settings, Page::Settings),
            (TrayAction::Switches, Page::Switches),
            (TrayAction::Volume, Page::Volume),
            (TrayAction::Tone, Page::Tone),
            (TrayAction::Enter, Page::Enter),
        ] {
            let ctx = egui::Context::default();
            let mut app = hidden_app(&ctx);
            let output = ctx.run(egui::RawInput::default(), |ctx| {
                app.tray_action(ctx, action)
            });
            let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
            assert!(commands.contains(&egui::ViewportCommand::Maximized(false)));
            assert!(commands.contains(&egui::ViewportCommand::InnerSize(egui::vec2(840.0, 660.0))));
            assert!(commands.contains(&egui::ViewportCommand::Visible(true)));
            assert!(commands.contains(&egui::ViewportCommand::Minimized(false)));
            assert!(commands.contains(&egui::ViewportCommand::Focus));
            assert!(!app.hidden);
            assert_eq!(app.page, page);
        }
    }

    #[test]
    fn tray_menu_keeps_the_main_window_hidden() {
        let ctx = egui::Context::default();
        let mut app = hidden_app(&ctx);
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            app.tray_action(ctx, TrayAction::Menu { x: 200, y: 200 });
        });
        assert!(app.hidden);
        assert!(app.popup.as_ref().unwrap().is_open());
        assert!(
            !output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::Visible(true))
        );
    }

    #[test]
    fn tray_quit_requests_exit_instead_of_hiding_again() {
        let ctx = egui::Context::default();
        let mut app = hidden_app(&ctx);
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            app.tray_action(ctx, TrayAction::Quit)
        });
        assert!(app.exiting);
        let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
        assert!(commands.contains(&egui::ViewportCommand::Close));
        assert!(!commands.contains(&egui::ViewportCommand::CancelClose));
    }

    #[test]
    fn lost_tray_restores_window_and_close_is_allowed() {
        let ctx = egui::Context::default();
        let mut app = hidden_app(&ctx);
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let mut frame = eframe::Frame::_new_kittest();
        let output = ctx.run(input, |ctx| {
            app.tray_action(ctx, TrayAction::Unavailable);
            app.update(ctx, &mut frame);
        });
        assert!(!app.hidden);
        assert!(app.tray_error.is_some());
        let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
        assert!(commands.contains(&egui::ViewportCommand::Visible(true)));
        assert!(!commands.contains(&egui::ViewportCommand::CancelClose));
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.saved);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        #[cfg(windows)]
        self.tray_actions(ctx);
        #[cfg(target_os = "macos")]
        self.mac_actions(ctx);
        #[cfg(windows)]
        if let Some(popup) = &mut self.popup {
            let before = self.saved.enabled;
            let action = popup.show(ctx, &mut self.saved);
            let open = popup.is_open();
            self.shared.update(&self.saved.settings);
            if before != self.saved.enabled {
                self.set_enabled(self.saved.enabled);
            }
            if !open {
                self.popup = None;
                if let Some(tray) = &self.tray {
                    tray.set_popup_open(false);
                }
            }
            if let Some(action) = action {
                self.tray_action(ctx, action);
            }
        }
        if ctx.input(|input| input.viewport().close_requested())
            && !self.exiting
            && self.saved.close_to_tray
            && self.tray_available()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.hidden = true;
            self.audition.cancel();
            self.preview.cancel();
            if let Some(storage) = frame.storage_mut() {
                self.save(storage);
                storage.flush();
            }
        }
        while let Some(event) = self.preview.next_due(Instant::now()) {
            self.shared.push(event);
        }
        self.focused_keys(ctx);
        if self.shared.stream_failed.swap(false, Ordering::Relaxed) {
            self.audio_error = Some("Output disconnected or interrupted. Reconnecting…".to_owned());
            self.audio = None;
        }
        if self.audio.is_none() && self.last_retry.elapsed() > Duration::from_secs(3) {
            self.restart_audio();
        }
        if self.hidden {
            ui::reset_visualizer(ctx);
            self.shared.set_visualize(false);
            #[cfg(target_os = "macos")]
            ctx.request_repaint_after(Duration::from_millis(250));
            return;
        }
        let old_enabled = self.saved.enabled;
        let old_enter_enabled = self.saved.settings.enter.enabled;
        let old_page = self.page;
        let old_switch = self.saved.switch;
        let old_device = self.saved.device.clone();
        let tray_available = self.tray_available();
        let visual_reset = self.shared.take_visual_events(&mut self.visual_events);
        let actions = ui::show(
            ctx,
            &mut self.page,
            &mut self.saved,
            ui::Status {
                key_events: &self.visual_events,
                visual_reset,
                audio_ready: self.audio.is_some(),
                input: &self.input_status,
                output_name: self.audio.as_ref().map(|audio| audio.device_name.as_str()),
                audio_error: self
                    .audio_error
                    .as_deref()
                    .or_else(|| self.audition.error()),
                devices: &self.devices,
                tray_available,
                tray_error: self.tray_error.as_deref(),
                image: Some(&self.image),
            },
        );
        if old_enabled != self.saved.enabled {
            self.set_enabled(self.saved.enabled);
        }
        if old_enter_enabled && !self.saved.settings.enter.enabled {
            self.preview.cancel();
        }
        self.shared.update(&self.saved.settings);
        if old_switch != self.saved.switch {
            match SampleBank::load(self.saved.switch) {
                Ok(bank) => {
                    self.bank = Arc::new(bank);
                    self.restart_audio();
                }
                Err(error) => {
                    self.saved.switch = old_switch;
                    self.audio_error = Some(error);
                }
            }
        }
        if actions.refresh_output {
            match audio::output_devices() {
                Ok(devices) => {
                    self.devices = devices;
                    self.restart_audio();
                }
                Err(error) => self.audio_error = Some(error),
            }
        } else if old_device != self.saved.device {
            self.restart_audio();
        }
        if old_page != self.page {
            self.preview.cancel();
        }
        if self.saved.enabled
            && self.audio.is_some()
            && let Some(target) = actions.tone_preview
        {
            self.preview.tone(target.scan(), Instant::now());
        }
        self.audition.update(
            if self.page == Page::Switches && self.saved.enabled && self.audio.is_some() {
                actions.switch_hover
            } else {
                None
            },
            &self.saved.settings,
            self.saved.device.as_deref(),
            Instant::now(),
        );
        ctx.request_repaint_after(self.audition.delay(Instant::now()));
        #[cfg(target_os = "macos")]
        if actions.input_permission {
            reeb::macos::request_input_access();
            self.input_retry = Instant::now() - Duration::from_secs(3);
        }
        if actions.quit {
            self.quit(ctx);
        }
        self.shared.set_visualize(self.page == Page::Settings);
        if self.page == Page::Settings {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
        ctx.request_repaint_after(self.preview.delay(Instant::now()));
    }
}
