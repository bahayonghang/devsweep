use std::{fs, path::Path};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::{
    AiClass, AiDisposition, AiError, AiInstalledTool, AiObservationV1, AiProvider,
    AiProviderObservation, AiRow, AiScanRequest, SESSION_ENUM_LIMIT, check_cancel, is_real_dir,
    normalize_home, protected_component, sha256_hex,
};
use crate::{
    filesystem::{PathSafety, SystemPathReparseProbe, inspect_entry_no_follow},
    process::CancelObserver,
};

/// Joins a provider id to the production default path. This does not read the
/// filesystem and does not scan a user profile.
pub fn default_home_path(
    provider: AiProvider,
    userprofile: Option<&Path>,
    appdata: Option<&Path>,
) -> Option<std::path::PathBuf> {
    match provider {
        AiProvider::ClaudeCode => Some(userprofile?.join(".claude")),
        AiProvider::Codex => Some(userprofile?.join(".codex")),
        AiProvider::Cursor => Some(appdata?.join("Cursor")),
    }
}

/// Classify one explicit home or the production defaults. Does not mutate.
pub fn scan_ai(
    request: &AiScanRequest,
    cancel: &dyn CancelObserver,
) -> Result<AiObservationV1, AiError> {
    check_cancel(cancel)?;
    if request.home.is_some() && request.provider.is_none() {
        return Err(AiError::usage("invalid_cli", "--home requires --provider"));
    }
    let providers = match request.provider {
        Some(provider) => vec![provider],
        None => AiProvider::ALL.to_vec(),
    };
    let mut observed = Vec::with_capacity(providers.len());
    for provider in providers {
        check_cancel(cancel)?;
        observed.push(observe_provider(provider, request.home.as_deref(), cancel)?);
    }
    Ok(AiObservationV1::new(observed))
}

fn observe_provider(
    provider: AiProvider,
    explicit: Option<&Path>,
    cancel: &dyn CancelObserver,
) -> Result<AiProviderObservation, AiError> {
    let Some(home) = explicit
        .map(std::path::Path::to_path_buf)
        .or_else(|| production_default(provider))
    else {
        return Ok(absent(provider));
    };
    if explicit.is_some() {
        if !home.is_absolute() {
            return Err(AiError::usage("invalid_home", "--home must be absolute"));
        }
        if !is_real_dir(&home) {
            return Err(AiError::unavailable(
                "source_unavailable",
                format!("AI home is unavailable: {}", home.display()),
            ));
        }
    } else if !is_real_dir(&home) {
        return Ok(absent(provider));
    }
    let normalized = normalize_home(&home)?;
    let mut rows = Vec::new();
    let session_bounded = classify_home(&home, provider, cancel, &mut rows)?;
    Ok(AiProviderObservation {
        provider,
        state: "present".to_string(),
        home: Some(normalized),
        installed_tool: Some(read_install_identity(&home, provider)),
        rows,
        session_bounded,
    })
}

fn absent(provider: AiProvider) -> AiProviderObservation {
    AiProviderObservation {
        provider,
        state: "absent".to_string(),
        home: None,
        installed_tool: None,
        rows: Vec::new(),
        session_bounded: false,
    }
}

fn production_default(provider: AiProvider) -> Option<std::path::PathBuf> {
    let userprofile = std::env::var_os("USERPROFILE").map(std::path::PathBuf::from);
    let appdata = std::env::var_os("APPDATA").map(std::path::PathBuf::from);
    default_home_path(provider, userprofile.as_deref(), appdata.as_deref())
}

pub(crate) fn classify_home(
    home: &Path,
    provider: AiProvider,
    cancel: &dyn CancelObserver,
    rows: &mut Vec<AiRow>,
) -> Result<bool, AiError> {
    let entries = read_dir_sorted(home).map_err(|error| {
        super::io_unavailable(error, format!("failed to read AI home {}", home.display()))
    })?;
    let mut session_bounded = false;
    for entry in entries {
        check_cancel(cancel)?;
        let Some(name) = utf8_name(&entry) else {
            continue;
        };
        if name == ".devsweep-recovery" || name.starts_with('.') && name != ".gitmodules" {
            continue;
        }
        if !entry_is_safe(entry.path().as_path()) {
            rows.push(row(
                provider,
                AiClass::SessionArchive,
                &name,
                AiDisposition::UnsupportedVersion,
                "unsupported_version",
                None,
            ));
            continue;
        }
        if protected_component(&name) {
            rows.push(row(
                provider,
                AiClass::InstalledTool,
                &name,
                AiDisposition::Protected,
                "protected",
                None,
            ));
            continue;
        }
        match name.as_str() {
            "cache" if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) => {
                classify_cache(home, provider, rows)?;
            }
            "versions" if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) => {
                classify_versions(home, provider, rows)?;
            }
            "sessions" if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) => {
                session_bounded = classify_sessions(home, provider, rows)?;
            }
            "worktrees" if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) => {
                super::worktree::classify_worktrees(home, provider, cancel, rows)?;
            }
            _ => rows.push(row(
                provider,
                AiClass::InstalledTool,
                &name,
                AiDisposition::UnsupportedVersion,
                "unsupported_version",
                None,
            )),
        }
    }
    Ok(session_bounded)
}

