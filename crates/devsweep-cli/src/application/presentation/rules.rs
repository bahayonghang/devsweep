//! Bilingual rules human renderer over the shipped inspect-only projection.

use super::render;
use crate::i18n::{CatalogueError, Locale};
use devsweep_core::rules::RuleProjectionV1;

#[cfg(test)]
pub(super) struct Route;

pub(crate) fn list(locale: Locale, rules: &[RuleProjectionV1]) -> Result<String, CatalogueError> {
    let count = rules.len().to_string();
    if rules.is_empty() {
        return render(locale, "rules.v1.empty", &[], None);
    }
    let mut lines = vec![
        render(
            locale,
            "rules.v1.list.summary",
            &[("count", count.as_str())],
            None,
        )?,
        render(locale, "rules.v1.inspect_only", &[], None)?,
    ];
    for rule in rules {
        let risk = serde_json::to_value(&rule.risk)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_else(|| "unknown".to_string());
        lines.push(render(
            locale,
            "rules.v1.list.entry",
            &[
                ("id", rule.id.as_str()),
                ("safety", rule.safety_class),
                ("risk", risk.as_str()),
                ("summary", rule.rationale.as_str()),
            ],
            None,
        )?);
    }
    Ok(lines.join("\n"))
}

pub(crate) fn show(locale: Locale, rule: &RuleProjectionV1) -> Result<String, CatalogueError> {
    Ok([
        render(
            locale,
            "rules.v1.show.header",
            &[
                ("id", rule.id.as_str()),
                ("safety", rule.safety_class),
                ("scope", rule.scope),
                ("platform", rule.platform_applicability),
            ],
            None,
        )?,
        render(locale, "rules.v1.source.shipped", &[], None)?,
        render(locale, "rules.v1.inspect_only", &[], None)?,
        rule.rationale.clone(),
    ]
    .join("\n"))
}
