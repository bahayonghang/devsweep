//! Bilingual history human renderer over redacted, non-replayable records.

use super::render;
use crate::i18n::{CatalogueError, Locale};
use devsweep_core::history::{
    HistoryDetailV1, HistoryListV1, HistoryRecordV1, HistoryStoreState, HistoryStoreStatusV1,
};

#[cfg(test)]
pub(super) struct Route;

pub(crate) fn list(locale: Locale, listed: &HistoryListV1) -> Result<String, CatalogueError> {
    let count = listed.operations.len().to_string();
    let mut lines = Vec::new();
    if listed.operations.is_empty() {
        lines.push(render(locale, "history.v1.empty", &[], None)?);
    } else {
        lines.push(render(
            locale,
            "history.v1.list.summary",
            &[("count", count.as_str())],
            None,
        )?);
        for operation in &listed.operations {
            lines.push(render(
                locale,
                "history.v1.list.entry",
                &[
                    ("domain", operation.domain.as_str()),
                    ("id", operation.operation_id.as_str()),
                    ("outcome", operation.outcome_code.as_str()),
                ],
                None,
            )?);
        }
    }
    lines.extend(store_lines(locale, &listed.stores)?);
    lines.push(render(locale, "history.v1.no_replay", &[], None)?);
    Ok(lines.join("\n"))
}

pub(crate) fn show(locale: Locale, detail: &HistoryDetailV1) -> Result<String, CatalogueError> {
    let mut lines = vec![
        render(
            locale,
            "history.v1.show.header",
            &[
                ("id", detail.summary.operation_id.as_str()),
                ("domain", detail.summary.domain.as_str()),
                ("outcome", detail.summary.outcome_code.as_str()),
            ],
            None,
        )?,
        render(locale, "history.v1.no_replay", &[], None)?,
    ];
    for record in &detail.records {
        let (kind, code) = record_kind_and_code(record);
        lines.push(render(
            locale,
            "history.v1.record",
            &[("kind", kind), ("code", code.as_str())],
            None,
        )?);
    }
    Ok(lines.join("\n"))
}

fn store_lines(
    locale: Locale,
    stores: &[HistoryStoreStatusV1],
) -> Result<Vec<String>, CatalogueError> {
    let mut lines = Vec::new();
    for store in stores {
        if matches!(
            store.state,
            HistoryStoreState::Unavailable | HistoryStoreState::Unsupported
        ) {
            let key = if store.state == HistoryStoreState::Unsupported {
                "history.v1.unsupported"
            } else {
                "history.v1.store.unavailable"
            };
            lines.push(render(
                locale,
                key,
                &[("domain", store.domain.as_str())],
                None,
            )?);
        }
    }
    Ok(lines)
}

fn record_kind_and_code(record: &HistoryRecordV1) -> (&'static str, String) {
    match record {
        HistoryRecordV1::Clean { .. } => ("clean", "redacted".to_string()),
        HistoryRecordV1::Software { record } => (
            "software",
            serde_json::to_value(record.status_code)
                .ok()
                .and_then(|value| value.as_str().map(str::to_string))
                .unwrap_or_else(|| "unknown".to_string()),
        ),
        HistoryRecordV1::SoftwareSupport { record } => (
            "software",
            serde_json::to_value(record.outcome_code())
                .ok()
                .and_then(|value| value.as_str().map(str::to_string))
                .unwrap_or_else(|| "unknown".to_string()),
        ),
        HistoryRecordV1::Optimize { record } => (
            "optimize",
            serde_json::to_value(record.status_code)
                .ok()
                .and_then(|value| value.as_str().map(str::to_string))
                .unwrap_or_else(|| "unknown".to_string()),
        ),
        HistoryRecordV1::Unsupported { reason_code, .. } => {
            ("unsupported", (*reason_code).to_string())
        }
    }
}
