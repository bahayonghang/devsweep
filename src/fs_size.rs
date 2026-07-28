use std::{fs, path::Path, time::SystemTime};

use rayon::prelude::*;

/// Size walk result that distinguishes a verified empty tree from a failed walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeEstimate {
    /// Lower bound of observed logical bytes. `None` means no trustworthy total.
    pub logical_bytes: Option<u64>,
    pub complete: bool,
    pub last_modified: Option<SystemTime>,
    pub warnings: Vec<String>,
}

impl SizeEstimate {
    pub fn trusted(bytes: u64, last_modified: Option<SystemTime>) -> Self {
        Self {
            logical_bytes: Some(bytes),
            complete: true,
            last_modified,
            warnings: Vec::new(),
        }
    }

    pub fn display_bytes(&self) -> u64 {
        self.logical_bytes.unwrap_or(0)
    }

    pub fn merge(mut self, other: Self) -> Self {
        let bytes = match (self.logical_bytes, other.logical_bytes) {
            (Some(left), Some(right)) => Some(left + right),
            (Some(left), None) => Some(left),
            (None, Some(right)) => Some(right),
            (None, None) => None,
        };
        self.logical_bytes = bytes;
        self.complete = self.complete && other.complete;
        self.last_modified = max_mtime(self.last_modified, other.last_modified);
        self.warnings.extend(other.warnings);
        self
    }
}

/// Default entry budget for a single size walk root.
pub const DEFAULT_SIZE_ENTRY_BUDGET: usize = 50_000;

pub fn estimate_tree(path: &Path) -> SizeEstimate {
    estimate_tree_with_budget(path, DEFAULT_SIZE_ENTRY_BUDGET)
}

pub fn estimate_tree_with_budget(path: &Path, entry_budget: usize) -> SizeEstimate {
    let mut remaining = entry_budget;
    estimate_tree_bounded(path, &mut remaining, 0, 64)
}

fn estimate_tree_bounded(
    path: &Path,
    remaining: &mut usize,
    depth: usize,
    max_depth: usize,
) -> SizeEstimate {
    if *remaining == 0 {
        return SizeEstimate {
            logical_bytes: None,
            complete: false,
            last_modified: None,
            warnings: vec![format!("size entry budget exhausted at {}", path.display())],
        };
    }
    *remaining = remaining.saturating_sub(1);

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return SizeEstimate {
                logical_bytes: None,
                complete: false,
                last_modified: None,
                warnings: vec![format!("failed to inspect {}: {error}", path.display())],
            };
        }
    };
    if is_unsafe_link(&metadata) {
        return SizeEstimate::trusted(0, metadata.modified().ok());
    }
    if metadata.is_file() {
        return SizeEstimate::trusted(metadata.len(), metadata.modified().ok());
    }
    if !metadata.is_dir() {
        return SizeEstimate::trusted(0, metadata.modified().ok());
    }
    if depth >= max_depth {
        return SizeEstimate {
            logical_bytes: Some(0),
            complete: false,
            last_modified: metadata.modified().ok(),
            warnings: vec![format!("max depth reached at {}", path.display())],
        };
    }

    let self_mtime = metadata.modified().ok();
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) => {
            return SizeEstimate {
                logical_bytes: None,
                complete: false,
                last_modified: self_mtime,
                warnings: vec![format!("failed to read {}: {error}", path.display())],
            };
        }
    };

    let mut children = Vec::new();
    let mut warnings = Vec::new();
    let mut entry_errors = false;
    let mut budget_hit = false;
    for entry in entries {
        if *remaining == 0 {
            budget_hit = true;
            warnings.push(format!(
                "size entry budget exhausted under {}",
                path.display()
            ));
            break;
        }
        match entry {
            Ok(entry) => children.push(entry.path()),
            Err(error) => {
                entry_errors = true;
                warnings.push(format!(
                    "failed to read directory entry under {}: {error}",
                    path.display()
                ));
            }
        }
    }

    // Top-level fan-out may use rayon; nested walks stay sequential to avoid
    // task explosion on deep trees.
    let child_estimate = if depth == 0 && children.len() > 1 {
        children
            .par_iter()
            .map(|child| {
                let mut local = (*remaining).min(DEFAULT_SIZE_ENTRY_BUDGET);
                estimate_tree_bounded(child, &mut local, depth + 1, max_depth)
            })
            .reduce(
                || SizeEstimate::trusted(0, None),
                |left, right| left.merge(right),
            )
    } else {
        let mut acc = SizeEstimate::trusted(0, None);
        for child in &children {
            acc = acc.merge(estimate_tree_bounded(
                child,
                remaining,
                depth + 1,
                max_depth,
            ));
        }
        acc
    };

    let mut estimate = child_estimate;
    estimate.last_modified = max_mtime(estimate.last_modified, self_mtime);
    estimate.warnings.extend(warnings);
    if entry_errors || budget_hit {
        estimate.complete = false;
    }
    estimate
}

