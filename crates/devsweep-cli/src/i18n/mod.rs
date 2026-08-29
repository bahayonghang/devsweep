use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::OnceLock,
};

const EN_JSON: &str = include_str!("../../../../resources/i18n/en.json");
const ZH_CN_JSON: &str = include_str!("../../../../resources/i18n/zh-CN.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Locale {
    En,
    ZhCn,
}

impl Locale {
    #[allow(dead_code)]
    pub(crate) const fn tag(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::ZhCn => "zh-CN",
        }
    }

    pub(crate) fn from_presentation_language_tag(tag: &str) -> Option<Self> {
        match tag {
            "en" => Some(Self::En),
            "zh-CN" => Some(Self::ZhCn),
            _ => None,
        }
    }
}

pub(crate) fn resolve_locale(
    explicit: Option<Locale>,
    windows_user_locale: Option<&str>,
) -> Locale {
    explicit
        .or_else(|| windows_user_locale.and_then(supported_windows_locale))
        .unwrap_or(Locale::En)
}

#[cfg(windows)]
pub(crate) fn windows_user_locale() -> Option<String> {
    const LOCALE_NAME_MAX_LENGTH: usize = 85;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetUserDefaultLocaleName(locale_name: *mut u16, locale_name_count: i32) -> i32;
    }

    let mut buffer = [0_u16; LOCALE_NAME_MAX_LENGTH];
    // SAFETY: `buffer` is writable for the advertised element count and the
    // Windows API always writes a terminating NUL on success.
    let written = unsafe {
        GetUserDefaultLocaleName(
            buffer.as_mut_ptr(),
            i32::try_from(buffer.len()).expect("locale buffer length fits i32"),
        )
    };
    if written <= 1 {
        return None;
    }
    let content_length = usize::try_from(written - 1).ok()?;
    String::from_utf16(buffer.get(..content_length)?).ok()
}

#[cfg(not(windows))]
pub(crate) fn windows_user_locale() -> Option<String> {
    std::env::var("LC_ALL")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| std::env::var("LANG").ok())
        .map(|value| value.split('.').next().unwrap_or(&value).to_string())
}

