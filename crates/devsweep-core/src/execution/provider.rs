//! Live, non-serialized provider authority used by preview and execution.
use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::process::{
    provider_file_digest as file_digest, resolve_provider_dispatch as resolve_dispatch,
};
use anyhow::{Context, Result, bail};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    filesystem::{normalize_absolute_path, paths_equal},
    model::{CleanAction, Scope},
    plan::ValidatedTarget,
    process::{
        CwdPolicy, DEFAULT_PROVIDER_PROBE_TIMEOUT, NoopCancelObserver, ProcessRequest,
        ProcessRunner, ProcessStatus,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct ProviderBinding {
    rule: String,
    footprint: String,
    pub(super) program: PathBuf,
    pub(super) prefix_args: Vec<String>,
    executable_sha256: String,
    support_files: Vec<(PathBuf, String)>,
    interpreter: Option<(PathBuf, String)>,
    capability_version: String,
    configuration_sha256: String,
}

pub(super) fn resolve(target: &ValidatedTarget) -> Result<Option<ProviderBinding>> {
    if !matches!(target.target().scope, Scope::Global) {
        return Ok(None);
    }
    let CleanAction::Command { program, .. } = &target.target().action else {
        return Ok(None);
    };
    let (path_args, version_args, config_keys): (&[&str], &[&str], &[&str]) = match target.rule_id()
    {
        "npm.cache.clean" => (
            &["config", "get", "cache"],
            &["--version"],
            &[
                "npm_config_cache",
                "NPM_CONFIG_CACHE",
                "NPM_CONFIG_USERCONFIG",
                "NPM_CONFIG_GLOBALCONFIG",
            ],
        ),
        "pip.cache.purge" => (
            &["-m", "pip", "cache", "dir"],
            &["-m", "pip", "--version"],
            &["PIP_CACHE_DIR", "PIP_CONFIG_FILE", "VIRTUAL_ENV"],
        ),
        "pnpm.store.prune" => (
            &["store", "path"],
            &["--version"],
            &["PNPM_HOME", "npm_config_store_dir", "NPM_CONFIG_USERCONFIG"],
        ),
        "yarn.cache.clean.classic" => (
            &["cache", "dir"],
            &["--version"],
            &["YARN_CACHE_FOLDER", "YARN_RC_FILENAME"],
        ),
        // `--mirror` acts on the global folder's cache, not cacheFolder.
        "yarn.cache.clean.modern" => (
            &["config", "get", "globalFolder"],
            &["--version"],
            &["YARN_GLOBAL_FOLDER", "YARN_RC_FILENAME"],
        ),
        "go.mod_cache.clean" => (
            &["env", "GOMODCACHE"],
            &["version"],
            &["GOMODCACHE", "GOPATH", "GOENV", "GOTOOLCHAIN"],
        ),
        "go.build_cache.clean" => (
            &["env", "GOCACHE"],
            &["version"],
            &["GOCACHE", "GOENV", "GOTOOLCHAIN"],
        ),
        "uv.cache.clean" => (
            &["cache", "dir"],
            &["--version"],
            &["UV_CACHE_DIR", "UV_CONFIG_FILE"],
        ),
        "bun.cache.clean" => (
            &["pm", "cache"],
            &["--version"],
            &["BUN_INSTALL_CACHE_DIR", "BUN_INSTALL"],
        ),
        _ => return Ok(None),
    };
    let observed = target
        .target()
        .path
        .as_ref()
        .context("provider observation lacks a cache footprint; rescan required")?;
    let resolved = crate::process::resolve_executable(program)
        .context("provider tool is unavailable; rescan required")?;
    let (executable, prefix_args, support_files) = resolve_dispatch(&resolved, target.rule_id())?;
    let executable_sha256 = file_digest(&executable)?;
    let raw_path = query(&executable, &prefix_args, path_args)?;
    let mut footprint = parse_path(&raw_path)?;
    if target.rule_id() == "yarn.cache.clean.modern" {
        footprint = footprint.join("cache");
    }
    if !paths_equal(observed, &footprint) {
        bail!("provider cache configuration changed; rescan required")
    }
    let version = query(&executable, &prefix_args, version_args)?;
    let version = version.trim();
    if version.is_empty() || version.len() > 512 || version.lines().count() != 1 {
        bail!("provider returned an unsupported version response")
    }
    crate::rules::validate_provider_version(target.rule_id(), version)?;
    let interpreter = if target.rule_id() == "pip.cache.purge" {
        let path = parse_path(&query(
            &executable,
            &prefix_args,
            &["-c", "import sys; print(sys.executable)"],
        )?)?
        .canonicalize()?;
        Some((path.clone(), file_digest(&path)?))
    } else {
        None
    };
    let effective_config = config_keys
        .iter()
        .map(|key| {
            (
                *key,
                env::var_os(key).map(|value| value.to_string_lossy().into_owned()),
            )
        })
        .collect::<Vec<_>>();
    // Only allowlisted effective cache configuration is hashed; no entire config
    // files, credentials, environment dumps or captured output are retained.
    let configuration_sha256 = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(
            normalize_absolute_path(&footprint)?,
            effective_config,
            relevant_config(target.rule_id())?
        ))?)
    );
    Ok(Some(ProviderBinding {
        rule: target.rule_id().to_string(),
        footprint: normalize_absolute_path(&footprint)?,
        program: executable,
        prefix_args,
        executable_sha256,
        support_files,
        interpreter,
        capability_version: version.to_string(),
        configuration_sha256,
    }))
}

