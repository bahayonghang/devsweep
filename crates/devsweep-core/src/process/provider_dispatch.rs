//! Closed provider dispatch shared by discovery and execution. No shell parsing.
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

pub(crate) type ProviderDispatch = (PathBuf, Vec<String>, Vec<(PathBuf, String)>);

pub(crate) fn resolve_provider_dispatch(resolved: &Path, rule: &str) -> Result<ProviderDispatch> {
    if rule.starts_with("yarn.cache.clean.") {
        crate::rules::yarn_configuration()?;
    }
    let executable = resolved
        .canonicalize()
        .context("provider executable cannot be resolved")?;
    let extension = executable
        .extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if matches!(extension.as_str(), "cmd" | "bat") {
        let relative = match rule {
            "npm.cache.clean" => "node_modules/npm/bin/npm-cli.js",
            "pnpm.store.prune" => "node_modules/pnpm/bin/pnpm.cjs",
            "yarn.cache.clean.classic" | "yarn.cache.clean.modern" => {
                "node_modules/yarn/bin/yarn.js"
            }
            _ => bail!("unsupported provider shim layout; install a supported native tool"),
        };
        let directory = executable
            .parent()
            .context("provider shim has no installation directory")?;
        let script = directory.join(relative).canonicalize().context("unsupported shim layout: trusted package CLI entrypoint missing (Corepack and custom shims are not supported)")?;
        let adjacent_node = directory.join("node.exe");
        let node = if adjacent_node.is_file() {
            adjacent_node
        } else {
            super::resolve_executable("node").context("native Node interpreter is unavailable")?
        }
        .canonicalize()?;
        if node
            .extension()
            .is_none_or(|extension| !extension.eq_ignore_ascii_case("exe"))
            || !native_binary(&node)?
        {
            bail!("Node interpreter must be a native executable")
        }
        let support = vec![
            (executable.clone(), provider_file_digest(&executable)?),
            (script.clone(), provider_file_digest(&script)?),
        ];
        return Ok((node, vec![node_script_argument(&script)?], support));
    }
    if matches!(extension.as_str(), "ps1" | "sh") {
        bail!("unsupported provider interpreter layout")
    }
    if matches!(
        rule,
        "npm.cache.clean"
            | "pnpm.store.prune"
            | "yarn.cache.clean.classic"
            | "yarn.cache.clean.modern"
    ) && !native_binary(&executable)?
    {
        let node =
            super::resolve_executable("node").context("native Node interpreter is unavailable")?;
        return resolve_node_script(&executable, rule, &node);
    }
    Ok((executable, Vec::new(), Vec::new()))
}

fn native_binary(path: &Path) -> Result<bool> {
    let mut magic = [0; 4];
    if File::open(path)?.read(&mut magic)? < 4 {
        return Ok(false);
    }
    Ok(if cfg!(windows) {
        magic.starts_with(b"MZ")
    } else {
        magic == *b"\x7fELF"
            || matches!(
                magic,
                [0xfe, 0xed, 0xfa, 0xce | 0xcf]
                    | [0xce | 0xcf, 0xfa, 0xed, 0xfe]
                    | [0xca, 0xfe, 0xba, 0xbe | 0xbf]
            )
    })
}

