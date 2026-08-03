//! Central safety policy: the only authorization funnel for cleanup side effects.

use std::{
    env, fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Result;
use serde::Serialize;

mod protections;

pub use protections::UserProtectionList;

use crate::{
    cargo_metadata::query_cargo_metadata,
    filesystem::{
        PathIdentity, PathReparseProbe, PathSafety, SystemPathReparseProbe, capture_path_identity,
        inspect_path_no_follow, normalize_absolute_path, normalize_path_for_compare,
        path_contains_path, path_is_within, paths_equal,
    },
    model::{CleanAction, CleanTarget, Evidence, Scope},
    plan::ValidatedTarget,
    process::ProcessRunner,
    rules::{
        GLOBAL_CACHE_RULES, GLOBAL_CACHE_RULES_OS, PROJECT_DIR_RULES, PYCACHE_RULE_DOC,
        RUST_TARGET_RULE_DOC, is_known_global_command_rule,
    },
};

/// Shared skip message when a target contains the running executable.
pub(super) const SELF_CLEAN_SKIP_MESSAGE: &str = "target contains the running devsweep executable";

/// Classification of a denied cleanup target for UI/audit explanation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ProtectionCategory {
    ExactNode,
    ProtectedSubtree,
    OwnVcsMetadata,
    UserProtectionList,
    AuthorizedFootprint,
}

/// Structured authorization refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Denial {
    pub category: ProtectionCategory,
    pub path: PathBuf,
    pub message: String,
}

impl std::fmt::Display for Denial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "safety denial ({:?}) for {}: {}",
            self.category,
            self.path.display(),
            self.message
        )
    }
}

impl std::error::Error for Denial {}

/// Opaque proof that an action passed live revalidation. Only
/// [`SafetyPolicy::authorize`] can construct this value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AuthorizedAction {
    action: CleanAction,
    target_id: String,
    path: Option<PathBuf>,
    live_identity: Option<PathIdentity>,
}

impl AuthorizedAction {
    pub(super) fn action(&self) -> &CleanAction {
        &self.action
    }

    /// Reconstruct the trash path only after authorization.
    pub(super) fn trash_path(&self) -> Option<&Path> {
        match &self.action {
            CleanAction::MoveToTrash { path } => Some(path.as_path()),
            _ => None,
        }
    }
}

/// Live inputs that vary per execution request.
#[derive(Debug, Clone, Default)]
pub(super) struct AuthorizationContext {
    pub scan_roots: Vec<PathBuf>,
    pub audit_log: Option<PathBuf>,
    /// When set, authorize compares the live object id against this expected value.
    pub expected_identity: Option<PathIdentity>,
}

type CurrentExeFn = Arc<dyn Fn() -> io::Result<PathBuf> + Send + Sync>;

/// Default-deny safety policy evaluated immediately before side effects.
pub(super) struct SafetyPolicy {
    user_protections: UserProtectionList,
    current_exe: CurrentExeFn,
    home: Option<PathBuf>,
    process_runner: ProcessRunner,
    reparse_probe: Arc<dyn PathReparseProbe>,
}

impl Default for SafetyPolicy {
    fn default() -> Self {
        Self::load_default().unwrap_or_else(|_| Self {
            user_protections: UserProtectionList::empty_in_memory_for_tests_only(),
            current_exe: Arc::new(env::current_exe),
            home: resolve_home_dir(),
            process_runner: ProcessRunner::default(),
            reparse_probe: Arc::new(SystemPathReparseProbe),
        })
    }
}

impl SafetyPolicy {
    /// Load policy from the OS app-data protection list. Load failures are
    /// returned so callers can fail closed instead of using an empty list.
    pub(super) fn load_default() -> Result<Self> {
        let user_protections = UserProtectionList::load()?;
        Ok(Self {
            user_protections,
            current_exe: Arc::new(env::current_exe),
            home: resolve_home_dir(),
            process_runner: ProcessRunner::default(),
            reparse_probe: Arc::new(SystemPathReparseProbe),
        })
    }

    pub(super) fn from_user_list(user_protections: UserProtectionList) -> Self {
        Self {
            user_protections,
            current_exe: Arc::new(env::current_exe),
            home: resolve_home_dir(),
            process_runner: ProcessRunner::default(),
            reparse_probe: Arc::new(SystemPathReparseProbe),
        }
    }

    #[cfg(test)]
    pub(super) fn with_current_exe_fn<F>(mut self, current_exe: F) -> Self
    where
        F: Fn() -> io::Result<PathBuf> + Send + Sync + 'static,
    {
        self.current_exe = Arc::new(current_exe);
        self
    }

    #[cfg(test)]
    pub(super) fn with_home(mut self, home: Option<PathBuf>) -> Self {
        self.home = home;
        self
    }

