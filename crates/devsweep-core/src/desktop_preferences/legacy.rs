//! Strict read-only V1 boundary. New theme variants cannot enter this schema.
use super::*;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Theme {
    Dark,
    Light,
    System,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Font {
    System,
    SegoeUi,
    MicrosoftYaheiUi,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DesktopPreferencesV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u8,
    theme: Theme,
    font_family: Font,
    #[serde(deserialize_with = "text_scale")]
    text_scale_percent: u8,
    motion: DesktopMotion,
    #[serde(deserialize_with = "planet_fps")]
    planet_fps: u8,
    #[serde(deserialize_with = "status_interval")]
    status_interval_seconds: u8,
    #[serde(deserialize_with = "status_process_limit")]
    status_process_limit: u8,
    #[serde(deserialize_with = "hud_interval")]
    hud_interval_seconds: u8,
}

fn version<'de, D: Deserializer<'de>>(decoder: D) -> Result<u8, D::Error> {
    choice(decoder, "schema_version", &[1])
}

impl From<DesktopPreferencesV1> for DesktopPreferencesV2 {
    fn from(value: DesktopPreferencesV1) -> Self {
        debug_assert_eq!(value.schema_version, 1);
        Self {
            schema_version: 2,
            theme: match value.theme {
                Theme::Dark => DesktopTheme::Dark,
                Theme::Light => DesktopTheme::Light,
                Theme::System => DesktopTheme::System,
            },
            font: match value.font_family {
                Font::System => DesktopFont::System {},
                Font::SegoeUi => DesktopFont::Installed {
                    family: "Segoe UI".into(),
                },
                Font::MicrosoftYaheiUi => DesktopFont::Installed {
                    family: "Microsoft YaHei UI".into(),
                },
            },
            text_scale_percent: value.text_scale_percent,
            motion: value.motion,
            planet_fps: value.planet_fps,
            status_interval_seconds: value.status_interval_seconds,
            status_process_limit: value.status_process_limit,
            hud_interval_seconds: value.hud_interval_seconds,
        }
    }
}
