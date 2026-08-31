//! Supporting Protection, Rules, and History destinations. These are not a sixth mode.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::text::Line;

use crate::i18n::{Locale, catalogue};
use devsweep_core::{
    execution::{ProtectionMutationAction, UserProtectionList},
    history::{HistoryListOptions, HistoryListV1, list_history, show_history},
    rules::{RuleProjectionV1, rule_projections},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SupportKind {
    Menu,
    Protection,
    Rules,
    History,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SupportConfirm {
    Add(String),
    Remove(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SupportOverlay {
    pub kind: SupportKind,
    pub query: String,
    pub selected: usize,
    pub filter_active: bool,
    pub protection: Result<Vec<String>, String>,
    pub rules: Vec<RuleProjectionV1>,
    pub history: Result<HistoryListV1, String>,
    pub detail: Option<String>,
    pub confirm: Option<SupportConfirm>,
    pub add_input: Option<String>,
    pub status: Option<String>,
}

impl SupportOverlay {
    pub(super) fn menu() -> Self {
        Self {
            kind: SupportKind::Menu,
            query: String::new(),
            selected: 0,
            filter_active: false,
            protection: Ok(Vec::new()),
            rules: Vec::new(),
            history: Ok(HistoryListV1 {
                operations: Vec::new(),
                stores: Vec::new(),
            }),
            detail: None,
            confirm: None,
            add_input: None,
            status: None,
        }
    }

    #[cfg(test)]
    pub(super) fn open(kind: SupportKind) -> Self {
        let mut overlay = Self::menu();
        overlay.kind = kind;
        overlay.reload();
        overlay
    }

    fn reload(&mut self) {
        self.protection = UserProtectionList::load()
            .map(|list| {
                list.list()
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect()
            })
            .map_err(|error| error.to_string());
        self.rules = rule_projections();
        self.history =
            list_history(HistoryListOptions::default()).map_err(|error| error.to_string());
        self.selected = 0;
        self.detail = None;
    }

    fn filtered_paths(&self) -> Vec<String> {
        let Ok(paths) = &self.protection else {
            return Vec::new();
        };
        if self.query.is_empty() {
            return paths.clone();
        }
        let needle = self.query.to_ascii_lowercase();
        paths
            .iter()
            .filter(|path| path.to_ascii_lowercase().contains(&needle))
            .cloned()
            .collect()
    }

    fn filtered_rules(&self) -> Vec<RuleProjectionV1> {
        if self.query.is_empty() {
            return self.rules.clone();
        }
        let needle = self.query.to_ascii_lowercase();
        self.rules
            .iter()
            .filter(|rule| {
                rule.id.to_ascii_lowercase().contains(&needle)
                    || rule.rationale.to_ascii_lowercase().contains(&needle)
            })
            .cloned()
            .collect()
    }

    fn filtered_operations(&self) -> Vec<(String, String, String)> {
        let Ok(listed) = &self.history else {
            return Vec::new();
        };
        let needle = self.query.to_ascii_lowercase();
        listed
            .operations
            .iter()
            .filter(|operation| {
                needle.is_empty()
                    || operation
                        .operation_id
                        .to_ascii_lowercase()
                        .contains(&needle)
                    || operation
                        .outcome_code
                        .to_ascii_lowercase()
                        .contains(&needle)
            })
            .map(|operation| {
                (
                    operation.domain.as_str().to_string(),
                    operation.operation_id.clone(),
                    operation.outcome_code.clone(),
                )
            })
            .collect()
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent, locale: Locale) -> bool {
        if let Some(confirm) = self.confirm.clone() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    self.confirm = None;
                    match confirm {
                        SupportConfirm::Add(path) => {
                            self.commit_mutation(ProtectionMutationAction::Add, &path, locale)
                        }
                        SupportConfirm::Remove(path) => {
                            self.commit_mutation(ProtectionMutationAction::Remove, &path, locale)
                        }
                    }
                }
                KeyCode::Esc | KeyCode::Char('n') => self.confirm = None,
                _ => {}
            }
            return false;
        }
        if let Some(input) = &mut self.add_input {
            match key.code {
                KeyCode::Esc => self.add_input = None,
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Enter => {
                    let path = input.clone();
                    self.add_input = None;
                    if !path.trim().is_empty() {
                        self.confirm = Some(SupportConfirm::Add(path));
                    }
                }
                KeyCode::Char(ch) if !ch.is_control() => input.push(ch),
                _ => {}
            }
            return false;
        }
        if self.filter_active {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.filter_active = false,
                KeyCode::Backspace => {
                    self.query.pop();
                    self.selected = 0;
                }
                KeyCode::Char(ch) if !ch.is_control() => {
                    self.query.push(ch);
                    self.selected = 0;
                }
                _ => {}
            }
            return false;
        }
        match (self.kind, key.code) {
            (SupportKind::Menu, KeyCode::Char('1') | KeyCode::Char('p')) => {
                self.kind = SupportKind::Protection;
                self.reload();
            }
            (SupportKind::Menu, KeyCode::Char('2') | KeyCode::Char('r')) => {
                self.kind = SupportKind::Rules;
                self.reload();
            }
            (SupportKind::Menu, KeyCode::Char('3') | KeyCode::Char('h')) => {
                self.kind = SupportKind::History;
                self.reload();
            }
            (_, KeyCode::Esc | KeyCode::Char('q')) if self.kind != SupportKind::Menu => {
                self.kind = SupportKind::Menu;
            }
            (_, KeyCode::Esc | KeyCode::Char('q')) => return true,
            (SupportKind::Menu, KeyCode::Enter) => return true,
            (_, KeyCode::Char('/')) => self.filter_active = true,
            (_, KeyCode::Down | KeyCode::Char('j')) => {
                self.selected = self.selected.saturating_add(1)
            }
            (_, KeyCode::Up | KeyCode::Char('k')) => {
                self.selected = self.selected.saturating_sub(1)
            }
            (SupportKind::Protection, KeyCode::Char('a')) => self.add_input = Some(String::new()),
            (SupportKind::Protection, KeyCode::Char('d') | KeyCode::Delete) => {
                if let Some(path) = self.filtered_paths().get(self.selected).cloned() {
                    self.confirm = Some(SupportConfirm::Remove(path));
                }
            }
            (SupportKind::History, KeyCode::Enter) => self.show_selected_history(),
            _ => {}
        }
        false
    }

    fn commit_mutation(&mut self, action: ProtectionMutationAction, path: &str, locale: Locale) {
        let result = UserProtectionList::load().and_then(|mut list| match action {
            ProtectionMutationAction::Add => list.add(std::path::Path::new(path)),
            ProtectionMutationAction::Remove => list.remove(std::path::Path::new(path)),
        });
        self.status = Some(match result {
            Ok(report) => {
                let key = match (action, report.changed) {
                    (ProtectionMutationAction::Add, _) => "protect.v1.add.committed",
                    (ProtectionMutationAction::Remove, true) => "protect.v1.remove.committed",
                    (ProtectionMutationAction::Remove, false) => "protect.v1.remove.unchanged",
                };
                let values: Vec<(&str, &str)> = if key == "protect.v1.remove.unchanged" {
                    Vec::new()
                } else {
                    vec![("id", report.operation_id.as_str())]
                };
                catalogue(locale)
                    .render(key, &values, None)
                    .unwrap_or_else(|_| report.operation_id.clone())
            }
            Err(error) => error.to_string(),
        });
        self.reload();
    }

    fn show_selected_history(&mut self) {
        let Some((_, id, _)) = self.filtered_operations().get(self.selected).cloned() else {
            return;
        };
        self.detail = Some(match show_history(&id) {
            Ok(detail) => {
                let encoded = serde_json::to_string(&detail).unwrap_or_default();
                if encoded.contains("\"argv\"") || encoded.contains("\"path\"") {
                    "redacted".to_string()
                } else {
                    format!(
                        "{} {} {}",
                        detail.summary.domain.as_str(),
                        detail.summary.operation_id,
                        detail.summary.outcome_code
                    )
                }
            }
            Err(error) => error.to_string(),
        });
    }

    pub(super) fn lines(&self, locale: Locale) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        match self.kind {
            SupportKind::Menu => {
                lines.push(Line::from(copy(locale, "support.v1.menu")));
                lines.push(Line::from(copy(locale, "shell.v1.supporting.protection")));
                lines.push(Line::from(copy(locale, "shell.v1.supporting.rules")));
                lines.push(Line::from(copy(locale, "shell.v1.supporting.history")));
            }
            SupportKind::Protection => {
                lines.push(Line::from(copy(locale, "shell.v1.supporting.protection")));
                match &self.protection {
                    Err(_) => lines.push(Line::from(copy(locale, "protect.v1.store.unavailable"))),
                    Ok(paths) if paths.is_empty() => {
                        lines.push(Line::from(copy(locale, "protect.v1.empty")))
                    }
                    Ok(_) => {
                        let filtered = self.filtered_paths();
                        lines.push(Line::from(copy_count(
                            locale,
                            "protect.v1.list.summary",
                            filtered.len(),
                        )));
                        for (index, path) in filtered.iter().enumerate() {
                            let marker = if index == self.selected { ">" } else { " " };
                            lines.push(Line::from(format!("{marker} {path}")));
                        }
                    }
                }
                lines.push(Line::from(format!(
                    "{} / {}",
                    copy(locale, "protect.v1.action.add"),
                    copy(locale, "protect.v1.action.remove")
                )));
                if let Some(input) = &self.add_input {
                    lines.push(Line::from(format!(
                        "{}: {input}",
                        copy(locale, "protect.v1.input.path")
                    )));
                }
                if let Some(SupportConfirm::Add(_) | SupportConfirm::Remove(_)) = &self.confirm {
                    let key = match &self.confirm {
                        Some(SupportConfirm::Add(_)) => "protect.v1.confirm.add",
                        _ => "protect.v1.confirm.remove",
                    };
                    lines.push(Line::from(copy(locale, key)));
                }
            }
            SupportKind::Rules => {
                lines.push(Line::from(copy(locale, "rules.v1.inspect_only")));
                let filtered = self.filtered_rules();
                lines.push(Line::from(copy_count(
                    locale,
                    "rules.v1.list.summary",
                    filtered.len(),
                )));
                for (index, rule) in filtered.iter().enumerate() {
                    let marker = if index == self.selected { ">" } else { " " };
                    lines.push(Line::from(format!(
                        "{marker} {} | {}",
                        rule.id, rule.safety_class
                    )));
                }
            }
            SupportKind::History => {
                lines.push(Line::from(copy(locale, "history.v1.no_replay")));
                match &self.history {
                    Err(_) => lines.push(Line::from(copy_domain(
                        locale,
                        "history.v1.store.unavailable",
                        "clean",
                    ))),
                    Ok(listed) if listed.operations.is_empty() => {
                        lines.push(Line::from(copy(locale, "history.v1.empty")))
                    }
                    Ok(_) => {
                        let filtered = self.filtered_operations();
                        lines.push(Line::from(copy_count(
                            locale,
                            "history.v1.list.summary",
                            filtered.len(),
                        )));
                        for (index, (domain, id, outcome)) in filtered.iter().enumerate() {
                            let marker = if index == self.selected { ">" } else { " " };
                            lines.push(Line::from(format!("{marker} {domain} | {id} | {outcome}")));
                        }
                    }
                }
                if let Some(detail) = &self.detail {
                    lines.push(Line::from(detail.clone()));
                }
            }
        }
        if let Some(status) = &self.status {
            lines.push(Line::from(status.clone()));
        }
        lines
    }

    pub(super) fn title(&self, locale: Locale) -> &'static str {
        catalogue(locale)
            .static_text(match self.kind {
                SupportKind::Menu => "shell.v1.supporting",
                SupportKind::Protection => "shell.v1.supporting.protection",
                SupportKind::Rules => "shell.v1.supporting.rules",
                SupportKind::History => "shell.v1.supporting.history",
            })
            .unwrap_or("Supporting destinations")
    }
}