fn supported_windows_locale(tag: &str) -> Option<Locale> {
    let normalized = tag.replace('_', "-");
    if ["zh-CN", "zh-SG", "zh-Hans", "zh-Hans-CN", "zh-Hans-SG"]
        .iter()
        .any(|supported| normalized.eq_ignore_ascii_case(supported))
    {
        return Some(Locale::ZhCn);
    }
    if normalized.eq_ignore_ascii_case("en")
        || normalized
            .get(..3)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("en-"))
    {
        return Some(Locale::En);
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Message {
    forms: BTreeMap<String, String>,
    placeholders: BTreeSet<String>,
    count: CountKind,
    accelerator: Option<char>,
    group: Option<String>,
    truncation: TruncationContract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CountKind {
    None,
    Cardinal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TruncationContract {
    Never,
    UserDataVisualOnly,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) struct MessageMetadata<'a> {
    pub(crate) placeholders: &'a BTreeSet<String>,
    pub(crate) count: CountKind,
    pub(crate) accelerator: Option<char>,
    pub(crate) group: Option<&'a str>,
    pub(crate) truncation: TruncationContract,
}

#[derive(Debug, Clone)]
pub(crate) struct Catalogue {
    locale: Locale,
    messages: BTreeMap<String, Message>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CatalogueError(String);

impl fmt::Display for CatalogueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for CatalogueError {}

pub(crate) fn catalogue(locale: Locale) -> &'static Catalogue {
    static EN: OnceLock<Catalogue> = OnceLock::new();
    static ZH_CN: OnceLock<Catalogue> = OnceLock::new();
    match locale {
        Locale::En => EN.get_or_init(|| {
            Catalogue::parse(EN_JSON).expect("the embedded English catalogue must be valid")
        }),
        Locale::ZhCn => ZH_CN.get_or_init(|| {
            Catalogue::parse(ZH_CN_JSON)
                .expect("the embedded Simplified-Chinese catalogue must be valid")
        }),
    }
}

pub(crate) fn validate_embedded_catalogues() -> Result<(), CatalogueError> {
    let english = Catalogue::parse(EN_JSON)?;
    let chinese = Catalogue::parse(ZH_CN_JSON)?;
    validate_parity(&english, &chinese)
}

#[allow(dead_code)]
pub(crate) fn message_metadata(
    locale: Locale,
    key: &str,
) -> Result<MessageMetadata<'static>, CatalogueError> {
    catalogue(locale).metadata(key)
}

impl Catalogue {
    fn parse(source: &str) -> Result<Self, CatalogueError> {
        let document: serde_json::Value = serde_json::from_str(source)
            .map_err(|error| CatalogueError(format!("catalogue is not valid JSON: {error}")))?;
        let locale = document
            .get("locale")
            .and_then(serde_json::Value::as_str)
            .and_then(Locale::from_presentation_language_tag)
            .ok_or_else(|| CatalogueError("catalogue locale must be en or zh-CN".to_string()))?;
        let entries = document
            .get("messages")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| CatalogueError("catalogue messages must be an object".to_string()))?;

        let mut messages = BTreeMap::new();
        for (key, value) in entries {
            let object = value
                .as_object()
                .ok_or_else(|| CatalogueError(format!("message {key} must be an object")))?;
            let forms_object = object
                .get("forms")
                .and_then(serde_json::Value::as_object)
                .ok_or_else(|| CatalogueError(format!("message {key} forms must be an object")))?;
            let mut forms = BTreeMap::new();
            for (form, text) in forms_object {
                if !matches!(form.as_str(), "one" | "other") {
                    return Err(CatalogueError(format!(
                        "message {key} has unsupported plural form {form}"
                    )));
                }
                let text = text.as_str().ok_or_else(|| {
                    CatalogueError(format!("message {key} form {form} must be text"))
                })?;
                forms.insert(form.clone(), text.to_string());
            }
            if !forms.contains_key("other") {
                return Err(CatalogueError(format!(
                    "message {key} must define an other form"
                )));
            }

            let placeholders = object
                .get("placeholders")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| {
                    CatalogueError(format!("message {key} placeholders must be an array"))
                })?
                .iter()
                .map(|value| {
                    value.as_str().map(str::to_string).ok_or_else(|| {
                        CatalogueError(format!("message {key} placeholder must be text"))
                    })
                })
                .collect::<Result<BTreeSet<_>, _>>()?;

            let count = match object.get("count").and_then(serde_json::Value::as_str) {
                Some("none") => CountKind::None,
                Some("cardinal") => CountKind::Cardinal,
                _ => {
                    return Err(CatalogueError(format!(
                        "message {key} must declare count metadata"
                    )));
                }
            };

            let form_names = forms.keys().map(String::as_str).collect::<Vec<_>>();
            let expected_forms: &[&str] = match (locale, count) {
                (_, CountKind::None) => &["other"],
                (Locale::En, CountKind::Cardinal) => &["one", "other"],
                (Locale::ZhCn, CountKind::Cardinal) => &["other"],
            };
            if form_names != expected_forms {
                return Err(CatalogueError(format!(
                    "message {key} plural forms do not match locale/count metadata"
                )));
            }

            for (form, text) in &forms {
                let actual = placeholder_names(text)?;
                if actual != placeholders {
                    return Err(CatalogueError(format!(
                        "message {key} form {form} placeholder signature differs from metadata"
                    )));
                }
            }

            let accelerator = match object.get("accelerator") {
                Some(serde_json::Value::String(value)) => {
                    let mut chars = value.chars();
                    let character =
                        chars
                            .next()
                            .filter(char::is_ascii_alphabetic)
                            .ok_or_else(|| {
                                CatalogueError(format!(
                                    "message {key} accelerator must be one ASCII letter"
                                ))
                            })?;
                    if chars.next().is_some() {
                        return Err(CatalogueError(format!(
                            "message {key} accelerator must be one ASCII letter"
                        )));
                    }
                    Some(character.to_ascii_lowercase())
                }
                Some(serde_json::Value::Null) | None => None,
                _ => {
                    return Err(CatalogueError(format!(
                        "message {key} accelerator must be text or null"
                    )));
                }
            };
            let group = match object.get("group") {
                Some(serde_json::Value::String(value)) => Some(value.clone()),
                Some(serde_json::Value::Null) | None => None,
                _ => {
                    return Err(CatalogueError(format!(
                        "message {key} group must be text or null"
                    )));
                }
            };
            if accelerator.is_some() != group.is_some() {
                return Err(CatalogueError(format!(
                    "message {key} accelerator and group must be defined together"
                )));
            }
            let truncation = match object.get("truncation").and_then(serde_json::Value::as_str) {
                Some("never") => TruncationContract::Never,
                Some("user_data_visual_only") => TruncationContract::UserDataVisualOnly,
                _ => {
                    return Err(CatalogueError(format!(
                        "message {key} must declare a supported truncation contract"
                    )));
                }
            };

            messages.insert(
                key.clone(),
                Message {
                    forms,
                    placeholders,
                    count,
                    accelerator,
                    group,
                    truncation,
                },
            );
        }

        validate_accelerators(&messages)?;
        Ok(Self { locale, messages })
    }

    pub(crate) fn render(
        &self,
        key: &str,
        values: &[(&str, &str)],
        count: Option<u64>,
    ) -> Result<String, CatalogueError> {
        let message = self
            .messages
            .get(key)
            .ok_or_else(|| CatalogueError(format!("unknown message key {key}")))?;
        let form = if self.locale == Locale::En && count == Some(1) {
            "one"
        } else {
            "other"
        };
        let template = message
            .forms
            .get(form)
            .or_else(|| message.forms.get("other"))
            .expect("validated messages always contain other");
        let supplied = values
            .iter()
            .map(|(name, value)| ((*name).to_string(), *value))
            .collect::<BTreeMap<_, _>>();
        if supplied.keys().cloned().collect::<BTreeSet<_>>() != message.placeholders {
            return Err(CatalogueError(format!(
                "message {key} interpolation signature does not match catalogue"
            )));
        }
        interpolate(template, &supplied)
    }

    fn metadata(&self, key: &str) -> Result<MessageMetadata<'_>, CatalogueError> {
        let message = self
            .messages
            .get(key)
            .ok_or_else(|| CatalogueError(format!("unknown message key {key}")))?;
        Ok(MessageMetadata {
            placeholders: &message.placeholders,
            count: message.count,
            accelerator: message.accelerator,
            group: message.group.as_deref(),
            truncation: message.truncation,
        })
    }
}

