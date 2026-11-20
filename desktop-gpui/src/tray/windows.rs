use crate::copy::Locale;
use async_channel::Sender;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{cell::Cell, rc::Rc};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint},
        UI::{Shell::*, WindowsAndMessaging::*},
    },
    core::w,
};

pub(crate) const CALLBACK: u32 = WM_APP + 19;
const ICON_ID: u32 = 1;
const KEY_SELECT: u32 = NIN_SELECT + 1;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Event {
    Toggle(POINT),
    Open,
    Exit,
    Visibility,
    Awake,
}

type Notify = fn(NOTIFY_ICON_MESSAGE, &NOTIFYICONDATAW) -> bool;
pub(crate) fn notify_shell(message: NOTIFY_ICON_MESSAGE, data: &NOTIFYICONDATAW) -> bool {
    unsafe { Shell_NotifyIconW(message, data).as_bool() }
}

struct State {
    sender: Sender<Event>,
    added: Cell<bool>,
    language: Cell<Locale>,
    tucked: Cell<bool>,
    icon: Option<TrayIcon>,
    taskbar_created: u32,
    notify: Notify,
}

impl State {
    fn data(&self, hwnd: HWND) -> NOTIFYICONDATAW {
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: ICON_ID,
            uCallbackMessage: CALLBACK,
            uFlags: NIF_MESSAGE | NIF_TIP | NIF_SHOWTIP,
            ..Default::default()
        };
        if let Some(icon) = &self.icon {
            data.hIcon = icon.handle;
            data.uFlags |= NIF_ICON;
        }
        let tip = match (self.tucked.get(), self.language.get()) {
            (false, _) => "DevSweep",
            (true, Locale::En) => "DevSweep is running in the tray. Right-click to reopen.",
            (true, Locale::ZhCn) => "DevSweep 正在托盘中运行。右键可重新打开主窗口。",
        };
        for (dest, unit) in data.szTip.iter_mut().zip(tip.encode_utf16()) {
            *dest = unit;
        }
        data
    }

    fn register(&self, hwnd: HWND) {
        let mut data = self.data(hwnd);
        self.added.set(false);
        if (self.notify)(NIM_ADD, &data) {
            data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            if (self.notify)(NIM_SETVERSION, &data) {
                self.added.set(true);
            } else {
                (self.notify)(NIM_DELETE, &data);
            }
        }
    }
    fn emit(&self, event: Event) {
        let _ = self.sender.try_send(event);
    }
}

pub(crate) struct Tray {
    receiver: Option<HWND>,
    state: Rc<State>,
}

impl Tray {
    pub(crate) fn new(sender: Sender<Event>, language: Locale, notify: Notify) -> Self {
        let state = Rc::new(State {
            sender,
            added: Cell::new(false),
            language: Cell::new(language),
            tucked: Cell::new(false),
            icon: TrayIcon::load(),
            notify,
            taskbar_created: unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) },
        });
        // TaskbarCreated is a broadcast. A hidden top-level window receives it,
        // including when Explorer did not exist at initial application startup.
        let receiver = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("DevSweep Tray Receiver"),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                None,
                None,
            )
        }
        .ok()
        .and_then(|hwnd| {
            if subclass(hwnd, receiver_proc, &state) {
                state.register(hwnd);
                Some(hwnd)
            } else {
                let _ = unsafe { DestroyWindow(hwnd) };
                None
            }
        });
        Self { receiver, state }
    }

    pub(crate) fn set_tucked(&self, tucked: bool) {
        if self.state.tucked.replace(tucked) != tucked
            && self.available()
            && let Some(hwnd) = self.receiver
        {
            (self.state.notify)(NIM_MODIFY, &self.state.data(hwnd));
        }
    }
    pub(crate) fn available(&self) -> bool {
        self.state.added.get()
    }
    pub(crate) fn set_locale(&self, locale: Locale) {
        self.state.language.set(locale);
    }
    pub(crate) fn watch_main(&self, window: &gpui::Window) {
        if let Some(hwnd) = window_hwnd(window) {
            subclass(hwnd, main_proc, &self.state);
        }
    }
    pub(crate) fn emit(&self, event: Event) {
        self.state.emit(event);
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        if let Some(hwnd) = self.receiver.take() {
            if self.state.added.replace(false) {
                (self.state.notify)(NIM_DELETE, &self.state.data(hwnd));
            }
            let _ = unsafe { DestroyWindow(hwnd) };
        }
    }
}

fn subclass(
    hwnd: HWND,
    proc: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM, usize, usize) -> LRESULT,
    state: &Rc<State>,
) -> bool {
    let data = Rc::into_raw(state.clone());
    if unsafe { SetWindowSubclass(hwnd, Some(proc), 1, data as usize) }.as_bool() {
        true
    } else {
        drop(unsafe { Rc::from_raw(data) });
        false
    }
}

unsafe extern "system" fn main_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    let state = unsafe { &*(data as *const State) };
    if matches!(msg, WM_SHOWWINDOW | WM_SIZE) {
        state.emit(Event::Visibility);
    }
    let result = unsafe { DefSubclassProc(hwnd, msg, w, l) };
    if msg == WM_NCDESTROY {
        drop(unsafe { Rc::from_raw(data as *const State) });
    }
    result
}

