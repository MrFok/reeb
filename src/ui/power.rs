use crate::{motion, theme};
use eframe::egui;
use reeb::keys::KeyEvent;

const FLASH_SECONDS: f64 = 0.3;
const PRESS_SECONDS: f64 = 0.14;
const STATE_ID: &str = "last-key-preview";

#[derive(Clone, Copy, Default)]
struct LastKey {
    key: Option<(&'static str, f64)>,
}

pub fn clear(ctx: &egui::Context) {
    ctx.data_mut(|data| data.remove::<LastKey>(egui::Id::new(STATE_ID)));
}

pub fn show(
    ui: &mut egui::Ui,
    enabled: &mut bool,
    events: &[KeyEvent],
    reset: bool,
    image: Option<&egui::TextureHandle>,
) {
    let colors = theme::palette(ui.ctx());
    let now = ui.input(|input| input.time);
    let id = egui::Id::new(STATE_ID);
    let mut state = if reset {
        LastKey::default()
    } else {
        ui.ctx()
            .data(|data| data.get_temp::<LastKey>(id))
            .unwrap_or_default()
    };
    for event in events {
        if event.pressed
            && let Some(label) = super::keyboard::key_label(event.scan)
        {
            state.key = Some((label, now));
        }
    }
    ui.ctx().data_mut(|data| data.insert_temp(id, state));
    ui.vertical_centered(|ui| {
        let (area, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 62.0), egui::Sense::hover());
        let notch = egui::Rect::from_center_size(area.center(), egui::vec2(220.0, 42.0));
        let painter = ui.painter();
        painter.rect_filled(notch, 12, egui::Color32::from_rgb(16, 17, 19));
        if let Some(image) = image {
            painter.image(
                image.id(),
                egui::Rect::from_center_size(
                    notch.left_center() + egui::vec2(24.0, 0.0),
                    egui::vec2(24.0, 24.0),
                ),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        painter.text(
            notch.center() - egui::vec2(10.0, 0.0),
            egui::Align2::CENTER_CENTER,
            "reeb",
            theme::heading(14.0),
            egui::Color32::from_gray(235),
        );
        let age = state.key.map_or(f64::INFINITY, |(_, time)| now - time);
        let animate = motion::allowed(ui.ctx());
        let flash = if animate {
            1.0 - (age / FLASH_SECONDS).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };
        let scale = if animate {
            1.0 - 0.06 * (std::f32::consts::PI * (age / PRESS_SECONDS).clamp(0.0, 1.0) as f32).sin()
        } else {
            1.0
        };
        let chip = egui::Rect::from_center_size(
            notch.right_center() - egui::vec2(28.0, 0.0),
            egui::vec2(40.0, 26.0) * scale,
        );
        painter.rect_filled(
            chip,
            5,
            egui::Color32::from_gray(48).lerp_to_gamma(egui::Color32::from_gray(128), flash),
        );
        painter.text(
            chip.center(),
            egui::Align2::CENTER_CENTER,
            state.key.map_or("—", |(label, _)| label),
            theme::heading(10.0),
            egui::Color32::from_gray(235),
        );
        if animate && age < FLASH_SECONDS {
            ui.ctx().request_repaint();
        }
        ui.add_space(8.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(72.0, 40.0), egui::Sense::hover());
        let mut response = ui.interact(rect, egui::Id::new("reeb-power"), egui::Sense::click());
        if response.clicked() {
            *enabled = !*enabled;
            response.mark_changed();
        }
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Checkbox,
                ui.is_enabled(),
                *enabled,
                "Enable Reeb",
            )
        });
        let t = motion::spring(
            ui.ctx(),
            response.id.with("power-thumb"),
            if *enabled { 1.0 } else { 0.0 },
            false,
        )
        .clamp(0.0, 1.0);
        ui.painter()
            .rect_filled(rect, 20, colors.border.lerp_to_gamma(theme::BLUE, t));
        ui.painter().circle_filled(
            egui::pos2(rect.left() + 20.0 + 32.0 * t, rect.center().y),
            15.0,
            egui::Color32::WHITE,
        );
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect.expand(3.0),
                23,
                egui::Stroke::new(1.0, colors.accent),
                egui::StrokeKind::Outside,
            );
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(if *enabled {
                "Reeb is on"
            } else {
                "Reeb is off"
            })
            .font(theme::heading(16.0)),
        );
        ui.label(
            egui::RichText::new(if cfg!(target_os = "macos") {
                "Type to see your keys."
            } else {
                "Type anywhere to see your keys."
            })
            .small()
            .color(colors.muted),
        );
    });
    ui.add_space(24.0);
}

pub fn refresh(ui: &mut egui::Ui) -> egui::Response {
    ui.push_id("audio-refresh", |ui| {
        ui.add(egui::Button::new("Refresh devices").min_size(egui::vec2(116.0, 30.0)))
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notch_keeps_only_the_last_key_and_ignores_releases() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut enabled = false;
        for (time, events, expected) in [
            (
                0.0,
                vec![KeyEvent {
                    scan: 0x1e,
                    pressed: true,
                }],
                "A",
            ),
            (
                0.1,
                vec![
                    KeyEvent {
                        scan: 0x1f,
                        pressed: true,
                    },
                    KeyEvent {
                        scan: 0x1f,
                        pressed: false,
                    },
                ],
                "S",
            ),
        ] {
            let _ = ctx.run(
                egui::RawInput {
                    time: Some(time),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| show(ui, &mut enabled, &events, false, None));
                },
            );
            assert_eq!(
                ctx.data(|data| data.get_temp::<LastKey>(egui::Id::new(STATE_ID)))
                    .unwrap()
                    .key
                    .unwrap()
                    .0,
                expected
            );
            assert!(!enabled);
        }
        clear(&ctx);
        assert!(
            ctx.data(|data| data.get_temp::<LastKey>(egui::Id::new(STATE_ID)))
                .is_none()
        );
    }
}
