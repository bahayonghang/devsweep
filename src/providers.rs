use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use crate::model::{
    CLEANUP_PLAN_VERSION, CleanAction, CleanTarget, CleanupPlan, Ecosystem, Evidence, RiskLevel,
    Scope, TargetId, TargetKind,
};

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
}

fn scan_with_probe(probe: &impl ProviderProbe) -> CleanupPlan {
    let mut targets = Vec::new();
    add_npm_targets(probe, &mut targets);
    add_pip_target(probe, &mut targets);
    add_pnpm_target(probe, &mut targets);
    add_yarn_target(probe, &mut targets);
    add_cargo_home_target(probe, &mut targets);

    CleanupPlan {
        version: CLEANUP_PLAN_VERSION,
        targets,
    }
}

fn add_npm_targets(probe: &impl ProviderProbe, targets: &mut Vec<CleanTarget>) {
    let Some(npm) = probe.resolve_executable("npm") else {
        return;
    };
    let cache_path = output_path(probe.command_output(&npm, &["config", "get", "cache"]));

    targets.push(command_target(CommandTargetInput {
        rule_id: "npm.cache.verify",
        ecosystem: Ecosystem::Node,
        path: cache_path.clone(),
        risk: RiskLevel::Low,
        selected_by_default: false,
        program: npm.clone(),
        args: vec!["cache", "verify"],
        evidence: command_evidence(
            "npm.cache.verify",
            cache_path.clone(),
            "npm config get cache",
            &["npm cache verify"],
        ),
    }));
    targets.push(command_target(CommandTargetInput {
        rule_id: "npm.cache.clean",
        ecosystem: Ecosystem::Node,
        path: cache_path.clone(),
        risk: RiskLevel::Medium,
        selected_by_default: false,
        program: npm,
        args: vec!["cache", "clean", "--force"],
        evidence: command_evidence(
            "npm.cache.clean",
            cache_path,
            "npm config get cache",
            &["npm cache clean --force"],
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
    let purge_command = format!("{pip_program_name} -m pip cache purge");
    let dir_command = format!("{pip_program_name} -m pip cache dir");
    let info_command = format!("{pip_program_name} -m pip cache info");

    targets.push(command_target(CommandTargetInput {
        rule_id: "pip.cache.purge",
        ecosystem: Ecosystem::Python,
        path: cache_path.clone(),
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

    targets.push(command_target(CommandTargetInput {
        rule_id: "pnpm.store.prune",
        ecosystem: Ecosystem::Node,
        path: store_path.clone(),
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
    let path_command_display = if major_version <= 1 {
        "yarn cache dir"
    } else {
        "yarn config get cacheFolder"
    };

    targets.push(command_target(CommandTargetInput {
        rule_id,
        ecosystem: Ecosystem::Node,
        path: cache_path.clone(),
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

    targets.push(CleanTarget {
        id: TargetId::new(format!("cargo.home.inspect:{}", cargo_home.display())),
        scope: Scope::Global,
        ecosystem: Ecosystem::Rust,
        kind: TargetKind::PackageCache,
        path: Some(cargo_home.clone()),
        estimated_bytes: 0,
        last_modified: None,
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

struct CommandTargetInput<'a> {
    rule_id: &'a str,
    ecosystem: Ecosystem,
    path: Option<PathBuf>,
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
        estimated_bytes: 0,
        last_modified: None,
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

        let plan = scan_with_probe(&probe);

        assert_command(
            &plan,
            "npm.cache.verify",
            "/tools/npm",
            &["cache", "verify"],
            false,
        );
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
                !matches!(target.action, CleanAction::MoveToTrash { .. }),
                "global providers must not delete cache internals directly"
            );
        }
    }

    #[test]
    fn classic_yarn_uses_classic_cache_clean_command() {
        let mut probe = FakeProbe::default();
        let yarn = probe.tool("yarn", "/tools/yarn");
        probe.output(&yarn, &["--version"], "1.22.22\n");
        probe.output(&yarn, &["cache", "dir"], "/cache/yarn-classic\n");

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

        let plan = scan_with_probe(&probe);

        assert_eq!(plan.targets.len(), 1);
        let target = &plan.targets[0];
        assert!(target.id.as_str().starts_with("cargo.home.inspect"));
        assert_eq!(target.path, Some(PathBuf::from("/cargo")));
        assert_eq!(target.action, CleanAction::NoopInspectOnly);
        assert!(!target.selected_by_default);
        assert!(
            plan.targets
                .iter()
                .all(|target| !target.id.as_str().contains("bin")
                    && !target.id.as_str().contains("credentials"))
        );
    }

    #[derive(Default)]
    struct FakeProbe {
        tools: HashMap<String, PathBuf>,
        outputs: HashMap<(PathBuf, Vec<String>), String>,
        env: HashMap<String, PathBuf>,
        home: Option<PathBuf>,
        dirs: HashSet<PathBuf>,
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
