use crate::{config::Saved, motion, theme};
use eframe::egui;
use reeb::{appearance::ThemeMode, keys::KeyEvent, mixer::Settings};

#[path = "ui/chrome.rs"]
mod chrome;
#[path = "ui/keyboard.rs"]
mod keyboard;
#[path = "ui/power.rs"]
mod power;
#[path = "ui/scroll.rs"]
mod scroll;
#[path = "ui/switch_grid.rs"]
mod switch_grid;
#[path = "ui/tone_pad.rs"]
mod tone_pad;

#[derive(Clone, Copy, Default, Debug, Hash, PartialEq, Eq)]
pub enum Page {
    #[default]
    Switches,
    Volume,
    Tone,
    Enter,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToneTarget {
    Keyboard,
    Enter,
}

impl ToneTarget {
    pub fn scan(self) -> u16 {
        match self {
            Self::Keyboard => 0x1e,
            Self::Enter => 0x1c,
        }
    }
}

pub struct Status<'a> {
    pub key_events: &'a [KeyEvent],
    pub visual_reset: bool,
    pub audio_ready: bool,
    pub input: &'a str,
    pub output_name: Option<&'a str>,
    pub audio_error: Option<&'a str>,
    pub devices: &'a [String],
    pub tray_available: bool,
    pub tray_error: Option<&'a str>,
    pub image: Option<&'a egui::TextureHandle>,
}

#[derive(Default)]
pub struct Actions {
    pub switch_hover: Option<reeb::samples::Switch>,
    pub tone_preview: Option<ToneTarget>,
    pub refresh_output: bool,
    pub quit: bool,
}

pub fn reset_visualizer(ctx: &egui::Context) {
    keyboard::clear(ctx);
    power::clear(ctx);
}

