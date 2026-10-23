use crate::keys::Group;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

mod packs;

const TARGET_PRESS_RMS: f32 = 0.02;

type EmbeddedSample = (&'static str, &'static [u8]);

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Switch {
    Alpaca,
    GateronBlackInk,
    GateronRedInk,
    HolyPanda,
    CherryMxBlack,
    CherryMxBlue,
    CherryMxBrown,
    KailhBoxNavy,
    BucklingSpring,
    BlueAlps,
    Topre,
    TurquoiseTealios,
    CherryMxBrownFoxfire,
    CherryKc1000,
    MacBookAirM2,
    #[default]
    #[serde(other)]
    NovelKeysCream,
}

impl Switch {
    pub const ALL: [Self; 16] = [
        Self::NovelKeysCream,
        Self::Alpaca,
        Self::GateronBlackInk,
        Self::GateronRedInk,
        Self::HolyPanda,
        Self::CherryMxBlack,
        Self::CherryMxBlue,
        Self::CherryMxBrown,
        Self::KailhBoxNavy,
        Self::BucklingSpring,
        Self::BlueAlps,
        Self::Topre,
        Self::TurquoiseTealios,
        Self::CherryMxBrownFoxfire,
        Self::CherryKc1000,
        Self::MacBookAirM2,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::NovelKeysCream => "NovelKeys Cream",
            Self::Alpaca => "Alpaca",
            Self::GateronBlackInk => "Gateron Black Ink",
            Self::GateronRedInk => "Gateron Red Ink",
            Self::HolyPanda => "Holy Panda",
            Self::CherryMxBlack => "Cherry MX Black",
            Self::CherryMxBlue => "Cherry MX Blue",
            Self::CherryMxBrown => "Cherry MX Brown",
            Self::KailhBoxNavy => "Kailh Box Navy",
            Self::BucklingSpring => "Buckling Spring",
            Self::BlueAlps => "SKCM Blue Alps",
            Self::Topre => "Topre",
            Self::TurquoiseTealios => "Turquoise Tealios",
            Self::CherryMxBrownFoxfire => "Cherry MX Brown · Foxfire",
            Self::CherryKc1000 => "Cherry KC 1000",
            Self::MacBookAirM2 => "MacBook Air M2",
        }
    }

    pub fn has_release_sounds(self) -> bool {
        !matches!(self, Self::CherryKc1000 | Self::MacBookAirM2)
    }
}

pub struct Sample {
    pub frames: Vec<f32>,
    pub rate: u32,
}

pub struct SampleBank {
    pub normalization_gain: f32,
    pub samples: Vec<Sample>,
    sets: [Vec<usize>; 8],
}

impl SampleBank {
    pub fn bundled() -> Result<Self, String> {
        Self::load(Switch::default())
    }