fn validate_parity(english: &Catalogue, chinese: &Catalogue) -> Result<(), CatalogueError> {
    if english.locale != Locale::En || chinese.locale != Locale::ZhCn {
        return Err(CatalogueError(
            "catalogue pair must contain en and zh-CN".to_string(),
        ));
    }
    if english.messages.keys().collect::<Vec<_>>() != chinese.messages.keys().collect::<Vec<_>>() {
        return Err(CatalogueError(
            "English and Chinese catalogue keys must match exactly".to_string(),
        ));
    }
    for (key, en) in &english.messages {
        let zh = &chinese.messages[key];
        if en.placeholders != zh.placeholders {
            return Err(CatalogueError(format!(
                "message {key} placeholder signatures differ by locale"
            )));
        }
        if en.count != zh.count {
            return Err(CatalogueError(format!(
                "message {key} count metadata differs by locale"
            )));
        }
        if en.group != zh.group || en.accelerator.is_some() != zh.accelerator.is_some() {
            return Err(CatalogueError(format!(
                "message {key} accelerator metadata differs by locale"
            )));
        }
        if en.truncation != zh.truncation {
            return Err(CatalogueError(format!(
                "message {key} truncation metadata differs by locale"
            )));
        }
    }
    Ok(())
}

fn validate_accelerators(messages: &BTreeMap<String, Message>) -> Result<(), CatalogueError> {
    let mut used = BTreeMap::<(&str, char), &str>::new();
    for (key, message) in messages {
        if let (Some(group), Some(accelerator)) = (message.group.as_deref(), message.accelerator)
            && let Some(previous) = used.insert((group, accelerator), key)
        {
            return Err(CatalogueError(format!(
                "messages {previous} and {key} reuse accelerator {accelerator} in group {group}"
            )));
        }
    }
    Ok(())
}

fn placeholder_names(template: &str) -> Result<BTreeSet<String>, CatalogueError> {
    let mut names = BTreeSet::new();
    let mut remaining = template;
    while let Some(start) = remaining.find('{') {
        remaining = &remaining[start + 1..];
        let end = remaining.find('}').ok_or_else(|| {
            CatalogueError("message contains an unclosed placeholder".to_string())
        })?;
        let name = &remaining[..end];
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Err(CatalogueError(format!(
                "message contains invalid placeholder {{{name}}}"
            )));
        }
        names.insert(name.to_string());
        remaining = &remaining[end + 1..];
    }
    Ok(names)
}

