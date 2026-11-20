//! Analyze list and treemap projection for the native window.
//!
//! Node ids come from the snapshot the walker stored. Recycle preview calls
//! the same plan and live digest the CLI uses. The only move is
//! [`AnalyzeWindow::execute_confirmed`], which hands the plan and the digest
//! the user saw to the same core executor `analyze trash-execute --confirm`
//! calls. It never deletes; the executor only moves to the Recycle Bin.

use std::path::{Path, PathBuf};

use devsweep_core::analysis::{
    AnalyzeSavedError, AnalyzeSnapshotV1, AnalyzeTrashPlanV1, analyze_node_path, analyze_path,
    explain_analyze_node, list_analyze_roots, plan_analyze_trash, preview_analyze_trash_plan,
};
use devsweep_core::process::FlagCancelObserver;

use crate::analyze::{AnalyzeBoard, TreemapRect, treemap_rects};
#[cfg(test)]
use crate::analyze::{NodeFace, classify};
use crate::copy::{self, Locale};

pub const ANALYZE_ROOT_ENV: &str = "DEVSWEEP_ANALYZE_ROOT";

/// Snapshot and the recycle preview bound to the checked node ids.
#[derive(Clone, Debug)]
pub struct AnalyzeWindow {
    snapshot: AnalyzeSnapshotV1,
    board: AnalyzeBoard,
    parent_id: u32,
    recycle_line: Option<String>,
    recycle_plan: Option<AnalyzeTrashPlanV1>,
    /// The `sha256:` digest painted with the preview. Confirm passes this,
    /// not a fresh one, so a changed file refuses.
    recycle_digest: Option<String>,
    stale: bool,
    largest: devsweep_core::analysis::AnalyzeLargestV1,
    largest_active: bool,
    explanation: Option<(
        u32,
        Result<devsweep_core::analysis::AnalyzeExplainV1, String>,
    )>,
}

/// One landing choice. A missing or unreadable root stays distinct from a scan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalyzeRootChoice {
    pub input: String,
    pub normalized: Option<String>,
    pub error: Option<String>,
    pub capacity: Option<devsweep_core::analysis::AnalyzeVolumeCapacityV1>,
}

impl AnalyzeWindow {
    #[must_use]
    pub fn board(&self) -> &AnalyzeBoard {
        &self.board
    }

    #[must_use]
    pub fn recycle_line(&self) -> Option<&str> {
        self.recycle_line.as_deref()
    }

    /// Walks an isolated root. Does not scan when `root` is missing.
    pub fn scan(root: &Path) -> Result<Self, String> {
        Self::scan_with_cancel(root, None)
    }

    /// Same walk with cooperative cancellation. The window runs this on a
    /// worker thread; a canceled walk returns an error and installs nothing.
    pub fn scan_with_cancel(
        root: &Path,
        cancel: Option<&FlagCancelObserver>,
    ) -> Result<Self, String> {
        if !root.exists() {
            return Err("analyze root is missing".to_string());
        }
        let outcome = analyze_path(root, cancel, None).map_err(|error| error.to_string())?;
        let snapshot = match outcome {
            devsweep_core::analysis::AnalyzeRunOutcome::Completed { snapshot } => snapshot,
            devsweep_core::analysis::AnalyzeRunOutcome::Canceled { .. } => {
                return Err("canceled".into());
            }
        };
        let board = AnalyzeBoard::new(snapshot.nodes.clone());
        let parent_id = snapshot.root_node().map(|node| node.id).unwrap_or(0);
        let largest = devsweep_core::analysis::largest_analyze_files(
            &snapshot,
            devsweep_core::analysis::ANALYZE_LARGEST_MIN_BYTES,
            devsweep_core::analysis::ANALYZE_LIST_DEFAULT_LIMIT,
        )
        .map_err(|err| err.code().to_string())?;
        Ok(Self {
            largest,
            largest_active: false,
            explanation: None,
            snapshot,
            board,
            parent_id,
            recycle_line: None,
            recycle_plan: None,
            recycle_digest: None,
            stale: false,
        })
    }

    #[must_use]
    pub fn snapshot(&self) -> &AnalyzeSnapshotV1 {
        &self.snapshot
    }

    #[must_use]
    pub fn parent_id(&self) -> u32 {
        self.parent_id
    }

    pub fn arm_stale(&mut self) {
        self.stale = true;
    }

    #[must_use]
    pub fn stale(&self) -> bool {
        self.stale
    }