fn relevant_config(rule: &str) -> Result<Vec<(String, Vec<String>)>> {
    if rule.starts_with("yarn.cache.clean.") {
        return crate::rules::yarn_configuration();
    }
    let home = super::resolve_home_dir();
    let env_path = |key: &str, relative: &str| {
        env::var_os(key)
            .map(PathBuf::from)
            .map(|path| path.join(relative))
    };
    let user_config = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|path| path.join(".config")));
    let (keys, paths): (&[&str], Vec<PathBuf>) = match rule {
        "npm.cache.clean" | "pnpm.store.prune" => (
            &["cache", "store-dir", "storeDir"],
            home.as_ref()
                .map(|home| home.join(".npmrc"))
                .into_iter()
                .chain(
                    [
                        "NPM_CONFIG_USERCONFIG",
                        "npm_config_userconfig",
                        "NPM_CONFIG_GLOBALCONFIG",
                        "npm_config_globalconfig",
                    ]
                    .iter()
                    .filter_map(env::var_os)
                    .map(PathBuf::from),
                )
                .collect(),
        ),
        "pip.cache.purge" => (
            &["cache-dir"],
            home.as_ref()
                .into_iter()
                .flat_map(|home| {
                    [
                        home.join("pip/pip.ini"),
                        home.join(".pip/pip.conf"),
                        home.join("Library/Application Support/pip/pip.conf"),
                    ]
                })
                .chain(user_config.as_ref().map(|path| path.join("pip/pip.conf")))
                .chain(env_path("APPDATA", "pip/pip.ini"))
                .chain(env_path("PROGRAMDATA", "pip/pip.ini"))
                .chain(env_path(
                    "VIRTUAL_ENV",
                    if cfg!(windows) { "pip.ini" } else { "pip.conf" },
                ))
                .chain((!cfg!(windows)).then(|| PathBuf::from("/etc/pip.conf")))
                .chain(env::var_os("PIP_CONFIG_FILE").map(PathBuf::from))
                .collect(),
        ),
        "go.mod_cache.clean" | "go.build_cache.clean" => (
            &["GOMODCACHE", "GOPATH", "GOCACHE", "GOTOOLCHAIN"],
            env::var_os("GOENV")
                .filter(|value| value != "off")
                .map(PathBuf::from)
                .into_iter()
                .chain(env_path("APPDATA", "go/env"))
                .chain(user_config.as_ref().map(|path| path.join("go/env")))
                .chain(
                    home.as_ref()
                        .map(|path| path.join("Library/Application Support/go/env")),
                )
                .collect(),
        ),
        "uv.cache.clean" => (
            &["cache-dir", "no-cache"],
            env::var_os("UV_CONFIG_FILE")
                .map(PathBuf::from)
                .into_iter()
                .chain(env_path("APPDATA", "uv/uv.toml"))
                .chain(env_path("PROGRAMDATA", "uv/uv.toml"))
                .chain(user_config.as_ref().map(|path| path.join("uv/uv.toml")))
                .chain((!cfg!(windows)).then(|| PathBuf::from("/etc/uv/uv.toml")))
                .collect(),
        ),
        "bun.cache.clean" => (
            &["dir", "disable", "disableManifest"],
            home.as_ref()
                .map(|path| path.join(".bunfig.toml"))
                .into_iter()
                .chain(user_config.as_ref().map(|path| path.join(".bunfig.toml")))
                .collect(),
        ),
        _ => (&[], Vec::new()),
    };
    read_relevant_config(keys, paths, rule == "bun.cache.clean")
}