pub fn show(
    ctx: &egui::Context,
    page: &mut Page,
    saved: &mut Saved,
    status: Status<'_>,
) -> Actions {
    theme::select(ctx, saved.theme);
    motion::begin(ctx);
    let colors = theme::palette(ctx);
    let mut actions = Actions::default();
    ctx.layer_painter(egui::LayerId::background()).rect_filled(
        ctx.content_rect(),
        0,
        colors.background,
    );
    chrome::show(ctx);
    if *page != Page::Settings {
        reset_visualizer(ctx);
    }
    egui::TopBottomPanel::bottom("status")
        .frame(
            egui::Frame::new()
                .fill(colors.background)
                .inner_margin(egui::Margin::symmetric(20, 10)),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().interact_size.y = 16.0;
            ui.horizontal(|ui| {
                let color = if saved.enabled && status.audio_ready {
                    colors.success
                } else {
                    colors.muted
                };
                let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 3.0, color);
                ui.label(
                    egui::RichText::new(if !saved.enabled {
                        "Sounds paused"
                    } else if status.audio_ready {
                        "Ready to type"
                    } else {
                        "Audio unavailable"
                    })
                    .small()
                    .color(colors.muted),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(
                            if cfg!(windows) && saved.close_to_tray && status.tray_available {
                                "Runs in the tray when closed"
                            } else {
                                "Close to quit"
                            },
                        )
                        .small()
                        .color(colors.muted),
                    );
                });
            });
            for error in [status.audio_error, status.tray_error]
                .into_iter()
                .flatten()
            {
                ui.label(egui::RichText::new(error).small().color(colors.error));
            }
        });
    egui::SidePanel::left("navigation")
        .exact_width(176.0)
        .resizable(false)
        .show_separator_line(true)
        .frame(
            egui::Frame::new()
                .fill(colors.card)
                .inner_margin(egui::Margin::symmetric(16, 16)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(image) = status.image {
                    ui.image((image.id(), egui::vec2(32.0, 32.0)));
                }
                ui.label(egui::RichText::new("reeb").font(theme::wordmark()));
            });
            ui.add_space(24.0);
            ui.label(
                egui::RichText::new("Keyboard")
                    .size(11.0)
                    .color(colors.muted),
            );
            ui.add_space(4.0);
            let highlight = ui.painter().add(egui::Shape::Noop);
            let mut selected_rect = egui::Rect::NOTHING;
            for (value, label) in [
                (Page::Switches, "Switches"),
                (Page::Volume, "Volume"),
                (Page::Tone, "Tone pad"),
                (Page::Enter, "Enter"),
                (Page::Settings, "Settings"),
            ] {
                if value == Page::Settings {
                    ui.add_space(14.0);
                }
                let response = navigation(ui, *page == value, value, label);
                if response.clicked()
                    || (response.has_focus()
                        && ui.input(|input| {
                            input.key_pressed(egui::Key::Enter)
                                || input.key_pressed(egui::Key::Space)
                        }))
                {
                    *page = value;
                }
                if *page == value {
                    selected_rect = response.rect;
                }
            }
            if selected_rect.is_finite() {
                let y = motion::spring(
                    ctx,
                    egui::Id::new("navigation-y"),
                    selected_rect.top(),
                    false,
                );
                let rect = selected_rect.translate(egui::vec2(0.0, y - selected_rect.top()));
                ui.painter()
                    .set(highlight, egui::Shape::rect_filled(rect, 6, colors.surface));
            }
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.label(
                    egui::RichText::new("No typed text is recorded.")
                        .size(10.0)
                        .color(colors.muted),
                );
                ui.add_space(12.0);
                if *page != Page::Settings {
                    toggle(ui, &mut saved.enabled, "Keyboard sounds");
                }
            });
        });
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(colors.card)
                .inner_margin(egui::Margin {
                    left: theme::PAGE_PADDING,
                    right: theme::PAGE_PADDING,
                    top: theme::PAGE_PADDING,
                    bottom: theme::PAGE_PADDING,
                }),
        )
        .show(ctx, |ui| {
            scroll::show(ui, *page, |ui| {
                ui.set_min_width(ui.available_width());
                ui.set_opacity(motion::page_opacity(ctx, egui::Id::new(*page)));
                match page {
                    Page::Switches => switches(ui, saved, &mut actions),
                    Page::Volume => volume(ui, saved),
                    Page::Tone => tones(ui, saved, &status, &mut actions),
                    Page::Enter => enter(ui, saved, &status, &mut actions),
                    Page::Settings => settings(ui, saved, &status, &mut actions),
                }
            });
        });
    theme::select(ctx, saved.theme);
    actions
}

