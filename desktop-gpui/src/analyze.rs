//! In-memory Analyze board. List and treemap share one node-id order and one
//! selection. Recycle reports that selection only; it does not delete, move,
//! or preview.

use devsweep_core::analysis::{
    AnalyzeEvidence, AnalyzeNodeKind, AnalyzeNodeV1, AnalyzeWarningClass,
};

/// Face word shown for one snapshot node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeFace {
    Empty,
    Unreadable,
    Present,
}

impl NodeFace {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Unreadable => "unreadable",
            Self::Present => "present",
        }
    }
}

/// One Analyze detail board. Nodes are constructed in memory; this type does
/// not walk a filesystem.
#[derive(Clone, Debug)]
pub struct AnalyzeBoard {
    nodes: Vec<AnalyzeNodeV1>,
    selection: Option<u32>,
}

impl AnalyzeBoard {
    #[must_use]
    pub fn new(nodes: Vec<AnalyzeNodeV1>) -> Self {
        Self {
            nodes,
            selection: None,
        }
    }

    /// Snapshot-local ids in board order. Same sequence as [`Self::treemap_ids`].
    #[must_use]
    pub fn list_ids(&self) -> Vec<u32> {
        self.node_ids()
    }

    /// Snapshot-local ids in board order. Same sequence as [`Self::list_ids`].
    #[must_use]
    pub fn treemap_ids(&self) -> Vec<u32> {
        self.node_ids()
    }

    #[must_use]
    pub fn list_selection(&self) -> Option<u32> {
        self.selection
    }

    #[must_use]
    pub fn treemap_selection(&self) -> Option<u32> {
        self.selection
    }

    pub fn select_from_list(&mut self, id: u32) {
        self.select(id);
    }

    pub fn select_from_treemap(&mut self, id: u32) {
        self.select(id);
    }

    #[must_use]
    pub fn face(&self, id: u32) -> Option<NodeFace> {
        self.nodes.iter().find(|node| node.id == id).map(classify)
    }

    /// Selected id only. Does not delete, move, or preview.
    #[must_use]
    pub fn recycle_ids(&self) -> Vec<u32> {
        self.selection.into_iter().collect()
    }

    #[must_use]
    pub fn selected_face_line(&self) -> Option<String> {
        let id = self.selection?;
        let face = self.face(id)?;
        Some(format!("{id} {}", face.as_str()))
    }

    fn select(&mut self, id: u32) {
        if self.nodes.iter().any(|node| node.id == id) {
            self.selection = Some(id);
        }
    }

    fn node_ids(&self) -> Vec<u32> {
        self.nodes.iter().map(|node| node.id).collect()
    }
}

/// Classify one node. Unreadable wins over a zero-byte empty lookalike.
#[must_use]
pub fn classify(node: &AnalyzeNodeV1) -> NodeFace {
    if is_unreadable(node) {
        NodeFace::Unreadable
    } else if node.kind == AnalyzeNodeKind::Directory
        && node.evidence == AnalyzeEvidence::Complete
        && node.immediate_count == 0
        && node.bytes == 0
    {
        NodeFace::Empty
    } else {
        NodeFace::Present
    }
}

fn is_unreadable(node: &AnalyzeNodeV1) -> bool {
    node.evidence == AnalyzeEvidence::Unknown
        || node.warnings.contains(&AnalyzeWarningClass::AccessDenied)
}

/// Bind a CLI trash-preview digest to recycle and plan node ids.
///
/// Returns the digest when both id lists match and `digest` is `sha256:` plus
/// 64 lowercase hex characters. A different id list is `Err`. This does not
/// preview, execute, delete, or move.
pub fn bind_recycle_digest(
    recycle_ids: &[u32],
    plan_ids: &[u32],
    digest: &str,
) -> Result<String, ()> {
    if recycle_ids != plan_ids || !sha256_wire_digest(digest) {
        return Err(());
    }
    Ok(digest.to_string())
}

