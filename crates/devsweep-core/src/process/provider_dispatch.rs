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
        return Ok((node, vec![script.display().to_string()], support));
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
        vec![script.display().to_string()],
        vec![(script.to_path_buf(), provider_file_digest(script)?)],
    ))
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
            assert_eq!(dispatch.1, vec![script.display().to_string()]);
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
