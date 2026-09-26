//! Ordered desktop preference reads, commits, and multi-window notifications.

use std::sync::{Arc, Mutex};

use devsweep_core::desktop_preferences::{
    DesktopPreferencesPatch, DesktopPreferencesV1, load_desktop_preferences,
    update_desktop_preferences,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime, State};

use crate::{error::CommandError, hud::HUD_WINDOW_LABEL, tray::MAIN_WINDOW_LABEL};

pub(crate) const PREFERENCES_CHANGED_EVENT: &str = "desktop-preferences-changed";
const MAX_SAFE_SEQUENCE: u64 = 9_007_199_254_740_991;

/// Runtime ordering metadata; the sequence is never persisted to disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize), serde(deny_unknown_fields))]
pub(crate) struct DesktopPreferencesSnapshot {
    pub(crate) sequence: u64,
    pub(crate) preferences: DesktopPreferencesV1,
}

/// One lock orders reads, successful commits, and their event publication.
#[derive(Clone, Default)]
pub(crate) struct DesktopPreferencesCoordinator(Arc<Mutex<u64>>);

impl DesktopPreferencesCoordinator {
    fn transact(
        &self,
        operation: impl FnOnce() -> Result<DesktopPreferencesV1, CommandError>,
        publish: impl FnOnce(&DesktopPreferencesSnapshot) -> Result<(), CommandError>,
    ) -> Result<DesktopPreferencesSnapshot, CommandError> {
        let mut sequence = self
            .0
            .lock()
            .map_err(|_| CommandError::io("desktop preference coordinator is poisoned"))?;
        let next_sequence = sequence
            .checked_add(1)
            .filter(|next| *next <= MAX_SAFE_SEQUENCE)
            .ok_or_else(|| CommandError::io("desktop preference sequence is exhausted"))?;
        let preferences = operation()?;
        let snapshot = DesktopPreferencesSnapshot {
            sequence: next_sequence,
            preferences,
        };
        *sequence = next_sequence;
        // A notification failure cannot roll back committed bytes. The command
        // response and the next get still carry the authoritative value.
        let _ = publish(&snapshot);
        Ok(snapshot)
    }

    fn get(&self) -> Result<DesktopPreferencesSnapshot, CommandError> {
        self.transact(
            || load_desktop_preferences().map_err(CommandError::io),
            |_| Ok(()),
        )
    }

    /// Existing hidden HUD windows must reload persisted values on each show.
    pub(crate) fn reload_and_publish<R: Runtime>(
        &self,
        app: &AppHandle<R>,
    ) -> Result<DesktopPreferencesSnapshot, CommandError> {
        self.transact(
            || load_desktop_preferences().map_err(CommandError::io),
            |snapshot| publish(app, snapshot),
        )
    }

    fn update<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        patch: DesktopPreferencesPatch,
    ) -> Result<DesktopPreferencesSnapshot, CommandError> {
        self.transact(
            || update_desktop_preferences(patch).map_err(CommandError::io),
            |snapshot| publish(app, snapshot),
        )
    }
}

fn publish<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: &DesktopPreferencesSnapshot,
) -> Result<(), CommandError> {
    // Try both windows even if one webview has already been destroyed.
    let main = app.emit_to(MAIN_WINDOW_LABEL, PREFERENCES_CHANGED_EVENT, snapshot);
    let hud = app.emit_to(HUD_WINDOW_LABEL, PREFERENCES_CHANGED_EVENT, snapshot);
    main.and(hud).map_err(CommandError::io)
}

#[tauri::command]
pub(crate) async fn desktop_preferences_get(
    state: State<'_, DesktopPreferencesCoordinator>,
) -> Result<DesktopPreferencesSnapshot, CommandError> {
    let coordinator = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || coordinator.get())
        .await
        .map_err(CommandError::io)?
}

