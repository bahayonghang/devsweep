use std::{ffi::OsString, fs, io, path::Path, sync::Arc, thread};

use super::store::{load_from_path, path_from_local_app_data, update_at_path, update_with_replace};
use super::*;

const DEFAULT_BYTES: &[u8] = br#"{"schema_version":2,"theme":"dark","font":{"kind":"system"},"text_scale_percent":100,"motion":"system","planet_fps":30,"status_interval_seconds":2,"status_process_limit":15,"hud_interval_seconds":2}"#;
const LEGACY_BYTES: &[u8] = br#"{"schema_version":1,"theme":"light","font_family":"system","text_scale_percent":125,"motion":"reduced","planet_fps":15,"status_interval_seconds":60,"status_process_limit":100,"hud_interval_seconds":10}"#;

#[test]
fn all_theme_ids_survive_save_reload_and_keep_legacy_bytes() {
    let themes = [
        ("dark", DesktopTheme::Dark),
        ("light", DesktopTheme::Light),
        ("system", DesktopTheme::System),
        ("catppuccin_latte", DesktopTheme::CatppuccinLatte),
        ("catppuccin_mocha", DesktopTheme::CatppuccinMocha),
        ("codex", DesktopTheme::Codex),
        ("claude", DesktopTheme::Claude),
    ];
    let root = tempfile::tempdir().unwrap();
    let path = path(root.path());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let legacy = path.with_file_name("desktop-preferences-v1.json");
    fs::write(&legacy, LEGACY_BYTES).unwrap();
    let original = load_from_path(&path).unwrap();
    for (id, theme) in themes {
        let patch: DesktopPreferencesPatch =
            serde_json::from_value(serde_json::json!({ "field": "theme", "value": id })).unwrap();
        let saved = update_at_path(&path, patch).unwrap();
        assert_eq!(saved.theme, theme);
        assert_eq!(
            saved,
            DesktopPreferencesV2 {
                theme,
                ..original.clone()
            }
        );
        assert_eq!(load_from_path(&path).unwrap(), saved);
        assert_eq!(serde_json::to_value(&saved).unwrap()["theme"], id);
        assert_eq!(fs::read(&legacy).unwrap(), LEGACY_BYTES);
    }
    let reset = update_at_path(&path, DesktopPreferencesPatch::ResetAppearance {}).unwrap();
    assert_eq!(reset.theme, DesktopTheme::Dark);
    assert_eq!(
        reset.status_interval_seconds,
        original.status_interval_seconds
    );
    assert_eq!(fs::read(legacy).unwrap(), LEGACY_BYTES);
}

