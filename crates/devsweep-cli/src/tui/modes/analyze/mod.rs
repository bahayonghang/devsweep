//! Read-only Analyze hierarchy state and bounded terminal projections.

use std::collections::HashMap;

use devsweep_core::analysis::{
    AnalyzeCompleteness, AnalyzeEvidence, AnalyzeNodeKind, AnalyzeNodeV1, AnalyzeProgressV1,
    AnalyzeRunOutcome, AnalyzeSnapshotV1,
};

use crate::tui::app::JobId;

pub(in crate::tui) const PAGE_SIZE: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::tui) enum AnalyzePhase {
    Idle,
    Loading,
    Canceling,
    Complete,
    Partial,
    Canceled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::tui) enum AnalyzeSort {
    SizeDescending,
    SizeAscending,
    Name,
    Kind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::tui) struct AnalyzeModeState {
    pub(in crate::tui) phase: AnalyzePhase,
    pub(in crate::tui) operation_id: Option<JobId>,
    pub(in crate::tui) last_sequence: u64,
    pub(in crate::tui) stored_nodes: u32,
    pub(in crate::tui) accounted_owned_bytes: u64,
    pub(in crate::tui) snapshot: Option<AnalyzeSnapshotV1>,
    pub(in crate::tui) current_node_id: u32,
    pub(in crate::tui) page: usize,
    pub(in crate::tui) cursor: usize,
    pub(in crate::tui) query: String,
    pub(in crate::tui) sort: AnalyzeSort,
    pub(in crate::tui) focus_anchors: HashMap<u32, u32>,
    pub(in crate::tui) error: Option<String>,
}

