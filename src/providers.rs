use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Instant, SystemTime},
};

use tracing::warn;

use crate::fs_size::{SizeEstimate, estimate_tree_with_budget_and_cancel};
use crate::model::{
    CLEANUP_PLAN_VERSION, CleanAction, CleanTarget, CleanupPlan, Ecosystem, Evidence, RiskLevel,
    Scope, TargetId, TargetKind,
};
use crate::process_runner::{
    CancelObserver, CwdPolicy, DEFAULT_PROVIDER_PHASE_DEADLINE, DEFAULT_PROVIDER_PROBE_TIMEOUT,
    FlagCancelObserver, NoopCancelObserver, ProcessRequest, ProcessRunner, ProcessStatus,
};
use crate::rules::{KnownCacheAction, RuleDoc, RuleScope, global_cache_rules};

/// Docs for the procedural command providers below; each is the single source
/// of its rule's identity (id / risk / action / summary), consumed by
/// [`crate::rules::rule_catalogue`].
pub(crate) const NPM_CACHE_RULE_DOC: RuleDoc = RuleDoc {
    id: "npm.cache.clean",
    ecosystem: Ecosystem::Node,
    scope: RuleScope::Global,
    risk: RiskLevel::Medium,
    action: "official command",
    summary: "npm cache via npm cache clean --force",
};

pub(crate) const PIP_CACHE_RULE_DOC: RuleDoc = RuleDoc {
    id: "pip.cache.purge",
    ecosystem: Ecosystem::Python,
    scope: RuleScope::Global,
    risk: RiskLevel::Medium,
    action: "official command",
    summary: "pip cache via pip cache purge",
};

pub(crate) const PNPM_STORE_RULE_DOC: RuleDoc = RuleDoc {
    id: "pnpm.store.prune",
    ecosystem: Ecosystem::Node,
    scope: RuleScope::Global,
    risk: RiskLevel::Medium,
    action: "official command",
    summary: "pnpm store via pnpm store prune",
};

/// Base yarn doc; catalogue also lists classic/modern variants explicitly.
pub(crate) const YARN_CACHE_RULE_DOC: RuleDoc = RuleDoc {
    id: "yarn.cache.clean",
    ecosystem: Ecosystem::Node,
    scope: RuleScope::Global,
    risk: RiskLevel::Medium,
    action: "official command",
    summary: "yarn cache via yarn cache clean (see classic/modern variants)",
};

pub(crate) const YARN_CACHE_CLASSIC_RULE_DOC: RuleDoc = RuleDoc {
    id: "yarn.cache.clean.classic",
    ecosystem: Ecosystem::Node,
    scope: RuleScope::Global,
    risk: RiskLevel::Medium,
    action: "official command",
    summary: "yarn classic cache via yarn cache clean",
};

pub(crate) const YARN_CACHE_MODERN_RULE_DOC: RuleDoc = RuleDoc {
    id: "yarn.cache.clean.modern",
    ecosystem: Ecosystem::Node,
    scope: RuleScope::Global,
    risk: RiskLevel::Medium,
    action: "official command",
    summary: "yarn berry cache via yarn cache clean --mirror",
};

pub(crate) const CARGO_HOME_RULE_DOC: RuleDoc = RuleDoc {
    id: "cargo.home.inspect",
    ecosystem: Ecosystem::Rust,
    scope: RuleScope::Global,
    risk: RiskLevel::High,
    action: "inspect only",
    summary: "Cargo home (~/.cargo) — inspect only, never auto-deleted",
};

pub(crate) const GO_MODCACHE_RULE_DOC: RuleDoc = RuleDoc {
    id: "go.mod_cache.clean",
    ecosystem: Ecosystem::Generic,
    scope: RuleScope::Global,
    risk: RiskLevel::Medium,
    action: "official command",
    summary: "Go module cache via go clean -modcache",
};

/// All procedural rule docs declared by the global providers.
pub(crate) const PROVIDER_RULE_DOCS: &[RuleDoc] = &[
    NPM_CACHE_RULE_DOC,
    PIP_CACHE_RULE_DOC,
    PNPM_STORE_RULE_DOC,
    YARN_CACHE_RULE_DOC,
    YARN_CACHE_CLASSIC_RULE_DOC,
    YARN_CACHE_MODERN_RULE_DOC,
    CARGO_HOME_RULE_DOC,
    GO_MODCACHE_RULE_DOC,
];