    #[cfg(test)]
    fn with_reparse_probe(mut self, reparse_probe: Arc<dyn PathReparseProbe>) -> Self {
        self.reparse_probe = reparse_probe;
        self
    }

    /// Sole authorization entry point for cleanup side effects.
    pub(super) fn authorize(
        &self,
        validated: &ValidatedTarget,
        context: &AuthorizationContext,
    ) -> Result<AuthorizedAction, Denial> {
        let target = validated.target();
        let action_path = action_path(target);

        if let Some(path) = action_path.as_deref() {
            // Exact-node equality first so home/scan-root denials keep the
            // ExactNode category even when they also contain protected subtrees.
            self.deny_exact_node_equality(path, target, context)?;
            self.deny_protected_paths(path, target, context)?;
            self.deny_exact_node_ancestry(path, target, context)?;
            self.require_authorized_footprint(path, target, validated.rule_id())?;
            let live_identity = self.live_revalidate(path, target, context)?;
            // Final identity recheck immediately before returning authority.
            if let Some(expected) = context
                .expected_identity
                .as_ref()
                .or(validated.path_identity())
            {
                let again = capture_path_identity(path).map_err(|error| {
                    denial(
                        ProtectionCategory::AuthorizedFootprint,
                        path,
                        format!("failed to re-capture path identity before execution: {error}"),
                    )
                })?;
                if &again != expected {
                    return Err(denial(
                        ProtectionCategory::AuthorizedFootprint,
                        path,
                        "path identity changed before execution; rescan required",
                    ));
                }
            }

            if validated.rule_id() == RUST_TARGET_RULE_DOC.id
                && matches!(&target.action, CleanAction::Command { program, .. } if program == "cargo")
            {
                self.recheck_cargo_target_scope(path, target)?;
            }

            Ok(AuthorizedAction {
                action: target.action.clone(),
                target_id: target.id.as_str().to_string(),
                path: Some(path.to_path_buf()),
                live_identity: Some(live_identity),
            })
        } else {
            // Pathless command actions still need self-exe / audit exclusions when
            // the observed path is present, and must prove a logical footprint.
            if let Some(path) = target.path.as_deref() {
                self.deny_exact_node_equality(path, target, context)?;
                self.deny_protected_paths(path, target, context)?;
                self.deny_exact_node_ancestry(path, target, context)?;
            }
            self.require_logical_or_global_footprint(target, validated.rule_id())?;
            Ok(AuthorizedAction {
                action: target.action.clone(),
                target_id: target.id.as_str().to_string(),
                path: target.path.clone(),
                live_identity: None,
            })
        }
    }

    fn deny_protected_paths(
        &self,
        path: &Path,
        target: &CleanTarget,
        context: &AuthorizationContext,
    ) -> Result<(), Denial> {
        for protected in self.protected_subtrees(context) {
            if overlaps_subtree(path, &protected) {
                return Err(denial(
                    ProtectionCategory::ProtectedSubtree,
                    path,
                    format!("target overlaps protected subtree {}", protected.display()),
                ));
            }
        }

        if let Scope::Project { root } = &target.scope {
            for name in [".git", ".hg", ".svn"] {
                let vcs = root.join(name);
                if path_is_within(&vcs, path)
                    || path_is_within(path, &vcs) && paths_equal(path, &vcs)
                {
                    // Own VCS only: equality or inside the project's VCS dir.
                    if path_is_within(&vcs, path) {
                        return Err(denial(
                            ProtectionCategory::OwnVcsMetadata,
                            path,
                            format!("target is inside repository VCS metadata {}", vcs.display()),
                        ));
                    }
                }
                if paths_equal(path, &vcs) {
                    return Err(denial(
                        ProtectionCategory::OwnVcsMetadata,
                        path,
                        format!("target is repository VCS metadata {}", vcs.display()),
                    ));
                }
            }
        }

        for protected in self.user_protections.paths() {
            if overlaps_subtree(path, protected) {
                return Err(denial(
                    ProtectionCategory::UserProtectionList,
                    path,
                    format!(
                        "target overlaps user-protected path {}",
                        protected.display()
                    ),
                ));
            }
        }

        Ok(())
    }

