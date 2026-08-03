use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{Result, bail};

use crate::{
    filesystem::normalize_absolute_path,
    model::{CleanAction, CleanupIntent, Ecosystem, RiskLevel, Scope, TargetKind, UntrustedTarget},
};

use super::{
    CARGO_HOME_RULE_DOC, KnownCacheAction, NPM_CACHE_RULE_DOC, PIP_CACHE_RULE_DOC,
    PNPM_STORE_RULE_DOC, PYCACHE_RULE_DOC, ProjectMarker, RUST_TARGET_RULE_DOC,
    YARN_CACHE_CLASSIC_RULE_DOC, YARN_CACHE_MODERN_RULE_DOC, global_cache_rules, project_dir_rules,
};

/// Trusted action metadata reconstructed from one declared rule/intent pair.
/// Callers must not construct this from plan JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionSpec {
    pub(crate) action: CleanAction,
    pub(crate) risk: RiskLevel,
    pub(crate) reversible: bool,
    pub(crate) identity: String,
    footprint: ActionFootprint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ActionFootprint {
    Path(PathBuf),
    Logical(String),
}

impl ActionSpec {
    pub(crate) fn canonical_footprint(&self) -> Result<String> {
        match &self.footprint {
            ActionFootprint::Path(path) => normalize_absolute_path(path),
            ActionFootprint::Logical(identity) => Ok(format!("logical:{identity}")),
        }
    }
}

/// The stable minimum registry contract. Future provider work extends this
/// resolver; it must not create a second action reconstruction path.
#[derive(Debug, Default)]
struct RuleRegistry;

pub(crate) fn resolve_action(target: &UntrustedTarget) -> Result<ActionSpec> {
    RuleRegistry.resolve(target)
}

pub(crate) fn intent_from_scan_action(rule_id: &str, program: &str) -> Result<CleanupIntent> {
    let (provider_id, action_id) = if rule_id == RUST_TARGET_RULE_DOC.id {
        ("cargo".to_string(), "clean_manifest".to_string())
    } else if rule_id == NPM_CACHE_RULE_DOC.id {
        ("npm".to_string(), "cache_clean".to_string())
    } else if rule_id == PIP_CACHE_RULE_DOC.id {
        (pip_provider_name(program)?, "cache_purge".to_string())
    } else if rule_id == PNPM_STORE_RULE_DOC.id {
        ("pnpm".to_string(), "store_prune".to_string())
    } else if rule_id == YARN_CACHE_CLASSIC_RULE_DOC.id {
        ("yarn".to_string(), "cache_clean_classic".to_string())
    } else if rule_id == YARN_CACHE_MODERN_RULE_DOC.id {
        ("yarn".to_string(), "cache_clean_modern".to_string())
    } else {
        bail!("scan target uses an unregistered command rule: {rule_id}")
    };
    Ok(CleanupIntent::RunBuiltInAction {
        provider_id,
        action_id,
    })
}

fn pip_provider_name(program: &str) -> Result<String> {
    let name = Path::new(program)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(program)
        .to_ascii_lowercase();
    if matches!(name.as_str(), "py" | "python" | "python3") {
        Ok(name)
    } else {
        bail!("unregistered pip provider program: {program}")
    }
}

impl RuleRegistry {
    fn resolve(&self, target: &UntrustedTarget) -> Result<ActionSpec> {
        match &target.intent {
            CleanupIntent::TrashProjectArtifact { rule_id } => {
                if rule_id != &target.rule_id {
                    bail!("intent rule id does not match target rule id")
                }
                self.resolve_trash(target)
            }
            CleanupIntent::RunBuiltInAction {
                provider_id,
                action_id,
            } => self.resolve_builtin(target, provider_id, action_id),
            CleanupIntent::InspectOnly { rule_id } => {
                if rule_id != &target.rule_id {
                    bail!("intent rule id does not match target rule id")
                }
                self.resolve_inspect_only(target)
            }
        }
    }

