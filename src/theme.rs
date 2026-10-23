use eframe::egui;
use reeb::appearance::ThemeMode;

pub const BLUE: egui::Color32 = egui::Color32::from_rgb(40, 101, 232);
pub const PAGE_TITLE: f32 = 22.0;
pub const SECTION_TITLE: f32 = 14.0;
pub const BODY: f32 = 13.0;
pub const CAPTION: f32 = 11.5;
pub const ROW_HEIGHT: f32 = 34.0;
pub const LABEL_WIDTH: f32 = 130.0;
pub const CONTROL_HEIGHT: f32 = 32.0;
pub const SECTION_GAP: f32 = 20.0;
pub const CARD_PADDING: i8 = 16;
pub const PAGE_PADDING: i8 = 24;

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: egui::Color32,
    pub surface: egui::Color32,
    pub card: egui::Color32,
    pub ink: egui::Color32,
    pub muted: egui::Color32,
    pub accent: egui::Color32,
    pub border: egui::Color32,
    pub hover: egui::Color32,
    pub success: egui::Color32,
    pub error: egui::Color32,
}

impl Palette {
    pub fn new(dark: bool) -> Self {
        let rgb = egui::Color32::from_rgb;
        if dark {
            Self {
                background: rgb(25, 26, 28),
                surface: rgb(44, 46, 49),
                card: rgb(31, 33, 36),
                ink: rgb(239, 241, 244),
                muted: rgb(166, 173, 184),
                accent: rgb(140, 180, 255),
                border: rgb(55, 58, 63),
                hover: rgb(38, 40, 44),
                success: rgb(89, 195, 150),
                error: rgb(248, 139, 144),
            }
        } else {
            Self {
                background: rgb(247, 246, 244),
                surface: rgb(240, 238, 234),
                card: egui::Color32::WHITE,
                ink: rgb(38, 39, 37),
                muted: rgb(112, 115, 111),
                accent: BLUE,
                border: rgb(228, 228, 223),
                hover: rgb(246, 245, 242),
                success: rgb(40, 141, 101),
                error: rgb(183, 64, 67),
            }
        }
    }
}

pub fn palette(ctx: &egui::Context) -> Palette {
    Palette::new(ctx.theme() == egui::Theme::Dark)
}

pub fn select(ctx: &egui::Context, mode: ThemeMode) {
    let preference = match mode {
        ThemeMode::System => egui::ThemePreference::System,
        ThemeMode::Light => egui::ThemePreference::Light,
        ThemeMode::Dark => egui::ThemePreference::Dark,
    };
    if ctx.options(|options| options.theme_preference) != preference {
        ctx.set_theme(preference);
        ctx.request_repaint();
    }
}

pub fn apply(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for (name, bytes) in [
        (
            "Jakarta Medium",
            include_bytes!("../assets/ui/Jakarta-Medium.ttf").as_slice(),
        ),
        (
            "Jakarta SemiBold",
            include_bytes!("../assets/ui/Jakarta-SemiBold.ttf").as_slice(),
        ),
        (
            "Jakarta Bold",
            include_bytes!("../assets/ui/Jakarta-Bold.ttf").as_slice(),
        ),
    ] {
        fonts
            .font_data
            .insert(name.to_owned(), egui::FontData::from_static(bytes).into());
        let mut fallback = fonts.families[&egui::FontFamily::Proportional].clone();
        fallback.insert(0, name.to_owned());
        fonts
            .families
            .insert(egui::FontFamily::Name(name.into()), fallback);
    }
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "Jakarta Medium".to_owned());
    ctx.set_fonts(fonts);
    ctx.options_mut(|options| options.fallback_theme = egui::Theme::Light);
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let colors = Palette::new(theme == egui::Theme::Dark);
        let mut visuals = if theme == egui::Theme::Dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.override_text_color = Some(colors.ink);
        visuals.weak_text_color = Some(colors.muted);
        visuals.panel_fill = colors.background;
        visuals.window_fill = colors.card;
        visuals.faint_bg_color = colors.surface;
        visuals.extreme_bg_color = colors.background;
        visuals.text_edit_bg_color = Some(colors.card);
        visuals.selection.bg_fill = BLUE;
        visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        visuals.hyperlink_color = colors.accent;
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, colors.border);
        visuals.window_stroke = egui::Stroke::new(1.0, colors.border);
        for widget in [
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            widget.corner_radius = egui::CornerRadius::same(6);
            widget.bg_stroke = egui::Stroke::new(1.0, colors.border);
            widget.fg_stroke = egui::Stroke::new(1.2, colors.ink);
        }
        visuals.widgets.inactive.bg_fill = colors.card;
        visuals.widgets.inactive.weak_bg_fill = colors.card;
        visuals.widgets.hovered.bg_fill = colors.hover;
        visuals.widgets.hovered.weak_bg_fill = colors.hover;
        visuals.widgets.active.bg_fill = colors.surface;
        visuals.widgets.active.weak_bg_fill = colors.surface;
        visuals.widgets.open.bg_fill = colors.surface;
        visuals.widgets.open.weak_bg_fill = colors.surface;
        ctx.set_visuals_of(theme, visuals);
    }
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.interact_size.y = CONTROL_HEIGHT;
        style.spacing.slider_width = 240.0;
        style.spacing.scroll.bar_width = 4.0;
        style
            .text_styles
            .insert(egui::TextStyle::Heading, heading(PAGE_TITLE));
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(BODY));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(BODY));
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(CAPTION));
    });
}

pub fn heading(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("Jakarta SemiBold".into()))
}

pub fn wordmark() -> egui::FontId {
    egui::FontId::new(24.0, egui::FontFamily::Name("Jakarta Bold".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_theme_tracks_os_changes_and_explicit_modes_override_them() {
        let ctx = egui::Context::default();
        apply(&ctx);
        for (mode, system, expected) in [
            (ThemeMode::System, egui::Theme::Dark, egui::Theme::Dark),
            (ThemeMode::System, egui::Theme::Light, egui::Theme::Light),
            (ThemeMode::Dark, egui::Theme::Light, egui::Theme::Dark),
            (ThemeMode::Light, egui::Theme::Dark, egui::Theme::Light),
            (ThemeMode::System, egui::Theme::Dark, egui::Theme::Dark),
        ] {
            let _ = ctx.run(
                egui::RawInput {
                    system_theme: Some(system),
                    ..Default::default()
                },
                |ctx| {
                    select(ctx, mode);
                    assert_eq!(ctx.theme(), expected);
                    assert_eq!(ctx.style().visuals.dark_mode, expected == egui::Theme::Dark);
                    assert_eq!(ctx.style().text_styles[&egui::TextStyle::Body].size, BODY);
                    let colors = palette(ctx);
                    assert!(colors.ink.is_opaque() && colors.card.is_opaque());
                },
            );
        }
    }
}