fn read_relevant_config(
    keys: &[&str],
    paths: Vec<PathBuf>,
    bun_cache_section_only: bool,
) -> Result<Vec<(String, Vec<String>)>> {
    let mut values = Vec::new();
    for path in paths {
        if !path.is_file() {
            continue;
        }
        if path.metadata()?.len() > 1024 * 1024 {
            bail!("provider configuration exceeds read budget")
        }
        let contents =
            std::fs::read_to_string(&path).context("provider configuration cannot be read")?;
        let mut in_cache_section = !bun_cache_section_only;
        let lines = contents
            .lines()
            .filter(|line| {
                if bun_cache_section_only && line.trim().starts_with('[') {
                    in_cache_section = line.trim() == "[install.cache]";
                }
                let key = line
                    .trim_start()
                    .split(['=', ':', ' ', '\t'])
                    .next()
                    .unwrap_or_default();
                in_cache_section
                    && keys
                        .iter()
                        .any(|allowed| key.trim_matches(['\'', '"']).eq_ignore_ascii_case(allowed))
            })
            .map(str::to_string)
            .collect();
        values.push((path.display().to_string(), lines));
    }
    values.sort();
    values.dedup();
    Ok(values)
}

pub(super) fn digest(base: &str, bindings: &[(String, ProviderBinding)]) -> Result<String> {
    if bindings.is_empty() {
        return Ok(base.to_string());
    }
    let mut ordered = bindings.to_vec();
    ordered.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(
            "devsweep.provider-confirmation.v1",
            base,
            ordered
        ))?)
    ))
}

fn query(program: &Path, prefix_args: &[String], args: &[&str]) -> Result<String> {
    let cancel = NoopCancelObserver;
    let result = ProcessRunner::default().run(&ProcessRequest {
        program: OsString::from(program),
        args: prefix_args
            .iter()
            .map(OsString::from)
            .chain(args.iter().map(OsString::from))
            .collect(),
        cwd: CwdPolicy::Neutral,
        timeout: Some(DEFAULT_PROVIDER_PROBE_TIMEOUT),
        job_deadline: None,
        cancel: &cancel,
    });
    if result.status != ProcessStatus::Success
        || result.output.stdout_truncated
        || result.output.stderr_truncated
    {
        bail!("provider query failed or exceeded its output/time budget; rescan required")
    }
    String::from_utf8(result.output.stdout).context("provider query returned invalid UTF-8")
}

