use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::fs_size::{estimate_tree, is_unsafe_link};
use crate::model::{
    CleanAction, CleanTarget, CleanupPlan, Ecosystem, Evidence, RiskLevel, Scope, TargetId,
    TargetKind,
};

pub struct ProjectScanner;

impl ProjectScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan_roots(&self, roots: &[PathBuf]) -> Result<CleanupPlan> {
        let mut targets = Vec::new();

        for root in roots {
            let root = root
                .canonicalize()
                .with_context(|| format!("failed to access scan root {}", root.display()))?;
            self.scan_dir(&root, None, &mut targets)?;
        }

        targets = dedupe_targets(targets);

        Ok(CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets,
        })
    }

    fn scan_dir(
        &self,
        dir: &Path,
        python_context: Option<&PythonContext>,
        targets: &mut Vec<CleanTarget>,
    ) -> Result<()> {
        let metadata = fs::symlink_metadata(dir)
            .with_context(|| format!("failed to inspect {}", dir.display()))?;
        if !metadata.is_dir() || is_unsafe_link(&metadata) {
            return Ok(());
        }

        if should_stop_descent(dir) {
            if dir.file_name().and_then(|name| name.to_str()) == Some("__pycache__")
                && let Some(context) = python_context
            {
                targets.push(build_path_target(PathTargetInput {
                    rule_id: "python.__pycache__",
                    ecosystem: Ecosystem::Python,
                    kind: TargetKind::TestCache,
                    project_root: context.project_root.clone(),
                    path: dir.to_path_buf(),
                    risk: RiskLevel::Low,
                    selected_by_default: true,
                    evidence: vec![
                        Evidence::MarkerFile {
                            path: context.marker.clone(),
                        },
                        Evidence::RuleMatched {
                            rule_id: "python.__pycache__".to_string(),
                        },
                    ],
                }));
            }
            return Ok(());
        }

        self.scan_rust_project(dir, targets);
        self.scan_node_project(dir, targets);

        let local_python_context = find_python_marker(dir).map(|marker| PythonContext {
            project_root: dir.to_path_buf(),
            marker,
        });
        let active_python_context = local_python_context.as_ref().or(python_context);
        if let Some(context) = active_python_context {
            self.scan_python_project_dir(dir, context, targets);
        }

        for entry in
            fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))?
        {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            let path = entry.path();
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            if metadata.is_dir() && !is_unsafe_link(&metadata) {
                self.scan_dir(&path, active_python_context, targets)?;
            }
        }

        Ok(())
    }

    fn scan_rust_project(&self, dir: &Path, targets: &mut Vec<CleanTarget>) {
        let manifest = dir.join("Cargo.toml");
        if !manifest.is_file() {
            return;
        }

        let target_dir = dir.join("target");
        if !is_real_dir(&target_dir) {
            return;
        }

        let manifest_arg = manifest.to_string_lossy().into_owned();
        let mut target = build_path_target(PathTargetInput {
            rule_id: "rust.target",
            ecosystem: Ecosystem::Rust,
            kind: TargetKind::BuildArtifacts,
            project_root: dir.to_path_buf(),
            path: target_dir,
            risk: RiskLevel::Low,
            selected_by_default: true,
            evidence: vec![
                Evidence::MarkerFile {
                    path: manifest.clone(),
                },
                Evidence::OfficialCommand {
                    command: format!("cargo clean --manifest-path {manifest_arg}"),
                },
                Evidence::RuleMatched {
                    rule_id: "rust.target".to_string(),
                },
            ],
        });
        target.reversible = false;
        target.action = CleanAction::Command {
            program: "cargo".to_string(),
            args: vec![
                "clean".to_string(),
                "--manifest-path".to_string(),
                manifest_arg,
            ],
            cwd: None,
            irreversible: true,
        };
        targets.push(target);
    }

    fn scan_node_project(&self, dir: &Path, targets: &mut Vec<CleanTarget>) {
        let Some(marker) = find_node_marker(dir) else {
            return;
        };

        let rules = [
            (
                "node.node_modules",
                PathBuf::from("node_modules"),
                TargetKind::DependencyDirectory,
                RiskLevel::Medium,
                false,
            ),
            (
                "node.next_cache",
                PathBuf::from(".next").join("cache"),
                TargetKind::BuildArtifacts,
                RiskLevel::Low,
                true,
            ),
            (
                "node.turbo",
                PathBuf::from(".turbo"),
                TargetKind::ToolCache,
                RiskLevel::Low,
                true,
            ),
            (
                "node.parcel_cache",
                PathBuf::from(".parcel-cache"),
                TargetKind::ToolCache,
                RiskLevel::Low,
                true,
            ),
        ];

        for (rule_id, relative, kind, risk, selected) in rules {
            let path = dir.join(relative);
            if is_real_dir(&path) {
                targets.push(build_path_target(PathTargetInput {
                    rule_id,
                    ecosystem: Ecosystem::Node,
                    kind,
                    project_root: dir.to_path_buf(),
                    path,
                    risk,
                    selected_by_default: selected,
                    evidence: vec![
                        Evidence::MarkerFile {
                            path: marker.clone(),
                        },
                        Evidence::RuleMatched {
                            rule_id: rule_id.to_string(),
                        },
                    ],
                }));
            }
        }
    }

    fn scan_python_project_dir(
        &self,
        dir: &Path,
        context: &PythonContext,
        targets: &mut Vec<CleanTarget>,
    ) {
        let rules = [
            (
                "python.venv_dot",
                ".venv",
                TargetKind::VirtualEnv,
                RiskLevel::Medium,
                false,
            ),
            (
                "python.venv",
                "venv",
                TargetKind::VirtualEnv,
                RiskLevel::Medium,
                false,
            ),
            (
                "python.pytest_cache",
                ".pytest_cache",
                TargetKind::TestCache,
                RiskLevel::Low,
                true,
            ),
            (
                "python.mypy_cache",
                ".mypy_cache",
                TargetKind::ToolCache,
                RiskLevel::Low,
                true,
            ),
            (
                "python.ruff_cache",
                ".ruff_cache",
                TargetKind::ToolCache,
                RiskLevel::Low,
                true,
            ),
            (
                "python.tox",
                ".tox",
                TargetKind::ToolCache,
                RiskLevel::Medium,
                false,
            ),
        ];

        for (rule_id, name, kind, risk, selected) in rules {
            let path = dir.join(name);
            if is_real_dir(&path) {
                targets.push(build_path_target(PathTargetInput {
                    rule_id,
                    ecosystem: Ecosystem::Python,
                    kind,
                    project_root: context.project_root.clone(),
                    path,
                    risk,
                    selected_by_default: selected,
                    evidence: vec![
                        Evidence::MarkerFile {
                            path: context.marker.clone(),
                        },
                        Evidence::RuleMatched {
                            rule_id: rule_id.to_string(),
                        },
                    ],
                }));
            }
        }
    }
}

