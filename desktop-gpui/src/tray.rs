//! Process-lifetime Windows notification-area icon.
//!
//! The shell adds the icon when the app runs and removes it when the guard
//! drops. Tests use [`RecordingTray`] and do not call `Shell_NotifyIcon`.

#[cfg(test)]
use std::cell::RefCell;
#[cfg(test)]
use std::rc::Rc;

pub trait TraySink {
    fn add(&mut self, tip: &str) -> bool;
    fn remove(&mut self);
}

#[must_use]
pub struct TrayGuard<S: TraySink> {
    sink: Option<S>,
}

impl<S: TraySink> Drop for TrayGuard<S> {
    fn drop(&mut self) {
        if let Some(mut sink) = self.sink.take() {
            sink.remove();
        }
    }
}

pub fn install_tray<S: TraySink>(mut sink: S) -> TrayGuard<S> {
    let added = sink.add("DevSweep");
    TrayGuard {
        sink: added.then_some(sink),
    }
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrayCall {
    Add(String),
    Remove,
}

#[cfg(test)]
#[derive(Clone, Debug, Default)]
pub struct RecordingTray {
    calls: Rc<RefCell<Vec<TrayCall>>>,
}

#[cfg(test)]
impl RecordingTray {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn calls(&self) -> Vec<TrayCall> {
        self.calls.borrow().clone()
    }
}

#[cfg(test)]
impl TraySink for RecordingTray {
    fn add(&mut self, tip: &str) -> bool {
        self.calls.borrow_mut().push(TrayCall::Add(tip.to_string()));
        true
    }

    fn remove(&mut self) {
        self.calls.borrow_mut().push(TrayCall::Remove);
    }
}

#[cfg(windows)]
pub struct RealTray {
    hwnd: Option<windows::Win32::Foundation::HWND>,
    icon: Option<TrayIcon>,
    added: bool,
}

#[cfg(windows)]
struct TrayIcon {
    handle: windows::Win32::UI::WindowsAndMessaging::HICON,
    /// `CreateIconFromResourceEx` handles are ours to destroy; shared system
    /// icons from `LoadIconW` are not.
    owned: bool,
}

#[cfg(windows)]
impl RealTray {
    pub fn new() -> Self {
        Self {
            hwnd: None,
            icon: None,
            added: false,
        }
    }

    fn icon(&mut self) -> Option<windows::Win32::UI::WindowsAndMessaging::HICON> {
        if self.icon.is_none() {
            self.icon = TrayIcon::load();
        }
        self.icon.as_ref().map(|icon| icon.handle)
    }

    fn destroy_hwnd(&mut self) {
        if let Some(hwnd) = self.hwnd.take() {
            let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd) };
        }
    }
}

#[cfg(windows)]
impl TrayIcon {
    fn load() -> Option<Self> {
        use windows::Win32::UI::WindowsAndMessaging::{HICON, IDI_APPLICATION, LoadIconW};

        if let Some(handle) = crate::icon::load_hicon(small_icon_size()) {
            return Some(Self {
                handle,
                owned: true,
            });
        }
        // Last resort for a malformed embedded icon: the generic application
        // icon. Shared by the system, so it is never destroyed here.
        unsafe { LoadIconW(None, IDI_APPLICATION) }
            .ok()
            .filter(|icon: &HICON| !icon.is_invalid())
            .map(|handle| Self {
                handle,
                owned: false,
            })
    }
}

#[cfg(windows)]
fn small_icon_size() -> u32 {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSMICON};

    let value = unsafe { GetSystemMetrics(SM_CXSMICON) };
    u32::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .unwrap_or(16)
}

#[cfg(windows)]
impl Default for RealTray {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(windows)]
impl TraySink for RealTray {
    fn add(&mut self, tip: &str) -> bool {
        if self.added {
            return true;
        }
        let Some(hwnd) = create_message_hwnd() else {
            return false;
        };
        self.hwnd = Some(hwnd);
        let data = notify_data(hwnd, tip, self.icon());
        let ok = unsafe {
            windows::Win32::UI::Shell::Shell_NotifyIconW(windows::Win32::UI::Shell::NIM_ADD, &data)
        }
        .as_bool();
        if ok {
            self.added = true;
            true
        } else {
            self.destroy_hwnd();
            false
        }
    }

