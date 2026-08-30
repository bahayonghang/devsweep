//! Frozen-grammar Software inventory and exact-selection-plan handlers.

use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use devsweep_core::software::{
    SoftwareInventorySource, SoftwareInventoryV1, SoftwarePlanError, SoftwareSourceState,
    build_selection_plan, inventory_software,
};

use super::super::super::{
    cli::{Cli, SoftwareInventoryCommand, SoftwarePlanCommand, SoftwareSource},
    output::{ApplicationError, ExitClass, OutputSink, ResultFormat, write_json},
    presentation,
};
use crate::i18n::Locale;

#[cfg(test)]
pub(super) struct Route;

pub(super) fn run_inventory(
    command: &SoftwareInventoryCommand,
    locale: Locale,
    cli: &Cli,
) -> Result<(), ApplicationError> {
    let inventory = inventory_software(source(command.source), None).map_err(|error| {
        ApplicationError::failed("software_inventory_failed", format!("{error:#}"))
    })?;
    emit_inventory(cli, locale, &inventory)
}

pub(super) fn run_plan(command: &SoftwarePlanCommand) -> Result<(), ApplicationError> {
    let inventory = load_inventory(&command.inventory)?;
    let plan = build_selection_plan(&inventory, &command.select, unix_ms(SystemTime::now()))
        .map_err(map_plan_error)?;
    let mut sink = OutputSink::open(Some(&command.output))?;
    let mut bytes = serde_json::to_vec_pretty(&plan)
        .map_err(|error| ApplicationError::failed("output_serialize_failed", error.to_string()))?;
    bytes.push(b'\n');
    sink.write_all(&bytes)
        .map_err(|error| ApplicationError::failed("output_write_failed", error.to_string()))
}

fn source(source: SoftwareSource) -> SoftwareInventorySource {
    match source {
        SoftwareSource::All => SoftwareInventorySource::All,
        SoftwareSource::Arp => SoftwareInventorySource::Arp,
        SoftwareSource::Msi => SoftwareInventorySource::Msi,
        SoftwareSource::Msix => SoftwareInventorySource::Msix,
    }
}

fn load_inventory(path: &Path) -> Result<SoftwareInventoryV1, ApplicationError> {
    let bytes = fs::read(path).map_err(|error| {
        ApplicationError::failed(
            "inventory_read_failed",
            format!("failed to read {}: {error}", path.display()),
        )
    })?;
    if let Ok(envelope) = serde_json::from_slice::<serde_json::Value>(&bytes)
        && envelope.get("schema_version") == Some(&serde_json::json!(1))
        && envelope.get("command") == Some(&serde_json::json!("software.inventory"))
        && let Some(data) = envelope.get("data")
    {
        return serde_json::from_value(data.clone()).map_err(|error| {
            ApplicationError::failed(
                "invalid_software_inventory",
                format!("failed to decode inventory {}: {error}", path.display()),
            )
        });
    }
    serde_json::from_slice(&bytes).map_err(|error| {
        ApplicationError::failed(
            "invalid_software_inventory",
            format!("failed to decode inventory {}: {error}", path.display()),
        )
    })
}

fn emit_inventory(
    cli: &Cli,
    locale: Locale,
    inventory: &SoftwareInventoryV1,
) -> Result<(), ApplicationError> {
    match cli.result_format() {
        ResultFormat::Human => {
            let text = presentation::software::inventory(locale, inventory).map_err(|error| {
                ApplicationError::failed("catalogue_render_failed", error.to_string())
            })?;
            let mut sink = OutputSink::open(cli.output_path())?;
            let mut bytes = text.into_bytes();
            bytes.push(b'\n');
            sink.write_all(&bytes)
                .map_err(|error| ApplicationError::failed("output_write_failed", error.to_string()))
        }
        _ => {
            let outcome = if inventory
                .sources
                .iter()
                .all(|evidence| evidence.state == SoftwareSourceState::Available)
            {
                "succeeded"
            } else {
                "partial"
            };
            let mut sink = OutputSink::open(cli.output_path())?;
            write_json(
                &mut sink,
                &serde_json::json!({
                    "schema_version": 1,
                    "command": "software.inventory",
                    "outcome": outcome,
                    "data": inventory,
                    "warnings": [],
                    "error": null
                }),
            )
        }
    }
}