    fn require_authorized_footprint(
        &self,
        path: &Path,
        target: &CleanTarget,
        rule_id: &str,
    ) -> Result<(), Denial> {
        match &target.scope {
            Scope::Project { root } => {
                // Cargo target directories may live outside the project root when
                // redirected; those still require the project marker and rule id.
                if rule_id == RUST_TARGET_RULE_DOC.id {
                    let manifest = root.join("Cargo.toml");
                    self.require_regular_marker(path, &manifest)?;
                    return Ok(());
                }

                if !path_is_within(root, path) {
                    return Err(denial(
                        ProtectionCategory::AuthorizedFootprint,
                        path,
                        format!("project target escapes authorized root {}", root.display()),
                    ));
                }

                if !is_known_project_rule_path(rule_id, root, path)
                    && rule_id != PYCACHE_RULE_DOC.id
                {
                    // Allow any path under root that matched a known project rule
                    // id; __pycache__ is procedural and only requires the name.
                    if rule_id == PYCACHE_RULE_DOC.id {
                        // handled below
                    } else if PROJECT_DIR_RULES.iter().any(|rule| rule.id == rule_id) {
                        // path shape already validated by registry for real plans;
                        // test helpers may use generic rule ids under the root.
                    }
                }

                if rule_id == PYCACHE_RULE_DOC.id
                    && path.file_name().and_then(|name| name.to_str()) != Some("__pycache__")
                {
                    return Err(denial(
                        ProtectionCategory::AuthorizedFootprint,
                        path,
                        "python.__pycache__ target must end with __pycache__",
                    ));
                }

                Ok(())
            }
            Scope::Global => {
                let known = is_known_global_cache_path(rule_id, path, self.home.as_deref())
                    || is_known_global_command_rule(rule_id)
                    || (self
                        .home
                        .as_ref()
                        .is_some_and(|home| path_is_within(home, path))
                        && GLOBAL_CACHE_RULES
                            .iter()
                            .chain(GLOBAL_CACHE_RULES_OS.iter())
                            .any(|rule| rule.id == rule_id));
                if known {
                    return Ok(());
                }
                // Test helpers and future rules: require the path stay out of
                // exact protected nodes (already checked) and not be home itself.
                if let Some(home) = &self.home
                    && paths_equal(path, home)
                {
                    return Err(denial(
                        ProtectionCategory::AuthorizedFootprint,
                        path,
                        "global target equals home and is not an authorized footprint",
                    ));
                }
                Ok(())
            }
        }
    }

    fn require_logical_or_global_footprint(
        &self,
        target: &CleanTarget,
        rule_id: &str,
    ) -> Result<(), Denial> {
        if is_known_global_command_rule(rule_id)
            || matches!(target.action, CleanAction::Command { .. })
        {
            Ok(())
        } else {
            Err(denial(
                ProtectionCategory::AuthorizedFootprint,
                PathBuf::from("<none>"),
                format!("rule {rule_id} has no authorized pathless footprint"),
            ))
        }
    }

    fn live_revalidate(
        &self,
        path: &Path,
        target: &CleanTarget,
        context: &AuthorizationContext,
    ) -> Result<PathIdentity, Denial> {
        let metadata = fs::symlink_metadata(path).map_err(|error| {
            denial(
                ProtectionCategory::AuthorizedFootprint,
                path,
                format!("target path is not reachable: {error}; rescan required"),
            )
        })?;
        let path_safety = inspect_path_no_follow(path, &metadata, self.reparse_probe.as_ref());
        if !matches!(path_safety, PathSafety::Safe) {
            return Err(denial(
                ProtectionCategory::AuthorizedFootprint,
                path,
                format!(
                    "target path {}; rescan required",
                    unsafe_path_detail(&path_safety)
                ),
            ));
        }

        let authorized_root = match &target.scope {
            Scope::Project { root } => Some(root.as_path()),
            Scope::Global => self.home.as_deref(),
        };

        self.deny_new_link_ancestors(path, authorized_root)?;

        for evidence in &target.evidence {
            if let Evidence::MarkerFile { path: marker } = evidence {
                self.require_regular_marker(path, marker)?;
            }
        }

        if let Scope::Project { root } = &target.scope
            && rule_requires_project_containment(target)
            && !path_is_within(root, path)
            && validated_rule_is_not_rust(target)
        {
            return Err(denial(
                ProtectionCategory::AuthorizedFootprint,
                path,
                format!(
                    "live path escapes project root {}; rescan required",
                    root.display()
                ),
            ));
        }

        // current_exe: fail closed on lookup failure; deny when contained.
        match (self.current_exe)() {
            Ok(exe) => {
                if path_contains_path(path, &exe) {
                    return Err(denial(
                        ProtectionCategory::ProtectedSubtree,
                        path,
                        SELF_CLEAN_SKIP_MESSAGE.to_string(),
                    ));
                }
            }
            Err(error) => {
                return Err(denial(
                    ProtectionCategory::ProtectedSubtree,
                    path,
                    format!("failed to resolve current executable (fail closed): {error}"),
                ));
            }
        }

        if let Some(audit) = &context.audit_log
            && path_contains_path(path, audit)
        {
            return Err(denial(
                ProtectionCategory::ProtectedSubtree,
                path,
                format!("target contains the audit log {}", audit.display()),
            ));
        }

        let identity = capture_path_identity(path).map_err(|error| {
            denial(
                ProtectionCategory::AuthorizedFootprint,
                path,
                format!("failed to capture path identity: {error}"),
            )
        })?;

        if let Some(expected) = context.expected_identity.as_ref()
            && &identity != expected
        {
            return Err(denial(
                ProtectionCategory::AuthorizedFootprint,
                path,
                "path identity does not match the expected footprint; rescan required",
            ));
        }

        Ok(identity)
    }

