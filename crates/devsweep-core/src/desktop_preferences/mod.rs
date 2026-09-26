//! Desktop-only preferences. The shared presentation language has its own store.

mod store;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Deserializer, Serialize, de};

pub use store::{
    DesktopPreferencesError, desktop_preferences_path, load_desktop_preferences,
    update_desktop_preferences,
};

const TEXT_SCALES: &[u8] = &[100, 110, 125];
const PLANET_FRAME_CAPS: &[u8] = &[15, 30];
const STATUS_INTERVALS: &[u8] = &[1, 2, 5, 10, 30, 60];
const STATUS_PROCESS_LIMITS: &[u8] = &[5, 15, 30, 50, 100];
const HUD_INTERVALS: &[u8] = &[2, 5, 10];

/// Closed main-window and HUD theme choices.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopTheme {
    /// Dark semantic palette.
    #[default]
    Dark,
    /// Light semantic palette.
    Light,
    /// Follow the operating-system palette.
    System,
}

/// Local font presets; no file paths or arbitrary CSS enter storage.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopFontFamily {
    /// Current system UI stack with Chinese and English fallbacks.
    #[default]
    System,
    /// Segoe UI with local fallbacks.
    SegoeUi,
    /// Microsoft YaHei UI with local fallbacks.
    MicrosoftYaheiUi,
}

/// Application motion preference. OS reduced motion always takes precedence.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopMotion {
    /// Follow the operating-system motion preference.
    #[default]
    System,
    /// Disable decorative animation.
    Reduced,
}

/// Exact V1 desktop preference document and committed preference value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopPreferencesV1 {
    /// Only schema version 1 is supported.
    #[serde(deserialize_with = "schema_version")]
    pub schema_version: u8,
    /// Theme applied in the main window and HUD after commit.
    pub theme: DesktopTheme,
    /// Local UI font preset.
    pub font_family: DesktopFontFamily,
    /// UI text scale: 100, 110, or 125 percent.
    #[serde(deserialize_with = "text_scale")]
    pub text_scale_percent: u8,
    /// Application reduced-motion choice.
    pub motion: DesktopMotion,
    /// Planet frame cap: 15 or 30 FPS.
    #[serde(deserialize_with = "planet_fps")]
    pub planet_fps: u8,
    /// Status live cadence: 1, 2, 5, 10, 30, or 60 seconds.
    #[serde(deserialize_with = "status_interval")]
    pub status_interval_seconds: u8,
    /// Returned Status process rows: 5, 15, 30, 50, or 100.
    #[serde(deserialize_with = "status_process_limit")]
    pub status_process_limit: u8,
    /// HUD cadence on its next show: 2, 5, or 10 seconds.
    #[serde(deserialize_with = "hud_interval")]
    pub hud_interval_seconds: u8,
}

impl Default for DesktopPreferencesV1 {
    fn default() -> Self {
        Self {
            schema_version: 1,
            theme: DesktopTheme::Dark,
            font_family: DesktopFontFamily::System,
            text_scale_percent: 100,
            motion: DesktopMotion::System,
            planet_fps: 30,
            status_interval_seconds: 2,
            status_process_limit: 15,
            hud_interval_seconds: 2,
        }
    }
}

/// One field update or one reset transaction. A reset has no value field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "field", rename_all = "snake_case", deny_unknown_fields)]
pub enum DesktopPreferencesPatch {
    /// Set the semantic palette.
    Theme { value: DesktopTheme },
    /// Set the local font preset.
    FontFamily { value: DesktopFontFamily },
    /// Set the bounded text scale.
    TextScalePercent {
        #[serde(deserialize_with = "text_scale")]
        value: u8,
    },
    /// Set the motion preference.
    Motion { value: DesktopMotion },
    /// Set the Planet frame cap.
    PlanetFps {
        #[serde(deserialize_with = "planet_fps")]
        value: u8,
    },
    /// Set the Status cadence in seconds.
    StatusIntervalSeconds {
        #[serde(deserialize_with = "status_interval")]
        value: u8,
    },
    /// Set the returned Status row limit.
    StatusProcessLimit {
        #[serde(deserialize_with = "status_process_limit")]
        value: u8,
    },
    /// Set the next HUD-show cadence.
    HudIntervalSeconds {
        #[serde(deserialize_with = "hud_interval")]
        value: u8,
    },
    /// Reset theme, font family, and text scale.
    ResetAppearance {},
    /// Reset motion, frame cap, Status inputs, and HUD cadence.
    ResetPerformance {},
}

