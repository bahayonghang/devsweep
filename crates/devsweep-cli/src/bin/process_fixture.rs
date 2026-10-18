//! Helper binary for ProcessRunner dynamic fixtures.
//! Invoked only from tests: program + argv, never through a shell.

use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

fn record_optimize_double() -> bool {
    let Ok(exe) = env::current_exe() else {
        return false;
    };
    let Some(dir) = exe.parent() else {
        return false;
    };
    let marker = dir.join("optimize-double.json");
    if !marker.is_file() {
        return false;
    }
    let argv = env::args().skip(1).collect::<Vec<_>>();
    let bytes = serde_json::to_vec(&argv).expect("optimize double argv");
    fs::write(dir.join("optimize-calls.json"), bytes).expect("optimize double calls");
    true
}

fn main() {
    if record_optimize_double() {
        return;
    }
    let mut args = env::args().skip(1).collect::<Vec<_>>().into_iter();
    let mode = args.next().unwrap_or_default();
    if provider_fixture(&mode, args.clone().collect()) {
        return;
    }
    if mode == "upgrade" {
        update_double(args.collect());
        return;
    }
    match mode.as_str() {
        "hang" => loop {
            thread::sleep(Duration::from_secs(1));
        },
        "hang-tree" => {
            let pid_file = args.next().map(PathBuf::from);
            let self_exe = env::current_exe().expect("fixture executable path");
            let mut child = Command::new(&self_exe)
                .arg("hang")
                .spawn()
                .expect("spawn grandchild hang");
            if let Some(path) = pid_file {
                fs::write(&path, child.id().to_string()).expect("write grandchild pid");
            }
            // Keep the child handle alive and hang alongside it.
            loop {
                thread::sleep(Duration::from_millis(200));
                if let Ok(Some(_)) = child.try_wait() {
                    // Grandchild exited unexpectedly; keep hanging so the parent
                    // runner still exercises timeout cleanup.
                }
            }
        }
        "endless-stdout" => {
            let chunk = vec![b'x'; 4096];
            loop {
                let _ = io::stdout().write_all(&chunk);
                let _ = io::stdout().flush();
            }
        }
        "endless-stderr" => {
            let chunk = vec![b'y'; 4096];
            loop {
                let _ = io::stderr().write_all(&chunk);
                let _ = io::stderr().flush();
            }
        }
        "non-utf8" => {
            let _ = io::stdout().write_all(&[0xff, 0xfe, 0x80, b'\n']);
            let _ = io::stderr().write_all(&[0x1b, b'[', b'3', b'1', b'm', 0xff, b'\n']);
        }
        "exit" => {
            let code = args
                .next()
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or(1);
            std::process::exit(code);
        }
        "print-cwd" => {
            let cwd = env::current_dir().expect("current dir");
            println!("{}", cwd.display());
        }
        "sleep-exit" => {
            let ms = args
                .next()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(50);
            thread::sleep(Duration::from_millis(ms));
        }
        "control-stderr" => {
            // BEL + CSI color + long payload for sanitizer tests.
            let mut msg = Vec::from(&b"\x1b[31m\x07SECRET\x1b[0m"[..]);
            msg.extend(std::iter::repeat_n(b'Z', 8 * 1024));
            let _ = io::stderr().write_all(&msg);
            std::process::exit(1);
        }
        other => {
            eprintln!("unknown process_fixture mode: {other}");
            std::process::exit(2);
        }
    }
}

/// Command double for software update tests. It is not the installed winget.
fn update_double(rest: Vec<String>) {
    let exe = env::current_exe().expect("fixture path");
    let directory = exe.parent().expect("fixture directory");
    let mut args = vec!["upgrade".to_string()];
    args.extend(rest);
    let mut calls = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("update-calls.jsonl"))
        .expect("update call log");
    writeln!(
        calls,
        "{}",
        serde_json::to_string(&args).expect("arguments")
    )
    .expect("record update call");
    let config_path = directory.join("update-double.json");
    let fixture: serde_json::Value =
        serde_json::from_slice(&fs::read(&config_path).expect("update double"))
            .expect("double JSON");
    let network = fixture["network"].as_bool().unwrap_or(false);
    let upgrade = args.iter().any(|arg| arg == "--id");
    if !upgrade && network {
        println!("network_unavailable");
        std::process::exit(1);
    }
    if upgrade {
        match fixture["upgrade_result"].as_str().unwrap_or("fail") {
            "restart_required" => {
                println!("restart_required");
                std::process::exit(1);
            }
            "updated" => {
                let id = args.iter().skip_while(|arg| *arg != "--id").nth(1);
                let version = args.iter().skip_while(|arg| *arg != "--version").nth(1);
                let mut rows = fixture["rows"].as_array().cloned().unwrap_or_default();
                if let (Some(id), Some(version)) = (id, version) {
                    for row in &mut rows {
                        if row["id"].as_str() == Some(id) {
                            row["installed"] = serde_json::Value::String(version.clone());
                        }
                    }
                }
                rows.retain(|row| row["installed"] != row["available"]);
                let mut next = fixture;
                next["rows"] = serde_json::Value::Array(rows);
                fs::write(&config_path, serde_json::to_vec(&next).expect("double"))
                    .expect("rewrite double");
                println!("No available upgrade found.");
                return;
            }
            _ => {
                println!("upgrade_failed");
                std::process::exit(1);
            }
        }
    }
    let rows = fixture["rows"].as_array().cloned().unwrap_or_default();
    if rows.is_empty() {
        println!("No available upgrade found.");
        return;
    }
    let header = "Name                                           Id                         Version                      Available  Source";
    println!("{header}");
    println!("{}", "-".repeat(header.len()));
    let starts = column_starts(header);
    for row in rows {
        let fields = [
            row["name"].as_str().unwrap_or(""),
            row["id"].as_str().unwrap_or(""),
            row["installed"].as_str().unwrap_or(""),
            row["available"].as_str().unwrap_or(""),
            row["source"].as_str().unwrap_or(""),
        ];
        let mut line = vec![' '; header.chars().count()];
        for (index, field) in fields.iter().enumerate() {
            for (offset, character) in field.chars().enumerate() {
                let slot = starts[index] + offset;
                if slot < line.len() {
                    line[slot] = character;
                }
            }
        }
        println!("{}", line.into_iter().collect::<String>());
    }
}

