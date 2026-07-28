use std::{
    collections::HashSet,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    model::{
        CLEANUP_PLAN_VERSION, CleanAction, CleanTarget, CleanupIntent, CleanupPlan, Evidence,
        LEGACY_CLEANUP_PLAN_VERSION, Scope, TargetId, UntrustedPlan, UntrustedTarget,
    },
    path_identity::normalize_absolute_path,
    registry::{ActionSpec, RuleRegistry},
};

pub const V1_RESCAN_MESSAGE: &str =
    "plan format v1 is no longer accepted; re-run `devsweep scan --json`";

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActionFingerprint(String);

impl ActionFingerprint {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedTarget {
    target: CleanTarget,
    rule_id: String,
    action_identity: String,
    fingerprint: ActionFingerprint,
}

impl ValidatedTarget {
    pub fn target(&self) -> &CleanTarget {
        &self.target
    }

    pub fn fingerprint(&self) -> &ActionFingerprint {
        &self.fingerprint
    }
}

/// Opaque execution input. Its fields are private so only `validate_plan` can
/// create normal values, and the executor never accepts untrusted DTOs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedPlan {
    targets: Vec<ValidatedTarget>,
    digest: String,
}

impl ValidatedPlan {
    pub fn targets(&self) -> &[ValidatedTarget] {
        &self.targets
    }

