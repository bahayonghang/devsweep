//! Shared helpers for unit-test fixtures.

/// Creates a temporary directory whose path has no symlinked ancestor.
///
/// The no-follow safety checks reject any root reached through a symlink.
/// macOS links `/var` to `/private/var`, so a plain `tempfile::tempdir()`
/// there yields a root those checks refuse. On Unix the system temp directory
/// is canonicalized first. Windows is unchanged, so tests never see verbatim
/// `\\?\` paths.
pub(crate) fn tempdir() -> std::io::Result<tempfile::TempDir> {
    #[cfg(unix)]
    {
        tempfile::tempdir_in(std::env::temp_dir().canonicalize()?)
    }
    #[cfg(not(unix))]
    {
        tempfile::tempdir()
    }
}
