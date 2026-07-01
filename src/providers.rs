use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

use crate::fs_size::estimate_tree;
use crate::model::{
    CLEANUP_PLAN_VERSION, CleanAction, CleanTarget, CleanupPlan, Ecosystem, Evidence, RiskLevel,
    Scope, TargetId, TargetKind,
};
use crate::ranking::rank_cleanup_plan;
use crate::rules::{KnownCacheAction, global_cache_rules};

pub struct GlobalProviderScanner;

impl GlobalProviderScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan(&self) -> CleanupPlan {
        scan_with_probe(&SystemProviderProbe)
    }
}

impl Default for GlobalProviderScanner {
    fn default() -> Self {
        Self::new()
    }
}

trait ProviderProbe {
    fn resolve_executable(&self, program: &str) -> Option<PathBuf>;
    fn command_output(&self, program: &Path, args: &[&str]) -> Option<String>;
    fn env_path(&self, key: &str) -> Option<PathBuf>;
    fn home_dir(&self) -> Option<PathBuf>;
    fn is_dir(&self, path: &Path) -> bool;
    fn estimate_path_size(&self, path: &Path) -> (u64, Option<SystemTime>);
}

struct SystemProviderProbe;

impl ProviderProbe for SystemProviderProbe {
    fn resolve_executable(&self, program: &str) -> Option<PathBuf> {
        resolve_executable(program)
    }

