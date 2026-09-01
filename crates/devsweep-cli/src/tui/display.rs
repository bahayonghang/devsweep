use std::path::{Path, PathBuf};

use crate::model::{CleanAction, CleanTarget, Scope, TargetId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CommandPreview {
    pub(super) target: String,
    pub(super) command: String,
}

pub(super) fn target_title(target: &CleanTarget) -> String {
    target
        .path
        .as_ref()
        .map(|path| compact_path(path))
        .unwrap_or_else(|| display_path_text(target.id.as_str()))
}

pub(super) fn compact_path(path: &Path) -> String {
    compact_text(&display_path(path), 48)
}

pub(super) fn compact_text(text: &str, max_cells: usize) -> String {
    let cleaned = sanitize_display_text(text);
    truncate_to_width(&cleaned, max_cells)
}

pub(super) fn compact_context_text(text: &str, max_cells: usize) -> String {
    let cleaned = sanitize_display_text(text);
    if display_width(&cleaned) <= max_cells {
        return cleaned;
    }

    let prefix_budget = max_cells.min(12);
    let prefix = truncate_to_width(&cleaned, prefix_budget);
    let tail_budget = max_cells.saturating_sub(display_width(&prefix) + 3);
    let tail = take_width_suffix(&cleaned, tail_budget);
    format!("{prefix}...{tail}")
}

pub(super) fn display_width(text: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(text)
}

fn truncate_to_width(text: &str, max_cells: usize) -> String {
    if display_width(text) <= max_cells {
        return text.to_string();
    }
    if max_cells <= 3 {
        return ".".repeat(max_cells.min(3));
    }
    let tail_budget = max_cells.saturating_sub(3);
    let tail = take_width_suffix(text, tail_budget);
    format!("...{tail}")
}

fn take_width_suffix(text: &str, max_cells: usize) -> String {
    let mut width = 0usize;
    let mut chars = Vec::new();
    for ch in text.chars().rev() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width > max_cells {
            break;
        }
        width += ch_width;
        chars.push(ch);
    }
    chars.into_iter().rev().collect()
}

pub(super) fn sanitize_display_text(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            '\n' | '\r' | '\t' => ' ',
            c if c.is_control() || c == '\u{7f}' => '?',
            c => c,
        })
        .collect()
}

fn quote_argv_part(part: &str) -> String {
    let cleaned = sanitize_display_text(part);
    if cleaned.is_empty()
        || cleaned.chars().any(|ch| {
            ch.is_whitespace()
                || matches!(ch, '"' | '\'' | '\\' | '*' | '?' | '|' | '&' | ';' | '>')
        })
    {
        format!("\"{}\"", cleaned.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        cleaned
    }
}

pub(super) fn scope_label(scope: &Scope) -> String {
    match scope {
        Scope::Global => "Global".to_string(),
        Scope::Project { root } => format!("Project ({})", display_path(root)),
    }
}

pub(super) fn path_label(path: Option<&PathBuf>) -> String {
    path.map(|path| display_path(path))
        .unwrap_or_else(|| "none".to_string())
}

pub(super) fn selected_target_summary(target: &CleanTarget) -> String {
    format!(
        "{} | {} | {}",
        scope_label(&target.scope),
        target_title(target),
        path_label(target.path.as_ref())
    )
}

pub(super) fn action_summary(action: &CleanAction) -> String {
    match action {
        CleanAction::Command {
            program,
            args,
            irreversible,
            cwd,
        } => {
            let suffix = if *irreversible { " irreversible" } else { "" };
            format!("command{}: {}", suffix, command_preview(program, args, cwd))
        }
        CleanAction::MoveToTrash { path } => format!("trash: {}", display_path(path)),
        CleanAction::DeletePermanently { path, .. } => {
            format!("permanent delete disabled: {}", display_path(path))
        }
        CleanAction::NoopInspectOnly => "inspect only".to_string(),
    }
}

pub(super) fn command_previews<'a>(
    targets: impl IntoIterator<Item = &'a CleanTarget>,
) -> Vec<CommandPreview> {
    targets
        .into_iter()
        .filter_map(|target| match &target.action {
            CleanAction::Command {
                program,
                args,
                cwd,
                irreversible: _,
            } => Some(CommandPreview {
                target: target_title(target),
                command: command_preview(program, args, cwd),
            }),
            CleanAction::MoveToTrash { .. }
            | CleanAction::DeletePermanently { .. }
            | CleanAction::NoopInspectOnly => None,
        })
        .collect()
}

pub(super) fn command_preview(program: &str, args: &[String], cwd: &Option<PathBuf>) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(quote_argv_part(&display_path_text(program)));
    parts.extend(
        args.iter()
            .map(|arg| quote_argv_part(&display_path_text(arg))),
    );
    let argv = parts.join(" ");
    if let Some(cwd) = cwd {
        format!("argv: {argv}  cwd: {}", display_path(cwd))
    } else {
        format!("argv: {argv}")
    }
}

pub(super) fn display_path(path: &Path) -> String {
    sanitize_display_text(&display_path_text(&path.display().to_string()))
}

pub(super) fn display_path_text(text: &str) -> String {
    text.replace("\\\\?\\UNC\\", "\\\\").replace("\\\\?\\", "")
}

pub(super) fn compact_target_id(target_id: &TargetId) -> String {
    compact_text(&display_path_text(target_id.as_str()), 48)
}

pub(super) fn format_cleanup_progress(completed: usize, total: usize, message: &str) -> String {
    format!("{completed} / {total} {message}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_preview_quotes_and_sanitizes_argv() {
        let preview = command_preview(
            "C:\\Program Files\\tool.exe",
            &["path with space".to_string(), "a\u{1b}[31mb".to_string()],
            &None,
        );

        assert!(preview.contains("\"C:\\\\Program Files\\\\tool.exe\""));
        assert!(preview.contains("\"path with space\""));
        assert!(!preview.contains('\u{1b}'));
    }

    #[test]
    fn display_path_removes_windows_verbatim_prefixes() {
        assert_eq!(
            display_path(Path::new("\\\\?\\D:\\code\\devsweep\\target")),
            "D:\\code\\devsweep\\target"
        );
        assert_eq!(
            display_path(Path::new("\\\\?\\UNC\\server\\share\\cache")),
            "\\\\server\\share\\cache"
        );
    }
}
