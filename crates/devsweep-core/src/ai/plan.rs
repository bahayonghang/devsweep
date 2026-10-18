use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{
    AiClass, AiDisposition, AiError, AiObservationV1, AiPlanKind, AiPlanTarget, AiPlanV1,
    AiPreviewV1, AiRefused, SCHEMA_OBSERVATION, SCHEMA_PLAN, SCHEMA_PREVIEW, is_real_dir,
    normalize_home, sha256_hex,
};
use crate::filesystem::{path_is_within, paths_equal};

/// Build a cleanup plan from one present provider. Does not mutate the home.
pub fn plan_ai(observation: &AiObservationV1, class: AiClass) -> Result<AiPlanV1, AiError> {
    if observation.version != 1 || observation.schema != SCHEMA_OBSERVATION {
        return Err(AiError::usage("invalid_plan", "unsupported AI observation"));
    }
    if class == AiClass::InstalledTool {
        return Err(AiError::usage(
            "class_not_selectable",
            "installed_tool is never a cleanup target",
        ));
    }
    let present = observation
        .providers
        .iter()
        .filter(|provider| provider.state == "present")
        .collect::<Vec<_>>();
    if present.is_empty() {
        return Err(AiError::unavailable(
            "source_unavailable",
            "AI home is absent",
        ));
    }
    if present.len() != 1 {
        return Err(AiError::usage(
            "ambiguous_provider",
            "scan one provider before planning",
        ));
    }
    let provider = present[0];
    if class == AiClass::SessionArchive && provider.session_bounded {
        return Err(AiError::partial(
            "incomplete_observation",
            "session enumeration stopped at the bound",
        ));
    }
    let home = provider
        .home
        .clone()
        .ok_or_else(|| AiError::unavailable("source_unavailable", "AI home is absent"))?;
    let mut targets = Vec::new();
    let mut refused = Vec::new();
    for row in &provider.rows {
        if row.class != class {
            continue;
        }
        if row.disposition == AiDisposition::Candidate {
            let Some(content_sha256) = row.content_sha256.clone() else {
                refused.push(AiRefused {
                    relative_path: row.relative_path.clone(),
                    reason: "unreadable".to_string(),
                });
                continue;
            };
            targets.push(AiPlanTarget {
                id: row.id.clone(),
                relative_path: row.relative_path.clone(),
                content_sha256,
                action: "move_to_trash".to_string(),
            });
        } else {
            refused.push(AiRefused {
                relative_path: row.relative_path.clone(),
                reason: row.reason.clone(),
            });
        }
    }
    targets.sort_by(|left, right| left.id.cmp(&right.id));
    refused.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(AiPlanV1 {
        schema: SCHEMA_PLAN.to_string(),
        version: 1,
        kind: AiPlanKind::Cleanup,
        provider: provider.provider,
        home,
        class,
        bundle_id: None,
        targets,
        refused,
    })
}

/// Recompute the live digest. Does not mutate the home.
pub fn preview_ai(plan: &AiPlanV1) -> Result<AiPreviewV1, AiError> {
    validate_plan_shape(plan)?;
    let home = PathBuf::from(&plan.home);
    if !is_real_dir(&home) {
        return Err(AiError::unavailable(
            "source_unavailable",
            "AI home is unavailable",
        ));
    }
    let mut targets = Vec::with_capacity(plan.targets.len());
    for target in &plan.targets {
        let content_sha256 = match plan.kind {
            AiPlanKind::Cleanup => {
                let path = resolve_inside(&home, &target.relative_path)?;
                super::scan::content_hash(&path).map_err(|_| {
                    AiError::unavailable(
                        "source_unavailable",
                        format!("AI target is unavailable: {}", target.relative_path),
                    )
                })?
            }
            AiPlanKind::Restore => super::execute::live_bundle_hash(plan)?,
        };
        targets.push(AiPlanTarget {
            id: target.id.clone(),
            relative_path: target.relative_path.clone(),
            content_sha256,
            action: target.action.clone(),
        });
    }
    targets.sort_by(|left, right| left.id.cmp(&right.id));
    let index_sha256 = match plan.kind {
        AiPlanKind::Cleanup => super::scan::index_hash(&home, plan.class)?,
        AiPlanKind::Restore => super::execute::restore_index(plan)?,
    };
    let digest = live_digest(plan, &index_sha256, &targets)?;
    Ok(AiPreviewV1 {
        schema: SCHEMA_PREVIEW.to_string(),
        version: 1,
        kind: plan.kind,
        provider: plan.provider,
        home: plan.home.clone(),
        class: plan.class,
        digest,
        targets,
        refused: plan.refused.clone(),
    })
}