    fn command_output(&self, program: &Path, args: &[&str]) -> Option<String> {
        let output = Command::new(program).args(args).output().ok()?;
        if !output.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn env_path(&self, key: &str) -> Option<PathBuf> {
        env::var_os(key).map(PathBuf::from)
    }

    fn home_dir(&self) -> Option<PathBuf> {
        env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .or_else(|| {
                let drive = env::var_os("HOMEDRIVE")?;
                let path = env::var_os("HOMEPATH")?;
                Some(PathBuf::from(format!(
                    "{}{}",
                    drive.to_string_lossy(),
                    path.to_string_lossy()
                )))
            })
            .or_else(|| env::var_os("HOME").map(PathBuf::from))
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn estimate_path_size(&self, path: &Path) -> (u64, Option<SystemTime>) {
        estimate_tree(path)
    }
}

fn scan_with_probe(probe: &impl ProviderProbe) -> CleanupPlan {
    let mut targets = Vec::new();
    add_npm_targets(probe, &mut targets);
    add_pip_target(probe, &mut targets);
    add_pnpm_target(probe, &mut targets);
    add_yarn_target(probe, &mut targets);
    add_cargo_home_target(probe, &mut targets);
    add_known_cache_targets(probe, &mut targets);

    let mut plan = CleanupPlan {
        version: CLEANUP_PLAN_VERSION,
        targets,
    };
    rank_cleanup_plan(&mut plan);
    plan
}

fn add_npm_targets(probe: &impl ProviderProbe, targets: &mut Vec<CleanTarget>) {
    let Some(npm) = probe.resolve_executable("npm") else {
        return;
    };
    let cache_path = output_path(probe.command_output(&npm, &["config", "get", "cache"]));
    let (cache_bytes, cache_modified) = estimate_path(probe, cache_path.as_deref());

    targets.push(command_target(CommandTargetInput {
        rule_id: "npm.cache.clean",
        ecosystem: Ecosystem::Node,
        path: cache_path.clone(),
        estimated_bytes: cache_bytes,
        last_modified: cache_modified,
        risk: RiskLevel::Medium,
        selected_by_default: false,
        program: npm,
        args: vec!["cache", "clean", "--force"],
        evidence: command_evidence(
            "npm.cache.clean",
            cache_path,
            "npm config get cache",
            &["npm cache verify", "npm cache clean --force"],
        ),
    }));
}

fn add_pip_target(probe: &impl ProviderProbe, targets: &mut Vec<CleanTarget>) {
    let candidates = if cfg!(windows) {
        ["py", "python", "python3"]
    } else {
        ["python3", "python", "py"]
    };
    let Some((pip_program_name, pip_program)) = candidates
        .into_iter()
        .find_map(|name| probe.resolve_executable(name).map(|path| (name, path)))
    else {
        return;
    };
    let cache_path =
        output_path(probe.command_output(&pip_program, &["-m", "pip", "cache", "dir"]));
    let (cache_bytes, cache_modified) = estimate_path(probe, cache_path.as_deref());
    let purge_command = format!("{pip_program_name} -m pip cache purge");
    let dir_command = format!("{pip_program_name} -m pip cache dir");
    let info_command = format!("{pip_program_name} -m pip cache info");

    targets.push(command_target(CommandTargetInput {
        rule_id: "pip.cache.purge",
        ecosystem: Ecosystem::Python,
        path: cache_path.clone(),
        estimated_bytes: cache_bytes,
        last_modified: cache_modified,
        risk: RiskLevel::Medium,
        selected_by_default: false,
        program: pip_program,
        args: vec!["-m", "pip", "cache", "purge"],
        evidence: command_evidence(
            "pip.cache.purge",
            cache_path,
            &dir_command,
            &[&info_command, &purge_command],
        ),
    }));
}

fn add_pnpm_target(probe: &impl ProviderProbe, targets: &mut Vec<CleanTarget>) {
    let Some(pnpm) = probe.resolve_executable("pnpm") else {
        return;
    };
    let store_path = output_path(probe.command_output(&pnpm, &["store", "path"]));
    let (store_bytes, store_modified) = estimate_path(probe, store_path.as_deref());

    targets.push(command_target(CommandTargetInput {
        rule_id: "pnpm.store.prune",
        ecosystem: Ecosystem::Node,
        path: store_path.clone(),
        estimated_bytes: store_bytes,
        last_modified: store_modified,
        risk: RiskLevel::Low,
        selected_by_default: false,
        program: pnpm,
        args: vec!["store", "prune"],
        evidence: command_evidence(
            "pnpm.store.prune",
            store_path,
            "pnpm store path",
            &["pnpm store prune"],
        ),
    }));
}

fn add_yarn_target(probe: &impl ProviderProbe, targets: &mut Vec<CleanTarget>) {
    let Some(yarn) = probe.resolve_executable("yarn") else {
        return;
    };
    let version = probe.command_output(&yarn, &["--version"]);
    let Some(major_version) = version.as_deref().and_then(parse_major_version) else {
        return;
    };
    let (rule_id, path_command, clean_args, clean_command) = if major_version <= 1 {
        (
            "yarn.cache.clean.classic",
            vec!["cache", "dir"],
            vec!["cache", "clean"],
            "yarn cache clean",
        )
    } else {
        (
            "yarn.cache.clean.modern",
            vec!["config", "get", "cacheFolder"],
            vec!["cache", "clean", "--mirror"],
            "yarn cache clean --mirror",
        )
    };
    let cache_path = output_path(probe.command_output(&yarn, &path_command));
    let (cache_bytes, cache_modified) = estimate_path(probe, cache_path.as_deref());
    let path_command_display = if major_version <= 1 {
        "yarn cache dir"
    } else {
        "yarn config get cacheFolder"
    };

    targets.push(command_target(CommandTargetInput {
        rule_id,
        ecosystem: Ecosystem::Node,
        path: cache_path.clone(),
        estimated_bytes: cache_bytes,
        last_modified: cache_modified,
        risk: RiskLevel::Medium,
        selected_by_default: false,
        program: yarn,
        args: clean_args,
        evidence: command_evidence(
            rule_id,
            cache_path,
            path_command_display,
            &["yarn --version", clean_command],
        ),
    }));
}

fn add_cargo_home_target(probe: &impl ProviderProbe, targets: &mut Vec<CleanTarget>) {
    let (source, Some(cargo_home)) = cargo_home(probe) else {
        return;
    };
    if !probe.is_dir(&cargo_home) {
        return;
    }

    let (estimated_bytes, last_modified) = probe.estimate_path_size(&cargo_home);

    targets.push(CleanTarget {
        id: TargetId::new(format!("cargo.home.inspect:{}", cargo_home.display())),
        scope: Scope::Global,
        ecosystem: Ecosystem::Rust,
        kind: TargetKind::PackageCache,
        path: Some(cargo_home.clone()),
        estimated_bytes,
        last_modified,
        risk: RiskLevel::High,
        reversible: true,
        selected_by_default: false,
        evidence: vec![
            Evidence::KnownCacheDir {
                source: source.to_string(),
                path: cargo_home,
            },
            Evidence::RuleMatched {
                rule_id: "cargo.home.inspect".to_string(),
            },
        ],
        action: CleanAction::NoopInspectOnly,
    });
}

/// Emit targets for known home-relative cache directories that have no official
/// cleanup command (gradle / maven / go / ...). Trash rules are reversible;
/// inspect-only rules are surfaced but never auto-deleted. Never auto-selected.
fn add_known_cache_targets(probe: &impl ProviderProbe, targets: &mut Vec<CleanTarget>) {
    let Some(home) = probe.home_dir() else {
        return;
    };
    for rule in global_cache_rules() {
        let path = home.join(rule.relative);
        if !probe.is_dir(&path) {
            continue;
        }
        let (estimated_bytes, last_modified) = probe.estimate_path_size(&path);
        let action = match rule.action {
            KnownCacheAction::Trash => CleanAction::MoveToTrash { path: path.clone() },
            KnownCacheAction::InspectOnly => CleanAction::NoopInspectOnly,
        };
        targets.push(CleanTarget {
            id: TargetId::new(format!("{}:{}", rule.id, path.display())),
            scope: Scope::Global,
            ecosystem: rule.ecosystem.clone(),
            kind: rule.kind.clone(),
            path: Some(path.clone()),
            estimated_bytes,
            last_modified,
            risk: rule.risk.clone(),
            reversible: true,
            selected_by_default: false,
            evidence: vec![
                Evidence::KnownCacheDir {
                    source: rule.label.to_string(),
                    path,
                },
                Evidence::RuleMatched {
                    rule_id: rule.id.to_string(),
                },
            ],
            action,
        });
    }
}

struct CommandTargetInput<'a> {
    rule_id: &'a str,
    ecosystem: Ecosystem,
    path: Option<PathBuf>,
    estimated_bytes: u64,
    last_modified: Option<SystemTime>,
    risk: RiskLevel,
    selected_by_default: bool,
    program: PathBuf,
    args: Vec<&'a str>,
    evidence: Vec<Evidence>,
}

fn command_target(input: CommandTargetInput<'_>) -> CleanTarget {
    let CommandTargetInput {
        rule_id,
        ecosystem,
        path,
        estimated_bytes,
        last_modified,
        risk,
        selected_by_default,
        program,
        args,
        evidence,
    } = input;
    CleanTarget {
        id: TargetId::new(format!(
            "{rule_id}:{}",
            path.as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "unresolved".to_string())
        )),
        scope: Scope::Global,
        ecosystem,
        kind: TargetKind::PackageCache,
        path,
        estimated_bytes,
        last_modified,
        risk,
        reversible: false,
        selected_by_default,
        evidence,
        action: CleanAction::Command {
            program: program.to_string_lossy().into_owned(),
            args: args.into_iter().map(str::to_string).collect(),
            cwd: None,
            irreversible: true,
        },
    }
}