fn resolve_node_script(script: &Path, rule: &str, node: &Path) -> Result<ProviderDispatch> {
    let suffix = match rule {
        "npm.cache.clean" => "npm/bin/npm-cli.js",
        "pnpm.store.prune" => "pnpm/bin/pnpm.cjs",
        "yarn.cache.clean.classic" | "yarn.cache.clean.modern" => "yarn/bin/yarn.js",
        _ => bail!("unsupported Node provider"),
    };
    if !script.ends_with(suffix) {
        bail!(
            "unsupported Node package entrypoint layout (custom wrappers and Corepack are unsupported)"
        )
    }
    let mut prefix = [0; 64];
    let count = File::open(script)?.read(&mut prefix)?;
    let line = prefix[..count]
        .split(|byte| *byte == b'\n')
        .next()
        .unwrap_or_default();
    if line.strip_suffix(b"\r").unwrap_or(line) != b"#!/usr/bin/env node" {
        bail!("unsupported Node entrypoint interpreter declaration")
    }
    let node = node
        .canonicalize()
        .context("Node interpreter cannot be resolved")?;
    if !native_binary(&node)? {
        bail!("Node interpreter must be a native executable")
    }
    Ok((
        node,
        vec![node_script_argument(script)?],
        vec![(script.to_path_buf(), provider_file_digest(script)?)],
    ))
}

fn node_script_argument(script: &Path) -> Result<String> {
    #[cfg(windows)]
    {
        // Node's main-module loader does not accept Windows verbatim paths.
        // Keep canonical paths in support identities; only translate the argv
        // spelling, and refuse if Windows resolves it to a different file.
        let canonical = script
            .canonicalize()
            .context("Node entrypoint cannot be resolved")?;
        let runtime = windows_node_script_path(&canonical)?;
        if runtime
            .canonicalize()
            .context("Node entrypoint runtime path cannot be resolved")?
            != canonical
        {
            bail!("Node entrypoint runtime path changes the canonical script identity")
        }
        runtime
            .to_str()
            .map(str::to_owned)
            .context("Node entrypoint path must be valid Unicode")
    }
    #[cfg(not(windows))]
    {
        Ok(script.display().to_string())
    }
}

#[cfg(windows)]
fn windows_node_script_path(path: &Path) -> Result<PathBuf> {
    use std::path::{Component, Prefix};

    if !path.is_absolute() {
        bail!("Node entrypoint must have an absolute filesystem path")
    }
    let mut components = path.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        bail!("Node entrypoint has an unsupported Windows path prefix")
    };
    let mut runtime = match prefix.kind() {
        Prefix::VerbatimDisk(drive) => PathBuf::from(format!("{}:", char::from(drive))),
        Prefix::VerbatimUNC(server, share) => PathBuf::from(format!(
            r"\\{}\{}",
            server
                .to_str()
                .context("Node entrypoint server must be valid Unicode")?,
            share
                .to_str()
                .context("Node entrypoint share must be valid Unicode")?
        )),
        Prefix::Disk(_) | Prefix::UNC(..) => return Ok(path.to_path_buf()),
        _ => bail!("Node entrypoint has an unsupported Windows path prefix"),
    };
    runtime.extend(components);
    Ok(runtime)
}

