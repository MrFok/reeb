#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Generic,
    Space,
    Enter,
    Backspace,
}

#[derive(Clone, Copy, Debug)]
pub struct KeyEvent {
    pub scan: u16,
    pub pressed: bool,
}

pub fn group(scan: u16) -> Group {
    match scan {
        0x39 => Group::Space,
        0x1c | 0x11c => Group::Enter,
        0x0e => Group::Backspace,
        _ => Group::Generic,
    }
}

fn ansi_center(scan: u16) -> Option<(f32, f32)> {
    let center = match scan {
        0x29 => (0.5, 0.0),
        0x02..=0x0d => (1.5 + f32::from(scan - 0x02), 0.0),
        0x0e => (14.0, 0.0),
        0x0f => (0.75, 1.0),
        0x10..=0x1b => (2.0 + f32::from(scan - 0x10), 1.0),
        0x2b => (14.25, 1.0),
        0x3a => (0.875, 2.0),
        0x1e..=0x28 => (2.25 + f32::from(scan - 0x1e), 2.0),
        0x1c => (13.875, 2.0),
        0x2a => (1.125, 3.0),
        0x2c..=0x35 => (2.75 + f32::from(scan - 0x2c), 3.0),
        0x36 => (13.625, 3.0),
        0x1d => (0.625, 4.0),
        0x15b => (1.875, 4.0),
        0x38 => (3.125, 4.0),
        0x39 => (6.875, 4.0),
        0x138 => (10.625, 4.0),
        0x15c => (11.875, 4.0),
        0x15d => (13.125, 4.0),
        0x11d => (14.375, 4.0),
        0x01 => (0.5, -1.5),
        0x3b..=0x44 | 0x57..=0x58 => {
            let index = if scan <= 0x44 {
                scan - 0x3b
            } else {
                scan - 0x57 + 10
            };
            (2.5 + f32::from(index) + 0.5 * f32::from(index / 4), -1.5)
        }
        0x137 => (16.0, -1.5),
        0x46 => (17.0, -1.5),
        0x145 => (18.0, -1.5),
        0x152 => (16.0, 0.0),
        0x147 => (17.0, 0.0),
        0x149 => (18.0, 0.0),
        0x153 => (16.0, 1.0),
        0x14f => (17.0, 1.0),
        0x151 => (18.0, 1.0),
        0x148 => (17.0, 3.0),
        0x14b => (16.0, 4.0),
        0x150 => (17.0, 4.0),
        0x14d => (18.0, 4.0),
        0x45 => (19.5, 0.0),
        0x135 => (20.5, 0.0),
        0x37 => (21.5, 0.0),
        0x4a => (22.5, 0.0),
        0x47..=0x49 => (19.5 + f32::from(scan - 0x47), 1.0),
        0x4e => (22.5, 1.5),
        0x4b..=0x4d => (19.5 + f32::from(scan - 0x4b), 2.0),
        0x4f..=0x51 => (19.5 + f32::from(scan - 0x4f), 3.0),
        0x11c => (22.5, 3.5),
        0x52 => (20.0, 4.0),
        0x53 => (21.5, 4.0),
        _ => return None,
    };
    Some(center)
}

pub fn pan(scan: u16) -> f32 {
    ansi_center(scan).map_or(0.0, |(x, _)| ((x - 7.5) / 7.5).clamp(-1.0, 1.0))
}

pub fn modifier(scan: u16) -> bool {
    matches!(scan, 0x01 | 0x1d | 0x11d | 0x2a | 0x36 | 0x38 | 0x138 | 0x3a | 0x15b..=0x15d | 0x3b..=0x44 | 0x57 | 0x58)
}

pub fn feel(scan: u16, softness: f32) -> f32 {
    let Some((x, y)) = ansi_center(scan) else {
        return 1.0;
    };
    let softness = if softness.is_finite() {
        softness.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let home_columns = [2.25_f32, 3.25, 4.25, 5.25, 8.25, 9.25, 10.25, 11.25];
    let distance_squared = home_columns
        .into_iter()
        .map(|home_x| (x - home_x).powi(2) + (y - 2.0).powi(2))
        .fold(f32::INFINITY, f32::min);
    let reach = (distance_squared.sqrt() / 2.0).min(1.0);
    1.0 + 0.15 * reach - 0.35 * softness * (-distance_squared).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ansi_rows_have_uniform_spacing_and_physical_stagger() {
        for row in [0x02..=0x0d, 0x10..=0x1b, 0x1e..=0x28, 0x2c..=0x35] {
            let keys: Vec<_> = row.collect();
            for pair in keys.windows(2) {
                assert!((pan(pair[1]) - pan(pair[0]) - 1.0 / 7.5).abs() < 1e-6);
            }
        }
        assert!((pan(0x1e) - pan(0x10) - 0.25 / 7.5).abs() < 1e-6);
        assert!((pan(0x2c) - pan(0x1e) - 0.5 / 7.5).abs() < 1e-6);
        assert!(pan(0x1e) < 0.0 && pan(0x27) > 0.0);
        assert_eq!(pan(0xffff), 0.0);
        for scan in 0..=0x1ff {
            assert!((-1.0..=1.0).contains(&pan(scan)));
        }
    }

    #[test]
    fn feel_is_bounded_and_varies_with_reach_and_softness() {
        for scan in [0x1e, 0x1f, 0x20, 0x21, 0x24, 0x25, 0x26, 0x27] {
            assert!((feel(scan, 1.0) - 0.65).abs() < 1e-6);
            assert_eq!(feel(scan, 0.0), 1.0);
            assert!(feel(scan, 0.5) > feel(scan, 1.0));
        }
        assert!(feel(0x1e, 1.0) < feel(0x10, 1.0));
        assert!(feel(0x10, 1.0) < feel(0x02, 1.0));
        for scan in 0..=0x1ff {
            for softness in [-1.0, 0.0, 0.5, 1.0, 2.0, f32::NAN] {
                assert!((0.64999..=1.15001).contains(&feel(scan, softness)));
            }
        }
        assert_eq!(feel(0xffff, 1.0), 1.0);
    }

    #[test]
    fn event_groups_and_modifier_policy_keep_existing_contracts() {
        assert_eq!(group(0x1c), Group::Enter);
        assert_eq!(group(0x11c), Group::Enter);
        assert_eq!(group(0x39), Group::Space);
        assert_eq!(group(0x0e), Group::Backspace);
        assert_eq!(group(0x1e), Group::Generic);
        for scan in [
            0x01, 0x1d, 0x11d, 0x2a, 0x36, 0x38, 0x138, 0x3a, 0x15b, 0x15c, 0x15d, 0x3b, 0x44,
            0x57, 0x58,
        ] {
            assert!(modifier(scan));
        }
        for scan in [0x0f, 0x1e, 0x39, 0x1c, 0x11c, 0x148] {
            assert!(!modifier(scan));
        }
    }
}
