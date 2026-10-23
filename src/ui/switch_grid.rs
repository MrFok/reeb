use crate::{motion, theme};
use eframe::egui;
use reeb::samples::Switch;

const IMAGES: [&[u8]; Switch::ALL.len()] = [
    include_bytes!("../../assets/brand/switches/generated/cream.png"),
    include_bytes!("../../assets/brand/switches/generated/alpaca.png"),
    include_bytes!("../../assets/brand/switches/generated/blackink.png"),
    include_bytes!("../../assets/brand/switches/generated/redink.png"),
    include_bytes!("../../assets/brand/switches/generated/holypanda.png"),
    include_bytes!("../../assets/brand/switches/generated/mxblack.png"),
    include_bytes!("../../assets/brand/switches/generated/mxblue.png"),
    include_bytes!("../../assets/brand/switches/generated/mxbrown.png"),
    include_bytes!("../../assets/brand/switches/generated/boxnavy.png"),
    include_bytes!("../../assets/brand/switches/generated/buckling.png"),
    include_bytes!("../../assets/brand/switches/generated/bluealps.png"),
    include_bytes!("../../assets/brand/switches/generated/topre.png"),
    include_bytes!("../../assets/brand/switches/generated/turquoise.png"),
    include_bytes!("../../assets/brand/switches/generated/foxfire-mxbrown.png"),
    include_bytes!("../../assets/brand/switches/generated/kc1000.png"),
    include_bytes!("../../assets/brand/switches/generated/macbook-air-m2.png"),
];

fn texture(ctx: &egui::Context, switch: Switch) -> egui::TextureHandle {
    let index = Switch::ALL
        .iter()
        .position(|value| *value == switch)
        .expect("Bundled switch");
    let id = egui::Id::new(("switch-image", index));
    ctx.data(|data| data.get_temp::<egui::TextureHandle>(id))
        .unwrap_or_else(|| {
            let image = eframe::icon_data::from_png_bytes(IMAGES[index])
                .expect("Bundled switch illustration");
            let texture = ctx.load_texture(
                format!("switch-{index}"),
                egui::ColorImage::from_rgba_unmultiplied(
                    [image.width as usize, image.height as usize],
                    &image.rgba,
                ),
                Default::default(),
            );
            ctx.data_mut(|data| data.insert_temp(id, texture.clone()));
            texture
        })
}

pub fn image(ui: &mut egui::Ui, switch: Switch, size: egui::Vec2) {
    let texture = texture(ui.ctx(), switch);
    ui.add(egui::Image::new((texture.id(), size)).alt_text(switch.name()));
}

pub fn show(ui: &mut egui::Ui, selected: &mut Switch) -> Option<Switch> {
    let mut hovered = None;
    let colors = theme::palette(ui.ctx());
    let columns = ((ui.available_width() + 8.0) / 260.0)
        .floor()
        .clamp(2.0, 3.0) as usize;
    let width = (ui.available_width() - (columns - 1) as f32 * 8.0) / columns as f32;
    egui::Grid::new("switch-grid")
        .spacing([8.0, 8.0])
        .show(ui, |ui| {
            for (index, switch) in Switch::ALL.into_iter().enumerate() {
                let texture = texture(ui.ctx(), switch);
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(width, 64.0), egui::Sense::click());
                if response.hovered() && ui.input(|input| input.pointer.hover_pos().is_some()) {
                    hovered = Some(switch);
                }
                let active = *selected == switch;
                let hover = motion::hover(ui.ctx(), response.id.with("hover"), response.hovered());
                let fill = if active {
                    colors
                        .card
                        .lerp_to_gamma(colors.accent, 0.06 + 0.02 * hover)
                } else {
                    colors.card.lerp_to_gamma(colors.hover, hover)
                };
                ui.painter().rect_filled(rect, 8, fill);
                ui.painter().rect_stroke(
                    rect,
                    8,
                    egui::Stroke::new(
                        if active { 1.5 } else { 1.0 },
                        if active || response.has_focus() {
                            colors.accent
                        } else {
                            colors.border
                        },
                    ),
                    egui::StrokeKind::Inside,
                );
                let image_rect = egui::Rect::from_center_size(
                    rect.left_center() + egui::vec2(32.0, 0.0),
                    egui::vec2(44.0, 44.0),
                );
                egui::Image::new((texture.id(), image_rect.size())).paint_at(ui, image_rect);
                let name_x = rect.left() + 66.0;
                let galley = ui.painter().layout(
                    switch.name().to_owned(),
                    if active {
                        theme::heading(12.0)
                    } else {
                        egui::FontId::proportional(12.0)
                    },
                    colors.ink,
                    width - 96.0,
                );
                let name_y = rect.center().y - galley.size().y * 0.5;
                ui.painter()
                    .galley(egui::pos2(name_x, name_y), galley, colors.ink);
                let check = rect.right_center() - egui::vec2(17.0, 0.0);
                if active {
                    ui.painter().circle_filled(check, 7.0, theme::BLUE);
                    ui.painter().add(egui::Shape::line(
                        vec![
                            check + egui::vec2(-3.0, 0.0),
                            check + egui::vec2(-1.0, 2.0),
                            check + egui::vec2(3.0, -2.0),
                        ],
                        egui::Stroke::new(1.3, egui::Color32::WHITE),
                    ));
                } else {
                    ui.painter()
                        .circle_stroke(check, 7.0, egui::Stroke::new(1.0, colors.border));
                }
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::RadioButton,
                        ui.is_enabled(),
                        active,
                        switch.name(),
                    )
                });
                if response.clicked()
                    || (response.has_focus()
                        && ui.input(|i| {
                            i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)
                        }))
                {
                    *selected = switch;
                    ui.ctx().request_repaint();
                }
                response.on_hover_cursor(egui::CursorIcon::PointingHand);
                if (index + 1) % columns == 0 {
                    ui.end_row();
                }
            }
        });
    hovered
}
