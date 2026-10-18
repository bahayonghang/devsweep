//! Fixture-only MSI, vendor, and Scoop launch policy.
//!
//! Registry uninstall text is split on quotes and whitespace. It is never
//! passed to a shell. A shape that is not one of the three fixture providers
//! stays manual and starts no process.

use std::path::{Component, Path, PathBuf};

use super::model::{
    FixtureLaunch, FixtureLaunchKind, SoftwareActionClass, SoftwareEligibilityReason,
    SoftwareIdentity,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LaunchRefusal {
    Malformed,
    #[cfg(test)]
    Manual,
    MsiUnsupported,
    MissingTool,
    OutsideFixture,
    ForbiddenArgument,
}

pub(crate) fn valid_product_code(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 38 || bytes[0] != b'{' || bytes[37] != b'}' {
        return false;
    }
    let hex_at = |index: usize| bytes[index].is_ascii_hexdigit();
    (1..9).all(hex_at)
        && bytes[9] == b'-'
        && (10..14).all(hex_at)
        && bytes[14] == b'-'
        && (15..19).all(hex_at)
        && bytes[19] == b'-'
        && (20..24).all(hex_at)
        && bytes[24] == b'-'
        && (25..37).all(hex_at)
}

pub(crate) fn valid_app_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

pub(crate) fn action_class(kind: FixtureLaunchKind) -> SoftwareActionClass {
    match kind {
        FixtureLaunchKind::Msi => SoftwareActionClass::RemoveMsiProduct,
        FixtureLaunchKind::Vendor => SoftwareActionClass::RemoveVendorUninstaller,
        FixtureLaunchKind::Scoop => SoftwareActionClass::RemoveScoopApp,
    }
}

pub(crate) fn eligibility_reason(kind: FixtureLaunchKind) -> SoftwareEligibilityReason {
    match kind {
        FixtureLaunchKind::Msi => SoftwareEligibilityReason::EligibleMsiQuiet,
        FixtureLaunchKind::Vendor => SoftwareEligibilityReason::EligibleVendorUninstaller,
        FixtureLaunchKind::Scoop => SoftwareEligibilityReason::EligibleScoopApp,
    }
}

pub(crate) fn launch_matches_identity(identity: &SoftwareIdentity, launch: &FixtureLaunch) -> bool {
    match (launch.kind, identity) {
        (FixtureLaunchKind::Msi, SoftwareIdentity::Msi { product_code, .. }) => {
            product_code == &launch.target_id
        }
        (
            FixtureLaunchKind::Vendor | FixtureLaunchKind::Scoop,
            SoftwareIdentity::Arp { subkey, .. },
        ) => subkey == &launch.target_id,
        _ => false,
    }
}

/// Accepts only the three fixture shapes. The file does not need to exist yet.
pub(crate) fn validate_fixture_launch(launch: &FixtureLaunch) -> Result<(), LaunchRefusal> {
    if !launch.root.is_absolute() || !launch.executable.is_absolute() {
        return Err(LaunchRefusal::OutsideFixture);
    }
    if !path_is_inside(&launch.root, &launch.executable) {
        return Err(LaunchRefusal::OutsideFixture);
    }
    if forbidden_executable(&launch.executable) || forbidden_arguments(&launch.arguments) {
        return Err(LaunchRefusal::ForbiddenArgument);
    }
    match launch.kind {
        FixtureLaunchKind::Msi => validate_msi(launch),
        FixtureLaunchKind::Vendor => validate_vendor(launch),
        FixtureLaunchKind::Scoop => validate_scoop(launch),
    }
}

/// Resolves a recognized fixture executable. A missing file does not create one.
pub(crate) fn prepare_fixture_start(launch: &FixtureLaunch) -> Result<PathBuf, LaunchRefusal> {
    validate_fixture_launch(launch)?;
    let path = normalize_lexical(&launch.executable);
    match path.symlink_metadata() {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(LaunchRefusal::OutsideFixture),
        Ok(metadata) if metadata.is_file() => Ok(path),
        _ => Err(LaunchRefusal::MissingTool),
    }
}

/// Decodes one registry uninstall value without a shell.
#[cfg(test)]
pub(crate) fn recognize_registry_uninstall(
    raw: &str,
    root: &Path,
    app_id: &str,
) -> Result<FixtureLaunch, LaunchRefusal> {
    let tokens = tokenize(raw).ok_or(LaunchRefusal::Malformed)?;
    if tokens.is_empty() {
        return Err(LaunchRefusal::Manual);
    }
    let program = &tokens[0];
    let arguments = &tokens[1..];
    if forbidden_token(program) || forbidden_arguments(arguments) {
        return Err(LaunchRefusal::ForbiddenArgument);
    }
    if is_msiexec_token(program) {
        return recognize_msi(program, arguments, root);
    }
    if is_scoop_token(program) {
        return recognize_scoop(program, arguments, root, app_id);
    }
    recognize_vendor(program, arguments, root, app_id)
}

fn validate_msi(launch: &FixtureLaunch) -> Result<(), LaunchRefusal> {
    if !file_name_is(&launch.executable, "msiexec.exe") || !valid_product_code(&launch.target_id) {
        return Err(LaunchRefusal::MsiUnsupported);
    }
    let expected = [
        "/x".to_string(),
        launch.target_id.clone(),
        "/qn".to_string(),
        "/norestart".to_string(),
    ];
    if launch.arguments != expected {
        return Err(LaunchRefusal::MsiUnsupported);
    }
    Ok(())
}

fn validate_vendor(launch: &FixtureLaunch) -> Result<(), LaunchRefusal> {
    if !valid_app_id(&launch.target_id) || script_extension(&launch.executable) {
        return Err(LaunchRefusal::Malformed);
    }
    Ok(())
}

fn validate_scoop(launch: &FixtureLaunch) -> Result<(), LaunchRefusal> {
    if !file_name_is(&launch.executable, "scoop.exe") || !valid_app_id(&launch.target_id) {
        return Err(LaunchRefusal::Malformed);
    }
    let expected = ["uninstall".to_string(), launch.target_id.clone()];
    if launch.arguments != expected {
        return Err(LaunchRefusal::Malformed);
    }
    Ok(())
}

#[cfg(test)]
fn recognize_msi(
    program: &str,
    arguments: &[String],
    root: &Path,
) -> Result<FixtureLaunch, LaunchRefusal> {
    let executable = resolve_named(program, root, "msiexec.exe")?;
    let target_id = arguments.get(1).cloned().unwrap_or_default();
    let launch = FixtureLaunch {
        kind: FixtureLaunchKind::Msi,
        root: root.to_path_buf(),
        executable,
        arguments: arguments.to_vec(),
        target_id,
    };
    validate_fixture_launch(&launch)?;
    Ok(launch)
}

#[cfg(test)]
fn recognize_scoop(
    program: &str,
    arguments: &[String],
    root: &Path,
    app_id: &str,
) -> Result<FixtureLaunch, LaunchRefusal> {
    if arguments != ["uninstall".to_string(), app_id.to_string()] {
        return Err(LaunchRefusal::Malformed);
    }
    let executable = resolve_named(program, root, "scoop.exe")?;
    let launch = FixtureLaunch {
        kind: FixtureLaunchKind::Scoop,
        root: root.to_path_buf(),
        executable,
        arguments: arguments.to_vec(),
        target_id: app_id.to_string(),
    };
    validate_fixture_launch(&launch)?;
    Ok(launch)
}

#[cfg(test)]
fn recognize_vendor(
    program: &str,
    arguments: &[String],
    root: &Path,
    app_id: &str,
) -> Result<FixtureLaunch, LaunchRefusal> {
    let path = Path::new(program);
    if !path.is_absolute() {
        return Err(LaunchRefusal::Manual);
    }
    if !path_is_inside(root, path) {
        return Err(LaunchRefusal::OutsideFixture);
    }
    let launch = FixtureLaunch {
        kind: FixtureLaunchKind::Vendor,
        root: root.to_path_buf(),
        executable: path.to_path_buf(),
        arguments: arguments.to_vec(),
        target_id: app_id.to_string(),
    };
    validate_fixture_launch(&launch)?;
    Ok(launch)
}

#[cfg(test)]
fn resolve_named(program: &str, root: &Path, file_name: &str) -> Result<PathBuf, LaunchRefusal> {
    let path = Path::new(program);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else if path.components().count() == 1 {
        root.join(path)
    } else {
        return Err(LaunchRefusal::Malformed);
    };
    if !file_name_is(&resolved, file_name) {
        return Err(LaunchRefusal::Malformed);
    }
    if !path_is_inside(root, &resolved) {
        return Err(LaunchRefusal::OutsideFixture);
    }
    Ok(resolved)
}

#[cfg(test)]
fn tokenize(raw: &str) -> Option<Vec<String>> {
    if raw.is_empty() || raw.len() > 4 * 1024 {
        return None;
    }
    if raw
        .bytes()
        .any(|byte| byte < 0x20 || matches!(byte, b'&' | b'|' | b'<' | b'>' | b'^' | b'%'))
    {
        return None;
    }
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for character in raw.chars() {
        match character {
            '"' => quoted = !quoted,
            ' ' | '\t' if !quoted => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(character),
        }
    }
    if quoted {
        return None;
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    if tokens.is_empty() {
        None
    } else {
        Some(tokens)
    }
}

fn forbidden_arguments(arguments: &[String]) -> bool {
    arguments.iter().any(|argument| forbidden_token(argument))
}

fn forbidden_token(token: &str) -> bool {
    const FORBIDDEN: &[&str] = &[
        "cmd",
        "cmd.exe",
        "/c",
        "powershell",
        "powershell.exe",
        "-command",
        "-enc",
    ];
    let lower = token.to_ascii_lowercase();
    FORBIDDEN.contains(&lower.as_str())
        || token.bytes().any(|byte| {
            matches!(
                byte,
                b'&' | b'|' | b'<' | b'>' | b'^' | b'%' | b'\n' | b'\r' | 0
            )
        })
}

fn forbidden_executable(path: &Path) -> bool {
    file_name_is(path, "cmd.exe")
        || file_name_is(path, "powershell.exe")
        || file_name_is(path, "cmd")
        || file_name_is(path, "powershell")
        || script_extension(path)
}

fn script_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "cmd" | "bat" | "ps1" | "vbs" | "js"
            )
        })
}

#[cfg(test)]
fn is_msiexec_token(token: &str) -> bool {
    file_name_is(Path::new(token), "msiexec.exe") || token.eq_ignore_ascii_case("msiexec.exe")
}

#[cfg(test)]
fn is_scoop_token(token: &str) -> bool {
    file_name_is(Path::new(token), "scoop.exe")
        || token.eq_ignore_ascii_case("scoop.exe")
        || token.eq_ignore_ascii_case("scoop")
}

fn file_name_is(path: &Path, expected: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(expected))
}

fn path_is_inside(root: &Path, target: &Path) -> bool {
    let root = normalize_lexical(root);
    let target = normalize_lexical(target);
    let root_text = trim_separator(&root.to_string_lossy().to_ascii_lowercase());
    let target_text = target.to_string_lossy().to_ascii_lowercase();
    let boundary = target_text.as_bytes().get(root_text.len()).copied();
    target_text.starts_with(&root_text) && matches!(boundary, Some(b'\\' | b'/'))
}

fn trim_separator(value: &str) -> String {
    value.trim_end_matches(['\\', '/']).to_string()
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir | Component::Normal(_) => out.push(component.as_os_str()),
        }
    }
    out
}