    fn remove(&mut self) {
        if self.added {
            if let Some(hwnd) = self.hwnd {
                let data = notify_data(hwnd, "", self.icon.as_ref().map(|icon| icon.handle));
                let _ = unsafe {
                    windows::Win32::UI::Shell::Shell_NotifyIconW(
                        windows::Win32::UI::Shell::NIM_DELETE,
                        &data,
                    )
                };
            }
            self.added = false;
        }
        self.destroy_hwnd();
    }
}

#[cfg(windows)]
impl Drop for RealTray {
    fn drop(&mut self) {
        if self.added || self.hwnd.is_some() {
            self.remove();
        }
        if let Some(icon) = self.icon.take()
            && icon.owned
        {
            let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::DestroyIcon(icon.handle) };
        }
    }
}

#[cfg(windows)]
fn create_message_hwnd() -> Option<windows::Win32::Foundation::HWND> {
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::CreateWindowExW(
            windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
            windows::core::w!("STATIC"),
            windows::core::w!("DevSweep"),
            windows::Win32::UI::WindowsAndMessaging::WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            Some(windows::Win32::UI::WindowsAndMessaging::HWND_MESSAGE),
            None,
            None,
            None,
        )
    }
    .ok()
}

#[cfg(windows)]
fn notify_data(
    hwnd: windows::Win32::Foundation::HWND,
    tip: &str,
    icon: Option<windows::Win32::UI::WindowsAndMessaging::HICON>,
) -> windows::Win32::UI::Shell::NOTIFYICONDATAW {
    use windows::Win32::UI::Shell::{NIF_ICON, NIF_TIP, NOTIFYICONDATAW};

    let mut data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        ..Default::default()
    };
    let icon = icon.unwrap_or_default();
    data.hIcon = icon;
    write_tip(&mut data.szTip, tip);
    data.uFlags = if icon.is_invalid() {
        NIF_TIP
    } else {
        NIF_ICON | NIF_TIP
    };
    data
}

#[cfg(windows)]
fn write_tip(dest: &mut [u16; 128], tip: &str) {
    dest.fill(0);
    for (slot, unit) in dest.iter_mut().zip(tip.encode_utf16()).take(127) {
        *slot = unit;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status_panel::StatusPanel;

    #[test]
    fn tray_add_is_removed_on_drop() {
        let sink = RecordingTray::new();
        let starts = 0u32;
        let panel = StatusPanel::new();

        let mut guard = Some(install_tray(sink.clone()));
        assert_eq!(sink.calls(), [TrayCall::Add("DevSweep".to_string())]);

        drop(guard.take());
        assert_eq!(
            sink.calls(),
            [TrayCall::Add("DevSweep".to_string()), TrayCall::Remove,]
        );

        drop(guard.take());
        assert_eq!(
            sink.calls()
                .iter()
                .filter(|call| *call == &TrayCall::Remove)
                .count(),
            1,
            "a second drop must not remove again"
        );

        assert_eq!(starts, 0, "status subscription start count is unchanged");
        assert!(
            panel.sensor_line().is_none(),
            "StatusPanel was constructed and not started"
        );

        let source = crate::source_lf(include_str!("tray.rs"));
        let recording = source
            .split("impl TraySink for RecordingTray")
            .nth(1)
            .and_then(|rest| rest.split("#[cfg(windows)]").next())
            .expect("RecordingTray sink impl");
        assert!(
            !recording.contains(&["Shell_Notify", "Icon"].concat()),
            "RecordingTray must not call the notify-icon API"
        );
        assert!(
            !source.contains(&["NIM_", "SETVERSION"].concat()),
            "do not keep an icon after process exit"
        );
    }
}
