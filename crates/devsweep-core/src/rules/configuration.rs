use anyhow::{Context, bail};
use std::path::{Path, PathBuf};

use super::{
    GO_MODCACHE_DIRECTORY_RULE_ID, GRADLE_CACHES_RULE_ID, GRADLE_WRAPPER_DISTS_RULE_ID,
    GlobalCacheRule, NUGET_PACKAGES_RULE_ID,
};

/// One closed configuration policy shared by discovery, registry and live safety.
/// Serialized paths are never configuration input to this resolver.
pub(crate) fn configured_cache_path(
    rule: &GlobalCacheRule,
    home: &Path,
    env_path: impl Fn(&str) -> Option<PathBuf>,
) -> PathBuf {
    match rule.id {
        GRADLE_CACHES_RULE_ID | GRADLE_WRAPPER_DISTS_RULE_ID => env_path("GRADLE_USER_HOME")
            .map(|base| base.join(rule.relative.trim_start_matches(".gradle/"))),
        NUGET_PACKAGES_RULE_ID => env_path("NUGET_PACKAGES"),
        GO_MODCACHE_DIRECTORY_RULE_ID => env_path("GOMODCACHE"),
        _ => None,
    }
    .unwrap_or_else(|| home.join(rule.relative))
}

/// Yarn may delegate to a different script through rc files. Until that layout
/// has a closed resolver, refuse delegation before discovery or live queries.
pub(crate) fn yarn_configuration() -> anyhow::Result<Vec<(String, Vec<String>)>> {
    if std::env::vars_os().any(|(key, _)| {
        key.to_str()
            .is_some_and(|key| key.replace('_', "").eq_ignore_ascii_case("YARNYARNPATH"))
    }) {
        bail!("Yarn yarnPath environment delegation is unsupported")
    }
    let mut configured_names = std::env::vars_os()
        .filter(|(key, _)| {
            key.to_str()
                .is_some_and(|key| key.eq_ignore_ascii_case("YARN_RC_FILENAME"))
        })
        .map(|(_, value)| {
            value
                .into_string()
                .map_err(|_| anyhow::anyhow!("unsupported Yarn rc filename encoding"))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    configured_names.sort();
    configured_names.dedup();
    if configured_names.len() > 1 {
        bail!("conflicting Yarn rc filename environment values")
    }
    let custom = configured_names.pop();
    if custom.as_ref().is_some_and(|name| {
        name.is_empty() || name.contains(['/', '\\']) || matches!(name.as_str(), "." | "..")
    }) {
        bail!("unsupported Yarn rc filename")
    }
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let temporary = std::env::temp_dir();
    let mut paths = home
        .iter()
        .map(PathBuf::as_path)
        .chain(temporary.ancestors())
        .flat_map(|directory| {
            [Some(".yarnrc"), Some(".yarnrc.yml"), custom.as_deref()]
                .into_iter()
                .flatten()
                .map(move |name| directory.join(name))
        })
        .collect::<Vec<_>>();
    // Yarn Classic 1.22.22 util/rc.js reads these in addition to cwd/home rc
    // files. Inspect every source before invoking Yarn: an unseen yarn-path
    // would otherwise delegate even a read-only version/cache query.
    if let Some(home) = &home {
        paths.extend([
            home.join(".config/yarn/config"),
            home.join(".config/yarn"),
            home.join(".yarn/config"),
        ]);
        #[cfg(windows)]
        let config_directory = std::env::var_os("LOCALAPPDATA")
            .filter(|value| !value.is_empty())
            .map(|base| PathBuf::from(base).join("Yarn/Config"))
            .unwrap_or_else(|| home.join(".config/yarn"));
        #[cfg(unix)]
        let config_directory = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|value| !value.is_empty())
            .map(|base| PathBuf::from(base).join("yarn"))
            .unwrap_or_else(|| {
                // SAFETY: getuid has no preconditions and only reads process identity.
                if unsafe { libc::getuid() } == 0
                    && std::env::var_os("FAKEROOTKEY").is_none_or(|value| value.is_empty())
                {
                    PathBuf::from("/usr/local/share/.config/yarn")
                } else {
                    home.join(".config/yarn")
                }
            });
        paths.push(config_directory);
    }
    #[cfg(unix)]
    paths.extend([
        PathBuf::from("/etc/yarn/config"),
        PathBuf::from("/etc/yarnrc"),
    ]);
    if let Some(config) = std::env::var_os("YARN_CONFIG").filter(|value| !value.is_empty()) {
        paths.push(PathBuf::from(config));
    }
    if paths.iter().any(|path| !path.is_absolute()) {
        bail!("unsupported relative Yarn configuration path")
    }
    paths.sort();
    paths.dedup();
    read_yarn_configuration(&paths)
}

fn read_yarn_configuration(paths: &[PathBuf]) -> anyhow::Result<Vec<(String, Vec<String>)>> {
    let mut values = Vec::new();
    for path in paths {
        let metadata = match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_dir() => continue,
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error).context("Yarn configuration cannot be inspected"),
        };
        if !metadata.is_file() {
            bail!("unsupported Yarn configuration file kind")
        }
        if metadata.len() > 1024 * 1024 {
            bail!("Yarn configuration exceeds read budget")
        }
        let contents =
            std::fs::read_to_string(path).context("Yarn configuration cannot be read")?;
        let contents = contents.trim_start_matches('\u{feff}');
        let mut relevant = Vec::new();
        for line in contents.lines() {
            let trimmed = line.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "---" {
                continue;
            }
            if trimmed.starts_with("---") || trimmed.starts_with("...") || trimmed.starts_with('%')
            {
                bail!("unsupported Yarn YAML document directive or end marker")
            }
            // Inspect scalar keys at every indentation: YAML document markers
            // must never conceal delegation or a relevant configuration line.
            let key = trimmed
                .split(['=', ':', ' ', '\t'])
                .next()
                .unwrap_or_default()
                .trim_matches(['\'', '"']);
            if key.starts_with("--") {
                bail!("Yarn rc command-line arguments are unsupported; use direct cache settings")
            }
            if key.contains('\\')
                || key.starts_with(['{', '[', '?', '&', '*', '!', '%'])
                || key == "<<"
            {
                bail!("unsupported complex Yarn configuration; explicit review required")
            }
            if matches!(key, "yarnPath" | "yarn-path") {
                bail!(
                    "Yarn yarnPath/yarn-path delegation is unsupported; use a directly supported provider"
                )
            }
            if matches!(key, "cache-folder" | "cacheFolder" | "globalFolder") {
                relevant.push(line.to_string());
            }
        }
        values.push((path.display().to_string(), relevant));
    }
    Ok(values)
}