    fn resolve_trash(&self, target: &UntrustedTarget) -> Result<ActionSpec> {
        if target.rule_id == RUST_TARGET_RULE_DOC.id {
            // Metadata-failure fallback only: local `<project>/target` trash.
            let (root, path) = project_target_path(target)?;
            require_same_path(&path, &root.join("target"), "target path")?;
            require_target_facts(
                target,
                &Ecosystem::Rust,
                &TargetKind::BuildArtifacts,
                &RiskLevel::Medium,
                true,
            )?;
            return Ok(trash_spec(
                path,
                RUST_TARGET_RULE_DOC.id,
                &RiskLevel::Medium,
            ));
        }

        for marker in [ProjectMarker::Node, ProjectMarker::Python] {
            if let Some(rule) = project_dir_rules(marker).find(|rule| rule.id == target.rule_id) {
                let (root, path) = project_target_path(target)?;
                require_same_path(&path, &root.join(rule.relative), "target path")?;
                require_target_facts(target, &rule.ecosystem, &rule.kind, &rule.risk, true)?;
                return Ok(trash_spec(path, rule.id, &rule.risk));
            }
        }

        if target.rule_id == PYCACHE_RULE_DOC.id {
            let (root, path) = project_target_path(target)?;
            require_path_within(&path, &root, "target path")?;
            if path.file_name().and_then(|name| name.to_str()) != Some("__pycache__") {
                bail!("python.__pycache__ target path must end with __pycache__")
            }
            require_target_facts(
                target,
                &Ecosystem::Python,
                &TargetKind::TestCache,
                &PYCACHE_RULE_DOC.risk,
                true,
            )?;
            return Ok(trash_spec(
                path,
                PYCACHE_RULE_DOC.id,
                &PYCACHE_RULE_DOC.risk,
            ));
        }

        if let Some(rule) = global_cache_rules().find(|rule| rule.id == target.rule_id) {
            if rule.action != KnownCacheAction::Trash {
                bail!("{} is inspect-only and cannot use a trash intent", rule.id)
            }
            let path = global_target_path(target)?;
            require_same_path(
                &path,
                &current_home_dir()?.join(rule.relative),
                "target path",
            )?;
            require_target_facts(target, &rule.ecosystem, &rule.kind, &rule.risk, true)?;
            return Ok(trash_spec(path, rule.id, &rule.risk));
        }

        bail!("unknown trash rule id: {}", target.rule_id)
    }

    fn resolve_builtin(
        &self,
        target: &UntrustedTarget,
        provider_id: &str,
        action_id: &str,
    ) -> Result<ActionSpec> {
        if target.rule_id == RUST_TARGET_RULE_DOC.id {
            if provider_id != "cargo" || action_id != "clean_manifest" {
                bail!(
                    "unknown built-in action {provider_id}/{action_id} for rule {}",
                    target.rule_id
                )
            }
            let Scope::Project { root } = &target.scope else {
                bail!("rule {} requires project scope", target.rule_id)
            };
            let path = target
                .path
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("rule {} requires a target path", target.rule_id))?
                .clone();
            normalize_absolute_path(root)?;
            normalize_absolute_path(&path)?;
            // Custom --target-dir may place the directory outside the project
            // root; containment is enforced by SafetyPolicy live revalidation
            // against cargo metadata rather than a hard root join("target").
            require_target_facts(
                target,
                &Ecosystem::Rust,
                &TargetKind::BuildArtifacts,
                &RUST_TARGET_RULE_DOC.risk,
                false,
            )?;
            return Ok(command_spec(
                "cargo",
                vec![
                    "clean".to_string(),
                    "--manifest-path".to_string(),
                    root.join("Cargo.toml").display().to_string(),
                    "--target-dir".to_string(),
                    path.display().to_string(),
                ],
                &format!("{}:cargo_clean_manifest", RUST_TARGET_RULE_DOC.id),
                &RUST_TARGET_RULE_DOC.risk,
                ActionFootprint::Path(path),
            ));
        }

        require_global_scope(target)?;
        if let Some(path) = &target.path {
            normalize_absolute_path(path)?;
        }