fn column_starts(header: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut previous_space = true;
    for (offset, character) in header.chars().enumerate() {
        let space = character == ' ';
        if !space && previous_space {
            starts.push(offset);
        }
        previous_space = space;
    }
    starts
}

/// Disposable native provider simulation. The fixture never removes cache files.
fn provider_fixture(first: &str, rest: Vec<String>) -> bool {
    let exe = env::current_exe().expect("fixture path");
    let config = exe
        .parent()
        .expect("fixture directory")
        .join("provider.json");
    if !config.is_file() {
        return false;
    }
    let fixture: serde_json::Value =
        serde_json::from_slice(&fs::read(config).expect("provider fixture")).expect("fixture JSON");
    let mut args = vec![first.to_string()];
    args.extend(rest);
    if args
        .first()
        .is_some_and(|arg| arg.ends_with(".js") || arg.ends_with(".cjs"))
    {
        args.remove(0);
    }
    let mut calls = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(
            exe.parent()
                .expect("fixture directory")
                .join("provider-calls.jsonl"),
        )
        .expect("fixture call log");
    writeln!(
        calls,
        "{}",
        serde_json::to_string(&args).expect("arguments")
    )
    .expect("record provider call");
    if record_disposable_provider_launch(&args, exe.parent().expect("fixture directory")) {
        return true;
    }
    let refs: Vec<_> = args.iter().map(String::as_str).collect();
    match refs.as_slice() {
        ["--version"] | ["version"] | ["-m", "pip", "--version"] => {
            println!("{}", fixture["version"].as_str().expect("version"))
        }
        ["-c", "import sys; print(sys.executable)"] => println!("{}", exe.display()),
        ["env", "GOCACHE"] => println!("{}", fixture["build_cache"].as_str().expect("build cache")),
        ["config", "get", "cache" | "globalFolder"]
        | ["store", "path"]
        | ["cache", "dir"]
        | ["-m", "pip", "cache", "dir"]
        | ["env", "GOMODCACHE"]
        | ["pm", "cache"] => println!("{}", fixture["cache"].as_str().expect("cache")),
        ["cache", "clean"]
        | ["cache", "clean", "--force" | "--mirror"]
        | ["store", "prune"]
        | ["-m", "pip", "cache", "purge"]
        | ["clean", "-modcache" | "-cache"]
        | ["pm", "cache", "rm"]
        | ["/flushdns"] => {
            fs::write(
                exe.parent()
                    .expect("fixture directory")
                    .join("invoked.json"),
                serde_json::to_vec(&args).expect("arguments"),
            )
            .expect("record simulated command");
        }
        _ => {
            eprintln!("unsupported fixture query");
            std::process::exit(2);
        }
    }
    true
}

/// MSI `/x {code} /qn /norestart`, vendor `/S`, and Scoop `uninstall <app>`.
/// Success writes one marker under the launch root. It never deletes files.
fn record_disposable_provider_launch(args: &[String], beside_exe: &Path) -> bool {
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let matched = match refs.as_slice() {
        ["/x", product, "/qn", "/norestart"] => {
            product.len() == 38 && product.starts_with('{') && product.ends_with('}')
        }
        ["/S"] => true,
        ["uninstall", app] => {
            !app.is_empty()
                && app
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        }
        _ => false,
    };
    if !matched {
        return false;
    }
    let directory = env::current_dir()
        .ok()
        .filter(|cwd| beside_exe.starts_with(cwd) || cwd.starts_with(beside_exe))
        .unwrap_or_else(|| beside_exe.to_path_buf());
    fs::write(
        directory.join("provider-launch.marker"),
        serde_json::to_vec(args).expect("marker argv"),
    )
    .expect("write disposable provider marker");
    true
}