fn classify_cache(home: &Path, provider: AiProvider, rows: &mut Vec<AiRow>) -> Result<(), AiError> {
    let cache = home.join("cache");
    for entry in read_dir_sorted(&cache)
        .map_err(|error| super::io_unavailable(error, "failed to read cache"))?
    {
        let Some(name) = utf8_name(&entry) else {
            continue;
        };
        let relative = format!("cache/{name}");
        if protected_component(&name) || !entry_is_safe(&entry.path()) {
            rows.push(row(
                provider,
                AiClass::RegenerableCache,
                &relative,
                AiDisposition::Protected,
                "protected",
                None,
            ));
            continue;
        }
        if name.starts_with("regen-") {
            match content_hash(&entry.path()) {
                Ok(hash) => rows.push(row(
                    provider,
                    AiClass::RegenerableCache,
                    &relative,
                    AiDisposition::Candidate,
                    "selectable",
                    Some(hash),
                )),
                Err(reason) => rows.push(row(
                    provider,
                    AiClass::RegenerableCache,
                    &relative,
                    AiDisposition::InspectOnly,
                    reason,
                    None,
                )),
            }
        } else {
            rows.push(row(
                provider,
                AiClass::RegenerableCache,
                &relative,
                AiDisposition::InspectOnly,
                "not_regen",
                None,
            ));
        }
    }
    Ok(())
}

fn classify_versions(
    home: &Path,
    provider: AiProvider,
    rows: &mut Vec<AiRow>,
) -> Result<(), AiError> {
    let root = home.join("versions");
    let current = read_single_line(&root.join("current.txt"));
    for entry in read_dir_sorted(&root)
        .map_err(|error| super::io_unavailable(error, "failed to read versions"))?
    {
        let Some(name) = utf8_name(&entry) else {
            continue;
        };
        if name == "current.txt" || !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        let relative = format!("versions/{name}");
        if protected_component(&name) || !entry_is_safe(&entry.path()) {
            rows.push(row(
                provider,
                AiClass::SupersededVersion,
                &relative,
                AiDisposition::Protected,
                "protected",
                None,
            ));
            continue;
        }
        let Some(current) = current.as_deref() else {
            rows.push(row(
                provider,
                AiClass::SupersededVersion,
                &relative,
                AiDisposition::InspectOnly,
                "current_unknown",
                None,
            ));
            continue;
        };
        if name == current {
            rows.push(row(
                provider,
                AiClass::SupersededVersion,
                &relative,
                AiDisposition::Protected,
                "current_version",
                None,
            ));
            continue;
        }
        match content_hash(&entry.path()) {
            Ok(hash) => rows.push(row(
                provider,
                AiClass::SupersededVersion,
                &relative,
                AiDisposition::Candidate,
                "selectable",
                Some(hash),
            )),
            Err(reason) => rows.push(row(
                provider,
                AiClass::SupersededVersion,
                &relative,
                AiDisposition::InspectOnly,
                reason,
                None,
            )),
        }
    }
    Ok(())
}

