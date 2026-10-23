use crate::{motion, theme};
use eframe::egui;
use reeb::keys::KeyEvent;
use std::collections::{HashSet, VecDeque};

const STATE_ID: &str = "keyboard-visualizer";
const MAX_WAVES: usize = 32;
const WAVE_SECONDS: f64 = 0.650;
const WAVE_SPEED: f32 = 13.0;
const ROWS: [&[(&str, u16, f32)]; 5] = [
    &[
        ("Esc", 1, 1.0),
        ("1", 2, 1.0),
        ("2", 3, 1.0),
        ("3", 4, 1.0),
        ("4", 5, 1.0),
        ("5", 6, 1.0),
        ("6", 7, 1.0),
        ("7", 8, 1.0),
        ("8", 9, 1.0),
        ("9", 10, 1.0),
        ("0", 11, 1.0),
        ("-", 12, 1.0),
        ("=", 13, 1.0),
        ("Back", 14, 2.0),
    ],
    &[
        ("Tab", 15, 1.5),
        ("Q", 16, 1.0),
        ("W", 17, 1.0),
        ("E", 18, 1.0),
        ("R", 19, 1.0),
        ("T", 20, 1.0),
        ("Y", 21, 1.0),
        ("U", 22, 1.0),
        ("I", 23, 1.0),
        ("O", 24, 1.0),
        ("P", 25, 1.0),
        ("[", 26, 1.0),
        ("]", 27, 1.0),
        ("\u{5c}", 43, 1.5),
    ],
    &[
        ("Caps", 58, 1.75),
        ("A", 30, 1.0),
        ("S", 31, 1.0),
        ("D", 32, 1.0),
        ("F", 33, 1.0),
        ("G", 34, 1.0),
        ("H", 35, 1.0),
        ("J", 36, 1.0),
        ("K", 37, 1.0),
        ("L", 38, 1.0),
        (";", 39, 1.0),
        ("'", 40, 1.0),
        ("Enter", 28, 2.25),
    ],
    &[
        ("Shift", 42, 2.25),
        ("Z", 44, 1.0),
        ("X", 45, 1.0),
        ("C", 46, 1.0),
        ("V", 47, 1.0),
        ("B", 48, 1.0),
        ("N", 49, 1.0),
        ("M", 50, 1.0),
        (",", 51, 1.0),
        (".", 52, 1.0),
        ("/", 53, 1.0),
        ("Shift", 54, 2.75),
    ],
    &[
        ("Ctrl", 29, 1.25),
        ("Win", 347, 1.25),
        ("Alt", 56, 1.25),
        ("", 57, 6.25),
        ("Alt", 312, 1.25),
        ("Win", 348, 1.25),
        ("Menu", 349, 1.25),
        ("Ctrl", 285, 1.25),
    ],
];

pub fn key_label(scan: u16) -> Option<&'static str> {
    #[cfg(target_os = "macos")]
    match scan {
        0x15b | 0x15c => return Some("Cmd"),
        0x38 | 0x138 => return Some("Opt"),
        0x1c => return Some("Return"),
        0x0e => return Some("Delete"),
        _ => {}
    }
    match scan {
        0x29 => Some("`"),
        0x45 => Some("Num"),
        0x135 => Some("/"),
        0x37 => Some("*"),
        0x4a => Some("-"),
        0x4e => Some("+"),
        0x47..=0x49 => Some(["7", "8", "9"][(scan - 0x47) as usize]),
        0x4b..=0x4d => Some(["4", "5", "6"][(scan - 0x4b) as usize]),
        0x4f..=0x51 => Some(["1", "2", "3"][(scan - 0x4f) as usize]),
        0x52 => Some("0"),
        0x53 => Some("."),
        0x39 => Some("Space"),
        0x11c => Some("Enter"),
        0x147 => Some("Home"),
        0x148 => Some("Up"),
        0x149 => Some("PgUp"),
        0x14b => Some("Left"),
        0x14d => Some("Right"),
        0x14f => Some("End"),
        0x150 => Some("Down"),
        0x151 => Some("PgDn"),
        0x152 => Some("Ins"),
        0x153 => Some("Del"),
        0x3b..=0x44 => Some(
            ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10"][(scan - 0x3b) as usize],
        ),
        0x57 => Some("F11"),
        0x58 => Some("F12"),
        _ => ROWS
            .iter()
            .flat_map(|row| row.iter())
            .find(|key| key.1 == scan)
            .map(|key| key.0),
    }
}

