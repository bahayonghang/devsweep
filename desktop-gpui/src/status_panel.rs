//! Shared Status main-panel and tray subscription.
//!
//! One running sampler bit. Main and tray visibility share it. This module
//! projects `AvailabilityV1` into panel text. It does not open a sensor API.
//! A displayed PID is presentation only and is not termination authority.

use std::fmt;

use devsweep_core::status::AvailabilityV1;

/// Why a displayed PID cannot authorize a kill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayedPidError;

impl fmt::Display for DisplayedPidError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("displayed PID is not termination authority")
    }
}

impl std::error::Error for DisplayedPidError {}

/// Main/tray visibility and the single running subscription bit.
#[derive(Debug)]
pub struct StatusPanel {
    main_visible: bool,
    tray_visible: bool,
    running: bool,
    sensor_line: Option<String>,
    displayed_pid: Option<u32>,
}

impl StatusPanel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            main_visible: false,
            tray_visible: false,
            running: false,
            sensor_line: None,
            displayed_pid: None,
        }
    }

    pub fn set_main_visible(&mut self, visible: bool) {
        self.main_visible = visible;
    }

    pub fn set_tray_visible(&mut self, visible: bool) {
        self.tray_visible = visible;
    }

    /// Start only when nothing was running and at least one panel is visible.
    /// Stop only when both are hidden and it was running.
    pub fn reconcile(&mut self, start: impl FnOnce(), stop: impl FnOnce()) {
        let want = self.main_visible || self.tray_visible;
        if want {
            if !self.running {
                start();
                self.running = true;
            }
        } else if self.running {
            stop();
            self.running = false;
        }
    }

    pub fn set_sensor<T>(&mut self, availability: &AvailabilityV1<T>) {
        self.sensor_line = Some(availability_line(availability));
    }

    #[must_use]
    pub fn sensor_line(&self) -> Option<&str> {
        self.sensor_line.as_deref()
    }

    pub fn set_displayed_pid(&mut self, pid: Option<u32>) {
        self.displayed_pid = pid;
    }

    #[must_use]
    pub fn displayed_pid(&self) -> Option<u32> {
        self.displayed_pid
    }

    #[must_use]
    pub fn displayed_pid_line(&self) -> Option<String> {
        self.displayed_pid.map(|pid| format!("pid {pid}"))
    }

    /// A displayed PID is not a kill plan. This never calls Status process
    /// termination.
    pub fn request_kill_displayed_pid(&self) -> Result<(), DisplayedPidError> {
        let _pid = self.displayed_pid;
        Err(DisplayedPidError)
    }
}