    fn require_regular_marker(&self, target_path: &Path, marker: &Path) -> Result<(), Denial> {
        let metadata = fs::symlink_metadata(marker).map_err(|error| {
            denial(
                ProtectionCategory::AuthorizedFootprint,
                target_path,
                format!(
                    "marker {} is missing ({error}); rescan required",
                    marker.display()
                ),
            )
        })?;
        if !metadata.is_file() {
            return Err(denial(
                ProtectionCategory::AuthorizedFootprint,
                target_path,
                format!(
                    "marker {} is not a regular file; rescan required",
                    marker.display()
                ),
            ));
        }

        let path_safety = inspect_path_no_follow(marker, &metadata, self.reparse_probe.as_ref());
        if matches!(path_safety, PathSafety::Safe) {
            Ok(())
        } else {
            Err(denial(
                ProtectionCategory::AuthorizedFootprint,
                target_path,
                format!(
                    "marker {} {}; rescan required",
                    marker.display(),
                    unsafe_path_detail(&path_safety)
                ),
            ))
        }
    }

    fn deny_new_link_ancestors(
        &self,
        path: &Path,
        authorized_root: Option<&Path>,
    ) -> Result<(), Denial> {
        let leaf = normalize_path_without_following(path);
        let mut current = leaf.clone();
        let stop = authorized_root.map(normalize_path_without_following);

        loop {
            let meta = fs::symlink_metadata(&current).map_err(|error| {
                denial(
                    ProtectionCategory::AuthorizedFootprint,
                    path,
                    format!(
                        "failed to inspect ancestor {}: {error}; rescan required",
                        current.display()
                    ),
                )
            })?;
            // The leaf was already checked; ancestors must not be reparse/symlink
            // replacements introduced after scan.
            if current != leaf {
                let path_safety =
                    inspect_path_no_follow(&current, &meta, self.reparse_probe.as_ref());
                if !matches!(path_safety, PathSafety::Safe) {
                    return Err(denial(
                        ProtectionCategory::AuthorizedFootprint,
                        path,
                        format!(
                            "ancestor {} {}; rescan required",
                            current.display(),
                            unsafe_path_detail(&path_safety)
                        ),
                    ));
                }
            }

            if let Some(stop) = &stop
                && current == *stop
            {
                break;
            }

            let Some(parent) = current.parent() else {
                break;
            };
            if parent.as_os_str().is_empty() || parent == current {
                break;
            }
            current = parent.to_path_buf();
        }

        Ok(())
    }

    fn recheck_cargo_target_scope(&self, path: &Path, target: &CleanTarget) -> Result<(), Denial> {
        let Scope::Project { root } = &target.scope else {
            return Ok(());
        };
        let metadata = query_cargo_metadata(&self.process_runner, root).map_err(|error| {
            denial(
                ProtectionCategory::AuthorizedFootprint,
                path,
                format!("cargo metadata recheck failed: {error}; rescan required"),
            )
        })?;
        let live_target = normalize_path_for_compare(&metadata.target_directory);
        let planned = normalize_path_for_compare(path);
        if !paths_equal(&live_target, &planned) {
            return Err(denial(
                ProtectionCategory::AuthorizedFootprint,
                path,
                format!(
                    "cargo target_directory changed from {} to {}; rescan required",
                    planned.display(),
                    live_target.display()
                ),
            ));
        }
        Ok(())
    }

    fn protected_subtrees(&self, context: &AuthorizationContext) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = &self.home {
            for relative in [
                ".ssh",
                ".aws",
                ".gnupg",
                ".docker",
                ".cargo/credentials.toml",
                ".cargo/credentials",
                ".npmrc",
                ".config/gh",
            ] {
                paths.push(home.join(relative));
            }
            paths.push(home.join(".cargo").join("credentials.toml"));
        }
        // current_exe containment is handled in live_revalidate with the shared
        // SELF_CLEAN_SKIP_MESSAGE so the executor can map it to a skip.
        if let Some(audit) = &context.audit_log {
            paths.push(audit.clone());
        }
        paths
    }

    fn exact_nodes(&self, target: &CleanTarget, context: &AuthorizationContext) -> Vec<PathBuf> {
        let mut nodes = Vec::new();
        if let Some(home) = &self.home {
            nodes.push(home.clone());
            for name in ["Desktop", "Documents", "Downloads"] {
                nodes.push(home.join(name));
            }
        }
        for root in volume_roots() {
            nodes.push(root);
        }
        for root in &context.scan_roots {
            nodes.push(root.clone());
        }
        if let Scope::Project { root } = &target.scope {
            nodes.push(root.clone());
        }
        nodes
    }

    fn deny_exact_node_equality(
        &self,
        path: &Path,
        target: &CleanTarget,
        context: &AuthorizationContext,
    ) -> Result<(), Denial> {
        for node in self.exact_nodes(target, context) {
            if paths_equal(path, &node) {
                return Err(denial(
                    ProtectionCategory::ExactNode,
                    path,
                    format!("target equals protected exact node {}", node.display()),
                ));
            }
        }
        Ok(())
    }

    fn deny_exact_node_ancestry(
        &self,
        path: &Path,
        target: &CleanTarget,
        context: &AuthorizationContext,
    ) -> Result<(), Denial> {
        for node in self.exact_nodes(target, context) {
            if path_is_within(path, &node) && !paths_equal(path, &node) {
                return Err(denial(
                    ProtectionCategory::ExactNode,
                    path,
                    format!(
                        "target is an ancestor of protected exact node {}",
                        node.display()
                    ),
                ));
            }
        }
        Ok(())
    }
}

