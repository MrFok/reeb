// Apple virtual key codes are physical positions, independent of the text layout.
// Convert to the existing sound/visualizer scan identifiers at the input boundary.
pub fn scan(code: u16) -> Option<u16> {
    Some(match code {
        0 => 0x1e,
        1 => 0x1f,
        2 => 0x20,
        3 => 0x21,
        4 => 0x23,
        5 => 0x22,
        6 => 0x2c,
        7 => 0x2d,
        8 => 0x2e,
        9 => 0x2f,
        10 => 0x56,
        11 => 0x30,
        12 => 0x10,
        13 => 0x11,
        14 => 0x12,
        15 => 0x13,
        16 => 0x15,
        17 => 0x14,
        18 => 0x02,
        19 => 0x03,
        20 => 0x04,
        21 => 0x05,
        22 => 0x07,
        23 => 0x06,
        24 => 0x0d,
        25 => 0x0a,
        26 => 0x08,
        27 => 0x0c,
        28 => 0x09,
        29 => 0x0b,
        30 => 0x1b,
        31 => 0x18,
        32 => 0x16,
        33 => 0x1a,
        34 => 0x17,
        35 => 0x19,
        36 => 0x1c,
        37 => 0x26,
        38 => 0x24,
        39 => 0x28,
        40 => 0x25,
        41 => 0x27,
        42 => 0x2b,
        43 => 0x33,
        44 => 0x35,
        45 => 0x31,
        46 => 0x32,
        47 => 0x34,
        48 => 0x0f,
        49 => 0x39,
        50 => 0x29,
        51 => 0x0e,
        53 => 0x01,
        54 => 0x15c,
        55 => 0x15b,
        56 => 0x2a,
        57 => 0x3a,
        58 => 0x38,
        59 => 0x1d,
        60 => 0x36,
        61 => 0x138,
        62 => 0x11d,
        65 => 0x53,
        67 => 0x37,
        69 => 0x4e,
        71 => 0x45,
        75 => 0x135,
        76 => 0x11c,
        78 => 0x4a,
        81 => 0x59,
        82 => 0x52,
        83 => 0x4f,
        84 => 0x50,
        85 => 0x51,
        86 => 0x4b,
        87 => 0x4c,
        88 => 0x4d,
        89 => 0x47,
        91 => 0x48,
        92 => 0x49,
        96 => 0x3f,
        97 => 0x40,
        98 => 0x41,
        99 => 0x3d,
        100 => 0x42,
        101 => 0x43,
        103 => 0x57,
        109 => 0x44,
        111 => 0x58,
        114 => 0x152,
        115 => 0x147,
        116 => 0x149,
        117 => 0x153,
        118 => 0x3e,
        119 => 0x14f,
        120 => 0x3c,
        121 => 0x151,
        122 => 0x3b,
        123 => 0x14b,
        124 => 0x14d,
        125 => 0x150,
        126 => 0x148,
        _ => return None,
    })
}

// NX_SUBTYPE_AUX_CONTROL_BUTTONS data1: key type in the upper 16 bits,
// key state in bits 8..15. Types are defined by IOKit hidsystem/ev_keymap.h.
pub fn media_key(data: u32) -> Option<(u16, bool)> {
    let scan = match data >> 16 {
        3 => 0x3b,       // Brightness down: F1.
        2 => 0x3c,       // Brightness up: F2.
        13 => 0x3e,      // Launch panel: F4 on older Apple keyboards.
        22 => 0x3f,      // Keyboard illumination down: F5.
        21 => 0x40,      // Keyboard illumination up: F6.
        18 | 20 => 0x41, // Previous / rewind: F7.
        16 => 0x42,      // Play / pause: F8.
        17 | 19 => 0x43, // Next / fast forward: F9.
        7 => 0x44,       // Mute: F10.
        1 => 0x57,       // Volume down: F11.
        0 => 0x58,       // Volume up: F12.
        _ => return None,
    };
    let pressed = match (data >> 8) & 0xff {
        0x0a => true,
        0x0b => false,
        _ => return None,
    };
    Some((scan, pressed))
}

// Device-dependent left/right modifier bits preserve overlapping modifier keys.
pub fn modifier_pressed(code: u16, flags: u64) -> Option<bool> {
    let mask = match code {
        54 => 0x10,
        55 => 0x08,
        56 => 0x02,
        57 => 1 << 16,
        58 => 0x20,
        59 => 0x01,
        60 => 0x04,
        61 => 0x40,
        62 => 0x2000,
        _ => return None,
    };
    Some(flags & mask != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::{self, Group};

    #[test]
    fn media_keys_decode_both_phases_and_ignore_unrelated_system_events() {
        for (kind, scan) in [
            (3, 0x3b),
            (2, 0x3c),
            (13, 0x3e),
            (22, 0x3f),
            (21, 0x40),
            (18, 0x41),
            (20, 0x41),
            (16, 0x42),
            (17, 0x43),
            (19, 0x43),
            (7, 0x44),
            (1, 0x57),
            (0, 0x58),
        ] {
            assert_eq!(media_key((kind << 16) | 0x0a00), Some((scan, true)));
            assert_eq!(media_key((kind << 16) | 0x0a01), Some((scan, true)));
            assert_eq!(media_key((kind << 16) | 0x0b00), Some((scan, false)));
            assert_eq!(media_key((kind << 16) | 0x0900), None);
        }
        assert_eq!(media_key((6 << 16) | 0x0a00), None); // Power.
        assert_eq!(media_key((14 << 16) | 0x0a00), None); // Eject.
        assert_eq!(media_key((0xffff << 16) | 0x0a00), None);
    }

    #[test]
    fn physical_keys_keep_sound_groups_and_navigation_positions() {
        assert_eq!(scan(0), Some(0x1e));
        for (code, group) in [
            (36, Group::Enter),
            (76, Group::Enter),
            (49, Group::Space),
            (51, Group::Backspace),
        ] {
            assert_eq!(keys::group(scan(code).unwrap()), group);
        }
        assert_eq!(scan(123), Some(0x14b));
        assert_eq!(scan(126), Some(0x148));
        assert_eq!(scan(255), None);
        let keys: Vec<_> = (0..128).filter_map(scan).collect();
        assert_eq!(
            keys.iter().collect::<std::collections::HashSet<_>>().len(),
            keys.len()
        );
    }

    #[test]
    fn left_and_right_modifiers_release_independently() {
        assert_eq!(modifier_pressed(56, 0x06), Some(true));
        assert_eq!(modifier_pressed(60, 0x06), Some(true));
        assert_eq!(modifier_pressed(56, 0x04), Some(false));
        assert_eq!(modifier_pressed(60, 0x04), Some(true));
        for code in [54, 55, 56, 57, 58, 59, 60, 61, 62] {
            assert!(keys::modifier(scan(code).unwrap()));
            assert_eq!(modifier_pressed(code, 0), Some(false));
        }
        assert_eq!(modifier_pressed(0, 0), None);
    }
}
