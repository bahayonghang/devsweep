//! Presentation-only TUI shell contracts.

use anyhow::Result;
use devsweep_core::presentation_settings::{
    PresentationLanguageTag, PresentationSettingsV1, load_presentation_settings,
    save_presentation_settings,
};

use crate::i18n::{
    Locale, SHELL_V1_KEYS, catalogue, message_metadata, resolve_locale, windows_user_locale,
};

/// The frozen five-mode navigation identity. Availability is supplied by
/// registrations; absent modes are not rendered as placeholders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ModeId {
    Clean,
    Software,
    Optimize,
    Analyze,
    Status,
}

impl ModeId {
    const ALL: [Self; 5] = [
        Self::Clean,
        Self::Software,
        Self::Optimize,
        Self::Analyze,
        Self::Status,
    ];

    const fn message_key(self) -> &'static str {
        match self {
            Self::Clean => "command.clean",
            Self::Software => "command.software",
            Self::Optimize => "command.optimize",
            Self::Analyze => "command.analyze",
            Self::Status => "command.status",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ModeRegistration {
    id: ModeId,
}

impl ModeRegistration {
    const fn clean() -> Self {
        Self { id: ModeId::Clean }
    }

    const fn analyze() -> Self {
        Self {
            id: ModeId::Analyze,
        }
    }

    const fn software() -> Self {
        Self {
            id: ModeId::Software,
        }
    }

    const fn optimize() -> Self {
        Self {
            id: ModeId::Optimize,
        }
    }

    const fn status() -> Self {
        Self { id: ModeId::Status }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ShellNavigationItem {
    pub(super) id: ModeId,
    pub(super) label: String,
    pub(super) accelerator: Option<char>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ShellCopy {
    pub(super) workbench: &'static str,
    pub(super) settings_title: &'static str,
    pub(super) settings_action: &'static str,
    pub(super) settings_instruction: &'static str,
    pub(super) english: &'static str,
    pub(super) simplified_chinese: &'static str,
    pub(super) save: &'static str,
    pub(super) saving: &'static str,
    pub(super) cancel: &'static str,
    pub(super) persistence_unavailable: &'static str,
}

impl ShellCopy {
    fn for_locale(locale: Locale) -> Result<Self> {
        let catalogue = catalogue(locale);
        Ok(Self {
            workbench: catalogue.static_text("shell.v1.workbench")?,
            settings_title: catalogue.static_text("shell.v1.settings.title")?,
            settings_action: catalogue.static_text("shell.v1.settings.action")?,
            settings_instruction: catalogue.static_text("shell.v1.settings.instruction")?,
            english: catalogue.static_text("shell.v1.settings.option.en")?,
            simplified_chinese: catalogue.static_text("shell.v1.settings.option.zh_cn")?,
            save: catalogue.static_text("shell.v1.settings.save")?,
            saving: catalogue.static_text("shell.v1.settings.saving")?,
            cancel: catalogue.static_text("shell.v1.settings.cancel")?,
            persistence_unavailable: catalogue.static_text("shell.v1.persistence.unavailable")?,
        })
    }
}

/// Typed presentation data consumed by the real TUI reducer and renderer.
/// Mode children can fill registrations without introducing shell authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ShellComposition {
    pub(super) locale: Locale,
    pub(super) app_title: String,
    pub(super) active_label: String,
    pub(super) active: ModeId,
    pub(super) navigation: Vec<ShellNavigationItem>,
    pub(super) copy: ShellCopy,
}

impl ShellComposition {
    pub(super) fn for_locale(locale: Locale) -> Result<Self> {
        TuiShell::with_locale(locale).compose()
    }

    pub(super) fn activate(&mut self, mode: ModeId) -> bool {
        let Some(item) = self.navigation.iter().find(|item| item.id == mode) else {
            return false;
        };
        self.active = mode;
        self.active_label.clone_from(&item.label);
        true
    }
}

/// Presentation-only shell state. Domain actions and mode reducers never enter
/// this type.
pub(super) struct TuiShell {
    locale: Locale,
    registrations: Vec<ModeRegistration>,
    active: ModeId,
}

impl TuiShell {
    pub(super) fn load(explicit: Option<Locale>) -> Result<Self> {
        // Always validate the persisted document, even when the explicit flag
        // wins. Corrupt/newer bytes make the store unavailable and are never
        // silently ignored or overwritten.
        let persisted = load_presentation_settings()?.language.map(Locale::from);
        let locale =
            resolve_shell_locale(explicit, Ok(persisted), windows_user_locale().as_deref())?;
        Ok(Self::with_locale(locale))
    }

    fn with_locale(locale: Locale) -> Self {
        Self {
            locale,
            registrations: vec![
                ModeRegistration::clean(),
                ModeRegistration::software(),
                ModeRegistration::optimize(),
                ModeRegistration::analyze(),
                ModeRegistration::status(),
            ],
            active: ModeId::Clean,
        }
    }

    pub(super) fn compose(&self) -> Result<ShellComposition> {
        // Validate every frozen mode against the canonical catalogue at shell
        // startup, while rendering only registrations that are actually
        // available in this staged build.
        for mode_id in ModeId::ALL {
            let key = mode_id.message_key();
            catalogue(self.locale).render(key, &[], None)?;
            message_metadata(self.locale, key)?;
        }
        for key in SHELL_V1_KEYS {
            catalogue(self.locale).static_text(key)?;
            message_metadata(self.locale, key)?;
        }
        let navigation = self
            .registrations
            .iter()
            .map(|registration| {
                let key = registration.id.message_key();
                Ok(ShellNavigationItem {
                    id: registration.id,
                    label: catalogue(self.locale).render(key, &[], None)?,
                    accelerator: message_metadata(self.locale, key)?.accelerator,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let active_label = catalogue(self.locale).render(self.active.message_key(), &[], None)?;
        Ok(ShellComposition {
            locale: self.locale,
            app_title: catalogue(self.locale).render("app.title", &[], None)?,
            active_label,
            active: self.active,
            navigation,
            copy: ShellCopy::for_locale(self.locale)?,
        })
    }
}

impl From<PresentationLanguageTag> for Locale {
    fn from(tag: PresentationLanguageTag) -> Self {
        match tag {
            PresentationLanguageTag::En => Self::En,
            PresentationLanguageTag::ZhCn => Self::ZhCn,
        }
    }
}

impl From<Locale> for PresentationLanguageTag {
    fn from(locale: Locale) -> Self {
        match locale {
            Locale::En => Self::En,
            Locale::ZhCn => Self::ZhCn,
        }
    }
}

fn resolve_shell_locale(
    explicit: Option<Locale>,
    persisted: Result<Option<Locale>>,
    windows_locale: Option<&str>,
) -> Result<Locale> {
    let persisted = persisted?;
    Ok(resolve_locale(explicit.or(persisted), windows_locale))
}

pub(super) fn persist_language(locale: Locale) -> Result<()> {
    save_presentation_settings(PresentationSettingsV1 {
        language: Some(locale.into()),
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;

    use super::*;

    #[test]
    fn frozen_mode_identity_is_complete_but_unregistered_modes_are_absent() {
        assert_eq!(ModeId::ALL.len(), 5);
        let english = TuiShell::with_locale(Locale::En).compose().unwrap();
        assert_eq!(english.active_label, "Clean");
        assert_eq!(
            english.navigation,
            [
                ShellNavigationItem {
                    id: ModeId::Clean,
                    label: "Clean".to_string(),
                    accelerator: Some('c'),
                },
                ShellNavigationItem {
                    id: ModeId::Software,
                    label: "Software".to_string(),
                    accelerator: Some('s'),
                },
                ShellNavigationItem {
                    id: ModeId::Optimize,
                    label: "Optimize".to_string(),
                    accelerator: Some('p'),
                },
                ShellNavigationItem {
                    id: ModeId::Analyze,
                    label: "Analyze".to_string(),
                    accelerator: Some('a'),
                },
                ShellNavigationItem {
                    id: ModeId::Status,
                    label: "Status".to_string(),
                    accelerator: Some('t'),
                },
            ]
        );

        let chinese = TuiShell::with_locale(Locale::ZhCn).compose().unwrap();
        assert_eq!(chinese.active_label, "清理");
        assert_eq!(
            chinese.navigation,
            [
                ShellNavigationItem {
                    id: ModeId::Clean,
                    label: "清理".to_string(),
                    accelerator: None,
                },
                ShellNavigationItem {
                    id: ModeId::Software,
                    label: "软件".to_string(),
                    accelerator: Some('r'),
                },
                ShellNavigationItem {
                    id: ModeId::Optimize,
                    label: "优化".to_string(),
                    accelerator: Some('y'),
                },
                ShellNavigationItem {
                    id: ModeId::Analyze,
                    label: "分析".to_string(),
                    accelerator: Some('f'),
                },
                ShellNavigationItem {
                    id: ModeId::Status,
                    label: "状态".to_string(),
                    accelerator: Some('z'),
                },
            ]
        );
        assert_eq!(chinese.copy.settings_title, "语言设置");
    }

    #[test]
    fn exhaustive_stable_tag_mapping_round_trips() {
        for tag in [PresentationLanguageTag::En, PresentationLanguageTag::ZhCn] {
            let locale = Locale::from(tag);
            assert_eq!(PresentationLanguageTag::from(locale), tag);
        }
    }

    #[test]
    fn locale_precedence_is_explicit_then_persisted_then_os_then_english() {
        assert_eq!(
            resolve_shell_locale(Some(Locale::En), Ok(Some(Locale::ZhCn)), Some("zh-CN")).unwrap(),
            Locale::En
        );
        assert_eq!(
            resolve_shell_locale(None, Ok(Some(Locale::ZhCn)), Some("en-US")).unwrap(),
            Locale::ZhCn
        );
        assert_eq!(
            resolve_shell_locale(None, Ok(None), Some("zh-SG")).unwrap(),
            Locale::ZhCn
        );
        assert_eq!(
            resolve_shell_locale(None, Ok(None), Some("fr-FR")).unwrap(),
            Locale::En
        );
        assert_eq!(
            resolve_shell_locale(None, Ok(None), None).unwrap(),
            Locale::En
        );
    }

    #[test]
    fn corrupt_persisted_store_fails_closed_at_every_precedence_level() {
        assert!(
            resolve_shell_locale(
                Some(Locale::En),
                Err(anyhow!("unsupported presentation document")),
                Some("zh-CN")
            )
            .is_err()
        );
        assert!(
            resolve_shell_locale(
                None,
                Err(anyhow!("unsupported presentation document")),
                Some("en-US")
            )
            .is_err()
        );
    }
}
