use crate::{motion, theme};
use eframe::egui::{self, pos2, vec2};

pub fn show(ctx: &egui::Context) {
    let colors = theme::palette(ctx);
    egui::TopBottomPanel::top("window-chrome")
        .exact_height(36.0)
        .frame(egui::Frame::new().fill(colors.background))
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            let button_size = vec2(40.0, 24.0);
            let inset = 6.0;
            let gap = 4.0;
            let controls_left = rect.right() - inset - 3.0 * button_size.x - 2.0 * gap;
            let drag = egui::Rect::from_min_max(
                rect.min + vec2(6.0, 3.0),
                pos2(controls_left - 4.0, rect.bottom() - 3.0),
            );
            let response = ui.interact(drag, ui.id().with("drag"), egui::Sense::click_and_drag());
            let maximized = ctx.input(|input| input.viewport().maximized.unwrap_or(false));
            if response.double_clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            } else if response.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            for index in 0..3 {
                let button = egui::Rect::from_min_size(
                    pos2(
                        controls_left + index as f32 * (button_size.x + gap),
                        rect.top() + inset,
                    ),
                    button_size,
                );
                let response = ui.interact(button, ui.id().with(index), egui::Sense::click());
                let label = ["Minimize", "Maximize or restore", "Close"][index];
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label)
                });
                let hover = motion::hover(ctx, response.id.with("hover"), response.hovered());
                let color = if index == 2 {
                    colors.muted.lerp_to_gamma(egui::Color32::WHITE, hover)
                } else {
                    colors.muted
                };
                let fill = if index == 2 {
                    egui::Color32::from_rgb(210, 51, 66)
                } else {
                    colors.surface
                };
                ui.painter()
                    .rect_filled(button, 6, colors.background.lerp_to_gamma(fill, hover));
                let center = button.center();
                let stroke = egui::Stroke::new(1.2, color);
                match index {
                    0 => {
                        ui.painter().line_segment(
                            [center - vec2(5.0, 0.0), center + vec2(5.0, 0.0)],
                            stroke,
                        );
                    }
                    1 => {
                        let icon = egui::Rect::from_center_size(center, vec2(9.0, 9.0));
                        if maximized {
                            ui.painter().rect_stroke(
                                icon.translate(vec2(2.0, -2.0)),
                                1,
                                stroke,
                                egui::StrokeKind::Inside,
                            );
                            ui.painter().rect_filled(
                                icon,
                                1,
                                if response.hovered() {
                                    colors.surface
                                } else {
                                    colors.background
                                },
                            );
                        }
                        ui.painter()
                            .rect_stroke(icon, 1, stroke, egui::StrokeKind::Inside);
                    }
                    _ => {
                        ui.painter().line_segment(
                            [center - vec2(4.0, 4.0), center + vec2(4.0, 4.0)],
                            stroke,
                        );
                        ui.painter().line_segment(
                            [center + vec2(-4.0, 4.0), center + vec2(4.0, -4.0)],
                            stroke,
                        );
                    }
                }
                if response.clicked() {
                    ctx.send_viewport_cmd(match index {
                        0 => egui::ViewportCommand::Minimized(true),
                        1 => egui::ViewportCommand::Maximized(!maximized),
                        _ => egui::ViewportCommand::Close,
                    });
                }
            }
        });
    resize(ctx);
}

fn resize(ctx: &egui::Context) {
    use egui::{CursorIcon, ResizeDirection};
    if ctx.input(|input| input.viewport().maximized.unwrap_or(false)) {
        return;
    }
    let Some(pointer) = ctx.pointer_hover_pos() else {
        return;
    };
    let rect = ctx.content_rect();
    let left = pointer.x < rect.left() + 5.0;
    let right = pointer.x > rect.right() - 5.0;
    let top = pointer.y < rect.top() + 5.0;
    let bottom = pointer.y > rect.bottom() - 5.0;
    let (direction, cursor) = match (left, right, top, bottom) {
        (true, _, true, _) => (ResizeDirection::NorthWest, CursorIcon::ResizeNwSe),
        (_, true, true, _) => (ResizeDirection::NorthEast, CursorIcon::ResizeNeSw),
        (true, _, _, true) => (ResizeDirection::SouthWest, CursorIcon::ResizeNeSw),
        (_, true, _, true) => (ResizeDirection::SouthEast, CursorIcon::ResizeNwSe),
        (true, _, _, _) => (ResizeDirection::West, CursorIcon::ResizeHorizontal),
        (_, true, _, _) => (ResizeDirection::East, CursorIcon::ResizeHorizontal),
        (_, _, true, _) => (ResizeDirection::North, CursorIcon::ResizeVertical),
        (_, _, _, true) => (ResizeDirection::South, CursorIcon::ResizeVertical),
        _ => return,
    };
    ctx.set_cursor_icon(cursor);
    if ctx.input(|input| input.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ctx: &egui::Context, pointer: egui::Pos2) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(840.0, 660.0),
                )),
                events: vec![
                    egui::Event::PointerMoved(pointer),
                    egui::Event::PointerButton {
                        pos: pointer,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                    egui::Event::PointerButton {
                        pos: pointer,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            show,
        )
    }

    #[test]
    fn integrated_caption_controls_send_native_window_commands() {
        for (x, command) in [
            (726.0, egui::ViewportCommand::Minimized(true)),
            (768.0, egui::ViewportCommand::Maximized(true)),
            (810.0, egui::ViewportCommand::Close),
        ] {
            let ctx = egui::Context::default();
            let _ = frame(&ctx, pos2(400.0, 300.0));
            let output = frame(&ctx, pos2(x, 18.0));
            let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
            assert!(
                commands.contains(&command),
                "Missing {command:?}: {commands:?}"
            );
            assert!(!commands.contains(&egui::ViewportCommand::StartDrag));
        }
    }

    #[test]
    fn undecorated_edges_keep_native_resize_and_caption_drag() {
        let ctx = egui::Context::default();
        let _ = frame(&ctx, pos2(400.0, 300.0));
        let output = frame(&ctx, pos2(1.0, 300.0));
        assert!(
            output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::BeginResize(
                    egui::ResizeDirection::West
                ))
        );
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(840.0, 660.0),
            )),
            ..Default::default()
        };
        input.events = vec![
            egui::Event::PointerMoved(pos2(400.0, 18.0)),
            egui::Event::PointerButton {
                pos: pos2(400.0, 18.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ];
        let _ = ctx.run(input.clone(), show);
        input.events = vec![egui::Event::PointerMoved(pos2(420.0, 18.0))];
        let output = ctx.run(input, show);
        assert!(
            output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::StartDrag)
        );
    }
}
