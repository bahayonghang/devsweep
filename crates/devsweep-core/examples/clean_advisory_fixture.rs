//! Regenerate the observational desktop fixture from its archived input report.
//! Run: cargo run -p devsweep-core --example clean_advisory_fixture
use devsweep_core::{
    clean_advisories::{CleanAdvisoryPreferencesV1, project_capacity_advisories},
    model::{ScanCompleteness, ScanHealth, ScanReport, Scope},
};
use std::{fs, path::Path};

fn main() -> anyhow::Result<()> {
    if !cfg!(windows) {
        anyhow::bail!("these archived native Windows reports must be regenerated on Windows");
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../desktop/src/api/fixtures/clean");
    let report: ScanReport = serde_json::from_slice(&fs::read(root.join("advisory-input.json"))?)?;
    let scan_report: ScanReport =
        serde_json::from_slice(&fs::read(root.join("../scan-report.json"))?)?;
    let mut replays = serde_json::Map::new();
    for (name, projects, global) in [
        ("empty", false, false),
        ("global", false, true),
        ("projects", true, false),
        ("combined", true, true),
    ] {
        let mut plan = scan_report.plan.clone();
        plan.targets.retain(|target| match target.scope {
            Scope::Project { .. } => projects,
            Scope::Global => global,
        });
        for target in &mut plan.targets {
            if matches!(target.scope, Scope::Project { .. }) {
                target.selected_by_default = false;
            }
        }
        let health = ScanHealth::new(
            ScanCompleteness::Complete,
            if global {
                scan_report.health.diagnostics.clone()
            } else {
                Vec::new()
            },
        );
        let report = ScanReport::new(plan, health);
        let advisories = project_capacity_advisories(&report, 5)?;
        replays.insert(
            name.into(),
            serde_json::json!({ "report": report, "advisories": advisories }),
        );
    }
    for (name, value) in [
        (
            "advisories.json",
            serde_json::to_value(project_capacity_advisories(&report, 5)?)?,
        ),
        (
            "scan-advisories.json",
            serde_json::to_value(project_capacity_advisories(&scan_report, 5)?)?,
        ),
        (
            "advisory-preferences.json",
            serde_json::to_value(CleanAdvisoryPreferencesV1::default())?,
        ),
        ("scan-advisory-replays.json", replays.into()),
    ] {
        fs::write(
            root.join(name),
            format!("{}\n", serde_json::to_string_pretty(&value)?),
        )?;
    }
    Ok(())
}
