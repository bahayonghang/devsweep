use std::path::Path;

use anyhow::{Result, bail};

/// Produces a lexical, platform-aware identity for an absolute path without
/// touching the filesystem. This is intentionally not a TOCTOU defense; the
/// execution-time safety policy owns live-object revalidation.
pub(crate) fn normalize_absolute_path(path: &Path) -> Result<String> {
    if !path.is_absolute() {
        bail!("path must be absolute: {}", path.display());
    }

    let raw = path.to_string_lossy();
    #[cfg(windows)]
    let raw = normalize_windows_path(raw.replace('\\', "/"));
    #[cfg(not(windows))]
    let raw = {
        // POSIX treats repeated leading separators as the same root. Keep the
        // Windows UNC branch above separate, where `//server/share` matters.
        let raw = raw.into_owned();
        format!("/{}", raw.trim_start_matches('/'))
    };

    let (prefix, rest) = split_root(&raw)?;
    let mut segments = Vec::new();
    for segment in rest.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if segments.pop().is_none() {
                    bail!("path escapes its absolute root: {}", path.display());
                }
            }
            value => segments.push(value),
        }
    }

    let joined = segments.join("/");
    if joined.is_empty() {
        Ok(prefix)
    } else if prefix.ends_with('/') {
        Ok(format!("{prefix}{joined}"))
    } else {
        Ok(format!("{prefix}/{joined}"))
    }
}

fn split_root(value: &str) -> Result<(String, &str)> {
    if let Some(rest) = value.strip_prefix("//") {
        return Ok(("//".to_string(), rest));
    }

    let bytes = value.as_bytes();
    if bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'/' {
        return Ok((value[..2].to_string(), &value[3..]));
    }

    if let Some(rest) = value.strip_prefix('/') {
        return Ok(("/".to_string(), rest));
    }

    bail!("path must include an absolute root: {value}")
}

#[cfg(windows)]
fn normalize_windows_path(value: String) -> String {
    let value = value.to_ascii_lowercase();
    if let Some(rest) = value.strip_prefix("//?/unc/") {
        format!("//{rest}")
    } else if let Some(rest) = value.strip_prefix("//?/") {
        rest.to_string()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::normalize_absolute_path;

    #[test]
    fn normalizes_lexical_dot_segments_without_filesystem_access() {
        #[cfg(windows)]
        let (path, expected) = (
            r"C:\workspace\.\app\cache\..\target",
            "c:/workspace/app/target",
        );
        #[cfg(not(windows))]
        let (path, expected) = ("/workspace/./app/cache/../target", "/workspace/app/target");

        assert_eq!(
            normalize_absolute_path(Path::new(path)).expect("absolute path normalizes"),
            expected
        );
    }

    #[test]
    fn rejects_relative_paths_and_root_escapes() {
        assert!(normalize_absolute_path(Path::new("relative/cache")).is_err());
        assert!(normalize_absolute_path(Path::new("/../escape")).is_err());
    }

    #[cfg(not(windows))]
    #[test]
    fn keeps_backslashes_as_unix_path_characters() {
        assert_eq!(
            normalize_absolute_path(Path::new(r"/workspace/cache\entry"))
                .expect("absolute path normalizes"),
            r"/workspace/cache\entry"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn collapses_equivalent_leading_posix_separators() {
        assert_eq!(
            normalize_absolute_path(Path::new("//workspace/cache/"))
                .expect("absolute path normalizes"),
            normalize_absolute_path(Path::new("/workspace/cache"))
                .expect("absolute path normalizes")
        );
    }

    #[cfg(windows)]
    #[test]
    fn normalizes_windows_case_separators_and_verbatim_prefixes() {
        let canonical = normalize_absolute_path(Path::new(r"C:\Work\Cache\")).expect("path");
        assert_eq!(canonical, "c:/work/cache");
        assert_eq!(
            normalize_absolute_path(Path::new(r"\\?\C:\WORK\cache")).expect("path"),
            canonical
        );
    }
}
