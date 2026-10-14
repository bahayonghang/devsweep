//! Closed, read-only Windows cache discovery and live eligibility checks.
//! Default application layouts only; a matching directory name is not authority.
use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

use crate::{
    filesystem::{
        PathSafety, SystemPathReparseProbe, inspect_entry_no_follow, path_is_within, paths_equal,
    },
    model::ScanCoverageStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowsCacheFamily {
    Chrome,
    Edge,
    Firefox,
    VsCode,
    Cursor,
    UserTemp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnerState {
    Idle,
    Busy,
    Unknown,
}

#[derive(Debug)]
pub(crate) struct WindowsCacheUnavailable {
    pub path: PathBuf,
    pub status: ScanCoverageStatus,
    pub detail: String,
}

#[derive(Debug, Default)]
pub(crate) struct WindowsCacheDiscovery {
    /// Recognized exact children, including unavailable observations. Never roots.
    pub candidates: Vec<PathBuf>,
    /// Path-specific refusals, or root/provider failures without a candidate.
    pub unavailable: Vec<WindowsCacheUnavailable>,
}

#[derive(Debug)]
struct Failure(ScanCoverageStatus, &'static str);
type Check<T> = Result<T, Failure>;
const CHILD_LIMIT: usize = 4096;
const PROFILE_LIMIT: usize = 128;
const METADATA_LIMIT: u64 = 1024 * 1024;
const MIN_TEMP_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);

struct Budget {
    entries: usize,
    started: Instant,
}
impl Budget {
    fn new() -> Self {
        Self {
            entries: 0,
            started: Instant::now(),
        }
    }
    fn spend(&mut self) -> Check<()> {
        self.entries += 1;
        if self.entries > 50_000 || self.started.elapsed() > Duration::from_secs(2) {
            return Err(Failure(
                ScanCoverageStatus::BudgetLimited,
                "cache verification budget exhausted",
            ));
        }
        Ok(())
    }
}

struct Roots {
    local: Option<PathBuf>,
    roaming: Option<PathBuf>,
    program_files: Vec<PathBuf>,
    temp_override: Option<PathBuf>,
    editor_override: bool,
}
impl Roots {
    fn current() -> Self {
        Self {
            local: env::var_os("LOCALAPPDATA").map(PathBuf::from),
            roaming: env::var_os("APPDATA").map(PathBuf::from),
            program_files: ["ProgramFiles", "ProgramFiles(x86)"]
                .iter()
                .filter_map(env::var_os)
                .map(PathBuf::from)
                .collect(),
            temp_override: env::var_os("TEMP")
                .or_else(|| env::var_os("TMP"))
                .map(PathBuf::from),
            editor_override: ["VSCODE_APPDATA", "VSCODE_PORTABLE", "VSCODE_DEV"]
                .iter()
                .any(|name| env::var_os(name).is_some()),
        }
    }
    fn root(&self, family: WindowsCacheFamily) -> Check<PathBuf> {
        use WindowsCacheFamily::*;
        let (base, relative) = match family {
            Chrome => (&self.local, "Google/Chrome/User Data"),
            Edge => (&self.local, "Microsoft/Edge/User Data"),
            Firefox => (&self.local, "Mozilla/Firefox"),
            VsCode => (&self.roaming, "Code"),
            Cursor => (&self.roaming, "Cursor"),
            UserTemp => (&self.local, "Temp"),
        };
        let base = base
            .as_ref()
            .filter(|path| path.is_absolute())
            .ok_or(Failure(
                ScanCoverageStatus::OwnershipUnknown,
                "current-user application-data root unavailable",
            ))?;
        let path = base.join(relative);
        if family == UserTemp
            && self
                .temp_override
                .as_ref()
                .is_some_and(|temp| !paths_equal(temp, &path))
        {
            return Err(Failure(
                ScanCoverageStatus::Unsupported,
                "custom temporary root is not supported; default current-user Temp only",
            ));
        }
        Ok(path)
    }
}

fn owner_names(family: WindowsCacheFamily) -> &'static [&'static str] {
    use WindowsCacheFamily::*;
    match family {
        Chrome => &["chrome.exe"],
        Edge => &["msedge.exe"],
        Firefox => &["firefox.exe"],
        VsCode => &["Code.exe"],
        Cursor => &["Cursor.exe"],
        UserTemp => &[],
    }
}

pub(crate) fn discover_windows_caches(family: WindowsCacheFamily) -> WindowsCacheDiscovery {
    if !cfg!(windows) {
        return WindowsCacheDiscovery {
            candidates: vec![],
            unavailable: vec![WindowsCacheUnavailable {
                path: PathBuf::new(),
                status: ScanCoverageStatus::Unsupported,
                detail: "Windows current-user caches are unsupported on this platform".into(),
            }],
        };
    }
    discover(
        &Roots::current(),
        family,
        owner_state(owner_names(family)),
        None,
    )
}

/// The registry and live execution guard both call this; scan facts are not authority.
pub(crate) fn validate_windows_cache(
    family: WindowsCacheFamily,
    path: &Path,
) -> anyhow::Result<()> {
    if !cfg!(windows) {
        anyhow::bail!("Windows cache cleanup is unsupported on this platform")
    }
    let outcome = discover(
        &Roots::current(),
        family,
        owner_state(owner_names(family)),
        Some(path),
    );
    if !outcome
        .candidates
        .iter()
        .any(|candidate| paths_equal(candidate, path))
    {
        anyhow::bail!("path is not a recognized current-user cache child; rescan required")
    }
    if let Some(issue) = outcome
        .unavailable
        .iter()
        .find(|issue| issue.path.as_os_str().is_empty() || path_is_within(&issue.path, path))
    {
        anyhow::bail!("{}", issue.detail)
    }
    Ok(())
}

/// Reconstruct only the closed footprint. Dynamic activity remains a preview/live
/// refusal so one running application cannot invalidate unrelated plan targets.
pub(crate) fn validate_windows_cache_location(
    family: WindowsCacheFamily,
    path: &Path,
) -> anyhow::Result<()> {
    if !cfg!(windows) {
        anyhow::bail!("Windows cache cleanup is unsupported on this platform")
    }
    let roots = Roots::current();
    let root = roots
        .root(family)
        .map_err(|error| anyhow::anyhow!(error.1))?;
    let mut evidence = WindowsCacheDiscovery::default();
    let paths = resolve(&roots, family, &root, &mut Budget::new(), &mut evidence)
        .map_err(|error| anyhow::anyhow!(error.1))?;
    if !paths.iter().any(|candidate| paths_equal(candidate, path)) {
        anyhow::bail!("path is not a recognized current-user cache child; rescan required")
    }
    if let Some(issue) = evidence
        .unavailable
        .iter()
        .find(|issue| path_is_within(&issue.path, path))
    {
        anyhow::bail!("{}", issue.detail)
    }
    cache_location_entry(path, family, &SystemPathReparseProbe)
        .map_err(|error| anyhow::anyhow!(error.1))
}

fn discover(
    roots: &Roots,
    family: WindowsCacheFamily,
    owner: OwnerState,
    only: Option<&Path>,
) -> WindowsCacheDiscovery {
    let mut result = WindowsCacheDiscovery::default();
    let mut budget = Budget::new();
    let root = match roots.root(family) {
        Ok(path) => path,
        Err(error) => {
            issue(&mut result, PathBuf::new(), error);
            return result;
        }
    };
    let resolved = resolve(roots, family, &root, &mut budget, &mut result);
    let paths = match resolved {
        Ok(paths) => paths,
        Err(error) => {
            issue(&mut result, root, error);
            return result;
        }
    };
    for path in paths {
        if only.is_some_and(|selected| !paths_equal(selected, &path)) {
            continue;
        }
        // A root which cannot be reconstructed must remain coverage-only.
        // Otherwise one unsafe observation would invalidate healthy plan rows.
        if let Err(error) = cache_location_entry(&path, family, &SystemPathReparseProbe) {
            issue(&mut result, path, error);
            continue;
        }
        result.candidates.push(path.clone());
        let state = if family == WindowsCacheFamily::UserTemp {
            Ok(())
        } else {
            match owner {
                OwnerState::Idle => Ok(()),
                OwnerState::Busy => Err(Failure(
                    ScanCoverageStatus::Busy,
                    "application is running; close it and rescan",
                )),
                OwnerState::Unknown => Err(Failure(
                    ScanCoverageStatus::OwnershipUnknown,
                    "application activity could not be verified",
                )),
            }
        };
        let checked = state.and_then(|()| {
            verify_tree(
                &path,
                family == WindowsCacheFamily::UserTemp,
                SystemTime::now(),
                &mut budget,
            )
        });
        if let Err(error) = checked {
            issue(&mut result, path, error);
        }
    }
    result
}

fn issue(result: &mut WindowsCacheDiscovery, path: PathBuf, error: Failure) {
    result.unavailable.push(WindowsCacheUnavailable {
        path,
        status: error.0,
        detail: error.1.into(),
    });
}

fn safe_entry(path: &Path) -> Check<crate::filesystem::InspectedEntry> {
    safe_entry_with_probe(path, &SystemPathReparseProbe)
}

fn cache_location_entry(
    path: &Path,
    family: WindowsCacheFamily,
    probe: &dyn crate::filesystem::PathReparseProbe,
) -> Check<()> {
    let entry = safe_entry_with_probe(path, probe)?;
    if family != WindowsCacheFamily::UserTemp && !entry.is_dir {
        return Err(Failure(
            ScanCoverageStatus::OwnershipUnknown,
            "application cache child is not a directory",
        ));
    }
    Ok(())
}

fn safe_entry_with_probe(
    path: &Path,
    probe: &dyn crate::filesystem::PathReparseProbe,
) -> Check<crate::filesystem::InspectedEntry> {
    // The shared no-follow probe also rejects cloud/offline reparse tags.
    let entry = inspect_entry_no_follow(path, probe).map_err(|error| {
        Failure(
            if error.kind() == std::io::ErrorKind::NotFound {
                ScanCoverageStatus::Absent
            } else {
                ScanCoverageStatus::Failed
            },
            "cache path is absent or unreadable",
        )
    })?;
    if !matches!(entry.safety, PathSafety::Safe) {
        return Err(Failure(
            ScanCoverageStatus::Protected,
            "cache path contains a link, cloud placeholder or unverifiable reparse point",
        ));
    }
    if !entry.is_dir && !entry.is_file {
        return Err(Failure(
            ScanCoverageStatus::OwnershipUnknown,
            "cache entry is not a regular file or directory",
        ));
    }
    Ok(entry)
}

fn safe_ancestors(path: &Path) -> Check<()> {
    if !path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err(Failure(
            ScanCoverageStatus::OwnershipUnknown,
            "cache root must be an absolute normalized path",
        ));
    }
    for ancestor in path.ancestors().collect::<Vec<_>>().into_iter().rev() {
        safe_entry(ancestor)?;
    }
    Ok(())
}