fn classify_sessions(
    home: &Path,
    provider: AiProvider,
    rows: &mut Vec<AiRow>,
) -> Result<bool, AiError> {
    let root = home.join("sessions");
    if lock_present(&root) {
        rows.push(row(
            provider,
            AiClass::SessionArchive,
            "sessions",
            AiDisposition::InspectOnly,
            "locked",
            None,
        ));
        return Ok(false);
    }
    let active = read_active_id(&root.join("active"));
    let mut json_names = Vec::new();
    for entry in read_dir_sorted(&root)
        .map_err(|error| super::io_unavailable(error, "failed to read sessions"))?
    {
        let Some(name) = utf8_name(&entry) else {
            continue;
        };
        if name == "active" || name == "index.lock" {
            continue;
        }
        if name.ends_with(".json") {
            json_names.push(name);
        } else {
            rows.push(row(
                provider,
                AiClass::SessionArchive,
                &format!("sessions/{name}"),
                AiDisposition::UnsupportedVersion,
                "unsupported_version",
                None,
            ));
        }
    }
    let bounded = json_names.len() > SESSION_ENUM_LIMIT;
    if bounded {
        json_names.truncate(SESSION_ENUM_LIMIT);
    }
    let active_id = match active {
        ActiveId::Known(id) => Some(id),
        ActiveId::Unknown => {
            rows.push(row(
                provider,
                AiClass::SessionArchive,
                "sessions/active",
                AiDisposition::InspectOnly,
                "unknown_active_owner",
                None,
            ));
            return Ok(bounded);
        }
        ActiveId::Absent => None,
    };
    for name in json_names {
        let relative = format!("sessions/{name}");
        let path = root.join(&name);
        if protected_component(&name) || !entry_is_safe(&path) {
            rows.push(row(
                provider,
                AiClass::SessionArchive,
                &relative,
                AiDisposition::Protected,
                "protected",
                None,
            ));
            continue;
        }
        match classify_session_file(&path, &name, active_id.as_deref()) {
            SessionClass::Candidate(hash) => rows.push(row(
                provider,
                AiClass::SessionArchive,
                &relative,
                AiDisposition::Candidate,
                "selectable",
                Some(hash),
            )),
            SessionClass::Refused(reason) => rows.push(row(
                provider,
                AiClass::SessionArchive,
                &relative,
                if reason == "active_session" {
                    AiDisposition::Protected
                } else {
                    AiDisposition::UnsupportedVersion
                },
                reason,
                None,
            )),
        }
    }
    Ok(bounded)
}

enum ActiveId {
    Absent,
    Known(String),
    Unknown,
}

fn read_active_id(path: &Path) -> ActiveId {
    if !path.exists() {
        return ActiveId::Absent;
    }
    let Ok(text) = fs::read_to_string(path) else {
        return ActiveId::Unknown;
    };
    let text = text.trim_end_matches(['\r', '\n']);
    if text.is_empty() || text.contains('\n') || text.contains('\r') || !safe_id(text) {
        return ActiveId::Unknown;
    }
    ActiveId::Known(text.to_string())
}

enum SessionClass {
    Candidate(String),
    Refused(&'static str),
}

fn classify_session_file(path: &Path, file_name: &str, active_id: Option<&str>) -> SessionClass {
    let Some(stem) = file_name.strip_suffix(".json") else {
        return SessionClass::Refused("unsupported_version");
    };
    if !safe_id(stem) {
        return SessionClass::Refused("unsupported_version");
    }
    let Ok(bytes) = fs::read(path) else {
        return SessionClass::Refused("unreadable");
    };
    if bytes.len() > MAX_FILE_BYTES {
        return SessionClass::Refused("bounded");
    }
    let Ok(parsed) = serde_json::from_slice::<SessionFile>(&bytes) else {
        return SessionClass::Refused("unsupported_version");
    };
    if parsed.version != 1 || parsed.id != stem || parsed.active {
        let reason = if parsed.active || active_id == Some(stem) {
            "active_session"
        } else {
            "unsupported_version"
        };
        return SessionClass::Refused(reason);
    }
    if active_id == Some(stem) {
        return SessionClass::Refused("active_session");
    }
    SessionClass::Candidate(sha256_hex(&bytes))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct SessionFile {
    version: u64,
    id: String,
    active: bool,
    body: String,
}

fn read_install_identity(home: &Path, provider: AiProvider) -> AiInstalledTool {
    let path = home.join("install.json");
    let Ok(bytes) = fs::read(&path) else {
        return unknown_install(provider);
    };
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct InstallFile {
        version: String,
    }
    match serde_json::from_slice::<InstallFile>(&bytes) {
        Ok(file) if valid_version_token(&file.version) => AiInstalledTool {
            provider,
            version: file.version,
            install_kind: "install_json".to_string(),
        },
        _ => unknown_install(provider),
    }
}

fn unknown_install(provider: AiProvider) -> AiInstalledTool {
    AiInstalledTool {
        provider,
        version: "unknown".to_string(),
        install_kind: "unknown".to_string(),
    }
}

fn valid_version_token(value: &str) -> bool {
    !value.is_empty()
        && !value.chars().any(|character| {
            character.is_whitespace() || character == '/' || character == '\\' || character == '\0'
        })
}

pub(crate) fn content_hash(path: &Path) -> Result<String, &'static str> {
    if !entry_is_safe(path) {
        return Err("unsupported_version");
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| "unreadable")?;
    if metadata.file_type().is_symlink() {
        return Err("unsupported_version");
    }
    if metadata.is_file() {
        if metadata.len() > MAX_FILE_BYTES as u64 {
            return Err("bounded");
        }
        let bytes = fs::read(path).map_err(|_| "unreadable")?;
        return Ok(sha256_hex(&bytes));
    }
    if metadata.is_dir() {
        return tree_hash(path);
    }
    Err("unsupported_version")
}

fn tree_hash(root: &Path) -> Result<String, &'static str> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files, 0)?;
    files.sort();
    let mut hasher = Sha256::new();
    for (relative, hash) in files {
        hasher.update(relative.as_bytes());
        hasher.update(b"\0");
        hasher.update(hash.as_bytes());
        hasher.update(b"\n");
    }
    Ok(super::hex_encode(hasher.finalize().as_slice()))
}