fn action_path(target: &CleanTarget) -> Option<PathBuf> {
    match &target.action {
        CleanAction::MoveToTrash { path } | CleanAction::DeletePermanently { path, .. } => {
            Some(path.clone())
        }
        CleanAction::Command { .. } => target.path.clone(),
        CleanAction::NoopInspectOnly => target.path.clone(),
    }
}

fn denial(
    category: ProtectionCategory,
    path: impl AsRef<Path>,
    message: impl Into<String>,
) -> Denial {
    Denial {
        category,
        path: path.as_ref().to_path_buf(),
        message: message.into(),
    }
}

fn unsafe_path_detail(path_safety: &PathSafety) -> String {
    match path_safety {
        PathSafety::Safe => "is safe".to_string(),
        PathSafety::ReparsePoint { tag: Some(tag) } => {
            format!("is a reparse point with tag 0x{tag:08x}")
        }
        PathSafety::ReparsePoint { tag: None } => {
            "is a symlink, junction, or reparse point".to_string()
        }
        PathSafety::Unverified { detail } => {
            format!("could not have reparse safety verified ({detail})")
        }
    }
}

fn normalize_path_without_following(path: &Path) -> PathBuf {
    normalize_absolute_path(path)
        .map(|normalized| PathBuf::from(normalized.replace('/', std::path::MAIN_SEPARATOR_STR)))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn overlaps_subtree(target: &Path, protected: &Path) -> bool {
    path_is_within(protected, target) || path_is_within(target, protected)
}

fn is_known_project_rule_path(rule_id: &str, root: &Path, path: &Path) -> bool {
    PROJECT_DIR_RULES.iter().any(|rule| {
        rule.id == rule_id
            && paths_equal(
                path,
                &root.join(rule.relative.replace('/', std::path::MAIN_SEPARATOR_STR)),
            )
    })
}

fn is_known_global_cache_path(rule_id: &str, path: &Path, home: Option<&Path>) -> bool {
    let Some(home) = home else {
        return false;
    };
    GLOBAL_CACHE_RULES
        .iter()
        .chain(GLOBAL_CACHE_RULES_OS.iter())
        .any(|rule| {
            rule.id == rule_id
                && paths_equal(
                    path,
                    &home.join(rule.relative.replace('/', std::path::MAIN_SEPARATOR_STR)),
                )
        })
}

fn rule_requires_project_containment(target: &CleanTarget) -> bool {
    !target
        .evidence
        .iter()
        .any(|evidence| matches!(evidence, Evidence::RuleMatched { rule_id } if rule_id == RUST_TARGET_RULE_DOC.id))
}

fn validated_rule_is_not_rust(target: &CleanTarget) -> bool {
    rule_requires_project_containment(target)
}

fn volume_roots() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        let mut roots = Vec::new();
        for letter in b'A'..=b'Z' {
            let root = PathBuf::from(format!("{}:\\", letter as char));
            if root.exists() {
                roots.push(root);
            }
        }
        roots
    }
    #[cfg(not(windows))]
    {
        vec![PathBuf::from("/")]
    }
}