fn navigation(ui: &mut egui::Ui, selected: bool, page: Page, label: &str) -> egui::Response {
    let colors = theme::palette(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), theme::ROW_HEIGHT),
        egui::Sense::click(),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let hover = motion::hover(ui.ctx(), response.id.with("hover"), response.hovered());
    if !selected && hover > 0.0 {
        ui.painter()
            .rect_filled(rect, 6, colors.hover.gamma_multiply(hover));
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            6,
            egui::Stroke::new(1.0, colors.accent),
            egui::StrokeKind::Inside,
        );
    }
    let color = if selected { colors.ink } else { colors.muted };
    let painter = ui.painter();
    let origin = rect.left_center() + egui::vec2(18.0, 0.0);
    glyph(painter, origin, page, color);
    painter.text(
        rect.left_center() + egui::vec2(36.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        if selected {
            theme::heading(13.0)
        } else {
            egui::FontId::proportional(13.0)
        },
        color,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn glyph(painter: &egui::Painter, origin: egui::Pos2, page: Page, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.3, color);
    match page {
        Page::Switches => {
            for x in [-3.5, 3.5] {
                for y in [-3.5, 3.5] {
                    painter.rect_stroke(
                        egui::Rect::from_center_size(
                            origin + egui::vec2(x, y),
                            egui::vec2(5.0, 5.0),
                        ),
                        1,
                        stroke,
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
        Page::Volume => {
            for (x, height) in [(-5.0, 4.0), (0.0, 7.0), (5.0, 5.0)] {
                painter.line_segment(
                    [
                        origin + egui::vec2(x, -height),
                        origin + egui::vec2(x, height),
                    ],
                    stroke,
                );
            }
        }
        Page::Tone => {
            painter.rect_stroke(
                egui::Rect::from_center_size(origin, egui::vec2(13.0, 13.0)),
                3,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.circle_filled(origin + egui::vec2(2.0, -2.0), 2.0, color);
        }
        Page::Enter => {
            painter.add(egui::Shape::line(
                vec![
                    origin + egui::vec2(5.0, -5.0),
                    origin + egui::vec2(5.0, 3.0),
                    origin + egui::vec2(-5.0, 3.0),
                ],
                stroke,
            ));
            painter.add(egui::Shape::line(
                vec![
                    origin + egui::vec2(-1.0, -1.0),
                    origin + egui::vec2(-5.0, 3.0),
                    origin + egui::vec2(-1.0, 7.0),
                ],
                stroke,
            ));
        }
        Page::Settings => {
            painter.rect_stroke(
                egui::Rect::from_center_size(origin, egui::vec2(25.0, 25.0)),
                5,
                egui::Stroke::new(1.0, theme::palette(painter.ctx()).border),
                egui::StrokeKind::Inside,
            );
            painter.circle_stroke(origin, 6.0, stroke);
            painter.circle_stroke(origin, 2.0, stroke);
            for delta in [
                egui::vec2(8.0, 0.0),
                egui::vec2(-8.0, 0.0),
                egui::vec2(0.0, 8.0),
                egui::vec2(0.0, -8.0),
            ] {
                painter.line_segment([origin + delta * 0.7, origin + delta], stroke);
            }
        }
    }
}

fn toggle(ui: &mut egui::Ui, value: &mut bool, label: &str) -> egui::Response {
    let colors = theme::palette(ui.ctx());
    let mut response = ui
        .horizontal(|ui| {
            let label_response = ui.add(egui::Label::new(label).sense(egui::Sense::click()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (rect, mut response) =
                    ui.allocate_exact_size(egui::vec2(32.0, 19.0), egui::Sense::click());
                if response.clicked()
                    || label_response.clicked()
                    || (ui.is_enabled()
                        && response.has_focus()
                        && ui.input(|input| {
                            input.key_pressed(egui::Key::Space)
                                || input.key_pressed(egui::Key::Enter)
                        }))
                {
                    *value = !*value;
                    response.mark_changed();
                }
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Checkbox,
                        ui.is_enabled(),
                        *value,
                        label,
                    )
                });
                let t = motion::spring(
                    ui.ctx(),
                    response.id.with("thumb"),
                    if *value { 1.0 } else { 0.0 },
                    false,
                )
                .clamp(0.0, 1.0);
                let color = colors.border.lerp_to_gamma(theme::BLUE, t);
                ui.painter().rect_filled(rect, 10, color);
                let center = egui::pos2(
                    egui::lerp(rect.left() + 9.5..=rect.right() - 9.5, t),
                    rect.center().y,
                );
                ui.painter()
                    .circle_filled(center, 6.5, egui::Color32::WHITE);
                if response.has_focus() {
                    ui.painter().rect_stroke(
                        rect.expand(3.0),
                        12,
                        egui::Stroke::new(1.0, colors.accent),
                        egui::StrokeKind::Outside,
                    );
                }
                response
            })
            .inner
        })
        .inner;
    response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    response
}

fn title(ui: &mut egui::Ui, heading: &str, description: &str) {
    let colors = theme::palette(ui.ctx());
    ui.heading(heading);
    ui.add(egui::Label::new(egui::RichText::new(description).color(colors.muted)).wrap());
    ui.add_space(theme::SECTION_GAP);
}

fn card(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    let colors = theme::palette(ui.ctx());
    egui::Frame::new()
        .fill(colors.card)
        .stroke(egui::Stroke::new(1.0, colors.border))
        .corner_radius(8)
        .inner_margin(theme::CARD_PADDING)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            content(ui);
        });
}

fn slider(ui: &mut egui::Ui, label: &str, value: &mut f32) {
    let colors = theme::palette(ui.ctx());
    ui.push_id(label, |ui| {
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(theme::LABEL_WIDTH, theme::CONTROL_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_width(theme::LABEL_WIDTH);
                    ui.label(label);
                },
            );
            ui.scope(|ui| {
                ui.spacing_mut().slider_width = (ui.available_width() - 52.0).max(80.0);
                ui.spacing_mut().slider_rail_height = 3.0;
                ui.spacing_mut().interact_size.y = 20.0;
                let visuals = ui.visuals_mut();
                for visual in [&mut visuals.widgets.hovered, &mut visuals.widgets.active] {
                    visual.bg_fill = colors.accent;
                    visual.fg_stroke = egui::Stroke::NONE;
                }
                visuals.widgets.inactive.bg_fill = colors.surface;
                visuals.widgets.inactive.fg_stroke = egui::Stroke::new(2.0, colors.accent);
                ui.add(
                    egui::Slider::new(value, 0.0..=1.0)
                        .show_value(false)
                        .trailing_fill(true)
                        .handle_shape(egui::style::HandleShape::Circle),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{:.0}%", *value * 100.0)).color(colors.muted),
                );
            });
        });
    });
}

