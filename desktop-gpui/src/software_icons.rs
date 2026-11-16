//! Bounded display-only icon cache and local Windows resource reads.
//! Requests are prepared without I/O; Session runs one small batch off-thread.

use std::{collections::VecDeque, sync::Arc};

use devsweep_core::software::IconEvidenceV1;
use gpui::RenderImage;

const CACHE_CAPACITY: usize = 256;
const BATCH_CAPACITY: usize = 4;
const ICON_PIXELS: u32 = 64;
#[cfg(windows)]
const MAX_DIMENSION: u32 = 512;
#[cfg(windows)]
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconRequest {
    pub id: String,
    pub evidence: IconEvidenceV1,
}

pub struct IconResult {
    pub request: IconRequest,
    pub pixels: Option<image::RgbaImage>,
}

#[derive(Default)]
pub struct SoftwareIcons {
    inventory: Option<String>,
    generation: u64,
    entries: VecDeque<(IconRequest, Option<Arc<RenderImage>>)>,
}

impl SoftwareIcons {
    pub fn synchronize(&mut self, inventory: Option<&str>) {
        if self.inventory.as_deref() != inventory {
            self.inventory = inventory.map(str::to_string);
            self.invalidate();
        }
    }

    pub fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.entries.clear();
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn image(&self, request: &IconRequest) -> Option<Arc<RenderImage>> {
        self.entries
            .iter()
            .find(|(key, _)| key == request)?
            .1
            .clone()
    }

    /// Both failed and successful reads occupy a cache slot, so bad evidence
    /// is not retried on every render. Only a refresh/evidence change retries.
    pub fn next_batch(&self, visible: impl Iterator<Item = IconRequest>) -> Vec<IconRequest> {
        visible
            .filter(|request| !self.entries.iter().any(|(key, _)| key == request))
            .take(BATCH_CAPACITY)
            .collect()
    }

    pub fn apply(&mut self, generation: u64, results: Vec<IconResult>) -> bool {
        if generation != self.generation {
            return false;
        }
        for result in results.into_iter().take(BATCH_CAPACITY) {
            self.entries.retain(|(key, _)| key != &result.request);
            let image = result.pixels.and_then(|pixels| {
                (pixels.width() <= ICON_PIXELS && pixels.height() <= ICON_PIXELS)
                    .then(|| Arc::new(RenderImage::new([image::Frame::new(pixels)])))
            });
            self.entries.push_back((result.request, image));
            if self.entries.len() > CACHE_CAPACITY {
                self.entries.pop_front();
            }
        }
        true
    }
}

/// This runs on the one outstanding icon job, never during rendering.
pub fn load_batch(requests: Vec<IconRequest>) -> Vec<IconResult> {
    requests
        .into_iter()
        .take(BATCH_CAPACITY)
        .map(|request| {
            let pixels = load_icon(&request.evidence);
            IconResult { request, pixels }
        })
        .collect()
}

/// Windows drive spelling only. Checking syntax must precede any OS path call.
#[cfg(any(windows, test))]
fn local_path(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    if bytes.len() < 4
        || bytes.len() > 32 * 1024
        || !bytes[0].is_ascii_alphabetic()
        || bytes[1] != b':'
        || !matches!(bytes[2], b'\\' | b'/')
        || value
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '"' | '%' | '<' | '>' | '|' | '?' | '*'))
        || value[2..].contains(':')
    {
        return None;
    }
    let value = value.replace('/', "\\");
    if value[3..].split('\\').any(|part| {
        let base = part
            .split('.')
            .next()
            .unwrap_or_default()
            .trim_end_matches(' ')
            .to_ascii_uppercase();
        let numbered_device = base
            .strip_prefix("COM")
            .or_else(|| base.strip_prefix("LPT"));
        part.is_empty()
            || matches!(part, "." | "..")
            || part.ends_with(['.', ' '])
            || matches!(
                base.as_str(),
                "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
            )
            || matches!(
                numbered_device,
                Some("1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³")
            )
    }) {
        return None;
    }
    Some(value)
}

/// SUBST, mapped shares, DOS aliases and remote redirectors fail before a stat.
#[cfg(any(windows, test))]
fn native_disk_mapping(mapping: &str) -> bool {
    mapping
        .strip_prefix("\\Device\\HarddiskVolume")
        .is_some_and(|suffix| {
            !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
        })
}

#[cfg(not(windows))]
fn load_icon(_: &IconEvidenceV1) -> Option<image::RgbaImage> {
    None
}

