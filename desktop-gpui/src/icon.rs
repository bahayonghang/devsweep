//! Embedded DevSweep beaver icon shared by the window and the notification area.
//!
//! `assets/devsweep.ico` is generated offline from
//! `assets/devsweep-icon-master.png` (1254x1254) with Pillow 13.0.0. Each of the
//! 16, 32, 48, and 256 pixel images is a premultiplied-alpha LANCZOS resize
//! stored as a 32 bpp bottom-up BGRA BMP entry with its AND mask under one
//! `ICONDIR`. There is no download and no runtime conversion. The legacy Tauri
//! `icon.ico` is not the beaver and is not used.

const ICO_BYTES: &[u8] = include_bytes!("../assets/devsweep.ico");
const ICONDIR_MAGIC: [u8; 4] = [0, 0, 1, 0];
const ICONDIR_HEADER_BYTES: usize = 6;
const ICONDIR_ENTRY_BYTES: usize = 16;

/// One `ICONDIRENTRY` resolved to its slice of the embedded file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IconEntry {
    pub width: u32,
    pub height: u32,
    pub image_offset: usize,
    pub image_len: usize,
}

/// Parse the embedded `ICONDIR`. A malformed file yields no entries, and the
/// loader then leaves the platform default in place.
pub(crate) fn parse_entries(ico: &[u8]) -> Vec<IconEntry> {
    if !ico.starts_with(&ICONDIR_MAGIC) {
        return Vec::new();
    }
    let Some(count) = read_u16(ico, 4) else {
        return Vec::new();
    };
    let mut entries = Vec::with_capacity(usize::from(count));
    for index in 0..usize::from(count) {
        let at = ICONDIR_HEADER_BYTES + index * ICONDIR_ENTRY_BYTES;
        let (Some(width), Some(height)) = (ico.get(at).copied(), ico.get(at + 1).copied()) else {
            return Vec::new();
        };
        let (Some(image_len), Some(image_offset)) = (read_u32(ico, at + 8), read_u32(ico, at + 12))
        else {
            return Vec::new();
        };
        let (Ok(image_len), Ok(image_offset)) =
            (usize::try_from(image_len), usize::try_from(image_offset))
        else {
            return Vec::new();
        };
        let Some(end) = image_offset.checked_add(image_len) else {
            return Vec::new();
        };
        if end > ico.len() {
            return Vec::new();
        }
        entries.push(IconEntry {
            // A zero width or height byte means 256 pixels.
            width: if width == 0 { 256 } else { u32::from(width) },
            height: if height == 0 { 256 } else { u32::from(height) },
            image_offset,
            image_len,
        });
    }
    entries
}

/// Pick the smallest entry at least `desired` pixels wide, else the largest.
pub(crate) fn select_entry(entries: &[IconEntry], desired: u32) -> Option<IconEntry> {
    entries
        .iter()
        .filter(|entry| entry.width >= desired)
        .min_by_key(|entry| entry.width)
        .or_else(|| entries.iter().max_by_key(|entry| entry.width))
        .copied()
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

#[cfg(windows)]
pub(crate) use windows_part::{WindowIcons, load_hicon};

#[cfg(windows)]
mod windows_part {
    use gpui::Window;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::WindowsAndMessaging::{
            CreateIconFromResourceEx, GetSystemMetrics, HICON, ICON_BIG, ICON_SMALL,
            LR_DEFAULTCOLOR, SM_CXICON, SM_CXSMICON, SYSTEM_METRICS_INDEX, SendMessageW,
            WM_SETICON,
        },
    };

    use super::{ICO_BYTES, parse_entries, select_entry};

    const ICON_RESOURCE_VERSION: u32 = 0x0003_0000;

    /// HICONs applied to the shell window. The struct lives for the whole
    /// `view::run` call, so the handles stay valid until the process exits.
    pub(crate) struct WindowIcons {
        small: Option<HICON>,
        large: Option<HICON>,
    }

    impl WindowIcons {
        pub(crate) fn load() -> Self {
            Self {
                small: load_hicon(system_metric(SM_CXSMICON, 16)),
                large: load_hicon(system_metric(SM_CXICON, 32)),
            }
        }

        /// Apply `WM_SETICON` to the real HWND. Returns whether any icon was
        /// applied; on failure the window keeps the platform default.
        pub(crate) fn apply(&self, window: &mut Window) -> bool {
            let Ok(handle) = window.window_handle() else {
                return false;
            };
            let RawWindowHandle::Win32(handle) = handle.as_raw() else {
                return false;
            };
            let hwnd = HWND(handle.hwnd.get() as *mut core::ffi::c_void);
            let mut applied = false;
            for (id, icon) in [(ICON_SMALL, self.small), (ICON_BIG, self.large)] {
                let Some(icon) = icon else {
                    continue;
                };
                unsafe {
                    SendMessageW(
                        hwnd,
                        WM_SETICON,
                        Some(WPARAM(id as usize)),
                        Some(LPARAM(icon.0 as isize)),
                    );
                }
                applied = true;
            }
            applied
        }
    }

    /// Create one HICON from the best embedded entry. The caller owns the
    /// handle; `CreateIconFromResourceEx` does not cache it.
    pub(crate) fn load_hicon(desired: u32) -> Option<HICON> {
        let entries = parse_entries(ICO_BYTES);
        let entry = select_entry(&entries, desired)?;
        let image = ICO_BYTES.get(entry.image_offset..entry.image_offset + entry.image_len)?;
        unsafe {
            CreateIconFromResourceEx(
                image,
                true,
                ICON_RESOURCE_VERSION,
                entry.width as i32,
                entry.height as i32,
                LR_DEFAULTCOLOR,
            )
        }
        .ok()
    }

    fn system_metric(index: SYSTEM_METRICS_INDEX, fallback: u32) -> u32 {
        let value = unsafe { GetSystemMetrics(index) };
        u32::try_from(value)
            .ok()
            .filter(|value| *value > 0)
            .unwrap_or(fallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_ico_is_a_multi_size_icondir() {
        assert_eq!(&ICO_BYTES[..4], &ICONDIR_MAGIC, "ICONDIR magic");
        let entries = parse_entries(ICO_BYTES);
        assert!(entries.len() >= 4, "at least four icon entries");
        let widths: Vec<u32> = entries.iter().map(|entry| entry.width).collect();
        for expected in [16, 32, 48, 256] {
            assert!(
                widths.contains(&expected),
                "missing {expected} px: {widths:?}"
            );
        }
        assert!(
            entries
                .iter()
                .all(|entry| entry.image_offset + entry.image_len <= ICO_BYTES.len()),
            "every entry must fit inside the embedded file"
        );
    }

    #[test]
    fn size_selection_prefers_the_smallest_fitting_entry() {
        let entries = parse_entries(ICO_BYTES);
        let width = |desired| select_entry(&entries, desired).map(|entry| entry.width);
        assert_eq!(width(16), Some(16));
        assert_eq!(width(20), Some(32), "20 px rounds up to 32");
        assert_eq!(width(32), Some(32));
        assert_eq!(width(48), Some(48));
        assert_eq!(width(64), Some(256), "64 px rounds up to 256");
        assert_eq!(
            width(512),
            Some(256),
            "larger requests fall back to the max"
        );
        assert_eq!(select_entry(&[], 16), None, "empty directory has no icon");
        assert_eq!(parse_entries(b"not an icon"), Vec::new());
    }
}
