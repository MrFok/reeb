use eframe::egui;

pub const SPRING_FREQUENCY: f32 = 38.0;
pub const HOVER_SECONDS: f32 = 0.12;
pub const PAGE_SECONDS: f32 = 0.14;
pub const PAGE_START_OPACITY: f32 = 0.84;
const SETTLE_DISTANCE: f32 = 0.001;
const SETTLE_VELOCITY: f32 = 0.01;

#[derive(Clone, Copy)]
struct Spring {
    value: f32,
    velocity: f32,
    time: f64,
}

impl Spring {
    fn advance(&mut self, target: f32, now: f64) {
        let dt = (now - self.time).clamp(0.0, 0.1) as f32;
        self.time = now;
        let offset = self.value - target;
        let coefficient = self.velocity + SPRING_FREQUENCY * offset;
        let decay = (-SPRING_FREQUENCY * dt).exp();
        self.value = target + (offset + coefficient * dt) * decay;
        self.velocity = (self.velocity - SPRING_FREQUENCY * coefficient * dt) * decay;
        if self.settled(target) {
            self.value = target;
            self.velocity = 0.0;
        }
    }

    fn settled(&self, target: f32) -> bool {
        (self.value - target).abs() < SETTLE_DISTANCE && self.velocity.abs() < SETTLE_VELOCITY
    }
}

pub fn begin(ctx: &egui::Context) {
    #[cfg(windows)]
    let allowed = unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SPI_GETCLIENTAREAANIMATION, SystemParametersInfoW,
        };
        let mut enabled = 1_i32;
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            (&mut enabled as *mut i32).cast(),
            0,
        );
        enabled != 0
    };
    #[cfg(not(windows))]
    let allowed = true;
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("motion-allowed"), allowed));
    ctx.all_styles_mut(|style| style.animation_time = if allowed { HOVER_SECONDS } else { 0.0 });
}

pub fn allowed(ctx: &egui::Context) -> bool {
    ctx.data(|data| {
        data.get_temp::<bool>(egui::Id::new("motion-allowed"))
            .unwrap_or(true)
    })
}

pub fn spring(ctx: &egui::Context, id: egui::Id, target: f32, direct: bool) -> f32 {
    let now = ctx.input(|input| input.time);
    let enabled = allowed(ctx) && !direct;
    let state = ctx.data_mut(|data| {
        let state = data.get_temp_mut_or_insert_with(id, || Spring {
            value: target,
            velocity: 0.0,
            time: now,
        });
        if enabled {
            state.advance(target, now);
        } else {
            *state = Spring {
                value: target,
                velocity: 0.0,
                time: now,
            };
        }
        *state
    });
    if !state.settled(target) {
        ctx.request_repaint();
    }
    state.value
}

pub fn hover(ctx: &egui::Context, id: egui::Id, hovered: bool) -> f32 {
    ctx.animate_bool_with_time(id, hovered, if allowed(ctx) { HOVER_SECONDS } else { 0.0 })
}

#[derive(Clone, Copy)]
struct PageFade {
    page: egui::Id,
    start: f64,
}

pub fn page_opacity(ctx: &egui::Context, page: egui::Id) -> f32 {
    let now = ctx.input(|input| input.time);
    let start = ctx.data_mut(|data| {
        let state = data.get_temp_mut_or_insert_with(egui::Id::new("page-fade"), || PageFade {
            page,
            start: now - PAGE_SECONDS as f64,
        });
        if state.page != page {
            state.page = page;
            state.start = now;
        }
        state.start
    });
    if !allowed(ctx) {
        return 1.0;
    }
    let progress = ((now - start) as f32 / PAGE_SECONDS).clamp(0.0, 1.0);
    if progress < 1.0 {
        ctx.request_repaint();
    }
    egui::lerp(PAGE_START_OPACITY..=1.0, 1.0 - (1.0 - progress).powi(3))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_settles_without_overshoot_and_handles_retargeting() {
        let mut spring = Spring {
            value: 0.0,
            velocity: 0.0,
            time: 0.0,
        };
        let mut previous = 0.0;
        for frame in 1..=60 {
            spring.advance(100.0, frame as f64 / 60.0);
            assert!(spring.value >= previous && spring.value <= 100.0);
            previous = spring.value;
        }
        assert!(spring.settled(100.0));
        for frame in 61..=120 {
            spring.advance(20.0, frame as f64 / 60.0);
            assert!((20.0..=100.0).contains(&spring.value));
        }
        assert!(spring.settled(20.0));
    }

    #[test]
    fn reduced_motion_snaps_and_page_transitions_finish() {
        let ctx = egui::Context::default();
        let id = egui::Id::new("test-spring");
        let _ = ctx.run(
            egui::RawInput {
                time: Some(0.0),
                ..Default::default()
            },
            |ctx| {
                spring(ctx, id, 0.0, false);
                assert_eq!(page_opacity(ctx, egui::Id::new("one")), 1.0);
                assert_eq!(page_opacity(ctx, egui::Id::new("two")), PAGE_START_OPACITY);
                ctx.data_mut(|data| data.insert_temp(egui::Id::new("motion-allowed"), false));
                assert_eq!(spring(ctx, id, 1.0, false), 1.0);
                assert_eq!(page_opacity(ctx, egui::Id::new("three")), 1.0);
            },
        );
        let _ = ctx.run(
            egui::RawInput {
                time: Some(1.0),
                ..Default::default()
            },
            |ctx| {
                ctx.data_mut(|data| data.insert_temp(egui::Id::new("motion-allowed"), true));
                assert_eq!(page_opacity(ctx, egui::Id::new("three")), 1.0);
            },
        );
    }
}
