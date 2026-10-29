//! Spawn a console program without allocating a console.
//!
//! This crate is `windows_subsystem = "windows"`. `Command::output` on a
//! console binary then hands the child to the default terminal. Windows
//! Terminal fails that handoff with `0x800700E8` (`ERROR_NO_DATA`, "The pipe
//! is being closed") and leaves the error window open. `CREATE_NO_WINDOW`
//! keeps the redirected stdout and stderr pipes and skips the handoff.

use std::ffi::OsStr;
use std::process::Command;

pub(crate) fn command_without_window(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW. Same flag as the core process runner.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    #[test]
    fn command_without_window_captures_stdout() {
        let output = command_without_window("cmd")
            .args(["/c", "echo", "devsweep-no-window"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("cmd starts");
        assert!(output.status.success(), "status {}", output.status);
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(
            text.contains("devsweep-no-window"),
            "captured stdout was {text:?}"
        );
    }
}