fn copy(locale: Locale, key: &str) -> String {
    catalogue(locale)
        .render(key, &[], None)
        .unwrap_or_else(|_| key.to_string())
}

fn copy_count(locale: Locale, key: &str, count: usize) -> String {
    let count = count.to_string();
    catalogue(locale)
        .render(key, &[("count", count.as_str())], None)
        .unwrap_or_else(|_| key.to_string())
}

fn copy_domain(locale: Locale, key: &str, domain: &str) -> String {
    catalogue(locale)
        .render(key, &[("domain", domain)], None)
        .unwrap_or_else(|_| key.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::{
        app::{App, Overlay, UiEvent},
        test_support::{key, render_text},
    };
    use crossterm::event::KeyCode;

    #[test]
    fn support_overlay_is_not_a_sixth_mode_and_history_cannot_replay() {
        let mut app = App::new();
        app.update(key(KeyCode::Char('?')));
        assert!(matches!(app.overlay, Overlay::Help));
        app.update(key(KeyCode::Char('o')));
        let Overlay::Support(support) = &app.overlay else {
            panic!("support menu");
        };
        assert_eq!(support.kind, SupportKind::Menu);
        app.update(key(KeyCode::Char('3')));
        let Overlay::Support(support) = &app.overlay else {
            panic!("history");
        };
        assert_eq!(support.kind, SupportKind::History);
        let lines = support
            .lines(crate::i18n::Locale::En)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
            .to_ascii_lowercase();
        assert!(lines.contains("cannot execute or replay") || lines.contains("inspect-only"));
        assert!(!lines.contains("run this operation"));
        let text = render_text(&app);
        assert!(text.contains("History") || text.contains("history"));
    }

    #[test]
    fn help_overflow_opens_support_from_analyze_without_a_sixth_mode() {
        let mut app = App::new();
        app.update(UiEvent::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('a'),
            crossterm::event::KeyModifiers::ALT,
        )));
        assert_eq!(app.shell.active, crate::tui::shell::ModeId::Analyze);
        assert_eq!(app.shell.navigation.len(), 5);
        app.update(key(KeyCode::Char('?')));
        assert!(matches!(app.overlay, Overlay::Help));
        app.update(key(KeyCode::Char('o')));
        assert!(matches!(app.overlay, Overlay::Support(_)));
        assert_eq!(app.shell.active, crate::tui::shell::ModeId::Analyze);
        assert_eq!(app.shell.navigation.len(), 5);
    }

    #[test]
    fn support_overlay_renders_chinese_protection_copy() {
        let overlay = SupportOverlay::open(SupportKind::Protection);
        let lines = overlay.lines(Locale::ZhCn);
        let text = lines
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("保护") || text.contains("路径"));
    }
}