fn directory(path: &Path) -> Check<()> {
    safe_ancestors(path)?;
    if !safe_entry(path)?.is_dir {
        return Err(Failure(
            ScanCoverageStatus::OwnershipUnknown,
            "expected application directory is not a directory",
        ));
    }
    Ok(())
}

fn read_metadata(path: &Path) -> Check<Vec<u8>> {
    safe_ancestors(path)?;
    let entry = safe_entry(path)?;
    if !entry.is_file || entry.len > METADATA_LIMIT {
        return Err(Failure(
            ScanCoverageStatus::OwnershipUnknown,
            "application metadata is not a bounded regular file",
        ));
    }
    let mut bytes = Vec::new();
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
        };
        options
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .share_mode(FILE_SHARE_READ);
    }
    options
        .open(path)
        .and_then(|file| file.take(METADATA_LIMIT + 1).read_to_end(&mut bytes))
        .map_err(|_| {
            Failure(
                ScanCoverageStatus::Failed,
                "application metadata could not be read",
            )
        })?;
    if bytes.len() as u64 > METADATA_LIMIT {
        return Err(Failure(
            ScanCoverageStatus::BudgetLimited,
            "application metadata exceeded its read budget",
        ));
    }
    Ok(bytes)
}

fn json_metadata(path: &Path) -> Check<serde_json::Value> {
    serde_json::from_slice(&read_metadata(path)?).map_err(|_| {
        Failure(
            ScanCoverageStatus::OwnershipUnknown,
            "application metadata is invalid",
        )
    })
}