pub(crate) fn validate_provider_version(rule: &str, version: &str) -> anyhow::Result<()> {
    if version.is_empty() || version.len() > 512 || version.chars().any(char::is_control) {
        bail!("unsupported provider version output")
    }
    let number = match rule {
        "pip.cache.purge" => version
            .strip_prefix("pip ")
            .and_then(|text| text.split_once(" from "))
            .filter(|(_, path)| !path.is_empty())
            .map(|(number, _)| number),
        "go.mod_cache.clean" | "go.build_cache.clean" => version
            .strip_prefix("go version go")
            .and_then(|text| text.split_once(' '))
            .filter(|(_, platform)| {
                platform.split_once('/').is_some_and(|(os, arch)| {
                    !os.is_empty()
                        && !arch.is_empty()
                        && os
                            .bytes()
                            .chain(arch.bytes())
                            .all(|byte| byte.is_ascii_alphanumeric())
                })
            })
            .map(|(number, _)| number),
        "uv.cache.clean" => version.strip_prefix("uv ").and_then(|text| {
            if let Some((number, build)) = text.split_once(' ') {
                let build = build.strip_prefix('(')?.strip_suffix(')')?;
                let (hash, date) = build.split_once(' ')?;
                if !(7..=40).contains(&hash.len())
                    || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())
                    || date.len() != 10
                    || !date.bytes().enumerate().all(|(index, byte)| {
                        if index == 4 || index == 7 {
                            byte == b'-'
                        } else {
                            byte.is_ascii_digit()
                        }
                    })
                {
                    return None;
                }
                Some(number)
            } else {
                Some(text)
            }
        }),
        _ => Some(version),
    }
    .context("unsupported provider version format")?;
    let components: Vec<_> = number.split('.').collect();
    let accepts_two = rule == "pip.cache.purge" || rule.starts_with("go.");
    if !(components.len() == 3 || accepts_two && components.len() == 2)
        || components.iter().any(|part| {
            part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || part.len() > 1 && part.starts_with('0')
        })
    {
        bail!("unsupported or prerelease provider version")
    }
    let mut parts = number.split('.');
    let major = parts
        .next()
        .and_then(|part| part.parse::<u64>().ok())
        .context("unsupported provider major version")?;
    let minor = parts
        .next()
        .and_then(|part| part.parse::<u64>().ok())
        .context("unsupported provider minor version")?;
    let patch = parts
        .next()
        .map(str::parse::<u64>)
        .transpose()
        .context("unsupported provider patch version")?
        .unwrap_or(0);
    let supported = match rule {
        "npm.cache.clean" => (6..=11).contains(&major),
        "pnpm.store.prune" => (8..=10).contains(&major),
        "pip.cache.purge" => (21..=26).contains(&major) || major == 20 && minor >= 1,
        "yarn.cache.clean.classic" => major == 1,
        "yarn.cache.clean.modern" => (2..=4).contains(&major),
        "go.mod_cache.clean" | "go.build_cache.clean" => major == 1 && (15..=27).contains(&minor),
        "uv.cache.clean" => major == 0 && (4..=11).contains(&minor),
        "bun.cache.clean" => major == 1 && minor <= 3 && (minor > 0 || patch >= 3),
        _ => false,
    };
    if !supported {
        bail!(
            "provider version is outside the reviewed capability range; rescan with a supported tool"
        )
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn yarn_rc_scoped_keys_and_delegation_are_explicit() {
        let fixture = tempfile::TempDir::new().unwrap();
        let config = fixture.path().join("custom.yml");
        std::fs::write(&config, "globalFolder: before\nnpmAuthToken: first\n").unwrap();
        let before = super::read_yarn_configuration(std::slice::from_ref(&config)).unwrap();
        std::fs::write(&config, "globalFolder: before\nnpmAuthToken: second\n").unwrap();
        assert_eq!(
            before,
            super::read_yarn_configuration(std::slice::from_ref(&config)).unwrap()
        );
        std::fs::write(&config, "globalFolder: after\n").unwrap();
        assert_ne!(
            before,
            super::read_yarn_configuration(std::slice::from_ref(&config)).unwrap()
        );
        for content in [
            "yarnPath: custom.cjs",
            "  yarnPath: custom.cjs\n",
            "  yarn-path \"custom.js\"\n",
            "  yarnPath: custom.cjs\n...\n",
            "  yarnPath: custom.cjs\n... # end\n",
            "--- # start\n  yarnPath: custom.cjs\n",
            "--- {yarnPath: custom.cjs}\n",
            "%YAML 1.2\n---\n  yarnPath: custom.cjs\n",
            "yarn-path \"custom.js\"",
            "\"yarnPath\": custom.cjs",
            "{yarnPath: custom.cjs}",
            "<<: *delegate",
            "\"yarn\\u0050ath\": custom.cjs",
        ] {
            std::fs::write(&config, content).unwrap();
            assert!(
                super::read_yarn_configuration(std::slice::from_ref(&config)).is_err(),
                "{content}"
            );
        }
    }

    use super::*;

    #[test]
    fn provider_capabilities_reject_unreviewed_and_malformed_versions() {
        for (rule, accepted, rejected) in [
            ("npm.cache.clean", "11.0.0", "12.0.0"),
            ("pnpm.store.prune", "10.0.0", "11.0.0"),
            (
                "pip.cache.purge",
                "pip 26.2.1 from fixture",
                "pip 20.0 from fixture",
            ),
            (
                "go.mod_cache.clean",
                "go version go1.27.0 windows/amd64",
                "go version go2.0.0 windows/amd64",
            ),
            ("uv.cache.clean", "uv 0.11.0", "uv 1.0.0"),
            ("bun.cache.clean", "1.3.0", "2.0.0"),
            ("yarn.cache.clean.classic", "1.22.22", "4.0.0"),
            ("yarn.cache.clean.modern", "4.0.0", "1.22.22"),
        ] {
            assert!(validate_provider_version(rule, accepted).is_ok(), "{rule}");
            assert!(validate_provider_version(rule, rejected).is_err(), "{rule}");
            assert!(
                validate_provider_version(rule, "unknown-tool").is_err(),
                "{rule}"
            );
        }
        for (rule, version) in [
            ("npm.cache.clean", "11.0.not-a-version"),
            ("npm.cache.clean", "11.0.0 arbitrary"),
            ("npm.cache.clean", "11.0.0-beta.1"),
            ("npm.cache.clean", "011.0.0"),
            ("pip.cache.purge", "pip 26.3.dev0 from fixture"),
            ("pip.cache.purge", "pip 26.2.1"),
            ("go.mod_cache.clean", "go version go1.27rc1 windows/amd64"),
            (
                "go.mod_cache.clean",
                "go version go1.27.0 windows/amd64 extra",
            ),
            ("uv.cache.clean", "uv 0.11.0 arbitrary"),
            ("uv.cache.clean", "uv 0.11.0-rc.1"),
            ("bun.cache.clean", "1.3.0+unknown"),
        ] {
            assert!(
                validate_provider_version(rule, version).is_err(),
                "{rule}: {version}"
            );
        }
        assert!(
            validate_provider_version("uv.cache.clean", "uv 0.11.0 (abcdef0 2026-04-01)").is_ok()
        );
        assert!(validate_provider_version("bun.cache.clean", "1.0.3").is_ok());
        assert!(validate_provider_version("bun.cache.clean", "1.0.2").is_err());
        assert!(validate_provider_version("bun.cache.clean", "1.4.0").is_err());
    }

    #[test]
    fn overrides_use_the_same_closed_policy_and_reject_relative_roots() {
        let fixture = tempfile::TempDir::new().expect("fixture");
        let custom = fixture.path().join("cache warning 空格");
        for rule in super::super::global_cache_rules().filter(|rule| {
            matches!(
                rule.id,
                GRADLE_CACHES_RULE_ID
                    | GRADLE_WRAPPER_DISTS_RULE_ID
                    | NUGET_PACKAGES_RULE_ID
                    | GO_MODCACHE_DIRECTORY_RULE_ID
            )
        }) {
            let expected = if rule.id.starts_with("gradle.") {
                custom.join(rule.relative.trim_start_matches(".gradle/"))
            } else {
                custom.clone()
            };
            assert_eq!(
                configured_cache_path(rule, fixture.path(), |_| Some(custom.clone())),
                expected
            );
            assert!(
                crate::filesystem::normalize_absolute_path(&configured_cache_path(
                    rule,
                    fixture.path(),
                    |_| Some(PathBuf::from("relative"))
                ))
                .is_err()
            );
        }
    }
}