impl Default for AnalyzeModeState {
    fn default() -> Self {
        Self {
            phase: AnalyzePhase::Idle,
            operation_id: None,
            last_sequence: 0,
            stored_nodes: 0,
            accounted_owned_bytes: 0,
            snapshot: None,
            current_node_id: 0,
            page: 0,
            cursor: 0,
            query: String::new(),
            sort: AnalyzeSort::SizeDescending,
            focus_anchors: HashMap::new(),
            error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::tui) enum AnalyzeAction {
    Started {
        operation_id: JobId,
    },
    Progressed {
        operation_id: JobId,
        progress: AnalyzeProgressV1,
    },
    CancelRequested {
        operation_id: JobId,
    },
    Finished {
        operation_id: JobId,
        outcome: AnalyzeRunOutcome,
    },
    Failed {
        operation_id: JobId,
        message: String,
    },
    Canceled {
        operation_id: JobId,
    },
    QueryChanged(String),
    SortChanged(AnalyzeSort),
    MoveCursor(isize),
    PageChanged(isize),
    OpenFocused,
    Up,
    Release,
}

impl AnalyzeModeState {
    pub(in crate::tui) fn reduce(&mut self, action: AnalyzeAction) {
        match action {
            AnalyzeAction::Started { operation_id } => {
                let query = std::mem::take(&mut self.query);
                let sort = self.sort;
                *self = Self {
                    phase: AnalyzePhase::Loading,
                    operation_id: Some(operation_id),
                    query,
                    sort,
                    ..Self::default()
                };
            }
            AnalyzeAction::Progressed {
                operation_id,
                progress,
            } => {
                if self.operation_id != Some(operation_id)
                    || !matches!(self.phase, AnalyzePhase::Loading | AnalyzePhase::Canceling)
                    || progress.sequence <= self.last_sequence
                {
                    return;
                }
                self.last_sequence = progress.sequence;
                self.stored_nodes = progress.stored_nodes;
                self.accounted_owned_bytes = progress.accounted_owned_bytes;
            }
            AnalyzeAction::CancelRequested { operation_id } => {
                if self.operation_id == Some(operation_id) && self.phase == AnalyzePhase::Loading {
                    self.phase = AnalyzePhase::Canceling;
                }
            }
            AnalyzeAction::Finished {
                operation_id,
                outcome,
            } => {
                if self.operation_id != Some(operation_id) {
                    return;
                }
                let snapshot = outcome.snapshot().clone();
                self.phase = match snapshot.completeness {
                    AnalyzeCompleteness::Complete => AnalyzePhase::Complete,
                    AnalyzeCompleteness::PartialBudget => AnalyzePhase::Partial,
                    AnalyzeCompleteness::Canceled => AnalyzePhase::Canceled,
                };
                self.operation_id = None;
                self.stored_nodes = snapshot.nodes.len().try_into().unwrap_or(u32::MAX);
                self.accounted_owned_bytes = snapshot.accounted_owned_bytes;
                self.snapshot = Some(snapshot);
                self.current_node_id = 0;
                self.page = 0;
                self.cursor = 0;
                self.focus_anchors.clear();
                self.error = None;
            }
            AnalyzeAction::Failed {
                operation_id,
                message,
            } => {
                if self.operation_id == Some(operation_id) {
                    self.phase = AnalyzePhase::Failed;
                    self.operation_id = None;
                    self.error = Some(message);
                }
            }
            AnalyzeAction::Canceled { operation_id } => {
                if self.operation_id == Some(operation_id) {
                    self.phase = AnalyzePhase::Canceled;
                    self.operation_id = None;
                }
            }
            AnalyzeAction::QueryChanged(query) => {
                self.query = query;
                self.page = 0;
                self.cursor = 0;
            }
            AnalyzeAction::SortChanged(sort) => {
                self.sort = sort;
                self.page = 0;
                self.cursor = 0;
            }
            AnalyzeAction::MoveCursor(delta) => self.move_cursor(delta),
            AnalyzeAction::PageChanged(delta) => self.change_page(delta),
            AnalyzeAction::OpenFocused => self.open_focused(),
            AnalyzeAction::Up => self.up(),
            AnalyzeAction::Release => {
                let query = std::mem::take(&mut self.query);
                let sort = self.sort;
                *self = Self {
                    query,
                    sort,
                    ..Self::default()
                };
            }
        }
    }

    pub(in crate::tui) fn is_active(&self) -> bool {
        matches!(self.phase, AnalyzePhase::Loading | AnalyzePhase::Canceling)
    }

    pub(in crate::tui) fn current_directory(&self) -> Option<&AnalyzeNodeV1> {
        let snapshot = self.snapshot.as_ref()?;
        snapshot
            .nodes
            .get(self.current_node_id as usize)
            .filter(|node| node.kind == AnalyzeNodeKind::Directory)
            .or_else(|| snapshot.nodes.first())
    }

    pub(in crate::tui) fn breadcrumbs(&self) -> Vec<&AnalyzeNodeV1> {
        let Some(snapshot) = &self.snapshot else {
            return Vec::new();
        };
        let mut result = Vec::new();
        let mut cursor = self.current_directory();
        while let Some(node) = cursor {
            result.push(node);
            cursor = node
                .parent_id
                .and_then(|parent| snapshot.nodes.get(parent as usize));
        }
        result.reverse();
        result
    }

    pub(in crate::tui) fn visible_children(&self) -> Vec<&AnalyzeNodeV1> {
        let Some(snapshot) = &self.snapshot else {
            return Vec::new();
        };
        let current = self.current_directory().map(|node| node.id).unwrap_or(0);
        let query = self.query.trim().to_lowercase();
        let mut children = snapshot
            .nodes
            .iter()
            .filter(|node| node.parent_id == Some(current))
            .filter(|node| query.is_empty() || node.name.to_lowercase().contains(&query))
            .collect::<Vec<_>>();
        children.sort_by(|left, right| {
            let primary = match self.sort {
                AnalyzeSort::SizeDescending => right.bytes.cmp(&left.bytes),
                AnalyzeSort::SizeAscending => left.bytes.cmp(&right.bytes),
                AnalyzeSort::Name => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
                AnalyzeSort::Kind => kind_rank(left.kind).cmp(&kind_rank(right.kind)),
            };
            primary
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
                .then_with(|| left.id.cmp(&right.id))
        });
        children
    }

    pub(in crate::tui) fn page_rows(&self) -> Vec<&AnalyzeNodeV1> {
        let visible = self.visible_children();
        let start = self.page.saturating_mul(PAGE_SIZE).min(visible.len());
        visible.into_iter().skip(start).take(PAGE_SIZE).collect()
    }

    pub(in crate::tui) fn viewport_rows(&self, height: usize) -> Vec<&AnalyzeNodeV1> {
        if height == 0 {
            return Vec::new();
        }
        let rows = self.page_rows();
        let start = self
            .cursor
            .saturating_add(1)
            .saturating_sub(height)
            .min(rows.len().saturating_sub(height));
        rows.into_iter().skip(start).take(height).collect()
    }

    pub(in crate::tui) fn focused_node(&self) -> Option<&AnalyzeNodeV1> {
        self.page_rows().get(self.cursor).copied()
    }

    pub(in crate::tui) fn page_count(&self) -> usize {
        self.visible_children().len().div_ceil(PAGE_SIZE).max(1)
    }

    fn move_cursor(&mut self, delta: isize) {
        let len = self.page_rows().len();
        if len == 0 {
            self.cursor = 0;
            return;
        }
        self.cursor = self.cursor.saturating_add_signed(delta).min(len - 1);
        if let Some(node_id) = self.focused_node().map(|node| node.id) {
            self.focus_anchors.insert(self.current_node_id, node_id);
        }
    }

    fn change_page(&mut self, delta: isize) {
        self.page = self
            .page
            .saturating_add_signed(delta)
            .min(self.page_count().saturating_sub(1));
        self.cursor = 0;
    }

    fn open_focused(&mut self) {
        let Some(node) = self.focused_node() else {
            return;
        };
        if node.kind != AnalyzeNodeKind::Directory {
            return;
        }
        let node_id = node.id;
        self.current_node_id = node_id;
        self.page = 0;
        self.cursor = 0;
        if let Some(anchor) = self.focus_anchors.get(&node_id).copied() {
            let rows = self.visible_children();
            if let Some(index) = rows.iter().position(|node| node.id == anchor) {
                self.page = index / PAGE_SIZE;
                self.cursor = index % PAGE_SIZE;
            }
        }
    }

    fn up(&mut self) {
        let Some(current) = self.current_directory() else {
            return;
        };
        let Some(parent_id) = current.parent_id else {
            return;
        };
        let child_id = current.id;
        self.current_node_id = parent_id;
        self.focus_anchors.insert(parent_id, child_id);
        let rows = self.visible_children();
        if let Some(index) = rows.iter().position(|node| node.id == child_id) {
            self.page = index / PAGE_SIZE;
            self.cursor = index % PAGE_SIZE;
        } else {
            self.page = 0;
            self.cursor = 0;
        }
    }
}

fn kind_rank(kind: AnalyzeNodeKind) -> u8 {
    match kind {
        AnalyzeNodeKind::Directory => 0,
        AnalyzeNodeKind::File => 1,
        AnalyzeNodeKind::Reparse => 2,
    }
}

pub(in crate::tui) fn proportional_bar(node: &AnalyzeNodeV1, maximum: u64, width: usize) -> String {
    if node.evidence == AnalyzeEvidence::Unknown {
        return "?".to_string();
    }
    if node.bytes == 0 || maximum == 0 || width == 0 {
        return String::new();
    }
    let filled = ((node.bytes as u128 * width as u128).div_ceil(maximum as u128))
        .try_into()
        .unwrap_or(width);
    let filled = filled.clamp(1, width);
    let symbol = if node.evidence == AnalyzeEvidence::Incomplete {
        '░'
    } else {
        '█'
    };
    std::iter::repeat_n(symbol, filled).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use devsweep_core::analysis::{
        ANALYZE_SNAPSHOT_VERSION, AnalyzeRootIdentity, AnalyzeWarningClass,
    };
    use ratatui::{Terminal, backend::TestBackend};

    use crate::{
        i18n::Locale,
        tui::{
            app::{App, Effect, UiEvent, WorkerEvent},
            render::render_app,
            shell::{ModeId, ShellComposition},
        },
    };

    fn node(
        id: u32,
        parent_id: Option<u32>,
        name: &str,
        bytes: u64,
        kind: AnalyzeNodeKind,
    ) -> AnalyzeNodeV1 {
        AnalyzeNodeV1 {
            id,
            parent_id,
            kind,
            name: name.to_string(),
            bytes,
            immediate_count: 0,
            recursive_count: 0,
            evidence: AnalyzeEvidence::Complete,
            warnings: Vec::new(),
            mtime_ms: None,
        }
    }

    fn snapshot(nodes: Vec<AnalyzeNodeV1>) -> AnalyzeSnapshotV1 {
        AnalyzeSnapshotV1 {
            version: ANALYZE_SNAPSHOT_VERSION,
            root: AnalyzeRootIdentity {
                input: "C:/root".into(),
                normalized: "C:/root".into(),
                volume: "vol".into(),
            },
            nodes,
            warnings: Vec::new(),
            completeness: AnalyzeCompleteness::Complete,
            accounted_owned_bytes: 0,
        }
    }

    fn completed(snapshot: AnalyzeSnapshotV1) -> AnalyzeModeState {
        let mut state = AnalyzeModeState::default();
        state.reduce(AnalyzeAction::Started { operation_id: 7 });
        state.reduce(AnalyzeAction::Finished {
            operation_id: 7,
            outcome: AnalyzeRunOutcome::Completed { snapshot },
        });
        state
    }

    #[test]
    fn stale_progress_and_terminal_events_are_rejected() {
        let mut state = AnalyzeModeState::default();
        state.reduce(AnalyzeAction::Started { operation_id: 7 });
        let progress = AnalyzeProgressV1 {
            sequence: 2,
            stored_nodes: 9,
            accounted_owned_bytes: 64,
            changed_nodes: Vec::new(),
            queue_depth: 1,
        };
        state.reduce(AnalyzeAction::Progressed {
            operation_id: 8,
            progress: progress.clone(),
        });
        state.reduce(AnalyzeAction::Progressed {
            operation_id: 7,
            progress,
        });
        state.reduce(AnalyzeAction::Progressed {
            operation_id: 7,
            progress: AnalyzeProgressV1 {
                sequence: 1,
                stored_nodes: 1,
                accounted_owned_bytes: 1,
                changed_nodes: Vec::new(),
                queue_depth: 1,
            },
        });
        assert_eq!(state.last_sequence, 2);
        assert_eq!(state.stored_nodes, 9);
    }

    #[test]
    fn paging_breadcrumbs_focus_and_viewport_are_bounded() {
        let mut nodes = vec![node(0, None, "root", 500, AnalyzeNodeKind::Directory)];
        for id in 1..=450 {
            nodes.push(node(
                id,
                Some(0),
                &format!("file-{id:03}"),
                u64::from(id),
                AnalyzeNodeKind::File,
            ));
        }
        let mut state = completed(snapshot(nodes));
        state.reduce(AnalyzeAction::PageChanged(2));
        assert_eq!(state.page_rows().len(), 50);
        assert_eq!(state.viewport_rows(12).len(), 12);
        assert_eq!(state.breadcrumbs().len(), 1);
    }

    #[test]
    fn viewport_scrolls_to_keep_the_keyboard_cursor_visible() {
        let mut nodes = vec![node(0, None, "root", 500, AnalyzeNodeKind::Directory)];
        for id in 1..=40 {
            nodes.push(node(
                id,
                Some(0),
                &format!("file-{id:03}"),
                u64::from(id),
                AnalyzeNodeKind::File,
            ));
        }
        let mut state = completed(snapshot(nodes));
        state.reduce(AnalyzeAction::MoveCursor(12));

        let viewport = state.viewport_rows(5);
        assert_eq!(viewport.len(), 5);
        assert_eq!(
            viewport.last().map(|node| node.id),
            state.focused_node().map(|node| node.id)
        );
    }

    #[test]
    fn keyboard_drill_down_and_up_restore_the_directory_anchor() {
        let root = node(0, None, "root", 10, AnalyzeNodeKind::Directory);
        let child = node(1, Some(0), "child", 10, AnalyzeNodeKind::Directory);
        let leaf = node(2, Some(1), "leaf", 10, AnalyzeNodeKind::File);
        let mut state = completed(snapshot(vec![root, child, leaf]));
        state.reduce(AnalyzeAction::OpenFocused);
        assert_eq!(state.current_node_id, 1);
        state.reduce(AnalyzeAction::Up);
        assert_eq!(state.current_node_id, 0);
        assert_eq!(state.focused_node().map(|node| node.id), Some(1));
    }

    #[test]
    fn bars_are_proportional_lower_bounds_and_never_invent_unknown_area() {
        let complete = node(1, Some(0), "complete", 50, AnalyzeNodeKind::File);
        let mut partial = node(2, Some(0), "partial", 25, AnalyzeNodeKind::File);
        partial.evidence = AnalyzeEvidence::Incomplete;
        partial.warnings.push(AnalyzeWarningClass::PartialBudget);
        let mut unknown = node(3, Some(0), "unknown", 0, AnalyzeNodeKind::File);
        unknown.evidence = AnalyzeEvidence::Unknown;
        assert_eq!(proportional_bar(&complete, 50, 10), "██████████");
        assert_eq!(proportional_bar(&partial, 50, 10), "░░░░░");
        assert_eq!(proportional_bar(&unknown, 50, 10), "?");
    }

    #[test]
    fn projection_keeps_only_frozen_analyze_snapshot_data() {
        let state = completed(snapshot(vec![node(
            0,
            None,
            "root",
            0,
            AnalyzeNodeKind::Directory,
        )]));
        assert!(state.visible_children().is_empty());
        assert!(state.focused_node().is_none());
        assert_eq!(state.phase, AnalyzePhase::Complete);
    }

    #[test]
    fn leaving_analyze_requests_cancel_and_releases_state_only_after_terminal_join_event() {
        let shell = ShellComposition::for_locale(Locale::En).expect("shell");
        let mut app = App::with_shell(shell);
        app.update(UiEvent::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::ALT,
        )));
        assert_eq!(app.shell.active, ModeId::Analyze);
        let effects = app.update(UiEvent::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::NONE,
        )));
        let [Effect::StartAnalyze { job_id }] = effects.as_slice() else {
            panic!("Analyze starts through the runtime worker seam");
        };
        let job_id = *job_id;
        let leave = app.update(UiEvent::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::ALT,
        )));
        assert_eq!(leave, vec![Effect::CancelJob { job_id }]);
        assert_eq!(app.shell.active, ModeId::Analyze);
        assert_eq!(app.analyze.phase, AnalyzePhase::Canceling);

        let mut canceled = snapshot(vec![node(0, None, "root", 0, AnalyzeNodeKind::Directory)]);
        canceled.completeness = AnalyzeCompleteness::Canceled;
        app.update(UiEvent::Worker(WorkerEvent::AnalyzeFinished {
            job_id,
            outcome: AnalyzeRunOutcome::Canceled { snapshot: canceled },
        }));
        assert_eq!(app.shell.active, ModeId::Clean);
        assert_eq!(app.analyze.phase, AnalyzePhase::Idle);
        assert!(app.analyze.snapshot.is_none());
    }

    #[test]
    fn test_backend_renders_only_viewport_rows_with_bilingual_evidence_copy() {
        let shell = ShellComposition::for_locale(Locale::ZhCn).expect("shell");
        let mut app = App::with_shell(shell);
        app.update(UiEvent::Key(KeyEvent::new(
            KeyCode::Char('f'),
            KeyModifiers::ALT,
        )));
        let effects = app.update(UiEvent::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::NONE,
        )));
        let [Effect::StartAnalyze { job_id }] = effects.as_slice() else {
            panic!("Analyze starts");
        };
        let mut nodes = vec![node(0, None, "root", 10, AnalyzeNodeKind::Directory)];
        nodes.push(node(1, Some(0), "child", 10, AnalyzeNodeKind::File));
        let mut denied = node(2, Some(0), "denied", 0, AnalyzeNodeKind::Directory);
        denied.evidence = AnalyzeEvidence::Incomplete;
        denied.warnings.push(AnalyzeWarningClass::AccessDenied);
        nodes.push(denied);
        let mut reparse = node(3, Some(0), "link", 0, AnalyzeNodeKind::Reparse);
        reparse.warnings.push(AnalyzeWarningClass::Reparse);
        nodes.push(reparse);
        app.update(UiEvent::Worker(WorkerEvent::AnalyzeFinished {
            job_id: *job_id,
            outcome: AnalyzeRunOutcome::Completed {
                snapshot: snapshot(nodes),
            },
        }));

        let backend = TestBackend::new(160, 24);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render_app(frame, &app))
            .expect("draw");
        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert_eq!(app.shell.active_label, "分析");
        assert_eq!(
            crate::i18n::catalogue(Locale::ZhCn)
                .static_text("analyze.v1.evidence.complete")
                .expect("Chinese evidence copy"),
            "可用"
        );
        assert!(rendered.contains("child"));
        let compact = rendered.replace(' ', "");
        assert!(compact.contains("权限被拒绝"), "{rendered}");
        assert!(compact.contains("不支持或未跟随的条目"), "{rendered}");
    }
}
