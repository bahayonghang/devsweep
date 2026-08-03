use std::path::Path;

use super::identity::normalize_path_for_compare;

/// Returns whether `target_path` contains the running executable.
///
/// Prefer the central [`crate::execution::SafetyPolicy`] for execution-time checks
/// (including fail-closed `current_exe` lookup). This helper remains for
/// lightweight containment queries.
#[allow(dead_code)]
pub(crate) fn target_contains_current_exe(target_path: Option<&Path>) -> bool {
    let Some(target_path) = target_path else {
        return false;
    };
    let Ok(current_exe) = std::env::current_exe() else {
        // Callers that need fail-closed behavior must use SafetyPolicy.
        return false;
    };
    path_contains_path(target_path, &current_exe)
}

pub(crate) fn path_contains_path(parent: &Path, child: &Path) -> bool {
    normalize_path_for_compare(child).starts_with(normalize_path_for_compare(parent))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn path_contains_path_handles_existing_and_missing_children() {
        let fixture = TempDir::new().expect("temp dir");
        let parent = fixture.path().join("target");
        let child_dir = parent.join("debug");
        let child = child_dir.join("devsweep.exe");
        fs::create_dir_all(&child_dir).expect("child dir");
        fs::write(&child, "exe").expect("child file");

        assert!(path_contains_path(&parent, &child));
        assert!(path_contains_path(&parent, &parent.join("missing.exe")));
        assert!(!path_contains_path(&child_dir, &parent));
    }
}
