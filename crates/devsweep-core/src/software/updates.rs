//! Read-only `winget upgrade` probe and column-position table parser.
//!
//! The probe runs through [`ProcessRunner`] with program and argv separate. It
//! never accepts source or package agreements and never runs an upgrade.

use std::{ffi::OsString, sync::Arc, time::SystemTime};

use serde::{Deserialize, Serialize};

use super::{SoftwareInventoryV1, unix_ms};
use crate::process::{
    CancelObserver, CwdPolicy, DEFAULT_PROVIDER_PHASE_DEADLINE, FlagCancelObserver,
    NoopCancelObserver, ProcessRequest, ProcessResult, ProcessRunner, ProcessStatus,
};

/// Software update-check wire version.
pub const SOFTWARE_UPDATES_VERSION: u32 = 1;

const WINGET_PROGRAM: &str = "winget";
const WINGET_UPGRADE_ARGS: [&str; 2] = ["upgrade", "--disable-interactivity"];
const TABLE_COLUMNS: usize = 5;
const TRUNCATION_MARK: char = '\u{2026}';

/// Exact winget messages for an empty upgrade table (English and zh-CN).
const NO_UPDATE_MESSAGES: &[&str] = &[
    "No installed package found matching input criteria.",
    "No available upgrade found.",
    "No applicable upgrade found.",
    "找不到与输入条件匹配的已安装程序包。",
    "找不到可用的升级。",
    "找不到适用的升级。",
];

/// Locale-neutral update-check result. Unknown output is never an empty success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum SoftwareUpdatesV1 {
    Available {
        version: u32,
        observed_at_unix_ms: u64,
        rows: Vec<SoftwareUpdateRowV1>,
    },
    Unavailable {
        version: u32,
        observed_at_unix_ms: u64,
        reason_code: SoftwareUpdatesReason,
    },
}

/// One parsed `winget upgrade` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareUpdateRowV1 {
    pub id: String,
    pub name: String,
    /// True when winget shortened the name with a trailing ellipsis.
    pub name_truncated: bool,
    pub installed_version: String,
    pub available_version: String,
    pub source: String,
    /// Inventory entries whose display name equals `name` (case-insensitive).
    pub matched_software_ids: Vec<String>,
}

/// Stable reason for an unavailable update check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareUpdatesReason {
    WingetMissing,
    SourceAgreementPending,
    NonZeroExit,
    TimedOut,
    Canceled,
    OutputTruncated,
    UnrecognizedOutput,
    ProcessFailed,
}