fn leaf(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.ends_with(['.', ' '])
        && !value.contains(['/', '\\', ':', '\0'])
}

fn children(path: &Path, limit: usize, budget: &mut Budget) -> Check<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(path).map_err(|_| {
        Failure(
            ScanCoverageStatus::Failed,
            "cache directory enumeration failed",
        )
    })? {
        budget.spend()?;
        if paths.len() == limit {
            return Err(Failure(
                ScanCoverageStatus::BudgetLimited,
                "cache directory enumeration exceeded its entry budget",
            ));
        }
        paths.push(
            entry
                .map_err(|_| {
                    Failure(
                        ScanCoverageStatus::Failed,
                        "cache directory entry could not be read",
                    )
                })?
                .path(),
        );
    }
    paths.sort();
    Ok(paths)
}

fn cache_children(root: &Path, names: &[&str], paths: &mut Vec<PathBuf>) -> Check<()> {
    for name in names {
        let path = root.join(name);
        match fs::symlink_metadata(&path) {
            Ok(_) => paths.push(path), // Keep unsafe/busy observations; verification refuses them.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => {
                return Err(Failure(
                    ScanCoverageStatus::Failed,
                    "cache child existence could not be determined",
                ));
            }
        }
    }
    Ok(())
}

fn resolve(
    roots: &Roots,
    family: WindowsCacheFamily,
    root: &Path,
    budget: &mut Budget,
    result: &mut WindowsCacheDiscovery,
) -> Check<Vec<PathBuf>> {
    directory(root)?;
    let mut paths = Vec::new();
    use WindowsCacheFamily::*;
    match family {
        Chrome | Edge => {
            let metadata = json_metadata(&root.join("Local State"))?;
            let profiles = metadata
                .pointer("/profile/info_cache")
                .and_then(serde_json::Value::as_object)
                .ok_or(Failure(
                    ScanCoverageStatus::OwnershipUnknown,
                    "registered Chromium profiles are unavailable",
                ))?;
            if profiles.len() > PROFILE_LIMIT {
                return Err(Failure(
                    ScanCoverageStatus::BudgetLimited,
                    "registered profile budget exhausted",
                ));
            }
            for name in profiles.keys() {
                budget.spend()?;
                if !leaf(name) {
                    return Err(Failure(
                        ScanCoverageStatus::OwnershipUnknown,
                        "invalid registered profile path",
                    ));
                }
                let profile = root.join(name);
                let checked = (|| {
                    directory(&profile)?;
                    if !json_metadata(&profile.join("Preferences"))?.is_object() {
                        return Err(Failure(
                            ScanCoverageStatus::OwnershipUnknown,
                            "profile preferences are invalid",
                        ));
                    }
                    cache_children(&profile, &["Cache", "Code Cache", "GPUCache"], &mut paths)
                })();
                if let Err(error) = checked {
                    issue(result, profile, error);
                }
            }
        }
        Firefox => {
            let roaming = roots
                .roaming
                .as_ref()
                .ok_or(Failure(
                    ScanCoverageStatus::OwnershipUnknown,
                    "Firefox roaming profile root unavailable",
                ))?
                .join("Mozilla/Firefox");
            let bytes = read_metadata(&roaming.join("profiles.ini"))?;
            let text = std::str::from_utf8(&bytes).map_err(|_| {
                Failure(
                    ScanCoverageStatus::OwnershipUnknown,
                    "Firefox profile registry encoding unsupported",
                )
            })?;
            let profiles = firefox_profiles(text)?;
            for (section, failure) in profiles.unavailable {
                result.unavailable.push(WindowsCacheUnavailable {
                    path: roaming.join("profiles.ini"),
                    status: failure.0,
                    detail: format!("Firefox {section}: {}", failure.1),
                });
            }
            for name in profiles.names {
                budget.spend()?;
                let profile = root.join("Profiles").join(&name);
                let checked = (|| {
                    directory(&roaming.join("Profiles").join(&name))?;
                    directory(&profile)?;
                    cache_children(&profile, &["cache2"], &mut paths)
                })();
                if let Err(error) = checked {
                    issue(result, profile, error);
                }
            }
        }
        VsCode | Cursor => {
            if roots.editor_override {
                return Err(Failure(
                    ScanCoverageStatus::Unsupported,
                    "custom or portable editor user-data layout requires separate verification",
                ));
            }
            directory(&root.join("User"))?;
            if !editor_installed(roots, family) {
                return Err(Failure(
                    ScanCoverageStatus::OwnershipUnknown,
                    "matching standard editor installation metadata unavailable",
                ));
            }
            cache_children(
                root,
                &["Cache", "Code Cache", "GPUCache", "CachedData"],
                &mut paths,
            )?;
        }
        UserTemp => {
            // Also prove the root belongs to the current token before enumerating.
            native_temp_probe(root, false)?;
            paths = children(root, CHILD_LIMIT, budget)?;
        }
    }
    Ok(paths)
}

#[derive(Default)]
struct FirefoxProfiles {
    names: Vec<String>,
    unavailable: Vec<(String, Failure)>,
}

#[derive(Default)]
struct FirefoxProfile {
    section: String,
    path: Option<String>,
    relative: Option<String>,
    ambiguous: bool,
}

