// Release builds have no console window. Debug builds keep the console so
// `just gpui-dev` shows panics and stack-overflow messages in the terminal
// instead of a silent exit.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg_attr(not(test), allow(dead_code))]
mod analyze;
#[cfg_attr(not(test), allow(dead_code))]
mod analyze_window;
mod buffer;
#[cfg_attr(not(test), allow(dead_code))]
mod clean;
#[cfg_attr(not(test), allow(dead_code))]
mod clean_ai;
#[cfg_attr(not(test), allow(dead_code))]
mod clean_map;
mod clean_workbench;
#[cfg_attr(not(test), allow(dead_code))]
mod copy;
mod icon;
mod input;
#[cfg_attr(not(test), allow(dead_code))]
mod optimize_board;
#[cfg_attr(not(test), allow(dead_code))]
mod preferences;
#[cfg_attr(not(test), allow(dead_code))]
mod process_spawn;
#[cfg_attr(not(test), allow(dead_code))]
mod screen;
#[cfg_attr(not(test), allow(dead_code))]
mod shell;
#[cfg_attr(not(test), allow(dead_code))]
mod software_review;
#[cfg_attr(not(test), allow(dead_code))]
mod status_panel;
mod tray;
mod ui;
mod view;
#[cfg_attr(not(test), allow(dead_code))]
mod workbench;
mod workbench_review;

use crate::preferences::PreferenceFiles;
use crate::shell::Session;

fn main() {
    install_crash_log();
    // Read happens before the GPUI render loop. Launch does not migrate or write.
    // `cargo test` does not call this function, so the test process does not open
    // the user preference files.
    let mut session = match PreferenceFiles::from_env() {
        Ok(files) => Session::launch(files),
        Err(_) => Session::unbacked(),
    };
    session.status_set_tray_visible(true);
    view::run(session);
}

/// A release build has no console, so a panic would otherwise vanish. Append
/// the message, location and backtrace to `devsweep-gpui-crash.log` in the temp
/// directory, then run the default hook. This writes nothing on a clean run and
/// never touches preference or history files.
fn install_crash_log() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let path = std::env::temp_dir().join("devsweep-gpui-crash.log");
        let backtrace = std::backtrace::Backtrace::force_capture();
        let thread = std::thread::current();
        let entry = format!(
            "---\nthread '{}' {info}\n{backtrace}\n",
            thread.name().unwrap_or("<unnamed>")
        );
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            use std::io::Write;
            let _ = file.write_all(entry.as_bytes());
        }
        default_hook(info);
    }));
}
