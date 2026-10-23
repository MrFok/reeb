use reeb::{appearance::ThemeMode, mixer::Settings, samples::Switch};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub settings: Settings,
    pub device: Option<String>,
    pub switch: Switch,
    pub enabled: bool,
    pub close_to_tray: bool,
    pub theme: ThemeMode,
}

impl Default for Saved {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            device: None,
            switch: Switch::default(),
            enabled: true,
            close_to_tray: true,
            theme: ThemeMode::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MemoryStorage(Option<String>);

    impl eframe::Storage for MemoryStorage {
        fn get_string(&self, _: &str) -> Option<String> {
            self.0.clone()
        }

        fn set_string(&mut self, _: &str, value: String) {
            self.0 = Some(value);
        }

        fn flush(&mut self) {}
    }

    #[test]
    fn original_settings_load_with_new_preferences_defaulted() {
        let storage = MemoryStorage(Some(
            "(settings:(volume:0.62,tone_x:0.2,tone_y:0.8,width:0.4,release_sounds:false),device:Some(\"Speakers\"))".to_owned(),
        ));
        let saved: Saved = eframe::get_value(&storage, eframe::APP_KEY).unwrap();
        assert_eq!(saved.settings.volume, 0.62);
        assert_eq!(saved.device.as_deref(), Some("Speakers"));
        assert!(!saved.settings.release_sounds);
        assert_eq!(saved.switch, Switch::NovelKeysCream);
        assert!(saved.enabled);
        assert!(saved.close_to_tray);
        assert_eq!(saved.theme, ThemeMode::System);
        assert!(saved.settings.enter.enabled);
        assert!(saved.settings.enter.use_keyboard_tone);
        assert_eq!(saved.settings.enter.volume, 1.0);
    }

    #[test]
    fn disabled_and_window_preferences_survive_restart() {
        let mut storage = MemoryStorage::default();
        let saved = Saved {
            enabled: false,
            close_to_tray: false,
            ..Saved::default()
        };
        eframe::set_value(&mut storage, eframe::APP_KEY, &saved);
        let restored: Saved = eframe::get_value(&storage, eframe::APP_KEY).unwrap();
        assert!(!restored.enabled);
        assert!(!restored.close_to_tray);
    }

    #[test]
    fn every_switch_selection_survives_restart() {
        for switch in Switch::ALL {
            let mut storage = MemoryStorage::default();
            let saved = Saved {
                switch,
                ..Saved::default()
            };
            eframe::set_value(&mut storage, eframe::APP_KEY, &saved);
            let restored: Saved = eframe::get_value(&storage, eframe::APP_KEY).unwrap();
            assert_eq!(restored.switch, switch);
        }
    }

    #[test]
    fn enter_controls_persist_independently_from_keyboard_tone() {
        let mut storage = MemoryStorage::default();
        let mut saved = Saved::default();
        saved.settings.enter.enabled = false;
        saved.settings.enter.volume = 0.42;
        saved.settings.enter.use_keyboard_tone = false;
        saved.settings.enter.tone_x = 0.1;
        saved.settings.enter.tone_y = 0.9;
        eframe::set_value(&mut storage, eframe::APP_KEY, &saved);
        let restored: Saved = eframe::get_value(&storage, eframe::APP_KEY).unwrap();
        assert!(!restored.settings.enter.enabled);
        assert!(!restored.settings.enter.use_keyboard_tone);
        assert_eq!(restored.settings.enter.volume, 0.42);
        assert_eq!(restored.settings.enter.tone_x, 0.1);
        assert_eq!(restored.settings.enter.tone_y, 0.9);
        assert_eq!(restored.settings.tone_x, saved.settings.tone_x);
        assert_eq!(restored.settings.tone_y, saved.settings.tone_y);
    }

    #[test]
    fn theme_modes_persist_and_unknown_modes_fall_back_to_system() {
        for theme in ThemeMode::ALL {
            let mut storage = MemoryStorage::default();
            let saved = Saved {
                theme,
                ..Default::default()
            };
            eframe::set_value(&mut storage, eframe::APP_KEY, &saved);
            let restored: Saved = eframe::get_value(&storage, eframe::APP_KEY).unwrap();
            assert_eq!(restored.theme, theme);
        }
        let storage = MemoryStorage(Some("(theme:FutureTheme,enabled:false)".to_owned()));
        let restored: Saved = eframe::get_value(&storage, eframe::APP_KEY).unwrap();
        assert_eq!(restored.theme, ThemeMode::System);
        assert!(!restored.enabled);
    }

    #[test]
    fn typing_settings_persist_and_legacy_sound_defaults_remain_compatible() {
        let mut storage = MemoryStorage::default();
        let mut saved = Saved::default();
        saved.settings.spatial_audio = false;
        saved.settings.normalize_volume = true;
        saved.settings.mute_modifiers = true;
        saved.settings.per_key_feel = true;
        saved.settings.home_row_softness = 0.37;
        eframe::set_value(&mut storage, eframe::APP_KEY, &saved);
        let restored: Saved = eframe::get_value(&storage, eframe::APP_KEY).unwrap();
        assert!(!restored.settings.spatial_audio);
        assert!(
            restored.settings.normalize_volume
                && restored.settings.mute_modifiers
                && restored.settings.per_key_feel
        );
        assert_eq!(restored.settings.home_row_softness, 0.37);
        let storage = MemoryStorage(Some("(settings:(volume:0.62))".to_owned()));
        let legacy: Saved = eframe::get_value(&storage, eframe::APP_KEY).unwrap();
        assert!(legacy.settings.spatial_audio);
        assert!(
            !legacy.settings.normalize_volume
                && !legacy.settings.mute_modifiers
                && !legacy.settings.per_key_feel
        );
    }

    #[test]
    fn unknown_switch_falls_back_to_default_without_losing_preferences() {
        let storage = MemoryStorage(Some("(switch:FutureSwitch,enabled:false)".to_owned()));
        let restored: Saved = eframe::get_value(&storage, eframe::APP_KEY).unwrap();
        assert_eq!(restored.switch, Switch::NovelKeysCream);
        assert!(!restored.enabled);
    }
}