#[test]
fn legacy_mapping_is_read_only_until_explicit_first_save() {
    for (preset, font) in [
        ("system", DesktopFont::System {}),
        (
            "segoe_ui",
            DesktopFont::Installed {
                family: "Segoe UI".into(),
            },
        ),
        (
            "microsoft_yahei_ui",
            DesktopFont::Installed {
                family: "Microsoft YaHei UI".into(),
            },
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = path(root.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let legacy_path = path.with_file_name("desktop-preferences-v1.json");
        let language = path.with_file_name("presentation-v1.json");
        let mut original: serde_json::Value = serde_json::from_slice(LEGACY_BYTES).unwrap();
        original["font_family"] = preset.into();
        let bytes = serde_json::to_vec_pretty(&original).unwrap();
        fs::write(&legacy_path, &bytes).unwrap();
        fs::write(&language, b"preserved language bytes").unwrap();
        let migrated = load_from_path(&path).unwrap();
        assert_eq!(migrated.font, font);
        let actual = serde_json::to_value(&migrated).unwrap();
        for key in [
            "theme",
            "text_scale_percent",
            "motion",
            "planet_fps",
            "status_interval_seconds",
            "status_process_limit",
            "hud_interval_seconds",
        ] {
            assert_eq!(actual[key], original[key], "{key}");
        }
        assert!(!path.exists());
        assert!(
            update_with_replace(
                &path,
                DesktopPreferencesPatch::ResetAppearance {},
                fail_replace
            )
            .is_err()
        );
        assert!(!path.exists());
        assert_eq!(fs::read(&legacy_path).unwrap(), bytes);
        let saved =
            update_at_path(&path, DesktopPreferencesPatch::PlanetFps { value: 30 }).unwrap();
        assert_eq!(saved.font, font);
        assert_eq!(saved.planet_fps, 30);
        assert_eq!(fs::read(&legacy_path).unwrap(), bytes);
        assert_eq!(fs::read(language).unwrap(), b"preserved language bytes");
        fs::write(&legacy_path, b"older app changed or damaged V1").unwrap();
        assert_eq!(load_from_path(&path).unwrap(), saved);
    }
}

#[test]
fn legacy_and_v2_invalid_or_unreadable_documents_never_fall_back_or_reset() {
    let future = String::from_utf8(LEGACY_BYTES.to_vec())
        .unwrap()
        .replace("\"schema_version\":1", "\"schema_version\":99")
        .into_bytes();
    for version in [1, 2] {
        for invalid in [b"{".to_vec(), b"\xff".to_vec(), future.clone()] {
            let root = tempfile::tempdir().unwrap();
            let path = path(root.path());
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let source = path.with_file_name(format!("desktop-preferences-v{version}.json"));
            if version == 2 {
                fs::write(
                    path.with_file_name("desktop-preferences-v1.json"),
                    LEGACY_BYTES,
                )
                .unwrap();
            }
            fs::write(&source, &invalid).unwrap();
            assert!(load_from_path(&path).is_err());
            for patch in [
                DesktopPreferencesPatch::ResetAppearance {},
                DesktopPreferencesPatch::ResetPerformance {},
            ] {
                assert!(update_at_path(&path, patch).is_err());
                assert_eq!(fs::read(&source).unwrap(), invalid);
            }
            if version == 1 {
                assert!(!path.exists());
            }
        }
        let root = tempfile::tempdir().unwrap();
        let path = path(root.path());
        let source = path.with_file_name(format!("desktop-preferences-v{version}.json"));
        fs::create_dir_all(&source).unwrap();
        assert!(load_from_path(&path).is_err());
        assert!(update_at_path(&path, DesktopPreferencesPatch::ResetAppearance {}).is_err());
        assert!(source.is_dir());
    }
}

#[test]
fn installed_family_validation_preserves_unicode_and_punctuation() {
    for family in [
        "中文字体",
        "A \"quoted\" \\ font; color: red",
        &"字".repeat(256),
        &"😀".repeat(256),
    ] {
        let value = DesktopFont::Installed {
            family: family.into(),
        };
        assert_eq!(
            serde_json::from_value::<DesktopFont>(serde_json::to_value(&value).unwrap()).unwrap(),
            value
        );
        assert!(
            DesktopPreferencesV2::default()
                .patched(DesktopPreferencesPatch::Font { value })
                .is_ok()
        );
    }
    for family in [
        "",
        " space",
        "space ",
        "a\nb",
        "a\0b",
        "a\u{7f}b",
        &"字".repeat(257),
    ] {
        let value = DesktopFont::Installed {
            family: family.into(),
        };
        assert!(
            serde_json::from_value::<DesktopFont>(serde_json::to_value(&value).unwrap()).is_err()
        );
        assert!(
            DesktopPreferencesV2::default()
                .patched(DesktopPreferencesPatch::Font { value })
                .is_err()
        );
    }
    for value in [
        serde_json::json!({"kind":"system","family":"x"}),
        serde_json::json!({"kind":"installed"}),
        serde_json::json!({"kind":"cloud","family":"x"}),
    ] {
        assert!(serde_json::from_value::<DesktopFont>(value).is_err());
    }
}

fn path(root: &Path) -> std::path::PathBuf {
    root.join("DevSweep/settings/desktop-preferences-v2.json")
}

fn all_changes() -> [DesktopPreferencesPatch; 8] {
    [
        DesktopPreferencesPatch::Theme {
            value: DesktopTheme::Light,
        },
        DesktopPreferencesPatch::Font {
            value: DesktopFont::Installed {
                family: "Microsoft YaHei UI".into(),
            },
        },
        DesktopPreferencesPatch::TextScalePercent { value: 125 },
        DesktopPreferencesPatch::Motion {
            value: DesktopMotion::Reduced,
        },
        DesktopPreferencesPatch::PlanetFps { value: 15 },
        DesktopPreferencesPatch::StatusIntervalSeconds { value: 60 },
        DesktopPreferencesPatch::StatusProcessLimit { value: 100 },
        DesktopPreferencesPatch::HudIntervalSeconds { value: 10 },
    ]
}

fn changed_preferences() -> DesktopPreferencesV2 {
    all_changes()
        .into_iter()
        .fold(DesktopPreferencesV2::default(), |value, patch| {
            value.patched(patch).unwrap()
        })
}

#[test]
fn fixed_path_missing_environment_and_missing_storage() {
    assert_eq!(
        path_from_local_app_data(Some(OsString::from(r"C:\Users\dev\AppData\Local"))).unwrap(),
        path(Path::new(r"C:\Users\dev\AppData\Local"))
    );
    for value in [None, Some(OsString::new())] {
        assert!(matches!(
            path_from_local_app_data(value),
            Err(DesktopPreferencesError::LocalAppDataUnavailable)
        ));
    }
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        load_from_path(&path(root.path())).unwrap(),
        DesktopPreferencesV2::default()
    );
    assert!(!path(root.path()).exists());
}

#[test]
fn default_document_is_exact_and_all_tags_round_trip() {
    let root = tempfile::tempdir().unwrap();
    let path = path(root.path());
    update_at_path(&path, DesktopPreferencesPatch::ResetAppearance {}).unwrap();
    assert_eq!(fs::read(&path).unwrap(), DEFAULT_BYTES);
    for patch in all_changes().into_iter().chain([
        DesktopPreferencesPatch::Theme {
            value: DesktopTheme::System,
        },
        DesktopPreferencesPatch::Font {
            value: DesktopFont::Installed {
                family: "Segoe UI".into(),
            },
        },
        DesktopPreferencesPatch::ResetAppearance {},
        DesktopPreferencesPatch::ResetPerformance {},
    ]) {
        let encoded = serde_json::to_value(&patch).unwrap();
        assert_eq!(
            serde_json::from_value::<DesktopPreferencesPatch>(encoded).unwrap(),
            patch
        );
        let committed = update_at_path(&path, patch).unwrap();
        assert_eq!(load_from_path(&path).unwrap(), committed);
    }
    assert_eq!(fs::read(&path).unwrap(), DEFAULT_BYTES);
}

#[test]
fn numeric_choices_and_patch_shapes_are_closed() {
    for (field, values) in [
        ("text_scale_percent", TEXT_SCALES),
        ("planet_fps", PLANET_FRAME_CAPS),
        ("status_interval_seconds", STATUS_INTERVALS),
        ("status_process_limit", STATUS_PROCESS_LIMITS),
        ("hud_interval_seconds", HUD_INTERVALS),
    ] {
        for value in 0..=255 {
            let payload = serde_json::json!({"field":field,"value":value});
            assert_eq!(
                serde_json::from_value::<DesktopPreferencesPatch>(payload).is_ok(),
                values.contains(&(value as u8))
            );
            let mut document = serde_json::to_value(DesktopPreferencesV2::default()).unwrap();
            document[field] = value.into();
            assert_eq!(
                serde_json::from_value::<DesktopPreferencesV2>(document).is_ok(),
                values.contains(&(value as u8))
            );
        }
    }
    for payload in [
        serde_json::json!({"field":"unknown","value":1}),
        serde_json::json!({"field":"theme","value":"auto"}),
        serde_json::json!({"field":"theme","value":"dark","other":true}),
        serde_json::json!({"field":"text_scale_percent","value":100.5}),
        serde_json::json!({"field":"planet_fps","value":-1}),
        serde_json::json!({"field":"reset_appearance","value":null}),
        serde_json::json!({"field":"reset_performance","value":{}}),
        serde_json::json!({"field":"reset_performance","extra":true}),
    ] {
        assert!(
            serde_json::from_value::<DesktopPreferencesPatch>(payload.clone()).is_err(),
            "accepted {payload}"
        );
    }
}

#[test]
fn malformed_future_unknown_and_invalid_documents_preserve_bytes_on_every_patch() {
    let mut invalid = vec![b"\xffnot-utf8".to_vec(), b"{".to_vec(), b"{}".to_vec()];
    for (field, value) in [
        ("schema_version", serde_json::json!(3)),
        ("schema_version", serde_json::json!(0)),
        ("theme", serde_json::json!("auto")),
        ("font", serde_json::json!("remote_font")),
        ("motion", serde_json::json!("full")),
        ("planet_fps", serde_json::json!(60)),
        ("text_scale_percent", serde_json::json!(101)),
        ("status_interval_seconds", serde_json::json!(3)),
        ("status_process_limit", serde_json::json!(0)),
        ("hud_interval_seconds", serde_json::json!(1)),
        ("future", serde_json::json!(true)),
    ] {
        let mut document = serde_json::to_value(DesktopPreferencesV2::default()).unwrap();
        document[field] = value;
        invalid.push(serde_json::to_vec(&document).unwrap());
    }
    for original in invalid {
        let root = tempfile::tempdir().unwrap();
        let path = path(root.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &original).unwrap();
        assert!(load_from_path(&path).is_err());
        for patch in all_changes().into_iter().chain([
            DesktopPreferencesPatch::ResetAppearance {},
            DesktopPreferencesPatch::ResetPerformance {},
        ]) {
            assert!(update_at_path(&path, patch).is_err());
            assert_eq!(fs::read(&path).unwrap(), original);
        }
    }
}

#[test]
fn resets_touch_only_their_group_and_preserve_language_bytes() {
    let root = tempfile::tempdir().unwrap();
    let path = path(root.path());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let language_path = path.with_file_name("presentation-v1.json");
    let language_bytes = b"{ \"schema_version\": 1, \"language\": \"zh-CN\" }\n";
    fs::write(&language_path, language_bytes).unwrap();
    for patch in all_changes() {
        update_at_path(&path, patch).unwrap();
    }
    let changed = changed_preferences();
    assert_eq!(load_from_path(&path).unwrap(), changed);
    let defaults = DesktopPreferencesV2::default();
    assert_eq!(
        update_at_path(&path, DesktopPreferencesPatch::ResetAppearance {}).unwrap(),
        DesktopPreferencesV2 {
            theme: defaults.theme,
            font: defaults.font,
            text_scale_percent: defaults.text_scale_percent,
            ..changed.clone()
        }
    );
    for patch in all_changes() {
        update_at_path(&path, patch).unwrap();
    }
    assert_eq!(
        update_at_path(&path, DesktopPreferencesPatch::ResetPerformance {}).unwrap(),
        DesktopPreferencesV2 {
            theme: changed.theme,
            font: changed.font,
            text_scale_percent: changed.text_scale_percent,
            ..defaults
        }
    );
    assert_eq!(fs::read(&language_path).unwrap(), language_bytes);
    let language: serde_json::Value =
        serde_json::from_slice(&fs::read(&language_path).unwrap()).unwrap();
    assert_eq!(language["language"], "zh-CN");
}

fn fail_replace(_source: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        "injected replacement failure",
    ))
}