pub(crate) fn is_unsafe_link(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink() || has_windows_reparse_point(metadata)
}

fn max_mtime(left: Option<SystemTime>, right: Option<SystemTime>) -> Option<SystemTime> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

#[cfg(windows)]
fn has_windows_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn has_windows_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::SystemTime,
    };

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn estimate_tree_counts_file_bytes_and_latest_mtime() {
        let fixture = Fixture::new();
        let root = fixture.path("root");
        let nested = fixture.path("root/nested");
        let first_file = fixture.file("root/a.txt", "abcd");
        let second_file = fixture.file("root/nested/b.txt", "123456");
        let third_file = fixture.file("root/nested/latest.txt", "xy");

        let estimate = estimate_tree(&root);

        assert_eq!(estimate.logical_bytes, Some(12));
        assert!(estimate.complete);
        assert_eq!(
            estimate.last_modified,
            latest_of([&root, &nested, &first_file, &second_file, &third_file])
        );
    }

    #[test]
    fn parallel_estimate_matches_serial_reference() {
        let fixture = Fixture::new();
        for dir in 0..8 {
            for file in 0..8 {
                fixture.file(&format!("root/dir-{dir}/file-{file}.txt"), "payload");
            }
        }
        fixture.file("root/empty/.keep", "");

        assert_eq!(
            estimate_tree(&fixture.path("root")),
            serial_estimate_tree(&fixture.path("root"))
        );
    }

    #[test]
    fn estimate_tree_marks_missing_path_unknown() {
        let fixture = Fixture::new();

        let estimate = estimate_tree(&fixture.path("missing"));
        assert_eq!(estimate.logical_bytes, None);
        assert!(!estimate.complete);
        assert!(!estimate.warnings.is_empty());
    }

    #[test]
    fn estimate_tree_marks_empty_directory_complete_zero() {
        let fixture = Fixture::new();
        fs::create_dir_all(fixture.path("empty")).expect("empty dir");

        let estimate = estimate_tree(&fixture.path("empty"));
        assert_eq!(estimate.logical_bytes, Some(0));
        assert!(estimate.complete);
        assert!(estimate.warnings.is_empty());
    }

    #[test]
    fn estimate_tree_does_not_follow_symlinked_directories() {
        let fixture = Fixture::new();
        fixture.file("outside/big.bin", "not counted");
        fs::create_dir_all(fixture.path("root")).expect("root directory");
        let link = fixture.path("root/link");

        if create_dir_symlink(&fixture.path("outside"), &link).is_err() {
            return;
        }

        let root = fixture.path("root");
        let link_mtime = fs::symlink_metadata(&link)
            .expect("link metadata")
            .modified()
            .ok();
        let root_mtime = fs::metadata(&root).and_then(|m| m.modified()).ok();

        let estimate = estimate_tree(&root);
        assert_eq!(estimate.logical_bytes, Some(0));
        assert!(estimate.complete);
        assert_eq!(estimate.last_modified, max_mtime(root_mtime, link_mtime));
    }

    #[cfg(windows)]
    #[test]
    fn estimate_tree_does_not_follow_windows_reparse_points() {
        let fixture = Fixture::new();
        fixture.file("outside/big.bin", "not counted");
        fs::create_dir_all(fixture.path("root")).expect("root directory");
        let link = fixture.path("root/reparse");

        if create_dir_symlink(&fixture.path("outside"), &link).is_err() {
            return;
        }

        let metadata = fs::symlink_metadata(&link).expect("link metadata");
        assert!(has_windows_reparse_point(&metadata));

        let root = fixture.path("root");
        let link_mtime = metadata.modified().ok();
        let root_mtime = fs::metadata(&root).and_then(|m| m.modified()).ok();
        let estimate = estimate_tree(&root);
        assert_eq!(estimate.logical_bytes, Some(0));
        assert!(estimate.complete);
        assert_eq!(estimate.last_modified, max_mtime(root_mtime, link_mtime));
    }

    #[cfg(windows)]
    #[test]
    fn estimate_tree_marks_denied_child_incomplete() {
        let fixture = Fixture::new();
        let root = fixture.path("root");
        let denied = fixture.path("root/denied");
        let visible = fixture.file("root/visible.txt", "hello");
        fs::create_dir_all(&denied).expect("denied dir");
        fs::write(denied.join("secret.bin"), "secret").expect("secret file");

        if deny_directory_read(&denied).is_err() {
            return;
        }
        assert!(
            fs::read_dir(&denied).is_err(),
            "fixture must actually deny read_dir"
        );

        let estimate = estimate_tree(&root);
        assert!(!estimate.complete, "denied child must mark incomplete");
        assert_eq!(
            estimate.logical_bytes,
            Some(5),
            "visible sibling bytes remain as lower bound"
        );
        assert!(
            estimate
                .warnings
                .iter()
                .any(|warning| warning.contains("denied")),
            "warning names the failed path: {:?}",
            estimate.warnings
        );
        let _ = visible;
        let _ = restore_directory_read(&denied);
    }

    #[cfg(unix)]
    #[test]
    fn estimate_tree_marks_denied_child_incomplete() {
        use std::os::unix::fs::PermissionsExt;

        let fixture = Fixture::new();
        let root = fixture.path("root");
        fixture.file("root/visible.txt", "hello");
        let denied = fixture.path("root/denied");
        fs::create_dir_all(&denied).expect("denied dir");
        fs::write(denied.join("secret.bin"), "secret").expect("secret file");
        fs::set_permissions(&denied, fs::Permissions::from_mode(0o000)).expect("chmod");

        assert!(
            fs::read_dir(&denied).is_err(),
            "fixture must actually deny read_dir"
        );

        let estimate = estimate_tree(&root);
        let _ = fs::set_permissions(&denied, fs::Permissions::from_mode(0o755));
        assert!(!estimate.complete);
        assert_eq!(estimate.logical_bytes, Some(5));
        assert!(!estimate.warnings.is_empty());
    }

    fn latest_of<'a>(paths: impl IntoIterator<Item = &'a PathBuf>) -> Option<SystemTime> {
        paths
            .into_iter()
            .filter_map(|path| {
                fs::metadata(path)
                    .and_then(|metadata| metadata.modified())
                    .ok()
            })
            .reduce(|left, right| left.max(right))
    }

    fn serial_estimate_tree(path: &Path) -> SizeEstimate {
        estimate_tree(path)
    }

    struct Fixture {
        temp: TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                temp: TempDir::new().expect("temp dir"),
            }
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.temp.path().join(relative)
        }

        fn file(&self, relative: &str, content: &str) -> PathBuf {
            let path = self.path(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("fixture directory");
            }
            fs::write(&path, content).expect("fixture file");
            path
        }
    }

    #[cfg(unix)]
    fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_dir(target, link)
    }

    #[cfg(windows)]
    fn deny_directory_read(path: &Path) -> std::io::Result<()> {
        use std::process::Command;
        let output = Command::new("icacls")
            .arg(path)
            .arg("/deny")
            .arg(format!("{}:(OI)(CI)(R,X)", current_user()?))
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(String::from_utf8_lossy(
                &output.stderr,
            )))
        }
    }

    #[cfg(windows)]
    fn restore_directory_read(path: &Path) -> std::io::Result<()> {
        use std::process::Command;
        let _ = Command::new("icacls")
            .arg(path)
            .arg("/remove:d")
            .arg(current_user()?)
            .status();
        Ok(())
    }

    #[cfg(windows)]
    fn current_user() -> std::io::Result<String> {
        std::env::var("USERNAME").map_err(|_| std::io::Error::other("USERNAME missing"))
    }
}