        match (target.rule_id.as_str(), provider_id, action_id) {
            (rule_id, "npm", "cache_clean") if rule_id == NPM_CACHE_RULE_DOC.id => {
                require_target_facts(
                    target,
                    &NPM_CACHE_RULE_DOC.ecosystem,
                    &TargetKind::PackageCache,
                    &NPM_CACHE_RULE_DOC.risk,
                    false,
                )?;
                Ok(command_spec(
                    "npm",
                    vec![
                        "cache".to_string(),
                        "clean".to_string(),
                        "--force".to_string(),
                    ],
                    &format!("{}:npm_cache_clean", NPM_CACHE_RULE_DOC.id),
                    &NPM_CACHE_RULE_DOC.risk,
                    ActionFootprint::Logical(NPM_CACHE_RULE_DOC.id.to_string()),
                ))
            }
            (rule_id, provider, "cache_purge")
                if rule_id == PIP_CACHE_RULE_DOC.id
                    && matches!(provider, "py" | "python" | "python3") =>
            {
                require_target_facts(
                    target,
                    &PIP_CACHE_RULE_DOC.ecosystem,
                    &TargetKind::PackageCache,
                    &PIP_CACHE_RULE_DOC.risk,
                    false,
                )?;
                Ok(command_spec(
                    provider,
                    vec![
                        "-m".to_string(),
                        "pip".to_string(),
                        "cache".to_string(),
                        "purge".to_string(),
                    ],
                    &format!("{}:cache_purge", PIP_CACHE_RULE_DOC.id),
                    &PIP_CACHE_RULE_DOC.risk,
                    ActionFootprint::Logical(PIP_CACHE_RULE_DOC.id.to_string()),
                ))
            }
            (rule_id, "pnpm", "store_prune") if rule_id == PNPM_STORE_RULE_DOC.id => {
                require_target_facts(
                    target,
                    &PNPM_STORE_RULE_DOC.ecosystem,
                    &TargetKind::PackageCache,
                    &PNPM_STORE_RULE_DOC.risk,
                    false,
                )?;
                Ok(command_spec(
                    "pnpm",
                    vec!["store".to_string(), "prune".to_string()],
                    &format!("{}:store_prune", PNPM_STORE_RULE_DOC.id),
                    &PNPM_STORE_RULE_DOC.risk,
                    ActionFootprint::Logical(PNPM_STORE_RULE_DOC.id.to_string()),
                ))
            }
            (rule_id, "yarn", "cache_clean_classic")
                if rule_id == YARN_CACHE_CLASSIC_RULE_DOC.id =>
            {
                require_target_facts(
                    target,
                    &YARN_CACHE_CLASSIC_RULE_DOC.ecosystem,
                    &TargetKind::PackageCache,
                    &YARN_CACHE_CLASSIC_RULE_DOC.risk,
                    false,
                )?;
                Ok(command_spec(
                    "yarn",
                    vec!["cache".to_string(), "clean".to_string()],
                    &format!("{}:cache_clean", YARN_CACHE_CLASSIC_RULE_DOC.id),
                    &YARN_CACHE_CLASSIC_RULE_DOC.risk,
                    ActionFootprint::Logical(YARN_CACHE_CLASSIC_RULE_DOC.id.to_string()),
                ))
            }
            (rule_id, "yarn", "cache_clean_modern") if rule_id == YARN_CACHE_MODERN_RULE_DOC.id => {
                require_target_facts(
                    target,
                    &YARN_CACHE_MODERN_RULE_DOC.ecosystem,
                    &TargetKind::PackageCache,
                    &YARN_CACHE_MODERN_RULE_DOC.risk,
                    false,
                )?;
                Ok(command_spec(
                    "yarn",
                    vec![
                        "cache".to_string(),
                        "clean".to_string(),
                        "--mirror".to_string(),
                    ],
                    &format!("{}:cache_clean_mirror", YARN_CACHE_MODERN_RULE_DOC.id),
                    &YARN_CACHE_MODERN_RULE_DOC.risk,
                    ActionFootprint::Logical(YARN_CACHE_MODERN_RULE_DOC.id.to_string()),
                ))
            }
            _ => bail!(
                "unknown built-in action {provider_id}/{action_id} for rule {}",
                target.rule_id
            ),
        }
    }

    fn resolve_inspect_only(&self, target: &UntrustedTarget) -> Result<ActionSpec> {
        require_global_scope(target)?;
        if target.rule_id == CARGO_HOME_RULE_DOC.id {
            let path = global_target_path(target)?;
            require_same_path(&path, &current_cargo_home()?, "target path")?;
            require_target_facts(
                target,
                &CARGO_HOME_RULE_DOC.ecosystem,
                &TargetKind::PackageCache,
                &CARGO_HOME_RULE_DOC.risk,
                true,
            )?;
            return Ok(inspect_spec(
                CARGO_HOME_RULE_DOC.id,
                &CARGO_HOME_RULE_DOC.risk,
            ));
        }

        if let Some(rule) = global_cache_rules().find(|rule| rule.id == target.rule_id) {
            if rule.action != KnownCacheAction::InspectOnly {
                bail!("{} requires a trash intent", rule.id)
            }
            let path = global_target_path(target)?;
            require_same_path(
                &path,
                &current_home_dir()?.join(rule.relative),
                "target path",
            )?;
            require_target_facts(target, &rule.ecosystem, &rule.kind, &rule.risk, true)?;
            return Ok(inspect_spec(rule.id, &rule.risk));
        }

        bail!("unknown inspect-only rule id: {}", target.rule_id)
    }
}