#[test]
fn failed_replace_and_invalid_rust_patch_keep_old_bytes_and_allow_retry() {
    let root = tempfile::tempdir().unwrap();
    let path = path(root.path());
    update_at_path(&path, DesktopPreferencesPatch::ResetAppearance {}).unwrap();
    assert!(
        update_with_replace(
            &path,
            DesktopPreferencesPatch::Theme {
                value: DesktopTheme::Light
            },
            fail_replace
        )
        .is_err()
    );
    for patch in [
        DesktopPreferencesPatch::TextScalePercent { value: 99 },
        DesktopPreferencesPatch::PlanetFps { value: 60 },
        DesktopPreferencesPatch::StatusIntervalSeconds { value: 3 },
        DesktopPreferencesPatch::StatusProcessLimit { value: 1 },
        DesktopPreferencesPatch::HudIntervalSeconds { value: 1 },
    ] {
        assert!(update_at_path(&path, patch).is_err());
    }
    assert_eq!(fs::read(&path).unwrap(), DEFAULT_BYTES);
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    assert_eq!(
        update_at_path(
            &path,
            DesktopPreferencesPatch::Theme {
                value: DesktopTheme::Light
            }
        )
        .unwrap()
        .theme,
        DesktopTheme::Light
    );
}

#[test]
fn unreadable_path_fails_without_replacing_the_existing_entry() {
    let root = tempfile::tempdir().unwrap();
    let path = path(root.path());
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("keep"), b"existing").unwrap();
    assert!(load_from_path(&path).is_err());
    assert!(update_at_path(&path, DesktopPreferencesPatch::ResetPerformance {}).is_err());
    assert_eq!(fs::read(path.join("keep")).unwrap(), b"existing");
}

