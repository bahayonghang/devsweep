//! Opt-in visual acceptance, launched by an isolated, non-input Windows desktop.
use super::*;
use crate::preferences::DesktopTheme;
use devsweep_core::status::{FakeClock, KeepAwakeService, RecordingBackend};

#[test]
#[ignore = "opens a fixed native review window for the Windows receipt harness"]
fn native_host_review_receipt() {
    let Ok(scene) = std::env::var("DEVSWEEP_NATIVE_HOST_RECEIPT") else {
        return;
    };
    let parts: Vec<_> = scene.split(':').collect();
    let [action, stage, language, theme] = parts.as_slice() else {
        panic!("expected action:stage:language:theme")
    };
    let locale = match *language {
        "en" => Locale::En,
        "zh" => Locale::ZhCn,
        _ => panic!("unknown language"),
    };
    let theme = match *theme {
        "light" => DesktopTheme::Light,
        "dark" => DesktopTheme::Dark,
        _ => panic!("unknown theme"),
    };
    assert!(matches!(*stage, "review" | "result"));
    let mut session = match *action {
        "msix" | "msi" => {
            let (mut session, _, id) = if *action == "msi" {
                super::msix::Fixture::msi(SoftwareExecutionOutcome::Removed, 0)
            } else {
                super::msix::Fixture::session(SoftwareExecutionOutcome::Removed)
            };
            session.activate_mode_row(Mode::Software, id);
            session
        }
        "leftovers" => {
            let (mut session, _) = super::leftovers::Fixture::session();
            super::leftovers::start_review(&mut session);
            session
        }
        "process" => {
            let mut session = ProcessFixture::new().session();
            assert!(session.status_preview_process_termination());
            session
        }
        "awake" => {
            let (backend, _) = RecordingBackend::pair();
            let mut session = Session::unbacked();
            session.host_executor = Some(Arc::new(host_review::CoreExecutor::with_awake(
                KeepAwakeService::new(Arc::new(FakeClock::new()), Box::new(backend), true),
            )));
            assert!(session.keep_awake_preview_toggle());
            session
        }
        "update" => {
            let mut session = Session::unbacked();
            session.host_executor = Some(Arc::new(FakeExecutor::default()));
            assert!(session.host_preview(vec![update("DevSweep Receipt Fixture")]));
            session
        }
        "startup" => {
            let mut session = Session::unbacked();
            let path = std::env::temp_dir().join(format!(
                "devsweep-startup-receipt-{}.json",
                std::process::id()
            ));
            let hive = StartupFixtureHiveV1 {
                version: 1,
                entries: vec![StartupFixtureEntryV1 {
                    name: "DevSweep Receipt Fixture".into(),
                    location: SoftwareStartupLocation::CurrentUserRun,
                    class: StartupFixtureClass::Ordinary,
                    run_value: b"fixture".to_vec(),
                    approval: None,
                }],
            };
            std::fs::File::create_new(&path)
                .and_then(|mut file| {
                    std::io::Write::write_all(&mut file, &serde_json::to_vec(&hive).unwrap())
                })
                .expect("create isolated fixture hive");
            session.show_mode(Mode::Software);
            session.software_review.open_hive(&path, locale);
            session.sync_software_rows();
            let id = session.software_review.filtered_ids()[0].clone();
            session.activate_mode_row(Mode::Software, id);
            session
        }
        _ => panic!("unknown action"),
    };
    apply_one(&mut session);
    assert_eq!(session.host_review.phase, HostPhase::Ready);
    if *stage == "result" {
        assert!(session.host_confirm(true));
        apply_one(&mut session);
        assert_eq!(session.host_review.phase, HostPhase::Finished);
    }
    if *action == "startup" {
        let HostIntent::Startup {
            source: host_review::StartupSource::Hive(path),
            ..
        } = &session.host_review.intents[0]
        else {
            panic!("fixture hive")
        };
        std::fs::remove_file(path).expect("remove the isolated fixture");
    }
    // Only the completed presentation crosses into the window. The owner,
    // fake executor and any queued workers are dropped before GPUI starts.
    let mut display = Session::unbacked();
    display.host_review = std::mem::take(&mut session.host_review);
    drop(session);
    display.set_locale(locale);
    display.set_theme(theme);
    crate::view::run_host_receipt(display);
}
