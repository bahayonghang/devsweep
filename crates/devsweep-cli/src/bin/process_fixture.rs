//! Helper binary for ProcessRunner dynamic fixtures.
//! Invoked only from tests: program + argv, never through a shell.

use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
    process::Command,
    thread,
    time::Duration,
};

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>().into_iter();
    let mode = args.next().unwrap_or_default();
    if provider_fixture(&mode, args.clone().collect()) {
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