/// Runs the read-only winget probe and matches rows to `inventory` by name.
pub fn check_software_updates(
    inventory: Option<&SoftwareInventoryV1>,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> SoftwareUpdatesV1 {
    let runner = ProcessRunner::default();
    check_updates_with(|request| runner.run(request), inventory, cancel)
}

fn check_updates_with(
    run: impl FnOnce(&ProcessRequest<'_>) -> ProcessResult,
    inventory: Option<&SoftwareInventoryV1>,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> SoftwareUpdatesV1 {
    let noop = NoopCancelObserver;
    let observer: &dyn CancelObserver = match cancel {
        Some(flag) => flag.as_ref(),
        None => &noop,
    };
    let request = update_request(observer);
    let result = run(&request);
    let observed_at_unix_ms = unix_ms(SystemTime::now());
    match classify(&result) {
        Ok(rows) => SoftwareUpdatesV1::Available {
            version: SOFTWARE_UPDATES_VERSION,
            observed_at_unix_ms,
            rows: rows
                .into_iter()
                .map(|row| match_row(row, inventory))
                .collect(),
        },
        Err(reason_code) => SoftwareUpdatesV1::Unavailable {
            version: SOFTWARE_UPDATES_VERSION,
            observed_at_unix_ms,
            reason_code,
        },
    }
}

fn update_request(cancel: &dyn CancelObserver) -> ProcessRequest<'_> {
    ProcessRequest {
        program: OsString::from(WINGET_PROGRAM),
        args: WINGET_UPGRADE_ARGS.iter().map(OsString::from).collect(),
        cwd: CwdPolicy::Neutral,
        timeout: Some(DEFAULT_PROVIDER_PHASE_DEADLINE),
        job_deadline: None,
        cancel,
    }
}

fn classify(result: &ProcessResult) -> Result<Vec<ParsedRow>, SoftwareUpdatesReason> {
    let agreement_pending = || {
        let stdout = String::from_utf8_lossy(&result.output.stdout);
        let stderr = String::from_utf8_lossy(&result.output.stderr);
        mentions_agreement(&stdout) || mentions_agreement(&stderr)
    };
    match result.status {
        ProcessStatus::NotFound => return Err(SoftwareUpdatesReason::WingetMissing),
        ProcessStatus::Timeout => return Err(SoftwareUpdatesReason::TimedOut),
        ProcessStatus::Canceled => return Err(SoftwareUpdatesReason::Canceled),
        ProcessStatus::InvalidOutput => return Err(SoftwareUpdatesReason::ProcessFailed),
        ProcessStatus::Exit { .. } => {
            return Err(if agreement_pending() {
                SoftwareUpdatesReason::SourceAgreementPending
            } else {
                SoftwareUpdatesReason::NonZeroExit
            });
        }
        ProcessStatus::Success => {}
    }
    if result.output.stdout_truncated {
        return Err(SoftwareUpdatesReason::OutputTruncated);
    }
    let stdout = std::str::from_utf8(&result.output.stdout)
        .map_err(|_| SoftwareUpdatesReason::UnrecognizedOutput)?;
    parse_winget_upgrade(stdout).map_err(|reason| {
        if agreement_pending() {
            SoftwareUpdatesReason::SourceAgreementPending
        } else {
            reason
        }
    })
}

fn mentions_agreement(text: &str) -> bool {
    text.to_ascii_lowercase().contains("agreement") || text.contains("协议")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedRow {
    id: String,
    name: String,
    name_truncated: bool,
    installed_version: String,
    available_version: String,
    source: String,
}

fn match_row(row: ParsedRow, inventory: Option<&SoftwareInventoryV1>) -> SoftwareUpdateRowV1 {
    let wanted = row.name.to_lowercase();
    let matched_software_ids = if row.name_truncated {
        Vec::new()
    } else {
        inventory
            .map(|inventory| {
                inventory
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry
                            .display_name
                            .as_deref()
                            .is_some_and(|name| name.trim().to_lowercase() == wanted)
                    })
                    .map(|entry| entry.id.clone())
                    .collect()
            })
            .unwrap_or_default()
    };
    SoftwareUpdateRowV1 {
        id: row.id,
        name: row.name,
        name_truncated: row.name_truncated,
        installed_version: row.installed_version,
        available_version: row.available_version,
        source: row.source,
        matched_software_ids,
    }
}

/// Parses the first `winget upgrade` table by display-width column offsets.
/// Header words are never interpreted, so localized headers parse the same way.
fn parse_winget_upgrade(stdout: &str) -> Result<Vec<ParsedRow>, SoftwareUpdatesReason> {
    let lines = stdout
        .split('\n')
        .map(|line| {
            let line = line.strip_suffix('\r').unwrap_or(line);
            // Progress output rewrites the line with carriage returns; keep
            // only the text after the last one.
            line.rsplit('\r').next().unwrap_or(line)
        })
        .collect::<Vec<_>>();
    let Some(separator) = lines.iter().position(|line| is_separator(line)) else {
        let no_updates = lines
            .iter()
            .any(|line| NO_UPDATE_MESSAGES.contains(&line.trim()));
        return if no_updates {
            Ok(Vec::new())
        } else {
            Err(SoftwareUpdatesReason::UnrecognizedOutput)
        };
    };
    let header = lines[..separator]
        .iter()
        .rev()
        .find(|line| !line.trim().is_empty())
        .ok_or(SoftwareUpdatesReason::UnrecognizedOutput)?;
    let starts = column_starts(header);
    if starts.len() != TABLE_COLUMNS {
        return Err(SoftwareUpdatesReason::UnrecognizedOutput);
    }
    let mut rows = Vec::new();
    for line in &lines[separator + 1..] {
        match parse_row(line, &starts) {
            Some(row) => rows.push(row),
            None => break,
        }
    }
    if rows.is_empty() {
        return Err(SoftwareUpdatesReason::UnrecognizedOutput);
    }
    Ok(rows)
}

fn is_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= 10 && trimmed.chars().all(|character| character == '-')
}

