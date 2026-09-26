use std::{ffi::OsString, fs, io, path::Path, sync::Arc, thread};

use super::store::{load_from_path, path_from_local_app_data, update_at_path, update_with_replace};
use super::*;

const DEFAULT_BYTES: &[u8] = br#"{"schema_version":1,"theme":"dark","font_family":"system","text_scale_percent":100,"motion":"system","planet_fps":30,"status_interval_seconds":2,"status_process_limit":15,"hud_interval_seconds":2}"#;

fn path(root: &Path) -> std::path::PathBuf {
    root.join("DevSweep/settings/desktop-preferences-v1.json")
}

fn all_changes() -> [DesktopPreferencesPatch; 8] {
    [
        DesktopPreferencesPatch::Theme {
            value: DesktopTheme::Light,
        },
        DesktopPreferencesPatch::FontFamily {
            value: DesktopFontFamily::MicrosoftYaheiUi,
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

fn changed_preferences() -> DesktopPreferencesV1 {
    all_changes()
        .into_iter()
        .fold(DesktopPreferencesV1::default(), |value, patch| {
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
        DesktopPreferencesV1::default()
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
        DesktopPreferencesPatch::FontFamily {
            value: DesktopFontFamily::SegoeUi,
        },
        DesktopPreferencesPatch::ResetAppearance {},
        DesktopPreferencesPatch::ResetPerformance {},
    ]) {
        let encoded = serde_json::to_value(patch).unwrap();
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
            let mut document = serde_json::to_value(DesktopPreferencesV1::default()).unwrap();
            document[field] = value.into();
            assert_eq!(
                serde_json::from_value::<DesktopPreferencesV1>(document).is_ok(),
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
        ("schema_version", serde_json::json!(2)),
        ("schema_version", serde_json::json!(0)),
        ("theme", serde_json::json!("auto")),
        ("font_family", serde_json::json!("remote_font")),
        ("motion", serde_json::json!("full")),
        ("planet_fps", serde_json::json!(60)),
        ("text_scale_percent", serde_json::json!(101)),
        ("status_interval_seconds", serde_json::json!(3)),
        ("status_process_limit", serde_json::json!(0)),
        ("hud_interval_seconds", serde_json::json!(1)),
        ("future", serde_json::json!(true)),
    ] {
        let mut document = serde_json::to_value(DesktopPreferencesV1::default()).unwrap();
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
    let defaults = DesktopPreferencesV1::default();
    assert_eq!(
        update_at_path(&path, DesktopPreferencesPatch::ResetAppearance {}).unwrap(),
        DesktopPreferencesV1 {
            theme: defaults.theme,
            font_family: defaults.font_family,
            text_scale_percent: defaults.text_scale_percent,
            ..changed
        }
    );
    for patch in all_changes() {
        update_at_path(&path, patch).unwrap();
    }
    assert_eq!(
        update_at_path(&path, DesktopPreferencesPatch::ResetPerformance {}).unwrap(),
        DesktopPreferencesV1 {
            theme: changed.theme,
            font_family: changed.font_family,
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
}
