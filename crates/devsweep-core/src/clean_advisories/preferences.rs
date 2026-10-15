use super::{CleanAdvisoryPreferencesV1, GIB, clean_advisory_preferences};
use crate::desktop_preferences::{
    TransactionLock, atomic_replace, desktop_preferences_path, replace_file,
};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema_version: u8,
    threshold_bytes: u64,
}

fn path() -> Result<PathBuf> {
    let path = desktop_preferences_path()?.with_file_name("clean-advisories-v1.json");
    if !path.is_absolute() {
        bail!("clean advisory settings require an absolute settings directory");
    }
    Ok(path)
}

/// Missing settings use the default without creating any file.
pub fn load_clean_advisory_preferences() -> Result<CleanAdvisoryPreferencesV1> {
    load(&path()?)
}

/// Commit a whole-GiB threshold without touching roots or appearance settings.
pub fn save_clean_advisory_preferences(threshold_gib: u64) -> Result<CleanAdvisoryPreferencesV1> {
    save(&path()?, threshold_gib, replace_file)
}

fn load(path: &Path) -> Result<CleanAdvisoryPreferencesV1> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Default::default()),
        Err(error) => return Err(error).context("cannot read clean advisory preferences"),
    };
    let document: Document = serde_json::from_slice(&bytes)
        .context("invalid clean advisory preferences; original bytes preserved")?;
    if document.schema_version != 1 || !document.threshold_bytes.is_multiple_of(GIB) {
        bail!("unsupported clean advisory preferences; original bytes preserved");
    }
    clean_advisory_preferences(document.threshold_bytes / GIB)
}

fn save(
    path: &Path,
    gib: u64,
    replace: fn(&Path, &Path) -> io::Result<()>,
) -> Result<CleanAdvisoryPreferencesV1> {
    let preferences = clean_advisory_preferences(gib)?;
    let directory = path
        .parent()
        .context("clean advisory settings path has no parent")?;
    fs::create_dir_all(directory)?;
    let _lock = TransactionLock::acquire(directory)?;
    load(path)?; // Never overwrite corrupt, future or unknown data.
    let document = Document {
        schema_version: 1,
        threshold_bytes: preferences.threshold_bytes,
    };
    atomic_replace(path, &serde_json::to_vec(&document)?, replace)?;
    Ok(preferences)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_defaults_are_read_only_and_whole_gib_bounds_are_checked() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings/clean-advisories-v1.json");
        assert_eq!(load(&path).unwrap(), CleanAdvisoryPreferencesV1::default());
        assert!(!path.parent().unwrap().exists());
        for gib in [0, super::super::MAX_THRESHOLD_GIB + 1, u64::MAX] {
            assert!(save(&path, gib, replace_file).is_err());
            assert!(!path.parent().unwrap().exists());
        }
        assert!(clean_advisory_preferences(super::super::MAX_THRESHOLD_GIB).is_ok());
        assert_eq!(
            save(&path, 7, replace_file).unwrap().threshold_bytes,
            7 * GIB
        );
        assert_eq!(load(&path).unwrap().threshold_bytes, 7 * GIB);
    }
    #[test]
    fn corrupt_future_unknown_and_invalid_threshold_bytes_survive_writes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("clean-advisories-v1.json");
        for bytes in [
            "broken",
            r#"{"schema_version":2,"threshold_bytes":5368709120}"#,
            r#"{"schema_version":1,"threshold_bytes":5368709120,"unknown":true}"#,
            r#"{"schema_version":1,"threshold_bytes":0}"#,
            r#"{"schema_version":1,"threshold_bytes":1073741825}"#,
            r#"{"schema_version":1,"threshold_bytes":18446744073709551615}"#,
        ] {
            fs::write(&path, bytes).unwrap();
            assert!(load(&path).is_err());
            assert!(save(&path, 6, replace_file).is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), bytes);
            assert_eq!(
                clean_advisory_preferences(6).unwrap().threshold_bytes,
                6 * GIB
            );
        }
    }
    #[test]
    fn failed_replace_preserves_committed_value_and_removes_temporary_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("clean-advisories-v1.json");
        save(&path, 5, replace_file).unwrap();
        let original = fs::read(&path).unwrap();
        assert!(
            save(&path, 6, |_, _| Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "fixture"
            )))
            .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }
    #[test]
    fn concurrent_commits_keep_one_whole_value_and_other_stores_unchanged() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("clean-advisories-v1.json");
        let roots = temp.path().join("clean-scan-roots-v1.json");
        let appearance = temp.path().join("desktop-preferences-v2.json");
        fs::write(&roots, "roots sentinel").unwrap();
        fs::write(&appearance, "appearance sentinel").unwrap();
        std::thread::scope(|scope| {
            for gib in 1..=4 {
                let path = &path;
                scope.spawn(move || {
                    save(path, gib, replace_file).unwrap();
                });
            }
        });
        assert!((1..=4).contains(&(load(&path).unwrap().threshold_bytes / GIB)));
        assert_eq!(fs::read_to_string(roots).unwrap(), "roots sentinel");
        assert_eq!(
            fs::read_to_string(appearance).unwrap(),
            "appearance sentinel"
        );
    }
}