impl Default for StatusPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Honest availability text. Missing sensors keep their reason; they are never a `0` reading.
#[must_use]
pub fn availability_line<T>(availability: &AvailabilityV1<T>) -> String {
    match availability {
        AvailabilityV1::Unavailable { reason_code, .. } => {
            format!("unavailable {reason_code}")
        }
        AvailabilityV1::PermissionDenied { reason_code, .. } => {
            format!("permission_denied {reason_code}")
        }
        AvailabilityV1::Unsupported { reason_code, .. } => {
            format!("unsupported {reason_code}")
        }
        AvailabilityV1::Available { .. } => "available".to_string(),
        AvailabilityV1::Partial { reason_codes, .. } => {
            format!("partial {}", reason_codes.join(","))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{Mode, Presence, Session, ShellState};
    use crate::view::panel_lines;

    fn looks_like_zero_reading(line: &str) -> bool {
        line.split_whitespace()
            .any(|token| token == "0" || token == "0%" || token == "0.0" || token == "0.00")
    }

    #[test]
    fn status_tray_shares_one_subscription() {
        let mut starts = 0u32;
        let mut stops = 0u32;
        let mut panel = StatusPanel::new();

        let mut shell = ShellState::new();
        shell.set_presence(Presence::Hidden);
        assert!(!shell.request_decorative_frame());
        assert_eq!(shell.decorative_frames_scheduled(), 0);

        panel.reconcile(|| starts += 1, || stops += 1);
        assert_eq!(starts, 0, "both hidden must not start");
        assert_eq!(stops, 0);

        panel.set_main_visible(true);
        panel.reconcile(|| starts += 1, || stops += 1);
        assert_eq!(starts, 1);
        assert_eq!(stops, 0);

        panel.set_tray_visible(true);
        panel.reconcile(|| starts += 1, || stops += 1);
        assert_eq!(starts, 1, "tray while main is visible must not start again");
        assert_eq!(stops, 0);

        panel.set_main_visible(false);
        panel.reconcile(|| starts += 1, || stops += 1);
        assert_eq!(starts, 1);
        assert_eq!(stops, 0, "hiding main while tray stays must not stop");

        panel.set_tray_visible(false);
        panel.reconcile(|| starts += 1, || stops += 1);
        assert_eq!(starts, 1);
        assert_eq!(stops, 1, "hiding both must stop once");

        panel.set_main_visible(true);
        panel.reconcile(|| starts += 1, || stops += 1);
        assert_eq!(starts, 2, "a later show must start once more");
        assert_eq!(stops, 1);

        let missing = availability_line(&AvailabilityV1::<()>::unavailable("counter_missing"));
        assert!(missing.contains("counter_missing"));
        assert!(missing.contains("unavailable"));
        assert!(!looks_like_zero_reading(&missing));

        let denied = availability_line(&AvailabilityV1::<()>::permission_denied("access_denied"));
        assert!(denied.contains("access_denied"));
        assert!(denied.contains("permission_denied"));
        assert!(!looks_like_zero_reading(&denied));

        let unsupported =
            availability_line(&AvailabilityV1::<()>::unsupported("platform_unsupported"));
        assert!(unsupported.contains("platform_unsupported"));
        assert!(unsupported.contains("unsupported"));
        assert!(!looks_like_zero_reading(&unsupported));

        let mut session = Session::unbacked();
        session.show_mode(Mode::Status);
        let mut status = StatusPanel::new();
        status.set_sensor(&AvailabilityV1::<()>::unavailable("counter_missing"));
        session.set_status_panel(Some(status));
        let lines = panel_lines(&session, None);
        let sensor = lines
            .iter()
            .find(|line| line.contains("counter_missing") && line.contains("unavailable"))
            .cloned()
            .expect("Status panel_lines must append the sensor line");
        assert_eq!(
            lines.iter().filter(|line| *line == &sensor).count(),
            1,
            "append that one sensor line: {lines:?}"
        );
        assert!(
            !lines.iter().any(|line| looks_like_zero_reading(line)),
            "Status panel_lines must not append a zero: {lines:?}"
        );
    }

    #[test]
    fn displayed_pid_does_not_kill() {
        const PID: u32 = 424242;
        let mut panel = StatusPanel::new();
        panel.set_displayed_pid(Some(PID));
        assert_eq!(panel.displayed_pid(), Some(PID));

        let error = panel
            .request_kill_displayed_pid()
            .expect_err("displayed PID is not termination authority");
        let text = error.to_string();
        assert!(
            text.contains("displayed PID is not termination authority"),
            "{text}"
        );
        let lower = text.to_ascii_lowercase();
        assert!(!lower.contains("terminated"), "{text}");
        assert!(!lower.contains("succeeded"), "{text}");
        assert!(!lower.contains("succeed"), "{text}");

        let mut session = Session::unbacked();
        session.show_mode(Mode::Status);
        session.set_status_panel(Some(panel));
        let lines = panel_lines(&session, None);
        let pid_line = format!("pid {PID}");
        assert_eq!(
            lines.iter().filter(|line| *line == &pid_line).count(),
            1,
            "Status panel_lines must append exactly one pid line: {lines:?}"
        );
        assert_eq!(
            lines.iter().filter(|line| line.starts_with("pid ")).count(),
            1,
            "no other PID line: {lines:?}"
        );

        let source = include_str!("status_panel.rs");
        assert!(!source.contains(&["terminate", "_bound_process"].concat()));
    }
}