fn firefox_profiles(text: &str) -> Check<FirefoxProfiles> {
    let mut sections: Vec<FirefoxProfile> = Vec::new();
    let mut profile = None;
    for raw in text.trim_start_matches('\u{feff}').lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with([';', '#']) {
            continue;
        }
        if line.starts_with('[') {
            if !line.ends_with(']') {
                return Err(Failure(
                    ScanCoverageStatus::OwnershipUnknown,
                    "invalid Firefox registry section",
                ));
            }
            let name = &line[1..line.len() - 1];
            profile = if name.strip_prefix("Profile").is_some_and(|suffix| {
                !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
            }) {
                if sections.len() == PROFILE_LIMIT {
                    return Err(Failure(
                        ScanCoverageStatus::BudgetLimited,
                        "Firefox registered profile budget exhausted",
                    ));
                }
                if sections.iter().any(|section| section.section == name) {
                    return Err(Failure(
                        ScanCoverageStatus::OwnershipUnknown,
                        "duplicate Firefox registry section",
                    ));
                }
                sections.push(FirefoxProfile {
                    section: name.to_owned(),
                    ..Default::default()
                });
                Some(sections.len() - 1)
            } else {
                None
            };
        } else if let Some(index) = profile {
            let (key, value) = line.split_once('=').ok_or(Failure(
                ScanCoverageStatus::OwnershipUnknown,
                "invalid Firefox profile metadata",
            ))?;
            let slot = match key.trim() {
                "Path" => Some(&mut sections[index].path),
                "IsRelative" => Some(&mut sections[index].relative),
                _ => None,
            };
            if let Some(slot) = slot
                && slot.replace(value.trim().to_owned()).is_some()
            {
                sections[index].ambiguous = true;
            }
        }
    }
    let mut profiles = FirefoxProfiles::default();
    for section in sections {
        let checked = (|| {
            if section.ambiguous {
                return Err(Failure(
                    ScanCoverageStatus::OwnershipUnknown,
                    "ambiguous Firefox profile metadata",
                ));
            }
            if section.relative.as_deref() != Some("1") {
                return Err(Failure(
                    ScanCoverageStatus::Unsupported,
                    "custom absolute Firefox profiles are not supported",
                ));
            }
            let path = section
                .path
                .ok_or(Failure(
                    ScanCoverageStatus::OwnershipUnknown,
                    "registered Firefox profile path missing",
                ))?
                .replace('\\', "/");
            let name = path
                .strip_prefix("Profiles/")
                .filter(|name| leaf(name))
                .ok_or(Failure(
                    ScanCoverageStatus::Unsupported,
                    "custom Firefox profile layout is not supported",
                ))?;
            Ok(name.to_owned())
        })();
        match checked {
            Ok(name) => {
                if !profiles
                    .names
                    .iter()
                    .any(|known: &String| known.eq_ignore_ascii_case(&name))
                {
                    profiles.names.push(name);
                }
            }
            Err(error) => profiles.unavailable.push((section.section, error)),
        }
    }
    Ok(profiles)
}

fn editor_installed(roots: &Roots, family: WindowsCacheFamily) -> bool {
    let (directory_name, exe, application) = if family == WindowsCacheFamily::VsCode {
        ("Microsoft VS Code", "Code.exe", "code")
    } else {
        ("cursor", "Cursor.exe", "cursor")
    };
    let mut installs: Vec<PathBuf> = roots
        .local
        .iter()
        .map(|root| root.join("Programs").join(directory_name))
        .collect();
    installs.extend(
        roots
            .program_files
            .iter()
            .map(|root| root.join(directory_name)),
    );
    installs.iter().any(|root| {
        directory(root).is_ok()
            && safe_entry(&root.join(exe)).is_ok_and(|entry| entry.is_file)
            && json_metadata(&root.join("resources/app/product.json")).is_ok_and(|json| {
                json.get("applicationName")
                    .and_then(serde_json::Value::as_str)
                    == Some(application)
            })
    })
}

fn retained_name(path: &Path) -> bool {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    matches!(
        name.as_str(),
        ".git"
            | ".hg"
            | ".svn"
            | "cookies"
            | "login data"
            | "history"
            | "bookmarks"
            | "indexeddb"
            | "local storage"
            | "service worker"
            | "user"
            | "backups"
            | "extensions"
            | "workspacestorage"
            | "globalstorage"
            | "state.vscdb"
            | "id_rsa"
            | "id_ed25519"
            | "credentials.toml"
            | ".npmrc"
            | ".pypirc"
            | ".netrc"
    ) || name.starts_with(".env")
        || name.ends_with(".pem")
        || name.ends_with(".key")
        || name.ends_with(".pfx")
        || name.ends_with(".p12")
}

fn verify_tree(path: &Path, temporary: bool, now: SystemTime, budget: &mut Budget) -> Check<()> {
    verify_tree_with_probe(path, temporary, now, budget, &SystemPathReparseProbe)
}

fn verify_tree_with_probe(
    path: &Path,
    temporary: bool,
    now: SystemTime,
    budget: &mut Budget,
    probe: &dyn crate::filesystem::PathReparseProbe,
) -> Check<()> {
    safe_ancestors(path)?;
    if !temporary && !safe_entry(path)?.is_dir {
        return Err(Failure(
            ScanCoverageStatus::OwnershipUnknown,
            "application cache child is not a directory",
        ));
    }
    let mut pending = vec![(path.to_path_buf(), 0)];
    while let Some((current, depth)) = pending.pop() {
        budget.spend()?;
        if depth > 64 {
            return Err(Failure(
                ScanCoverageStatus::BudgetLimited,
                "cache verification depth budget exhausted",
            ));
        }
        let entry = safe_entry_with_probe(&current, probe)?;
        if retained_name(&current) {
            return Err(Failure(
                ScanCoverageStatus::Protected,
                "entry contains persistent, repository or secret material",
            ));
        }
        if temporary {
            if entry
                .modified
                .and_then(|modified| now.duration_since(modified).ok())
                .is_none_or(|age| age < MIN_TEMP_AGE)
            {
                return Err(Failure(
                    ScanCoverageStatus::Skipped,
                    "temporary entry or descendant is recent or has unknown age",
                ));
            }
            native_temp_probe(&current, true)?;
        }
        if entry.is_dir {
            pending.extend(
                children(&current, CHILD_LIMIT, budget)?
                    .into_iter()
                    .map(|child| (child, depth + 1)),
            );
        }
    }
    Ok(())
}