fn command_evidence(
    rule_id: &str,
    path: Option<PathBuf>,
    path_source: &str,
    official_commands: &[&str],
) -> Vec<Evidence> {
    let mut evidence = Vec::new();
    if let Some(path) = path {
        evidence.push(Evidence::KnownCacheDir {
            source: path_source.to_string(),
            path,
        });
    }
    evidence.extend(
        official_commands
            .iter()
            .map(|command| Evidence::OfficialCommand {
                command: (*command).to_string(),
            }),
    );
    evidence.push(Evidence::RuleMatched {
        rule_id: rule_id.to_string(),
    });
    evidence
}

fn cargo_home(probe: &impl ProviderProbe) -> (&'static str, Option<PathBuf>) {
    if let Some(path) = probe.env_path("CARGO_HOME") {
        return ("CARGO_HOME", Some(path));
    }
    (
        "default cargo home",
        probe.home_dir().map(|home| home.join(".cargo")),
    )
}

fn output_path(output: Option<String>) -> Option<PathBuf> {
    output
        .and_then(|output| {
            output
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty() && *line != "undefined" && *line != "null")
                .map(str::to_string)
        })
        .map(PathBuf::from)
}

fn estimate_path(probe: &impl ProviderProbe, path: Option<&Path>) -> (u64, Option<SystemTime>) {
    path.map(|path| probe.estimate_path_size(path))
        .unwrap_or((0, None))
}