fn map_plan_error(error: SoftwarePlanError) -> ApplicationError {
    let (exit, code) = match error {
        SoftwarePlanError::UnsupportedInventoryVersion(_)
        | SoftwarePlanError::UnsupportedPlanVersion(_)
        | SoftwarePlanError::InvalidFingerprint
        | SoftwarePlanError::InventoryFingerprintMismatch
        | SoftwarePlanError::InventoryExpired
        | SoftwarePlanError::StaleSelection(_)
        | SoftwarePlanError::DigestMismatch => {
            (ExitClass::InvalidAuthority, "invalid_software_authority")
        }
        SoftwarePlanError::EmptySelection
        | SoftwarePlanError::DuplicateSelection(_)
        | SoftwarePlanError::UnknownSelection(_)
        | SoftwarePlanError::ManualSelection(_)
        | SoftwarePlanError::InvalidSelectedIdentity(_) => {
            (ExitClass::Failed, "invalid_software_selection")
        }
        SoftwarePlanError::SerializationFailed => {
            (ExitClass::Failed, "software_plan_serialize_failed")
        }
    };
    ApplicationError::new(exit, code, error.to_string())
}

fn unix_ms(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |duration| {
        u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use devsweep_core::software::{
        SoftwareEntryV1, SoftwareLastUsedEvidence, SoftwareSizeEvidence, SoftwareSourceEvidence,
        SoftwareSourceId,
    };
    use tempfile::TempDir;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("software CLI parses")
    }

    fn partial_inventory() -> SoftwareInventoryV1 {
        SoftwareInventoryV1 {
            version: 1,
            observed_at_unix_ms: 7,
            sources: vec![SoftwareSourceEvidence {
                source: SoftwareSourceId::MsixCurrentUser,
                state: SoftwareSourceState::Partial,
                reason_code: Some("hostile_fixture_incomplete".to_string()),
            }],
            entries: Vec::<SoftwareEntryV1>::new(),
            fingerprint: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
        }
    }

    #[test]
    fn software_inventory_json_is_locale_invariant_and_preserves_partial_evidence() {
        let fixture = TempDir::new().unwrap();
        let en_path = fixture.path().join("en.json");
        let zh_path = fixture.path().join("zh.json");
        let en = parse(&[
            "devsweep",
            "software",
            "inventory",
            "--format",
            "json",
            "--output",
            en_path.to_str().unwrap(),
        ]);
        let zh = parse(&[
            "devsweep",
            "software",
            "inventory",
            "--format",
            "json",
            "--output",
            zh_path.to_str().unwrap(),
        ]);
        let inventory = partial_inventory();
        emit_inventory(&en, Locale::En, &inventory).unwrap();
        emit_inventory(&zh, Locale::ZhCn, &inventory).unwrap();
        let en_bytes = fs::read(&en_path).unwrap();
        let zh_bytes = fs::read(&zh_path).unwrap();
        assert_eq!(en_bytes, zh_bytes);
        let document: serde_json::Value = serde_json::from_slice(&en_bytes).unwrap();
        assert_eq!(document["outcome"], "partial");
        assert_eq!(document["data"]["sources"][0]["state"], "partial");
        assert_eq!(
            document["data"]["sources"][0]["reason_code"],
            "hostile_fixture_incomplete"
        );
    }

    #[test]
    fn hostile_inventory_fields_cannot_enter_a_saved_plan() {
        let fixture = TempDir::new().unwrap();
        let inventory_path = fixture.path().join("hostile.json");
        let plan_path = fixture.path().join("plan.json");
        fs::write(
            &inventory_path,
            br#"{
                "version":1,
                "observed_at_unix_ms":7,
                "sources":[],
                "entries":[],
                "fingerprint":"sha256:0000000000000000000000000000000000000000000000000000000000000000",
                "UninstallString":"\"cmd.exe /c calc\" & %TEMP% | ${env:USERPROFILE}",
                "QuietUninstallString":"powershell -Command Remove-Item",
                "DisplayIcon":"C:\\hostile.exe,0"
            }"#,
        )
        .unwrap();
        let command = SoftwarePlanCommand {
            inventory: inventory_path,
            select: vec!["software:v1:msix:hostile".to_string()],
            output: plan_path.clone(),
        };
        let error = run_plan(&command).expect_err("unknown hostile fields fail closed");
        assert_eq!(error.code, "invalid_software_inventory");
        assert!(!plan_path.exists());
        assert!(!error.message.contains("cmd.exe"));
        assert!(!error.message.contains("Remove-Item"));
    }

    #[test]
    fn truthful_last_used_shape_is_not_relabelled_by_the_cli() {
        let evidence = SoftwareLastUsedEvidence::default();
        let value = serde_json::to_value(evidence).unwrap();
        assert_eq!(value["state"], "unknown");
        assert_eq!(value["reason_code"], "no_supported_exact_source");
        assert!(
            !serde_json::to_string(&SoftwareSizeEvidence::Unknown {
                reason_code: "not_reported".to_string(),
            })
            .unwrap()
            .contains("InstallDate")
        );
    }
}
