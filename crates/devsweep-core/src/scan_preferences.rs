//! Narrow, desktop-only persistence for explicitly selected cleanup scan roots.

use std::{
    collections::BTreeSet,
    env, fs, io,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{
    desktop_preferences::{
        TransactionLock, atomic_replace, desktop_preferences_path, replace_file,
    },
    filesystem::{
        PathSafety, SystemPathReparseProbe, inspect_entry_no_follow, normalize_absolute_path,
    },
};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScanRootsDocument {
    schema_version: u8,
    roots: Vec<String>,
}

/// Resolve the fixed user settings file. No working-directory fallback exists.
pub fn scan_roots_path() -> Result<PathBuf> {
    let path = desktop_preferences_path()?.with_file_name("clean-scan-roots-v1.json");
    if !path.is_absolute() {
        bail!("scan-root settings require an absolute LOCALAPPDATA directory");
    }
    Ok(path)
}

/// Load the selected roots without requiring them to still exist.
pub fn load_scan_roots() -> Result<Vec<PathBuf>> {
    load_from_path(&scan_roots_path()?)
}

/// Validate and commit an explicit replacement under the shared settings lock.
pub fn save_scan_roots(roots: &[PathBuf]) -> Result<Vec<PathBuf>> {
    save_at_path(&scan_roots_path()?, roots)
}

/// Validate a session scope without reading or modifying preference storage.
pub fn validate_scan_roots(roots: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let roots = normalize_roots(roots)?;
    for root in &roots {
        for ancestor in root.ancestors().collect::<Vec<_>>().into_iter().rev() {
            let entry = inspect_entry_no_follow(ancestor, &SystemPathReparseProbe)
                .with_context(|| format!("cannot inspect scan root {}", ancestor.display()))?;
            if !entry.is_dir || entry.safety != PathSafety::Safe {
                bail!(
                    "scan root must be an existing directory without reparse points: {}",
                    root.display()
                );
            }
        }
    }
    Ok(roots)
}

/// Offer only existing conventional containers. This performs metadata checks,
/// never project discovery, recursive scanning, or implicit selection.
pub fn scan_root_suggestions() -> Vec<PathBuf> {
    let home = env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" });
    let Some(home) = home.filter(|value| !value.is_empty()).map(PathBuf::from) else {
        return Vec::new();
    };
    [
        "Projects",
        "Documents/Code",
        "source/repos",
        ".codex/worktrees",
    ]
    .into_iter()
    .filter_map(|suffix| validate_scan_roots(&[home.join(suffix)]).ok())
    .flatten()
    .collect()
}

fn normalize_roots(roots: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for root in roots {
        let normalized = normalize_absolute_path(root)?;
        if seen.insert(normalized.clone()) {
            result.push(PathBuf::from(normalized));
        }
    }
    Ok(result)
}

fn load_from_path(path: &Path) -> Result<Vec<PathBuf>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error).with_context(|| format!("cannot read {}", path.display())),
    };
    let document: ScanRootsDocument = serde_json::from_slice(&bytes)
        .with_context(|| format!("unsupported scan-root document at {}", path.display()))?;
    if document.schema_version != 1 {
        bail!(
            "unsupported scan-root schema version {} at {}",
            document.schema_version,
            path.display()
        );
    }
    normalize_roots(
        &document
            .roots
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>(),
    )
}

fn save_at_path(path: &Path, roots: &[PathBuf]) -> Result<Vec<PathBuf>> {
    save_with_replace(path, roots, replace_file)
}