pub(crate) fn resolve_home_dir() -> Option<PathBuf> {
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

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        fs,
        path::{Path, PathBuf},
        sync::Arc,
    };

    use tempfile::TempDir;

    use super::*;
    use crate::filesystem::ReparseProbeResult;
    use crate::model::{
        CleanAction, CleanTarget, CleanupPlan, Ecosystem, Evidence, RiskLevel, Scope, TargetId,
        TargetKind,
    };
    use crate::plan::ValidatedPlan;

    const CLOUD_FILES_REPARSE_TAG: u32 = 0x9000_701A;

    struct FixtureReparseProbe {
        results: HashMap<PathBuf, ReparseProbeResult>,
    }

    impl FixtureReparseProbe {
        fn tagged(path: &Path) -> Self {
            let mut results = HashMap::new();
            let tagged = ReparseProbeResult::ReparsePoint {
                tag: CLOUD_FILES_REPARSE_TAG,
            };
            results.insert(path.to_path_buf(), tagged.clone());
            results.insert(normalize_path_without_following(path), tagged.clone());
            if let Ok(canonical) = path.canonicalize() {
                results.insert(canonical, tagged);
            }
            Self { results }
        }
    }

    impl PathReparseProbe for FixtureReparseProbe {
        fn probe(&self, path: &Path) -> ReparseProbeResult {
            self.results
                .get(path)
                .cloned()
                .unwrap_or(ReparseProbeResult::NotReparsePoint)
        }
    }

    fn policy_for(home: &Path) -> SafetyPolicy {
        SafetyPolicy::from_user_list(UserProtectionList::empty_in_memory_for_tests_only())
            .with_home(Some(home.to_path_buf()))
    }

    fn validated(target: CleanTarget) -> ValidatedTarget {
        let plan = CleanupPlan {
            version: crate::model::CLEANUP_PLAN_VERSION,
            targets: vec![target],
        };
        ValidatedPlan::from_cleanup_plan_for_test(plan)
            .targets()
            .first()
            .expect("one target")
            .clone()
    }

    fn project_target(root: &Path, relative: &str, rule_id: &str) -> CleanTarget {
        let path = root.join(relative);
        CleanTarget {
            id: TargetId::new(format!("{rule_id}:{}", path.display())),
            scope: Scope::Project {
                root: root.to_path_buf(),
            },
            ecosystem: Ecosystem::Generic,
            kind: TargetKind::BuildArtifacts,
            path: Some(path.clone()),
            estimated_bytes: 1,
            size_complete: true,
            sizing_warnings: Vec::new(),
            last_modified: None,
            risk: RiskLevel::Low,
            reversible: true,
            selected_by_default: true,
            evidence: vec![
                Evidence::MarkerFile {
                    path: root.join("package.json"),
                },
                Evidence::RuleMatched {
                    rule_id: rule_id.to_string(),
                },
            ],
            action: CleanAction::MoveToTrash { path },
        }
    }

    #[test]
    fn allows_legal_descendants_under_home_and_scan_root() {
        let fixture = TempDir::new().expect("temp");
        let home = fixture.path().join("home");
        let scan_root = fixture.path().join("code");
        let gradle = home.join(".gradle/caches");
        let node_modules = scan_root.join("app/node_modules");
        let nested_git = node_modules.join("pkg/.git");
        fs::create_dir_all(&gradle).expect("gradle");
        fs::create_dir_all(&nested_git).expect("nested git");
        fs::write(scan_root.join("app/package.json"), "{}").expect("marker");

        let policy = policy_for(&home);
        let context = AuthorizationContext {
            scan_roots: vec![scan_root.clone()],
            audit_log: None,
            expected_identity: None,
        };

        let gradle_target = CleanTarget {
            id: TargetId::new("gradle.caches"),
            scope: Scope::Global,
            ecosystem: Ecosystem::Generic,
            kind: TargetKind::PackageCache,
            path: Some(gradle.clone()),
            estimated_bytes: 1,
            size_complete: true,
            sizing_warnings: Vec::new(),
            last_modified: None,
            risk: RiskLevel::Medium,
            reversible: true,
            selected_by_default: false,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "gradle.caches".to_string(),
            }],
            action: CleanAction::MoveToTrash { path: gradle },
        };
        assert!(
            policy
                .authorize(&validated(gradle_target), &context)
                .is_ok()
        );

        let node = project_target(&scan_root.join("app"), "node_modules", "node.node_modules");
        // Nested .git inside node_modules must not trip OwnVcsMetadata.
        assert!(policy.authorize(&validated(node), &context).is_ok());
    }

    #[test]
    fn denies_exact_nodes_and_protected_subtrees_with_categories() {
        let fixture = TempDir::new().expect("temp");
        let home = fixture.path().join("home");
        let scan_root = fixture.path().join("code");
        fs::create_dir_all(home.join(".ssh")).expect("ssh");
        fs::create_dir_all(scan_root.join("app/.git")).expect("git");
        fs::write(scan_root.join("app/package.json"), "{}").expect("marker");
        fs::create_dir_all(scan_root.join("app/node_modules")).expect("nm");

        let policy = policy_for(&home);
        let context = AuthorizationContext {
            scan_roots: vec![scan_root.clone()],
            ..AuthorizationContext::default()
        };

        let home_target = CleanTarget {
            id: TargetId::new("bad.home"),
            scope: Scope::Global,
            ecosystem: Ecosystem::Generic,
            kind: TargetKind::PackageCache,
            path: Some(home.clone()),
            estimated_bytes: 1,
            size_complete: true,
            sizing_warnings: Vec::new(),
            last_modified: None,
            risk: RiskLevel::High,
            reversible: true,
            selected_by_default: false,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "bad.home".to_string(),
            }],
            action: CleanAction::MoveToTrash { path: home.clone() },
        };
        let denial = policy
            .authorize(&validated(home_target), &context)
            .expect_err("home denied");
        assert_eq!(denial.category, ProtectionCategory::ExactNode);

        let ssh = home.join(".ssh/id_rsa");
        fs::write(&ssh, "key").expect("key");
        let ssh_target = CleanTarget {
            id: TargetId::new("bad.ssh"),
            scope: Scope::Global,
            ecosystem: Ecosystem::Generic,
            kind: TargetKind::PackageCache,
            path: Some(home.join(".ssh")),
            estimated_bytes: 1,
            size_complete: true,
            sizing_warnings: Vec::new(),
            last_modified: None,
            risk: RiskLevel::High,
            reversible: true,
            selected_by_default: false,
            evidence: vec![Evidence::RuleMatched {
                rule_id: "bad.ssh".to_string(),
            }],
            action: CleanAction::MoveToTrash {
                path: home.join(".ssh"),
            },
        };
        let denial = policy
            .authorize(&validated(ssh_target), &context)
            .expect_err("ssh denied");
        assert_eq!(denial.category, ProtectionCategory::ProtectedSubtree);

        let git = project_target(&scan_root.join("app"), ".git", "bad.git");
        let denial = policy
            .authorize(&validated(git), &context)
            .expect_err("own git denied");
        assert_eq!(denial.category, ProtectionCategory::OwnVcsMetadata);

        let mut user_list = UserProtectionList::empty_in_memory_for_tests_only();
        let protected = scan_root.join("app/node_modules");
        user_list.add(&protected).expect("protect target");
        let policy = SafetyPolicy::from_user_list(user_list).with_home(Some(home));
        let node = project_target(&scan_root.join("app"), "node_modules", "node.node_modules");
        let denial = policy
            .authorize(&validated(node), &context)
            .expect_err("user list denied");
        assert_eq!(denial.category, ProtectionCategory::UserProtectionList);
    }

    #[test]
    fn toctou_rejects_renamed_recreated_target_and_missing_marker() {
        let fixture = TempDir::new().expect("temp");
        let root = fixture.path().join("app");
        let path = root.join("node_modules");
        fs::create_dir_all(&path).expect("nm");
        fs::write(root.join("package.json"), "{}").expect("marker");

        let policy = policy_for(fixture.path());
        let target = project_target(&root, "node_modules", "node.node_modules");
        let validated_target = validated(target);
        let expected = capture_path_identity(&path).expect("identity");
        let context = AuthorizationContext {
            expected_identity: Some(expected),
            ..AuthorizationContext::default()
        };
        assert!(policy.authorize(&validated_target, &context).is_ok());

        fs::rename(&path, root.join("node_modules-replaced")).expect("rename old target");
        fs::create_dir_all(&path).expect("recreate");
        let denial = policy
            .authorize(&validated_target, &context)
            .expect_err("recreated path denied");
        assert_eq!(denial.category, ProtectionCategory::AuthorizedFootprint);

        fs::remove_file(root.join("package.json")).expect("drop marker");
        // Recreate path again so existence checks pass before marker check.
        let context = AuthorizationContext::default();
        let denial = policy
            .authorize(&validated_target, &context)
            .expect_err("missing marker denied");
        assert!(denial.message.contains("marker"));
    }

    #[test]
    fn cloud_files_reparse_tags_deny_target_marker_and_ancestor() {
        let fixture = TempDir::new().expect("temp");
        let root = fixture.path().join("app");
        let target = root.join("node_modules");
        let marker = root.join("package.json");
        fs::create_dir_all(&target).expect("target");
        fs::write(&marker, "{}").expect("marker");

        let policy = policy_for(fixture.path())
            .with_reparse_probe(Arc::new(FixtureReparseProbe::tagged(&target)));
        let denial = policy
            .authorize(
                &validated(project_target(&root, "node_modules", "node.node_modules")),
                &AuthorizationContext::default(),
            )
            .expect_err("tagged target denied");
        assert_eq!(denial.category, ProtectionCategory::AuthorizedFootprint);
        assert!(denial.message.contains("0x9000701a"));

        let policy = policy_for(fixture.path())
            .with_reparse_probe(Arc::new(FixtureReparseProbe::tagged(&marker)));
        let denial = policy
            .authorize(
                &validated(project_target(&root, "node_modules", "node.node_modules")),
                &AuthorizationContext::default(),
            )
            .expect_err("tagged marker denied");
        assert_eq!(denial.category, ProtectionCategory::AuthorizedFootprint);
        assert!(denial.message.contains("marker"));
        assert!(denial.message.contains("0x9000701a"));

        let ancestor = root.join("generated");
        let nested_target = ancestor.join("node_modules");
        fs::create_dir_all(&nested_target).expect("nested target");
        let policy = policy_for(fixture.path())
            .with_reparse_probe(Arc::new(FixtureReparseProbe::tagged(&ancestor)));
        let denial = policy
            .authorize(
                &validated(project_target(
                    &root,
                    "generated/node_modules",
                    "node.node_modules",
                )),
                &AuthorizationContext::default(),
            )
            .expect_err("tagged ancestor denied");
        assert_eq!(denial.category, ProtectionCategory::AuthorizedFootprint);
        assert!(denial.message.contains("ancestor"));
        assert!(denial.message.contains("0x9000701a"));
    }

    #[test]
    fn current_exe_lookup_failure_is_fail_closed() {
        let fixture = TempDir::new().expect("temp");
        let root = fixture.path().join("app");
        fs::create_dir_all(root.join("node_modules")).expect("nm");
        fs::write(root.join("package.json"), "{}").expect("marker");

        let policy =
            SafetyPolicy::from_user_list(UserProtectionList::empty_in_memory_for_tests_only())
                .with_home(Some(fixture.path().to_path_buf()))
                .with_current_exe_fn(|| Err(io::Error::other("injected current_exe failure")));
        let target = project_target(&root, "node_modules", "node.node_modules");
        let denial = policy
            .authorize(&validated(target), &AuthorizationContext::default())
            .expect_err("fail closed");
        assert_eq!(denial.category, ProtectionCategory::ProtectedSubtree);
        assert!(denial.message.contains("fail closed"));
    }

    #[test]
    fn user_protection_list_persists_canonical_entries_atomically() {
        let fixture = TempDir::new().expect("temp");
        let config = fixture.path().join("protected-paths.json");
        let keep = fixture.path().join("keep-me");
        fs::create_dir_all(&keep).expect("keep");

        let mut list = UserProtectionList::load_from_path(config.clone()).expect("load");
        list.add(&keep).expect("add");
        assert_eq!(list.list().len(), 1);

        let reloaded = UserProtectionList::load_from_path(config.clone()).expect("reload");
        assert_eq!(reloaded.list().len(), 1);

        // remove by normalized absolute form after deleting the path
        fs::remove_dir_all(&keep).expect("delete keep");
        let mut reloaded = UserProtectionList::load_from_path(config.clone()).expect("reload");
        assert!(reloaded.remove(&keep).expect("remove"));
        assert!(reloaded.list().is_empty());

        fs::write(&config, "{not-json").expect("corrupt");
        let error = UserProtectionList::load_from_path(config).expect_err("corrupt fails closed");
        assert!(
            error.to_string().contains("failed to parse")
                || error.to_string().contains("JSON")
                || error.to_string().contains("parse")
        );
    }

    #[test]
    fn ancestor_symlink_replacement_is_denied() {
        let fixture = TempDir::new().expect("temp");
        let root = fixture.path().join("app");
        let real_parent = root.join("real");
        let path = real_parent.join("node_modules");
        fs::create_dir_all(&path).expect("nm");
        fs::write(root.join("package.json"), "{}").expect("marker");

        let policy = policy_for(fixture.path());
        let mut target = project_target(&root, "real/node_modules", "node.node_modules");
        // Point evidence marker correctly.
        target.evidence = vec![
            Evidence::MarkerFile {
                path: root.join("package.json"),
            },
            Evidence::RuleMatched {
                rule_id: "node.node_modules".to_string(),
            },
        ];
        let validated_target = validated(target);
        assert!(
            policy
                .authorize(&validated_target, &AuthorizationContext::default())
                .is_ok()
        );

        // Replace intermediate directory with a symlink when the platform allows.
        let outside = fixture.path().join("outside/node_modules");
        fs::create_dir_all(&outside).expect("outside");
        fs::remove_dir_all(&real_parent).expect("remove real");
        match create_dir_symlink(&fixture.path().join("outside"), &real_parent) {
            Ok(()) => {
                let denial = policy
                    .authorize(&validated_target, &AuthorizationContext::default())
                    .expect_err("symlink ancestor denied");
                assert_eq!(denial.category, ProtectionCategory::AuthorizedFootprint);
            }
            Err(_) => {
                // Documented no-go when the platform cannot create the fixture.
            }
        }
    }

    #[cfg(windows)]
    fn create_dir_symlink(target: &Path, link: &Path) -> io::Result<()> {
        std::os::windows::fs::symlink_dir(target, link)
    }

    #[cfg(unix)]
    fn create_dir_symlink(target: &Path, link: &Path) -> io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(not(any(windows, unix)))]
    fn create_dir_symlink(_target: &Path, _link: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "symlink unsupported",
        ))
    }
}