impl Default for ProjectScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct PythonContext {
    project_root: PathBuf,
    marker: PathBuf,
}

struct PathTargetInput {
    rule_id: &'static str,
    ecosystem: Ecosystem,
    kind: TargetKind,
    project_root: PathBuf,
    path: PathBuf,
    risk: RiskLevel,
    selected_by_default: bool,
    evidence: Vec<Evidence>,
}

fn build_path_target(input: PathTargetInput) -> CleanTarget {
    let PathTargetInput {
        rule_id,
        ecosystem,
        kind,
        project_root,
        path,
        risk,
        selected_by_default,
        evidence,
    } = input;
    let (estimated_bytes, last_modified) = estimate_tree(&path);
    CleanTarget {
        id: TargetId::new(format!("{rule_id}:{}", path.display())),
        scope: Scope::Project { root: project_root },
        ecosystem,
        kind,
        path: Some(path.clone()),
        estimated_bytes,
        last_modified,
        risk,
        reversible: true,
        selected_by_default,
        evidence,
        action: CleanAction::MoveToTrash { path },
    }
}

fn dedupe_targets(mut targets: Vec<CleanTarget>) -> Vec<CleanTarget> {
    targets.sort_by(|left, right| {
        let left_path = left.path.as_ref().map(|path| path.components().count());
        let right_path = right.path.as_ref().map(|path| path.components().count());
        left_path
            .cmp(&right_path)
            .then_with(|| left.id.as_str().cmp(right.id.as_str()))
    });

    let mut kept: Vec<CleanTarget> = Vec::new();
    for target in targets {
        let is_nested = target.path.as_ref().is_some_and(|path| {
            kept.iter().any(|existing| {
                existing.path.as_ref().is_some_and(|existing_path| {
                    path != existing_path && path.starts_with(existing_path)
                })
            })
        });

        if !is_nested {
            kept.push(target);
        }
    }

    kept
}