fn parse_major_version(version: &str) -> Option<u64> {
    version
        .trim()
        .split('.')
        .next()
        .and_then(|major| major.parse().ok())
}

fn resolve_executable(program: &str) -> Option<PathBuf> {
    let program_path = Path::new(program);
    if program_path.components().count() > 1 && program_path.is_file() {
        return Some(program_path.to_path_buf());
    }

    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        #[cfg(windows)]
        {
            if program_path.extension().is_none() {
                for extension in windows_path_extensions() {
                    let candidate = dir.join(format!("{program}{extension}"));
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }

        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(windows)]
fn windows_path_extensions() -> Vec<String> {
    env::var_os("PATHEXT")
        .map(|value| {
            value
                .to_string_lossy()
                .split(';')
                .filter(|extension| !extension.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_else(|| {
            vec![
                ".COM".to_string(),
                ".EXE".to_string(),
                ".BAT".to_string(),
                ".CMD".to_string(),
            ]
        })
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;

    #[test]
    fn missing_command_providers_are_non_fatal() {
        let probe = FakeProbe::default();

        let plan = scan_with_probe(&probe);

        assert!(plan.targets.is_empty());
    }

    #[test]
    fn command_providers_emit_official_actions_when_tools_are_available() {
        let mut probe = FakeProbe::default();
        let npm = probe.tool("npm", "/tools/npm");
        let py = probe.tool("py", "/tools/py");
        let pnpm = probe.tool("pnpm", "/tools/pnpm");
        let yarn = probe.tool("yarn", "/tools/yarn");
        probe.output(&npm, &["config", "get", "cache"], "/cache/npm\n");
        probe.output(&py, &["-m", "pip", "cache", "dir"], "/cache/pip\n");
        probe.output(&pnpm, &["store", "path"], "/cache/pnpm\n");
        probe.output(&yarn, &["--version"], "4.12.0\n");
        probe.output(&yarn, &["config", "get", "cacheFolder"], "/cache/yarn\n");
        probe.path_size("/cache/npm", 100);
        probe.path_size("/cache/pip", 200);
        probe.path_size("/cache/pnpm", 300);
        probe.path_size("/cache/yarn", 400);

        let plan = scan_with_probe(&probe);

        assert_command(
            &plan,
            "npm.cache.clean",
            "/tools/npm",
            &["cache", "clean", "--force"],
            false,
        );
        assert_command(
            &plan,
            "pip.cache.purge",
            "/tools/py",
            &["-m", "pip", "cache", "purge"],
            false,
        );
        assert_command(
            &plan,
            "pnpm.store.prune",
            "/tools/pnpm",
            &["store", "prune"],
            false,
        );
        assert_command(
            &plan,
            "yarn.cache.clean.modern",
            "/tools/yarn",
            &["cache", "clean", "--mirror"],
            false,
        );

        for target in &plan.targets {
            assert_eq!(target.scope, Scope::Global);
            assert!(!target.evidence.is_empty());
            assert_ne!(target.ecosystem, Ecosystem::Docker);
            assert!(
                target.estimated_bytes > 0,
                "{} should include an estimated cache size",
                target.id.as_str()
            );
            assert!(
                !matches!(target.action, CleanAction::MoveToTrash { .. }),
                "global providers must not delete cache internals directly"
            );
        }
    }

    #[test]
    fn npm_cache_path_emits_one_cleanup_target() {
        let mut probe = FakeProbe::default();
        let npm = probe.tool("npm", "/tools/npm");
        probe.output(&npm, &["config", "get", "cache"], "/cache/npm\n");
        probe.path_size("/cache/npm", 100);

        let plan = scan_with_probe(&probe);

        let npm_targets: Vec<_> = plan
            .targets
            .iter()
            .filter(|target| target.path == Some(PathBuf::from("/cache/npm")))
            .collect();
        assert_eq!(npm_targets.len(), 1);

        let target = npm_targets[0];
        assert!(target.id.as_str().starts_with("npm.cache.clean"));
        assert!(target.evidence.iter().any(|evidence| {
            matches!(
                evidence,
                Evidence::OfficialCommand { command } if command == "npm cache verify"
            )
        }));
    }

    #[test]
    fn classic_yarn_uses_classic_cache_clean_command() {
        let mut probe = FakeProbe::default();
        let yarn = probe.tool("yarn", "/tools/yarn");
        probe.output(&yarn, &["--version"], "1.22.22\n");
        probe.output(&yarn, &["cache", "dir"], "/cache/yarn-classic\n");
        probe.path_size("/cache/yarn-classic", 512);

        let plan = scan_with_probe(&probe);

        assert_command(
            &plan,
            "yarn.cache.clean.classic",
            "/tools/yarn",
            &["cache", "clean"],
            false,
        );
        let target = find_target(&plan, "yarn.cache.clean.classic");
        assert_eq!(target.path, Some(PathBuf::from("/cache/yarn-classic")));
        assert_eq!(target.estimated_bytes, 512);
    }

    #[test]
    fn cargo_home_is_inspect_only() {
        let mut probe = FakeProbe::default();
        probe
            .env
            .insert("CARGO_HOME".to_string(), PathBuf::from("/cargo"));
        probe.dirs.insert(PathBuf::from("/cargo"));
        probe.dirs.insert(PathBuf::from("/cargo/bin"));
        probe.dirs.insert(PathBuf::from("/cargo/registry/cache"));
        probe.path_size("/cargo", 777);

        let plan = scan_with_probe(&probe);

        assert_eq!(plan.targets.len(), 1);
        let target = &plan.targets[0];
        assert!(target.id.as_str().starts_with("cargo.home.inspect"));
        assert_eq!(target.path, Some(PathBuf::from("/cargo")));
        assert_eq!(target.estimated_bytes, 777);
        assert_eq!(target.action, CleanAction::NoopInspectOnly);
        assert!(!target.selected_by_default);
        assert!(
            plan.targets
                .iter()
                .all(|target| !target.id.as_str().contains("bin")
                    && !target.id.as_str().contains("credentials"))
        );
    }

    #[test]
    fn provider_scan_returns_ranked_targets() {
        let mut probe = FakeProbe::default();
        let npm = probe.tool("npm", "/tools/npm");
        let py = probe.tool("py", "/tools/py");
        probe.output(&npm, &["config", "get", "cache"], "/cache/npm\n");
        probe.output(&py, &["-m", "pip", "cache", "dir"], "/cache/pip\n");
        probe.path_size("/cache/npm", 100);
        probe.path_size("/cache/pip", 900);

        let plan = scan_with_probe(&probe);

        let ids: Vec<_> = plan
            .targets
            .iter()
            .map(|target| target.id.as_str())
            .collect();
        assert!(
            ids[0].starts_with("pip.cache.purge"),
            "largest provider target should be first: {ids:?}"
        );
        assert!(
            ids[1].starts_with("npm.cache.clean"),
            "smaller provider target should be second: {ids:?}"
        );
    }

    #[test]
    fn known_cache_targets_emit_trash_for_present_dirs() {
        let home = PathBuf::from("/home");
        let gradle = home.join(".gradle/caches");
        let probe = FakeProbe {
            home: Some(home),
            dirs: HashSet::from([gradle.clone()]),
            sizes: HashMap::from([(gradle.clone(), 4096)]),
            ..Default::default()
        };

        let plan = scan_with_probe(&probe);

        let target = plan
            .targets
            .iter()
            .find(|target| target.id.as_str().starts_with("gradle.caches"))
            .expect("gradle cache target discovered");
        assert_eq!(target.scope, Scope::Global);
        assert_eq!(target.path, Some(gradle.clone()));
        assert_eq!(target.estimated_bytes, 4096);
        assert!(!target.selected_by_default);
        assert!(matches!(target.risk, RiskLevel::Medium));
        assert_eq!(target.action, CleanAction::MoveToTrash { path: gradle });
        assert!(target.evidence.iter().any(|evidence| matches!(
            evidence,
            Evidence::KnownCacheDir { source, .. } if source == "gradle caches"
        )));
        assert!(target.evidence.iter().any(|evidence| matches!(
            evidence,
            Evidence::RuleMatched { rule_id } if rule_id == "gradle.caches"
        )));
    }

    #[test]
    fn known_cache_inspect_rule_is_noop() {
        let inspect_rule = global_cache_rules()
            .find(|rule| rule.action == KnownCacheAction::InspectOnly)
            .expect("an inspect-only global cache rule exists");
        let home = PathBuf::from("/home");
        let path = home.join(inspect_rule.relative);
        let probe = FakeProbe {
            home: Some(home),
            dirs: HashSet::from([path.clone()]),
            sizes: HashMap::from([(path, 123)]),
            ..Default::default()
        };

        let plan = scan_with_probe(&probe);

        let target = plan
            .targets
            .iter()
            .find(|target| target.id.as_str().starts_with(inspect_rule.id))
            .expect("inspect-only cache target discovered");
        assert_eq!(target.action, CleanAction::NoopInspectOnly);
        assert!(!target.selected_by_default);
    }

    #[test]
    fn missing_home_dir_emits_no_known_cache_targets() {
        // Even with a matching directory present, no resolvable home means no
        // known-cache targets are emitted.
        let mut probe = FakeProbe::default();
        probe.dirs.insert(PathBuf::from("/home/.gradle/caches"));
        probe
            .sizes
            .insert(PathBuf::from("/home/.gradle/caches"), 4096);

        let plan = scan_with_probe(&probe);

        assert!(
            plan.targets
                .iter()
                .all(|target| !target.id.as_str().starts_with("gradle.caches")),
            "known-cache targets must require a resolvable home dir"
        );
    }

    #[derive(Default)]
    struct FakeProbe {
        tools: HashMap<String, PathBuf>,
        outputs: HashMap<(PathBuf, Vec<String>), String>,
        env: HashMap<String, PathBuf>,
        home: Option<PathBuf>,
        dirs: HashSet<PathBuf>,
        sizes: HashMap<PathBuf, u64>,
    }

    impl FakeProbe {
        fn tool(&mut self, name: &str, path: &str) -> PathBuf {
            let path = PathBuf::from(path);
            self.tools.insert(name.to_string(), path.clone());
            path
        }

        fn output(&mut self, program: &Path, args: &[&str], output: &str) {
            self.outputs.insert(
                (
                    program.to_path_buf(),
                    args.iter().map(|arg| (*arg).to_string()).collect(),
                ),
                output.to_string(),
            );
        }

        fn path_size(&mut self, path: &str, bytes: u64) {
            self.sizes.insert(PathBuf::from(path), bytes);
        }
    }

    impl ProviderProbe for FakeProbe {
        fn resolve_executable(&self, program: &str) -> Option<PathBuf> {
            self.tools.get(program).cloned()
        }

        fn command_output(&self, program: &Path, args: &[&str]) -> Option<String> {
            self.outputs
                .get(&(
                    program.to_path_buf(),
                    args.iter().map(|arg| (*arg).to_string()).collect(),
                ))
                .cloned()
        }

        fn env_path(&self, key: &str) -> Option<PathBuf> {
            self.env.get(key).cloned()
        }

        fn home_dir(&self) -> Option<PathBuf> {
            self.home.clone()
        }

        fn is_dir(&self, path: &Path) -> bool {
            self.dirs.contains(path)
        }

        fn estimate_path_size(&self, path: &Path) -> (u64, Option<SystemTime>) {
            (self.sizes.get(path).copied().unwrap_or_default(), None)
        }
    }

    fn assert_command(
        plan: &CleanupPlan,
        rule_id: &str,
        program: &str,
        args: &[&str],
        selected_by_default: bool,
    ) {
        let target = find_target(plan, rule_id);
        assert_eq!(target.selected_by_default, selected_by_default);
        assert!(matches!(target.risk, RiskLevel::Low | RiskLevel::Medium));
        match &target.action {
            CleanAction::Command {
                program: actual_program,
                args: actual_args,
                cwd: None,
                irreversible: true,
            } => {
                assert_eq!(actual_program, program);
                assert_eq!(
                    actual_args,
                    &args
                        .iter()
                        .map(|arg| (*arg).to_string())
                        .collect::<Vec<_>>()
                );
            }
            action => panic!("unexpected action for {rule_id}: {action:?}"),
        }
        assert!(
            target
                .evidence
                .iter()
                .any(|evidence| matches!(evidence, Evidence::OfficialCommand { .. })),
            "{rule_id} should include official command evidence"
        );
    }

    fn find_target<'a>(plan: &'a CleanupPlan, rule_id: &str) -> &'a CleanTarget {
        plan.targets
            .iter()
            .find(|target| target.id.as_str().starts_with(rule_id))
            .unwrap_or_else(|| panic!("missing target {rule_id}"))
    }
}
