//! Existing desktop preference, presentation, scan-root, and protection documents.
//!
//! Reading does not create files and does not migrate V1 into V2. A write happens
//! only when a caller passes an explicit patch or replacement. Scan roots and the
//! protection list use separate paths. The directory lock is local to the supplied
//! folder. It is not the user-session mutex, so a test does not take the live
//! DevSweep preference lock and does not open the user store.

use std::{
    env, fs,
    fs::OpenOptions,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::Duration,
};

use serde::{Deserialize, Deserializer, Serialize, de};

const TEXT_SCALES: &[u8] = &[100, 110, 125];
const PLANET_FRAME_CAPS: &[u8] = &[15, 30];
const STATUS_INTERVALS: &[u8] = &[1, 2, 5, 10, 30, 60];
const STATUS_PROCESS_LIMITS: &[u8] = &[5, 15, 30, 50, 100];
const HUD_INTERVALS: &[u8] = &[2, 5, 10];
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Paths for one preference session. Tests pass a temporary directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreferenceFiles {
    pub settings_dir: PathBuf,
    pub protection_path: PathBuf,
}

impl PreferenceFiles {
    pub fn new(settings_dir: PathBuf, protection_path: PathBuf) -> Self {
        Self {
            settings_dir,
            protection_path,
        }
    }

    /// Build the fixed store paths from the process environment.
    ///
    /// This does not open either file. Startup is the only caller.
    pub fn from_env() -> Result<Self, StoreError> {
        let local = env::var_os("LOCALAPPDATA")
            .filter(|value| !value.is_empty())
            .ok_or(StoreError::Unavailable)?;
        let roaming = env::var_os("APPDATA")
            .filter(|value| !value.is_empty())
            .ok_or(StoreError::Unavailable)?;
        Ok(Self {
            settings_dir: PathBuf::from(local).join("DevSweep").join("settings"),
            protection_path: PathBuf::from(roaming)
                .join("devsweep")
                .join("protected-paths.json"),
        })
    }

    fn desktop_v2(&self) -> PathBuf {
        self.settings_dir.join("desktop-preferences-v2.json")
    }

    fn desktop_v1(&self) -> PathBuf {
        self.settings_dir.join("desktop-preferences-v1.json")
    }

    fn presentation(&self) -> PathBuf {
        self.settings_dir.join("presentation-v1.json")
    }

    fn scan_roots(&self) -> PathBuf {
        self.settings_dir.join("clean-scan-roots-v1.json")
    }
}