pub(crate) fn provider_file_digest(path: &Path) -> Result<String> {
    let mut file = File::open(path).context("provider executable cannot be read")?;
    if file.metadata()?.len() > 256 * 1024 * 1024 {
        bail!("provider executable exceeds identity-read budget")
    }
    let mut hash = Sha256::new();
    let mut bytes = [0; 64 * 1024];
    loop {
        let count = file.read(&mut bytes)?;
        if count == 0 {
            break;
        }
        hash.update(&bytes[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn windows_node_paths_translate_disk_and_unc_without_accepting_device_namespaces() {
        for (canonical, runtime) in [
            (r"\\?\C:\tools 空格\npm-cli.js", r"C:\tools 空格\npm-cli.js"),
            (
                r"\\?\UNC\server\share\tools 空格\npm-cli.js",
                r"\\server\share\tools 空格\npm-cli.js",
            ),
        ] {
            assert_eq!(
                windows_node_script_path(Path::new(canonical)).unwrap(),
                Path::new(runtime)
            );
        }
        for unsupported in [
            r"C:npm-cli.js",
            r"npm-cli.js",
            r"\\.\pipe\node",
            r"\\?\Volume{example}\npm-cli.js",
        ] {
            assert!(windows_node_script_path(Path::new(unsupported)).is_err());
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_node_argument_refuses_a_non_equivalent_normalized_path() {
        let fixture = tempfile::TempDir::new().unwrap();
        let directory = fixture.path().canonicalize().unwrap();
        let script = directory.join("entry.js.");
        std::fs::write(&script, "// verbatim-only filename").unwrap();
        std::fs::write(directory.join("entry.js"), "// different file").unwrap();
        assert!(node_script_argument(&script).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn windows_shim_runtime_argument_preserves_the_canonical_script_identity() {
        use std::path::{Component, Prefix};

        let fixture = tempfile::TempDir::new().unwrap();
        let tools = fixture.path().join("tools 空格");
        let script = tools.join("node_modules/npm/bin/npm-cli.js");
        std::fs::create_dir_all(script.parent().unwrap()).unwrap();
        std::fs::write(&script, "// fixed npm entrypoint\n").unwrap();
        let shim = tools.join("npm.cmd");
        std::fs::write(&shim, "this shim must never execute").unwrap();
        let node = tools.join("node.exe");
        std::fs::copy(std::env::current_exe().unwrap(), &node).unwrap();

        let (program, args, support) = resolve_provider_dispatch(&shim, "npm.cache.clean").unwrap();
        assert_eq!(program, node.canonicalize().unwrap());
        assert_eq!(args.len(), 1);
        let runtime_script = Path::new(&args[0]);
        assert!(matches!(runtime_script.components().next(),
            Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::UNC(..))));
        assert_eq!(
            runtime_script.canonicalize().unwrap(),
            script.canonicalize().unwrap()
        );
        assert_eq!(
            support,
            vec![
                (
                    shim.canonicalize().unwrap(),
                    provider_file_digest(&shim).unwrap()
                ),
                (
                    script.canonicalize().unwrap(),
                    provider_file_digest(&script).unwrap()
                ),
            ]
        );
    }

    #[test]
    fn fixed_node_entries_bind_script_and_use_native_interpreter() {
        let fixture = tempfile::TempDir::new().unwrap();
        let node = fixture
            .path()
            .join(format!("node{}", std::env::consts::EXE_SUFFIX));
        std::fs::copy(std::env::current_exe().unwrap(), &node).unwrap();
        for (rule, relative) in [
            ("npm.cache.clean", "npm/bin/npm-cli.js"),
            ("pnpm.store.prune", "pnpm/bin/pnpm.cjs"),
            ("yarn.cache.clean.classic", "yarn/bin/yarn.js"),
        ] {
            let script = fixture.path().join(relative);
            std::fs::create_dir_all(script.parent().unwrap()).unwrap();
            std::fs::write(&script, "#!/usr/bin/env node\n// fixture\n").unwrap();
            let dispatch = resolve_node_script(&script, rule, &node).unwrap();
            assert_eq!(dispatch.0, node.canonicalize().unwrap());
            assert_eq!(dispatch.1.len(), 1);
            assert_eq!(
                Path::new(&dispatch.1[0]).canonicalize().unwrap(),
                script.canonicalize().unwrap()
            );
            assert_eq!(
                dispatch.2,
                vec![(script.clone(), provider_file_digest(&script).unwrap())]
            );
            std::fs::write(&script, "#!/bin/sh\nnode arbitrary.js\n").unwrap();
            assert!(resolve_node_script(&script, rule, &node).is_err());
        }
        let wrapper = fixture.path().join("wrapper");
        std::fs::write(&wrapper, "#!/usr/bin/env node\n").unwrap();
        assert!(resolve_node_script(&wrapper, "npm.cache.clean", &node).is_err());
        let script = fixture.path().join("npm/bin/npm-cli.js");
        std::fs::write(&script, "#!/usr/bin/env node\n").unwrap();
        std::fs::write(&node, "#!/bin/sh\nexec other-node\n").unwrap();
        assert!(resolve_node_script(&script, "npm.cache.clean", &node).is_err());
    }
}
