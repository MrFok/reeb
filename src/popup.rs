use crate::{
    config::Saved,
    motion, theme,
    ui::{self, Page},
};
use eframe::egui;
use reeb::tray::TrayAction;

pub const WIDTH: f32 = 280.0;
pub const HEIGHT: f32 = 356.0;
const TITLE: &str = "Reeb controls";

pub struct Popup {
    position: egui::Pos2,
    visible: bool,
    focused: bool,
    open: bool,
    keyboard: usize,
}

impl Popup {
    pub fn new(x: i32, y: i32, visible: bool) -> Self {
        Self {
            position: position(x, y),
            visible,
            focused: false,
            open: true,
            keyboard: 0,
        }
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn show(&mut self, ctx: &egui::Context, saved: &mut Saved) -> Option<TrayAction> {
        theme::select(ctx, saved.theme);
        let id = egui::ViewportId::from_hash_of("reeb-controls");
        ctx.show_viewport_immediate(
            id,
            egui::ViewportBuilder::default()
                .with_title(TITLE)
                .with_inner_size([WIDTH, HEIGHT])
                .with_position(self.position)
                .with_decorations(false)
                .with_resizable(false)
                .with_taskbar(false)
                .with_visible(self.visible)
                .with_active(self.visible)
                .with_window_level(egui::WindowLevel::AlwaysOnTop),
            |ctx, class| {
                motion::begin(ctx);
                let (focus, close, escape) = ctx.input(|input| {
                    (
                        input.viewport().focused,
                        input.viewport().close_requested(),
                        input.key_pressed(egui::Key::Escape),
                    )
                });
                if focus == Some(true) {
                    self.focused = true;
                }
                if close || escape || (self.visible && self.focused && focus == Some(false)) {
                    self.open = false;
                }
                let action = if class == egui::ViewportClass::Embedded {
                    egui::Window::new(TITLE)
                        .title_bar(false)
                        .resizable(false)
                        .fixed_size([WIDTH, HEIGHT])
                        .show(ctx, |ui| content(ui, saved, &mut self.keyboard))
                        .and_then(|result| result.inner)
                        .flatten()
                } else {
                    reeb::window::round_owned_window(TITLE);
                    panel(ctx, saved, &mut self.keyboard)
                };
                if action.is_some() {
                    self.open = false;
                }
                if !self.open {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                action
            },
        )
    }
}

fn position(x: i32, y: i32) -> egui::Pos2 {
    use windows_sys::Win32::{
        Foundation::POINT,
        Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint},
        UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
    };
    unsafe {
        let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..std::mem::zeroed()
        };
        let (mut dpi, mut dpi_y) = (96, 96);
        GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut dpi_y);
        let scale = dpi as f32 / 96.0;
        if GetMonitorInfoW(monitor, &mut info) != 0 {
            let area = info.rcWork;
            egui::pos2(
                (x as f32).clamp(
                    area.left as f32,
                    (area.right as f32 - WIDTH * scale).max(area.left as f32),
                ) / scale,
                (y as f32).clamp(
                    area.top as f32,
                    (area.bottom as f32 - HEIGHT * scale).max(area.top as f32),
                ) / scale,
            )
        } else {
            egui::pos2(x as f32 / scale, y as f32 / scale)
        }
    }
}

pub fn panel(ctx: &egui::Context, saved: &mut Saved, keyboard: &mut usize) -> Option<TrayAction> {
    let colors = theme::palette(ctx);
    let background = if ctx.theme() == egui::Theme::Dark {
        egui::Color32::from_rgb(16, 17, 19)
    } else {
        colors.card
    };
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(background)
                .stroke(egui::Stroke::new(1.0, colors.border))
                .corner_radius(9)
                .inner_margin(10),
        )
        .show(ctx, |ui| content(ui, saved, keyboard))
        .inner
}

fn section(ui: &mut egui::Ui, label: &str) {
    ui.add_space(3.0);
    ui.label(
        egui::RichText::new(label)
            .size(11.0)
            .color(theme::palette(ui.ctx()).muted),
    );
    ui.add_space(2.0);
}

enum Icon {
    Page(Page),
    Power(bool),
    Quit,
}