impl DesktopPreferencesV1 {
    fn patched(mut self, patch: DesktopPreferencesPatch) -> Result<Self, DesktopPreferencesError> {
        match patch {
            DesktopPreferencesPatch::Theme { value } => self.theme = value,
            DesktopPreferencesPatch::FontFamily { value } => self.font_family = value,
            DesktopPreferencesPatch::TextScalePercent { value } => self.text_scale_percent = value,
            DesktopPreferencesPatch::Motion { value } => self.motion = value,
            DesktopPreferencesPatch::PlanetFps { value } => self.planet_fps = value,
            DesktopPreferencesPatch::StatusIntervalSeconds { value } => {
                self.status_interval_seconds = value;
            }
            DesktopPreferencesPatch::StatusProcessLimit { value } => {
                self.status_process_limit = value
            }
            DesktopPreferencesPatch::HudIntervalSeconds { value } => {
                self.hud_interval_seconds = value
            }
            DesktopPreferencesPatch::ResetAppearance {} => {
                let defaults = Self::default();
                self.theme = defaults.theme;
                self.font_family = defaults.font_family;
                self.text_scale_percent = defaults.text_scale_percent;
            }
            DesktopPreferencesPatch::ResetPerformance {} => {
                let defaults = Self::default();
                self.motion = defaults.motion;
                self.planet_fps = defaults.planet_fps;
                self.status_interval_seconds = defaults.status_interval_seconds;
                self.status_process_limit = defaults.status_process_limit;
                self.hud_interval_seconds = defaults.hud_interval_seconds;
            }
        }
        // Rust callers also receive validation; constructing a numeric patch
        // directly must not bypass the closed JSON decoder.
        for (field, value, choices) in [
            ("text_scale_percent", self.text_scale_percent, TEXT_SCALES),
            ("planet_fps", self.planet_fps, PLANET_FRAME_CAPS),
            (
                "status_interval_seconds",
                self.status_interval_seconds,
                STATUS_INTERVALS,
            ),
            (
                "status_process_limit",
                self.status_process_limit,
                STATUS_PROCESS_LIMITS,
            ),
            (
                "hud_interval_seconds",
                self.hud_interval_seconds,
                HUD_INTERVALS,
            ),
        ] {
            if !choices.contains(&value) {
                return Err(DesktopPreferencesError::InvalidPatch { field, value });
            }
        }
        Ok(self)
    }
}

fn choice<'de, D: Deserializer<'de>>(
    decoder: D,
    field: &str,
    choices: &[u8],
) -> Result<u8, D::Error> {
    let value = u8::deserialize(decoder)?;
    if choices.contains(&value) {
        Ok(value)
    } else {
        Err(de::Error::custom(format!("unsupported {field}: {value}")))
    }
}

fn schema_version<'de, D: Deserializer<'de>>(decoder: D) -> Result<u8, D::Error> {
    choice(decoder, "schema_version", &[1])
}

fn text_scale<'de, D: Deserializer<'de>>(decoder: D) -> Result<u8, D::Error> {
    choice(decoder, "text_scale_percent", TEXT_SCALES)
}

fn planet_fps<'de, D: Deserializer<'de>>(decoder: D) -> Result<u8, D::Error> {
    choice(decoder, "planet_fps", PLANET_FRAME_CAPS)
}

fn status_interval<'de, D: Deserializer<'de>>(decoder: D) -> Result<u8, D::Error> {
    choice(decoder, "status_interval_seconds", STATUS_INTERVALS)
}

fn status_process_limit<'de, D: Deserializer<'de>>(decoder: D) -> Result<u8, D::Error> {
    choice(decoder, "status_process_limit", STATUS_PROCESS_LIMITS)
}

fn hud_interval<'de, D: Deserializer<'de>>(decoder: D) -> Result<u8, D::Error> {
    choice(decoder, "hud_interval_seconds", HUD_INTERVALS)
}