fn trash_spec(path: PathBuf, rule_id: &str, risk: &RiskLevel) -> ActionSpec {
    ActionSpec {
        action: CleanAction::MoveToTrash { path: path.clone() },
        risk: risk.clone(),
        reversible: true,
        identity: format!("trash:{rule_id}"),
        footprint: ActionFootprint::Path(path),
    }
}

fn inspect_spec(rule_id: &str, risk: &RiskLevel) -> ActionSpec {
    ActionSpec {
        action: CleanAction::NoopInspectOnly,
        risk: risk.clone(),
        reversible: true,
        identity: format!("inspect:{rule_id}"),
        footprint: ActionFootprint::Logical(format!("inspect:{rule_id}")),
    }
}

fn command_spec(
    program: &str,
    args: Vec<String>,
    identity: &str,
    risk: &RiskLevel,
    footprint: ActionFootprint,
) -> ActionSpec {
    ActionSpec {
        action: CleanAction::Command {
            program: program.to_string(),
            args,
            cwd: None,
            irreversible: true,
        },
        risk: risk.clone(),
        reversible: false,
        identity: identity.to_string(),
        footprint,
    }
}

fn require_target_facts(
    target: &UntrustedTarget,
    ecosystem: &Ecosystem,
    kind: &TargetKind,
    risk: &RiskLevel,
    reversible: bool,
) -> Result<()> {
    if &target.ecosystem != ecosystem {
        bail!(
            "target ecosystem is inconsistent with rule {}",
            target.rule_id
        )
    }
    if &target.kind != kind {
        bail!("target kind is inconsistent with rule {}", target.rule_id)
    }
    if &target.risk != risk {
        bail!("target risk is inconsistent with rule {}", target.rule_id)
    }
    if target.reversible != reversible {
        bail!(
            "target reversibility is inconsistent with rule {}",
            target.rule_id
        )
    }
    Ok(())
}

fn project_target_path(target: &UntrustedTarget) -> Result<(PathBuf, PathBuf)> {
    let Scope::Project { root } = &target.scope else {
        bail!("rule {} requires project scope", target.rule_id)
    };
    let path = target
        .path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("rule {} requires a target path", target.rule_id))?;
    normalize_absolute_path(root)?;
    normalize_absolute_path(path)?;
    require_path_within(path, root, "target path")?;
    Ok((root.clone(), path.clone()))
}

fn global_target_path(target: &UntrustedTarget) -> Result<PathBuf> {
    require_global_scope(target)?;
    let path = target
        .path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("rule {} requires a target path", target.rule_id))?;
    normalize_absolute_path(path)?;
    Ok(path.clone())
}

fn require_global_scope(target: &UntrustedTarget) -> Result<()> {
    if !matches!(&target.scope, Scope::Global) {
        bail!("rule {} requires global scope", target.rule_id)
    }
    Ok(())
}

fn require_same_path(left: &Path, right: &Path, label: &str) -> Result<()> {
    if normalize_absolute_path(left)? != normalize_absolute_path(right)? {
        bail!("{label} is inconsistent with the trusted rule action")
    }
    Ok(())
}

fn require_path_within(path: &Path, root: &Path, label: &str) -> Result<()> {
    let path = normalize_absolute_path(path)?;
    let root = normalize_absolute_path(root)?;
    let child_prefix = if root.ends_with('/') {
        root.clone()
    } else {
        format!("{root}/")
    };
    if path != root && !path.starts_with(&child_prefix) {
        bail!("{label} escapes the declared scope")
    }
    Ok(())
}

fn current_home_dir() -> Result<PathBuf> {
    env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| {
            let drive = env::var_os("HOMEDRIVE")?;
            let path = env::var_os("HOMEPATH")?;
            Some(PathBuf::from(format!(
                "{}{}",
                drive.to_string_lossy(),
                path.to_string_lossy()
            )))
        })
        .or_else(|| env::var_os("HOME").map(PathBuf::from))
        .ok_or_else(|| anyhow::anyhow!("unable to resolve the current user home directory"))
}

fn current_cargo_home() -> Result<PathBuf> {
    Ok(env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or(current_home_dir()?.join(".cargo")))
}