#[derive(Clone)]
struct Wave {
    origin: egui::Pos2,
    started: f64,
}

impl Wave {
    fn intensity(&self, point: egui::Pos2, now: f64) -> f32 {
        let age = now - self.started;
        if !(0.0..WAVE_SECONDS).contains(&age) {
            return 0.0;
        }
        let radius = age as f32 * WAVE_SPEED;
        if (self.origin.distance(point) - radius).abs() <= 0.6 {
            1.0 - (age / WAVE_SECONDS) as f32
        } else {
            0.0
        }
    }
}

#[derive(Clone, Default)]
struct State {
    held: HashSet<u16>,
    waves: VecDeque<Wave>,
}

fn layout() -> (Vec<(u16, &'static str, egui::Rect)>, egui::Vec2) {
    let mut keys = Vec::with_capacity(61);
    for (row_index, row) in ROWS.iter().enumerate() {
        let mut x = 0.25;
        for &(label, scan, width) in *row {
            let cell = egui::Rect::from_min_size(
                egui::pos2(x, 0.25 + row_index as f32),
                egui::vec2(width, 1.0),
            );
            keys.push((scan, label, cell.shrink(0.04)));
            x += width;
        }
    }
    (keys, egui::vec2(15.5, 5.5))
}

pub fn clear(ctx: &egui::Context) {
    ctx.data_mut(|data| data.remove::<State>(egui::Id::new(STATE_ID)));
}

pub fn show(ui: &mut egui::Ui, events: &[KeyEvent], reset: bool) {
    let colors = theme::palette(ui.ctx());
    let now = ui.input(|input| input.time);
    let animate = motion::allowed(ui.ctx());
    let id = egui::Id::new(STATE_ID);
    let mut state = ui
        .ctx()
        .data_mut(|data| data.remove_temp::<State>(id))
        .unwrap_or_default();
    if reset {
        state = State::default();
    }
    state
        .waves
        .retain(|wave| animate && now - wave.started < WAVE_SECONDS);
    let (keys, size) = layout();
    for event in events {
        let Some(key) = keys.iter().find(|key| key.0 == event.scan) else {
            continue;
        };
        if event.pressed {
            state.held.insert(event.scan);
            if animate {
                if state.waves.len() == MAX_WAVES {
                    state.waves.pop_front();
                }
                state.waves.push_back(Wave {
                    origin: key.2.center(),
                    started: now,
                });
            }
        } else {
            state.held.remove(&event.scan);
        }
    }
    let scale = ui.available_width() / size.x;
    let (rect, _) = ui.allocate_exact_size(size * scale, egui::Sense::hover());
    ui.painter().rect_filled(rect, 8, colors.background);
    for (scan, label, key) in keys {
        let wave = state
            .waves
            .iter()
            .map(|wave| wave.intensity(key.center(), now))
            .fold(0.0, f32::max);
        let strength = if state.held.contains(&scan) {
            wave.max(0.5)
        } else {
            wave
        };
        let key_rect = egui::Rect::from_min_max(
            rect.min + key.min.to_vec2() * scale,
            rect.min + key.max.to_vec2() * scale,
        );
        ui.painter().rect_filled(
            key_rect,
            3,
            colors.card.lerp_to_gamma(colors.accent, strength),
        );
        ui.painter().rect_stroke(
            key_rect,
            3,
            egui::Stroke::new(1.0, colors.border),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            key_rect.center(),
            egui::Align2::CENTER_CENTER,
            key_label(scan).unwrap_or(label),
            egui::FontId::proportional((0.35 * scale).clamp(8.0, 11.0)),
            colors.ink,
        );
    }
    if !state.waves.is_empty() {
        ui.ctx().request_repaint();
    }
    ui.ctx().data_mut(|data| data.insert_temp(id, state));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(ctx: &egui::Context, time: f64, events: &[KeyEvent], reset: bool) -> State {
        let _ = ctx.run(
            egui::RawInput {
                time: Some(time),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| show(ui, events, reset));
            },
        );
        ctx.data(|data| data.get_temp::<State>(egui::Id::new(STATE_ID)))
            .unwrap()
    }

    #[test]
    fn ansi_layout_has_61_unique_scans_and_fifteen_unit_rows() {
        let (keys, _) = layout();
        assert_eq!(keys.len(), 61);
        assert_eq!(
            keys.iter().map(|key| key.0).collect::<HashSet<_>>().len(),
            61
        );
        for row in ROWS {
            assert_eq!(row.iter().map(|key| key.2).sum::<f32>(), 15.0);
        }
        for scan in [
            14, 15, 43, 58, 28, 42, 54, 57, 29, 347, 56, 312, 348, 349, 285,
        ] {
            let expected = match scan {
                14 => 2.0,
                15 | 43 => 1.5,
                58 => 1.75,
                28 | 42 => 2.25,
                54 => 2.75,
                57 => 6.25,
                _ => 1.25,
            };
            assert_eq!(
                ROWS.iter()
                    .flat_map(|row| row.iter())
                    .find(|key| key.1 == scan)
                    .unwrap()
                    .2,
                expected
            );
        }
        let mut offset = 0;
        for row in ROWS {
            let placed = &keys[offset..offset + row.len()];
            assert!((placed.first().unwrap().2.left() - 0.29).abs() < 0.0001);
            assert!((placed.last().unwrap().2.right() - 15.21).abs() < 0.0001);
            offset += row.len();
        }
        assert_eq!(key_label(57), Some("Space"));
        assert_eq!(key_label(0x11c), Some("Enter"));
    }

    #[test]
    fn radial_ring_travels_thirteen_units_per_second_and_fades_linearly() {
        let wave = Wave {
            origin: egui::Pos2::ZERO,
            started: 0.0,
        };
        assert_eq!(wave.intensity(egui::Pos2::ZERO, 0.0), 1.0);
        assert!((wave.intensity(egui::pos2(4.225, 0.0), 0.325) - 0.5).abs() < 0.0001);
        assert_eq!(wave.intensity(egui::Pos2::ZERO, 0.325), 0.0);
        assert_eq!(wave.intensity(egui::pos2(8.45, 0.0), 0.650), 0.0);
    }

    #[test]
    fn waves_are_bounded_expire_and_preserve_held_keys_until_release() {
        let ctx = egui::Context::default();
        let events = vec![
            KeyEvent {
                scan: 30,
                pressed: true
            };
            512
        ];
        let state = draw(&ctx, 0.0, &events, false);
        assert_eq!(state.waves.len(), MAX_WAVES);
        assert!(state.held.contains(&30));
        let state = draw(&ctx, WAVE_SECONDS, &[], false);
        assert!(state.waves.is_empty());
        assert!(state.held.contains(&30));
        let state = draw(
            &ctx,
            0.7,
            &[KeyEvent {
                scan: 30,
                pressed: false,
            }],
            false,
        );
        assert!(state.held.is_empty());
        draw(&ctx, 0.8, &events, false);
        let state = draw(&ctx, 0.9, &[], true);
        assert!(state.held.is_empty() && state.waves.is_empty());
        clear(&ctx);
        assert!(
            ctx.data(|data| data.get_temp::<State>(egui::Id::new(STATE_ID)))
                .is_none()
        );
    }

    #[test]
    fn reduced_motion_clears_waves_but_keeps_press_and_release_feedback() {
        let ctx = egui::Context::default();
        let press = [KeyEvent {
            scan: 30,
            pressed: true,
        }];
        assert!(!draw(&ctx, 0.0, &press, false).waves.is_empty());
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("motion-allowed"), false));
        let state = draw(&ctx, 0.1, &press, false);
        assert!(state.waves.is_empty());
        assert!(state.held.contains(&30));
        let state = draw(
            &ctx,
            0.2,
            &[KeyEvent {
                scan: 30,
                pressed: false,
            }],
            false,
        );
        assert!(state.held.is_empty() && state.waves.is_empty());
    }
}