#[cfg(windows)]
fn load_icon(evidence: &IconEvidenceV1) -> Option<image::RgbaImage> {
    let path = local_path(&evidence.path)?;
    let mut guarded = native::open_local(&path)?;
    if guarded.file.metadata().ok()?.len() > MAX_FILE_BYTES {
        return None;
    }
    let extension = std::path::Path::new(&path)
        .extension()?
        .to_str()?
        .to_ascii_lowercase();
    let mut pixels = if extension == "png" {
        use std::io::Read;
        let mut bytes = Vec::new();
        guarded
            .file
            .by_ref()
            .take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return None;
        }
        let mut reader =
            image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(MAX_DIMENSION);
        limits.max_image_height = Some(MAX_DIMENSION);
        limits.max_alloc = Some(u64::from(MAX_DIMENSION) * u64::from(MAX_DIMENSION) * 8);
        reader.limits(limits);
        let mut pixels = reader.decode().ok()?.to_rgba8();
        // GPUI's in-memory image buffer is BGRA.
        for pixel in pixels.pixels_mut() {
            pixel.0.swap(0, 2);
        }
        pixels
    } else if matches!(extension.as_str(), "exe" | "dll" | "ico") {
        native::resource_pixels(&path, evidence.index)?
    } else {
        return None;
    };
    if pixels.width() == 0 || pixels.height() == 0 {
        return None;
    }
    if pixels.width() > ICON_PIXELS || pixels.height() > ICON_PIXELS {
        let ratio = (ICON_PIXELS as f32 / pixels.width() as f32)
            .min(ICON_PIXELS as f32 / pixels.height() as f32);
        pixels = image::imageops::resize(
            &pixels,
            (pixels.width() as f32 * ratio).round().max(1.0) as u32,
            (pixels.height() as f32 * ratio).round().max(1.0) as u32,
            image::imageops::FilterType::Triangle,
        );
    }
    // Keep the ancestor/file handles alive through the independent resource read.
    drop(guarded);
    Some(pixels)
}