fn interpolate(template: &str, values: &BTreeMap<String, &str>) -> Result<String, CatalogueError> {
    let mut rendered = String::with_capacity(template.len());
    let mut remaining = template;
    while let Some(start) = remaining.find('{') {
        rendered.push_str(&remaining[..start]);
        let after = &remaining[start + 1..];
        let end = after.find('}').ok_or_else(|| {
            CatalogueError("message contains an unclosed placeholder".to_string())
        })?;
        let name = &after[..end];
        let value = values
            .get(name)
            .ok_or_else(|| CatalogueError(format!("missing interpolation value {name}")))?;
        rendered.push_str(&sanitize_terminal_data(value));
        remaining = &after[end + 1..];
    }
    rendered.push_str(remaining);
    Ok(rendered)
}

fn sanitize_terminal_data(value: &str) -> String {
    let mut sanitized = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_control()
            || character == '\u{7f}'
            || matches!(character, '\u{061c}' | '\u{200e}' | '\u{200f}')
            || matches!(character, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        {
            sanitized.push_str(&format!("\\u{{{:x}}}", u32::from(character)));
        } else {
            sanitized.push(character);
        }
    }
    sanitized
}

#[allow(dead_code)]
pub(crate) fn format_binary_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut divisor = 1024_u128;
    let mut unit = 1;
    while unit < UNITS.len() - 1 && u128::from(bytes) >= divisor * 1024 {
        divisor *= 1024;
        unit += 1;
    }
    let tenths = (u128::from(bytes) * 10 + divisor / 2) / divisor;
    format!("{}.{} {}", tenths / 10, tenths % 10, UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum PresentationLanguageTagFixture {
        En,
        ZhCn,
    }

    impl PresentationLanguageTagFixture {
        fn parse(value: &str) -> Option<Self> {
            match value {
                "en" => Some(Self::En),
                "zh-CN" => Some(Self::ZhCn),
                _ => None,
            }
        }
    }

    impl From<PresentationLanguageTagFixture> for Locale {
        fn from(value: PresentationLanguageTagFixture) -> Self {
            match value {
                PresentationLanguageTagFixture::En => Self::En,
                PresentationLanguageTagFixture::ZhCn => Self::ZhCn,
            }
        }
    }

    impl From<Locale> for PresentationLanguageTagFixture {
        fn from(value: Locale) -> Self {
            match value {
                Locale::En => Self::En,
                Locale::ZhCn => Self::ZhCn,
            }
        }
    }

    #[test]
    fn embedded_catalogues_have_exact_schema_parity() {
        validate_embedded_catalogues().expect("embedded catalogues validate");
    }

    #[test]
    fn locale_resolution_and_presentation_tag_boundary_are_closed() {
        assert_eq!(resolve_locale(Some(Locale::En), Some("zh-CN")), Locale::En);
        assert_eq!(resolve_locale(None, Some("zh_CN")), Locale::ZhCn);
        assert_eq!(resolve_locale(None, Some("zh-SG")), Locale::ZhCn);
        assert_eq!(resolve_locale(None, Some("zh-Hans")), Locale::ZhCn);
        for traditional in ["zh-TW", "zh-HK", "zh-MO", "zh-Hant"] {
            assert_eq!(resolve_locale(None, Some(traditional)), Locale::En);
        }
        assert_eq!(resolve_locale(None, Some("fr-FR")), Locale::En);
        assert_eq!(
            Locale::from_presentation_language_tag("en"),
            Some(Locale::En)
        );
        assert_eq!(
            Locale::from_presentation_language_tag("zh-CN"),
            Some(Locale::ZhCn)
        );
        assert_eq!(Locale::from_presentation_language_tag("zh-cn"), None);
        assert_eq!(Locale::from_presentation_language_tag("fr"), None);
        assert_eq!(Locale::En.tag(), "en");
        assert_eq!(Locale::ZhCn.tag(), "zh-CN");

        for tag in [
            PresentationLanguageTagFixture::En,
            PresentationLanguageTagFixture::ZhCn,
        ] {
            let locale = Locale::from(tag);
            assert_eq!(PresentationLanguageTagFixture::from(locale), tag);
            assert_eq!(
                PresentationLanguageTagFixture::parse(locale.tag()),
                Some(tag)
            );
        }
        for unknown in ["", "EN", "zh-cn", "zh-TW", "fr"] {
            assert_eq!(PresentationLanguageTagFixture::parse(unknown), None);
            assert_eq!(Locale::from_presentation_language_tag(unknown), None);
        }
    }

    #[test]
    fn plural_selection_and_interpolation_are_locale_specific_and_injection_safe() {
        let en = catalogue(Locale::En);
        let zh = catalogue(Locale::ZhCn);
        assert_eq!(
            en.render("summary.item_count", &[("count", "1")], Some(1))
                .unwrap(),
            "1 item"
        );
        assert_eq!(
            en.render("summary.item_count", &[("count", "0")], Some(0))
                .unwrap(),
            "0 items"
        );
        assert_eq!(
            zh.render("summary.item_count", &[("count", "1")], Some(1))
                .unwrap(),
            "1 项"
        );
        let payload = "<b>{command}</b> & data";
        assert_eq!(
            en.render("error.mode_unavailable", &[("command", payload)], None)
                .unwrap(),
            format!("The {payload} command is not available in this staged build.")
        );

        let hostile =
            "normal中文🙂\u{1b}]0;owned\u{7}\n{command}\u{061c}\u{200e}\u{200f}\u{202e}tail";
        let rendered = en
            .render("error.mode_unavailable", &[("command", hostile)], None)
            .unwrap();
        assert!(rendered.contains(
            "normal中文🙂\\u{1b}]0;owned\\u{7}\\u{a}{command}\\u{61c}\\u{200e}\\u{200f}\\u{202e}tail"
        ));
        assert!(!rendered.chars().any(char::is_control));
        for directional_control in ['\u{061c}', '\u{200e}', '\u{200f}', '\u{202e}'] {
            assert!(!rendered.contains(directional_control));
        }
        assert!(
            rendered.contains("normal中文🙂"),
            "ordinary Unicode must be preserved"
        );
        assert_eq!(rendered.matches("{command}").count(), 1);
    }

    #[test]
    fn shell_metadata_facade_exposes_exact_locale_specific_contracts() {
        let en = message_metadata(Locale::En, "command.clean").unwrap();
        let zh = message_metadata(Locale::ZhCn, "command.clean").unwrap();
        assert!(en.placeholders.is_empty());
        assert_eq!(en.count, CountKind::None);
        assert_eq!(en.accelerator, Some('c'));
        assert_eq!(zh.accelerator, Some('q'));
        assert_eq!(en.group, Some("root_modes"));
        assert_eq!(zh.group, en.group);
        assert_eq!(en.truncation, TruncationContract::Never);
        assert_eq!(zh.truncation, en.truncation);

        let counted = message_metadata(Locale::En, "summary.item_count").unwrap();
        assert_eq!(counted.count, CountKind::Cardinal);
        assert_eq!(
            counted
                .placeholders
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["count"]
        );
        let path = message_metadata(Locale::ZhCn, "field.path").unwrap();
        assert_eq!(path.truncation, TruncationContract::UserDataVisualOnly);
    }

    #[test]
    fn binary_formatter_covers_boundaries_without_floating_point() {
        let cases = [
            (0, "0 B"),
            (1, "1 B"),
            (1023, "1023 B"),
            (1024, "1.0 KiB"),
            (1536, "1.5 KiB"),
            (1024 * 1024 - 1, "1024.0 KiB"),
            (1024 * 1024, "1.0 MiB"),
            (u64::MAX, "16777216.0 TiB"),
        ];
        for (bytes, expected) in cases {
            assert_eq!(format_binary_bytes(bytes), expected);
        }
    }

    #[test]
    fn duplicate_accelerators_are_rejected_within_one_group() {
        let invalid = EN_JSON.replace("\"accelerator\": \"P\"", "\"accelerator\": \"S\"");
        assert!(Catalogue::parse(&invalid).is_err());
    }

    #[test]
    fn long_user_data_is_returned_in_full_by_the_catalogue() {
        let long_path = format!("C:/{}", "很长的目录/".repeat(100));
        let rendered = catalogue(Locale::ZhCn)
            .render("field.path", &[("path", &long_path)], None)
            .expect("long path renders");
        assert!(rendered.contains(&long_path));
        assert!(!rendered.contains('…'));
    }

    #[test]
    fn human_golden_fixtures_are_owned_by_the_canonical_catalogues() {
        let english = format!(
            "{}\n{}\n",
            catalogue(Locale::En)
                .render("app.title", &[], None)
                .unwrap(),
            catalogue(Locale::En)
                .render("app.about", &[], None)
                .unwrap()
        );
        let chinese = format!(
            "{}\n{}\n",
            catalogue(Locale::ZhCn)
                .render("app.title", &[], None)
                .unwrap(),
            catalogue(Locale::ZhCn)
                .render("app.about", &[], None)
                .unwrap()
        );
        assert_eq!(
            english,
            include_str!("../../tests/fixtures/cli/human-en.txt")
        );
        assert_eq!(
            chinese,
            include_str!("../../tests/fixtures/cli/human-zh-CN.txt")
        );
    }
}
