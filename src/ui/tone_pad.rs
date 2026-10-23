use crate::{motion, theme};
use eframe::egui;
use reeb::mixer::pitch_rate;

pub fn show(ui: &mut egui::Ui, label: &str, x: &mut f32, y: &mut f32) -> egui::Response {
    let colors = theme::palette(ui.ctx());
    let size = ui.available_width().min(200.0);
    let (labels, _) = ui.allocate_exact_size(egui::vec2(size, 16.0), egui::Sense::hover());
    ui.painter().text(
        labels.left_center(),
        egui::Align2::LEFT_CENTER,
        "Warm · High",
        egui::FontId::proportional(theme::CAPTION),
        colors.muted,
    );
    ui.painter().text(
        labels.right_center(),
        egui::Align2::RIGHT_CENTER,
        "Bright · High",
        egui::FontId::proportional(theme::CAPTION),
        colors.muted,
    );
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let mut response = ui.interact(
        rect,
        egui::Id::new(("tone-pad", label)),
        egui::Sense::click_and_drag(),
    );
    let inner = rect.shrink(14.0);
    let before = (*x, *y);
    if (response.clicked() || response.dragged() || response.is_pointer_button_down_on())
        && let Some(pointer) = response.interact_pointer_pos()
    {
        *x = ((pointer.x - inner.left()) / inner.width()).clamp(0.0, 1.0);
        *y = (1.0 - (pointer.y - inner.top()) / inner.height()).clamp(0.0, 1.0);
        response.request_focus();
    }
    if ui.is_enabled() && response.has_focus() {
        ui.input(|input| {
            let step = if input.modifiers.shift {
                0.01
            } else {
                1.0 / 12.0
            };
            if input.key_pressed(egui::Key::ArrowRight) {
                *x += step;
            }
            if input.key_pressed(egui::Key::ArrowLeft) {
                *x -= step;
            }
            if input.key_pressed(egui::Key::ArrowUp) {
                *y += step;
            }
            if input.key_pressed(egui::Key::ArrowDown) {
                *y -= step;
            }
        });
    }
    *x = x.clamp(0.0, 1.0);
    *y = y.clamp(0.0, 1.0);
    if before != (*x, *y) {
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Slider,
            ui.is_enabled(),
            format!(
                "{label}: brightness {:.0}%, pitch {:.2}×",
                *x * 100.0,
                pitch_rate(*y)
            ),
        )
    });
    let painter = ui.painter();
    painter.rect_filled(rect, 12, colors.surface);
    painter.rect_stroke(
        rect,
        12,
        egui::Stroke::new(
            1.0,
            if response.has_focus() {
                colors.accent
            } else {
                colors.border
            },
        ),
        egui::StrokeKind::Inside,
    );
    for row in 0..13 {
        for col in 0..13 {
            let point = egui::pos2(
                inner.left() + inner.width() * col as f32 / 12.0,
                inner.top() + inner.height() * row as f32 / 12.0,
            );
            if row == 6 && col == 6 {
                painter.circle_stroke(point, 4.0, egui::Stroke::new(1.0, colors.muted));
            } else {
                painter.circle_filled(point, 1.5, colors.border);
            }
        }
    }
    let direct = response.clicked() || response.dragged() || response.is_pointer_button_down_on();
    let shown_x = motion::spring(ui.ctx(), response.id.with("x"), *x, direct).clamp(0.0, 1.0);
    let shown_y = motion::spring(ui.ctx(), response.id.with("y"), *y, direct).clamp(0.0, 1.0);
    let point = egui::pos2(
        inner.left() + shown_x * inner.width(),
        inner.bottom() - shown_y * inner.height(),
    );
    let halo = motion::spring(
        ui.ctx(),
        response.id.with("halo"),
        if response.hovered() || direct {
            14.0
        } else {
            11.0
        },
        false,
    );
    painter.circle_filled(point, halo, colors.accent.gamma_multiply(0.12));
    painter.circle_filled(point, 8.0, colors.accent);
    painter.circle_stroke(point, 8.0, egui::Stroke::new(2.0, egui::Color32::WHITE));
    let (labels, _) = ui.allocate_exact_size(egui::vec2(size, 18.0), egui::Sense::hover());
    for (position, align, text) in [
        (
            labels.left_center(),
            egui::Align2::LEFT_CENTER,
            "Warm · Low",
        ),
        (
            labels.right_center(),
            egui::Align2::RIGHT_CENTER,
            "Bright · Low",
        ),
    ] {
        ui.painter().text(
            position,
            align,
            text,
            egui::FontId::proportional(theme::CAPTION),
            colors.muted,
        );
    }
    ui.label(
        egui::RichText::new(format!("Pitch {:.2}×", pitch_rate(*y)))
            .small()
            .color(colors.muted),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_position_and_arrow_keys_adjust_both_axes_with_clamping() {
        let ctx = egui::Context::default();
        let (mut x, mut y) = (0.5, 0.5);
        let mut rect = egui::Rect::NOTHING;
        let mut draw = |events: Vec<egui::Event>| {
            let mut changed = false;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400.0, 500.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let response = show(ui, "Test tone", &mut x, &mut y);
                        rect = response.rect;
                        changed = response.changed();
                    });
                },
            );
            (rect, x, y, changed)
        };
        let (rect, _, _, _) = draw(vec![]);
        let point = rect.left_top();
        let (_, x, y, changed) = draw(vec![
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
        ]);
        assert!(changed);
        assert_eq!((x, y), (0.0, 1.0));
        let (_, x, y, changed) = draw(vec![egui::Event::Key {
            key: egui::Key::ArrowRight,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        assert!(changed);
        assert!(x > 0.0 && x < 0.1);
        assert_eq!(y, 1.0);
    }
}