fn row(
    ui: &mut egui::Ui,
    label: &str,
    index: usize,
    keyboard: usize,
    icon: Icon,
    request_focus: bool,
) -> egui::Response {
    let page = match icon {
        Icon::Page(page) => Some(page),
        _ => None,
    };
    let power = match icon {
        Icon::Power(enabled) => Some(enabled),
        _ => None,
    };
    let quit = matches!(icon, Icon::Quit);
    let colors = theme::palette(ui.ctx());
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), egui::Sense::hover());
    let response = ui.interact(
        rect,
        egui::Id::new(("popup-row", index)),
        egui::Sense::click(),
    );
    if request_focus && keyboard == index {
        response.request_focus();
    }
    let hover = motion::spring(
        ui.ctx(),
        response.id.with("hover"),
        if response.hovered() || response.has_focus() {
            1.0
        } else {
            0.0
        },
        false,
    );
    ui.painter()
        .rect_filled(rect, 6, colors.surface.gamma_multiply(hover));
    let origin = rect.left_center() + egui::vec2(17.0, 0.0);
    let color = if power == Some(true) {
        colors.accent
    } else {
        colors.ink
    };
    if let Some(page) = page {
        ui::glyph(ui.painter(), origin, page, color);
    } else {
        let stroke = egui::Stroke::new(1.4, color);
        ui.painter().circle_stroke(origin, 7.0, stroke);
        let points = if quit {
            vec![
                origin + egui::vec2(-2.5, -2.5),
                origin + egui::vec2(2.5, 2.5),
            ]
        } else if power == Some(true) {
            vec![
                origin + egui::vec2(-4.0, 0.0),
                origin + egui::vec2(-1.0, 3.0),
                origin + egui::vec2(4.0, -3.0),
            ]
        } else {
            vec![
                origin + egui::vec2(-6.0, -6.0),
                origin + egui::vec2(6.0, 6.0),
            ]
        };
        ui.painter().add(egui::Shape::line(points, stroke));
        if quit {
            ui.painter().line_segment(
                [
                    origin + egui::vec2(-2.5, 2.5),
                    origin + egui::vec2(2.5, -2.5),
                ],
                stroke,
            );
        }
    }
    ui.painter().text(
        rect.left_center() + egui::vec2(36.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        colors.ink,
    );
    if page.is_some_and(|page| page != Page::Settings) {
        let center = rect.right_center() - egui::vec2(12.0, 0.0);
        ui.painter().add(egui::Shape::line(
            vec![
                center + egui::vec2(-2.0, -4.0),
                center + egui::vec2(2.0, 0.0),
                center + egui::vec2(-2.0, 4.0),
            ],
            egui::Stroke::new(1.4, colors.muted),
        ));
    }
    response.widget_info(|| {
        if let Some(enabled) = power {
            egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, enabled, label)
        } else {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label)
        }
    });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn content(ui: &mut egui::Ui, saved: &mut Saved, keyboard: &mut usize) -> Option<TrayAction> {
    let colors = theme::palette(ui.ctx());
    ui.spacing_mut().item_spacing.y = 0.0;
    let (up, down) = ui.input(|input| {
        (
            input.key_pressed(egui::Key::ArrowUp),
            input.key_pressed(egui::Key::ArrowDown),
        )
    });
    if down {
        *keyboard = (*keyboard + 1) % 6;
    } else if up {
        *keyboard = (*keyboard + 5) % 6;
    }
    let focus = up || down;
    if focus {
        ui.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
    }
    let mut action = None;
    section(ui, "Control");
    let enabled = saved.enabled;
    if row(
        ui,
        if enabled {
            "Disable Reeb"
        } else {
            "Enable Reeb"
        },
        0,
        *keyboard,
        Icon::Power(enabled),
        focus,
    )
    .clicked()
    {
        saved.enabled = !saved.enabled;
    }
    ui.horizontal(|ui| {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(27.0, 28.0), egui::Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Volume settings")
        });
        if response.clicked() {
            action = Some(TrayAction::Volume);
        }
        response
            .on_hover_text("Volume settings")
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        ui::glyph(ui.painter(), rect.center(), Page::Volume, colors.muted);
        ui.spacing_mut().slider_width = (ui.available_width() - 12.0).max(80.0);
        let slider = ui
            .scope(|ui| {
                ui.spacing_mut().interact_size.y = 16.0;
                ui.spacing_mut().slider_rail_height = 3.0;
                ui.add(
                    egui::Slider::new(&mut saved.settings.volume, 0.0..=1.0)
                        .show_value(false)
                        .handle_shape(egui::style::HandleShape::Circle)
                        .trailing_fill(true),
                )
            })
            .inner;
        slider
            .widget_info(|| egui::WidgetInfo::slider(true, saved.settings.volume as f64, "Volume"));
        slider.on_hover_text("Volume");
    });
    ui.add_space(7.0);
    ui.separator();
    section(ui, "Configure");
    for (index, label, page, value) in [
        (1, "Switches", Page::Switches, TrayAction::Switches),
        (2, "Enter sound", Page::Enter, TrayAction::Enter),
        (3, "2D tone pad", Page::Tone, TrayAction::Tone),
    ] {
        if row(ui, label, index, *keyboard, Icon::Page(page), focus).clicked() {
            action = Some(value);
        }
    }
    ui.add_space(7.0);
    ui.separator();
    section(ui, "App");
    if row(
        ui,
        "Settings…",
        4,
        *keyboard,
        Icon::Page(Page::Settings),
        focus,
    )
    .clicked()
    {
        action = Some(TrayAction::Settings);
    }
    if row(ui, "Quit Reeb", 5, *keyboard, Icon::Quit, focus).clicked() {
        action = Some(TrayAction::Quit);
    }
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(
        ctx: &egui::Context,
        saved: &mut Saved,
        keyboard: &mut usize,
        events: Vec<egui::Event>,
    ) -> (egui::FullOutput, Option<TrayAction>) {
        let mut action = None;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(WIDTH, HEIGHT),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                action = panel(ctx, saved, keyboard);
            },
        );
        (output, action)
    }
    fn click(point: egui::Pos2) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(point),
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }
    fn key(key: egui::Key) -> Vec<egui::Event> {
        vec![egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]
    }
    #[test]
    fn popup_routes_pages_and_quit_and_toggle_stays_open() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut saved = Saved::default();
        let mut keyboard = 0;
        draw(&ctx, &mut saved, &mut keyboard, vec![]);
        let power = ctx
            .read_response(egui::Id::new(("popup-row", 0_usize)))
            .unwrap()
            .rect;
        let (_, action) = draw(&ctx, &mut saved, &mut keyboard, click(power.center()));
        assert!(!saved.enabled);
        assert_eq!(action, None);
        for (index, expected) in [
            (1, TrayAction::Switches),
            (2, TrayAction::Enter),
            (3, TrayAction::Tone),
            (4, TrayAction::Settings),
            (5, TrayAction::Quit),
        ] {
            let rect = ctx
                .read_response(egui::Id::new(("popup-row", index as usize)))
                .unwrap()
                .rect;
            assert!(rect.bottom() < HEIGHT - 10.0);
            let (_, action) = draw(&ctx, &mut saved, &mut keyboard, click(rect.center()));
            assert_eq!(action, Some(expected));
        }
        let original = saved.settings.volume;
        let (_, action) = draw(
            &ctx,
            &mut saved,
            &mut keyboard,
            click(egui::pos2(220.0, 80.0))[..2].to_vec(),
        );
        assert_eq!(action, None);
        assert!(saved.settings.volume > original);
    }
    #[test]
    fn popup_arrow_keys_focus_rows_and_enter_opens_the_selected_page() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut saved = Saved::default();
        let mut keyboard = 0;
        draw(&ctx, &mut saved, &mut keyboard, vec![]);
        draw(&ctx, &mut saved, &mut keyboard, key(egui::Key::ArrowDown));
        assert_eq!(keyboard, 1);
        let (_, action) = draw(&ctx, &mut saved, &mut keyboard, key(egui::Key::Enter));
        assert_eq!(action, Some(TrayAction::Switches));
    }
    #[test]
    fn escape_dismisses_popup_without_changing_preferences() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut saved = Saved::default();
        let before = (
            saved.enabled,
            saved.switch,
            saved.settings.volume,
            saved.theme,
        );
        let mut popup = Popup::new(0, 0, false);
        let _ = ctx.run(
            egui::RawInput {
                events: key(egui::Key::Escape),
                ..Default::default()
            },
            |ctx| {
                popup.show(ctx, &mut saved);
            },
        );
        assert!(!popup.is_open());
        assert_eq!(
            (
                saved.enabled,
                saved.switch,
                saved.settings.volume,
                saved.theme
            ),
            before
        );
    }
}