#[tauri::command]
pub(crate) async fn desktop_preferences_update(
    app: AppHandle,
    state: State<'_, DesktopPreferencesCoordinator>,
    patch: DesktopPreferencesPatch,
) -> Result<DesktopPreferencesSnapshot, CommandError> {
    let coordinator = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || coordinator.update(&app, patch))
        .await
        .map_err(CommandError::io)?
}

#[cfg(test)]
mod tests {
    use super::*;
    use devsweep_core::desktop_preferences::DesktopTheme;
    use std::thread;

    #[test]
    fn reads_commits_and_notifications_share_one_positive_sequence() {
        let coordinator = DesktopPreferencesCoordinator::default();
        let mut committed = DesktopPreferencesV1::default();
        let mut events = Vec::new();
        let initial = coordinator.transact(|| Ok(committed), |_| Ok(())).unwrap();
        assert_eq!(initial.sequence, 1);
        let updated = coordinator
            .transact(
                || {
                    committed.theme = DesktopTheme::Light;
                    Ok(committed)
                },
                |snapshot| {
                    events.push(*snapshot);
                    Ok(())
                },
            )
            .unwrap();
        let reloaded = coordinator.transact(|| Ok(committed), |_| Ok(())).unwrap();
        assert_eq!(events, [updated]);
        assert_eq!(updated.sequence, 2);
        assert_eq!(reloaded.sequence, 3);
        assert_eq!(reloaded.preferences, updated.preferences);
    }

    #[test]
    fn failed_commit_publishes_nothing_and_preserves_ordered_view() {
        let coordinator = DesktopPreferencesCoordinator::default();
        let committed = DesktopPreferencesV1::default();
        coordinator.transact(|| Ok(committed), |_| Ok(())).unwrap();
        assert!(
            coordinator
                .transact(
                    || Err(CommandError::io("injected save failure")),
                    |_| panic!("a failed save cannot publish"),
                )
                .is_err()
        );
        let next = coordinator.transact(|| Ok(committed), |_| Ok(())).unwrap();
        assert_eq!(next.sequence, 2);
        assert_eq!(next.preferences, committed);
    }

    #[test]
    fn event_delivery_failure_does_not_report_a_committed_write_as_failed() {
        let coordinator = DesktopPreferencesCoordinator::default();
        let committed = DesktopPreferencesV1 {
            theme: DesktopTheme::Light,
            ..DesktopPreferencesV1::default()
        };
        let updated = coordinator
            .transact(
                || Ok(committed),
                |_| Err(CommandError::io("webview was destroyed")),
            )
            .unwrap();
        let read = coordinator.transact(|| Ok(committed), |_| Ok(())).unwrap();
        assert_eq!(updated.sequence, 1);
        assert_eq!(read.sequence, 2);
        assert_eq!(read.preferences, committed);
    }

    #[test]
    fn concurrent_updates_publish_in_commit_order() {
        let coordinator = DesktopPreferencesCoordinator::default();
        let published = Arc::new(Mutex::new(Vec::new()));
        let barrier = Arc::new(std::sync::Barrier::new(16));
        let threads = (0..16)
            .map(|_| {
                let coordinator = coordinator.clone();
                let published = Arc::clone(&published);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    coordinator
                        .transact(
                            || Ok(DesktopPreferencesV1::default()),
                            |snapshot| {
                                published.lock().unwrap().push(snapshot.sequence);
                                Ok(())
                            },
                        )
                        .unwrap()
                })
            })
            .collect::<Vec<_>>();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(*published.lock().unwrap(), (1..=16).collect::<Vec<_>>());
    }

    #[test]
    fn exhausted_safe_integer_sequence_refuses_before_persistence() {
        let coordinator = DesktopPreferencesCoordinator(Arc::new(Mutex::new(MAX_SAFE_SEQUENCE)));
        assert!(
            coordinator
                .transact(|| panic!("exhausted sequence cannot commit"), |_| Ok(()),)
                .is_err()
        );
        assert_eq!(*coordinator.0.lock().unwrap(), MAX_SAFE_SEQUENCE);
    }
}