pub struct GlobalProviderScanner;

impl GlobalProviderScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan(&self) -> CleanupPlan {
        self.scan_with_cancel(None)
    }

    pub fn scan_with_cancel(&self, cancel: Option<&Arc<FlagCancelObserver>>) -> CleanupPlan {
        scan_with_probe(&SystemProviderProbe::with_cancel(cancel.cloned()))
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
    fn estimate_path_size(&self, path: &Path) -> SizeEstimate;
}

struct SystemProviderProbe {
    runner: ProcessRunner,
    phase_deadline: Instant,
    cancel: Option<Arc<FlagCancelObserver>>,
}

impl SystemProviderProbe {
    fn with_cancel(cancel: Option<Arc<FlagCancelObserver>>) -> Self {
        Self {
            runner: ProcessRunner::default(),
            phase_deadline: Instant::now() + DEFAULT_PROVIDER_PHASE_DEADLINE,
            cancel,
        }
    }
}

impl ProviderProbe for SystemProviderProbe {
    fn resolve_executable(&self, program: &str) -> Option<PathBuf> {
        resolve_executable(program)
    }

    fn command_output(&self, program: &Path, args: &[&str]) -> Option<String> {
        let now = Instant::now();
        if now >= self.phase_deadline {
            warn!(
                program = %program.display(),
                args = ?args,
                "provider probe skipped: global phase deadline reached"
            );
            return None;
        }

        if self
            .cancel
            .as_ref()
            .is_some_and(|flag| flag.is_cancel_requested())
        {
            warn!(
                program = %program.display(),
                "provider probe skipped: cancel requested"
            );
            return None;
        }

        let remaining = self.phase_deadline.saturating_duration_since(now);
        let timeout = DEFAULT_PROVIDER_PROBE_TIMEOUT.min(remaining);
        let noop = NoopCancelObserver;
        let cancel: &dyn CancelObserver = match self.cancel.as_ref() {
            Some(flag) => flag.as_ref(),
            None => &noop,
        };
        let request = ProcessRequest {
            program: OsString::from(program.as_os_str()),
            args: args.iter().map(|arg| OsString::from(*arg)).collect(),
            cwd: CwdPolicy::Neutral,
            timeout: Some(timeout),
            job_deadline: Some(self.phase_deadline),
            cancel,
        };

        let result = self.runner.run(&request);
        match result.status {
            ProcessStatus::Success => {
                Some(String::from_utf8_lossy(&result.output.stdout).into_owned())
            }
            status => {
                warn!(
                    program = %program.display(),
                    args = ?args,
                    ?status,
                    stdout_truncated = result.output.stdout_truncated,
                    stderr_truncated = result.output.stderr_truncated,
                    "provider probe failed with typed process status"
                );
                None
            }
        }
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

    fn estimate_path_size(&self, path: &Path) -> SizeEstimate {
        estimate_tree_with_budget_and_cancel(
            path,
            crate::fs_size::DEFAULT_SIZE_ENTRY_BUDGET,
            self.cancel.as_ref(),
        )
    }
}

fn scan_with_probe(probe: &impl ProviderProbe) -> CleanupPlan {
    let mut targets = Vec::new();
    add_npm_targets(probe, &mut targets);
    add_pip_target(probe, &mut targets);
    add_pnpm_target(probe, &mut targets);
    add_yarn_target(probe, &mut targets);
    add_cargo_home_target(probe, &mut targets);
    add_go_modcache_target(probe, &mut targets);
    add_known_cache_targets(probe, &mut targets);

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
    let cache_estimate = estimate_path(probe, cache_path.as_deref());

    targets.push(command_target(CommandTargetInput {
        rule_id: NPM_CACHE_RULE_DOC.id,
        ecosystem: Ecosystem::Node,
        path: cache_path.clone(),
        estimated_bytes: cache_estimate.display_bytes(),
        size_complete: cache_estimate.complete,
        last_modified: cache_estimate.last_modified,
        risk: NPM_CACHE_RULE_DOC.risk,
        selected_by_default: false,
        program: npm,
        args: vec!["cache", "clean", "--force"],
        evidence: command_evidence(
            NPM_CACHE_RULE_DOC.id,
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
    let cache_estimate = estimate_path(probe, cache_path.as_deref());
    let purge_command = format!("{pip_program_name} -m pip cache purge");
    let dir_command = format!("{pip_program_name} -m pip cache dir");
    let info_command = format!("{pip_program_name} -m pip cache info");

    targets.push(command_target(CommandTargetInput {
        rule_id: PIP_CACHE_RULE_DOC.id,
        ecosystem: Ecosystem::Python,
        path: cache_path.clone(),
        estimated_bytes: cache_estimate.display_bytes(),
        size_complete: cache_estimate.complete,
        last_modified: cache_estimate.last_modified,
        risk: PIP_CACHE_RULE_DOC.risk,
        selected_by_default: false,
        program: pip_program,
        args: vec!["-m", "pip", "cache", "purge"],
        evidence: command_evidence(
            PIP_CACHE_RULE_DOC.id,
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
    let store_estimate = estimate_path(probe, store_path.as_deref());

    targets.push(command_target(CommandTargetInput {
        rule_id: PNPM_STORE_RULE_DOC.id,
        ecosystem: Ecosystem::Node,
        path: store_path.clone(),
        estimated_bytes: store_estimate.display_bytes(),
        size_complete: store_estimate.complete,
        last_modified: store_estimate.last_modified,
        risk: PNPM_STORE_RULE_DOC.risk,
        selected_by_default: false,
        program: pnpm,
        args: vec!["store", "prune"],
        evidence: command_evidence(
            PNPM_STORE_RULE_DOC.id,
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
    let (rule_suffix, path_command, clean_args, clean_command) = if major_version <= 1 {
        (
            "classic",
            vec!["cache", "dir"],
            vec!["cache", "clean"],
            "yarn cache clean",
        )
    } else {
        (
            "modern",
            vec!["config", "get", "cacheFolder"],
            vec!["cache", "clean", "--mirror"],
            "yarn cache clean --mirror",
        )
    };
    let rule_id = format!("{}.{rule_suffix}", YARN_CACHE_RULE_DOC.id);
    let cache_path = output_path(probe.command_output(&yarn, &path_command));
    let cache_estimate = estimate_path(probe, cache_path.as_deref());
    let path_command_display = if major_version <= 1 {
        "yarn cache dir"
    } else {
        "yarn config get cacheFolder"
    };

    targets.push(command_target(CommandTargetInput {
        rule_id: &rule_id,
        ecosystem: Ecosystem::Node,
        path: cache_path.clone(),
        estimated_bytes: cache_estimate.display_bytes(),
        size_complete: cache_estimate.complete,
        last_modified: cache_estimate.last_modified,
        risk: YARN_CACHE_RULE_DOC.risk,
        selected_by_default: false,
        program: yarn,
        args: clean_args,
        evidence: command_evidence(
            &rule_id,
            cache_path,
            path_command_display,
            &["yarn --version", clean_command],
        ),
    }));
}

fn add_go_modcache_target(probe: &impl ProviderProbe, targets: &mut Vec<CleanTarget>) {
    let Some(go) = probe.resolve_executable("go") else {
        return;
    };
    let (source, cache_path) = if let Some(env_path) = probe.env_path("GOMODCACHE") {
        ("GOMODCACHE", Some(env_path))
    } else {
        (
            "go env GOMODCACHE",
            output_path(probe.command_output(&go, &["env", "GOMODCACHE"])),
        )
    };
    let Some(cache_path) = cache_path.filter(|path| validated_cache_dir(probe, path)) else {
        return;
    };
    let estimate = probe.estimate_path_size(&cache_path);
    targets.push(command_target(CommandTargetInput {
        rule_id: GO_MODCACHE_RULE_DOC.id,
        ecosystem: Ecosystem::Generic,
        path: Some(cache_path.clone()),
        estimated_bytes: estimate.display_bytes(),
        size_complete: estimate.complete,
        last_modified: estimate.last_modified,
        risk: GO_MODCACHE_RULE_DOC.risk,
        selected_by_default: false,
        program: go,
        args: vec!["clean", "-modcache"],
        evidence: command_evidence(
            GO_MODCACHE_RULE_DOC.id,
            Some(cache_path),
            source,
            &["go env GOMODCACHE", "go clean -modcache"],
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

    let estimate = probe.estimate_path_size(&cargo_home);

    targets.push(CleanTarget {
        id: TargetId::new(format!(
            "{}:{}",
            CARGO_HOME_RULE_DOC.id,
            cargo_home.display()
        )),
        scope: Scope::Global,
        ecosystem: Ecosystem::Rust,
        kind: TargetKind::PackageCache,
        path: Some(cargo_home.clone()),
        estimated_bytes: estimate.display_bytes(),
        size_complete: estimate.complete,
        last_modified: estimate.last_modified,
        risk: CARGO_HOME_RULE_DOC.risk,
        reversible: true,
        selected_by_default: false,
        evidence: vec![
            Evidence::KnownCacheDir {
                source: source.to_string(),
                path: cargo_home,
            },
            Evidence::RuleMatched {
                rule_id: CARGO_HOME_RULE_DOC.id.to_string(),
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
        // Official go clean provider owns the executable action; skip the
        // inspect-only home-relative duplicate when go is available.
        if rule.id == "go.mod_cache" && probe.resolve_executable("go").is_some() {
            continue;
        }
        let path = resolve_known_cache_path(probe, &home, rule);
        if !probe.is_dir(&path) {
            continue;
        }
        let estimate = probe.estimate_path_size(&path);
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
            estimated_bytes: estimate.display_bytes(),
            size_complete: estimate.complete,
            last_modified: estimate.last_modified,
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
    size_complete: bool,
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
        size_complete,
        last_modified,
        risk,
        selected_by_default,
        program,
        args,
        evidence,
    } = input;
    let selected_by_default = selected_by_default && size_complete;
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
        size_complete,
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
    output.and_then(|output| {
        output
            .lines()
            .map(str::trim)
            .find(|line| {
                !line.is_empty()
                    && *line != "undefined"
                    && *line != "null"
                    && !line.contains(' ')
                    && !line.to_ascii_lowercase().contains("warning")
                    && !line.to_ascii_lowercase().contains("error")
            })
            .map(PathBuf::from)
            .filter(|path| is_usable_tool_path(path))
    })
}

/// Accept absolute paths, and rooted probe paths (`/cache/...`) used by tools
/// and fixtures across platforms.
fn is_usable_tool_path(path: &Path) -> bool {
    path.is_absolute()
        || path.to_string_lossy().starts_with('/')
        || path.to_string_lossy().starts_with('\\')
}

fn validated_cache_dir(probe: &impl ProviderProbe, path: &Path) -> bool {
    is_usable_tool_path(path) && probe.is_dir(path)
}

fn resolve_known_cache_path(
    probe: &impl ProviderProbe,
    home: &Path,
    rule: &crate::rules::GlobalCacheRule,
) -> PathBuf {
    match rule.id {
        "gradle.caches" | "gradle.wrapper_dists" => {
            if let Some(gradle_home) = probe.env_path("GRADLE_USER_HOME") {
                let rest = rule.relative.trim_start_matches(".gradle/");
                return gradle_home.join(rest);
            }
        }
        "nuget.packages" => {
            if let Some(nuget) = probe.env_path("NUGET_PACKAGES") {
                return nuget;
            }
        }
        "go.mod_cache" => {
            if let Some(gomod) = probe.env_path("GOMODCACHE") {
                return gomod;
            }
        }
        _ => {}
    }
    home.join(rule.relative)
}

fn estimate_path(probe: &impl ProviderProbe, path: Option<&Path>) -> SizeEstimate {
    path.map(|path| probe.estimate_path_size(path))
        .unwrap_or_else(|| SizeEstimate {
            logical_bytes: None,
            complete: false,
            last_modified: None,
            warnings: vec!["cache path was not resolved".to_string()],
        })
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
    use std::{
        cell::Cell,
        collections::{HashMap, HashSet},
        time::Duration,
    };

    use super::*;
    use crate::process_runner::{
        CwdPolicy, NoopCancelObserver, ProcessRequest, ProcessRunner, ProcessStatus,
        test_support::process_fixture_exe,
    };

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
    fn provider_scan_returns_all_discovered_targets() {
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
        assert_eq!(ids.len(), 2);
        assert!(
            ids.iter().any(|id| id.starts_with("pip.cache.purge")),
            "pip target discovered: {ids:?}"
        );
        assert!(
            ids.iter().any(|id| id.starts_with("npm.cache.clean")),
            "npm target discovered: {ids:?}"
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

    #[test]
    fn provider_probe_timeout_does_not_abort_later_providers() {
        let fixture = process_fixture_exe();

        let mut probe = TimeoutAwareProbe {
            runner: ProcessRunner::default(),
            tools: HashMap::new(),
            outputs: HashMap::new(),
            hang: HashMap::new(),
            env: HashMap::new(),
            home: None,
            dirs: HashSet::new(),
            sizes: HashMap::new(),
            phase_deadline: Instant::now() + Duration::from_secs(30),
            seen_timeout: Cell::new(false),
        };
        let npm = probe.tool("npm", fixture.to_string_lossy().as_ref());
        let py = probe.tool("py", "/tools/py");
        probe.hang.insert(
            (
                npm.clone(),
                vec!["config".to_string(), "get".to_string(), "cache".to_string()],
            ),
            true,
        );
        probe.output(&py, &["-m", "pip", "cache", "dir"], "/cache/pip\n");
        probe.path_size("/cache/pip", 200);

        let plan = scan_with_probe(&probe);

        assert!(
            probe.seen_timeout.get(),
            "npm hang fixture must surface as a typed timeout"
        );
        assert!(
            plan.targets
                .iter()
                .any(|target| target.id.as_str().starts_with("pip.cache.purge")),
            "later providers must still run after an earlier probe timeout: {:?}",
            plan.targets
                .iter()
                .map(|target| target.id.as_str())
                .collect::<Vec<_>>()
        );
        // npm still emits a command target with unresolved path when the probe
        // returns None — discovery failure is non-fatal.
        assert!(
            plan.targets
                .iter()
                .any(|target| target.id.as_str().starts_with("npm.cache.clean")),
            "npm target remains discoverable without cache path"
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

        fn estimate_path_size(&self, path: &Path) -> SizeEstimate {
            SizeEstimate::trusted(self.sizes.get(path).copied().unwrap_or_default(), None)
        }
    }

    struct TimeoutAwareProbe {
        runner: ProcessRunner,
        tools: HashMap<String, PathBuf>,
        outputs: HashMap<(PathBuf, Vec<String>), String>,
        hang: HashMap<(PathBuf, Vec<String>), bool>,
        env: HashMap<String, PathBuf>,
        home: Option<PathBuf>,
        dirs: HashSet<PathBuf>,
        sizes: HashMap<PathBuf, u64>,
        phase_deadline: Instant,
        seen_timeout: Cell<bool>,
    }

    impl TimeoutAwareProbe {
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

    impl ProviderProbe for TimeoutAwareProbe {
        fn resolve_executable(&self, program: &str) -> Option<PathBuf> {
            self.tools.get(program).cloned()
        }

        fn command_output(&self, program: &Path, args: &[&str]) -> Option<String> {
            let key = (
                program.to_path_buf(),
                args.iter().map(|arg| (*arg).to_string()).collect(),
            );
            if self.hang.get(&key).copied().unwrap_or(false) {
                let cancel = NoopCancelObserver;
                let result = self.runner.run(&ProcessRequest {
                    program: program.as_os_str().to_os_string(),
                    args: vec![OsString::from("hang")],
                    cwd: CwdPolicy::Neutral,
                    timeout: Some(Duration::from_millis(300)),
                    job_deadline: Some(self.phase_deadline),
                    cancel: &cancel,
                });
                self.seen_timeout
                    .set(result.status == ProcessStatus::Timeout);
                assert_eq!(result.status, ProcessStatus::Timeout);
                return None;
            }
            self.outputs.get(&key).cloned()
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

        fn estimate_path_size(&self, path: &Path) -> SizeEstimate {
            SizeEstimate::trusted(self.sizes.get(path).copied().unwrap_or_default(), None)
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
