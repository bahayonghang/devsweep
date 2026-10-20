#![windows_subsystem = "windows"]

#[cfg_attr(not(test), allow(dead_code))]
mod analyze;
mod buffer;
#[cfg_attr(not(test), allow(dead_code))]
mod clean;
#[cfg_attr(not(test), allow(dead_code))]
mod copy;
mod icon;
mod input;
#[cfg_attr(not(test), allow(dead_code))]
mod preferences;
#[cfg_attr(not(test), allow(dead_code))]
mod screen;
#[cfg_attr(not(test), allow(dead_code))]
mod shell;
#[cfg_attr(not(test), allow(dead_code))]
mod status_panel;
mod tray;
mod view;
mod workbench;

use crate::preferences::PreferenceFiles;
use crate::shell::Session;

fn main() {
    // Read happens before the GPUI render loop. Launch does not migrate or write.
    // `cargo test` does not call this function, so the test process does not open
    // the user preference files.
    let session = match PreferenceFiles::from_env() {
        Ok(files) => Session::launch(files),
        Err(_) => Session::unbacked(),
    };
    view::run(session);
}