    pub fn default_selected_ids(&self) -> Vec<TargetId> {
        self.targets
            .iter()
            .filter(|target| target.target.selected_by_default)
            .map(|target| target.target.id.clone())
            .collect()
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn digest_prefix(&self) -> &str {
        &self.digest[..12]
    }

    #[cfg(test)]
    pub(crate) fn from_cleanup_plan_for_test(plan: CleanupPlan) -> Self {
        let targets = plan
            .targets
            .into_iter()
            .map(|target| {
                let rule_id = target
                    .evidence
                    .iter()
                    .find_map(|evidence| match evidence {
                        Evidence::RuleMatched { rule_id } => Some(rule_id.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| target.id.as_str().to_string());
                let action_identity = test_action_identity(&target.action);
                let footprint = target
                    .path
                    .as_ref()
                    .map(|path| normalize_absolute_path(path).expect("test target path"))
                    .unwrap_or_else(|| format!("logical:{rule_id}"));
                ValidatedTarget {
                    target,
                    rule_id,
                    action_identity: action_identity.clone(),
                    fingerprint: ActionFingerprint(format!("{action_identity}|{footprint}")),
                }
            })
            .collect::<Vec<_>>();
        let digest = digest_for_targets(&targets).expect("test targets canonicalize");
        Self { targets, digest }
    }
}

#[cfg(test)]
fn test_action_identity(action: &CleanAction) -> String {
    match action {
        CleanAction::Command {
            program,
            args,
            cwd,
            irreversible,
        } => format!("command:{program}:{args:?}:{cwd:?}:{irreversible}"),
        CleanAction::MoveToTrash { .. } => "trash".to_string(),
        CleanAction::DeletePermanently {
            requires_explicit_flag,
            ..
        } => format!("delete:{requires_explicit_flag}"),
        CleanAction::NoopInspectOnly => "inspect".to_string(),
    }
}

pub fn validate_plan(plan: &UntrustedPlan) -> Result<ValidatedPlan> {
    if plan.version == LEGACY_CLEANUP_PLAN_VERSION {
        bail!(V1_RESCAN_MESSAGE)
    }
    if plan.version != CLEANUP_PLAN_VERSION {
        bail!(
            "unsupported cleanup plan version {}; expected {}",
            plan.version,
            CLEANUP_PLAN_VERSION
        )
    }

    let registry = RuleRegistry;
    let mut ids = HashSet::new();
    let mut fingerprints = HashSet::new();
    let mut targets = Vec::with_capacity(plan.targets.len());

    for untrusted in &plan.targets {
        if untrusted.id.as_str().trim().is_empty() {
            bail!("cleanup target id must not be empty")
        }
        if !ids.insert(untrusted.id.clone()) {
            bail!("duplicate cleanup target id: {}", untrusted.id.as_str())
        }
        validate_intent_selection(untrusted)?;
        validate_observed_paths(untrusted)?;

        let spec = registry
            .resolve(untrusted)
            .with_context(|| format!("invalid target {}", untrusted.id.as_str()))?;
        validate_resolved_action(untrusted, &spec)?;
        let fingerprint = action_fingerprint(&spec)?;
        if !fingerprints.insert(fingerprint.clone()) {
            bail!(
                "duplicate canonical action fingerprint: {}",
                fingerprint.as_str()
            )
        }

        targets.push(ValidatedTarget {
            target: resolved_target(untrusted, spec.action),
            rule_id: untrusted.rule_id.clone(),
            action_identity: spec.identity,
            fingerprint,
        });
    }

    let digest = digest_for_targets(&targets)?;
    Ok(ValidatedPlan { targets, digest })
}

/// Converts an in-memory scan result into the only JSON-facing v2 DTO. This
/// boundary is intentionally strict: a scanner bug must not serialize a direct
/// executable action as a saved plan.
pub fn untrusted_plan_from_scan(plan: &CleanupPlan) -> Result<UntrustedPlan> {
    let targets = plan
        .targets
        .iter()
        .map(untrusted_target_from_scan)
        .collect::<Result<Vec<_>>>()?;
    Ok(UntrustedPlan {
        version: CLEANUP_PLAN_VERSION,
        targets,
    })
}

pub fn validate_scanned_plan(plan: &CleanupPlan) -> Result<ValidatedPlan> {
    validate_plan(&untrusted_plan_from_scan(plan)?)
}

fn untrusted_target_from_scan(target: &CleanTarget) -> Result<UntrustedTarget> {
    let rule_id = target
        .evidence
        .iter()
        .find_map(|evidence| match evidence {
            Evidence::RuleMatched { rule_id } => Some(rule_id.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            anyhow::anyhow!("scan target {} has no rule evidence", target.id.as_str())
        })?;

    let intent = match &target.action {
        CleanAction::MoveToTrash { path } => {
            if target.path.as_ref() != Some(path) {
                bail!(
                    "scan target {} has mismatched trash action path",
                    target.id.as_str()
                )
            }
            CleanupIntent::TrashProjectArtifact {
                rule_id: rule_id.clone(),
            }
        }
        CleanAction::NoopInspectOnly => CleanupIntent::InspectOnly {
            rule_id: rule_id.clone(),
        },
        CleanAction::Command { program, .. } => built_in_intent(&rule_id, program)?,
        CleanAction::DeletePermanently { .. } => {
            bail!(
                "scan target {} uses disabled permanent delete",
                target.id.as_str()
            )
        }
    };

    Ok(UntrustedTarget {
        id: target.id.clone(),
        rule_id,
        scope: target.scope.clone(),
        ecosystem: target.ecosystem.clone(),
        kind: target.kind.clone(),
        path: target.path.clone(),
        estimated_bytes: target.estimated_bytes,
        size_complete: target.size_complete,
        last_modified: target.last_modified,
        risk: target.risk.clone(),
        reversible: target.reversible,
        selected_by_default: target.selected_by_default,
        evidence: target.evidence.clone(),
        intent,
    })
}

fn built_in_intent(rule_id: &str, program: &str) -> Result<CleanupIntent> {
    let (provider_id, action_id) = match rule_id {
        "rust.target" => ("cargo".to_string(), "clean_manifest".to_string()),
        "npm.cache.clean" => ("npm".to_string(), "cache_clean".to_string()),
        "pip.cache.purge" => (provider_name(program)?, "cache_purge".to_string()),
        "pnpm.store.prune" => ("pnpm".to_string(), "store_prune".to_string()),
        "yarn.cache.clean.classic" => ("yarn".to_string(), "cache_clean_classic".to_string()),
        "yarn.cache.clean.modern" => ("yarn".to_string(), "cache_clean_modern".to_string()),
        _ => bail!("scan target uses an unregistered command rule: {rule_id}"),
    };
    Ok(CleanupIntent::RunBuiltInAction {
        provider_id,
        action_id,
    })
}

fn provider_name(program: &str) -> Result<String> {
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

fn validate_observed_paths(target: &UntrustedTarget) -> Result<()> {
    if let Scope::Project { root } = &target.scope {
        normalize_absolute_path(root)?;
    }
    if let Some(path) = &target.path {
        normalize_absolute_path(path)?;
    }
    Ok(())
}

fn validate_intent_selection(target: &UntrustedTarget) -> Result<()> {
    if matches!(&target.intent, CleanupIntent::InspectOnly { .. }) && target.selected_by_default {
        bail!("inspect-only targets cannot be selected for execution")
    }
    Ok(())
}

fn validate_resolved_action(target: &UntrustedTarget, spec: &ActionSpec) -> Result<()> {
    if target.risk != spec.risk {
        bail!("target risk is inconsistent with reconstructed action")
    }
    if target.reversible != spec.reversible {
        bail!("target reversibility is inconsistent with reconstructed action")
    }
    if matches!(spec.action, CleanAction::NoopInspectOnly) && target.selected_by_default {
        bail!("inspect-only targets cannot be selected for execution")
    }
    if let CleanAction::MoveToTrash { path } = &spec.action {
        let Some(target_path) = &target.path else {
            bail!("trash action requires a target path")
        };
        if normalize_absolute_path(target_path)? != normalize_absolute_path(path)? {
            bail!("target path and reconstructed action path do not match")
        }
    }
    Ok(())
}

fn resolved_target(target: &UntrustedTarget, action: CleanAction) -> CleanTarget {
    CleanTarget {
        id: target.id.clone(),
        scope: target.scope.clone(),
        ecosystem: target.ecosystem.clone(),
        kind: target.kind.clone(),
        path: target.path.clone(),
        estimated_bytes: target.estimated_bytes,
        size_complete: target.size_complete,
        last_modified: target.last_modified,
        risk: target.risk.clone(),
        reversible: target.reversible,
        selected_by_default: target.selected_by_default,
        evidence: target.evidence.clone(),
        action,
    }
}

fn action_fingerprint(spec: &ActionSpec) -> Result<ActionFingerprint> {
    let footprint = spec.canonical_footprint()?;
    Ok(ActionFingerprint(format!("{}|{footprint}", spec.identity)))
}

fn digest_for_targets(targets: &[ValidatedTarget]) -> Result<String> {
    let mut entries = targets
        .iter()
        .map(canonical_target)
        .collect::<Result<Vec<_>>>()?;
    entries.sort_by(|left, right| {
        left.fingerprint
            .cmp(&right.fingerprint)
            .then_with(|| left.id.cmp(&right.id))
    });
    let bytes = serde_json::to_vec(&CanonicalManifest {
        domain: "devsweep.validated-plan.v2",
        version: CLEANUP_PLAN_VERSION,
        targets: entries,
    })
    .context("failed to encode canonical validated plan")?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[derive(Serialize)]
struct CanonicalManifest {
    domain: &'static str,
    version: u32,
    targets: Vec<CanonicalTarget>,
}

#[derive(Serialize)]
struct CanonicalTarget {
    fingerprint: String,
    id: String,
    rule_id: String,
    action_identity: String,
    action: CanonicalAction,
    scope: String,
    ecosystem: String,
    kind: String,
    path: Option<String>,
    estimated_bytes: u64,
    size_complete: bool,
    last_modified_ms: Option<u128>,
    risk: String,
    reversible: bool,
    selected_by_default: bool,
    evidence: Vec<String>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum CanonicalAction {
    Command {
        program: String,
        args: Vec<String>,
        cwd: Option<String>,
        irreversible: bool,
    },
    MoveToTrash {
        path: String,
    },
    DeletePermanently {
        path: String,
        requires_explicit_flag: bool,
    },
    NoopInspectOnly,
}

fn canonical_target(target: &ValidatedTarget) -> Result<CanonicalTarget> {
    let target_path = target
        .target
        .path
        .as_ref()
        .map(|path| normalize_absolute_path(path))
        .transpose()?;
    let scope = match &target.target.scope {
        Scope::Global => "global".to_string(),
        Scope::Project { root } => format!("project:{}", normalize_absolute_path(root)?),
    };
    let mut evidence = target
        .target
        .evidence
        .iter()
        .map(canonical_evidence)
        .collect::<Result<Vec<_>>>()?;
    evidence.sort();
    Ok(CanonicalTarget {
        fingerprint: target.fingerprint.0.clone(),
        id: target.target.id.as_str().to_string(),
        rule_id: target.rule_id.clone(),
        action_identity: target.action_identity.clone(),
        action: canonical_action(&target.target.action)?,
        scope,
        ecosystem: format!("{:?}", target.target.ecosystem),
        kind: format!("{:?}", target.target.kind),
        path: target_path,
        estimated_bytes: target.target.estimated_bytes,
        size_complete: target.target.size_complete,
        last_modified_ms: target.target.last_modified.and_then(system_time_ms),
        risk: format!("{:?}", target.target.risk),
        reversible: target.target.reversible,
        selected_by_default: target.target.selected_by_default,
        evidence,
    })
}

fn canonical_action(action: &CleanAction) -> Result<CanonicalAction> {
    match action {
        CleanAction::Command {
            program,
            args,
            cwd,
            irreversible,
        } => Ok(CanonicalAction::Command {
            program: program.clone(),
            args: args.clone(),
            cwd: cwd
                .as_ref()
                .map(|path| normalize_absolute_path(path))
                .transpose()?,
            irreversible: *irreversible,
        }),
        CleanAction::MoveToTrash { path } => Ok(CanonicalAction::MoveToTrash {
            path: normalize_absolute_path(path)?,
        }),
        CleanAction::DeletePermanently {
            path,
            requires_explicit_flag,
        } => Ok(CanonicalAction::DeletePermanently {
            path: normalize_absolute_path(path)?,
            requires_explicit_flag: *requires_explicit_flag,
        }),
        CleanAction::NoopInspectOnly => Ok(CanonicalAction::NoopInspectOnly),
    }
}

fn canonical_evidence(evidence: &Evidence) -> Result<String> {
    match evidence {
        Evidence::MarkerFile { path } => Ok(format!("marker:{}", normalize_absolute_path(path)?)),
        Evidence::KnownCacheDir { source, path } => {
            Ok(format!("known:{source}:{}", normalize_absolute_path(path)?))
        }
        Evidence::OfficialCommand { command } => Ok(format!("official:{command}")),
        Evidence::RuleMatched { rule_id } => Ok(format!("rule:{rule_id}")),
        Evidence::UserConfigured => Ok("user_configured".to_string()),
    }
}

fn system_time_ms(value: SystemTime) -> Option<u128> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|value| value.as_millis())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::*;
    use crate::model::{Ecosystem, RiskLevel, Scope, TargetKind};

    #[test]
    fn rejects_v1_with_rescan_guidance() {
        let plan = UntrustedPlan {
            version: LEGACY_CLEANUP_PLAN_VERSION,
            targets: Vec::new(),
        };

        assert_eq!(
            validate_plan(&plan).unwrap_err().to_string(),
            V1_RESCAN_MESSAGE
        );
    }

    #[test]
    fn rejects_unknown_plan_versions() {
        for version in [0, CLEANUP_PLAN_VERSION + 1] {
            let error = validate_plan(&UntrustedPlan {
                version,
                targets: Vec::new(),
            })
            .expect_err("only the current plan version is accepted");

            assert!(
                error
                    .to_string()
                    .contains("unsupported cleanup plan version"),
                "unexpected error for version {version}: {error:#}"
            );
        }
    }

    #[test]
    fn json_dto_rejects_arbitrary_action_fields() {
        let value = json!({
            "version": CLEANUP_PLAN_VERSION,
            "targets": [],
            "action": { "program": "cmd.exe" }
        });

        assert!(serde_json::from_value::<UntrustedPlan>(value).is_err());
    }

    #[test]
    fn typed_intents_reject_arbitrary_programs_and_arguments() {
        for program in ["cmd.exe", "powershell", "/bin/sh"] {
            let plan = UntrustedPlan {
                version: CLEANUP_PLAN_VERSION,
                targets: vec![npm_target("npm.cache.clean:untrusted", None)],
            };
            let mut value = serde_json::to_value(plan).expect("plan serializes");
            let intent = value["targets"][0]["intent"]
                .as_object_mut()
                .expect("intent is an object");
            intent.insert("program".to_string(), json!(program));
            intent.insert("args".to_string(), json!(["-Command", "arbitrary"]));

            assert!(
                serde_json::from_value::<UntrustedPlan>(value).is_err(),
                "{program} must not fit the declarative intent schema"
            );
        }
    }

    #[test]
    fn nested_plan_facts_reject_unknown_fields() {
        let root = temp_root();
        let plan = single_target_plan(project_target(
            "node.next_cache",
            &root,
            root.join(".next/cache"),
            TargetKind::BuildArtifacts,
            RiskLevel::Low,
            true,
        ));

        let mut scope_value = serde_json::to_value(&plan).expect("plan serializes");
        scope_value["targets"][0]["scope"]["unexpected"] = json!(true);
        assert!(serde_json::from_value::<UntrustedPlan>(scope_value).is_err());

        let mut evidence_value = serde_json::to_value(plan).expect("plan serializes");
        evidence_value["targets"][0]["evidence"][0]["unexpected"] = json!(true);
        assert!(serde_json::from_value::<UntrustedPlan>(evidence_value).is_err());
    }

    #[test]
    fn rejects_relative_paths_before_registry_resolution() {
        let root = temp_root();
        let mut target = project_target(
            "node.next_cache",
            &root,
            root.join(".next/cache"),
            TargetKind::BuildArtifacts,
            RiskLevel::Low,
            true,
        );
        target.path = Some(PathBuf::from(".next/cache"));

        let error = validate_plan(&single_target_plan(target)).expect_err("relative path rejects");

        assert!(error.to_string().contains("path must be absolute"));
    }

    #[test]
    fn rejects_unknown_rules_and_actions() {
        let root = temp_root();
        let mut unknown_rule = project_target(
            "node.next_cache",
            &root,
            root.join(".next/cache"),
            TargetKind::BuildArtifacts,
            RiskLevel::Low,
            true,
        );
        unknown_rule.rule_id = "unknown.rule".to_string();
        unknown_rule.intent = CleanupIntent::TrashProjectArtifact {
            rule_id: "unknown.rule".to_string(),
        };

        let unknown_rule_error =
            validate_plan(&single_target_plan(unknown_rule)).expect_err("unknown rule rejects");
        assert!(
            format!("{unknown_rule_error:#}").contains("unknown trash rule id"),
            "{unknown_rule_error:#}"
        );

        let mut unknown_action = npm_target("npm.cache.clean:unknown-action", None);
        unknown_action.intent = CleanupIntent::RunBuiltInAction {
            provider_id: "npm".to_string(),
            action_id: "arbitrary_action".to_string(),
        };

        let unknown_action_error =
            validate_plan(&single_target_plan(unknown_action)).expect_err("unknown action rejects");
        assert!(
            format!("{unknown_action_error:#}").contains("unknown built-in action"),
            "{unknown_action_error:#}"
        );
    }

    #[test]
    fn rejects_path_mismatches_before_action_reconstruction() {
        let root = temp_root();
        let target = project_target(
            "node.next_cache",
            &root,
            root.join(".turbo"),
            TargetKind::BuildArtifacts,
            RiskLevel::Low,
            true,
        );

        let mismatch_error =
            validate_plan(&single_target_plan(target)).expect_err("path mismatch rejects");
        assert!(
            format!("{mismatch_error:#}")
                .contains("target path is inconsistent with the trusted rule action"),
            "{mismatch_error:#}"
        );
    }

    #[test]
    fn rejects_paths_that_escape_the_declared_project_scope() {
        let root = temp_root();
        let escaped_path = root
            .parent()
            .expect("temporary fixture has a parent")
            .join(".next/cache");
        let target = project_target(
            "node.next_cache",
            &root,
            escaped_path,
            TargetKind::BuildArtifacts,
            RiskLevel::Low,
            true,
        );

        let error = validate_plan(&single_target_plan(target))
            .expect_err("targets outside the project root reject");
        assert!(
            format!("{error:#}").contains("target path escapes the declared scope"),
            "{error:#}"
        );
    }

    #[test]
    fn rejects_risk_and_reversibility_that_drift_from_the_registry() {
        let root = temp_root();
        let mut risk_drift = project_target(
            "node.next_cache",
            &root,
            root.join(".next/cache"),
            TargetKind::BuildArtifacts,
            RiskLevel::Low,
            true,
        );
        risk_drift.risk = RiskLevel::Medium;
        assert!(
            format!(
                "{:#}",
                validate_plan(&single_target_plan(risk_drift))
                    .expect_err("risk must agree with the registry")
            )
            .contains("target risk is inconsistent"),
        );

        let mut reversibility_drift = project_target(
            "node.next_cache",
            &root,
            root.join(".next/cache"),
            TargetKind::BuildArtifacts,
            RiskLevel::Low,
            true,
        );
        reversibility_drift.reversible = false;
        assert!(
            format!(
                "{:#}",
                validate_plan(&single_target_plan(reversibility_drift))
                    .expect_err("reversibility must agree with the registry")
            )
            .contains("target reversibility is inconsistent"),
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn accepts_project_targets_under_the_filesystem_root() {
        let root = PathBuf::from("/");
        let target = project_target(
            "node.next_cache",
            &root,
            root.join(".next/cache"),
            TargetKind::BuildArtifacts,
            RiskLevel::Low,
            true,
        );

        validate_plan(&single_target_plan(target))
            .expect("a child of the filesystem root remains within project scope");
    }

    #[test]
    fn rejects_selected_inspect_only_intents() {
        let target = UntrustedTarget {
            id: TargetId::new("cargo.home.inspect:untrusted"),
            rule_id: "cargo.home.inspect".to_string(),
            scope: Scope::Global,
            ecosystem: Ecosystem::Rust,
            kind: TargetKind::PackageCache,
            path: None,
            estimated_bytes: 0,
            size_complete: true,
            last_modified: None,
            risk: RiskLevel::High,
            reversible: true,
            selected_by_default: true,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "cargo.home.inspect".to_string(),
            }],
            intent: CleanupIntent::InspectOnly {
                rule_id: "cargo.home.inspect".to_string(),
            },
        };

        assert_eq!(
            validate_plan(&single_target_plan(target))
                .expect_err("inspect-only selection rejects")
                .to_string(),
            "inspect-only targets cannot be selected for execution"
        );
    }

    #[test]
    fn equivalent_target_order_has_the_same_digest() {
        let root = temp_root();
        let first = project_target(
            "node.next_cache",
            &root,
            root.join(".next/cache"),
            TargetKind::BuildArtifacts,
            RiskLevel::Low,
            true,
        );
        let second = project_target(
            "node.turbo",
            &root,
            root.join(".turbo"),
            TargetKind::ToolCache,
            RiskLevel::Low,
            true,
        );
        let forward = UntrustedPlan {
            version: CLEANUP_PLAN_VERSION,
            targets: vec![first.clone(), second.clone()],
        };
        let reverse = UntrustedPlan {
            version: CLEANUP_PLAN_VERSION,
            targets: vec![second, first],
        };

        assert_eq!(
            validate_plan(&forward).expect("forward validates").digest(),
            validate_plan(&reverse).expect("reverse validates").digest()
        );
    }

    #[test]
    fn rejects_duplicate_equivalent_fingerprints() {
        let root = temp_root();
        let first = project_target(
            "node.next_cache",
            &root,
            root.join(".next/cache"),
            TargetKind::BuildArtifacts,
            RiskLevel::Low,
            true,
        );
        let mut duplicate = first.clone();
        duplicate.id = TargetId::new("another.id");
        let plan = UntrustedPlan {
            version: CLEANUP_PLAN_VERSION,
            targets: vec![first, duplicate],
        };

        assert!(
            validate_plan(&plan)
                .unwrap_err()
                .to_string()
                .contains("duplicate canonical action fingerprint")
        );
    }

    #[test]
    fn provider_commands_use_trusted_logical_footprints_for_deduplication() {
        let first = npm_target(
            "npm.cache.clean:first",
            Some(temp_root().join("npm-cache-a")),
        );
        let second = npm_target(
            "npm.cache.clean:second",
            Some(temp_root().join("npm-cache-b")),
        );

        assert!(
            validate_plan(&UntrustedPlan {
                version: CLEANUP_PLAN_VERSION,
                targets: vec![first, second],
            })
            .expect_err("different untrusted provider paths cannot duplicate a command")
            .to_string()
            .contains("duplicate canonical action fingerprint")
        );
    }

    #[test]
    fn digest_binds_the_reconstructed_provider_command() {
        let py = pip_target("pip.cache.purge:py", "py");
        let python = pip_target("pip.cache.purge:python", "python");

        assert_ne!(
            validate_plan(&single_target_plan(py))
                .expect("py provider validates")
                .digest(),
            validate_plan(&single_target_plan(python))
                .expect("python provider validates")
                .digest(),
            "different reconstructed programs must not share a confirmation digest"
        );
    }

    #[test]
    fn provider_aliases_cannot_bypass_logical_fingerprint_deduplication() {
        let error = validate_plan(&UntrustedPlan {
            version: CLEANUP_PLAN_VERSION,
            targets: vec![
                pip_target("pip.cache.purge:py", "py"),
                pip_target("pip.cache.purge:python", "python"),
            ],
        })
        .expect_err("provider aliases describe one logical pip purge");

        assert!(
            error
                .to_string()
                .contains("duplicate canonical action fingerprint"),
            "{error:#}"
        );
    }

    #[test]
    fn digest_changes_when_a_validated_target_changes() {
        let root = temp_root();
        let next_cache = project_target(
            "node.next_cache",
            &root,
            root.join(".next/cache"),
            TargetKind::BuildArtifacts,
            RiskLevel::Low,
            true,
        );
        let turbo_cache = project_target(
            "node.turbo",
            &root,
            root.join(".turbo"),
            TargetKind::ToolCache,
            RiskLevel::Low,
            true,
        );

        assert_ne!(
            validate_plan(&single_target_plan(next_cache))
                .expect("next cache validates")
                .digest(),
            validate_plan(&single_target_plan(turbo_cache))
                .expect("turbo cache validates")
                .digest()
        );
    }

    #[cfg(windows)]
    #[test]
    fn rejects_windows_case_separator_and_trailing_separator_duplicates() {
        let root = temp_root();
        let first = project_target(
            "node.next_cache",
            &root,
            root.join(".next/cache"),
            TargetKind::BuildArtifacts,
            RiskLevel::Low,
            true,
        );
        let mut duplicate = first.clone();
        duplicate.id = TargetId::new("node.next_cache:case-variant");
        duplicate.scope = Scope::Project {
            root: windows_equivalent_path(&root, false),
        };
        duplicate.path = Some(windows_equivalent_path(&root.join(".next/cache"), true));

        assert!(
            validate_plan(&UntrustedPlan {
                version: CLEANUP_PLAN_VERSION,
                targets: vec![first, duplicate],
            })
            .expect_err("case and separator variants must share one fingerprint")
            .to_string()
            .contains("duplicate canonical action fingerprint")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn rejects_posix_leading_separator_duplicates() {
        let root = temp_root();
        let first = project_target(
            "node.next_cache",
            &root,
            root.join(".next/cache"),
            TargetKind::BuildArtifacts,
            RiskLevel::Low,
            true,
        );
        let mut duplicate = first.clone();
        duplicate.id = TargetId::new("node.next_cache:leading-separator-variant");
        duplicate.scope = Scope::Project {
            root: posix_leading_separator_variant(&root),
        };
        duplicate.path = Some(posix_leading_separator_variant(&root.join(".next/cache")));

        assert!(
            validate_plan(&UntrustedPlan {
                version: CLEANUP_PLAN_VERSION,
                targets: vec![first, duplicate],
            })
            .expect_err("leading POSIX separator variants must share one fingerprint")
            .to_string()
            .contains("duplicate canonical action fingerprint")
        );
    }

    fn temp_root() -> PathBuf {
        std::env::temp_dir().join("devsweep-plan-validation-fixture")
    }

    fn single_target_plan(target: UntrustedTarget) -> UntrustedPlan {
        UntrustedPlan {
            version: CLEANUP_PLAN_VERSION,
            targets: vec![target],
        }
    }

    fn npm_target(id: &str, path: Option<PathBuf>) -> UntrustedTarget {
        UntrustedTarget {
            id: TargetId::new(id),
            rule_id: "npm.cache.clean".to_string(),
            scope: Scope::Global,
            ecosystem: Ecosystem::Node,
            kind: TargetKind::PackageCache,
            path,
            estimated_bytes: 1,
            size_complete: true,
            last_modified: None,
            risk: RiskLevel::Medium,
            reversible: false,
            selected_by_default: false,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "npm.cache.clean".to_string(),
            }],
            intent: CleanupIntent::RunBuiltInAction {
                provider_id: "npm".to_string(),
                action_id: "cache_clean".to_string(),
            },
        }
    }

    fn pip_target(id: &str, provider_id: &str) -> UntrustedTarget {
        UntrustedTarget {
            id: TargetId::new(id),
            rule_id: "pip.cache.purge".to_string(),
            scope: Scope::Global,
            ecosystem: Ecosystem::Python,
            kind: TargetKind::PackageCache,
            path: None,
            estimated_bytes: 1,
            size_complete: true,
            last_modified: None,
            risk: RiskLevel::Medium,
            reversible: false,
            selected_by_default: false,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "pip.cache.purge".to_string(),
            }],
            intent: CleanupIntent::RunBuiltInAction {
                provider_id: provider_id.to_string(),
                action_id: "cache_purge".to_string(),
            },
        }
    }

    #[cfg(windows)]
    fn windows_equivalent_path(path: &std::path::Path, trailing_separator: bool) -> PathBuf {
        let mut value = path
            .to_string_lossy()
            .replace('/', "\\")
            .to_ascii_uppercase();
        if trailing_separator {
            value.push('\\');
        }
        PathBuf::from(value)
    }

    #[cfg(not(windows))]
    fn posix_leading_separator_variant(path: &std::path::Path) -> PathBuf {
        PathBuf::from(format!("/{}", path.display()))
    }

    fn project_target(
        rule_id: &str,
        root: &std::path::Path,
        path: PathBuf,
        kind: TargetKind,
        risk: crate::model::RiskLevel,
        selected_by_default: bool,
    ) -> UntrustedTarget {
        UntrustedTarget {
            id: TargetId::new(format!("{rule_id}:{}", path.display())),
            rule_id: rule_id.to_string(),
            scope: Scope::Project {
                root: root.to_path_buf(),
            },
            ecosystem: Ecosystem::Node,
            kind,
            path: Some(path),
            estimated_bytes: 1,
            size_complete: true,
            last_modified: None,
            risk,
            reversible: true,
            selected_by_default,
            evidence: vec![Evidence::RuleMatched {
                rule_id: rule_id.to_string(),
            }],
            intent: CleanupIntent::TrashProjectArtifact {
                rule_id: rule_id.to_string(),
            },
        }
    }
}
