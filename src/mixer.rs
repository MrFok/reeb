use crate::{
    keys::{self, KeyEvent},
    samples::SampleBank,
};
use crossbeam_queue::ArrayQueue;
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
};

pub const VOICES: usize = 64;
pub const QUEUE_SIZE: usize = 256;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EnterSettings {
    pub enabled: bool,
    pub volume: f32,
    pub use_keyboard_tone: bool,
    pub tone_x: f32,
    pub tone_y: f32,
}

impl Default for EnterSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 1.0,
            use_keyboard_tone: true,
            tone_x: 0.5,
            tone_y: 0.5,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub volume: f32,
    pub tone_x: f32,
    pub tone_y: f32,
    pub width: f32,
    pub release_sounds: bool,
    pub enter: EnterSettings,
    pub spatial_audio: bool,
    pub normalize_volume: bool,
    pub mute_modifiers: bool,
    pub per_key_feel: bool,
    pub home_row_softness: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            volume: 0.3,
            tone_x: 0.65,
            tone_y: 0.5,
            width: 0.8,
            release_sounds: true,
            enter: EnterSettings::default(),
            spatial_audio: true,
            normalize_volume: false,
            mute_modifiers: false,
            per_key_feel: false,
            home_row_softness: 1.0,
        }
    }
}

impl Settings {
    pub fn sanitize(&mut self) {
        let defaults = Self::default();
        for (value, fallback) in [
            (&mut self.volume, defaults.volume),
            (&mut self.tone_x, defaults.tone_x),
            (&mut self.tone_y, defaults.tone_y),
            (&mut self.width, defaults.width),
            (&mut self.home_row_softness, defaults.home_row_softness),
            (&mut self.enter.volume, defaults.enter.volume),
            (&mut self.enter.tone_x, defaults.enter.tone_x),
            (&mut self.enter.tone_y, defaults.enter.tone_y),
        ] {
            *value = if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                fallback
            };
        }
    }
}

pub struct QueuedEvent {
    event: KeyEvent,
    generation: u64,
}

pub struct Shared {
    pub queue: ArrayQueue<QueuedEvent>,
    visual_queue: ArrayQueue<KeyEvent>,
    visual_enabled: AtomicBool,
    visual_overflow: AtomicBool,
    spatial_audio: AtomicBool,
    normalize_volume: AtomicBool,
    mute_modifiers: AtomicBool,
    per_key_feel: AtomicBool,
    home_row_softness: AtomicU32,
    pub enabled: AtomicBool,
    reset_generation: AtomicU64,
    volume: AtomicU32,
    tone_x: AtomicU32,
    tone_y: AtomicU32,
    width: AtomicU32,
    release_sounds: AtomicBool,
    enter_enabled: AtomicBool,
    enter_volume: AtomicU32,
    enter_use_keyboard_tone: AtomicBool,
    enter_tone_x: AtomicU32,
    enter_tone_y: AtomicU32,
    pub events: AtomicU64,
    pub dropped: AtomicU64,
    pub frames: AtomicU64,
    pub stream_failed: AtomicBool,
}

impl Shared {
    pub fn new(settings: &Settings) -> Arc<Self> {
        let mut settings = settings.clone();
        settings.sanitize();
        Arc::new(Self {
            queue: ArrayQueue::new(QUEUE_SIZE),
            visual_queue: ArrayQueue::new(QUEUE_SIZE),
            visual_enabled: AtomicBool::new(false),
            visual_overflow: AtomicBool::new(false),
            spatial_audio: AtomicBool::new(settings.spatial_audio),
            normalize_volume: AtomicBool::new(settings.normalize_volume),
            mute_modifiers: AtomicBool::new(settings.mute_modifiers),
            per_key_feel: AtomicBool::new(settings.per_key_feel),
            home_row_softness: AtomicU32::new(settings.home_row_softness.to_bits()),
            enabled: AtomicBool::new(true),
            reset_generation: AtomicU64::new(0),
            volume: AtomicU32::new(settings.volume.to_bits()),
            tone_x: AtomicU32::new(settings.tone_x.to_bits()),
            tone_y: AtomicU32::new(settings.tone_y.to_bits()),
            width: AtomicU32::new(settings.width.to_bits()),
            release_sounds: AtomicBool::new(settings.release_sounds),
            enter_enabled: AtomicBool::new(settings.enter.enabled),
            enter_volume: AtomicU32::new(settings.enter.volume.to_bits()),
            enter_use_keyboard_tone: AtomicBool::new(settings.enter.use_keyboard_tone),
            enter_tone_x: AtomicU32::new(settings.enter.tone_x.to_bits()),
            enter_tone_y: AtomicU32::new(settings.enter.tone_y.to_bits()),
            events: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            frames: AtomicU64::new(0),
            stream_failed: AtomicBool::new(false),
        })
    }

