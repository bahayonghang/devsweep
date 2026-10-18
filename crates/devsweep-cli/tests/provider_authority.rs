//! Real CLI/process/provider authority roundtrips against disposable fixtures.
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::TempDir;

struct Fixture {
    _dir: TempDir,
    root: PathBuf,
    bin: PathBuf,
    home: PathBuf,
    environment: Vec<(String, std::ffi::OsString)>,
}
impl Fixture {
    fn new(name: &str, version: &str, script: Option<&str>) -> Self {
        let dir = TempDir::new().expect("fixture");
        // macOS temp lives under the /var symlink. Walk the resolved directory
        // so ancestor checks see /private/var, which is not a symlink.
        #[cfg(unix)]
        let root = dir.path().canonicalize().expect("canonical fixture");
        #[cfg(not(unix))]
        let root = dir.path().to_path_buf();
        let bin = root.join("tools 空格");
        let home = root.join("home");
        let cache = root.join("cache warning 空格");
        let build_cache = root.join("build cache 空格");
        for path in [&bin, &home, &cache, &build_cache, &cache.join("cache")] {
            fs::create_dir_all(path).expect("directory");
        }
        fs::write(cache.join("sentinel"), "must remain").expect("sentinel");
        fs::write(
            bin.join("provider.json"),
            serde_json::to_vec(&json!({"version":version,"cache":cache,"build_cache":build_cache}))
                .expect("JSON"),
        )
        .expect("fixture config");
        if let Some(script) = script {
            #[cfg(windows)]
            fs::write(
                bin.join(format!("{name}.cmd")),
                "THIS SHIM MUST NEVER BE EXECUTED",
            )
            .expect("shim");
            let entry = bin.join(script);
            fs::create_dir_all(entry.parent().expect("parent")).expect("script directory");
            fs::write(&entry, "#!/usr/bin/env node\n// fixed entrypoint\n").expect("script");
            #[cfg(unix)]
            std::os::unix::fs::symlink(&entry, bin.join(name))
                .expect("standard package bin symlink");
            fs::copy(
                env!("CARGO_BIN_EXE_process_fixture"),
                bin.join(format!("node{}", std::env::consts::EXE_SUFFIX)),
            )
            .expect("native fixture");
        } else {
            fs::copy(
                env!("CARGO_BIN_EXE_process_fixture"),
                bin.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)),
            )
            .expect("native fixture");
        }
        Self {
            _dir: dir,
            root,
            bin,
            home,
            environment: vec![],
        }
    }
    fn run(&self, args: &[&str]) -> Output {
        self.run_with_env(args, &[])
    }
    fn run_with_env(&self, args: &[&str], environment: &[(&str, &str)]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_devsweep"))
            .args(args)
            .env("PATH", &self.bin)
            .env("USERPROFILE", &self.home)
            .env("HOME", &self.home)
            .env("APPDATA", self.home.join("AppData/Roaming"))
            .env("LOCALAPPDATA", self.home.join("AppData/Local"))
            .env("PROGRAMDATA", self.home.join("ProgramData"))
            .env_remove("CARGO_HOME")
            .env_remove("NPM_CONFIG_USERCONFIG")
            .env_remove("NPM_CONFIG_GLOBALCONFIG")
            .env_remove("UV_CONFIG_FILE")
            .env_remove("PIP_CONFIG_FILE")
            .env_remove("GOENV")
            .env_remove("YARN_CONFIG")
            .env_remove("YARN_YARN_PATH")
            .env_remove("yarn_yarn_path")
            .env_remove("yarn_rc_filename")
            .env("XDG_CONFIG_HOME", self.home.join(".config"))
            .env("YARN_RC_FILENAME", "fixture-yarn.yml")
            .env("TEMP", &self.root)
            .env("TMP", &self.root)
            .env("TMPDIR", &self.root)
            .envs(
                self.environment
                    .iter()
                    .map(|(key, value)| (key.as_str(), value)),
            )
            .envs(environment.iter().copied())
            .current_dir(&self.home)
            .output()
            .expect("CLI process")
    }
    fn json(&self, args: &[&str]) -> Value {
        let result = self.run(args);
        assert!(
            result.status.success(),
            "{args:?}: {} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        serde_json::from_slice(&result.stdout).expect("one JSON document")
    }
    fn plan(&self, rule: &str) -> PathBuf {
        let observation = self.root.join("observation.json");
        let output = self.run(&["clean", "scan", "--scope", "global", "--format", "json"]);
        assert!(
            output.status.success(),
            "scan: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let document: Value = serde_json::from_slice(&output.stdout).expect("scan");
        fs::write(&observation, output.stdout).expect("observation");
        let target = document["data"]["plan"]["targets"]
            .as_array()
            .expect("targets")
            .iter()
            .find(|target| target["rule_id"] == rule)
            .expect("provider target");
        let plan = self.root.join("plan.json");
        let result = self.run(&[
            "clean",
            "plan",
            "--observation",
            observation.to_str().unwrap(),
            "--select",
            target["id"].as_str().unwrap(),
            "--output",
            plan.to_str().unwrap(),
        ]);
        assert!(
            result.status.success(),
            "plan: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        plan
    }
    fn preview(&self, plan: &Path) -> String {
        let value = self.json(&[
            "clean",
            "preview",
            "--plan",
            plan.to_str().unwrap(),
            "--format",
            "json",
        ]);
        value["data"]["digest"]
            .as_str()
            .expect("preview digest")
            .to_string()
    }
    fn execute(&self, plan: &Path, digest: &str) -> Output {
        self.run(&[
            "clean",
            "execute",
            "--plan",
            plan.to_str().unwrap(),
            "--preview-digest",
            digest,
            "--confirm",
            "--format",
            "json",
        ])
    }
}

#[test]
fn standard_native_providers_roundtrip_and_preserve_cache_sentinels() {
    for (name, version, script, rule) in [
        (
            "npm",
            "11.0.0",
            Some("node_modules/npm/bin/npm-cli.js"),
            "npm.cache.clean",
        ),
        (
            "pnpm",
            "10.0.0",
            Some("node_modules/pnpm/bin/pnpm.cjs"),
            "pnpm.store.prune",
        ),
        (
            "yarn",
            "1.22.22",
            Some("node_modules/yarn/bin/yarn.js"),
            "yarn.cache.clean.classic",
        ),
        (
            "yarn",
            "4.0.0",
            Some("node_modules/yarn/bin/yarn.js"),
            "yarn.cache.clean.modern",
        ),
        (
            "py",
            "pip 25.0 from fixture (python 3.13)",
            None,
            "pip.cache.purge",
        ),
        (
            "go",
            "go version go1.26.0 windows/amd64",
            None,
            "go.mod_cache.clean",
        ),
        (
            "go",
            "go version go1.26.0 windows/amd64",
            None,
            "go.build_cache.clean",
        ),
        ("uv", "uv 0.9.0", None, "uv.cache.clean"),
        ("bun", "1.3.0", None, "bun.cache.clean"),
    ] {
        let fixture = Fixture::new(name, version, script);
        let plan = fixture.plan(rule);
        let digest = fixture.preview(&plan);
        let result = fixture.execute(&plan, &digest);
        assert!(
            result.status.success(),
            "{name}: {} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            fixture.bin.join("invoked.json").is_file(),
            "owner command was dispatched"
        );
        assert!(fixture.root.join("cache warning 空格/sentinel").is_file());
    }
}

#[test]
fn production_preview_rejects_replaced_shim_script_node_and_configuration() {
    for changed in [
        #[cfg(windows)]
        "npm.cmd",
        "node_modules/npm/bin/npm-cli.js",
        if cfg!(windows) { "node.exe" } else { "node" },
        "config",
    ] {
        let fixture = Fixture::new("npm", "11.0.0", Some("node_modules/npm/bin/npm-cli.js"));
        let config = fixture.home.join(".npmrc");
        fs::write(&config, "cache=before\n//registry/:_authToken=not-hashed\n").expect("config");
        let plan = fixture.plan("npm.cache.clean");
        let digest = fixture.preview(&plan);
        if changed == "config" {
            fs::write(&config, "cache=after\n//registry/:_authToken=not-hashed\n")
                .expect("config drift");
        } else {
            use std::io::Write;
            fs::OpenOptions::new()
                .append(true)
                .open(fixture.bin.join(changed))
                .expect("same path")
                .write_all(b"changed")
                .expect("replace identity");
        }
        let output = fixture.execute(&plan, &digest);
        assert!(
            !output.status.success(),
            "stale {changed} authority accepted"
        );
        assert!(
            !fixture.bin.join("invoked.json").exists(),
            "no simulated cleanup occurred"
        );
        assert!(fixture.root.join("cache warning 空格/sentinel").is_file());
    }
}

#[test]
fn yarn_custom_rc_content_is_bound_and_delegation_is_unavailable_during_scan() {
    let fixture = Fixture::new("yarn", "4.0.0", Some("node_modules/yarn/bin/yarn.js"));
    let config = fixture.home.join("fixture-yarn.yml");
    fs::write(&config, "globalFolder: before\nnpmAuthToken: retained\n").unwrap();
    let plan = fixture.plan("yarn.cache.clean.modern");
    let digest = fixture.preview(&plan);
    fs::write(&config, "globalFolder: after\nnpmAuthToken: retained\n").unwrap();
    assert!(!fixture.execute(&plan, &digest).status.success());
    assert!(!fixture.bin.join("invoked.json").exists());
    for declaration in ["yarnPath: delegate.cjs", "yarn-path \"delegate.js\""] {
        fs::write(&config, declaration).unwrap();
        let output = fixture.run(&["clean", "scan", "--scope", "global", "--format", "json"]);
        let document: Value = serde_json::from_slice(&output.stdout).unwrap();
        let yarn = document["data"]["health"]["coverage"]
            .as_array()
            .unwrap()
            .iter()
            .find(|unit| unit["id"] == "yarn")
            .unwrap();
        assert_eq!(yarn["status"], "unsupported");
        assert!(yarn["detail"].as_str().unwrap().contains("delegation"));
        assert!(
            !document["data"]["plan"]["targets"]
                .as_array()
                .unwrap()
                .iter()
                .any(|target| target["rule_id"].as_str().unwrap().starts_with("yarn."))
        );
    }
    fs::write(&config, "globalFolder: after\n").unwrap();
    fs::write(
        fixture.home.join("lowercase-rc.yml"),
        "  yarnPath: delegate.cjs\n",
    )
    .unwrap();
    for environment in [
        ("YARN_YARN_PATH", "delegate.cjs"),
        ("yarn_yarn_path", "delegate.cjs"),
        ("yarn_rc_filename", "lowercase-rc.yml"),
    ] {
        let output = fixture.run_with_env(
            &["clean", "scan", "--scope", "global", "--format", "json"],
            &[environment],
        );
        let document: Value = serde_json::from_slice(&output.stdout).unwrap();
        let yarn = document["data"]["health"]["coverage"]
            .as_array()
            .unwrap()
            .iter()
            .find(|unit| unit["id"] == "yarn")
            .unwrap();
        assert_eq!(yarn["status"], "unsupported");
        assert!(
            yarn["detail"].as_str().unwrap().contains("delegation")
                || yarn["detail"].as_str().unwrap().contains("conflicting")
        );
    }
}

#[test]
fn yarn_classic_config_sources_bind_content_and_refuse_delegation_before_queries() {
    for source in [
        ".config/yarn/config",
        #[cfg(windows)] // Unix returns ENOTDIR for the preceding nested config path.
        ".config/yarn",
        ".yarn/config",
        "platform-config",
        "YARN_CONFIG",
    ] {
        let mut fixture = Fixture::new("yarn", "1.22.22", Some("node_modules/yarn/bin/yarn.js"));
        let config = match source {
            "platform-config" => {
                #[cfg(windows)]
                let path = fixture.home.join("AppData/Local/Yarn/Config");
                #[cfg(unix)]
                let path = {
                    fixture.environment.push((
                        "XDG_CONFIG_HOME".into(),
                        fixture.home.join("xdg").into_os_string(),
                    ));
                    fixture.home.join("xdg/yarn")
                };
                path
            }
            "YARN_CONFIG" => {
                let path = fixture.home.join("custom-classic.rc");
                fixture
                    .environment
                    .push(("YARN_CONFIG".into(), path.clone().into_os_string()));
                path
            }
            _ => fixture.home.join(source),
        };
        fs::create_dir_all(config.parent().unwrap()).unwrap();
        fs::write(&config, "cache-folder before\nnpmAuthToken retained\n").unwrap();
        let plan = fixture.plan("yarn.cache.clean.classic");
        let digest = fixture.preview(&plan);
        fs::write(&config, "cache-folder after\nnpmAuthToken retained\n").unwrap();
        assert!(
            !fixture.execute(&plan, &digest).status.success(),
            "{source}: same-path configuration drift must reject the old digest"
        );
        assert!(!fixture.bin.join("invoked.json").exists());
        let call_log = fixture.bin.join("provider-calls.jsonl");
        for declaration in [
            "yarn-path delegate.js",
            "yarnPath: delegate.cjs",
            "--cwd another-project",
            "--use-yarnrc another.rc",
            "--cache.cache-folder another-cache",
        ] {
            fs::write(&config, declaration).unwrap();
            let calls = fs::read(&call_log).unwrap();
            let scan = fixture.run(&["clean", "scan", "--scope", "global", "--format", "json"]);
            let document: Value = serde_json::from_slice(&scan.stdout).unwrap();
            let coverage = document["data"]["health"]["coverage"]
                .as_array()
                .unwrap()
                .iter()
                .find(|unit| unit["id"] == "yarn")
                .unwrap();
            assert_eq!(coverage["status"], "unsupported", "{source}: {declaration}");
            assert!(
                !document["data"]["plan"]["targets"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|target| target["rule_id"].as_str().unwrap().starts_with("yarn."))
            );
            let preview = fixture.run(&[
                "clean",
                "preview",
                "--plan",
                plan.to_str().unwrap(),
                "--format",
                "json",
            ]);
            assert!(!preview.status.success(), "{source}: {declaration}");
            assert_eq!(
                fs::read(&call_log).unwrap(),
                calls,
                "{source}: {declaration} must refuse discovery and preview before any provider invocation"
            );
            assert!(!fixture.bin.join("invoked.json").exists());
        }
        assert_eq!(
            fs::read_to_string(fixture.root.join("cache warning 空格/sentinel")).unwrap(),
            "must remain"
        );
    }
    let fixture = Fixture::new("yarn", "1.22.22", Some("node_modules/yarn/bin/yarn.js"));
    let output = fixture.run_with_env(
        &["clean", "scan", "--scope", "global", "--format", "json"],
        &[("YARN_CONFIG", "relative.rc")],
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    let coverage = document["data"]["health"]["coverage"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["id"] == "yarn")
        .unwrap();
    assert_eq!(coverage["status"], "unsupported");
    assert!(coverage["detail"].as_str().unwrap().contains("relative"));
    assert!(!fixture.bin.join("provider-calls.jsonl").exists());
}

#[test]
#[cfg(windows)]
fn all_provider_config_sources_bind_nonsecret_content_without_binding_credentials() {
    for (name, version, script, rule, relative, before, after) in [
        (
            "npm",
            "11.0.0",
            Some("node_modules/npm/bin/npm-cli.js"),
            "npm.cache.clean",
            ".npmrc",
            "cache=before",
            "cache=after",
        ),
        (
            "pnpm",
            "10.0.0",
            Some("node_modules/pnpm/bin/pnpm.cjs"),
            "pnpm.store.prune",
            ".npmrc",
            "store-dir=before",
            "store-dir=after",
        ),
        (
            "py",
            "pip 25.0 from fixture (python 3.13)",
            None,
            "pip.cache.purge",
            "AppData/Roaming/pip/pip.ini",
            "[global]\ncache-dir=before",
            "[global]\ncache-dir=after",
        ),
        (
            "yarn",
            "4.0.0",
            Some("node_modules/yarn/bin/yarn.js"),
            "yarn.cache.clean.modern",
            ".yarnrc.yml",
            "globalFolder: before",
            "globalFolder: after",
        ),
        (
            "go",
            "go version go1.26.0 windows/amd64",
            None,
            "go.mod_cache.clean",
            "AppData/Roaming/go/env",
            "GOMODCACHE=before",
            "GOMODCACHE=after",
        ),
        (
            "uv",
            "uv 0.9.0",
            None,
            "uv.cache.clean",
            "AppData/Roaming/uv/uv.toml",
            "cache-dir='before'",
            "cache-dir='after'",
        ),
        (
            "bun",
            "1.3.0",
            None,
            "bun.cache.clean",
            ".bunfig.toml",
            "[install.cache]\ndir='before'",
            "[install.cache]\ndir='after'",
        ),
    ] {
        let fixture = Fixture::new(name, version, script);
        let config = fixture.home.join(relative);
        fs::create_dir_all(config.parent().expect("parent")).expect("configuration directory");
        fs::write(&config, before).expect("configuration");
        let plan = fixture.plan(rule);
        let digest = fixture.preview(&plan);
        fs::write(config, after).expect("same-path config replacement");
        assert!(
            !fixture.execute(&plan, &digest).status.success(),
            "{rule} config drift was accepted"
        );
        assert!(!fixture.bin.join("invoked.json").exists());
    }
    let fixture = Fixture::new("npm", "11.0.0", Some("node_modules/npm/bin/npm-cli.js"));
    let config = fixture.home.join(".npmrc");
    fs::write(&config, "cache=same\n//registry/:_authToken=first").expect("configuration");
    let plan = fixture.plan("npm.cache.clean");
    let digest = fixture.preview(&plan);
    fs::write(config, "cache=same\n//registry/:_authToken=second").expect("credentials update");
    assert!(
        fixture.execute(&plan, &digest).status.success(),
        "unrelated credentials must not be fingerprinted"
    );
}
