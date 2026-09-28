//! Read-only local DirectWrite families. Native interfaces stay in one worker.
#[cfg(any(windows, test))]
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FontName {
    pub(crate) locale: String,
    pub(crate) name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FontFamily {
    pub(crate) family: String,
    pub(crate) names: Vec<FontName>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FontsUnavailableReason {
    UnsupportedPlatform,
    EnumerationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum DesktopFonts {
    Available { families: Vec<FontFamily> },
    Unavailable { reason: FontsUnavailableReason },
}

#[cfg(any(windows, test))]
fn catalogue(families: Vec<Vec<FontName>>) -> Vec<FontFamily> {
    let mut unique: BTreeMap<String, FontFamily> = BTreeMap::new();
    for mut names in families {
        names.retain(|value| !value.name.trim().is_empty());
        let Some(canonical) = names
            .iter()
            .find(|value| value.locale.eq_ignore_ascii_case("en-US"))
            .or(names.first())
        else {
            continue;
        };
        let family = canonical.name.clone();
        let entry = unique
            .entry(family.to_lowercase())
            .or_insert_with(|| FontFamily {
                family,
                names: Vec::new(),
            });
        for name in names {
            if !entry.names.iter().any(|existing| {
                existing.locale.eq_ignore_ascii_case(&name.locale)
                    && existing.name.to_lowercase() == name.name.to_lowercase()
            }) {
                entry.names.push(name);
            }
        }
    }
    unique.into_values().collect()
}

#[cfg(windows)]
fn enumerate() -> DesktopFonts {
    use windows::Win32::Graphics::DirectWrite::{
        DWRITE_FACTORY_TYPE_SHARED, DWriteCreateFactory, IDWriteFactory3, IDWriteFontCollection1,
    };
    // SAFETY: DirectWrite creates these interfaces for this worker. Every
    // interface and UTF-16 buffer lives and is released on this worker. Only
    // owned Rust strings cross the join boundary. Downloadable fonts are off.
    let result = (|| unsafe {
        let factory: IDWriteFactory3 = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
        let mut collection: Option<IDWriteFontCollection1> = None;
        factory.GetSystemFontCollection(false, &mut collection, true)?;
        let collection = collection.ok_or_else(windows::core::Error::empty)?;
        let mut families = Vec::new();
        for index in 0..collection.GetFontFamilyCount() {
            let family = collection.GetFontFamily(index)?;
            let strings = family.GetFamilyNames()?;
            let mut names = Vec::new();
            for index in 0..strings.GetCount() {
                let mut locale = vec![0; strings.GetLocaleNameLength(index)? as usize + 1];
                let mut name = vec![0; strings.GetStringLength(index)? as usize + 1];
                strings.GetLocaleName(index, &mut locale)?;
                strings.GetString(index, &mut name)?;
                names.push(FontName {
                    locale: String::from_utf16(&locale[..locale.len() - 1])
                        .map_err(|_| windows::core::Error::empty())?,
                    name: String::from_utf16(&name[..name.len() - 1])
                        .map_err(|_| windows::core::Error::empty())?,
                });
            }
            families.push(names);
        }
        Ok::<_, windows::core::Error>(catalogue(families))
    })();
    match result {
        Ok(families) => DesktopFonts::Available { families },
        Err(_) => DesktopFonts::Unavailable {
            reason: FontsUnavailableReason::EnumerationFailed,
        },
    }
}

#[cfg(not(windows))]
fn enumerate() -> DesktopFonts {
    DesktopFonts::Unavailable {
        reason: FontsUnavailableReason::UnsupportedPlatform,
    }
}

#[tauri::command]
pub(crate) async fn desktop_fonts_list() -> DesktopFonts {
    tauri::async_runtime::spawn_blocking(enumerate)
        .await
        .unwrap_or(DesktopFonts::Unavailable {
            reason: FontsUnavailableReason::EnumerationFailed,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn name(locale: &str, name: &str) -> FontName {
        FontName {
            locale: locale.into(),
            name: name.into(),
        }
    }

    #[test]
    fn canonical_names_aliases_deduplication_and_no_result_cap() {
        let result = catalogue(vec![
            vec![name("zh-CN", "中文字体"), name("en-US", "Host Family")],
            vec![name("en-us", "HOST FAMILY"), name("zh-TW", "字體")],
            vec![name("ja-JP", "日本語")],
            vec![],
        ]);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].family, "Host Family");
        assert_eq!(result[0].names.len(), 3);
        assert_eq!(result[1].family, "日本語");
        let large = catalogue(
            (0..2000)
                .map(|i| vec![name("en-US", &format!("Family {i:04}"))])
                .collect(),
        );
        assert_eq!(large.len(), 2000);
        assert_eq!(large[1999].family, "Family 1999");
    }

    #[test]
    fn payloads_are_closed_and_contain_names_only() {
        for value in [
            serde_json::json!({"status":"available","families":[],"path":"x"}),
            serde_json::json!({"status":"unavailable","reason":"unknown"}),
            serde_json::json!({"status":"available","families":[{"family":"A","names":[],"bytes":[]}]}),
        ] {
            assert!(serde_json::from_value::<DesktopFonts>(value).is_err());
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn unsupported_platform_is_explicit() {
        assert_eq!(
            enumerate(),
            DesktopFonts::Unavailable {
                reason: FontsUnavailableReason::UnsupportedPlatform
            }
        );
    }
}
