use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};

use super::GeneratedProjectRule;
use crate::{
    filesystem::{PathSafety, SystemPathReparseProbe, inspect_path_no_follow},
    process::{CwdPolicy, NoopCancelObserver, ProcessRequest, ProcessRunner, ProcessStatus},
};

fn real_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| {
        metadata.is_file()
            && matches!(
                inspect_path_no_follow(path, &metadata, &SystemPathReparseProbe),
                PathSafety::Safe
            )
    })
}

pub(crate) fn generated_marker(
    root: &Path,
    rule: &GeneratedProjectRule,
    cancel: Option<&Arc<crate::process::FlagCancelObserver>>,
) -> std::result::Result<Option<PathBuf>, (crate::model::ScanCoverageStatus, &'static str)> {
    if rule.owner == "dotnet" {
        let entries = fs::read_dir(root).map_err(|_| {
            (
                crate::model::ScanCoverageStatus::Failed,
                "project marker directory is unreadable",
            )
        })?;
        return dotnet_marker(entries.map(|entry| entry.map(|entry| entry.path())), cancel);
    }
    Ok(simple_generated_marker(root, rule))
}

fn dotnet_marker(
    entries: impl Iterator<Item = std::io::Result<PathBuf>>,
    cancel: Option<&Arc<crate::process::FlagCancelObserver>>,
) -> std::result::Result<Option<PathBuf>, (crate::model::ScanCoverageStatus, &'static str)> {
    use crate::{model::ScanCoverageStatus, process::CancelObserver};
    let deadline = Instant::now() + Duration::from_secs(2);
    for (index, entry) in entries.enumerate() {
        if cancel.is_some_and(|flag| flag.is_cancel_requested()) {
            return Err((
                ScanCoverageStatus::Canceled,
                "project marker discovery canceled",
            ));
        }
        if index >= 1000 || Instant::now() >= deadline {
            return Err((
                ScanCoverageStatus::BudgetLimited,
                ".NET project marker discovery budget exhausted",
            ));
        }
        let path = entry.map_err(|_| {
            (
                ScanCoverageStatus::Failed,
                "project marker entry is unreadable",
            )
        })?;
        if path
            .extension()
            .is_some_and(|value| matches!(value.to_str(), Some("csproj" | "fsproj" | "vbproj")))
            && real_file(&path)
        {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

fn simple_generated_marker(root: &Path, rule: &GeneratedProjectRule) -> Option<PathBuf> {
    match rule.owner {
        "maven" => ["pom.xml"]
            .iter()
            .map(|name| root.join(name))
            .find(|path| real_file(path)),
        "gradle" => [
            "build.gradle",
            "build.gradle.kts",
            "settings.gradle",
            "settings.gradle.kts",
        ]
        .iter()
        .map(|name| root.join(name))
        .find(|path| real_file(path)),
        "nox" => Some(root.join("noxfile.py")).filter(|path| real_file(path)),
        owner => {
            let marker = root.join("package.json");
            if !real_file(&marker) || marker.metadata().ok()?.len() > 256 * 1024 {
                return None;
            }
            let json: serde_json::Value = serde_json::from_slice(&fs::read(&marker).ok()?).ok()?;
            let owners: &[&str] = match owner {
                "node_build" => &["vite", "webpack", "rollup", "react-scripts"],
                "node_test" => &["vitest", "jest", "nyc", "c8"],
                _ => std::slice::from_ref(&owner),
            };
            let has_owner = ["dependencies", "devDependencies"].iter().any(|section| {
                json.get(section)
                    .and_then(|value| value.as_object())
                    .is_some_and(|dependencies| {
                        owners.iter().any(|owner| dependencies.contains_key(*owner))
                    })
            });
            has_owner.then_some(marker)
        }
    }
}

/// Read-only proof reused by discovery, intent reconstruction and live safety.
pub(crate) fn validate_generated_artifact(root: &Path, rule: &GeneratedProjectRule) -> Result<()> {
    validate_generated_artifact_with_cancel(root, rule, None)
}

pub(crate) fn validate_generated_artifact_with_cancel(
    root: &Path,
    rule: &GeneratedProjectRule,
    cancel: Option<&Arc<crate::process::FlagCancelObserver>>,
) -> Result<()> {
    let names: &[&str] = match rule.owner {
        "maven" | "gradle" => &["java.exe", "javaw.exe"],
        "dotnet" => &["dotnet.exe", "MSBuild.exe", "VBCSCompiler.exe"],
        "nox" => &["python.exe", "python3.exe", "py.exe"],
        _ => &["node.exe", "bun.exe"],
    };
    match super::windows_caches::owner_state(names) {
        super::windows_caches::OwnerState::Idle => {}
        super::windows_caches::OwnerState::Busy => {
            bail!("a possible build owner is running; close it and rescan")
        }
        super::windows_caches::OwnerState::Unknown => {
            bail!("build-owner activity is unknown; retained for inspection")
        }
    }
    validate_generated_contents_with_cancel(root, rule, cancel)
}

#[cfg(test)]
fn validate_generated_contents(root: &Path, rule: &GeneratedProjectRule) -> Result<()> {
    validate_generated_contents_with_cancel(root, rule, None)
}

fn validate_generated_contents_with_cancel(
    root: &Path,
    rule: &GeneratedProjectRule,
    cancel: Option<&Arc<crate::process::FlagCancelObserver>>,
) -> Result<()> {
    use crate::process::CancelObserver;
    generated_marker(root, rule, cancel)
        .map_err(|(_, detail)| anyhow::anyhow!(detail))?
        .context("required tool/project marker is missing; rescan required")?;
    let path = root.join(rule.relative);
    if !rule.evidence_file.is_empty() {
        let evidence = fs::symlink_metadata(path.join(rule.evidence_file))
            .context("generated-content evidence is missing; retained for inspection")?;
        let directory_evidence = matches!(rule.id, "maven.target" | "gradle.build");
        if directory_evidence && !evidence.is_dir() || !directory_evidence && !evidence.is_file() {
            bail!("generated-content evidence has an unexpected kind")
        }
    }
    let mut pending = vec![path.clone()];
    let mut count = 0;
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut specialized_evidence = !rule.evidence_file.is_empty();
    while let Some(current) = pending.pop() {
        count += 1;
        if cancel.is_some_and(|flag| flag.is_cancel_requested()) {
            bail!("generated-content verification canceled")
        }
        if count > 50_000 || Instant::now() >= deadline {
            bail!("generated-content verification budget exhausted")
        }
        let metadata = fs::symlink_metadata(&current)?;
        if !matches!(
            inspect_path_no_follow(&current, &metadata, &SystemPathReparseProbe),
            PathSafety::Safe
        ) {
            bail!("generated output contains a link or unverifiable path")
        }
        let original_name = current
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        let name = if cfg!(windows) {
            original_name.to_ascii_lowercase()
        } else {
            original_name.to_string()
        };
        let name = name.as_str();
        if matches!(
            name,
            ".git"
                | ".hg"
                | ".svn"
                | "id_rsa"
                | "id_ed25519"
                | "credentials.toml"
                | ".npmrc"
                | ".pypirc"
                | ".netrc"
        ) || name.starts_with(".env")
            || [".pem", ".key", ".pfx", ".p12"]
                .iter()
                .any(|suffix| name.ends_with(suffix))
        {
            bail!("output contains repository or secret material; retained for inspection")
        }
        if rule.owner == "dotnet" && name.ends_with(".deps.json")
            || rule.owner == "nox" && name == "pyvenv.cfg"
            || rule.owner == "@angular/cli" && name.ends_with(".db")
        {
            specialized_evidence = true;
        }
        if metadata.is_dir() {
            for entry in fs::read_dir(&current)? {
                if pending.len() + count >= 50_000 || Instant::now() >= deadline {
                    bail!("generated-content verification budget exhausted")
                }
                if cancel.is_some_and(|flag| flag.is_cancel_requested()) {
                    bail!("generated-content verification canceled")
                }
                pending.push(entry?.path());
            }
        }
    }
    if !specialized_evidence {
        bail!("tool-generated contents cannot be established; retained for inspection")
    }
    let ignored = git_output(
        root,
        &["check-ignore", "--quiet", "--", rule.relative],
        cancel,
    )?;
    if ignored.0 != ProcessStatus::Success {
        bail!("output is not proven ignored by its repository; retained for inspection")
    }
    let tracked = git_output(root, &["ls-files", "--cached", "--", rule.relative], cancel)?;
    if tracked.0 != ProcessStatus::Success || !tracked.1.trim().is_empty() {
        bail!("output contains tracked files or repository ownership is unavailable")
    }
    Ok(())
}

pub(crate) fn validate_python_scope(root: &Path, path: &Path) -> Result<()> {
    let markers = super::PYTHON_PROJECT_MARKERS;
    if !markers.iter().any(|marker| real_file(&root.join(marker))) {
        bail!("Python project marker is missing; rescan required")
    }
    let mut parent = path.parent();
    while let Some(directory) = parent {
        if crate::filesystem::paths_equal(directory, root) {
            return Ok(());
        }
        if markers
            .iter()
            .any(|marker| real_file(&directory.join(marker)))
            || ["package.json", "Cargo.toml", "pom.xml"]
                .iter()
                .any(|marker| real_file(&directory.join(marker)))
            || directory.join(".git").exists()
        {
            bail!("Python cache crosses a nearer project or repository boundary; rescan required")
        }
        parent = directory.parent();
    }
    bail!("Python cache is outside its marker scope")
}

fn git_output(
    root: &Path,
    args: &[&str],
    cancel: Option<&Arc<crate::process::FlagCancelObserver>>,
) -> Result<(ProcessStatus, String)> {
    let noop = NoopCancelObserver;
    let cancel: &dyn crate::process::CancelObserver = cancel
        .map(|flag| flag.as_ref() as &dyn crate::process::CancelObserver)
        .unwrap_or(&noop);
    let result = ProcessRunner::default().run(&ProcessRequest {
        program: OsString::from("git"),
        args: args.iter().map(OsString::from).collect(),
        cwd: CwdPolicy::Explicit {
            path: root.to_path_buf(),
            reason: "read-only generated artifact ownership".to_string(),
        },
        timeout: Some(Duration::from_secs(3)),
        job_deadline: None,
        cancel,
    });
    if result.output.stdout_truncated || result.output.stderr_truncated {
        bail!("repository proof output exceeded its budget")
    }
    Ok((result.status, String::from_utf8(result.output.stdout)?))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn dotnet_marker_budget_and_read_errors_are_not_absence() {
        use crate::model::ScanCoverageStatus;
        let fixture = tempfile::TempDir::new().unwrap();
        let marker = fixture.path().join("late.csproj");
        fs::write(&marker, "<Project/>").unwrap();
        let entries = (0..1000)
            .map(|index| Ok(fixture.path().join(format!("{index}.txt"))))
            .chain(std::iter::once(Ok(marker.clone())));
        assert_eq!(
            dotnet_marker(entries, None).unwrap_err().0,
            ScanCoverageStatus::BudgetLimited
        );
        assert_eq!(
            dotnet_marker(std::iter::once(Ok(marker.clone())), None).unwrap(),
            Some(marker)
        );
        assert_eq!(
            dotnet_marker(
                std::iter::once(Err(std::io::Error::from(
                    std::io::ErrorKind::PermissionDenied
                ))),
                None
            )
            .unwrap_err()
            .0,
            ScanCoverageStatus::Failed
        );
        let cancel = Arc::new(crate::process::FlagCancelObserver::default());
        cancel.request_cancel();
        assert_eq!(
            dotnet_marker(
                std::iter::once(Ok(fixture.path().to_path_buf())),
                Some(&cancel)
            )
            .unwrap_err()
            .0,
            ScanCoverageStatus::Canceled
        );
    }

    fn git(root: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("fixture Git");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    pub(crate) fn fixture(rule: &GeneratedProjectRule) -> (tempfile::TempDir, PathBuf, PathBuf) {
        let fixture = tempfile::TempDir::new().expect("fixture");
        let root = fixture.path();
        let marker = root.join(match rule.owner {
            "maven" => "pom.xml",
            "gradle" => "build.gradle",
            "dotnet" => "fixture.csproj",
            "nox" => "noxfile.py",
            _ => "package.json",
        });
        let package = serde_json::json!({"devDependencies": {"nuxt":"1", "@sveltejs/kit":"1", "@angular/cli":"1", "astro":"1", "vite":"1", "vitest":"1"}});
        fs::write(
            &marker,
            if marker.file_name().unwrap() == "package.json" {
                package.to_string()
            } else {
                "fixture".to_string()
            },
        )
        .expect("marker");
        let path = root.join(rule.relative);
        fs::create_dir_all(&path).expect("output");
        let evidence = path.join(if rule.evidence_file.is_empty() {
            match rule.owner {
                "dotnet" => "fixture.deps.json",
                "nox" => "env/pyvenv.cfg",
                _ => "compiler.db",
            }
        } else {
            rule.evidence_file
        });
        fs::create_dir_all(evidence.parent().expect("parent")).expect("evidence directory");
        if matches!(rule.id, "maven.target" | "gradle.build") {
            fs::create_dir_all(&evidence).expect("evidence directory");
            fs::write(evidence.join("generated.bin"), "generated").expect("evidence");
        } else {
            fs::write(&evidence, "generated").expect("evidence");
        }
        fs::write(root.join(".gitignore"), format!("/{}/\n", rule.relative)).expect("ignore");
        git(root, &["init", "--quiet"]);
        (fixture, marker, evidence)
    }

    #[test]
    fn every_generated_rule_requires_marker_untracked_output_and_retains_secrets() {
        for rule in crate::rules::GENERATED_PROJECT_RULES {
            let (fixture, marker, evidence) = fixture(rule);
            let root = fixture.path();
            let path = root.join(rule.relative);
            validate_generated_contents(root, rule)
                .unwrap_or_else(|error| panic!("{}: {error}", rule.id));
            let secret = path.join(".env.local");
            fs::write(&secret, "retained secret").expect("secret sentinel");
            assert!(
                validate_generated_contents(root, rule).is_err(),
                "{} secret",
                rule.id
            );
            assert_eq!(
                fs::read_to_string(&secret).expect("retained"),
                "retained secret"
            );
            fs::remove_file(&secret).expect("remove fixture sentinel only");
            git(
                root,
                &["add", "--force", "--", evidence.to_str().expect("path")],
            );
            assert!(
                validate_generated_contents(root, rule).is_err(),
                "{} tracked output",
                rule.id
            );
            assert!(evidence.exists());
            fs::remove_file(marker).expect("remove fixture marker only");
            assert!(generated_marker(root, rule, None).unwrap().is_none());
        }
    }

    #[test]
    #[cfg(windows)]
    fn generated_outputs_retain_case_insensitive_repository_and_secret_names() {
        let rule = &crate::rules::GENERATED_PROJECT_RULES[0];
        for name in [
            ".GIT",
            ".ENV.local",
            "CREDENTIALS.TOML",
            "secret.PEM",
            "private.PFX",
            ".NPMRC",
        ] {
            let (fixture, _, _) = fixture(rule);
            let sentinel = fixture.path().join(rule.relative).join(name);
            fs::write(&sentinel, "retained").expect("sentinel");
            assert!(
                validate_generated_contents(fixture.path(), rule).is_err(),
                "{name}"
            );
            assert_eq!(
                fs::read_to_string(sentinel).expect("retained sentinel"),
                "retained"
            );
        }
    }

    #[test]
    fn python_nested_scope_refuses_nearer_project_markers() {
        let fixture = tempfile::TempDir::new().expect("fixture");
        let root = fixture.path();
        fs::write(root.join("pyproject.toml"), "[project]").expect("marker");
        let path = root.join("nested/src/.pytest_cache");
        fs::create_dir_all(&path).expect("cache");
        assert!(validate_python_scope(root, &path).is_ok());
        fs::write(root.join("nested/pyproject.toml"), "[project]").expect("nearer marker");
        assert!(validate_python_scope(root, &path).is_err());
        assert!(validate_python_scope(&root.join("nested"), &path).is_ok());
    }
}
