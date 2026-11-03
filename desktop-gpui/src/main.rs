#![windows_subsystem = "windows"]

#[cfg_attr(not(test), allow(dead_code))]
mod analyze;
mod analyze_window;
mod buffer;
#[cfg_attr(not(test), allow(dead_code))]
mod clean;
mod clean_ai;
mod clean_map;
mod clean_workbench;
#[cfg_attr(not(test), allow(dead_code))]
mod copy;
mod icon;
mod input;
mod optimize_board;
#[cfg_attr(not(test), allow(dead_code))]
mod preferences;
#[cfg_attr(not(test), allow(dead_code))]
mod process_spawn;
#[cfg_attr(not(test), allow(dead_code))]
mod screen;
#[cfg_attr(not(test), allow(dead_code))]
mod shell;
mod software_review;
#[cfg_attr(not(test), allow(dead_code))]
mod status_panel;
mod tray;
mod view;
mod workbench;
mod workbench_review;

use crate::preferences::PreferenceFiles;
use crate::shell::Session;

fn main() {
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