unsafe extern "system" fn receiver_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    let state = unsafe { &*(data as *const State) };
    if msg != 0 && msg == state.taskbar_created {
        state.register(hwnd);
        if !state.added.get() {
            state.emit(Event::Open);
        }
        return LRESULT(0);
    }
    if msg == CALLBACK && ((l.0 as u32) >> 16) == ICON_ID && state.added.get() {
        let point = icon_anchor(
            hwnd,
            POINT {
                x: w.0 as i16 as i32,
                y: (w.0 >> 16) as i16 as i32,
            },
        );
        match l.0 as u16 as u32 {
            NIN_SELECT | KEY_SELECT => state.emit(Event::Toggle(point)),
            WM_CONTEXTMENU => context_menu(hwnd, point, state),
            _ => {}
        }
        return LRESULT(0);
    }
    let result = unsafe { DefSubclassProc(hwnd, msg, w, l) };
    if msg == WM_NCDESTROY {
        drop(unsafe { Rc::from_raw(data as *const State) });
    }
    result
}

fn icon_anchor(hwnd: HWND, fallback: POINT) -> POINT {
    let identifier = NOTIFYICONIDENTIFIER {
        cbSize: std::mem::size_of::<NOTIFYICONIDENTIFIER>() as u32,
        hWnd: hwnd,
        uID: ICON_ID,
        ..Default::default()
    };
    if let Ok(rect) = unsafe { Shell_NotifyIconGetRect(&identifier) } {
        POINT {
            x: rect.right,
            y: rect.top,
        }
    } else if fallback.x == -1 && fallback.y == -1 {
        let mut point = POINT::default();
        let _ = unsafe { GetCursorPos(&mut point) };
        point
    } else {
        fallback
    }
}

fn context_menu(hwnd: HWND, point: POINT, state: &State) {
    let Ok(menu) = (unsafe { CreatePopupMenu() }) else {
        return;
    };
    let (open, exit) = match state.language.get() {
        Locale::En => (w!("&Open DevSweep"), w!("E&xit")),
        Locale::ZhCn => (w!("打开 DevSweep(&O)"), w!("退出(&X)")),
    };
    unsafe {
        if AppendMenuW(menu, MF_STRING, 1, open).is_ok()
            && AppendMenuW(menu, MF_STRING, 2, exit).is_ok()
        {
            let _ = SetForegroundWindow(hwnd);
            let selected = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_RIGHTALIGN | TPM_BOTTOMALIGN,
                point.x,
                point.y,
                None,
                hwnd,
                None,
            )
            .0;
            let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
            match selected {
                1 => state.emit(Event::Open),
                2 => state.emit(Event::Exit),
                _ => {
                    (state.notify)(NIM_SETFOCUS, &state.data(hwnd));
                }
            }
        }
        let _ = DestroyMenu(menu);
    }
}

fn window_hwnd(window: &gpui::Window) -> Option<HWND> {
    let handle = HasWindowHandle::window_handle(window).ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Some(HWND(handle.hwnd.get() as *mut _)),
        _ => None,
    }
}

pub(crate) fn main_visible(window: &gpui::Window) -> bool {
    window_hwnd(window)
        .is_some_and(|hwnd| unsafe { IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool() })
}

pub(crate) fn hide(window: &gpui::Window) -> bool {
    window_hwnd(window).is_some_and(|hwnd| unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
        !IsWindowVisible(hwnd).as_bool()
    })
}

pub(crate) fn restore(window: &gpui::Window) {
    if let Some(hwnd) = window_hwnd(window) {
        unsafe {
            let _ = ShowWindow(
                hwnd,
                if IsIconic(hwnd).as_bool() {
                    SW_RESTORE
                } else {
                    SW_SHOW
                },
            );
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

// GetWindowRect and rcWork are physical desktop coordinates in this
// per-monitor-aware process. Re-read dimensions after crossing a DPI boundary.
pub(crate) fn place_panel(window: &gpui::Window, anchor: POINT) {
    let Some(hwnd) = window_hwnd(window) else {
        return;
    };
    let monitor = unsafe { MonitorFromPoint(anchor, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return;
    }
    let work = info.rcWork;
    for _ in 0..2 {
        let mut rect = RECT::default();
        if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
            break;
        }
        let width = (rect.right - rect.left).min(work.right - work.left);
        let height = (rect.bottom - rect.top).min(work.bottom - work.top);
        let x = (anchor.x - width).clamp(work.left, work.right - width);
        let y = (anchor.y - height).clamp(work.top, work.bottom - height);
        let _ = unsafe {
            SetWindowPos(
                hwnd,
                None,
                x,
                y,
                width,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
        };
    }
}
struct TrayIcon {
    handle: windows::Win32::UI::WindowsAndMessaging::HICON,
    /// `CreateIconFromResourceEx` handles are ours to destroy; shared system
    /// icons from `LoadIconW` are not.
    owned: bool,
}

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

impl Drop for TrayIcon {
    fn drop(&mut self) {
        if self.owned {
            let _ = unsafe { DestroyIcon(self.handle) };
        }
    }
}