fn find_node_marker(dir: &Path) -> Option<PathBuf> {
    [
        "package.json",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
    ]
    .into_iter()
    .map(|name| dir.join(name))
    .find(|path| path.is_file())
}

fn find_python_marker(dir: &Path) -> Option<PathBuf> {
    [
        "pyproject.toml",
        "requirements.txt",
        "setup.py",
        "setup.cfg",
        "tox.ini",
    ]
    .into_iter()
    .map(|name| dir.join(name))
    .find(|path| path.is_file())
}

fn is_real_dir(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.is_dir() && !is_unsafe_link(&metadata))
        .unwrap_or(false)
}

fn should_stop_descent(dir: &Path) -> bool {
    matches!(
        dir.file_name().and_then(|name| name.to_str()),
        Some(
            "target"
                | "node_modules"
                | "cache"
                | ".turbo"
                | ".parcel-cache"
                | ".venv"
                | "venv"
                | "__pycache__"
                | ".pytest_cache"
                | ".mypy_cache"
                | ".ruff_cache"
                | ".tox"
        )
    )
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn scanner_finds_marker_backed_project_targets() {
        let fixture = Fixture::new();
        fixture.file("rust-app/Cargo.toml", "[package]\nname = \"rust-app\"\n");
        fixture.file("rust-app/target/debug/app.bin", "binary");
        fixture.file("node-app/package.json", "{}");
        fixture.file("node-app/node_modules/pkg/index.js", "module");
        fixture.file("node-app/.next/cache/blob", "cache");
        fixture.file("node-app/.turbo/state", "turbo");
        fixture.file("node-app/.parcel-cache/state", "parcel");
        fixture.file(
            "python-app/pyproject.toml",
            "[project]\nname = \"python-app\"\n",
        );
        fixture.file("python-app/.venv/pyvenv.cfg", "home = python");
        fixture.file("python-app/pkg/__pycache__/mod.pyc", "pyc");
        fixture.file("python-app/.pytest_cache/CACHEDIR.TAG", "pytest");
        fixture.file("python-app/.mypy_cache/state", "mypy");
        fixture.file("python-app/.ruff_cache/state", "ruff");
        fixture.file("python-app/.tox/state", "tox");

        let plan = ProjectScanner::new()
            .scan_roots(&[fixture.path().to_path_buf()])
            .expect("fixture scans");
        let ids = target_ids(&plan);

        for rule in [
            "rust.target",
            "node.node_modules",
            "node.next_cache",
            "node.turbo",
            "node.parcel_cache",
            "python.venv_dot",
            "python.__pycache__",
            "python.pytest_cache",
            "python.mypy_cache",
            "python.ruff_cache",
            "python.tox",
        ] {
            assert!(ids.iter().any(|id| id.starts_with(rule)), "missing {rule}");
        }
    }

    #[test]
    fn scanner_ignores_markerless_cleanup_names() {
        let fixture = Fixture::new();
        fixture.file("loose/target/debug/file", "target");
        fixture.file("loose/build/file", "build");
        fixture.file("loose/dist/file", "dist");
        fixture.file("loose/node_modules/pkg/index.js", "module");

        let plan = ProjectScanner::new()
            .scan_roots(&[fixture.path().to_path_buf()])
            .expect("fixture scans");

        assert!(plan.targets.is_empty());
    }

    #[test]
    fn scanner_does_not_follow_symlinked_project_directories() {
        let fixture = Fixture::new();
        fixture.file("outside/package.json", "{}");
        fixture.file("outside/node_modules/pkg/index.js", "module");
        fs::create_dir_all(fixture.path().join("scan")).expect("scan directory");
        let outside = fixture.path().join("outside");
        let link = fixture.path().join("scan/link-app");

        if create_dir_symlink(&outside, &link).is_err() {
            return;
        }

        let plan = ProjectScanner::new()
            .scan_roots(&[fixture.path().join("scan")])
            .expect("fixture scans");

        assert!(
            plan.targets.is_empty(),
            "scanner must not follow symlinked cleanup projects"
        );
    }

    #[cfg(windows)]
    #[test]
    fn scanner_skips_windows_reparse_point_directories() {
        let fixture = Fixture::new();
        fixture.file("outside/package.json", "{}");
        fixture.file("outside/node_modules/pkg/index.js", "module");
        fs::create_dir_all(fixture.path().join("scan")).expect("scan directory");
        let outside = fixture.path().join("outside");
        let link = fixture.path().join("scan/reparse-app");

        if create_dir_symlink(&outside, &link).is_err() {
            return;
        }

        let metadata = fs::symlink_metadata(&link).expect("link metadata");
        assert!(crate::fs_size::is_unsafe_link(&metadata));

        let plan = ProjectScanner::new()
            .scan_roots(&[fixture.path().join("scan")])
            .expect("fixture scans");

        assert!(
            plan.targets.is_empty(),
            "scanner must not follow Windows reparse point directories"
        );
    }

    #[test]
    fn scanner_output_has_plan_contract_fields_and_keeps_files() {
        let fixture = Fixture::new();
        fixture.file("node-app/package.json", "{}");
        fixture.file("node-app/node_modules/pkg/index.js", "module");
        let module_file = fixture.path().join("node-app/node_modules/pkg/index.js");

        let plan = ProjectScanner::new()
            .scan_roots(&[fixture.path().to_path_buf()])
            .expect("fixture scans");
        let target = plan.targets.first().expect("target discovered");

        assert!(module_file.exists(), "scan must not mutate fixtures");
        assert!(target.estimated_bytes > 0);
        assert!(!target.evidence.is_empty());
        assert!(matches!(target.risk, RiskLevel::Medium));
        assert!(!target.selected_by_default);
        assert!(matches!(target.action, CleanAction::MoveToTrash { .. }));

        let json = serde_json::to_value(&plan).expect("plan serializes");
        let first = &json["targets"][0];
        for key in [
            "risk",
            "evidence",
            "selected_by_default",
            "action",
            "estimated_bytes",
        ] {
            assert!(!first[key].is_null(), "missing JSON field {key}");
        }
    }

    #[test]
    fn rust_target_uses_cargo_clean_with_manifest_path() {
        let fixture = Fixture::new();
        fixture.file("rust-app/Cargo.toml", "[package]\nname = \"rust-app\"\n");
        fixture.file("rust-app/target/debug/app.bin", "binary");
        let manifest = fixture.path().join("rust-app/Cargo.toml");
        let manifest = manifest.canonicalize().expect("manifest canonicalizes");

        let plan = ProjectScanner::new()
            .scan_roots(&[fixture.path().to_path_buf()])
            .expect("fixture scans");
        let target = plan
            .targets
            .iter()
            .find(|target| target.id.as_str().starts_with("rust.target"))
            .expect("rust target discovered");

        match &target.action {
            CleanAction::Command {
                program,
                args,
                cwd: None,
                irreversible: true,
            } => {
                assert_eq!(program, "cargo");
                assert_eq!(args[0], "clean");
                assert_eq!(args[1], "--manifest-path");
                assert_eq!(PathBuf::from(&args[2]), manifest);
            }
            action => panic!("unexpected rust target action: {action:?}"),
        }
    }

    #[test]
    fn dedupe_targets_removes_nested_cleanup_paths() {
        let root = PathBuf::from("C:/workspace/app");
        let parent = build_path_target(PathTargetInput {
            rule_id: "python.venv_dot",
            ecosystem: Ecosystem::Python,
            kind: TargetKind::VirtualEnv,
            project_root: root.clone(),
            path: root.join(".venv"),
            risk: RiskLevel::Medium,
            selected_by_default: false,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "python.venv_dot".to_string(),
            }],
        });
        let child = build_path_target(PathTargetInput {
            rule_id: "python.__pycache__",
            ecosystem: Ecosystem::Python,
            kind: TargetKind::TestCache,
            project_root: root.clone(),
            path: root.join(".venv/Lib/site-packages/__pycache__"),
            risk: RiskLevel::Low,
            selected_by_default: true,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "python.__pycache__".to_string(),
            }],
        });

        let deduped = dedupe_targets(vec![child, parent]);

        assert_eq!(deduped.len(), 1);
        assert!(deduped[0].id.as_str().starts_with("python.venv_dot"));
    }

    fn target_ids(plan: &CleanupPlan) -> Vec<&str> {
        plan.targets
            .iter()
            .map(|target| target.id.as_str())
            .collect()
    }

    struct Fixture {
        temp: TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                temp: TempDir::new().expect("temp dir"),
            }
        }

        fn path(&self) -> &Path {
            self.temp.path()
        }

        fn file(&self, relative: &str, content: &str) {
            let path = self.temp.path().join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("fixture directory");
            }
            fs::write(path, content).expect("fixture file");
        }
    }

    #[cfg(unix)]
    fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_dir(target, link)
    }
}