    /// The digest shown with the current preview, if any.
    #[must_use]
    pub fn recycle_digest(&self) -> Option<&str> {
        self.recycle_digest.as_deref()
    }

    /// Drops the preview without touching disk.
    pub fn cancel_recycle(&mut self) {
        self.recycle_line = None;
        self.recycle_plan = None;
        self.recycle_digest = None;
    }

    /// Moves the previewed node to the Recycle Bin.
    ///
    /// Requires a preview plan and the digest painted with it. A stale
    /// snapshot refuses here; the core executor then rebuilds the live
    /// digest and refuses on any mismatch before a side effect, exactly as
    /// the CLI does. A successful move marks the snapshot stale so a second
    /// confirm cannot reuse it.
    pub fn execute_confirmed(&mut self, locale: Locale) -> Result<String, String> {
        let (Some(plan), Some(digest)) = (self.recycle_plan.clone(), self.recycle_digest.clone())
        else {
            let error = AnalyzeSavedError::EmptySelection;
            let line = recycle_error_line(locale, &error);
            self.recycle_line = Some(line.clone());
            return Err(line);
        };
        if self.stale {
            self.cancel_recycle();
            let line = recycle_error_line(locale, &AnalyzeSavedError::StaleConfirmation);
            self.recycle_line = Some(line.clone());
            return Err(line);
        }
        match devsweep_core::analysis::execute_analyze_trash_plan(&plan, &digest, true, None) {
            Ok(report) => {
                let failed = report.report.as_ref().map_or(0, |item| item.failed);
                self.stale = true;
                self.recycle_plan = None;
                self.recycle_digest = None;
                let moved = report.moved_node_ids.len();
                let line = match locale {
                    Locale::En => format!(
                        "Moved {moved} item(s) to the Recycle Bin, {failed} failed. Rescan to refresh sizes."
                    ),
                    Locale::ZhCn => {
                        format!("已移到回收站 {moved} 项，失败 {failed} 项。重新扫描以刷新大小。")
                    }
                };
                self.recycle_line = Some(line.clone());
                if failed > 0 { Err(line) } else { Ok(line) }
            }
            Err(error) => {
                self.cancel_recycle();
                let line = recycle_error_line(locale, &error);
                self.recycle_line = Some(line.clone());
                Err(line)
            }
        }
    }