/// Closed callers supply executable basenames only; snapshot failure is never idle.
pub(crate) fn owner_state(names: &[&str]) -> OwnerState {
    #[cfg(test)]
    if let Some(state) = TEST_OWNER_STATE.with(|value| value.get()) {
        return state;
    }
    if names.is_empty()
        || names
            .iter()
            .any(|name| !leaf(name) || !name.to_ascii_lowercase().ends_with(".exe"))
    {
        return OwnerState::Unknown;
    }
    native_owner_state(names)
}

#[cfg(test)]
thread_local! { static TEST_OWNER_STATE: std::cell::Cell<Option<OwnerState>> = const { std::cell::Cell::new(None) }; }

/// Test-only observation seam; root, profile, path, content and executor checks remain real.
#[cfg(test)]
pub(crate) fn with_owner_state_for_test<T>(state: OwnerState, f: impl FnOnce() -> T) -> T {
    struct Restore(Option<OwnerState>);
    impl Drop for Restore {
        fn drop(&mut self) {
            TEST_OWNER_STATE.with(|value| value.set(self.0));
        }
    }
    let _restore = Restore(TEST_OWNER_STATE.with(|value| value.replace(Some(state))));
    f()
}

#[cfg(not(windows))]
fn native_owner_state(_names: &[&str]) -> OwnerState {
    OwnerState::Unknown
}

#[cfg(windows)]
fn native_owner_state(names: &[&str]) -> OwnerState {
    use windows_sys::Win32::{
        Foundation::{ERROR_NO_MORE_FILES, GetLastError},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
    };
    // SAFETY: read-only snapshot; Handle owns exactly the returned handle.
    let Some(snapshot) =
        NativeHandle::new(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) })
    else {
        return OwnerState::Unknown;
    };
    // SAFETY: sized zeroed PROCESSENTRY32W is the documented API input.
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    // SAFETY: snapshot and entry remain valid throughout enumeration.
    if unsafe { Process32FirstW(snapshot.0, &mut entry) } == 0 {
        return OwnerState::Unknown;
    }
    for _ in 0..32_768 {
        let end = entry
            .szExeFile
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(entry.szExeFile.len());
        let Ok(name) = String::from_utf16(&entry.szExeFile[..end]) else {
            return OwnerState::Unknown;
        };
        if name.is_empty() {
            return OwnerState::Unknown;
        }
        if names
            .iter()
            .any(|expected| name.eq_ignore_ascii_case(expected))
        {
            return OwnerState::Busy;
        }
        // SAFETY: snapshot and sized output buffer remain valid; terminal error is checked.
        if unsafe { Process32NextW(snapshot.0, &mut entry) } == 0 {
            // SAFETY: reads this thread's immediately preceding Win32 error.
            return if unsafe { GetLastError() } == ERROR_NO_MORE_FILES {
                OwnerState::Idle
            } else {
                OwnerState::Unknown
            };
        }
    }
    OwnerState::Unknown
}