#[derive(Debug)]
pub enum StoreError {
    Unavailable,
    Blocked,
    LockUnavailable(PathBuf),
    Unsupported(PathBuf),
    InvalidValue,
    Io {
        stage: &'static str,
        path: PathBuf,
        source: io::Error,
    },
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("preference storage is unavailable"),
            Self::Blocked => {
                formatter.write_str("preference save is blocked until the stored bytes validate")
            }
            Self::LockUnavailable(path) => {
                write!(
                    formatter,
                    "preference lock is unavailable at {}",
                    path.display()
                )
            }
            Self::Unsupported(path) => write!(
                formatter,
                "unsupported preference document at {}",
                path.display()
            ),
            Self::InvalidValue => formatter.write_str("unsupported preference value"),
            Self::Io {
                stage,
                path,
                source,
            } => write!(
                formatter,
                "preference I/O failed at {stage} for {}: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn io_error(stage: &'static str, path: &Path, source: io::Error) -> StoreError {
    StoreError::Io {
        stage,
        path: path.to_path_buf(),
        source,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopTheme {
    #[default]
    Dark,
    Light,
    System,
    CatppuccinLatte,
    CatppuccinMocha,
    Codex,
    Claude,
}

impl DesktopTheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::System => "system",
            Self::CatppuccinLatte => "catppuccin_latte",
            Self::CatppuccinMocha => "catppuccin_mocha",
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::System,
            Self::System => Self::CatppuccinLatte,
            Self::CatppuccinLatte => Self::CatppuccinMocha,
            Self::CatppuccinMocha => Self::Codex,
            Self::Codex => Self::Claude,
            Self::Claude => Self::Dark,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DesktopFont {
    System {},
    Installed {
        #[serde(deserialize_with = "font_family")]
        family: String,
    },
}

impl Default for DesktopFont {
    fn default() -> Self {
        Self::System {}
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopMotion {
    #[default]
    System,
    Reduced,
}

impl DesktopMotion {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Reduced => "reduced",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopPreferences {
    #[serde(deserialize_with = "schema_version_v2")]
    pub schema_version: u8,
    pub theme: DesktopTheme,
    pub font: DesktopFont,
    #[serde(deserialize_with = "text_scale")]
    pub text_scale_percent: u8,
    pub motion: DesktopMotion,
    #[serde(deserialize_with = "planet_fps")]
    pub planet_fps: u8,
    #[serde(deserialize_with = "status_interval")]
    pub status_interval_seconds: u8,
    #[serde(deserialize_with = "status_process_limit")]
    pub status_process_limit: u8,
    #[serde(deserialize_with = "hud_interval")]
    pub hud_interval_seconds: u8,
}

impl Default for DesktopPreferences {
    fn default() -> Self {
        Self {
            schema_version: 2,
            theme: DesktopTheme::Dark,
            font: DesktopFont::System {},
            text_scale_percent: 100,
            motion: DesktopMotion::System,
            planet_fps: 30,
            status_interval_seconds: 2,
            status_process_limit: 15,
            hud_interval_seconds: 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "field", rename_all = "snake_case", deny_unknown_fields)]
pub enum DesktopPatch {
    Theme {
        value: DesktopTheme,
    },
    Font {
        value: DesktopFont,
    },
    TextScalePercent {
        #[serde(deserialize_with = "text_scale")]
        value: u8,
    },
    Motion {
        value: DesktopMotion,
    },
    PlanetFps {
        #[serde(deserialize_with = "planet_fps")]
        value: u8,
    },
    StatusIntervalSeconds {
        #[serde(deserialize_with = "status_interval")]
        value: u8,
    },
    StatusProcessLimit {
        #[serde(deserialize_with = "status_process_limit")]
        value: u8,
    },
    HudIntervalSeconds {
        #[serde(deserialize_with = "hud_interval")]
        value: u8,
    },
    ResetAppearance {},
    ResetPerformance {},
}

impl DesktopPreferences {
    pub fn patched(mut self, patch: DesktopPatch) -> Result<Self, StoreError> {
        match patch {
            DesktopPatch::Theme { value } => self.theme = value,
            DesktopPatch::Font { value } => self.font = value,
            DesktopPatch::TextScalePercent { value } => self.text_scale_percent = value,
            DesktopPatch::Motion { value } => self.motion = value,
            DesktopPatch::PlanetFps { value } => self.planet_fps = value,
            DesktopPatch::StatusIntervalSeconds { value } => self.status_interval_seconds = value,
            DesktopPatch::StatusProcessLimit { value } => self.status_process_limit = value,
            DesktopPatch::HudIntervalSeconds { value } => self.hud_interval_seconds = value,
            DesktopPatch::ResetAppearance {} => {
                let defaults = Self::default();
                self.theme = defaults.theme;
                self.font = defaults.font;
                self.text_scale_percent = defaults.text_scale_percent;
            }
            DesktopPatch::ResetPerformance {} => {
                let defaults = Self::default();
                self.motion = defaults.motion;
                self.planet_fps = defaults.planet_fps;
                self.status_interval_seconds = defaults.status_interval_seconds;
                self.status_process_limit = defaults.status_process_limit;
                self.hud_interval_seconds = defaults.hud_interval_seconds;
            }
        }
        validate_closed_numbers(&self)?;
        if let DesktopFont::Installed { family } = &self.font
            && !valid_font_family(family)
        {
            return Err(StoreError::InvalidValue);
        }
        Ok(self)
    }
}

fn validate_closed_numbers(preferences: &DesktopPreferences) -> Result<(), StoreError> {
    for (value, choices) in [
        (preferences.text_scale_percent, TEXT_SCALES),
        (preferences.planet_fps, PLANET_FRAME_CAPS),
        (preferences.status_interval_seconds, STATUS_INTERVALS),
        (preferences.status_process_limit, STATUS_PROCESS_LIMITS),
        (preferences.hud_interval_seconds, HUD_INTERVALS),
    ] {
        if !choices.contains(&value) {
            return Err(StoreError::InvalidValue);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageTag {
    #[serde(rename = "en")]
    En,
    #[serde(rename = "zh-CN")]
    ZhCn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedStores {
    pub desktop: DesktopPreferences,
    pub desktop_blocked: bool,
    pub language: Option<LanguageTag>,
    pub language_blocked: bool,
    pub scan_roots: Vec<String>,
    pub scan_blocked: bool,
    pub protections: Vec<String>,
    pub protection_blocked: bool,
}

/// Read every store. Missing files become defaults. Nothing is created.
pub fn load_stores(files: &PreferenceFiles) -> LoadedStores {
    let (desktop, desktop_blocked) = match read_desktop(files) {
        Ok(preferences) => (preferences, false),
        Err(_) => (DesktopPreferences::default(), true),
    };
    let (language, language_blocked) = match read_language(files) {
        Ok(language) => (language, false),
        Err(_) => (None, true),
    };
    let (scan_roots, scan_blocked) = match read_scan_roots(files) {
        Ok(roots) => (roots, false),
        Err(_) => (Vec::new(), true),
    };
    let (protections, protection_blocked) = match read_protections(files) {
        Ok(paths) => (paths, false),
        Err(_) => (Vec::new(), true),
    };
    LoadedStores {
        desktop,
        desktop_blocked,
        language,
        language_blocked,
        scan_roots,
        scan_blocked,
        protections,
        protection_blocked,
    }
}

pub fn save_desktop_patch(
    files: &PreferenceFiles,
    patch: DesktopPatch,
) -> Result<DesktopPreferences, StoreError> {
    save_desktop_patch_pausing(files, patch, Duration::ZERO)
}

pub fn save_desktop_patch_pausing(
    files: &PreferenceFiles,
    patch: DesktopPatch,
    pause: Duration,
) -> Result<DesktopPreferences, StoreError> {
    let path = files.desktop_v2();
    let lock_path = files.settings_dir.join("desktop-preferences.lock");
    with_lock(&lock_path, pause, || {
        let current = read_desktop(files)?;
        let next = current.patched(patch)?;
        let bytes = serde_json::to_vec(&next).map_err(|_| StoreError::InvalidValue)?;
        atomic_replace(&path, &bytes)?;
        Ok(next)
    })
}

pub fn save_language(files: &PreferenceFiles, language: LanguageTag) -> Result<(), StoreError> {
    let path = files.presentation();
    let lock_path = files.settings_dir.join("presentation-v1.lock");
    with_lock(&lock_path, Duration::ZERO, || {
        if path.exists() {
            read_language(files)?;
        }
        let document = PresentationDocument {
            schema_version: 1,
            language: Some(language),
        };
        let bytes = serde_json::to_vec(&document).map_err(|_| StoreError::InvalidValue)?;
        atomic_replace(&path, &bytes)
    })
}

pub fn save_scan_roots(
    files: &PreferenceFiles,
    roots: &[String],
) -> Result<Vec<String>, StoreError> {
    let path = files.scan_roots();
    let lock_path = files.settings_dir.join("clean-scan-roots.lock");
    with_lock(&lock_path, Duration::ZERO, || {
        if path.exists() {
            read_scan_roots(files)?;
        }
        let roots = validate_scan_roots(roots)?;
        let document = ScanRootsDocument {
            schema_version: 1,
            roots: roots.clone(),
        };
        let bytes = serde_json::to_vec(&document).map_err(|_| StoreError::InvalidValue)?;
        atomic_replace(&path, &bytes)?;
        Ok(roots)
    })
}

pub fn save_protections(
    files: &PreferenceFiles,
    paths: &[String],
) -> Result<Vec<String>, StoreError> {
    let path = files.protection_path.clone();
    let lock_path = path.with_file_name("protected-paths.lock");
    with_lock(&lock_path, Duration::ZERO, || {
        if path.exists() {
            read_protections(files)?;
        }
        let paths = validate_protection_paths(paths)?;
        let document = ProtectionDocument {
            version: 1,
            paths: paths.clone(),
        };
        let mut bytes =
            serde_json::to_vec_pretty(&document).map_err(|_| StoreError::InvalidValue)?;
        bytes.push(b'\n');
        atomic_replace(&path, &bytes)?;
        Ok(paths)
    })
}

fn read_desktop(files: &PreferenceFiles) -> Result<DesktopPreferences, StoreError> {
    let path = files.desktop_v2();
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| StoreError::Unsupported(path)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            read_legacy_v1(&files.desktop_v1())
        }
        Err(error) => Err(io_error("read_document", &path, error)),
    }
}

fn read_legacy_v1(path: &Path) -> Result<DesktopPreferences, StoreError> {
    match fs::read(path) {
        Ok(bytes) => {
            let legacy: DesktopPreferencesV1 = serde_json::from_slice(&bytes)
                .map_err(|_| StoreError::Unsupported(path.to_path_buf()))?;
            Ok(legacy.into())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(DesktopPreferences::default()),
        Err(error) => Err(io_error("read_legacy_document", path, error)),
    }
}

fn read_language(files: &PreferenceFiles) -> Result<Option<LanguageTag>, StoreError> {
    let path = files.presentation();
    match fs::read(&path) {
        Ok(bytes) => {
            let document: PresentationDocument = serde_json::from_slice(&bytes)
                .map_err(|_| StoreError::Unsupported(path.clone()))?;
            if document.schema_version != 1 {
                return Err(StoreError::Unsupported(path));
            }
            Ok(document.language)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error("read_presentation", &path, error)),
    }
}

fn read_scan_roots(files: &PreferenceFiles) -> Result<Vec<String>, StoreError> {
    let path = files.scan_roots();
    match fs::read(&path) {
        Ok(bytes) => {
            let document: ScanRootsDocument = serde_json::from_slice(&bytes)
                .map_err(|_| StoreError::Unsupported(path.clone()))?;
            if document.schema_version != 1 {
                return Err(StoreError::Unsupported(path));
            }
            Ok(document.roots)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(io_error("read_scan_roots", &path, error)),
    }
}

fn read_protections(files: &PreferenceFiles) -> Result<Vec<String>, StoreError> {
    let path = &files.protection_path;
    match fs::read(path) {
        Ok(bytes) => {
            let document: ProtectionDocument = serde_json::from_slice(&bytes)
                .map_err(|_| StoreError::Unsupported(path.clone()))?;
            if document.version != 1 {
                return Err(StoreError::Unsupported(path.clone()));
            }
            Ok(document.paths)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(io_error("read_protections", path, error)),
    }
}

fn validate_scan_roots(roots: &[String]) -> Result<Vec<String>, StoreError> {
    let mut accepted = Vec::new();
    for root in roots {
        let path = PathBuf::from(root);
        if !path.is_absolute() || !path.is_dir() || is_reparse(&path) {
            return Err(StoreError::InvalidValue);
        }
        if !accepted.iter().any(|existing: &String| existing == root) {
            accepted.push(root.clone());
        }
    }
    Ok(accepted)
}

fn validate_protection_paths(paths: &[String]) -> Result<Vec<String>, StoreError> {
    let mut accepted = Vec::new();
    for path in paths {
        if path.is_empty() || Path::new(path).is_relative() {
            return Err(StoreError::InvalidValue);
        }
        if !accepted.iter().any(|existing: &String| existing == path) {
            accepted.push(path.clone());
        }
    }
    Ok(accepted)
}

fn is_reparse(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        const INVALID_FILE_ATTRIBUTES: u32 = 0xFFFF_FFFF;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let attributes = unsafe { GetFileAttributesW(wide.as_ptr()) };
        attributes != INVALID_FILE_ATTRIBUTES && attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        false
    }
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetFileAttributesW(path: *const u16) -> u32;
    fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
}

fn with_lock<T>(
    lock_path: &Path,
    pause: Duration,
    body: impl FnOnce() -> Result<T, StoreError>,
) -> Result<T, StoreError> {
    let Some(directory) = lock_path.parent() else {
        return Err(io_error(
            "resolve_lock_parent",
            lock_path,
            io::Error::new(io::ErrorKind::InvalidInput, "lock path has no parent"),
        ));
    };
    fs::create_dir_all(directory)
        .map_err(|error| io_error("create_settings_directory", directory, error))?;
    let _guard = acquire_lock(lock_path)?;
    body_after_pause(pause, body)
}

fn body_after_pause<T>(
    pause: Duration,
    body: impl FnOnce() -> Result<T, StoreError>,
) -> Result<T, StoreError> {
    if !pause.is_zero() {
        thread::sleep(pause);
    }
    body()
}

fn acquire_lock(path: &Path) -> Result<fs::File, StoreError> {
    for attempt in 0..500 {
        match open_exclusive(path) {
            Ok(file) => return Ok(file),
            Err(error) if is_lock_contention(&error) => {
                if attempt + 1 == 500 {
                    return Err(StoreError::LockUnavailable(path.to_path_buf()));
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(io_error("acquire_lock", path, error)),
        }
    }
    Err(StoreError::LockUnavailable(path.to_path_buf()))
}

fn open_exclusive(path: &Path) -> io::Result<fs::File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
    }
    options.open(path)
}

fn is_lock_contention(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(32 | 33))
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let directory = path.parent().ok_or_else(|| {
        io_error(
            "resolve_settings_parent",
            path,
            io::Error::new(io::ErrorKind::InvalidInput, "preference path has no parent"),
        )
    })?;
    fs::create_dir_all(directory)
        .map_err(|error| io_error("create_settings_directory", directory, error))?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary_path = directory.join(format!(
        ".{}.{}.{}.tmp",
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("preference"),
        std::process::id(),
        sequence
    ));
    let result = (|| {
        let mut temporary = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
            .map_err(|error| io_error("create_temporary_document", &temporary_path, error))?;
        temporary
            .write_all(bytes)
            .map_err(|error| io_error("write_temporary_document", &temporary_path, error))?;
        temporary
            .sync_all()
            .map_err(|error| io_error("sync_temporary_document", &temporary_path, error))?;
        drop(temporary);
        replace_file(&temporary_path, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> Result<(), StoreError> {
    use std::os::windows::ffi::OsStrExt;
    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;
    let source_wide: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination_wide: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let replaced = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        Err(io_error(
            "replace_document",
            destination,
            io::Error::last_os_error(),
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> Result<(), StoreError> {
    fs::rename(source, destination)
        .map_err(|error| io_error("replace_document", destination, error))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LegacyTheme {
    Dark,
    Light,
    System,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LegacyFont {
    System,
    SegoeUi,
    MicrosoftYaheiUi,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DesktopPreferencesV1 {
    #[serde(deserialize_with = "schema_version_v1")]
    #[allow(dead_code)]
    schema_version: u8,
    theme: LegacyTheme,
    font_family: LegacyFont,
    #[serde(deserialize_with = "text_scale")]
    text_scale_percent: u8,
    motion: DesktopMotion,
    #[serde(deserialize_with = "planet_fps")]
    planet_fps: u8,
    #[serde(deserialize_with = "status_interval")]
    status_interval_seconds: u8,
    #[serde(deserialize_with = "status_process_limit")]
    status_process_limit: u8,
    #[serde(deserialize_with = "hud_interval")]
    hud_interval_seconds: u8,
}

impl From<DesktopPreferencesV1> for DesktopPreferences {
    fn from(value: DesktopPreferencesV1) -> Self {
        Self {
            schema_version: 2,
            theme: match value.theme {
                LegacyTheme::Dark => DesktopTheme::Dark,
                LegacyTheme::Light => DesktopTheme::Light,
                LegacyTheme::System => DesktopTheme::System,
            },
            font: match value.font_family {
                LegacyFont::System => DesktopFont::System {},
                LegacyFont::SegoeUi => DesktopFont::Installed {
                    family: "Segoe UI".to_string(),
                },
                LegacyFont::MicrosoftYaheiUi => DesktopFont::Installed {
                    family: "Microsoft YaHei UI".to_string(),
                },
            },
            text_scale_percent: value.text_scale_percent,
            motion: value.motion,
            planet_fps: value.planet_fps,
            status_interval_seconds: value.status_interval_seconds,
            status_process_limit: value.status_process_limit,
            hud_interval_seconds: value.hud_interval_seconds,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PresentationDocument {
    schema_version: u8,
    language: Option<LanguageTag>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScanRootsDocument {
    schema_version: u8,
    roots: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProtectionDocument {
    version: u32,
    paths: Vec<String>,
}

fn valid_font_family(family: &str) -> bool {
    !family.is_empty()
        && family.trim() == family
        && family.chars().count() <= 256
        && !family.chars().any(char::is_control)
}

fn font_family<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let family = String::deserialize(deserializer)?;
    if valid_font_family(&family) {
        Ok(family)
    } else {
        Err(de::Error::custom("invalid installed font family"))
    }
}

fn choice<'de, D: Deserializer<'de>>(
    deserializer: D,
    field: &str,
    choices: &[u8],
) -> Result<u8, D::Error> {
    let value = u8::deserialize(deserializer)?;
    if choices.contains(&value) {
        Ok(value)
    } else {
        Err(de::Error::custom(format!("unsupported {field}: {value}")))
    }
}

fn schema_version_v2<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    choice(deserializer, "schema_version", &[2])
}

fn schema_version_v1<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    choice(deserializer, "schema_version", &[1])
}

fn text_scale<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    choice(deserializer, "text_scale_percent", TEXT_SCALES)
}

fn planet_fps<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    choice(deserializer, "planet_fps", PLANET_FRAME_CAPS)
}

fn status_interval<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    choice(deserializer, "status_interval_seconds", STATUS_INTERVALS)
}

fn status_process_limit<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    choice(deserializer, "status_process_limit", STATUS_PROCESS_LIMITS)
}

fn hud_interval<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    choice(deserializer, "hud_interval_seconds", HUD_INTERVALS)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};

    use super::*;

    const DEFAULT_BYTES: &[u8] = br#"{"schema_version":2,"theme":"dark","font":{"kind":"system"},"text_scale_percent":100,"motion":"system","planet_fps":30,"status_interval_seconds":2,"status_process_limit":15,"hud_interval_seconds":2}"#;
    const LEGACY_BYTES: &[u8] = br#"{"schema_version":1,"theme":"light","font_family":"system","text_scale_percent":125,"motion":"reduced","planet_fps":15,"status_interval_seconds":60,"status_process_limit":100,"hud_interval_seconds":10}"#;

    fn isolated(name: &str) -> (PathBuf, PreferenceFiles) {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = env::temp_dir().join(format!(
            "devsweep-gpui-c10-{}-{}-{name}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(
            root.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("devsweep-gpui-c10-")),
            "refusing a preference fixture outside the temp prefix"
        );
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("settings")).unwrap();
        let files = PreferenceFiles::new(
            root.join("settings"),
            root.join("protect").join("protected-paths.json"),
        );
        (root, files)
    }

    fn cleanup(root: &Path) {
        assert!(
            root.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("devsweep-gpui-c10-"))
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn default_document_matches_the_closed_v2_bytes() {
        let bytes = serde_json::to_vec(&DesktopPreferences::default()).unwrap();
        assert_eq!(bytes, DEFAULT_BYTES);
        let loaded: DesktopPreferences = serde_json::from_slice(DEFAULT_BYTES).unwrap();
        assert_eq!(loaded, DesktopPreferences::default());
    }

    #[test]
    fn c10_ac2_launch_does_not_migrate_and_downgrade_keeps_v1() {
        let (root, files) = isolated("downgrade");
        let legacy = files.desktop_v1();
        let presentation = files.presentation();
        fs::write(&legacy, LEGACY_BYTES).unwrap();
        fs::write(&presentation, b"preserved language bytes").unwrap();
        let loaded = load_stores(&files);
        assert!(!loaded.desktop_blocked);
        assert_eq!(loaded.desktop.theme, DesktopTheme::Light);
        assert_eq!(loaded.desktop.text_scale_percent, 125);
        assert_eq!(loaded.desktop.motion, DesktopMotion::Reduced);
        assert_eq!(loaded.desktop.planet_fps, 15);
        assert_eq!(loaded.desktop.status_interval_seconds, 60);
        assert_eq!(loaded.desktop.status_process_limit, 100);
        assert_eq!(loaded.desktop.hud_interval_seconds, 10);
        assert!(matches!(loaded.desktop.font, DesktopFont::System {}));
        assert!(!files.desktop_v2().exists());
        assert_eq!(fs::read(&legacy).unwrap(), LEGACY_BYTES);
        assert_eq!(
            fs::read(&presentation).unwrap(),
            b"preserved language bytes"
        );
        assert!(!files.protection_path.exists());
        assert!(!files.scan_roots().exists());

        let saved = save_desktop_patch(
            &files,
            DesktopPatch::Theme {
                value: DesktopTheme::Dark,
            },
        )
        .unwrap();
        assert_eq!(saved.theme, DesktopTheme::Dark);
        assert_eq!(saved.text_scale_percent, 125);
        assert_eq!(saved.status_interval_seconds, 60);
        assert_eq!(fs::read(&legacy).unwrap(), LEGACY_BYTES);
        assert_eq!(
            fs::read(&presentation).unwrap(),
            b"preserved language bytes"
        );
        let v1_reader: DesktopPreferencesV1 =
            serde_json::from_slice(&fs::read(&legacy).unwrap()).unwrap();
        assert!(matches!(v1_reader.theme, LegacyTheme::Light));

        fs::write(&legacy, b"older app changed V1").unwrap();
        let reread = load_stores(&files);
        assert_eq!(reread.desktop, saved);
        assert!(!fs::read(files.desktop_v2()).unwrap().is_empty());
        cleanup(&root);
    }

    #[test]
    fn legacy_font_ids_convert_in_memory_without_a_write() {
        for (preset, family) in [
            ("system", None),
            ("segoe_ui", Some("Segoe UI")),
            ("microsoft_yahei_ui", Some("Microsoft YaHei UI")),
        ] {
            let (root, files) = isolated("legacy-font");
            let mut document: serde_json::Value = serde_json::from_slice(LEGACY_BYTES).unwrap();
            document["font_family"] = preset.into();
            let bytes = serde_json::to_vec(&document).unwrap();
            fs::write(files.desktop_v1(), &bytes).unwrap();
            let loaded = load_stores(&files);
            match family {
                None => assert_eq!(loaded.desktop.font, DesktopFont::System {}),
                Some(family) => assert_eq!(
                    loaded.desktop.font,
                    DesktopFont::Installed {
                        family: family.to_string(),
                    }
                ),
            }
            assert!(!files.desktop_v2().exists());
            assert_eq!(fs::read(files.desktop_v1()).unwrap(), bytes);
            cleanup(&root);
        }
    }

    #[test]
    fn invalid_v2_does_not_fall_back_or_get_overwritten() {
        let (root, files) = isolated("invalid-v2");
        let future = br#"{"schema_version":99,"theme":"dark","font":{"kind":"system"},"text_scale_percent":100,"motion":"system","planet_fps":30,"status_interval_seconds":2,"status_process_limit":15,"hud_interval_seconds":2}"#;
        for invalid in [
            future.as_slice(),
            &b"{"[..],
            b"\xff",
            br#"{"schema_version":2,"theme":"dark","unknown":true}"#,
        ] {
            fs::write(files.desktop_v2(), invalid).unwrap();
            fs::write(files.desktop_v1(), LEGACY_BYTES).unwrap();
            let loaded = load_stores(&files);
            assert!(loaded.desktop_blocked);
            assert_eq!(loaded.desktop, DesktopPreferences::default());
            assert!(save_desktop_patch(&files, DesktopPatch::ResetAppearance {}).is_err());
            assert_eq!(fs::read(files.desktop_v2()).unwrap(), invalid);
            assert_eq!(fs::read(files.desktop_v1()).unwrap(), LEGACY_BYTES);
        }
        cleanup(&root);
    }

    #[test]
    fn c10_ac2_concurrent_patches_keep_both_fields_and_leave_other_stores() {
        let (root, files) = isolated("concurrent");
        let scan = br#"{"schema_version":1,"roots":[]}"#;
        let protection = b"{\n  \"version\": 1,\n  \"paths\": []\n}\n";
        fs::write(files.scan_roots(), scan).unwrap();
        fs::create_dir_all(files.protection_path.parent().unwrap()).unwrap();
        fs::write(&files.protection_path, protection).unwrap();
        fs::write(files.desktop_v1(), LEGACY_BYTES).unwrap();
        let shared = Arc::new(files.clone());
        let barrier = Arc::new(Barrier::new(2));
        let left_barrier = Arc::clone(&barrier);
        let right_barrier = Arc::clone(&barrier);
        let left_files = Arc::clone(&shared);
        let right_files = Arc::clone(&shared);
        let left = thread::spawn(move || {
            left_barrier.wait();
            save_desktop_patch_pausing(
                &left_files,
                DesktopPatch::Theme {
                    value: DesktopTheme::Light,
                },
                Duration::from_millis(80),
            )
        });
        let right = thread::spawn(move || {
            right_barrier.wait();
            save_desktop_patch_pausing(
                &right_files,
                DesktopPatch::StatusIntervalSeconds { value: 10 },
                Duration::from_millis(80),
            )
        });
        left.join().unwrap().unwrap();
        right.join().unwrap().unwrap();
        let loaded = load_stores(&shared);
        assert_eq!(loaded.desktop.theme, DesktopTheme::Light);
        assert_eq!(loaded.desktop.status_interval_seconds, 10);
        assert_eq!(loaded.desktop.text_scale_percent, 125);
        assert_eq!(fs::read(shared.desktop_v1()).unwrap(), LEGACY_BYTES);
        assert_eq!(fs::read(shared.scan_roots()).unwrap(), scan);
        assert_eq!(fs::read(&shared.protection_path).unwrap(), protection);
        cleanup(&root);
    }

    #[test]
    fn scan_roots_and_protections_stay_separate_until_each_explicit_save() {
        let (root, files) = isolated("separate");
        let project = root.join("project");
        fs::create_dir_all(&project).unwrap();
        let loaded = load_stores(&files);
        assert!(loaded.scan_roots.is_empty());
        assert!(loaded.protections.is_empty());
        assert!(!files.scan_roots().exists());
        assert!(!files.protection_path.exists());
        assert!(!files.desktop_v2().exists());

        let roots = save_scan_roots(&files, &[project.display().to_string()]).unwrap();
        assert_eq!(roots, vec![project.display().to_string()]);
        assert!(!files.protection_path.exists());
        assert!(!files.desktop_v2().exists());

        let protected = root.join("protect-me");
        fs::create_dir_all(&protected).unwrap();
        let paths = save_protections(&files, &[protected.display().to_string()]).unwrap();
        assert_eq!(paths, vec![protected.display().to_string()]);
        let scan_text = String::from_utf8(fs::read(files.scan_roots()).unwrap()).unwrap();
        assert!(!scan_text.contains("protect-me"));
        let scan_bytes = fs::read(files.scan_roots()).unwrap();
        let protection_bytes = fs::read(&files.protection_path).unwrap();
        save_desktop_patch(
            &files,
            DesktopPatch::Motion {
                value: DesktopMotion::Reduced,
            },
        )
        .unwrap();
        assert_eq!(fs::read(files.scan_roots()).unwrap(), scan_bytes);
        assert_eq!(fs::read(&files.protection_path).unwrap(), protection_bytes);

        let missing = root.join("missing-root");
        let original = fs::read(files.scan_roots()).unwrap();
        assert!(save_scan_roots(&files, &[missing.display().to_string()]).is_err());
        assert_eq!(fs::read(files.scan_roots()).unwrap(), original);
        let future = br#"{"schema_version":2,"roots":[]}"#;
        fs::write(files.scan_roots(), future).unwrap();
        assert!(save_scan_roots(&files, &[project.display().to_string()]).is_err());
        assert_eq!(fs::read(files.scan_roots()).unwrap(), future);
        let unknown = br#"{"version":2,"paths":[]}"#;
        fs::write(&files.protection_path, unknown).unwrap();
        assert!(save_protections(&files, &[protected.display().to_string()]).is_err());
        assert_eq!(fs::read(&files.protection_path).unwrap(), unknown);
        cleanup(&root);
    }

    #[test]
    fn explicit_language_save_does_not_touch_desktop_bytes() {
        let (root, files) = isolated("language");
        fs::write(files.desktop_v2(), DEFAULT_BYTES).unwrap();
        assert!(load_stores(&files).language.is_none());
        assert!(!files.presentation().exists());
        save_language(&files, LanguageTag::ZhCn).unwrap();
        assert_eq!(fs::read(files.desktop_v2()).unwrap(), DEFAULT_BYTES);
        let saved = fs::read(files.presentation()).unwrap();
        let document: PresentationDocument = serde_json::from_slice(&saved).unwrap();
        assert_eq!(document.language, Some(LanguageTag::ZhCn));
        assert_eq!(document.schema_version, 1);
        cleanup(&root);
    }

    #[test]
    fn reset_groups_do_not_cross_fields() {
        let (root, files) = isolated("reset");
        fs::write(files.desktop_v1(), LEGACY_BYTES).unwrap();
        let themed = save_desktop_patch(
            &files,
            DesktopPatch::Theme {
                value: DesktopTheme::Codex,
            },
        )
        .unwrap();
        assert_eq!(themed.theme, DesktopTheme::Codex);
        assert_eq!(themed.status_interval_seconds, 60);
        let appearance = save_desktop_patch(&files, DesktopPatch::ResetAppearance {}).unwrap();
        assert_eq!(appearance.theme, DesktopTheme::Dark);
        assert_eq!(appearance.status_interval_seconds, 60);
        let performance = save_desktop_patch(&files, DesktopPatch::ResetPerformance {}).unwrap();
        assert_eq!(performance.theme, DesktopTheme::Dark);
        assert_eq!(performance.status_interval_seconds, 2);
        assert_eq!(performance.motion, DesktopMotion::System);
        assert_eq!(fs::read(files.desktop_v1()).unwrap(), LEGACY_BYTES);
        cleanup(&root);
    }
}
