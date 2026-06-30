use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use rayon::prelude::*;

pub fn estimate_tree(path: &Path) -> (u64, Option<SystemTime>) {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return (0, None);
    };
    if is_unsafe_link(&metadata) {
        return (0, metadata.modified().ok());
    }
    if metadata.is_file() {
        return (metadata.len(), metadata.modified().ok());
    }
    if !metadata.is_dir() {
        return (0, metadata.modified().ok());
    }

    let self_mtime = metadata.modified().ok();
    let Ok(entries) = fs::read_dir(path) else {
        return (0, self_mtime);
    };
    let children: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();

    let (bytes, latest) = children
        .par_iter()
        .map(|child| estimate_tree(child))
        .reduce(|| (0, None), combine_estimates);

    (bytes, max_mtime(latest, self_mtime))
}

pub(crate) fn is_unsafe_link(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink() || has_windows_reparse_point(metadata)
}

fn combine_estimates(
    left: (u64, Option<SystemTime>),
    right: (u64, Option<SystemTime>),
) -> (u64, Option<SystemTime>) {
    (left.0 + right.0, max_mtime(left.1, right.1))
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

        let (bytes, latest) = estimate_tree(&root);

        assert_eq!(bytes, 12);
        assert_eq!(
            latest,
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
    fn estimate_tree_returns_zero_for_missing_path() {
        let fixture = Fixture::new();

        assert_eq!(estimate_tree(&fixture.path("missing")), (0, None));
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

        assert_eq!(estimate_tree(&root), (0, max_mtime(root_mtime, link_mtime)));
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
        assert_eq!(estimate_tree(&root), (0, max_mtime(root_mtime, link_mtime)));
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

    fn serial_estimate_tree(path: &Path) -> (u64, Option<SystemTime>) {
        let Ok(metadata) = fs::symlink_metadata(path) else {
            return (0, None);
        };
        if is_unsafe_link(&metadata) {
            return (0, metadata.modified().ok());
        }
        if metadata.is_file() {
            return (metadata.len(), metadata.modified().ok());
        }
        if !metadata.is_dir() {
            return (0, metadata.modified().ok());
        }

        let mut bytes = 0;
        let mut latest = metadata.modified().ok();
        let Ok(entries) = fs::read_dir(path) else {
            return (bytes, latest);
        };

        for entry in entries.flatten() {
            let (entry_bytes, entry_modified) = serial_estimate_tree(&entry.path());
            bytes += entry_bytes;
            latest = max_mtime(latest, entry_modified);
        }

        (bytes, latest)
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
}