fn switches(ui: &mut egui::Ui, saved: &mut Saved, actions: &mut Actions) {
    title(ui, "Switches", "Hover to listen. Click to select.");
    ui.horizontal(|ui| {
        switch_grid::image(ui, saved.switch, egui::vec2(36.0, 36.0));
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("Selected switch")
                    .small()
                    .color(theme::palette(ui.ctx()).muted),
            );
            ui.label(egui::RichText::new(saved.switch.name()).font(theme::heading(14.0)));
        });
    });
    ui.add_space(16.0);
    actions.switch_hover = switch_grid::show(ui, &mut saved.switch);
}

fn volume(ui: &mut egui::Ui, saved: &mut Saved) {
    let colors = theme::palette(ui.ctx());
    title(
        ui,
        "Volume settings",
        "Set the overall level and the volume of the Enter key.",
    );
    card(ui, |ui| {
        slider(ui, "Master volume", &mut saved.settings.volume);
        ui.separator();
        slider(ui, "Enter volume", &mut saved.settings.enter.volume);
        ui.label(
            egui::RichText::new("Enter volume is relative to master volume.")
                .small()
                .color(colors.muted),
        );
    });
}

fn tone_editor(
    ui: &mut egui::Ui,
    label: &str,
    x: &mut f32,
    y: &mut f32,
    target: ToneTarget,
    ready: bool,
    actions: &mut Actions,
) -> bool {
    ui.label(egui::RichText::new(label).font(theme::heading(theme::SECTION_TITLE)));
    let mut changed = tone_pad::show(ui, label, x, y).changed();
    ui.horizontal(|ui| {
        if ui.button("Reset tone").clicked() {
            *x = 0.5;
            *y = 0.5;
            changed = true;
        }
    });
    if changed && ready {
        actions.tone_preview = Some(target);
    }
    changed
}

fn enter_tone(ui: &mut egui::Ui, settings: &mut Settings, ready: bool, actions: &mut Actions) {
    let (mut x, mut y) = if settings.enter.use_keyboard_tone {
        (settings.tone_x, settings.tone_y)
    } else {
        (settings.enter.tone_x, settings.enter.tone_y)
    };
    if tone_editor(
        ui,
        "Enter tone",
        &mut x,
        &mut y,
        ToneTarget::Enter,
        ready && settings.enter.enabled,
        actions,
    ) {
        settings.enter.use_keyboard_tone = false;
        settings.enter.tone_x = x;
        settings.enter.tone_y = y;
    }
    toggle(
        ui,
        &mut settings.enter.use_keyboard_tone,
        "Use keyboard tone",
    );
}

