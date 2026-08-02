use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    model::{CLEANUP_PLAN_VERSION, CleanAction, Evidence, Scope},
    path_identity::normalize_absolute_path,
};

use super::ValidatedTarget;

pub(super) fn digest_for_targets(targets: &[ValidatedTarget]) -> Result<String> {
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
