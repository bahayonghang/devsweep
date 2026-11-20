//! Windows notification icon and messages. The application owns both HWNDs on
//! the GPUI UI thread. Callback handlers enqueue events without borrowing GPUI.
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::*;