/// Display-width offsets of each whitespace-separated header token.
fn column_starts(header: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut offset = 0;
    let mut previous_space = true;
    for character in header.chars() {
        let space = character == ' ';
        if !space && previous_space {
            starts.push(offset);
        }
        previous_space = space;
        offset += display_width(character);
    }
    starts
}

fn parse_row(line: &str, starts: &[usize]) -> Option<ParsedRow> {
    let mut columns = vec![String::new(); starts.len()];
    let mut offset = 0;
    for character in line.chars() {
        let width = display_width(character);
        // Each column boundary must be preceded by a separating space.
        if starts[1..].contains(&(offset + width)) && character != ' ' {
            return None;
        }
        let column = starts.iter().rposition(|start| *start <= offset)?;
        columns[column].push(character);
        offset += width;
    }
    let mut columns = columns.into_iter().map(|value| value.trim().to_string());
    let name = columns.next()?;
    let id = columns.next()?;
    let installed_version = columns.next()?;
    let available_version = columns.next()?;
    let source = columns.next()?;
    let single_token = |value: &str| !value.is_empty() && !value.contains(char::is_whitespace);
    if name.is_empty()
        || installed_version.is_empty()
        || !single_token(&id)
        || !single_token(&available_version)
        || !single_token(&source)
    {
        return None;
    }
    let (name, name_truncated) = match name.strip_suffix(TRUNCATION_MARK) {
        Some(stripped) => (stripped.trim_end().to_string(), true),
        None => (name, false),
    };
    Some(ParsedRow {
        id,
        name,
        name_truncated,
        installed_version,
        available_version,
        source,
    })
}

