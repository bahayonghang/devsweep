//! Proves the Tauri adapter reaches `devsweep-core` and nothing else.
//!
//! The desktop shell and the CLI must share one typed core service path. The
//! adapter therefore may not import the CLI crate, launch a `devsweep` child
//! process, or compose a shell command string. The checks read the production
//! part of each adapter file, which ends at its `mod tests` block: the
//! process-tree fixture inside that block spawns the test binary itself and is
//! not a production authority path.

/// Every Rust file of the Tauri adapter.
const ADAPTER_SOURCES: &[(&str, &str)] = &[
    ("lib.rs", include_str!("lib.rs")),
    ("analyze.rs", include_str!("analyze.rs")),
    ("clean.rs", include_str!("clean.rs")),
    ("commands.rs", include_str!("commands.rs")),
    (
        "desktop_preferences.rs",
        include_str!("desktop_preferences.rs"),
    ),
    ("error.rs", include_str!("error.rs")),
    ("hud.rs", include_str!("hud.rs")),
    ("optimize.rs", include_str!("optimize.rs")),
    ("scan.rs", include_str!("scan.rs")),
    ("software.rs", include_str!("software.rs")),
    ("status.rs", include_str!("status.rs")),
    ("support.rs", include_str!("support.rs")),
    ("tray.rs", include_str!("tray.rs")),
];

/// Returns the part of an adapter file before its test module.
fn production_source(source: &str) -> &str {
    source
        .split_once("mod tests {")
        .map_or(source, |(production, _)| production)
}

#[test]
fn adapter_sources_cover_every_module_of_the_crate() {
    let declared = production_source(include_str!("lib.rs"))
        .lines()
        .filter_map(|line| line.trim().strip_prefix("mod "))
        .filter_map(|name| name.strip_suffix(';'))
        .filter(|name| !["wire_parity", "service_boundary"].contains(name))
        .count();

    assert_eq!(
        ADAPTER_SOURCES.len(),
        declared + 1,
        "ADAPTER_SOURCES must list lib.rs and every module it declares"
    );
}

#[test]
fn production_adapter_sources_never_reach_the_cli_or_a_shell() {
    for (name, source) in ADAPTER_SOURCES {
        let production = production_source(source);
        for forbidden in [
            "devsweep_cli",
            "devsweep-cli",
            "devsweep::run",
            "Command::new",
            "std::process::Command",
            "cmd.exe",
            "powershell",
            "/c ",
        ] {
            assert!(
                !production.contains(forbidden),
                "{name}: the production adapter must not contain {forbidden}"
            );
        }
    }
}

#[test]
fn the_desktop_manifest_depends_on_core_only() {
    let manifest = include_str!("../Cargo.toml");

    assert!(
        manifest.contains("devsweep-core"),
        "the desktop shell must depend on devsweep-core"
    );
    assert!(
        !manifest.contains("devsweep-cli"),
        "the desktop shell must not depend on the CLI crate"
    );
}
