//! Bilingual protection human renderer over typed support outcomes.

use super::render;
use crate::i18n::{CatalogueError, Locale};
use devsweep_core::execution::{ProtectionMutationAction, ProtectionMutationReport};

#[cfg(test)]
pub(super) struct Route;

pub(crate) fn list(locale: Locale, paths: &[String]) -> Result<String, CatalogueError> {
    let count = paths.len().to_string();
    if paths.is_empty() {
        return render(locale, "protect.v1.empty", &[], None);
    }
    let mut lines = vec![render(
        locale,
        "protect.v1.list.summary",
        &[("count", count.as_str())],
        None,
    )?];
    for path in paths {
        lines.push(render(
            locale,
            "protect.v1.list.entry",
            &[("path", path.as_str())],
            None,
        )?);
    }
    Ok(lines.join("\n"))
}

pub(crate) fn mutation(
    locale: Locale,
    report: &ProtectionMutationReport,
) -> Result<String, CatalogueError> {
    let key = match (report.action, report.changed) {
        (ProtectionMutationAction::Add, _) => "protect.v1.add.committed",
        (ProtectionMutationAction::Remove, true) => "protect.v1.remove.committed",
        (ProtectionMutationAction::Remove, false) => "protect.v1.remove.unchanged",
    };
    match key {
        "protect.v1.remove.unchanged" => render(locale, key, &[], None),
        _ => render(locale, key, &[("id", report.operation_id.as_str())], None),
    }
}
