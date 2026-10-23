use crate::preview::{BURST_DURATION, Preview};
use reeb::{
    audio::Audio,
    mixer::{Settings, Shared, pitch_rate},
    samples::{SampleBank, Switch},
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

const DWELL: Duration = Duration::from_millis(220);

struct Hover {
    switch: Switch,
    entered: Instant,
    started: bool,
}
struct Playback {
    _audio: Audio,
    shared: Arc<Shared>,
    preview: Preview,
    expires: Instant,
}

#[derive(Default)]
pub struct Audition {
    hover: Option<Hover>,
    playback: Option<Playback>,
    error: Option<String>,
}

impl Audition {
    pub fn cancel(&mut self) {
        self.playback = None;
        self.hover = None;
        self.error = None;
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    fn enter(&mut self, target: Option<Switch>, now: Instant) {
        if self.hover.as_ref().map(|hover| hover.switch) != target {
            self.cancel();
            self.hover = target.map(|switch| Hover {
                switch,
                entered: now,
                started: false,
            });
        }
    }
    fn due(&mut self, now: Instant) -> Option<Switch> {
        let hover = self.hover.as_mut()?;
        if !hover.started && now.duration_since(hover.entered) >= DWELL {
            hover.started = true;
            Some(hover.switch)
        } else {
            None
        }
    }
    pub fn update(
        &mut self,
        target: Option<Switch>,
        settings: &Settings,
        device: Option<&str>,
        now: Instant,
    ) {
        self.enter(target, now);
        if let Some(switch) = self.due(now) {
            let start = || {
                let bank = Arc::new(SampleBank::load(switch)?);
                let tail = bank
                    .samples
                    .iter()
                    .map(|sample| {
                        sample.frames.len() as f64 / sample.rate as f64 / pitch_rate(0.0) as f64
                    })
                    .fold(0.0, f64::max);
                let expires = now + BURST_DURATION + Duration::from_secs_f64(tail);
                let shared = Shared::new(settings);
                let audio = Audio::start(bank, shared.clone(), device)?;
                let mut preview = Preview::default();
                preview.tone(0x1e, now);
                Ok::<_, String>(Playback {
                    _audio: audio,
                    shared,
                    preview,
                    expires,
                })
            };
            match start() {
                Ok(playback) => self.playback = Some(playback),
                Err(error) => self.error = Some(format!("Switch preview unavailable: {error}")),
            }
        }
        if self
            .playback
            .as_ref()
            .is_some_and(|playback| now >= playback.expires)
        {
            self.playback = None;
        }
        if let Some(playback) = &mut self.playback {
            playback.shared.update(settings);
            while let Some(event) = playback.preview.next_due(now) {
                playback.shared.push(event);
            }
        }
    }
    pub fn delay(&self, now: Instant) -> Duration {
        if let Some(playback) = &self.playback {
            playback.preview.delay(now)
        } else if let Some(hover) = &self.hover
            && !hover.started
        {
            DWELL.saturating_sub(now.duration_since(hover.entered))
        } else {
            Duration::from_millis(100)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a real Windows audio output"]
    fn hover_preview_and_selected_audio_can_share_the_output() {
        use std::sync::atomic::Ordering;
        let settings = Settings {
            volume: 0.0,
            ..Default::default()
        };
        let main_shared = Shared::new(&settings);
        let _main_audio = Audio::start(
            Arc::new(SampleBank::bundled().unwrap()),
            main_shared.clone(),
            None,
        )
        .unwrap();
        let mut audition = Audition::default();
        let entered = Instant::now();
        audition.update(Some(Switch::CherryMxBrownFoxfire), &settings, None, entered);
        std::thread::sleep(DWELL);
        let deadline = Instant::now() + Duration::from_millis(650);
        while Instant::now() < deadline {
            audition.update(
                Some(Switch::CherryMxBrownFoxfire),
                &settings,
                None,
                Instant::now(),
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(audition.error().is_none(), "{:?}", audition.error());
        let preview = &audition.playback.as_ref().unwrap().shared;
        assert_eq!(preview.events.load(Ordering::Relaxed), 6);
        assert!(preview.frames.load(Ordering::Relaxed) > 0);
        assert!(main_shared.frames.load(Ordering::Relaxed) > 0);
        assert_eq!(main_shared.events.load(Ordering::Relaxed), 0);
        assert!(!preview.stream_failed.load(Ordering::Relaxed));
        assert!(!main_shared.stream_failed.load(Ordering::Relaxed));
        audition.update(None, &settings, None, Instant::now());
        assert!(audition.playback.is_none());
    }

    #[test]
    fn each_hover_auditions_once_after_dwell_and_crossing_tiles_cancels() {
        let mut audition = Audition::default();
        let now = Instant::now();
        audition.enter(Some(Switch::CherryMxBrownFoxfire), now);
        assert_eq!(audition.due(now + Duration::from_millis(100)), None);
        assert_eq!(
            audition.due(now + DWELL),
            Some(Switch::CherryMxBrownFoxfire)
        );
        assert_eq!(audition.due(now + Duration::from_secs(2)), None);
        audition.enter(Some(Switch::CherryKc1000), now + Duration::from_secs(2));
        assert_eq!(audition.due(now + Duration::from_secs(2)), None);
        assert_eq!(
            audition.due(now + Duration::from_secs(2) + DWELL),
            Some(Switch::CherryKc1000)
        );
        audition.enter(None, now + Duration::from_secs(3));
        assert!(audition.hover.is_none() && audition.playback.is_none());
        audition.enter(
            Some(Switch::CherryMxBrownFoxfire),
            now + Duration::from_secs(4),
        );
        assert_eq!(
            audition.due(now + Duration::from_secs(4) + DWELL),
            Some(Switch::CherryMxBrownFoxfire)
        );
    }
}
