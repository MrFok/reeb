use eframe::egui;
use std::sync::OnceLock;

pub fn data() -> egui::IconData {
    static ICON: OnceLock<egui::IconData> = OnceLock::new();
    ICON.get_or_init(|| {
        eframe::icon_data::from_png_bytes(include_bytes!("../assets/brand/reeb-icon-256.png"))
            .expect("Bundled app icon is invalid")
    })
    .clone()
}

#[cfg(windows)]
pub fn tray() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/ui/reeb-tray.png"))
        .expect("Bundled tray icon is invalid")
}

pub fn image(ctx: &egui::Context) -> egui::TextureHandle {
    let icon = data();
    ctx.load_texture(
        "reeb-icon",
        egui::ColorImage::from_rgba_unmultiplied(
            [icon.width as usize, icon.height as usize],
            &icon.rgba,
        ),
        egui::TextureOptions::LINEAR,
    )
}
