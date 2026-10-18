//! Presentation session for Clean Screen and Keep Screen On.
//!
//! Clean Screen remembers the current mode and surface and covers the view.
//! Escape restores that pair through [`Session::apply_key`]. Keep Screen On
//! calls `run` on a caller-supplied C08 lease service and never constructs a
//! production backend.

use std::sync::Arc;
use std::time::Duration;

use devsweep_core::status::{KeepAwakeError, KeepAwakeOwner, KeepAwakeService, KeepAwakeStatusV1};

use crate::shell::{Mode, NavKey, Session, Surface};

struct ScreenOnLease {
    service: Arc<KeepAwakeService>,
    token: String,
    owner_name: String,
    #[allow(dead_code)]
    owner: KeepAwakeOwner,
}

pub struct PresentationSession {
    session: Session,
    lease: Option<ScreenOnLease>,
}

impl PresentationSession {
    #[must_use]
    pub fn new(session: Session) -> Self {
        Self {
            session,
            lease: None,
        }
    }

    pub fn show_mode(&mut self, mode: Mode) {
        self.session.show_mode(mode);
    }

    pub fn begin_clean_screen(&mut self) {
        self.session.begin_clean_screen();
    }

    pub fn apply_key(&mut self, key: NavKey) {
        self.session.apply_key(key);
    }

    #[must_use]
    pub fn mode(&self) -> Mode {
        self.session.shell().mode()
    }

    #[must_use]
    pub fn surface(&self) -> Surface {
        self.session.shell().surface()
    }

    #[must_use]
    pub fn screen_covered(&self) -> bool {
        self.session.screen_covered()
    }

    #[must_use]
    pub fn input_locked(&self) -> bool {
        self.session.input_locked()
    }

    #[must_use]
    pub fn stored_lease_count(&self) -> usize {
        usize::from(self.lease.is_some())
    }

    pub fn start_screen_on(
        &mut self,
        service: Arc<KeepAwakeService>,
        owner: &str,
        duration: Duration,
    ) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        if self.lease.is_some() {
            let _ = self.stop_screen_on();
        }
        let status = service.run(owner, duration, true)?;
        self.lease = Some(ScreenOnLease {
            owner: KeepAwakeOwner::new(Arc::clone(&service), owner),
            token: status.token.clone(),
            owner_name: owner.to_string(),
            service,
        });
        Ok(status)
    }

    pub fn stop_screen_on(&mut self) -> Result<KeepAwakeStatusV1, KeepAwakeError> {
        let Some(lease) = self.lease.take() else {
            return Err(KeepAwakeError::UnknownToken);
        };
        lease.service.stop(&lease.token, &lease.owner_name)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use devsweep_core::status::{
        BackendCall, FakeClock, KeepAwakeClock, KeepAwakeService, RecordingBackend,
    };

    use super::*;
    use crate::shell::Session;

    #[test]
    fn clean_screen_escape_restores_view_and_releases_lease() {
        let clock = Arc::new(FakeClock::new());
        let (backend, log) = RecordingBackend::pair();
        let service = Arc::new(KeepAwakeService::new(
            Arc::clone(&clock) as Arc<dyn KeepAwakeClock>,
            Box::new(backend),
            true,
        ));

        let mut session = PresentationSession::new(Session::unbacked());
        assert!(!session.input_locked());

        let status = session
            .start_screen_on(Arc::clone(&service), "gpui-test", Duration::from_secs(1))
            .expect("lease");
        assert!(!status.power_settings_changed);
        assert_eq!(service.stored_lease_count(), 1);
        assert_eq!(session.stored_lease_count(), 1);

        session.show_mode(Mode::Software);
        assert_eq!(session.surface(), Surface::Mode);
        session.begin_clean_screen();
        assert!(session.screen_covered());
        assert!(!session.input_locked());
        session.apply_key(NavKey::Right);
        assert_eq!(session.mode(), Mode::Software);
        assert_eq!(session.surface(), Surface::Mode);
        session.apply_key(NavKey::Left);
        assert_eq!(session.mode(), Mode::Software);
        session.apply_key(NavKey::Escape);
        assert!(!session.screen_covered());
        assert_eq!(session.mode(), Mode::Software);
        assert_eq!(session.surface(), Surface::Mode);
        assert!(!session.input_locked());
        session.apply_key(NavKey::Right);
        assert_eq!(session.mode(), Mode::Optimize);
        assert!(!session.input_locked());

        let token = status.token.clone();
        drop(session);
        assert_eq!(
            service
                .status(&token, "gpui-test")
                .expect("owner exit")
                .state,
            "owner_exited"
        );
        assert!(
            log.lock()
                .expect("log")
                .iter()
                .any(|call| matches!(call, BackendCall::Release(_)))
        );

        let mut session = PresentationSession::new(Session::unbacked());
        session
            .start_screen_on(Arc::clone(&service), "gpui-test", Duration::from_secs(1))
            .expect("stop target");
        assert_eq!(session.stored_lease_count(), 1);
        let stopped = session.stop_screen_on().expect("stop");
        assert_eq!(stopped.state, "stopped");
        assert_eq!(session.stored_lease_count(), 0);
        assert!(!session.input_locked());

        let source = include_str!("screen.rs");
        assert!(!source.contains(&["power", "cfg"].concat()));
    }
}