#[test]
fn concurrent_unrelated_thread_patches_keep_all_fields() {
    let root = tempfile::tempdir().unwrap();
    let path = Arc::new(path(root.path()));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let legacy = path.with_file_name("desktop-preferences-v1.json");
    fs::write(&legacy, LEGACY_BYTES).unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(8));
    let handles = all_changes().map(|patch| {
        let path = Arc::clone(&path);
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            barrier.wait();
            update_at_path(&path, patch).unwrap();
        })
    });
    for handle in handles {
        handle.join().unwrap();
    }
    assert_eq!(load_from_path(&path).unwrap(), changed_preferences());
    assert_eq!(fs::read(legacy).unwrap(), LEGACY_BYTES);
}

#[test]
fn v1_remains_closed_to_unknown_fields_tags_and_values() {
    for (field, value) in [
        ("schema_version", serde_json::json!(2)),
        ("theme", serde_json::json!("catppuccin_latte")),
        ("theme", serde_json::json!("catppuccin_mocha")),
        ("theme", serde_json::json!("codex")),
        ("theme", serde_json::json!("claude")),
        ("font_family", serde_json::json!("installed")),
        ("text_scale_percent", serde_json::json!(101)),
        ("motion", serde_json::json!("full")),
        ("planet_fps", serde_json::json!(60)),
        ("status_interval_seconds", serde_json::json!(3)),
        ("status_process_limit", serde_json::json!(0)),
        ("hud_interval_seconds", serde_json::json!(1)),
        ("unknown", serde_json::json!(true)),
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = path(root.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let legacy = path.with_file_name("desktop-preferences-v1.json");
        let mut document: serde_json::Value = serde_json::from_slice(LEGACY_BYTES).unwrap();
        document[field] = value;
        let bytes = serde_json::to_vec(&document).unwrap();
        fs::write(&legacy, &bytes).unwrap();
        assert!(load_from_path(&path).is_err(), "accepted {field}");
        assert!(update_at_path(&path, DesktopPreferencesPatch::ResetAppearance {}).is_err());
        assert_eq!(fs::read(legacy).unwrap(), bytes);
        assert!(!path.exists());
    }
}

#[cfg(windows)]
#[test]
fn readonly_v2_refuses_replace_and_preserves_both_versions() {
    let root = tempfile::tempdir().unwrap();
    let path = path(root.path());
    update_at_path(&path, DesktopPreferencesPatch::ResetAppearance {}).unwrap();
    let legacy = path.with_file_name("desktop-preferences-v1.json");
    fs::write(&legacy, LEGACY_BYTES).unwrap();
    let writable = fs::metadata(&path).unwrap().permissions();
    let mut readonly = writable.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&path, readonly).unwrap();
    let result = update_at_path(
        &path,
        DesktopPreferencesPatch::Theme {
            value: DesktopTheme::Light,
        },
    );
    fs::set_permissions(&path, writable).unwrap();
    assert!(result.is_err());
    assert_eq!(fs::read(path).unwrap(), DEFAULT_BYTES);
    assert_eq!(fs::read(legacy).unwrap(), LEGACY_BYTES);
}
