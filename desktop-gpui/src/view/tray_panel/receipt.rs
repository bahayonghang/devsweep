//! This native window uses the normal callback receiver, menu, popup and shell.
//! Only the notification shell, audit totals and supplied Status observations are fixtures.
use super::*;
use devsweep_core::{
    history::{CleanMovedTotalsV1, SuccessTotalV1},
    status::{AvailabilityV1, StatusSnapshotV1},
};
use windows::Win32::UI::Shell::{
    NIM_ADD, NIM_DELETE, NIM_SETVERSION, NOTIFY_ICON_MESSAGE, NOTIFYICONDATAW,
};

pub(super) fn fixture_notification(message: NOTIFY_ICON_MESSAGE, _: &NOTIFYICONDATAW) -> bool {
    let name = match message {
        NIM_ADD => "add",
        NIM_SETVERSION => "version",
        NIM_DELETE => "delete",
        _ => "focus",
    };
    println!("tray-shell {name}");
    true
}

pub(super) fn totals() -> HistoryTotalsV1 {
    HistoryTotalsV1 {
        clean: CleanMovedTotalsV1 {
            known_bytes: 128_000_000,
            unknown_records: 1,
            lower_bound: true,
        },
        software: SuccessTotalV1 {
            succeeded: 3,
            lower_bound: false,
        },
        optimize: SuccessTotalV1 {
            succeeded: 7,
            lower_bound: false,
        },
    }
}

#[test]
#[ignore = "opens a fixture-only review window on the private desktop selected by the receipt harness"]
fn native_tray_receipt() {
    let Ok(scene) = std::env::var("DEVSWEEP_NATIVE_TRAY_RECEIPT") else {
        return;
    };
    let (language, theme) = scene.split_once(':').expect("en|zh:dark|light");
    let locale = match language {
        "en" => copy::Locale::En,
        "zh" => copy::Locale::ZhCn,
        _ => panic!("unknown language"),
    };
    let theme = match theme {
        "dark" => DesktopTheme::Dark,
        "light" => DesktopTheme::Light,
        _ => panic!("unknown theme"),
    };
    let mut session = Session::unbacked();
    session.set_locale(locale);
    session.set_theme(theme);
    session.show_mode(Mode::Status);
    let mut sample: StatusSnapshotV1 = serde_json::from_str(include_str!(
        "../../../../crates/devsweep-cli/tests/fixtures/status/snapshot.json"
    ))
    .unwrap();
    if let AvailabilityV1::Partial { value, .. } = &mut sample.processes {
        let seed = value.items[0].clone();
        value.items = (1..=6)
            .map(|id| {
                let mut p = seed.clone();
                p.name = format!("fixture-{id}.exe");
                p.pid = id + 32;
                p.cpu_basis_points_of_one_logical_core = 3300 - id * 450;
                p
            })
            .collect();
        value.returned_count = 6;
    }
    let root = std::env::temp_dir().join(format!("devsweep-tray-native-{}", std::process::id()));
    std::fs::create_dir(&root).expect("new owned fixture directory");
    let files = crate::preferences::PreferenceFiles::new(
        root.join("settings"),
        root.join("protection.json"),
    );
    session.install_tray_fixture(sample, files);
    if std::env::var("DEVSWEEP_TRAY_RESIDENT").as_deref() == Ok("1") {
        session.set_close_to_tray(true);
        session
            .save_settings()
            .expect("private close-to-tray setting");
        assert!(session.close_to_tray());
    }
    super::super::run(session);
    std::fs::remove_dir_all(root).expect("release owned fixture directory");
}