#[cfg(windows)]
mod native {
    use std::{
        fs::File,
        os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    };
    use windows::{
        Win32::{
            Foundation::{GENERIC_READ, HANDLE},
            Graphics::Gdi::{
                BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS,
                DeleteDC, DeleteObject, GetDIBits, GetObjectW, HBITMAP, HDC, HGDIOBJ,
            },
            Storage::FileSystem::{
                BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_ATTRIBUTE_REPARSE_POINT,
                FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
                FILE_SHARE_READ, GetDriveTypeW, GetFileInformationByHandle, OPEN_EXISTING,
                QueryDosDeviceW,
            },
            UI::{
                Shell::ExtractIconExW,
                WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO},
            },
        },
        core::PCWSTR,
    };

    pub struct GuardedFile {
        pub file: File,
        _ancestors: Vec<OwnedHandle>,
    }
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }

    pub fn open_local(path: &str) -> Option<GuardedFile> {
        let drive = wide(&path[..2]);
        let mut mapping = [0_u16; 512];
        let length = unsafe { QueryDosDeviceW(PCWSTR(drive.as_ptr()), Some(&mut mapping)) };
        if length == 0 || length as usize > mapping.len() {
            return None;
        }
        let end = mapping.iter().position(|unit| *unit == 0)?;
        if !super::native_disk_mapping(&String::from_utf16(&mapping[..end]).ok()?) {
            return None;
        }
        let root = wide(&path[..3]);
        if unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) } != 3 {
            return None;
        } // DRIVE_FIXED
        let mut ancestors = Vec::new();
        let mut current = path[..3].to_string();
        let parts = path[3..].split('\\').collect::<Vec<_>>();
        for (index, part) in parts.iter().enumerate() {
            if index > 0 {
                current.push('\\');
            }
            current.push_str(part);
            let final_file = index + 1 == parts.len();
            let name = wide(&current);
            let handle = unsafe {
                CreateFileW(
                    PCWSTR(name.as_ptr()),
                    if final_file {
                        GENERIC_READ.0
                    } else {
                        FILE_READ_ATTRIBUTES.0
                    },
                    // Deny writes and renames while every checked ancestor is held.
                    FILE_SHARE_READ,
                    None,
                    OPEN_EXISTING,
                    FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
                    None,
                )
                .ok()?
            };
            let owned = unsafe { OwnedHandle::from_raw_handle(handle.0) };
            let mut info = BY_HANDLE_FILE_INFORMATION::default();
            unsafe {
                GetFileInformationByHandle(HANDLE(owned.as_raw_handle()), &mut info).ok()?;
            }
            if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
                return None;
            }
            if final_file {
                let file = File::from(owned);
                if !file.metadata().ok()?.is_file() {
                    return None;
                }
                return Some(GuardedFile {
                    file,
                    _ancestors: ancestors,
                });
            }
            ancestors.push(owned);
        }
        None
    }

    struct Icon(HICON);
    impl Drop for Icon {
        fn drop(&mut self) {
            let _ = unsafe { DestroyIcon(self.0) };
        }
    }
    struct Bitmap(HBITMAP);
    impl Drop for Bitmap {
        fn drop(&mut self) {
            let _ = unsafe { DeleteObject(HGDIOBJ(self.0.0)) };
        }
    }
    struct Dc(HDC);
    impl Drop for Dc {
        fn drop(&mut self) {
            let _ = unsafe { DeleteDC(self.0) };
        }
    }

    fn bitmap_pixels(
        dc: HDC,
        bitmap: HBITMAP,
        width: u32,
        height: u32,
    ) -> Option<image::RgbaImage> {
        let mut pixels = image::RgbaImage::new(width, height);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let copied = unsafe {
            GetDIBits(
                dc,
                bitmap,
                0,
                height,
                Some(pixels.as_mut_ptr().cast()),
                &mut info,
                DIB_RGB_COLORS,
            )
        };
        (copied == height as i32).then_some(pixels)
    }

    pub fn resource_pixels(path: &str, index: i32) -> Option<image::RgbaImage> {
        let path = wide(path);
        let mut icon = HICON::default();
        let extracted =
            unsafe { ExtractIconExW(PCWSTR(path.as_ptr()), index, Some(&mut icon), None, 1) };
        let icon = Icon(icon);
        if extracted != 1 || icon.0.0.is_null() {
            return None;
        }
        let mut info = ICONINFO::default();
        unsafe {
            GetIconInfo(icon.0, &mut info).ok()?;
        }
        let color = Bitmap(info.hbmColor);
        let mask = Bitmap(info.hbmMask);
        let mut dimensions = BITMAP::default();
        if unsafe {
            GetObjectW(
                HGDIOBJ(color.0.0),
                std::mem::size_of::<BITMAP>() as i32,
                Some((&raw mut dimensions).cast()),
            )
        } == 0
        {
            return None;
        }
        let width = u32::try_from(dimensions.bmWidth).ok()?;
        let height = u32::try_from(dimensions.bmHeight).ok()?;
        if width == 0
            || height == 0
            || width > super::MAX_DIMENSION
            || height > super::MAX_DIMENSION
        {
            return None;
        }
        let dc = Dc(unsafe { CreateCompatibleDC(None) });
        if dc.0.0.is_null() {
            return None;
        }
        let mut pixels = bitmap_pixels(dc.0, color.0, width, height)?;
        // Older resources store transparency in the AND mask, not an alpha channel.
        if pixels.pixels().all(|pixel| pixel[3] == 0) {
            let mask_pixels = bitmap_pixels(dc.0, mask.0, width, height)?;
            for (pixel, mask) in pixels.pixels_mut().zip(mask_pixels.pixels()) {
                pixel[3] = if mask[0] == 0 { 255 } else { 0 };
            }
        }
        Some(pixels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(id: usize) -> IconRequest {
        IconRequest {
            id: id.to_string(),
            evidence: IconEvidenceV1 {
                path: format!("C:\\Apps\\{id}.png"),
                index: 0,
            },
        }
    }

    #[test]
    fn unsafe_path_and_drive_alias_forms_are_rejected_before_file_reads() {
        for path in [
            "\\\\server\\share\\icon.png",
            "\\\\?\\C:\\icon.png",
            "https://host/icon.png",
            "C:icon.png",
            "C:\\app.exe:stream",
            "C:\\..\\icon.png",
            "C:\\app.\\icon.png",
            "C:\\%LOCALAPPDATA%\\icon.png",
            "C:\\icon.png\0",
            "C:\\folder\\\\icon.png",
            "C:\\NUL.ico",
            "C:\\com1.png",
            "C:\\folder\\LPT9.ico",
            "C:\\COM¹.png",
            "C:\\Apps\\COM³.png",
            "C:\\LPT².png",
            "C:\\NUL.tar.gz",
            "C:\\NUL .ico",
            "C:\\CONOUT$.ico",
        ] {
            assert_eq!(local_path(path), None, "{path:?}");
        }
        assert_eq!(
            local_path("C:/Apps/软件.png").as_deref(),
            Some("C:\\Apps\\软件.png")
        );
        assert_eq!(
            local_path("C:\\Apps\\COM10.png").as_deref(),
            Some("C:\\Apps\\COM10.png")
        );
        assert!(native_disk_mapping("\\Device\\HarddiskVolume12"));
        for mapping in [
            "\\??\\C:\\alias",
            "\\Device\\Mup\\server",
            "\\Device\\LanmanRedirector",
            "\\Device\\HarddiskVolume12\\nested",
            "\\Device\\HarddiskVolume",
        ] {
            assert!(!native_disk_mapping(mapping), "{mapping}");
        }
    }

    #[test]
    fn cache_is_bounded_and_remembers_failures_without_blocking_text() {
        let mut cache = SoftwareIcons::default();
        cache.synchronize(Some("inventory-a"));
        assert!(
            cache.image(&request(0)).is_none(),
            "text/avatar precedes any job"
        );
        assert_eq!(
            cache.next_batch((0..1000).map(request)).len(),
            BATCH_CAPACITY
        );
        for id in 0..CACHE_CAPACITY + 1 {
            assert!(cache.apply(
                cache.generation(),
                vec![IconResult {
                    request: request(id),
                    pixels: None
                }]
            ));
        }
        assert_eq!(cache.entries.len(), CACHE_CAPACITY);
        assert!(
            cache.next_batch(std::iter::once(request(1))).is_empty(),
            "failed reads are cached"
        );
        assert_eq!(
            cache.next_batch(std::iter::once(request(0))).len(),
            1,
            "oldest evicted"
        );
        let old = cache.generation();
        cache.synchronize(Some("inventory-b"));
        assert!(!cache.apply(
            old,
            vec![IconResult {
                request: request(1),
                pixels: Some(image::RgbaImage::new(1, 1))
            }]
        ));
        assert!(cache.entries.is_empty());
        assert!(cache.apply(
            cache.generation(),
            vec![IconResult {
                request: request(1),
                pixels: Some(image::RgbaImage::new(1, 1))
            }]
        ));
        assert!(cache.image(&request(1)).is_some());
        let mut changed = request(1);
        changed.evidence.index = -2;
        assert!(
            cache.image(&changed).is_none(),
            "same id with different evidence cannot reuse pixels"
        );
        cache.invalidate();
        assert!(cache.entries.is_empty(), "refresh retries failures");
    }

    #[cfg(windows)]
    #[test]
    fn missing_local_network_and_corrupt_files_use_the_avatar() {
        for path in [
            "C:\\devsweep-missing-icon-fixture.png",
            "\\\\invalid\\share\\icon.png",
        ] {
            assert!(
                load_icon(&IconEvidenceV1 {
                    path: path.to_string(),
                    index: 0
                })
                .is_none()
            );
        }
        let temp = std::env::temp_dir().join(format!(
            "devsweep-software-icon-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir(&temp).expect("fixture directory");
        let path = temp.join("invalid.png");
        std::fs::write(&path, b"not an image").expect("write fixture");
        assert!(
            load_icon(&IconEvidenceV1 {
                path: path.to_string_lossy().into_owned(),
                index: 0
            })
            .is_none()
        );
        image::RgbaImage::from_pixel(4, 4, image::Rgba([0x12, 0x34, 0x56, 255]))
            .save(&path)
            .expect("PNG fixture");
        let evidence = IconEvidenceV1 {
            path: path.to_string_lossy().into_owned(),
            index: 0,
        };
        let pixels = load_icon(&evidence).expect("local fixed-disk PNG fixture");
        assert_eq!(pixels.dimensions(), (4, 4));
        assert_eq!(
            pixels.get_pixel(0, 0).0,
            [0x56, 0x34, 0x12, 255],
            "GPUI BGRA order"
        );
        let alias = temp.join("via");
        std::os::windows::fs::symlink_dir(&temp, &alias).expect("directory reparse fixture");
        assert!(
            load_icon(&IconEvidenceV1 {
                path: alias.join("invalid.png").to_string_lossy().into_owned(),
                index: 0
            })
            .is_none(),
            "reject an intermediate reparse point before reading image bytes"
        );
        std::fs::remove_dir(&alias).expect("remove owned directory link only");
        let file_alias = temp.join("alias.png");
        std::os::windows::fs::symlink_file(&path, &file_alias).expect("file reparse fixture");
        assert!(
            load_icon(&IconEvidenceV1 {
                path: file_alias.to_string_lossy().into_owned(),
                index: 0
            })
            .is_none()
        );
        std::fs::remove_file(file_alias).expect("remove owned file link only");
        image::RgbaImage::new(MAX_DIMENSION + 1, 1)
            .save(&path)
            .expect("oversized PNG fixture");
        assert!(
            load_icon(&evidence).is_none(),
            "decoder rejects oversized dimensions"
        );
        std::fs::remove_file(path).expect("remove fixture");
        std::fs::remove_dir(temp).expect("remove fixture directory");
    }
}
