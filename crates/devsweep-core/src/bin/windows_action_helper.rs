//! Short-lived Windows fixture helper. Tests inject this binary.
//! It reads one sealed frame on stdin and does not accept a command string.

fn main() {
    let code = devsweep_core::execution::run_windows_action_helper();
    std::process::exit(code);
}