/// Terminal column width: East Asian Wide and Fullwidth characters count as 2.
fn display_width(character: char) -> usize {
    let code = u32::from(character);
    let wide = matches!(
        code,
        0x1100..=0x115F
            | 0x2E80..=0x303E
            | 0x3041..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA000..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1F64F
            | 0x1F900..=0x1F9FF
            | 0x20000..=0x2FFFD
            | 0x30000..=0x3FFFD
    );
    if wide { 2 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::ProcessOutput;

    const ENGLISH: &str = include_str!("fixtures/winget-upgrade-en.txt");
    const CHINESE: &str = include_str!("fixtures/winget-upgrade-zh-CN.txt");

    fn success(stdout: &str) -> ProcessResult {
        ProcessResult {
            status: ProcessStatus::Success,
            output: ProcessOutput {
                stdout: stdout.as_bytes().to_vec(),
                ..ProcessOutput::default()
            },
        }
    }

    fn rows(updates: SoftwareUpdatesV1) -> Vec<SoftwareUpdateRowV1> {
        match updates {
            SoftwareUpdatesV1::Available { rows, .. } => rows,
            SoftwareUpdatesV1::Unavailable { reason_code, .. } => {
                panic!("expected rows, got {reason_code:?}")
            }
        }
    }

    fn reason(updates: SoftwareUpdatesV1) -> SoftwareUpdatesReason {
        match updates {
            SoftwareUpdatesV1::Unavailable { reason_code, .. } => reason_code,
            SoftwareUpdatesV1::Available { rows, .. } => {
                panic!("expected unavailable, got {rows:?}")
            }
        }
    }

    #[test]
    fn english_fixture_parses_every_row_and_ignores_summary_lines() {
        let parsed = parse_winget_upgrade(ENGLISH).expect("english table");
        assert_eq!(parsed.len(), 13);
        assert_eq!(
            parsed[0],
            ParsedRow {
                id: "Bandisoft.Bandizip".into(),
                name: "Bandizip Professional".into(),
                name_truncated: false,
                installed_version: "7.43".into(),
                available_version: "7.46".into(),
                source: "winget".into(),
            }
        );
        let physx = parsed.iter().find(|row| row.id == "Nvidia.PhysX").unwrap();
        assert_eq!(physx.name, "NVIDIA PhysX 系统软件 9.23.1019");
        assert_eq!(physx.installed_version, "9.23.1019");
        assert_eq!(physx.available_version, "9.26.0703");
        let nutstore = parsed
            .iter()
            .find(|row| row.id == "Nutstore.Nutstore")
            .unwrap();
        assert_eq!(nutstore.name, "坚果云");
        assert_eq!(nutstore.available_version, "7.2.12");
        let t3 = parsed
            .iter()
            .find(|row| row.id == "T3Tools.T3Code")
            .unwrap();
        assert_eq!(t3.installed_version, "0.0.38-nightly.20260831.1236");
    }

    #[test]
    fn chinese_fixture_parses_by_display_width_not_header_words() {
        let parsed = parse_winget_upgrade(CHINESE).expect("zh-CN table");
        assert_eq!(parsed.len(), 4);
        assert_eq!(parsed[0].id, "Bandisoft.Bandizip");
        assert_eq!(parsed[1].name, "NVIDIA PhysX 系统软件 9.23.1019");
        assert_eq!(parsed[1].source, "winget");
        assert_eq!(parsed[2].name, "坚果云");
        assert_eq!(parsed[2].installed_version, "7.2.7");
        assert!(parsed[3].name_truncated);
        assert_eq!(parsed[3].name, "Microsoft Visual C++ 2015-2022 Redistr");
        assert_eq!(parsed[3].id, "Microsoft.VCRedist.2015+.x64");
    }

    #[test]
    fn cjk_characters_count_two_columns() {
        assert_eq!(display_width('坚'), 2);
        assert_eq!(display_width('A'), 1);
        assert_eq!(display_width(TRUNCATION_MARK), 1);
        assert_eq!(column_starts("名称  ID  版本"), vec![0, 6, 10]);
    }

    #[test]
    fn truncated_names_lose_the_ellipsis_and_never_match_inventory() {
        let table = "Name                 Id          Version Available Source\n\
                     -----------------------------------------------------------\n\
                     Very Long Product N\u{2026} Vendor.Long 1.0     2.0       winget\n";
        let parsed = parse_winget_upgrade(table).expect("table");
        assert_eq!(parsed[0].name, "Very Long Product N");
        assert!(parsed[0].name_truncated);
        let row = match_row(parsed[0].clone(), None);
        assert!(row.matched_software_ids.is_empty());
    }

    #[test]
    fn no_update_messages_are_an_empty_success_in_both_locales() {
        for message in [
            "No installed package found matching input criteria.\r\n",
            "No available upgrade found.\r\n",
            "  \r找不到与输入条件匹配的已安装程序包。\r\n",
        ] {
            assert!(rows(check_updates_with(|_| success(message), None, None)).is_empty());
        }
    }

    #[test]
    fn unknown_output_and_failures_are_unavailable_never_empty_success() {
        assert_eq!(
            reason(check_updates_with(|_| success(""), None, None)),
            SoftwareUpdatesReason::UnrecognizedOutput
        );
        assert_eq!(
            reason(check_updates_with(
                |_| success("Something changed.\n"),
                None,
                None
            )),
            SoftwareUpdatesReason::UnrecognizedOutput
        );
        let four_columns = "Name   Id   Version   Available\n--------------------------------\n\
                            A      B    1         2\n";
        assert_eq!(
            reason(check_updates_with(|_| success(four_columns), None, None)),
            SoftwareUpdatesReason::UnrecognizedOutput
        );
        let exit = |code| ProcessResult {
            status: ProcessStatus::Exit { code: Some(code) },
            output: ProcessOutput::default(),
        };
        assert_eq!(
            reason(check_updates_with(|_| exit(-1_978_335_212), None, None)),
            SoftwareUpdatesReason::NonZeroExit
        );
        let agreement = ProcessResult {
            status: ProcessStatus::Exit { code: Some(1) },
            output: ProcessOutput {
                stdout: "The `msstore` source requires that you view the following agreements before using.\n"
                    .as_bytes()
                    .to_vec(),
                ..ProcessOutput::default()
            },
        };
        assert_eq!(
            reason(check_updates_with(|_| agreement.clone(), None, None)),
            SoftwareUpdatesReason::SourceAgreementPending
        );
        for (status, expected) in [
            (
                ProcessStatus::NotFound,
                SoftwareUpdatesReason::WingetMissing,
            ),
            (ProcessStatus::Timeout, SoftwareUpdatesReason::TimedOut),
            (ProcessStatus::Canceled, SoftwareUpdatesReason::Canceled),
            (
                ProcessStatus::InvalidOutput,
                SoftwareUpdatesReason::ProcessFailed,
            ),
        ] {
            let result = ProcessResult {
                status,
                output: ProcessOutput::default(),
            };
            assert_eq!(
                reason(check_updates_with(|_| result.clone(), None, None)),
                expected
            );
        }
        let truncated = ProcessResult {
            status: ProcessStatus::Success,
            output: ProcessOutput {
                stdout: ENGLISH.as_bytes().to_vec(),
                stdout_truncated: true,
                ..ProcessOutput::default()
            },
        };
        assert_eq!(
            reason(check_updates_with(|_| truncated.clone(), None, None)),
            SoftwareUpdatesReason::OutputTruncated
        );
    }

    #[test]
    fn update_argv_is_separate_bounded_and_never_accepts_agreements() {
        let mut captured = None;
        let _ = check_updates_with(
            |request| {
                captured = Some((
                    request.program.clone(),
                    request.args.clone(),
                    request.cwd.clone(),
                    request.timeout,
                ));
                success(ENGLISH)
            },
            None,
            None,
        );
        let (program, args, cwd, timeout) = captured.expect("probe went through ProcessRequest");
        assert_eq!(program, OsString::from("winget"));
        assert_eq!(
            args,
            vec![
                OsString::from("upgrade"),
                OsString::from("--disable-interactivity")
            ]
        );
        assert_eq!(cwd, CwdPolicy::Neutral);
        assert_eq!(timeout, Some(DEFAULT_PROVIDER_PHASE_DEADLINE));
        for arg in &args {
            let arg = arg.to_string_lossy();
            assert!(!arg.starts_with("--accept"), "agreement flag {arg}");
            assert!(!arg.contains(' '), "shell-composed argument {arg}");
        }
        let source = include_str!("updates.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        for forbidden in [
            ["--accept-source", "-agreements"].concat(),
            ["--accept-package", "-agreements"].concat(),
            ["Command", "::new"].concat(),
            ["cmd", ".exe"].concat(),
            ["power", "shell"].concat(),
        ] {
            assert!(!production.contains(&forbidden), "{forbidden}");
        }
    }

    #[test]
    fn rows_match_inventory_by_exact_case_insensitive_name() {
        let inventory: SoftwareInventoryV1 = serde_json::from_value(serde_json::json!({
            "version": 1,
            "observed_at_unix_ms": 1,
            "sources": [],
            "entries": [{
                "id": "software:v1:arp:a",
                "identity": {"source": "arp", "hive": "current_user", "view": "registry64", "subkey": "Obsidian"},
                "scope": "current_user",
                "display_name": "obsidian",
                "provenance": [],
                "eligibility": {"state": "manual", "reason": "registry_only_manual"},
                "size": {"state": "unknown", "reason_code": "not_reported"},
                "last_used": {"state": "unknown", "reason_code": "no_supported_exact_source"}
            }],
            "fingerprint": format!("sha256:{}", "0".repeat(64)),
        }))
        .unwrap();
        let updates = rows(check_updates_with(
            |_| success(ENGLISH),
            Some(&inventory),
            None,
        ));
        let obsidian = updates
            .iter()
            .find(|row| row.id == "Obsidian.Obsidian")
            .unwrap();
        assert_eq!(obsidian.matched_software_ids, vec!["software:v1:arp:a"]);
        assert!(
            updates
                .iter()
                .filter(|row| row.id != "Obsidian.Obsidian")
                .all(|row| row.matched_software_ids.is_empty())
        );
    }
}