fn tones(ui: &mut egui::Ui, saved: &mut Saved, status: &Status<'_>, actions: &mut Actions) {
    title(
        ui,
        "Tone pad",
        "Move right for a brighter tone. Move up for a higher pitch.",
    );
    let ready = saved.enabled && status.audio_ready;
    ui.columns(2, |columns| {
        tone_editor(
            &mut columns[0],
            "Keyboard tone",
            &mut saved.settings.tone_x,
            &mut saved.settings.tone_y,
            ToneTarget::Keyboard,
            ready,
            actions,
        );
        enter_tone(&mut columns[1], &mut saved.settings, ready, actions);
    });
}

fn enter(ui: &mut egui::Ui, saved: &mut Saved, status: &Status<'_>, actions: &mut Actions) {
    let colors = theme::palette(ui.ctx());
    title(
        ui,
        "Enter settings",
        "Adjust the Enter recording from your selected switch.",
    );
    card(ui, |ui| {
        toggle(
            ui,
            &mut saved.settings.enter.enabled,
            "Play Enter key sounds",
        );
        slider(ui, "Enter volume", &mut saved.settings.enter.volume);
        ui.separator();
        let settings = &mut saved.settings;
        let (mut x, mut y) = if settings.enter.use_keyboard_tone {
            (settings.tone_x, settings.tone_y)
        } else {
            (settings.enter.tone_x, settings.enter.tone_y)
        };
        let ready = saved.enabled && status.audio_ready && settings.enter.enabled;
        let mut changed = false;
        ui.columns(2, |columns| {
            changed = tone_pad::show(&mut columns[0], "Enter tone", &mut x, &mut y).changed();
            let ui = &mut columns[1];
            ui.add_space(18.0);
            ui.label(egui::RichText::new("Enter tone").font(theme::heading(theme::SECTION_TITLE)));
            if changed {
                settings.enter.use_keyboard_tone = false;
            }
            toggle(
                ui,
                &mut settings.enter.use_keyboard_tone,
                "Use keyboard tone",
            );
            ui.add(
                egui::Label::new(
                    egui::RichText::new("Move the point to adjust brightness and pitch.")
                        .small()
                        .color(colors.muted),
                )
                .wrap(),
            );
            ui.add_space(10.0);
            if ui.button("Reset tone").clicked() {
                x = 0.5;
                y = 0.5;
                changed = true;
                settings.enter.use_keyboard_tone = false;
            }
        });
        if changed {
            settings.enter.tone_x = x;
            settings.enter.tone_y = y;
            if ready {
                actions.tone_preview = Some(ToneTarget::Enter);
            }
        }
    });
}