fn collect_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<(String, String)>,
    depth: usize,
) -> Result<(), &'static str> {
    if depth > 32 || files.len() > 8192 {
        return Err("bounded");
    }
    let metadata = fs::symlink_metadata(current).map_err(|_| "unreadable")?;
    if metadata.file_type().is_symlink() || !entry_is_safe(current) {
        return Err("unsupported_version");
    }
    if metadata.is_file() {
        if metadata.len() > MAX_FILE_BYTES as u64 {
            return Err("bounded");
        }
        let bytes = fs::read(current).map_err(|_| "unreadable")?;
        files.push((relative_from(root, current)?, sha256_hex(&bytes)));
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err("unsupported_version");
    }
    for entry in read_dir_sorted(current).map_err(|_| "unreadable")? {
        let Some(name) = utf8_name(&entry) else {
            return Err("unsupported_version");
        };
        if name == ".git" && entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        collect_files(root, &entry.path(), files, depth + 1)?;
    }
    Ok(())
}

fn relative_from(root: &Path, path: &Path) -> Result<String, &'static str> {
    let relative = path.strip_prefix(root).map_err(|_| "unreadable")?;
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            std::path::Component::Normal(part) => {
                let part = part.to_str().ok_or("unsupported_version")?;
                parts.push(part);
            }
            _ => return Err("unsupported_version"),
        }
    }
    Ok(parts.join("/"))
}

pub(crate) fn read_single_line(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let text = text.trim_end_matches(['\r', '\n']);
    if text.is_empty()
        || text.contains('\n')
        || text.contains('\r')
        || text.contains('/')
        || text.contains('\\')
    {
        return None;
    }
    Some(text.to_string())
}

pub(crate) fn lock_present(dir: &Path) -> bool {
    ["index.lock", "HEAD.lock", "shallow.lock", "config.lock"]
        .into_iter()
        .any(|name| dir.join(name).is_file())
}

pub(crate) fn entry_is_safe(path: &Path) -> bool {
    let probe = SystemPathReparseProbe;
    inspect_entry_no_follow(path, &probe)
        .map(|entry| matches!(entry.safety, PathSafety::Safe))
        .unwrap_or(false)
}

pub(crate) fn read_dir_sorted(path: &Path) -> std::io::Result<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(path)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    Ok(entries)
}

pub(crate) fn utf8_name(entry: &fs::DirEntry) -> Option<String> {
    entry.file_name().into_string().ok()
}

pub(crate) fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

fn row(
    provider: AiProvider,
    class: AiClass,
    relative: &str,
    disposition: AiDisposition,
    reason: &str,
    content_sha256: Option<String>,
) -> AiRow {
    AiRow {
        id: format!("ai:{}:{}:{relative}", provider.as_str(), class.as_str()),
        class,
        relative_path: relative.to_string(),
        disposition,
        reason: reason.to_string(),
        content_sha256,
    }
}

const MAX_FILE_BYTES: usize = 32 * 1024 * 1024;

pub(crate) fn index_hash(home: &Path, class: AiClass) -> Result<String, AiError> {
    let relative = match class {
        AiClass::RegenerableCache => "cache",
        AiClass::SupersededVersion => "versions",
        AiClass::SessionArchive => "sessions",
        AiClass::Worktree => "worktrees",
        AiClass::InstalledTool => return Ok(sha256_hex(b"")),
    };
    let root = home.join(relative);
    if !root.exists() {
        return Ok(sha256_hex(b""));
    }
    Ok(content_hash(&root).unwrap_or_else(|_| sha256_hex(b"unreadable")))
}