fn parse_path(output: &str) -> Result<PathBuf> {
    let mut lines = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let line = lines.next().context("provider path is empty")?;
    if lines.next().is_some() || line.chars().any(char::is_control) {
        bail!("provider path output is ambiguous")
    }
    let path = PathBuf::from(line);
    normalize_absolute_path(&path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_confirmation_serialization_has_a_stable_domain_and_digest() {
        let binding = ProviderBinding {
            rule: "go.mod_cache.clean".into(),
            footprint: "/cache/go".into(),
            program: PathBuf::from("/tools/go"),
            prefix_args: vec![],
            executable_sha256: "executable".into(),
            support_files: vec![],
            interpreter: None,
            capability_version: "go version go1.27.2 linux/amd64".into(),
            configuration_sha256: "configuration".into(),
        };
        assert_eq!(
            digest("manifest", &[("go-target".into(), binding)]).unwrap(),
            "61a9ad597294141c680a8b32d0911715b3c76b0c3e25ca81d749e5be517dcc56"
        );
        assert_eq!(digest("legacy-manifest", &[]).unwrap(), "legacy-manifest");
    }

    #[test]
    fn same_path_executable_replacement_changes_identity_and_confirmation() {
        let fixture = tempfile::TempDir::new().expect("fixture");
        let path = fixture.path().join("tool.exe");
        std::fs::write(&path, b"first").expect("fixture");
        let first = file_digest(&path).expect("hash");
        std::fs::write(&path, b"other").expect("replace same path and length");
        assert_ne!(first, file_digest(&path).expect("new hash"));
        let binding = ProviderBinding {
            rule: "go.mod_cache.clean".into(),
            footprint: "cache".into(),
            program: path,
            prefix_args: Vec::new(),
            executable_sha256: first,
            support_files: Vec::new(),
            interpreter: None,
            capability_version: "go1.27".into(),
            configuration_sha256: "first".into(),
        };
        let original = digest("base", &[("id".into(), binding.clone())]).expect("digest");
        let mut changed = binding;
        changed.configuration_sha256 = "second".into();
        assert_ne!(
            original,
            digest("base", &[("id".into(), changed)]).expect("changed digest")
        );
    }

    #[test]
    fn standard_node_shims_bind_native_node_and_fixed_cli_without_a_shell() {
        let fixture = tempfile::TempDir::new().expect("fixture");
        std::fs::copy(
            std::env::current_exe().expect("test binary"),
            fixture.path().join("node.exe"),
        )
        .expect("native fixture");
        for (name, rule, script) in [
            ("npm", "npm.cache.clean", "node_modules/npm/bin/npm-cli.js"),
            ("pnpm", "pnpm.store.prune", "node_modules/pnpm/bin/pnpm.cjs"),
            (
                "yarn",
                "yarn.cache.clean.classic",
                "node_modules/yarn/bin/yarn.js",
            ),
        ] {
            let shim = fixture.path().join(format!("{name}.cmd"));
            let entry = fixture.path().join(script);
            std::fs::create_dir_all(entry.parent().expect("parent")).expect("directory");
            std::fs::write(&shim, "this command must never run").expect("shim");
            std::fs::write(&entry, "console.log('fixture')").expect("script");
            let (program, prefix, identities) =
                resolve_dispatch(&shim, rule).expect("supported layout");
            assert!(program.ends_with("node.exe"));
            assert_eq!(prefix.len(), 1);
            assert_eq!(
                Path::new(&prefix[0]).canonicalize().expect("runtime entry"),
                entry.canonicalize().expect("entry")
            );
            assert_eq!(identities.len(), 2);
            assert_eq!(identities[1].0, entry.canonicalize().expect("bound entry"));
            let before = identities[1].1.clone();
            std::fs::write(&entry, "changed script").expect("replace script");
            assert_ne!(
                before,
                resolve_dispatch(&shim, rule).expect("fresh dispatch").2[1].1
            );
        }
        let unknown = fixture.path().join("custom.cmd");
        std::fs::write(&unknown, "unknown").expect("shim");
        assert!(resolve_dispatch(&unknown, "go.mod_cache.clean").is_err());
    }
}