    pub fn push(&self, event: KeyEvent) {
        if self.visual_enabled.load(Ordering::Relaxed)
            && self.visual_queue.force_push(event).is_some()
        {
            self.visual_overflow.store(true, Ordering::Relaxed);
        }
        let generation = self.reset_generation.load(Ordering::Acquire);
        if !self.enabled.load(Ordering::Acquire) {
            return;
        }
        if self
            .queue
            .force_push(QueuedEvent { event, generation })
            .is_some()
        {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
        self.events.fetch_add(1, Ordering::Relaxed);
    }

    pub fn set_visualize(&self, enabled: bool) {
        self.visual_enabled.store(enabled, Ordering::Relaxed);
        if !enabled {
            self.visual_overflow.store(false, Ordering::Relaxed);
            while self.visual_queue.pop().is_some() {}
        }
    }

    pub fn reset_visualization(&self) {
        for _ in 0..QUEUE_SIZE {
            if self.visual_queue.pop().is_none() {
                break;
            }
        }
        self.visual_overflow.store(true, Ordering::Release);
    }

    pub fn take_visual_events(&self, events: &mut Vec<KeyEvent>) -> bool {
        events.clear();
        for _ in 0..QUEUE_SIZE {
            let Some(event) = self.visual_queue.pop() else {
                break;
            };
            events.push(event);
        }
        self.visual_overflow.swap(false, Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        if self.enabled.swap(enabled, Ordering::AcqRel) && !enabled {
            self.reset_generation.fetch_add(1, Ordering::Release);
            for _ in 0..QUEUE_SIZE {
                if self.queue.pop().is_none() {
                    break;
                }
            }
        }
    }

    pub fn update(&self, settings: &Settings) {
        let mut settings = settings.clone();
        settings.sanitize();
        self.spatial_audio
            .store(settings.spatial_audio, Ordering::Relaxed);
        self.normalize_volume
            .store(settings.normalize_volume, Ordering::Relaxed);
        self.mute_modifiers
            .store(settings.mute_modifiers, Ordering::Relaxed);
        self.per_key_feel
            .store(settings.per_key_feel, Ordering::Relaxed);
        self.home_row_softness
            .store(settings.home_row_softness.to_bits(), Ordering::Relaxed);
        self.volume
            .store(settings.volume.to_bits(), Ordering::Relaxed);
        self.tone_x
            .store(settings.tone_x.to_bits(), Ordering::Relaxed);
        self.tone_y
            .store(settings.tone_y.to_bits(), Ordering::Relaxed);
        self.width
            .store(settings.width.to_bits(), Ordering::Relaxed);
        self.release_sounds
            .store(settings.release_sounds, Ordering::Relaxed);
        self.enter_enabled
            .store(settings.enter.enabled, Ordering::Relaxed);
        self.enter_volume
            .store(settings.enter.volume.to_bits(), Ordering::Relaxed);
        self.enter_use_keyboard_tone
            .store(settings.enter.use_keyboard_tone, Ordering::Relaxed);
        self.enter_tone_x
            .store(settings.enter.tone_x.to_bits(), Ordering::Relaxed);
        self.enter_tone_y
            .store(settings.enter.tone_y.to_bits(), Ordering::Relaxed);
    }
}

const ATTACK_MS: f64 = 0.5;
const END_FADE_MS: f64 = 4.0;
const OUTPUT_GAIN: f32 = 4.0;

#[derive(Clone, Copy, Default)]
struct Voice {
    sample: Option<usize>,
    enter: bool,
    modifier: bool,
    position: f64,
    step: f64,
    age: u64,
    attack_frames: f64,
    end_fade_frames: f64,
    filter_alpha: f32,
    low_pass: f32,
    left: f32,
    right: f32,
}

impl Voice {
    fn next(&mut self, bank: &SampleBank) -> f32 {
        let Some(index) = self.sample else {
            return 0.0;
        };
        let frames = &bank.samples[index].frames;
        if frames.len() < 2 || self.position >= (frames.len() - 1) as f64 {
            self.sample = None;
            return 0.0;
        }
        let index = self.position as usize;
        let fraction = (self.position - index as f64) as f32;
        let input = frames[index] * (1.0 - fraction) + frames[index + 1] * fraction;
        self.low_pass += self.filter_alpha * (input - self.low_pass);
        let attack = (self.age as f64 / self.attack_frames).min(1.0);
        let remaining = ((frames.len() - 1) as f64 - self.position) / self.step;
        let fade = (remaining / self.end_fade_frames).min(1.0);
        self.position += self.step;
        self.age += 1;
        self.low_pass * attack.min(fade) as f32
    }
}

pub fn pitch_rate(y: f32) -> f32 {
    let y = if y.is_finite() {
        y.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let semitones = 6.0 * y - 3.0;
    2.0_f32.powf(semitones / 12.0)
}

fn brightness_alpha(tone_x: f32, rate: u32) -> f32 {
    let hz = (800.0 * (18_000.0_f32 / 800.0).powf(tone_x)).min(rate as f32 * 0.45);
    1.0 - (-std::f32::consts::TAU * hz / rate as f32).exp()
}

pub struct Mixer {
    bank: Arc<SampleBank>,
    shared: Arc<Shared>,
    voices: [Voice; VOICES],
    steal: usize,
    cursors: [usize; 8],
    rate: u32,
    reset_generation: u64,
}

impl Mixer {
    pub fn new(bank: Arc<SampleBank>, shared: Arc<Shared>, rate: u32) -> Self {
        let reset_generation = shared.reset_generation.load(Ordering::Acquire);
        Self {
            bank,
            shared,
            voices: [Voice::default(); VOICES],
            steal: 0,
            cursors: [0; 8],
            rate: rate.max(1),
            reset_generation,
        }
    }

    fn begin_block(&mut self) -> Option<f32> {
        let enabled = self.shared.enabled.load(Ordering::Acquire);
        let generation = self.shared.reset_generation.load(Ordering::Acquire);
        if self.reset_generation != generation || !enabled {
            self.voices.fill(Voice::default());
            self.reset_generation = generation;
        }
        if !enabled {
            return None;
        }
        let tone_x = f32::from_bits(self.shared.tone_x.load(Ordering::Relaxed));
        let tone_y = f32::from_bits(self.shared.tone_y.load(Ordering::Relaxed));
        let width = if self.shared.spatial_audio.load(Ordering::Relaxed) {
            f32::from_bits(self.shared.width.load(Ordering::Relaxed))
        } else {
            0.0
        };
        let normalization = if self.shared.normalize_volume.load(Ordering::Relaxed) {
            self.bank.normalization_gain
        } else {
            1.0
        };
        let per_key_feel = self.shared.per_key_feel.load(Ordering::Relaxed);
        let softness = f32::from_bits(self.shared.home_row_softness.load(Ordering::Relaxed));
        let mute_modifiers = self.shared.mute_modifiers.load(Ordering::Relaxed);
        let releases = self.shared.release_sounds.load(Ordering::Relaxed);
        let enter_enabled = self.shared.enter_enabled.load(Ordering::Relaxed);
        let enter_volume = f32::from_bits(self.shared.enter_volume.load(Ordering::Relaxed));
        let enter_own_tone = !self.shared.enter_use_keyboard_tone.load(Ordering::Relaxed);
        let enter_x = f32::from_bits(self.shared.enter_tone_x.load(Ordering::Relaxed));
        let enter_y = f32::from_bits(self.shared.enter_tone_y.load(Ordering::Relaxed));
        let keyboard_pitch = pitch_rate(tone_y);
        let enter_pitch = pitch_rate(enter_y);
        let keyboard_alpha = brightness_alpha(tone_x, self.rate);
        let enter_alpha = brightness_alpha(enter_x, self.rate);
        for voice in &mut self.voices {
            if (mute_modifiers && voice.modifier) || (!enter_enabled && voice.enter) {
                *voice = Voice::default();
            }
        }
        for _ in 0..QUEUE_SIZE {
            let Some(queued) = self.shared.queue.pop() else {
                break;
            };
            let Some(event) = self.current_event(queued) else {
                continue;
            };
            let group = keys::group(event.scan);
            let enter = group == keys::Group::Enter;
            let modifier = keys::modifier(event.scan);
            if (!event.pressed && !releases)
                || (modifier && mute_modifiers)
                || (enter && !enter_enabled)
            {
                continue;
            }
            let Some(sample) = self.bank.choose(group, event.pressed, &mut self.cursors) else {
                continue;
            };
            let slot = self
                .voices
                .iter()
                .position(|voice| voice.sample.is_none())
                .unwrap_or_else(|| {
                    let slot = self.steal;
                    self.steal = (self.steal + 1) % VOICES;
                    slot
                });
            let pan = keys::pan(event.scan) * width;
            let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
            let gain = normalization
                * if enter { enter_volume } else { 1.0 }
                * if per_key_feel {
                    keys::feel(event.scan, softness)
                } else {
                    1.0
                };
            let (pitch, filter_alpha) = if enter && enter_own_tone {
                (enter_pitch, enter_alpha)
            } else {
                (keyboard_pitch, keyboard_alpha)
            };
            self.voices[slot] = Voice {
                sample: Some(sample),
                enter,
                modifier,
                step: f64::from(pitch) * f64::from(self.bank.samples[sample].rate)
                    / f64::from(self.rate),
                attack_frames: ATTACK_MS * f64::from(self.rate) / 1000.0,
                end_fade_frames: END_FADE_MS * f64::from(self.rate) / 1000.0,
                filter_alpha,
                left: angle.cos() * gain,
                right: angle.sin() * gain,
                ..Voice::default()
            };
        }
        Some(f32::from_bits(self.shared.volume.load(Ordering::Relaxed)))
    }

    fn current_event(&mut self, queued: QueuedEvent) -> Option<KeyEvent> {
        let generation = self.shared.reset_generation.load(Ordering::Acquire);
        if self.reset_generation != generation {
            self.voices.fill(Voice::default());
            self.reset_generation = generation;
        }
        (queued.generation == generation && self.shared.enabled.load(Ordering::Acquire))
            .then_some(queued.event)
    }

    pub fn render<T>(&mut self, output: &mut [T], channels: usize)
    where
        T: cpal::Sample + cpal::FromSample<f32>,
    {
        output.fill(T::EQUILIBRIUM);
        if channels == 0 || output.len() < channels {
            return;
        }
        let Some(volume) = self.begin_block() else {
            return;
        };
        for frame in output.chunks_exact_mut(channels) {
            let (mut left, mut right) = (0.0, 0.0);
            for voice in &mut self.voices {
                if voice.sample.is_some() {
                    let value = voice.next(&self.bank);
                    left += value * voice.left;
                    right += value * voice.right;
                }
            }
            let gain = OUTPUT_GAIN * volume;
            if channels == 1 {
                frame[0] = T::from_sample(((left + right) * 0.5 * gain).tanh());
            } else {
                frame[0] = T::from_sample((left * gain).tanh());
                frame[1] = T::from_sample((right * gain).tanh());
            }
        }
        self.shared
            .frames
            .fetch_add((output.len() / channels) as u64, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::samples::Switch;

    #[test]
    fn delayed_enqueue_from_before_pause_cannot_play_after_resume() {
        let (mut mixer, shared) = setup(constant_bank(), &Settings::default(), 48_000);
        let generation = shared.reset_generation.load(Ordering::Acquire);
        shared.set_enabled(false);
        shared.set_enabled(true);
        shared.queue.force_push(QueuedEvent {
            event: KeyEvent {
                scan: 30,
                pressed: true,
            },
            generation,
        });
        let mut output = [0.0_f32; 1024];
        mixer.render(&mut output, 2);
        assert!(output.iter().all(|&value| value == 0.0));
        shared.push(KeyEvent {
            scan: 30,
            pressed: true,
        });
        mixer.render(&mut output, 2);
        assert!(output.iter().any(|&value| value != 0.0));
    }

    #[test]
    fn visual_reset_discards_disconnected_presses_and_is_reported_once() {
        let shared = Shared::new(&Settings::default());
        shared.set_visualize(true);
        shared.push(KeyEvent {
            scan: 30,
            pressed: true,
        });
        shared.reset_visualization();
        let mut events = Vec::new();
        assert!(shared.take_visual_events(&mut events));
        assert!(events.is_empty());
        assert!(!shared.take_visual_events(&mut events));
    }

    #[test]
    fn callbacks_keep_new_events_when_pause_and_resume_cross_a_block() {
        let (mut mixer, shared) = setup(constant_bank(), &Settings::default(), 48_000);
        let old_generation = mixer.reset_generation;
        shared.set_enabled(false);
        assert!(mixer.begin_block().is_none());
        shared.set_enabled(true);
        shared.push(KeyEvent {
            scan: 30,
            pressed: true,
        });
        assert_eq!(shared.queue.len(), 1);
        mixer.reset_generation = old_generation;
        let queued = shared.queue.pop().unwrap();
        assert!(mixer.current_event(queued).unwrap().pressed);
        shared.push(KeyEvent {
            scan: 30,
            pressed: true,
        });
        let mut output = [0.0_f32; 1024];
        mixer.render(&mut output, 2);
        assert!(output.iter().any(|&value| value != 0.0));
    }

    fn fixture(rate: u32, frames: Vec<f32>) -> Arc<SampleBank> {
        let mut bank = SampleBank::load(Switch::CherryMxBrownFoxfire).unwrap();
        for sample in &mut bank.samples {
            sample.rate = rate;
            sample.frames = frames.clone();
        }
        bank.normalization_gain = 0.6;
        Arc::new(bank)
    }

    fn constant_bank() -> Arc<SampleBank> {
        fixture(48_000, vec![0.2; 4_801])
    }

    fn setup(bank: Arc<SampleBank>, settings: &Settings, rate: u32) -> (Mixer, Arc<Shared>) {
        let shared = Shared::new(settings);
        (Mixer::new(bank, shared.clone(), rate), shared)
    }

    fn stroke(bank: Arc<SampleBank>, settings: &Settings, scan: u16, pressed: bool) -> Vec<f32> {
        let (mut mixer, shared) = setup(bank, settings, 48_000);
        shared.push(KeyEvent { scan, pressed });
        let mut output = vec![0.0; 12_000];
        mixer.render(&mut output, 2);
        output
    }

    fn energy(output: &[f32]) -> f32 {
        output.iter().map(|value| value * value).sum()
    }

    #[test]
    fn overload_is_bounded_audible_and_event_work_is_limited() {
        let (mut mixer, shared) = setup(
            constant_bank(),
            &Settings {
                volume: 1.0,
                normalize_volume: true,
                ..Settings::default()
            },
            48_000,
        );
        for index in 0..QUEUE_SIZE + 17 {
            shared.push(KeyEvent {
                scan: if index % 2 == 0 { 0x1e } else { 0x27 },
                pressed: true,
            });
        }
        assert_eq!(shared.queue.len(), QUEUE_SIZE);
        assert_eq!(shared.dropped.load(Ordering::Relaxed), 17);
        let mut output = vec![0.0_f32; 12_000];
        mixer.render(&mut output, 2);
        assert!(shared.queue.is_empty());
        assert!(
            output
                .iter()
                .all(|value| value.is_finite() && value.abs() <= 1.0)
        );
        assert!(output.iter().any(|value| value.abs() > 0.9));
        assert!(output[10_000..].iter().all(|&value| value == 0.0));
    }

    #[test]
    fn panning_follows_geometry_and_width_can_center_it() {
        let bank = constant_bank();
        let mut settings = Settings {
            width: 1.0,
            ..Settings::default()
        };
        let left = stroke(bank.clone(), &settings, 0x1e, true);
        let right = stroke(bank.clone(), &settings, 0x27, true);
        let channel_energy = |output: &[f32], channel: usize| {
            output
                .chunks_exact(2)
                .map(|frame| frame[channel].powi(2))
                .sum::<f32>()
        };
        assert!(channel_energy(&left, 0) > channel_energy(&left, 1) * 3.0);
        assert!(channel_energy(&right, 1) > channel_energy(&right, 0) * 3.0);
        for disable_spatial in [false, true] {
            settings.spatial_audio = !disable_spatial;
            settings.width = if disable_spatial { 1.0 } else { 0.0 };
            let centered = stroke(bank.clone(), &settings, 0x1e, true);
            assert!(
                centered
                    .chunks_exact(2)
                    .all(|frame| (frame[0] - frame[1]).abs() < 1e-7)
            );
        }
    }

    #[test]
    fn millisecond_envelopes_and_resampling_preserve_duration_at_device_rates() {
        let bank = fixture(22_050, vec![0.1; 2_206]);
        let settings = Settings {
            spatial_audio: false,
            tone_x: 1.0,
            ..Settings::default()
        };
        for rate in [44_100, 48_000, 96_000] {
            let (mut mixer, shared) = setup(bank.clone(), &settings, rate);
            shared.push(KeyEvent {
                scan: 0x1e,
                pressed: true,
            });
            let mut output = vec![0.0_f32; rate as usize / 5];
            mixer.render(&mut output, 1);
            assert_eq!(output[0], 0.0);
            let at_ms = |ms: f64| output[(ms * f64::from(rate) / 1000.0) as usize];
            let plateau = at_ms(50.0);
            assert!(plateau > 0.05);
            assert!(at_ms(0.25) > 0.0 && at_ms(0.25) < plateau * 0.65);
            assert!((at_ms(1.0) - plateau).abs() < 0.001);
            assert!(at_ms(98.0) > plateau * 0.4 && at_ms(98.0) < plateau * 0.6);
            let last = output.iter().rposition(|value| *value != 0.0).unwrap();
            assert!((last as f64 / f64::from(rate) - 0.1).abs() < 2.0 / f64::from(rate));
            assert!(
                output[(rate as usize * 101 / 1000)..]
                    .iter()
                    .all(|&value| value == 0.0)
            );
        }
    }

    #[test]
    fn pitch_uses_semitones_and_changes_frequency_and_playback_length() {
        assert_eq!(pitch_rate(0.5), 1.0);
        assert!((pitch_rate(0.0) - 0.840_896_4).abs() < 1e-6);
        assert!((pitch_rate(1.0) - 1.189_207_1).abs() < 1e-6);
        assert_eq!(pitch_rate(f32::NAN), 1.0);
        assert_eq!(pitch_rate(-1.0), pitch_rate(0.0));
        let frames = (0..24_001)
            .map(|i| (std::f32::consts::TAU * 440.0 * i as f32 / 48_000.0).sin() * 0.1)
            .collect();
        let bank = fixture(48_000, frames);
        for (tone_y, frequency) in [(0.0, 370.0), (0.5, 440.0), (1.0, 523.25)] {
            for rate in [44_100, 48_000, 96_000] {
                let (mut mixer, shared) = setup(
                    bank.clone(),
                    &Settings {
                        tone_y,
                        ..Settings::default()
                    },
                    rate,
                );
                shared.push(KeyEvent {
                    scan: 0x1e,
                    pressed: true,
                });
                let mut output = vec![0.0_f32; rate as usize * 3 / 4];
                mixer.render(&mut output, 1);
                let middle = &output[rate as usize / 20..rate as usize * 7 / 20];
                let crossings = middle
                    .windows(2)
                    .filter(|pair| pair[0] <= 0.0 && pair[1] > 0.0)
                    .count();
                assert!((crossings as f32 / 0.3 - frequency).abs() < 4.0);
                let last = output.iter().rposition(|value| value.abs() > 1e-8).unwrap();
                let duration = last as f32 / rate as f32;
                assert!((duration - 0.5 / pitch_rate(tone_y)).abs() < 0.001);
            }
        }
    }

    #[test]
    fn brightness_attenuates_high_frequencies_without_adding_a_tail() {
        let frames = (0..4_801)
            .map(|i| (std::f32::consts::TAU * 5_000.0 * i as f32 / 48_000.0).sin() * 0.1)
            .collect();
        let bank = fixture(48_000, frames);
        let render = |tone_x| {
            stroke(
                bank.clone(),
                &Settings {
                    tone_x,
                    ..Settings::default()
                },
                0x1e,
                true,
            )
        };
        let dark = render(0.0);
        let bright = render(1.0);
        assert!(energy(&bright) > energy(&dark) * 8.0);
        assert!(bright[9_600..].iter().all(|&value| value == 0.0));
        assert!(dark[9_600..].iter().all(|&value| value == 0.0));
    }

    #[test]
    fn splitting_callbacks_preserves_all_samples_and_frame_accounting() {
        let bank = constant_bank();
        let settings = Settings {
            tone_x: 0.27,
            tone_y: 0.81,
            ..Settings::default()
        };
        let (mut whole, one) = setup(bank.clone(), &settings, 44_100);
        let (mut split, two) = setup(bank, &settings, 44_100);
        for shared in [one.clone(), two.clone()] {
            for scan in [0x1e, 0x27, 0x39] {
                shared.push(KeyEvent {
                    scan,
                    pressed: true,
                });
            }
        }
        let mut expected = vec![0.0_f32; 12_000];
        let mut actual = vec![0.0_f32; 12_000];
        whole.render(&mut expected, 2);
        for block in actual.chunks_mut(74) {
            split.render(block, 2);
        }
        assert_eq!(actual, expected);
        assert_eq!(one.frames.load(Ordering::Relaxed), 6_000);
        assert_eq!(two.frames.load(Ordering::Relaxed), 6_000);
    }

    #[test]
    fn gain_controls_apply_to_both_phases_and_normalization_uses_bank_gain() {
        let bank = Arc::new(SampleBank::load(Switch::CherryKc1000).unwrap());
        assert!(bank.normalization_gain.is_finite() && bank.normalization_gain > 0.0);
        let normal = stroke(bank.clone(), &Settings::default(), 0x1e, true);
        let normalized = stroke(
            bank.clone(),
            &Settings {
                normalize_volume: true,
                ..Settings::default()
            },
            0x1e,
            true,
        );
        assert!(energy(&normal) > 0.0);
        for (plain, adjusted) in normal
            .iter()
            .zip(normalized.iter())
            .filter(|(a, _)| a.abs() > 1e-5)
        {
            assert!((adjusted.atanh() - plain.atanh() * bank.normalization_gain).abs() < 1e-4);
        }
        for pressed in [true, false] {
            let bank = constant_bank();
            let baseline = stroke(bank.clone(), &Settings::default(), 0x1e, pressed);
            let quieter = stroke(
                bank.clone(),
                &Settings {
                    volume: 0.1,
                    ..Settings::default()
                },
                0x1e,
                pressed,
            );
            let feel = stroke(
                bank.clone(),
                &Settings {
                    per_key_feel: true,
                    ..Settings::default()
                },
                0x1e,
                pressed,
            );
            assert!(energy(&quieter) > 0.0 && energy(&quieter) < energy(&baseline));
            assert!(energy(&feel) > 0.0 && energy(&feel) < energy(&baseline));
            let muted = stroke(
                bank.clone(),
                &Settings {
                    volume: 0.0,
                    ..Settings::default()
                },
                0x1e,
                pressed,
            );
            assert_eq!(energy(&muted), 0.0);
        }
    }

    #[test]
    fn enter_settings_cover_both_keys_and_phases_without_changing_other_keys() {
        let bank = constant_bank();
        let normal = Settings::default();
        for pressed in [true, false] {
            let ordinary = stroke(bank.clone(), &normal, 0x1e, pressed);
            for scan in [0x1c, 0x11c] {
                let baseline = stroke(bank.clone(), &normal, scan, pressed);
                assert!(energy(&baseline) > 0.0);
                let mut changed = normal.clone();
                changed.enter.enabled = false;
                assert_eq!(energy(&stroke(bank.clone(), &changed, scan, pressed)), 0.0);
                assert_eq!(stroke(bank.clone(), &changed, 0x1e, pressed), ordinary);
                changed.enter.enabled = true;
                changed.enter.volume = 0.0;
                assert_eq!(energy(&stroke(bank.clone(), &changed, scan, pressed)), 0.0);
                changed.enter.volume = 0.5;
                assert!(energy(&stroke(bank.clone(), &changed, scan, pressed)) < energy(&baseline));
                changed.enter.volume = 1.0;
                changed.enter.tone_x = 0.0;
                changed.enter.tone_y = 1.0;
                assert_eq!(stroke(bank.clone(), &changed, scan, pressed), baseline);
                changed.enter.use_keyboard_tone = false;
                assert_ne!(stroke(bank.clone(), &changed, scan, pressed), baseline);
                assert_eq!(stroke(bank.clone(), &changed, 0x1e, pressed), ordinary);
            }
        }
    }

    #[test]
    fn live_mutes_stop_existing_voices_and_reset_survives_fast_reenable() {
        let bank = constant_bank();
        for (scan, mute_enter) in [(0x38, false), (0x1c, true), (0x11c, true)] {
            let (mut mixer, shared) = setup(bank.clone(), &Settings::default(), 48_000);
            shared.push(KeyEvent {
                scan,
                pressed: true,
            });
            let mut output = [0.0_f32; 200];
            mixer.render(&mut output, 2);
            assert!(energy(&output) > 0.0);
            let changed = Settings {
                mute_modifiers: !mute_enter,
                enter: EnterSettings {
                    enabled: !mute_enter,
                    ..EnterSettings::default()
                },
                ..Settings::default()
            };
            shared.update(&changed);
            for pressed in [false, true] {
                shared.push(KeyEvent { scan, pressed });
                mixer.render(&mut output, 2);
                assert_eq!(energy(&output), 0.0);
            }
            shared.push(KeyEvent {
                scan: 0x0f,
                pressed: true,
            });
            mixer.render(&mut output, 2);
            assert!(energy(&output) > 0.0);
            shared.push(KeyEvent {
                scan: 0x39,
                pressed: true,
            });
            shared.set_enabled(false);
            shared.push(KeyEvent {
                scan: 0x1e,
                pressed: true,
            });
            shared.set_enabled(true);
            mixer.render(&mut output, 2);
            assert_eq!(energy(&output), 0.0);
            shared.push(KeyEvent {
                scan: 0x1e,
                pressed: true,
            });
            mixer.render(&mut output, 2);
            assert!(energy(&output) > 0.0);
            shared.enabled.store(false, Ordering::Relaxed);
            mixer.render(&mut output, 2);
            assert_eq!(energy(&output), 0.0);
        }
    }

    #[test]
    fn release_setting_and_available_pack_phases_are_respected() {
        for switch in Switch::ALL {
            let bank = Arc::new(SampleBank::load(switch).unwrap());
            for rate in [44_100, 48_000, 96_000] {
                for pressed in [false, true] {
                    let (mut mixer, shared) = setup(bank.clone(), &Settings::default(), rate);
                    for scan in [0x1e, 0x39, 0x1c, 0x0e] {
                        shared.push(KeyEvent { scan, pressed });
                    }
                    let mut output = vec![0.0_f32; rate as usize / 2];
                    mixer.render(&mut output, 2);
                    assert!(
                        output
                            .iter()
                            .all(|value| value.is_finite() && value.abs() <= 1.0)
                    );
                    assert_eq!(
                        energy(&output) > 0.0,
                        pressed || switch.has_release_sounds()
                    );
                }
            }
            let muted = stroke(
                bank,
                &Settings {
                    release_sounds: false,
                    ..Settings::default()
                },
                0x1e,
                false,
            );
            assert_eq!(energy(&muted), 0.0);
        }
    }

    fn check_format<T>()
    where
        T: cpal::Sample + cpal::FromSample<f32> + PartialEq + std::fmt::Debug,
    {
        let (mut mixer, shared) = setup(constant_bank(), &Settings::default(), 48_000);
        let mut silent = vec![T::from_sample(0.7); 19];
        mixer.render(&mut silent, 6);
        assert!(silent.iter().all(|&sample| sample == T::EQUILIBRIUM));
        shared.push(KeyEvent {
            scan: 0x1e,
            pressed: true,
        });
        let mut output = vec![T::EQUILIBRIUM; 6_001];
        mixer.render(&mut output, 6);
        assert!(
            output
                .chunks_exact(6)
                .any(|frame| frame[0] != T::EQUILIBRIUM)
        );
        assert!(
            output
                .chunks_exact(6)
                .all(|frame| frame[2..].iter().all(|&value| value == T::EQUILIBRIUM))
        );
        assert_eq!(output[6_000], T::EQUILIBRIUM);
        mixer.render(&mut silent, 0);
        assert!(silent.iter().all(|&sample| sample == T::EQUILIBRIUM));
    }

    #[test]
    fn output_formats_and_partial_frames_use_the_correct_silence_value() {
        check_format::<f32>();
        check_format::<f64>();
        check_format::<i16>();
        check_format::<i32>();
        check_format::<u16>();
    }

    #[test]
    fn visualization_stays_independent_and_reports_overflow_once() {
        let shared = Shared::new(&Settings::default());
        let mut events = Vec::new();
        shared.set_visualize(true);
        shared.push(KeyEvent {
            scan: 0x1e,
            pressed: true,
        });
        assert!(!shared.take_visual_events(&mut events));
        assert_eq!(events.len(), 1);
        assert_eq!(shared.queue.len(), 1);
        shared.set_enabled(false);
        for _ in 0..QUEUE_SIZE + 5 {
            shared.push(KeyEvent {
                scan: 0x1e,
                pressed: false,
            });
        }
        assert!(shared.take_visual_events(&mut events));
        assert_eq!(events.len(), QUEUE_SIZE);
        assert!(shared.queue.is_empty());
        assert!(!shared.take_visual_events(&mut events));
        shared.set_visualize(false);
        shared.push(KeyEvent {
            scan: 0x1e,
            pressed: true,
        });
        shared.take_visual_events(&mut events);
        assert!(events.is_empty());
    }

    #[test]
    fn settings_are_sanitized_at_creation_and_update() {
        let settings = Settings {
            volume: f32::NAN,
            tone_x: -1.0,
            tone_y: f32::INFINITY,
            width: 3.0,
            home_row_softness: f32::NAN,
            enter: EnterSettings {
                volume: -1.0,
                tone_x: f32::NAN,
                tone_y: 2.0,
                ..EnterSettings::default()
            },
            ..Settings::default()
        };
        let mut clean = settings.clone();
        clean.sanitize();
        assert_eq!(clean.volume, Settings::default().volume);
        assert_eq!(clean.tone_x, 0.0);
        assert_eq!(clean.tone_y, Settings::default().tone_y);
        assert_eq!(clean.width, 1.0);
        assert_eq!(
            clean.home_row_softness,
            Settings::default().home_row_softness
        );
        assert_eq!(clean.enter.volume, 0.0);
        assert_eq!(clean.enter.tone_x, EnterSettings::default().tone_x);
        assert_eq!(clean.enter.tone_y, 1.0);
        let expected = stroke(constant_bank(), &clean, 0x1e, true);
        assert_eq!(stroke(constant_bank(), &settings, 0x1e, true), expected);
        let (mut mixer, shared) = setup(constant_bank(), &Settings::default(), 48_000);
        shared.update(&settings);
        shared.push(KeyEvent {
            scan: 0x1e,
            pressed: true,
        });
        let mut updated = vec![0.0_f32; expected.len()];
        mixer.render(&mut updated, 2);
        assert_eq!(updated, expected);
    }
}