fn save_with_replace(
    path: &Path,
    roots: &[PathBuf],
    replace: fn(&Path, &Path) -> io::Result<()>,
) -> Result<Vec<PathBuf>> {
    let roots = validate_scan_roots(roots)?;
    let directory = path
        .parent()
        .context("scan-root settings path has no parent")?;
    fs::create_dir_all(directory)?;
    let _lock = TransactionLock::acquire(directory)?;
    // Refuse to overwrite corrupt or future documents, even for a valid edit.
    load_from_path(path)?;
    let document = ScanRootsDocument {
        schema_version: 1,
        roots: roots
            .iter()
            .map(|root| root.to_string_lossy().into_owned())
            .collect(),
    };
    atomic_replace(path, &serde_json::to_vec(&document)?, replace)?;
    Ok(roots)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roots_are_explicit_normalized_deduplicated_and_nested_selections_survive() {
        let temp = crate::test_support::tempdir().unwrap();
        let project = temp.path().join("Project space 测试");
        let nested = project.join("nested");
        fs::create_dir_all(&nested).unwrap();
        let roots = validate_scan_roots(&[project.clone(), project.join("."), nested]).unwrap();
        assert_eq!(roots.len(), 2);
        assert!(validate_scan_roots(&[PathBuf::from(".")]).is_err());
        assert!(validate_scan_roots(&[temp.path().join("missing")]).is_err());
        assert!(validate_scan_roots(&[]).unwrap().is_empty());
    }

    #[test]
    fn reads_are_nonmutating_and_missing_saved_roots_remain_removable() {
        let temp = crate::test_support::tempdir().unwrap();
        let path = temp.path().join("settings/clean-scan-roots-v1.json");
        assert!(load_from_path(&path).unwrap().is_empty());
        assert!(!path.exists());
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();
        let saved = save_at_path(&path, std::slice::from_ref(&project)).unwrap();
        fs::remove_dir(&project).unwrap();
        assert_eq!(load_from_path(&path).unwrap(), saved);
        assert!(save_at_path(&path, &[]).unwrap().is_empty());
    }

    #[test]
    fn corrupt_future_and_unknown_fields_are_preserved_while_session_validation_works() {
        let temp = crate::test_support::tempdir().unwrap();
        let path = temp.path().join("clean-scan-roots-v1.json");
        for bytes in [
            b"broken".as_slice(),
            br#"{"schema_version":2,"roots":[]}"#,
            br#"{"schema_version":1,"roots":[],"extra":true}"#,
        ] {
            fs::write(&path, bytes).unwrap();
            assert!(load_from_path(&path).is_err());
            assert!(save_at_path(&path, &[]).is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes);
            assert!(validate_scan_roots(&[]).is_ok());
        }
    }

    #[test]
    fn failed_atomic_replace_preserves_previous_bytes_and_removes_temporary_file() {
        let temp = crate::test_support::tempdir().unwrap();
        let path = temp.path().join("clean-scan-roots-v1.json");
        let original = br#"{"schema_version":1,"roots":[]}"#;
        fs::write(&path, original).unwrap();
        let result = save_with_replace(&path, &[], |_, _| {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "controlled replacement failure",
            ))
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[test]
    fn concurrent_replacements_produce_one_whole_document_without_affecting_appearance() {
        let temp = crate::test_support::tempdir().unwrap();
        let path = temp.path().join("clean-scan-roots-v1.json");
        let appearance = temp.path().join("desktop-preferences-v2.json");
        fs::write(&appearance, b"appearance sentinel").unwrap();
        let paths = (0..4)
            .map(|index| {
                let root = temp.path().join(format!("project-{index}"));
                fs::create_dir(&root).unwrap();
                root
            })
            .collect::<Vec<_>>();
        std::thread::scope(|scope| {
            for root in &paths {
                let path = &path;
                scope.spawn(move || {
                    save_at_path(path, std::slice::from_ref(root)).unwrap();
                });
            }
        });
        let saved = load_from_path(&path).unwrap();
        assert_eq!(saved.len(), 1);
        assert!(normalize_roots(&paths).unwrap().contains(&saved[0]));
        assert_eq!(fs::read(&appearance).unwrap(), b"appearance sentinel");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_ancestry_is_rejected_without_following_into_a_selected_directory() {
        let temp = crate::test_support::tempdir().unwrap();
        let real = temp.path().join("real");
        fs::create_dir_all(real.join("child")).unwrap();
        let link = temp.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert!(validate_scan_roots(&[link.join("child")]).is_err());
    }
}