#[cfg(windows)]
struct NativeHandle(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl NativeHandle {
    fn new(handle: windows_sys::Win32::Foundation::HANDLE) -> Option<Self> {
        (!handle.is_null() && handle != windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE)
            .then_some(Self(handle))
    }
}
#[cfg(windows)]
impl Drop for NativeHandle {
    fn drop(&mut self) {
        // SAFETY: the constructor accepted one owned non-null, non-invalid handle.
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(not(windows))]
fn native_temp_probe(_path: &Path, _exclusive: bool) -> Check<()> {
    Err(Failure(
        ScanCoverageStatus::Unsupported,
        "Windows temporary ownership proof unavailable",
    ))
}

#[cfg(windows)]
fn native_temp_probe(path: &Path, exclusive: bool) -> Check<()> {
    use std::{
        os::windows::{
            fs::{MetadataExt, OpenOptionsExt},
            io::AsRawHandle,
        },
        ptr,
    };
    use windows_sys::Win32::{
        Foundation::{ERROR_LOCK_VIOLATION, ERROR_SHARING_VIOLATION, GENERIC_READ},
        Security::{
            EqualSid, GetKernelObjectSecurity, GetSecurityDescriptorOwner, GetTokenInformation,
            IsValidSid, OWNER_SECURITY_INFORMATION, TOKEN_QUERY, TOKEN_USER, TokenUser,
        },
        Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
            FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, READ_CONTROL,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };
    let file = fs::OpenOptions::new().read(true).access_mode(READ_CONTROL | FILE_READ_ATTRIBUTES | if exclusive { GENERIC_READ } else { 0 }).share_mode(if exclusive { 0 } else { FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE }).custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT).open(path).map_err(|error| Failure(if matches!(error.raw_os_error(), Some(code) if code == ERROR_SHARING_VIOLATION as i32 || code == ERROR_LOCK_VIOLATION as i32) { ScanCoverageStatus::Busy } else { ScanCoverageStatus::OwnershipUnknown }, "temporary entry is locked or cannot be inspected exclusively"))?;
    let metadata = file.metadata().map_err(|_| {
        Failure(
            ScanCoverageStatus::OwnershipUnknown,
            "temporary open-handle metadata unavailable",
        )
    })?;
    if metadata.file_attributes()
        & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
        != 0
    {
        return Err(Failure(
            ScanCoverageStatus::Protected,
            "temporary entry became a reparse point",
        ));
    }
    let mut token = ptr::null_mut();
    // SAFETY: valid process pseudo-handle and writable output; read-only token query.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(Failure(
            ScanCoverageStatus::OwnershipUnknown,
            "current-user token unavailable",
        ));
    }
    let token = NativeHandle::new(token).ok_or(Failure(
        ScanCoverageStatus::OwnershipUnknown,
        "current-user token unavailable",
    ))?;
    let mut length = 0;
    // SAFETY: null buffer with zero length asks for required byte count.
    unsafe {
        GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut length);
    }
    if length < std::mem::size_of::<TOKEN_USER>() as u32 || length > 65536 {
        return Err(Failure(
            ScanCoverageStatus::OwnershipUnknown,
            "current-user SID size unavailable",
        ));
    }
    let mut user = vec![0usize; (length as usize).div_ceil(std::mem::size_of::<usize>())];
    // SAFETY: usize storage is aligned and has at least length bytes.
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            user.as_mut_ptr().cast(),
            length,
            &mut length,
        )
    } == 0
    {
        return Err(Failure(
            ScanCoverageStatus::OwnershipUnknown,
            "current-user SID unavailable",
        ));
    }
    let mut security_length = 0;
    // SAFETY: live no-follow file handle, null buffer requests required descriptor size.
    unsafe {
        GetKernelObjectSecurity(
            file.as_raw_handle(),
            OWNER_SECURITY_INFORMATION,
            ptr::null_mut(),
            0,
            &mut security_length,
        );
    }
    if security_length == 0 || security_length > 65536 {
        return Err(Failure(
            ScanCoverageStatus::OwnershipUnknown,
            "temporary owner descriptor unavailable",
        ));
    }
    let mut security =
        vec![0usize; (security_length as usize).div_ceil(std::mem::size_of::<usize>())];
    let mut owner = ptr::null_mut();
    let mut defaulted = 0;
    // SAFETY: descriptor output is aligned/bounded; returned SID pointers stay within
    // live buffers until comparison. Never dereference an unsuccessful API result.
    let same_owner = unsafe {
        GetKernelObjectSecurity(
            file.as_raw_handle(),
            OWNER_SECURITY_INFORMATION,
            security.as_mut_ptr().cast(),
            security_length,
            &mut security_length,
        ) != 0
            && GetSecurityDescriptorOwner(security.as_mut_ptr().cast(), &mut owner, &mut defaulted)
                != 0
            && !owner.is_null()
            && IsValidSid(owner) != 0
            && {
                let sid = (*(user.as_ptr().cast::<TOKEN_USER>())).User.Sid;
                !sid.is_null() && IsValidSid(sid) != 0 && EqualSid(owner, sid) != 0
            }
    };
    if !same_owner {
        return Err(Failure(
            ScanCoverageStatus::OwnershipUnknown,
            "temporary entry is not proven owned by the current user",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{TempDir, tempdir};

    fn fixture() -> (TempDir, Roots) {
        let directory = tempdir().unwrap();
        let roots = Roots {
            local: Some(directory.path().join("Local")),
            roaming: Some(directory.path().join("Roaming")),
            program_files: vec![],
            temp_override: None,
            editor_override: false,
        };
        (directory, roots)
    }
    fn file(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    fn chromium(roots: &Roots, family: WindowsCacheFamily) -> PathBuf {
        let root = roots.root(family).unwrap();
        file(
            &root.join("Local State"),
            r#"{"profile":{"info_cache":{"Default":{},"Profile 1":{}}}}"#,
        );
        for profile in ["Default", "Profile 1"] {
            file(&root.join(profile).join("Preferences"), "{}");
            for cache in ["Cache", "Code Cache", "GPUCache"] {
                file(&root.join(profile).join(cache).join("payload"), "cached");
            }
            for retained in [
                "Cookies",
                "Login Data",
                "History",
                "IndexedDB/db",
                "Local Storage/db",
                "Service Worker/db",
            ] {
                file(&root.join(profile).join(retained), "keep");
            }
        }
        file(&root.join("Unregistered/Cache/payload"), "keep");
        root
    }

    #[test]
    fn registered_chromium_profiles_only_and_persistent_sentinels_survive() {
        for family in [WindowsCacheFamily::Chrome, WindowsCacheFamily::Edge] {
            let (_directory, roots) = fixture();
            let root = chromium(&roots, family);
            let result = discover(&roots, family, OwnerState::Idle, None);
            assert_eq!(result.candidates.len(), 6);
            assert!(result.unavailable.is_empty(), "{:?}", result.unavailable);
            assert!(!result.candidates.contains(&root));
            assert!(
                !result
                    .candidates
                    .iter()
                    .any(|path| path.starts_with(root.join("Unregistered")))
            );
            assert_eq!(
                fs::read_to_string(root.join("Default/Login Data")).unwrap(),
                "keep"
            );
            assert_eq!(
                fs::read_to_string(root.join("Default/Service Worker/db")).unwrap(),
                "keep"
            );
        }
    }

    #[test]
    fn busy_and_unknown_are_visible_and_profile_failure_does_not_hide_healthy_profile() {
        let (_directory, roots) = fixture();
        let root = chromium(&roots, WindowsCacheFamily::Chrome);
        for (state, expected) in [
            (OwnerState::Busy, ScanCoverageStatus::Busy),
            (OwnerState::Unknown, ScanCoverageStatus::OwnershipUnknown),
        ] {
            let result = discover(&roots, WindowsCacheFamily::Chrome, state, None);
            assert_eq!(result.candidates.len(), 6);
            assert_eq!(result.unavailable.len(), 6);
            assert!(
                result
                    .unavailable
                    .iter()
                    .all(|issue| issue.status == expected)
            );
        }
        file(&root.join("Profile 1/Preferences"), "invalid");
        let result = discover(&roots, WindowsCacheFamily::Chrome, OwnerState::Idle, None);
        assert_eq!(result.candidates.len(), 3);
        assert_eq!(result.unavailable[0].path, root.join("Profile 1"));
        assert!(
            discover(
                &roots,
                WindowsCacheFamily::Chrome,
                OwnerState::Idle,
                Some(&root.join("Default/History"))
            )
            .candidates
            .is_empty()
        );
    }

    #[test]
    fn profile_path_traversal_foreign_names_and_oversized_metadata_fail_closed() {
        let (_directory, roots) = fixture();
        let root = chromium(&roots, WindowsCacheFamily::Chrome);
        for contents in [
            r#"{"profile":{"info_cache":{"../foreign":{}}}}"#,
            r#"{"profile":{"info_cache":{"C:\\foreign":{}}}}"#,
            "{}",
        ] {
            file(&root.join("Local State"), contents);
            let result = discover(&roots, WindowsCacheFamily::Chrome, OwnerState::Idle, None);
            assert!(result.candidates.is_empty());
            assert_eq!(
                result.unavailable[0].status,
                ScanCoverageStatus::OwnershipUnknown
            );
        }
        file(
            &root.join("Local State"),
            &" ".repeat(METADATA_LIMIT as usize + 1),
        );
        assert!(
            discover(&roots, WindowsCacheFamily::Chrome, OwnerState::Idle, None)
                .candidates
                .is_empty()
        );
    }

    #[test]
    fn firefox_registration_and_local_roaming_separation_are_required() {
        let (_directory, roots) = fixture();
        let root = roots.root(WindowsCacheFamily::Firefox).unwrap();
        let roaming = roots.roaming.as_ref().unwrap().join("Mozilla/Firefox");
        file(
            &roaming.join("profiles.ini"),
            "[General]\nStartWithLastProfile=1\n[Profile0]\nName=default\nIsRelative=1\nPath=Profiles/abc.default\n",
        );
        file(&roaming.join("Profiles/abc.default/places.sqlite"), "keep");
        file(
            &root.join("Profiles/abc.default/cache2/entries/payload"),
            "cache",
        );
        file(
            &root.join("Profiles/foreign/cache2/entries/payload"),
            "keep",
        );
        let result = discover(&roots, WindowsCacheFamily::Firefox, OwnerState::Idle, None);
        assert_eq!(
            result.candidates,
            vec![root.join("Profiles/abc.default/cache2")]
        );
        assert!(result.unavailable.is_empty(), "{:?}", result.unavailable);
        assert_eq!(
            fs::read_to_string(roaming.join("Profiles/abc.default/places.sqlite")).unwrap(),
            "keep"
        );
        for text in [
            "[Profile0]\nIsRelative=0\nPath=C:/foreign\n",
            "[Profile0]\nIsRelative=1\nPath=Profiles/../foreign\n",
            "[Profile0]\nIsRelative=1\nPath=Profiles/a\nPath=Profiles/b\n",
        ] {
            let parsed = firefox_profiles(text).expect("independent profile failure");
            assert!(parsed.names.is_empty());
            assert_eq!(parsed.unavailable.len(), 1);
        }
        assert!(firefox_profiles("[Profile0\nPath=Profiles/a").is_err());
        let mixed = "[Profile0]\nIsRelative=1\nPath=Profiles/abc.default\n[Profile1]\nIsRelative=0\nPath=C:/foreign\n[Profile2]\nIsRelative=1\nPath=Profiles/a\nPath=Profiles/b\n";
        file(&roaming.join("profiles.ini"), mixed);
        let result = discover(&roots, WindowsCacheFamily::Firefox, OwnerState::Idle, None);
        assert_eq!(
            result.candidates,
            vec![root.join("Profiles/abc.default/cache2")]
        );
        assert_eq!(result.unavailable.len(), 2);
        assert!(
            result
                .unavailable
                .iter()
                .all(|issue| issue.path == roaming.join("profiles.ini"))
        );
        assert!(
            result
                .unavailable
                .iter()
                .any(|issue| issue.detail.contains("Profile1")
                    && issue.status == ScanCoverageStatus::Unsupported)
        );
        assert!(
            result
                .unavailable
                .iter()
                .any(|issue| issue.detail.contains("Profile2")
                    && issue.status == ScanCoverageStatus::OwnershipUnknown)
        );
    }

    #[test]
    fn editors_require_matching_installation_and_keep_user_agent_state() {
        for (family, directory_name, exe, application) in [
            (
                WindowsCacheFamily::VsCode,
                "Microsoft VS Code",
                "Code.exe",
                "code",
            ),
            (WindowsCacheFamily::Cursor, "cursor", "Cursor.exe", "cursor"),
        ] {
            let (_directory, mut roots) = fixture();
            let root = roots.root(family).unwrap();
            for retained in [
                "User/globalStorage/state.vscdb",
                "User/workspaceStorage/session",
                "Backups/backup",
                "extensions/manifest",
            ] {
                file(&root.join(retained), "keep");
            }
            for cache in ["Cache", "Code Cache", "GPUCache", "CachedData"] {
                file(&root.join(cache).join("payload"), "cache");
            }
            let unproven = discover(&roots, family, OwnerState::Idle, None);
            assert!(unproven.candidates.is_empty());
            assert_eq!(
                unproven.unavailable[0].status,
                ScanCoverageStatus::OwnershipUnknown
            );
            let install = roots
                .local
                .as_ref()
                .unwrap()
                .join("Programs")
                .join(directory_name);
            file(&install.join(exe), "fixture executable marker; never run");
            file(
                &install.join("resources/app/product.json"),
                &format!("{{\"applicationName\":\"{application}\"}}"),
            );
            let result = discover(&roots, family, OwnerState::Idle, None);
            assert_eq!(result.candidates.len(), 4);
            assert!(result.unavailable.is_empty(), "{:?}", result.unavailable);
            assert_eq!(
                fs::read_to_string(root.join("User/workspaceStorage/session")).unwrap(),
                "keep"
            );
            roots.editor_override = true;
            assert_eq!(
                discover(&roots, family, OwnerState::Idle, None).unavailable[0].status,
                ScanCoverageStatus::Unsupported
            );
        }
    }

    #[test]
    fn tree_preserves_secrets_and_reports_budget_exhaustion() {
        let directory = tempdir().unwrap();
        for name in [
            ".git/config",
            ".GIT/config",
            ".env.local",
            ".ENV.local",
            ".npmrc",
            ".pypirc",
            ".netrc",
            "private.PFX",
            "private.P12",
            "id_ed25519",
            "IndexedDB/db",
            "workspaceStorage/state",
        ] {
            let candidate = directory.path().join(name.replace('/', "-"));
            file(&candidate.join(name), "keep");
            assert_eq!(
                verify_tree(&candidate, false, SystemTime::now(), &mut Budget::new())
                    .unwrap_err()
                    .0,
                ScanCoverageStatus::Protected
            );
        }
        let mut budget = Budget {
            entries: 50_000,
            started: Instant::now(),
        };
        assert_eq!(
            verify_tree(directory.path(), false, SystemTime::now(), &mut budget)
                .unwrap_err()
                .0,
            ScanCoverageStatus::BudgetLimited
        );
    }

    #[test]
    fn reparse_and_unverified_children_are_never_traversed() {
        use crate::filesystem::{PathReparseProbe, ReparseProbeResult};
        struct Probe {
            blocked: PathBuf,
            unknown: bool,
            visited: std::sync::Mutex<Vec<PathBuf>>,
        }
        impl PathReparseProbe for Probe {
            fn probe(&self, path: &Path) -> ReparseProbeResult {
                self.visited.lock().unwrap().push(path.to_path_buf());
                if path == self.blocked {
                    if self.unknown {
                        ReparseProbeResult::Unsupported {
                            detail: "fixture unavailable".into(),
                        }
                    } else {
                        ReparseProbeResult::ReparsePoint { tag: 0x9000_001A }
                    }
                } else {
                    ReparseProbeResult::NotReparsePoint
                }
            }
        }
        let directory = tempdir().unwrap();
        let root = directory.path().join("Cache");
        file(&root.join("placeholder/keep"), "sentinel");
        for unknown in [false, true] {
            let probe = Probe {
                blocked: root.join("placeholder"),
                unknown,
                visited: std::sync::Mutex::new(Vec::new()),
            };
            assert_eq!(
                verify_tree_with_probe(&root, false, SystemTime::now(), &mut Budget::new(), &probe)
                    .unwrap_err()
                    .0,
                ScanCoverageStatus::Protected
            );
            assert!(
                !probe
                    .visited
                    .lock()
                    .unwrap()
                    .contains(&root.join("placeholder/keep"))
            );
            assert_eq!(
                fs::read_to_string(root.join("placeholder/keep")).unwrap(),
                "sentinel"
            );
            let root_probe = Probe {
                blocked: root.clone(),
                unknown,
                visited: std::sync::Mutex::new(Vec::new()),
            };
            assert_eq!(
                cache_location_entry(&root, WindowsCacheFamily::Chrome, &root_probe)
                    .unwrap_err()
                    .0,
                ScanCoverageStatus::Protected
            );
            assert_eq!(*root_probe.visited.lock().unwrap(), vec![root.clone()]);
        }
    }

    #[cfg(windows)]
    fn age(path: &Path, when: SystemTime) {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .write(true)
            .custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(when))
            .unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn stale_owned_temp_is_individual_and_recent_nested_or_locked_files_are_unavailable() {
        use std::os::windows::fs::OpenOptionsExt;
        let (_directory, roots) = fixture();
        let root = roots.root(WindowsCacheFamily::UserTemp).unwrap();
        let old = SystemTime::now() - Duration::from_secs(9 * 24 * 60 * 60);
        file(&root.join("old.tmp"), "stale owned fixture");
        file(&root.join("recent.tmp"), "recent fixture");
        file(&root.join("mixed/new.tmp"), "recent nested fixture");
        file(&root.join("locked.tmp"), "locked fixture");
        for path in [
            root.join("old.tmp"),
            root.join("mixed"),
            root.join("locked.tmp"),
        ] {
            age(&path, old);
        }
        let lock = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(root.join("locked.tmp"))
            .unwrap();
        let result = discover(
            &roots,
            WindowsCacheFamily::UserTemp,
            OwnerState::Unknown,
            None,
        );
        assert_eq!(result.candidates.len(), 4, "{:?}", result.unavailable);
        assert!(!result.candidates.contains(&root));
        assert!(
            !result
                .unavailable
                .iter()
                .any(|issue| issue.path == root.join("old.tmp")),
            "{:?}",
            result.unavailable
        );
        assert!(
            result
                .unavailable
                .iter()
                .any(|issue| issue.path == root.join("mixed")
                    && issue.status == ScanCoverageStatus::Skipped)
        );
        assert!(
            result
                .unavailable
                .iter()
                .any(|issue| issue.path == root.join("locked.tmp")
                    && issue.status == ScanCoverageStatus::Busy)
        );
        drop(lock);
        assert!(
            discover(
                &roots,
                WindowsCacheFamily::UserTemp,
                OwnerState::Unknown,
                Some(&root.join("locked.tmp"))
            )
            .unavailable
            .is_empty()
        );
        assert!(root.join("old.tmp").exists());
    }

    #[cfg(windows)]
    #[test]
    fn native_owner_detects_current_process_and_invalid_queries_fail_closed() {
        let name = env::current_exe()
            .unwrap()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        assert_eq!(owner_state(&[&name]), OwnerState::Busy);
        assert_eq!(owner_state(&[]), OwnerState::Unknown);
        assert_eq!(owner_state(&["../code.exe"]), OwnerState::Unknown);
        assert_eq!(
            owner_state(&["devsweep-fixture-impossible-owner-73488.exe"]),
            OwnerState::Idle
        );
    }
}