    /// Moves the list parent when `id` is a directory. File ids stay selected.
    pub fn drill(&mut self, id: u32) -> bool {
        let kind = self
            .snapshot
            .nodes
            .iter()
            .find(|node| node.id == id)
            .map(|node| node.kind);
        let Some(kind) = kind else {
            return false;
        };
        self.select(id);
        self.largest_active = false;
        if kind == devsweep_core::analysis::AnalyzeNodeKind::Directory {
            self.parent_id = id;
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn child_ids(&self) -> Vec<u32> {
        if self.largest_active {
            return self.largest.files.iter().map(|file| file.node_id).collect();
        }
        let mut nodes: Vec<_> = self
            .snapshot
            .nodes
            .iter()
            .filter(|node| node.parent_id == Some(self.parent_id))
            .collect();
        nodes.sort_unstable_by(|a, b| {
            b.bytes
                .cmp(&a.bytes)
                .then(a.name.cmp(&b.name))
                .then(a.id.cmp(&b.id))
        });
        nodes.into_iter().map(|node| node.id).collect()
    }

    #[must_use]
    pub fn treemap(&self) -> Vec<TreemapRect> {
        treemap_rects(&self.snapshot.nodes)
    }

    #[must_use]
    pub fn breadcrumb(&self) -> String {
        let mut names = Vec::new();
        let mut cursor = self.board.list_selection().or(Some(self.parent_id));
        let mut guard = 0;
        while let Some(id) = cursor {
            let Some(node) = self.snapshot.nodes.iter().find(|node| node.id == id) else {
                break;
            };
            names.push(node.name.clone());
            cursor = node.parent_id;
            guard += 1;
            if guard > self.snapshot.nodes.len() {
                break;
            }
        }
        names.reverse();
        names.join(" / ")
    }

    #[must_use]
    pub fn explain_line(&self, id: u32) -> String {
        match explain_analyze_node(&self.snapshot, id) {
            Ok(explained) => format!(
                "explain {id} class={:?} evidence={:?} disposable={}",
                explained.class, explained.evidence, explained.disposable
            ),
            Err(error) => format!("explain {id} {}: {error}", error.code()),
        }
    }

    /// Program and argv stay separate. `spawn` is the only path that starts Explorer.
    pub fn reveal_selected(&self, spawn: bool) -> Result<(String, Vec<String>), String> {
        let id = self
            .board
            .list_selection()
            .ok_or_else(|| AnalyzeSavedError::EmptySelection.to_string())?;
        let path = analyze_node_path(&self.snapshot, id).map_err(|error| error.to_string())?;
        let program = "explorer.exe".to_string();
        let args = vec!["/select,".to_string(), path.display().to_string()];
        if spawn {
            // The saved-snapshot entry point validates the node and refuses
            // off Windows before starting a process.
            devsweep_core::analysis::reveal_saved_analyze_node(&self.snapshot, id)
                .map_err(|error| error.to_string())?;
        }
        Ok((program, args))
    }

    /// Re-checks the live digest. Missing and stale digests refuse before any move.
    pub fn confirm_recycle(&mut self, locale: Locale) -> String {
        let Some(plan) = self.recycle_plan.clone() else {
            let line = recycle_error_line(locale, &AnalyzeSavedError::EmptySelection);
            self.recycle_line = Some(line.clone());
            return line;
        };
        let live = match preview_analyze_trash_plan(&plan) {
            Ok(preview) => preview,
            Err(error) => {
                let line = recycle_error_line(locale, &error);
                self.recycle_line = Some(line.clone());
                return line;
            }
        };
        let shown = self.recycle_line.clone().unwrap_or_default();
        if self.stale || !shown.contains(&live.digest) {
            let line = recycle_error_line(locale, &AnalyzeSavedError::StaleConfirmation);
            self.recycle_line = Some(line.clone());
            return line;
        }
        self.refuse_unconfirmed(locale)
    }

    pub fn show_largest(&mut self, largest: bool) {
        if self.largest_active != largest {
            self.largest_active = largest;
            self.cancel_recycle();
        }
    }

    pub fn largest_active(&self) -> bool {
        self.largest_active
    }

    pub fn largest(&self) -> &devsweep_core::analysis::AnalyzeLargestV1 {
        &self.largest
    }

    pub fn toggle(&mut self, id: u32) {
        self.board.toggle(id);
        self.cancel_recycle();
    }

    pub fn extend_range(&mut self, id: u32) {
        let order = self.child_ids();
        self.board.range(id, &order);
        self.cancel_recycle();
    }

    pub fn explain(&mut self, id: u32) {
        self.explanation = Some((
            id,
            explain_analyze_node(&self.snapshot, id).map_err(|error| error.code().to_string()),
        ));
    }

    pub fn explanation(
        &self,
        id: u32,
    ) -> Option<&Result<devsweep_core::analysis::AnalyzeExplainV1, String>> {
        self.explanation
            .as_ref()
            .filter(|(node, _)| *node == id)
            .map(|(_, result)| result)
    }

    pub fn focus_node(&mut self, id: u32) {
        self.board.focus_node(id);
    }

    pub fn select(&mut self, id: u32) {
        self.board.select_from_list(id);
        self.recycle_line = None;
        self.recycle_plan = None;
        self.recycle_digest = None;
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn recycle_plan(&self) -> Option<&AnalyzeTrashPlanV1> {
        self.recycle_plan.as_ref()
    }

    /// First complete file node. Directory roots are not recycle targets here.
    #[must_use]
    pub fn first_file_id(&self) -> Option<u32> {
        self.snapshot
            .nodes
            .iter()
            .find(|node| {
                node.kind == devsweep_core::analysis::AnalyzeNodeKind::File
                    && node.evidence == devsweep_core::analysis::AnalyzeEvidence::Complete
            })
            .map(|node| node.id)
    }

    /// Live trash preview for the selected node. Does not move or delete.
    pub fn preview_selected(&mut self, locale: Locale) -> Result<(), String> {
        self.cancel_recycle();
        if self.stale {
            let line = recycle_error_line(locale, &AnalyzeSavedError::StaleConfirmation);
            self.recycle_line = Some(line.clone());
            return Err(line);
        }
        let ids = self.board.recycle_ids();
        if ids.is_empty() {
            self.recycle_line = Some(recycle_error_line(
                locale,
                &AnalyzeSavedError::EmptySelection,
            ));
            return Err(AnalyzeSavedError::EmptySelection.to_string());
        }
        let plan = match plan_analyze_trash(&self.snapshot, &ids) {
            Ok(plan) => plan,
            Err(error) => {
                self.recycle_line = Some(recycle_error_line(locale, &error));
                return Err(error.to_string());
            }
        };
        match preview_analyze_trash_plan(&plan) {
            Ok(preview) => {
                if let Some(refused) = preview.refused.first() {
                    let line = format!(
                        "{} · {} #{}",
                        copy::pick(locale, "Selection refused", "所选项无法移入回收站"),
                        refusal_label(locale, refused.reason_code),
                        refused.node_id
                    );
                    self.recycle_line = Some(line.clone());
                    return Err(line);
                }
                self.recycle_plan = Some(plan);
                self.recycle_digest = Some(preview.digest.clone());
                self.recycle_line = Some(format!(
                    "{} {} | {}",
                    copy::pick(locale, "recycle", "回收节点"),
                    ids.iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                    preview.digest
                ));
                Ok(())
            }
            Err(error) => {
                self.recycle_line = Some(recycle_error_line(locale, &error));
                Err(error.to_string())
            }
        }
    }

    /// Confirmed=false gate. The file is not moved.
    pub fn refuse_unconfirmed(&mut self, locale: Locale) -> String {
        let ids = self.board.recycle_ids();
        let line = match plan_analyze_trash(&self.snapshot, &ids) {
            Ok(plan) => {
                match devsweep_core::analysis::execute_analyze_trash_plan(&plan, "", false, None) {
                    Err(error) => recycle_error_line(locale, &error),
                    Ok(_) => recycle_error_line(locale, &AnalyzeSavedError::ConfirmationRequired),
                }
            }
            Err(error) => recycle_error_line(locale, &error),
        };
        self.recycle_line = Some(line.clone());
        line
    }
}

/// Lines the window paints. Empty, unreadable, and present stay distinct.
/// List and treemap use the same node id.
#[must_use]
pub fn window_lines(window: &AnalyzeWindow, locale: Locale, visible: &[String]) -> Vec<String> {
    let mut lines = Vec::new();
    let selected = window.board.list_selection();
    let ids: Vec<u32> = if visible.is_empty() {
        window.board.list_ids()
    } else {
        visible.iter().filter_map(|id| id.parse().ok()).collect()
    };
    for id in ids {
        let Some(face) = window.board.face(id) else {
            continue;
        };
        let name = window
            .snapshot
            .nodes
            .iter()
            .find(|node| node.id == id)
            .map(|node| node.name.as_str())
            .unwrap_or("");
        let mark = if selected == Some(id) { "> " } else { "" };
        lines.push(format!(
            "{mark}{id} {} {} {name}",
            face.as_str(),
            copy::analyze_face_label(locale, face.as_str())
        ));
        lines.push(format!("treemap {id} {}", face.as_str()));
    }
    if let Some(line) = window.recycle_line() {
        lines.push(line.to_string());
    }
    lines.push(copy::analyze_list_title(locale).to_string());
    lines.push(copy::analyze_treemap_description(locale).to_string());
    lines.push(format!(
        "{} {} parent={}",
        copy::analyze_breadcrumb_label(locale),
        window.breadcrumb(),
        window.parent_id()
    ));
    if let Some(id) = window.board.list_selection() {
        lines.push(window.explain_line(id));
    }
    for rect in window.treemap() {
        if visible.is_empty() || visible.iter().any(|item| item == &rect.id.to_string()) {
            lines.push(format!(
                "treemap-rect {} x={:.1} y={:.1} w={:.1} h={:.1}",
                rect.id, rect.x, rect.y, rect.width, rect.height
            ));
        }
    }
    lines
}

/// Lists only the requested paths. An empty request is not implied here.
pub fn landing_roots(paths: &[PathBuf]) -> Vec<AnalyzeRootChoice> {
    let owned = paths.to_vec();
    match list_analyze_roots(&owned) {
        Ok(document) => document
            .roots
            .into_iter()
            .map(|root| AnalyzeRootChoice {
                input: root.input,
                normalized: root.normalized,
                capacity: root.capacity,
                error: root
                    .error
                    .map(|error| format!("{error:?}").to_ascii_lowercase()),
            })
            .collect(),
        Err(error) => vec![AnalyzeRootChoice {
            input: String::new(),
            normalized: None,
            capacity: None,
            error: Some(error.code().to_string()),
        }],
    }
}

pub fn refusal_label(
    locale: Locale,
    reason: devsweep_core::analysis::AnalyzeTrashRefusalCode,
) -> &'static str {
    use devsweep_core::analysis::AnalyzeTrashRefusalCode::*;
    let (en, zh) = match reason {
        Protected => ("Protected path", "路径已受保护"),
        SystemLocation => ("System location", "系统位置"),
        VolumeRoot => ("Volume root", "卷根目录"),
        ProfileRoot => ("Profile directory", "用户配置目录"),
        AnalysisRoot => ("Scan root", "扫描根目录"),
        ReparsePoint => ("Reparse point", "重解析点"),
        ChangedSinceSnapshot => ("Changed since the scan", "扫描后已发生变化"),
        NotFound => ("Item is missing", "项目已不存在"),
    };
    copy::pick(locale, en, zh)
}

pub fn localized_error(locale: Locale, reason: &str) -> String {
    if locale == Locale::En {
        return reason.into();
    }
    match reason {
        "missing" | "analyze root is missing" => "分析根目录已不存在。",
        "inaccessible" | "roots_unavailable" => "无法读取所选位置。",
        "invalid_snapshot" => "快照数据无效，请重新扫描。",
        "reveal_unavailable" => "无法在资源管理器中显示该项目。",
        "empty_selection" => "请选择至少一项。",
        "confirmation_required" => "需要先预览并明确确认。",
        "stale_confirmation" | "changed_since_snapshot" => "项目或摘要已变化，请重新扫描并预览。",
        "incomplete_observation" | "identity_unavailable" => "无法确认项目的完整信息或当前身份。",
        "protection_unavailable" => "无法读取保护列表，已停止操作。",
        "path_outside_root" | "snapshot_root_mismatch" => "路径与保存的扫描根目录不一致。",
        "unsupported_platform" => "当前系统不支持此操作。",
        "canceled" => "操作已取消。",
        "execution_failed" => "移动失败，请检查本次操作记录。",
        _ => "无法完成分析操作，请检查路径和访问权限。",
    }
    .into()
}

fn recycle_error_line(locale: Locale, error: &AnalyzeSavedError) -> String {
    match locale {
        Locale::En => format!("{}: {error}", error.code()),
        Locale::ZhCn => localized_error(locale, error.code()),
    }
}

/// Probe-only reader. The window does not scan this path.
#[must_use]
#[allow(dead_code)]
pub fn env_root() -> Option<PathBuf> {
    let value = std::env::var_os(ANALYZE_ROOT_ENV)?;
    if value.is_empty() {
        return None;
    }
    Some(PathBuf::from(value))
}

/// Classifies one stored node. Exposed so tests can compare faces without
/// painting.
#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, process::Stdio};

    fn isolated_root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("devsweep-gpui-c13-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("analyze root");
        root
    }

    #[test]
    #[cfg_attr(
        not(windows),
        ignore = "Windows-only behavior (Recycle Bin, Win32 paths, cmd.exe or file locking)"
    )]
    fn scan_distinguishes_empty_unreadable_and_present_and_preview_matches_cli() {
        let root = isolated_root("faces");
        fs::create_dir_all(root.join("empty-dir")).expect("empty dir");
        fs::write(root.join("keep.bin"), b"c13-window-fixture").expect("file");
        let denied = root.join("denied-dir");
        fs::create_dir_all(&denied).expect("denied dir");
        let user = std::env::var("USERNAME").unwrap_or_else(|_| "Users".to_string());
        let deny = format!("{user}:(OI)(CI)R");
        let _ = crate::process_spawn::command_without_window("icacls")
            .arg(&denied)
            .arg("/inheritance:r")
            .arg("/deny")
            .arg(&deny)
            .status();

        let mut window = AnalyzeWindow::scan(&root).expect("scan");
        assert_eq!(window.board().list_ids(), window.board().treemap_ids());
        let faces: Vec<(u32, NodeFace, String)> = window
            .snapshot
            .nodes
            .iter()
            .map(|node| (node.id, classify(node), node.name.clone()))
            .collect();
        let empty = faces
            .iter()
            .find(|(_, face, name)| *face == NodeFace::Empty && name.contains("empty-dir"));
        let present = faces
            .iter()
            .find(|(_, face, name)| *face == NodeFace::Present && name == "keep.bin");
        assert!(
            empty.is_some(),
            "empty directory must be distinct: {faces:?}"
        );
        assert!(present.is_some(), "file must be present: {faces:?}");
        let unreadable = faces
            .iter()
            .find(|(_, face, name)| *face == NodeFace::Unreadable && name.contains("denied"));
        if unreadable.is_none() {
            // The ACL deny may not be visible to the walker. Do not invent the face.
            eprintln!("unreadable face was not produced by the walker: {faces:?}");
        }

        let present_id = present.expect("present").0;
        window.select(present_id);
        window.preview_selected(Locale::En).expect("preview");
        let line = window.recycle_line().expect("digest line");
        assert!(line.contains("sha256:"), "{line}");
        assert!(line.contains(&present_id.to_string()), "{line}");

        let plan = window.recycle_plan().expect("stored plan").clone();
        let core = preview_analyze_trash_plan(&plan).expect("core preview");
        assert!(line.contains(&core.digest), "{line} vs {}", core.digest);

        let refused = window.refuse_unconfirmed(Locale::En);
        assert!(refused.contains("confirmation_required"), "{refused}");
        assert!(
            root.join("keep.bin").is_file(),
            "preview must not move the file"
        );

        let cli = ensure_cli();
        let plan_path = root.join("plan.json");
        fs::write(&plan_path, serde_json::to_vec(&plan).expect("plan bytes")).expect("write plan");
        let preview_path = root.join("preview.json");
        let output = crate::process_spawn::command_without_window(&cli)
            .args([
                "analyze",
                "trash-preview",
                "--plan",
                plan_path.to_str().expect("plan path"),
                "--format",
                "json",
                "--output",
                preview_path.to_str().expect("preview path"),
            ])
            .env("LOCALAPPDATA", &root)
            .env("APPDATA", &root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("cli preview");
        assert!(
            output.status.success(),
            "cli trash-preview failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let document: serde_json::Value =
            serde_json::from_slice(&fs::read(&preview_path).expect("preview file")).expect("json");
        let cli_digest = document["data"]["digest"].as_str().expect("cli digest");
        assert_eq!(cli_digest, core.digest);
        assert!(root.join("keep.bin").is_file());

        let _ = crate::process_spawn::command_without_window("icacls")
            .args([denied.as_os_str(), std::ffi::OsStr::new("/reset")])
            .status();
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[cfg_attr(
        not(windows),
        ignore = "Windows-only behavior (Recycle Bin, Win32 paths, cmd.exe or file locking)"
    )]
    fn landing_detail_reveal_and_stale_digest_do_not_delete() {
        let root = isolated_root("detail");
        fs::create_dir_all(root.join("empty-dir")).expect("empty");
        fs::write(root.join("keep.bin"), b"detail").expect("file");
        let missing = root.join("missing-root");
        let choices = landing_roots(&[root.clone(), missing.clone()]);
        let mut session = crate::shell::Session::unbacked();
        session.show_mode(crate::shell::Mode::Analyze);
        session.offer_analyze_roots(&[root.clone(), missing.clone()]);
        let painted = crate::view::panel_lines(&session, None).join("\n");
        assert!(painted.contains("root:0"), "{painted}");
        assert!(painted.contains("root:1"), "{painted}");
        assert!(
            choices
                .iter()
                .any(|choice| choice.error.is_none() && choice.input == root.display().to_string())
        );
        assert!(
            choices.iter().any(|choice| choice.error.is_some()),
            "{choices:?}"
        );

        let mut window = AnalyzeWindow::scan(&root).expect("scan");
        let file_id = window.first_file_id().expect("file");
        window.select(file_id);
        let rects = window.treemap();
        assert!(rects.iter().any(|rect| rect.id == file_id));
        assert_eq!(window.board().list_ids().len(), rects.len());
        let crumb = window.breadcrumb();
        assert!(!crumb.is_empty(), "{crumb}");
        let explained = window.explain_line(file_id);
        assert!(explained.contains("disposable=false"), "{explained}");
        let (program, args) = window.reveal_selected(false).expect("reveal identity");
        assert_eq!(program, "explorer.exe");
        assert_eq!(args[0], "/select,");
        assert!(args[1].contains("keep.bin"), "{args:?}");
        assert_ne!(program, args.join(" "));

        let missing_digest = window.confirm_recycle(Locale::En);
        assert!(
            missing_digest.contains("empty_selection") || missing_digest.contains("selection"),
            "{missing_digest}"
        );
        window.preview_selected(Locale::En).expect("preview");
        window.arm_stale();
        let stale = window.confirm_recycle(Locale::ZhCn);
        assert!(stale.contains("重新扫描"), "{stale}");
        assert!(!stale.contains("成功"));
        assert!(root.join("keep.bin").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[cfg(windows)]
    fn three_checked_rows_bind_one_plan_and_replacing_the_selection_drops_its_digest() {
        let fixture = isolated_root("three-checked");
        for name in ["one.bin", "two.bin", "three.bin", "four.bin"] {
            fs::write(fixture.join(name), name).expect("sample file");
        }
        let mut session = crate::shell::Session::unbacked();
        session.show_mode(crate::shell::Mode::Analyze);
        assert!(session.analyze_open_isolated_root(fixture.clone()));
        let order = session.analyze_window().unwrap().child_ids();
        assert_eq!(order.len(), 4);
        for &id in &order[..3] {
            session.analyze_mark(id, false);
        }
        assert!(session.analyze_preview_recycle());
        let window = session.analyze_window().unwrap();
        let mut checked = order[..3].to_vec();
        checked.sort();
        assert_eq!(window.recycle_plan().unwrap().selected_node_ids, checked);
        assert_eq!(
            window.board().selected_bytes(),
            checked
                .iter()
                .map(|id| window.board().node(*id).unwrap().bytes)
                .sum::<u64>()
        );
        assert!(window.recycle_digest().unwrap().starts_with("sha256:"));

        session.analyze_context_focus(order[0]);
        assert!(session.analyze_window().unwrap().recycle_digest().is_some());
        session.analyze_select(order[0]);
        let window = session.analyze_window().unwrap();
        assert_eq!(window.board().recycle_ids(), vec![order[0]]);
        assert!(window.recycle_digest().is_none());
        assert!(session.analyze_confirm_recycle().is_err());
        session.analyze_mark(order[2], true);
        assert_eq!(
            session.analyze_window().unwrap().board().recycle_ids(),
            checked
        );
        assert!(session.analyze_preview_recycle());
        session.analyze_mark(order[1], false);
        assert!(session.analyze_window().unwrap().recycle_digest().is_none());
        assert!(session.analyze_confirm_recycle().is_err());
        assert_eq!(fs::read_dir(&fixture).unwrap().count(), 4);
        fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn confirm_without_a_previewed_digest_refuses_and_moves_nothing() {
        let root = isolated_root("confirm-empty");
        fs::write(root.join("keep.bin"), b"keep").expect("file");
        let mut window = AnalyzeWindow::scan(&root).expect("scan");
        let file_id = window.first_file_id().expect("file");
        window.select(file_id);
        let refused = window
            .execute_confirmed(Locale::En)
            .expect_err("no preview means no move");
        assert!(refused.contains("empty_selection"), "{refused}");
        assert!(window.recycle_digest().is_none());
        assert!(root.join("keep.bin").is_file());

        // Selecting another node drops any preview, so a confirm can never
        // reuse a digest painted for a different row.
        window.select(window.parent_id());
        assert!(window.recycle_digest().is_none());
        assert!(window.execute_confirmed(Locale::ZhCn).is_err());
        assert!(root.join("keep.bin").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[cfg_attr(
        not(windows),
        ignore = "Windows-only behavior (Recycle Bin, Win32 paths, cmd.exe or file locking)"
    )]
    fn confirm_refuses_a_stale_snapshot_or_a_changed_file_before_any_move() {
        let root = isolated_root("confirm-changed");
        fs::write(root.join("keep.bin"), b"before").expect("file");
        let mut window = AnalyzeWindow::scan(&root).expect("scan");
        let file_id = window.first_file_id().expect("file");
        window.select(file_id);
        window.preview_selected(Locale::En).expect("preview");
        let digest = window.recycle_digest().expect("digest").to_string();
        assert!(digest.starts_with("sha256:"), "{digest}");
        assert!(
            window
                .recycle_line()
                .is_some_and(|line| line.contains(&digest))
        );

        // The file changes after the digest was painted. The executor rebuilds
        // the live digest and refuses before any side effect. The live identity
        // binds the file id, the millisecond mtime and the file length. Windows
        // stamps writes from a coarse clock tick, so an in-place rewrite within
        // the same tick can keep the mtime; the length binding catches that case
        // (see the core test `same_file_id_and_mtime_with_a_new_length_refuses_before_trash`).
        // Move the mtime here too so this test does not depend on clock ticks.
        let path = root.join("keep.bin");
        let written = fs::metadata(&path)
            .and_then(|meta| meta.modified())
            .expect("mtime");
        fs::write(&path, b"after, and longer").expect("rewrite");
        fs::File::options()
            .write(true)
            .open(&path)
            .and_then(|file| file.set_modified(written + std::time::Duration::from_secs(5)))
            .expect("advance mtime");
        let refused = window
            .execute_confirmed(Locale::En)
            .expect_err("a changed file must refuse");
        assert!(!refused.contains("Moved"), "{refused}");
        assert!(refused.contains("changed_since_snapshot"), "{refused}");
        assert!(root.join("keep.bin").is_file());

        // Planning still compares the live file with the snapshot that was
        // previewed. Rescan so the next preview binds the rewritten file,
        // then arm a stale digest without moving it.
        let mut window = AnalyzeWindow::scan(&root).expect("rescan");
        let file_id = window.first_file_id().expect("file");
        window.select(file_id);
        window.preview_selected(Locale::En).expect("preview again");
        window.arm_stale();
        let stale = window
            .execute_confirmed(Locale::En)
            .expect_err("stale snapshot refuses");
        assert!(stale.contains("stale_confirmation"), "{stale}");
        assert!(root.join("keep.bin").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    fn ensure_cli() -> PathBuf {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repo")
            .to_path_buf();
        let output = crate::process_spawn::command_without_window("cargo")
            .args([
                "build",
                "--locked",
                "-p",
                "devsweep-cli",
                "--bin",
                "devsweep",
                "--target-dir",
                "dist/ci-target",
            ])
            .current_dir(&repo)
            .output()
            .expect("cargo");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        repo.join(format!(
            "dist/ci-target/debug/devsweep{}",
            std::env::consts::EXE_SUFFIX
        ))
    }

    #[test]
    fn listed_root_opens_without_reading_analyze_env() {
        let root = isolated_root("pick");
        fs::create_dir_all(root.join("nested")).expect("nested");
        fs::write(root.join("keep.bin"), b"keep").expect("file");
        let decoy = root.join("not-the-scan");
        fs::create_dir_all(&decoy).expect("decoy");
        let previous = std::env::var_os(ANALYZE_ROOT_ENV);
        unsafe {
            std::env::set_var(ANALYZE_ROOT_ENV, &decoy);
        }
        assert!(env_root().is_some());

        let mut session = crate::shell::Session::unbacked();
        session.show_mode(crate::shell::Mode::Analyze);
        session.offer_analyze_roots(&[root.clone(), root.join("missing-root")]);
        session.apply_key(crate::shell::NavKey::Enter);
        assert!(
            session.analyze_window().is_none(),
            "Enter without a selected root must not scan DEVSWEEP_ANALYZE_ROOT"
        );

        assert!(session.analyze_open_listed(0));
        let window = session.analyze_window().expect("listed root");
        assert!(
            window.snapshot().root.input.contains("pick")
                || window.snapshot().root.normalized.contains("pick"),
            "{}",
            window.snapshot().root.input
        );
        assert!(!window.snapshot().root.input.contains("not-the-scan"));
        assert!(!window.snapshot().root.normalized.contains("not-the-scan"));

        session.apply_key(crate::shell::NavKey::RowDown);
        let selected = session
            .shell()
            .selection(crate::shell::Mode::Analyze)
            .expect("row")
            .to_string();
        let selected_id: u32 = selected.parse().expect("node id");
        assert_eq!(
            session
                .analyze_window()
                .expect("window")
                .board()
                .list_selection(),
            Some(selected_id)
        );
        assert!(
            session
                .analyze_window()
                .expect("window")
                .treemap()
                .iter()
                .any(|rect| rect.id == selected_id)
        );

        let before = session
            .analyze_window()
            .expect("window")
            .snapshot()
            .root
            .normalized
            .clone();
        assert!(!session.analyze_open_isolated_root(decoy.join("missing")));
        assert!(session.analyze_notice().is_some());
        assert_eq!(
            session
                .analyze_window()
                .expect("failed pick must not replace the scan")
                .snapshot()
                .root
                .normalized,
            before
        );

        unsafe {
            match previous {
                Some(value) => std::env::set_var(ANALYZE_ROOT_ENV, value),
                None => std::env::remove_var(ANALYZE_ROOT_ENV),
            }
        }
        let _ = fs::remove_dir_all(&root);
    }
}