    pub fn load(switch: Switch) -> Result<Self, String> {
        let mut sets: [Vec<usize>; 8] = Default::default();
        let samples = packs::files(switch)
            .iter()
            .enumerate()
            .map(|(index, (name, bytes))| {
                let (event, key) = name.split_once('-').ok_or("Invalid bundled sound name")?;
                let group = match key.split('_').next().unwrap_or_default() {
                    "generic" => 0,
                    "space" => 2,
                    "enter" => 4,
                    "backspace" => 6,
                    _ => return Err(format!("Unknown bundled sound group: {name}")),
                };
                let set = match event {
                    "press" => group,
                    "release" => group + 1,
                    _ => return Err(format!("Unknown bundled sound event: {name}")),
                };
                let mut reader =
                    hound::WavReader::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
                let spec = reader.spec();
                if spec.channels != 1
                    || spec.bits_per_sample != 16
                    || spec.sample_format != hound::SampleFormat::Int
                    || spec.sample_rate == 0
                {
                    return Err("Bundled sound has an unsupported WAV format".to_owned());
                }
                let frames = reader
                    .samples::<i16>()
                    .map(|sample| sample.map(|v| f32::from(v) / 32768.0))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| error.to_string())?;
                if frames.len() < 2 {
                    return Err("Bundled sound is empty".to_owned());
                }
                sets[set].push(index);
                Ok(Sample {
                    frames,
                    rate: spec.sample_rate,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        if sets[0].is_empty() {
            return Err("Bundled pack has no regular key press sounds".to_owned());
        }
        let energy = sets[0]
            .iter()
            .map(|&index| {
                let frames = &samples[index].frames;
                let first = frames
                    .iter()
                    .position(|value| value.abs() > 0.001)
                    .unwrap_or(0);
                let end = frames
                    .iter()
                    .rposition(|value| value.abs() > 0.001)
                    .map_or(frames.len(), |index| index + 1);
                let active = &frames[first..end];
                active
                    .iter()
                    .map(|&value| f64::from(value).powi(2))
                    .sum::<f64>()
                    / active.len().max(1) as f64
            })
            .sum::<f64>()
            / sets[0].len() as f64;
        let normalization_gain = if energy > 0.0 {
            (TARGET_PRESS_RMS / energy.sqrt() as f32).clamp(0.05, 4.0)
        } else {
            1.0
        };
        for set in 2..sets.len() {
            if sets[set].is_empty() {
                sets[set] = sets[set % 2].clone();
            }
        }
        Ok(Self {
            samples,
            sets,
            normalization_gain,
        })
    }

    pub fn choose(&self, group: Group, pressed: bool, cursors: &mut [usize; 8]) -> Option<usize> {
        let base = match group {
            Group::Generic => 0,
            Group::Space => 2,
            Group::Enter => 4,
            Group::Backspace => 6,
        };
        let set = base + usize::from(!pressed);
        if self.sets[set].is_empty() {
            return None;
        }
        let cursor = &mut cursors[set];
        let sample = self.sets[set][*cursor % self.sets[set].len()];
        *cursor = cursor.wrapping_add(1);
        Some(sample)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_packs_decode_and_select_valid_variations() {
        for switch in Switch::ALL {
            let bank = SampleBank::load(switch).unwrap();
            assert!(bank.normalization_gain.is_finite());
            assert!((0.05..=4.0).contains(&bank.normalization_gain));
            assert!(
                bank.samples
                    .iter()
                    .all(|s| s.rate == 44100 && s.frames.len() > 1)
            );
            for group in [Group::Generic, Group::Space, Group::Enter, Group::Backspace] {
                let mut cursors = [0; 8];
                for _ in 0..40 {
                    let sample = bank.choose(group, true, &mut cursors).unwrap();
                    assert!(sample < bank.samples.len());
                    assert_eq!(
                        bank.choose(group, false, &mut cursors).is_some(),
                        switch.has_release_sounds()
                    );
                }
            }
        }
    }

    #[test]
    fn complete_keystrokes_cycle_without_duplicate_releases() {
        let bank = SampleBank::load(Switch::CherryKc1000).unwrap();
        let mut cursors = [0; 8];
        let first: Vec<_> = (0..32)
            .map(|_| bank.choose(Group::Generic, true, &mut cursors).unwrap())
            .collect();
        assert_eq!(
            first.iter().collect::<std::collections::HashSet<_>>().len(),
            32
        );
        assert_eq!(
            bank.choose(Group::Generic, true, &mut cursors),
            Some(first[0])
        );
        assert!(bank.choose(Group::Generic, false, &mut cursors).is_none());
    }

    #[test]
    fn special_keys_fall_back_to_their_own_pack() {
        let bank = SampleBank::load(Switch::CherryMxBrownFoxfire).unwrap();
        for group in [Group::Space, Group::Enter, Group::Backspace] {
            assert_eq!(
                bank.choose(group, true, &mut [0; 8]),
                bank.choose(Group::Generic, true, &mut [0; 8])
            );
            assert_eq!(
                bank.choose(group, false, &mut [0; 8]),
                bank.choose(Group::Generic, false, &mut [0; 8])
            );
        }
    }
}
