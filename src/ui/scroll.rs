use crate::theme;
use eframe::egui;

pub fn show(ui: &mut egui::Ui, salt: impl std::hash::Hash, content: impl FnOnce(&mut egui::Ui)) {
    let available = ui.available_rect_before_wrap();
    let width = (available.width() - 14.0).max(1.0);
    let output = egui::ScrollArea::vertical()
        .id_salt(salt)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .max_width(width)
        .auto_shrink([false, false])
        .show(ui, content);
    let viewport = output.inner_rect.height();
    let overflow = (output.content_size.y - viewport).max(0.0);
    if overflow <= 0.0 || viewport <= 32.0 {
        return;
    }
    let colors = theme::palette(ui.ctx());
    let track = egui::Rect::from_min_max(
        egui::pos2(available.right() - 7.0, available.top()),
        egui::pos2(available.right(), available.top() + viewport),
    );
    let thumb_height = (viewport * viewport / output.content_size.y).clamp(32.0, viewport);
    let travel = viewport - thumb_height;
    let thumb = egui::Rect::from_min_size(
        track.min + egui::vec2(0.0, travel * output.state.offset.y / overflow),
        egui::vec2(7.0, thumb_height),
    );
    let response = ui.interact(
        track.expand2(egui::vec2(3.0, 0.0)),
        egui::Id::new("page-scrollbar"),
        egui::Sense::click_and_drag(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Slider, ui.is_enabled(), "Scroll page")
    });
    let drag_id = output.id.with("scroll-grab");
    if response.is_pointer_button_down_on()
        && ui.input(|input| input.pointer.any_pressed())
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let grab = if thumb.contains(pointer) {
            pointer.y - thumb.top()
        } else {
            thumb_height * 0.5
        };
        ui.ctx().data_mut(|data| data.insert_temp(drag_id, grab));
    }
    if (response.clicked() || response.dragged())
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let grab = ui
            .ctx()
            .data(|data| data.get_temp::<f32>(drag_id))
            .unwrap_or(thumb_height * 0.5);
        let mut state = output.state;
        state.offset.y = ((pointer.y - track.top() - grab) / travel).clamp(0.0, 1.0) * overflow;
        state.store(ui.ctx(), output.id);
        ui.ctx().request_repaint();
    }
    ui.painter().rect_filled(track, 4, colors.surface);
    ui.painter().rect_filled(
        thumb,
        4,
        if response.hovered() || response.dragged() {
            colors.muted
        } else {
            colors.border
        },
    );
}