pub(crate) fn validate_plan_shape(plan: &AiPlanV1) -> Result<(), AiError> {
    if plan.version != 1 || plan.schema != SCHEMA_PLAN {
        return Err(AiError::usage("invalid_plan", "unsupported AI plan"));
    }
    if plan.home.is_empty() {
        return Err(AiError::usage(
            "invalid_plan",
            "AI plan is missing its home",
        ));
    }
    let home = PathBuf::from(&plan.home);
    if normalize_home(&home).ok().as_deref() != Some(plan.home.as_str()) {
        return Err(AiError::usage(
            "invalid_plan",
            "AI plan home is not canonical",
        ));
    }
    if plan.kind == AiPlanKind::Restore && plan.bundle_id.is_none() {
        return Err(AiError::usage(
            "invalid_plan",
            "restore plan is missing its bundle",
        ));
    }
    for target in &plan.targets {
        resolve_inside(&home, &target.relative_path)?;
        if plan.kind == AiPlanKind::Cleanup && target.action != "move_to_trash" {
            return Err(AiError::usage("invalid_plan", "unsupported AI action"));
        }
        if plan.kind == AiPlanKind::Restore && target.action != "restore_bytes" {
            return Err(AiError::usage("invalid_plan", "unsupported AI action"));
        }
    }
    Ok(())
}

pub(crate) fn resolve_inside(home: &Path, relative: &str) -> Result<PathBuf, AiError> {
    if relative.is_empty() || relative.contains('\0') || relative.contains('\\') {
        return Err(AiError::usage("invalid_plan", "AI path is not relative"));
    }
    let mut path = home.to_path_buf();
    for segment in relative.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(AiError::usage("invalid_plan", "AI path escapes its home"));
        }
        path.push(segment);
    }
    if paths_equal(home, &path) || !path_is_within(home, &path) {
        return Err(AiError::usage("invalid_plan", "AI path escapes its home"));
    }
    Ok(path)
}

fn live_digest(
    plan: &AiPlanV1,
    index_sha256: &str,
    targets: &[AiPlanTarget],
) -> Result<String, AiError> {
    #[derive(Serialize)]
    struct Body<'a> {
        version: u32,
        kind: &'a str,
        provider: &'a str,
        home: &'a str,
        class: &'a str,
        bundle_id: Option<&'a str>,
        index_sha256: &'a str,
        targets: Vec<Target<'a>>,
    }
    #[derive(Serialize)]
    struct Target<'a> {
        id: &'a str,
        relative_path: &'a str,
        action: &'a str,
        content_sha256: &'a str,
    }
    let body = Body {
        version: 1,
        kind: plan.kind.as_str(),
        provider: plan.provider.as_str(),
        home: &plan.home,
        class: plan.class.as_str(),
        bundle_id: plan.bundle_id.as_deref(),
        index_sha256,
        targets: targets
            .iter()
            .map(|target| Target {
                id: &target.id,
                relative_path: &target.relative_path,
                action: &target.action,
                content_sha256: &target.content_sha256,
            })
            .collect(),
    };
    let bytes = serde_json::to_vec(&body).map_err(|error| {
        AiError::failed(
            "invalid_plan",
            format!("failed to encode the AI digest: {error}"),
        )
    })?;
    Ok(format!("sha256:{}", sha256_hex(&bytes)))
}

pub(crate) fn still_candidate(
    plan: &AiPlanV1,
    cancel: &dyn crate::process::CancelObserver,
) -> Result<(), AiError> {
    let home = PathBuf::from(&plan.home);
    let mut rows = Vec::new();
    super::scan::classify_home(&home, plan.provider, cancel, &mut rows)?;
    for target in &plan.targets {
        let candidate = rows.iter().any(|row| {
            row.relative_path == target.relative_path
                && row.class == plan.class
                && row.disposition == AiDisposition::Candidate
        });
        if !candidate {
            return Err(AiError::stale(
                "changed_identity",
                format!(
                    "AI target is no longer selectable: {}",
                    target.relative_path
                ),
            ));
        }
    }
    Ok(())
}