fn settings(ui: &mut egui::Ui, saved: &mut Saved, status: &Status<'_>, actions: &mut Actions) {
    let colors = theme::palette(ui.ctx());
    power::show(
        ui,
        &mut saved.enabled,
        status.key_events,
        status.visual_reset,
        status.image,
    );
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width() - 232.0, 40.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.label("Appearance");
                    ui.label(
                        egui::RichText::new("System follows Windows automatically.")
                            .small()
                            .color(colors.muted),
                    );
                },
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::Frame::new()
                    .fill(colors.surface)
                    .corner_radius(7)
                    .inner_margin(3)
                    .show(ui, |ui| {
                        ui.set_width(208.0);
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            for mode in ThemeMode::ALL {
                                let selected = saved.theme == mode;
                                let label = egui::RichText::new(mode.label()).color(if selected {
                                    colors.ink
                                } else {
                                    colors.muted
                                });
                                let button = egui::Button::new(label)
                                    .selected(selected)
                                    .fill(if selected {
                                        colors.card
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    })
                                    .stroke(if selected {
                                        egui::Stroke::new(1.0, colors.border)
                                    } else {
                                        egui::Stroke::NONE
                                    })
                                    .min_size(egui::vec2(68.0, 28.0));
                                if ui.add(button).clicked() {
                                    saved.theme = mode;
                                }
                            }
                        });
                    });
            });
        });
    });
    ui.add_space(16.0);
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Audio output").font(theme::heading(theme::SECTION_TITLE)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if power::refresh(ui).clicked() {
                    actions.refresh_output = true;
                }
            });
        });
        egui::ComboBox::from_id_salt("output")
            .width(ui.available_width() - 16.0)
            .truncate()
            .selected_text(saved.device.as_deref().unwrap_or("System default output"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut saved.device, None, "System default output");
                for device in status.devices {
                    ui.selectable_value(&mut saved.device, Some(device.clone()), device);
                }
            });
        if let Some(name) = status.output_name
            && saved.device.as_deref() != Some(name)
        {
            ui.add(
                egui::Label::new(egui::RichText::new(name).small().color(colors.muted)).truncate(),
            );
        }
        ui.separator();
        toggle(ui, &mut saved.settings.spatial_audio, "Spatial audio")
            .on_hover_text("Position sounds across your speakers to match the keyboard.");
        ui.add_enabled_ui(saved.settings.spatial_audio, |ui| {
            slider(ui, "Stereo width", &mut saved.settings.width)
        });
        ui.separator();
        toggle(
            ui,
            &mut saved.settings.normalize_volume,
            "Volume normalization",
        )
        .on_hover_text("Balance the volume when changing switches.");
        toggle(ui, &mut saved.settings.mute_modifiers, "Mute modifier keys")
            .on_hover_text("Silence Ctrl, Alt, Shift, Windows, Caps Lock, Esc, and function keys. Tab stays audible.");
        ui.separator();
        ui.add_enabled_ui(saved.switch.has_release_sounds(), |ui| {
            toggle(
                ui,
                &mut saved.settings.release_sounds,
                "Play key release sounds",
            );
        });
    });
    ui.add_space(20.0);
    ui.label(egui::RichText::new("Typing feel").font(theme::heading(theme::SECTION_TITLE)));
    toggle(ui, &mut saved.settings.per_key_feel, "Per-key feel");
    ui.add_enabled_ui(saved.settings.per_key_feel, |ui| {
        slider(
            ui,
            "Home row softness",
            &mut saved.settings.home_row_softness,
        )
    });
    ui.label(
        egui::RichText::new(
            "Softer home keys, stronger reaches. Softness affects the home row only.",
        )
        .small()
        .color(colors.muted),
    );
    ui.add_space(20.0);
    keyboard::show(ui, status.key_events, status.visual_reset);
    ui.add_space(20.0);
    card(ui, |ui| {
        #[cfg(windows)]
        toggle(ui, &mut saved.close_to_tray, "Keep running when closed");
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.label(
                    egui::RichText::new(if status.tray_available {
                        "Find Reeb in the Windows tray."
                    } else {
                        "Tray unavailable. Closing will quit."
                    })
                    .small()
                    .color(colors.muted),
                );
                ui.label(
                    egui::RichText::new(status.input)
                        .small()
                        .color(colors.muted),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Quit Reeb").clicked() {
                    actions.quit = true;
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(
        ctx: &egui::Context,
        page: &mut Page,
        saved: &mut Saved,
        events: Vec<egui::Event>,
    ) -> (egui::FullOutput, Actions) {
        if ctx.cumulative_pass_nr() == 0 {
            theme::apply(ctx);
        }
        let mut actions = Actions::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(840.0, 660.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                actions = show(
                    ctx,
                    page,
                    saved,
                    Status {
                        key_events: &[],
                        visual_reset: false,
                        audio_ready: true,
                        input: "Listening across Windows",
                        output_name: Some("Headphones"),
                        audio_error: None,
                        devices: &[],
                        tray_available: true,
                        tray_error: None,
                        image: None,
                    },
                );
            },
        );
        (output, actions)
    }

    fn position(output: &egui::FullOutput, text: &str) -> egui::Pos2 {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(shape) if shape.galley.text() == text => {
                    Some(shape.pos + shape.galley.rect.size() * 0.5)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("Missing control: {text}"))
    }

    fn click(position: egui::Pos2) -> Vec<egui::Event> {
        [
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]
        .to_vec()
    }

    #[test]
    fn appearance_selector_changes_the_saved_mode_and_active_palette() {
        let ctx = egui::Context::default();
        let mut page = Page::Settings;
        let mut saved = Saved::default();
        for mode in [ThemeMode::Dark, ThemeMode::Light, ThemeMode::System] {
            let (output, _) = draw(&ctx, &mut page, &mut saved, vec![]);
            draw(
                &ctx,
                &mut page,
                &mut saved,
                click(position(&output, mode.label())),
            );
            assert_eq!(saved.theme, mode);
            assert_eq!(ctx.theme() == egui::Theme::Dark, mode == ThemeMode::Dark);
        }
    }

    #[test]
    fn settings_power_toggles_and_refresh_stays_separate_from_device_name() {
        let ctx = egui::Context::default();
        let mut page = Page::Settings;
        let mut saved = Saved::default();
        draw(&ctx, &mut page, &mut saved, vec![]);
        let power = ctx.read_response(egui::Id::new("reeb-power")).unwrap().rect;
        assert_eq!(power.size(), egui::vec2(72.0, 40.0));
        draw(&ctx, &mut page, &mut saved, click(power.center()));
        assert!(!saved.enabled);
        let (output, _) = draw(&ctx, &mut page, &mut saved, vec![]);
        position(&output, "Reeb is off");
        let refresh = position(&output, "Refresh devices");
        let device = position(&output, "Headphones");
        assert!(device.y > refresh.y);
        let (_, actions) = draw(&ctx, &mut page, &mut saved, click(refresh));
        assert!(actions.refresh_output);
    }

    #[test]
    fn settings_navigation_exposes_audio_controls_and_quit() {
        let ctx = egui::Context::default();
        let mut page = Page::default();
        let mut saved = Saved::default();
        let (output, _) = draw(&ctx, &mut page, &mut saved, vec![]);
        let (output, _) = draw(
            &ctx,
            &mut page,
            &mut saved,
            click(position(&output, "Settings")),
        );
        assert!(page == Page::Settings);
        position(&output, "Audio output");
        let track = ctx
            .read_response(egui::Id::new("page-scrollbar"))
            .unwrap()
            .rect;
        draw(
            &ctx,
            &mut page,
            &mut saved,
            click(track.center_bottom() - egui::vec2(0.0, 2.0)),
        );
        let (output, _) = draw(&ctx, &mut page, &mut saved, vec![]);
        let (_, actions) = draw(
            &ctx,
            &mut page,
            &mut saved,
            click(position(&output, "Quit Reeb")),
        );
        assert!(actions.quit);
    }

    #[test]
    fn tone_pads_update_the_right_sound_and_enter_customization_unlinks() {
        let ctx = egui::Context::default();
        let mut page = Page::Tone;
        let mut saved = Saved::default();
        draw(&ctx, &mut page, &mut saved, vec![]);
        let pad = ctx
            .read_response(egui::Id::new(("tone-pad", "Keyboard tone")))
            .unwrap()
            .rect;
        let (_, actions) = draw(
            &ctx,
            &mut page,
            &mut saved,
            click(pad.right_top() + egui::vec2(-4.0, 4.0)),
        );
        assert_eq!((saved.settings.tone_x, saved.settings.tone_y), (1.0, 1.0));
        assert_eq!(actions.tone_preview, Some(ToneTarget::Keyboard));
        assert!(saved.settings.enter.use_keyboard_tone);
        page = Page::Enter;
        draw(&ctx, &mut page, &mut saved, vec![]);
        draw(&ctx, &mut page, &mut saved, vec![]);
        let pad = ctx
            .read_response(egui::Id::new(("tone-pad", "Enter tone")))
            .unwrap()
            .rect;
        let (_, actions) = draw(
            &ctx,
            &mut page,
            &mut saved,
            click(pad.left_top() + egui::vec2(4.0, 4.0)),
        );
        assert!(!saved.settings.enter.use_keyboard_tone);
        assert_eq!(
            (saved.settings.enter.tone_x, saved.settings.enter.tone_y),
            (0.0, 1.0)
        );
        assert_eq!((saved.settings.tone_x, saved.settings.tone_y), (1.0, 1.0));
        assert_eq!(actions.tone_preview, Some(ToneTarget::Enter));
    }

    #[test]
    fn switch_grid_selects_tiles_and_scroll_reaches_the_last_pack() {
        let ctx = egui::Context::default();
        let mut page = Page::Switches;
        let mut saved = Saved::default();
        draw(&ctx, &mut page, &mut saved, vec![]);
        let (output, _) = draw(&ctx, &mut page, &mut saved, vec![]);
        draw(
            &ctx,
            &mut page,
            &mut saved,
            click(position(&output, "Alpaca")),
        );
        assert_eq!(saved.switch, reeb::samples::Switch::Alpaca);
        let track = ctx
            .read_response(egui::Id::new("page-scrollbar"))
            .unwrap()
            .rect;
        draw(
            &ctx,
            &mut page,
            &mut saved,
            click(track.center_bottom() - egui::vec2(0.0, 1.0)),
        );
        let (output, _) = draw(&ctx, &mut page, &mut saved, vec![]);
        draw(
            &ctx,
            &mut page,
            &mut saved,
            click(position(&output, "Cherry KC 1000")),
        );
        assert_eq!(saved.switch, reeb::samples::Switch::CherryKc1000);
    }

    #[test]
    fn hovering_a_switch_requests_its_preview_without_selecting_it() {
        let ctx = egui::Context::default();
        let mut page = Page::Switches;
        let mut saved = Saved::default();
        draw(&ctx, &mut page, &mut saved, vec![]);
        let (output, _) = draw(&ctx, &mut page, &mut saved, vec![]);
        let selected = saved.switch;
        let point = position(&output, "Alpaca");
        let (_, actions) = draw(
            &ctx,
            &mut page,
            &mut saved,
            vec![egui::Event::PointerMoved(point)],
        );
        assert_eq!(actions.switch_hover, Some(reeb::samples::Switch::Alpaca));
        assert_eq!(saved.switch, selected);
        let (_, actions) = draw(&ctx, &mut page, &mut saved, vec![egui::Event::PointerGone]);
        assert_eq!(actions.switch_hover, None);
    }

    #[test]
    fn tone_changes_preview_only_while_enabled() {
        let ctx = egui::Context::default();
        let mut page = Page::Tone;
        let mut saved = Saved {
            enabled: false,
            ..Default::default()
        };
        draw(&ctx, &mut page, &mut saved, vec![]);
        let pad = ctx
            .read_response(egui::Id::new(("tone-pad", "Keyboard tone")))
            .unwrap()
            .rect;
        let (_, actions) = draw(
            &ctx,
            &mut page,
            &mut saved,
            click(pad.right_top() + egui::vec2(-4.0, 4.0)),
        );
        assert_eq!(saved.settings.tone_x, 1.0);
        assert_eq!(actions.tone_preview, None);
        saved.enabled = true;
        draw(&ctx, &mut page, &mut saved, vec![]);
        let (_, actions) = draw(
            &ctx,
            &mut page,
            &mut saved,
            click(pad.left_bottom() + egui::vec2(4.0, -4.0)),
        );
        assert_eq!(actions.tone_preview, Some(ToneTarget::Keyboard));
    }
}
