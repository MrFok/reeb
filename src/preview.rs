use reeb::keys::KeyEvent;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

const RELEASE_DELAY: Duration = Duration::from_millis(100);
const STROKE_INTERVAL: Duration = Duration::from_millis(240);
const COOLDOWN: Duration = Duration::from_millis(600);
pub(crate) const BURST_DURATION: Duration = Duration::from_millis(580);

#[derive(Default)]
pub struct Preview {
    pending: VecDeque<(Instant, KeyEvent)>,
    last_tone: Option<Instant>,
}

impl Preview {
    pub fn tone(&mut self, scan: u16, now: Instant) {
        if self
            .last_tone
            .is_some_and(|last| now.duration_since(last) < COOLDOWN)
        {
            return;
        }
        self.cancel();
        self.last_tone = Some(now);
        for index in 0..3 {
            self.strike(scan, now + STROKE_INTERVAL * index);
        }
    }

    fn strike(&mut self, scan: u16, at: Instant) {
        self.pending.push_back((
            at,
            KeyEvent {
                scan,
                pressed: true,
            },
        ));
        self.pending.push_back((
            at + RELEASE_DELAY,
            KeyEvent {
                scan,
                pressed: false,
            },
        ));
    }

    pub fn next_due(&mut self, now: Instant) -> Option<KeyEvent> {
        if self.pending.front().is_some_and(|(at, _)| *at <= now) {
            self.pending.pop_front().map(|(_, event)| event)
        } else {
            None
        }
    }

    pub fn delay(&self, now: Instant) -> Duration {
        self.pending
            .front()
            .map_or(Duration::from_millis(100), |(at, _)| {
                at.saturating_duration_since(now)
                    .min(Duration::from_millis(100))
            })
    }

    pub fn cancel(&mut self) {
        self.pending.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burst_plays_exactly_three_strikes_with_matching_releases() {
        let mut preview = Preview::default();
        let now = Instant::now();
        preview.tone(0x1e, now);
        for (at, pressed) in [
            (0, true),
            (100, false),
            (240, true),
            (340, false),
            (480, true),
            (580, false),
        ] {
            if at > 0 {
                assert!(
                    preview
                        .next_due(now + Duration::from_millis(at - 1))
                        .is_none()
                );
            }
            let event = preview.next_due(now + Duration::from_millis(at)).unwrap();
            assert_eq!(event.scan, 0x1e);
            assert_eq!(event.pressed, pressed);
            assert!(preview.next_due(now + Duration::from_millis(at)).is_none());
        }
        assert!(preview.next_due(now + Duration::from_secs(5)).is_none());
    }

    #[test]
    fn cooldown_rejects_early_requests_and_accepts_at_six_hundred_ms() {
        let mut preview = Preview::default();
        let now = Instant::now();
        preview.tone(30, now);
        preview.tone(31, now + Duration::from_millis(599));
        assert_eq!(preview.pending.len(), 6);
        assert!(preview.pending.iter().all(|(_, event)| event.scan == 30));
        preview.tone(31, now + COOLDOWN);
        assert_eq!(preview.pending.len(), 6);
        assert!(preview.pending.iter().all(|(_, event)| event.scan == 31));
        assert_eq!(
            preview.pending.back().unwrap().0,
            now + COOLDOWN + BURST_DURATION
        );
    }

    #[test]
    fn tone_preview_is_bounded_and_cancellation_discards_future_strikes() {
        let mut preview = Preview::default();
        let now = Instant::now();
        preview.tone(0x1e, now);
        assert!(preview.next_due(now).unwrap().pressed);
        assert!(preview.next_due(now).is_none());
        preview.tone(0x1e, now + Duration::from_millis(10));
        assert_eq!(preview.pending.len(), 5);
        preview.cancel();
        assert!(preview.next_due(now + Duration::from_secs(2)).is_none());
    }
}
