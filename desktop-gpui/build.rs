//! Link settings for the native window binary.
//!
//! GPUI lays out and paints every frame on the main thread, and element
//! builders move large `Div` values by value. Unoptimized builds therefore need
//! far more main-thread stack than the Windows default of 1 MiB: measured on a
//! Linux debug build, the Clean landing needs more than 448 KiB and the Analyze
//! detail page more than 512 KiB, and MSVC debug frames are larger still. A
//! debug `just gpui-dev` overflowed on Windows, which a `windows` subsystem
//! binary reports as a silent freeze and exit. Reserve 8 MiB, as Zed does.
//! Only the main thread's reserve changes; committed memory grows on demand.

const MAIN_STACK_BYTES: u64 = 8 * 1024 * 1024;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if os != "windows" {
        // Linux and macOS main threads take the stack size from the process
        // limit (8 MiB by default), so no link flag is needed there.
        return;
    }
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let arg = if env == "msvc" {
        format!("/STACK:{MAIN_STACK_BYTES}")
    } else {
        format!("-Wl,--stack,{MAIN_STACK_BYTES}")
    };
    // Bins only: test harness threads size their own stacks.
    println!("cargo:rustc-link-arg-bins={arg}");
}