fn sha256_wire_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::{Path, PathBuf},
        process::{Command, Output, Stdio},
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;
    use crate::shell::{Mode, Session};
    use crate::view::panel_lines;
    use devsweep_core::analysis::{
        AnalyzeEvidence, AnalyzeNodeKind, AnalyzeSnapshotV1, AnalyzeTrashPlanV1,
        AnalyzeWarningClass,
    };

    fn node(
        id: u32,
        kind: AnalyzeNodeKind,
        bytes: u64,
        immediate_count: u32,
        evidence: AnalyzeEvidence,
        warnings: Vec<AnalyzeWarningClass>,
    ) -> AnalyzeNodeV1 {
        AnalyzeNodeV1 {
            id,
            parent_id: None,
            kind,
            name: format!("node-{id}"),
            bytes,
            immediate_count,
            recursive_count: 0,
            evidence,
            warnings,
            mtime_ms: None,
        }
    }

    #[test]
    fn analyze_list_and_treemap_share_ids() {
        let empty_id = 11;
        let unreadable_id = 22;
        let file_id = 33;
        let empty = node(
            empty_id,
            AnalyzeNodeKind::Directory,
            0,
            0,
            AnalyzeEvidence::Complete,
            Vec::new(),
        );
        let unreadable = node(
            unreadable_id,
            AnalyzeNodeKind::Directory,
            0,
            0,
            AnalyzeEvidence::Complete,
            vec![AnalyzeWarningClass::AccessDenied],
        );
        let present = node(
            file_id,
            AnalyzeNodeKind::File,
            64,
            0,
            AnalyzeEvidence::Complete,
            Vec::new(),
        );

        let mut board = AnalyzeBoard::new(vec![empty, unreadable, present]);
        assert_eq!(board.list_ids(), board.treemap_ids());
        assert_eq!(board.list_ids(), vec![empty_id, unreadable_id, file_id]);

        board.select_from_list(empty_id);
        assert_eq!(board.treemap_selection(), Some(empty_id));
        board.select_from_treemap(file_id);
        assert_eq!(board.list_selection(), Some(file_id));
        assert_eq!(board.treemap_selection(), Some(file_id));
        assert_eq!(board.face(empty_id), Some(NodeFace::Empty));
        assert_eq!(board.face(unreadable_id), Some(NodeFace::Unreadable));
        assert_eq!(board.face(file_id), Some(NodeFace::Present));
        assert_eq!(board.recycle_ids(), vec![file_id]);

        let mut session = Session::unbacked();
        session.show_mode(Mode::Analyze);
        let baseline = panel_lines(&session, None);
        session.set_analyze_board(Some(board.clone()));
        assert_eq!(
            session.analyze_board().map(AnalyzeBoard::recycle_ids),
            Some(vec![file_id])
        );
        let lines = panel_lines(&session, None);
        let face = board.selected_face_line().expect("selected face");
        assert_eq!(face, format!("{file_id} present"));
        assert_eq!(lines.len(), baseline.len() + 1);
        assert_eq!(lines.last(), Some(&face));
        let face_lines: Vec<&String> = lines
            .iter()
            .filter(|line| {
                line.ends_with(" empty")
                    || line.ends_with(" unreadable")
                    || line.ends_with(" present")
            })
            .collect();
        assert_eq!(face_lines, vec![&face]);
        for id in [empty_id, unreadable_id, file_id] {
            assert!(
                !lines.iter().any(|line| line == &id.to_string()),
                "panel must not list raw id {id}: {lines:?}"
            );
        }
        assert!(
            !lines.iter().any(|line| line.ends_with(" empty")),
            "unselected empty node must not appear: {lines:?}"
        );
        assert!(
            !lines.iter().any(|line| line.ends_with(" unreadable")),
            "unselected unreadable node must not appear: {lines:?}"
        );

        let source = include_str!("analyze.rs");
        assert!(!source.contains(&["execute_analyze_trash", "_plan"].concat()));
        assert!(!source.contains(&["preview_analyze_trash", "_plan"].concat()));
    }

    #[test]
    fn analyze_recycle_ids_match_cli_trash_plan() {
        let root = isolated_c13_root();
        let source_file = root.join("keep.bin");
        fs::write(&source_file, b"c13-recycle-id-fixture").expect("plant keep.bin");
        let snapshot_path = root.join("snapshot.json");
        let plan_path = root.join("plan.json");
        let bin = ensure_devsweep_cli();
        let root_arg = root.to_str().expect("utf-8 isolated root");
        let snapshot_arg = snapshot_path.to_str().expect("utf-8 snapshot");
        let plan_arg = plan_path.to_str().expect("utf-8 plan");

        let scan_args = [
            "analyze",
            "scan",
            "--root",
            root_arg,
            "--format",
            "json",
            "--output",
            snapshot_arg,
        ];
        let scan = run_devsweep(&bin, &scan_args, &root);
        assert_eq!(scan.status.code(), Some(0), "{}", stderr(&scan));

        let snapshot = load_scan_snapshot(&snapshot_path);
        let file_node = snapshot
            .nodes
            .iter()
            .find(|node| {
                node.name == "keep.bin"
                    && node.kind == AnalyzeNodeKind::File
                    && node.evidence == AnalyzeEvidence::Complete
            })
            .expect("complete keep.bin file node");
        let file_id = file_node.id;

        let mut board = AnalyzeBoard::new(snapshot.nodes.clone());
        board.select_from_list(file_id);
        assert_eq!(board.recycle_ids(), vec![file_id]);

        let selected = file_id.to_string();
        let plan_args = [
            "analyze",
            "trash-plan",
            "--snapshot",
            snapshot_arg,
            "--select",
            &selected,
            "--output",
            plan_arg,
        ];
        let planned = run_devsweep(&bin, &plan_args, &root);
        assert_eq!(planned.status.code(), Some(0), "{}", stderr(&planned));
        let plan: AnalyzeTrashPlanV1 =
            serde_json::from_slice(&fs::read(&plan_path).expect("read AnalyzeTrashPlanV1 file"))
                .expect("plan JSON is AnalyzeTrashPlanV1, not an envelope");
        assert_eq!(plan.selected_node_ids, board.recycle_ids());
        assert!(
            source_file.is_file(),
            "trash-plan must leave the source file in place"
        );

        cleanup(&root);
    }

    #[test]
    fn analyze_trash_preview_digest_binds_recycle_ids() {
        let bucket = isolated_c13_root();
        let profile = bucket.join("home").join("user");
        let appdata = profile.join("AppData").join("Roaming");
        let work = bucket.join("work");
        fs::create_dir_all(&appdata).expect("fake APPDATA");
        fs::create_dir_all(profile.join("AppData").join("Local")).expect("fake LOCALAPPDATA");
        fs::create_dir_all(&work).expect("scan root");
        let source_file = work.join("keep.bin");
        let planted = b"c13-recycle-digest-fixture";
        fs::write(&source_file, planted).expect("plant keep.bin");
        let snapshot_path = bucket.join("snapshot.json");
        let plan_path = bucket.join("plan.json");
        let bin = ensure_devsweep_cli();
        let work_arg = work.to_str().expect("utf-8 scan root");
        let snapshot_arg = snapshot_path.to_str().expect("utf-8 snapshot");
        let plan_arg = plan_path.to_str().expect("utf-8 plan");

        let scan_args = [
            "analyze",
            "scan",
            "--root",
            work_arg,
            "--format",
            "json",
            "--output",
            snapshot_arg,
        ];
        let scan = run_devsweep(&bin, &scan_args, &profile);
        assert_eq!(scan.status.code(), Some(0), "{}", stderr(&scan));

        let snapshot = load_scan_snapshot(&snapshot_path);
        let file_node = snapshot
            .nodes
            .iter()
            .find(|node| {
                node.name == "keep.bin"
                    && node.kind == AnalyzeNodeKind::File
                    && node.evidence == AnalyzeEvidence::Complete
            })
            .expect("complete keep.bin file node");
        let file_id = file_node.id;

        let mut board = AnalyzeBoard::new(snapshot.nodes.clone());
        board.select_from_list(file_id);
        assert_eq!(board.recycle_ids(), vec![file_id]);

        let selected = file_id.to_string();
        let plan_args = [
            "analyze",
            "trash-plan",
            "--snapshot",
            snapshot_arg,
            "--select",
            &selected,
            "--output",
            plan_arg,
        ];
        let planned = run_devsweep(&bin, &plan_args, &profile);
        assert_eq!(planned.status.code(), Some(0), "{}", stderr(&planned));
        let plan: AnalyzeTrashPlanV1 =
            serde_json::from_slice(&fs::read(&plan_path).expect("read AnalyzeTrashPlanV1 file"))
                .expect("plan JSON is AnalyzeTrashPlanV1, not an envelope");
        assert_eq!(plan.selected_node_ids, board.recycle_ids());

        let preview_args = [
            "analyze",
            "trash-preview",
            "--plan",
            plan_arg,
            "--format",
            "json",
        ];
        let first = run_devsweep_trash_preview(&bin, &preview_args, &profile, &appdata);
        assert_eq!(first.status.code(), Some(0), "{}", stderr(&first));
        let first_document = json_envelope(&first);
        if let Some(code) = keep_bin_refusal(&first_document, file_id) {
            panic!("trash-preview refused keep.bin node {file_id}: {code}");
        }
        assert!(
            first_document["data"]["items"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|item| item["node_id"] == file_id),
            "trash-preview items must include keep.bin: {first_document}"
        );
        let first_digest = first_document["data"]["digest"]
            .as_str()
            .expect("first data.digest")
            .to_string();

        let second = run_devsweep_trash_preview(&bin, &preview_args, &profile, &appdata);
        assert_eq!(second.status.code(), Some(0), "{}", stderr(&second));
        let second_document = json_envelope(&second);
        if let Some(code) = keep_bin_refusal(&second_document, file_id) {
            panic!("trash-preview refused keep.bin node {file_id}: {code}");
        }
        let second_digest = second_document["data"]["digest"]
            .as_str()
            .expect("second data.digest")
            .to_string();
        assert_eq!(first_digest, second_digest);
        assert!(
            first_digest.starts_with("sha256:"),
            "digest must start with sha256: {first_digest}"
        );
        assert!(
            second_digest.starts_with("sha256:"),
            "digest must start with sha256: {second_digest}"
        );
        assert_eq!(
            fs::read(&source_file).expect("keep.bin after previews"),
            planted
        );
        assert_eq!(
            bind_recycle_digest(&board.recycle_ids(), &plan.selected_node_ids, &first_digest),
            Ok(first_digest.clone())
        );
        assert_eq!(
            bind_recycle_digest(&[file_id + 1], &plan.selected_node_ids, &first_digest),
            Err(())
        );

        cleanup(&bucket);
    }

    fn isolated_c13_root() -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = env::temp_dir().join(format!(
            "devsweep-gpui-c13-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(
            root.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("devsweep-gpui-c13-")),
            "refusing a fixture outside the temp prefix"
        );
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn cleanup(root: &Path) {
        assert!(
            root.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("devsweep-gpui-c13-"))
        );
        let _ = fs::remove_dir_all(root);
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("desktop-gpui lives in the repository")
            .to_path_buf()
    }

    fn ensure_devsweep_cli() -> PathBuf {
        let repo = repo_root();
        let output = Command::new("cargo")
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
            .expect("cargo build starts");
        assert!(
            output.status.success(),
            "cargo build --locked -p devsweep-cli failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bin = repo.join(format!(
            "dist/ci-target/debug/devsweep{}",
            std::env::consts::EXE_SUFFIX
        ));
        assert!(bin.is_file(), "missing built CLI {}", bin.display());
        bin
    }

    fn run_devsweep(bin: &Path, args: &[&str], config_dir: &Path) -> Output {
        assert_no_trash_mutation_argv(args);
        Command::new(bin)
            .args(args)
            .env("LOCALAPPDATA", config_dir)
            .env("APPDATA", config_dir)
            .env("USERPROFILE", config_dir)
            .env("HOME", config_dir)
            .env("XDG_CONFIG_HOME", config_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("devsweep process runs")
    }

    fn assert_no_trash_mutation_argv(args: &[&str]) {
        assert!(
            !args
                .iter()
                .any(|arg| arg.contains("trash-preview") || arg.contains("trash-execute")),
            "argv must not contain trash-preview or trash-execute: {args:?}"
        );
    }

    fn run_devsweep_trash_preview(
        bin: &Path,
        args: &[&str],
        profile: &Path,
        appdata: &Path,
    ) -> Output {
        assert!(
            args.contains(&"trash-preview"),
            "argv may contain trash-preview: {args:?}"
        );
        assert!(
            !args.iter().any(|arg| arg.contains("trash-execute")),
            "argv must not contain trash-execute: {args:?}"
        );
        Command::new(bin)
            .args(args)
            .env("USERPROFILE", profile)
            .env("APPDATA", appdata)
            .env("LOCALAPPDATA", profile.join("AppData").join("Local"))
            .env("HOME", profile)
            .env("XDG_CONFIG_HOME", appdata)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("devsweep process runs")
    }

    fn json_envelope(output: &Output) -> serde_json::Value {
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "json envelope {error}: stdout {} stderr {}",
                String::from_utf8_lossy(&output.stdout),
                stderr(output)
            )
        })
    }

    fn keep_bin_refusal(document: &serde_json::Value, file_id: u32) -> Option<String> {
        document["data"]["refused"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|item| item["node_id"] == file_id)
            .map(|item| {
                item["reason_code"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string()
            })
    }

    fn load_scan_snapshot(path: &Path) -> AnalyzeSnapshotV1 {
        let bytes = fs::read(path).expect("read snapshot");
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("snapshot JSON");
        let data = if value.get("schema_version") == Some(&serde_json::json!(1))
            && value.get("command") == Some(&serde_json::json!("analyze.scan"))
        {
            value
                .get("data")
                .cloned()
                .expect("analyze.scan envelope data")
        } else {
            value
        };
        serde_json::from_value(data).expect("AnalyzeSnapshotV1")
    }

    fn stderr(output: &Output) -> String {
        String::from_utf8_lossy(&output.stderr).into_owned()
    }
}
