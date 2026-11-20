//! Shared host-action review. Only a worker may prepare or dispatch an action.
//! Display fixtures never become production update or startup authority.

mod leftovers;
pub(crate) use leftovers::Leftovers;
mod uninstall;
pub(crate) use uninstall::UninstallSelection;

use std::{
    ffi::OsStr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use devsweep_core::{
    process::{CancelObserver, FlagCancelObserver},
    software::{self, SoftwareUpdatePlanV1, SoftwareUpdateRowV1, StartupTogglePlanV1},
    status::{
        self, KeepAwakePlan, KeepAwakeService, KeepAwakeSession, ProcessActionAudit, ProcessKiller,
        ProcessProbe, ProcessTerminationPlanV1,
    },
};

use crate::copy::{self, Locale};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartupSource {
    Host,
    Hive(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    Uninstall(uninstall::UninstallSelection),
    Leftovers(Leftovers),
    KeepAwake {
        enabled: bool,
    },
    Startup {
        source: StartupSource,
        id: String,
        name: String,
        enabled: bool,
    },
    Update {
        row: SoftwareUpdateRowV1,
    },
    Process {
        pid: u32,
        name: String,
    },
}

impl Intent {
    pub fn id(&self) -> &str {
        match self {
            Self::Uninstall(row) => &row.id,
            Self::Leftovers(row) => &row.id,
            Self::KeepAwake { .. } => "keep-awake",
            Self::Startup { id, .. } => id,
            Self::Update { row } => &row.id,
            Self::Process { .. } => "process-termination",
        }
    }

    pub fn label(&self, locale: Locale) -> String {
        match self {
            Self::Leftovers(row) => row.label(locale),
            Self::Uninstall(row) => format!(
                "{} {} · {}",
                copy::pick(
                    locale,
                    "Uninstall for the current user:",
                    "为当前用户卸载："
                ),
                row.name,
                row.identity_label()
            ),
            Self::KeepAwake { enabled } => if *enabled {
                copy::pick(
                    locale,
                    "Keep this session awake for one hour; release on exit",
                    "此会话保持唤醒一小时，退出时释放",
                )
            } else {
                copy::pick(
                    locale,
                    "Stop this session's keep-awake request",
                    "停止此会话的保持唤醒请求",
                )
            }
            .to_string(),
            Self::Startup { name, enabled, .. } => format!(
                "{} {name}",
                if *enabled {
                    copy::pick(locale, "Enable startup:", "启用启动项：")
                } else {
                    copy::pick(locale, "Disable startup:", "禁用启动项：")
                }
            ),
            Self::Update { row } => format!(
                "{}: {} ({} → {})",
                copy::pick(locale, "Update current-user app", "更新当前用户应用"),
                row.name,
                row.installed_version,
                row.available_version
            ),
            Self::Process { pid, name } => format!(
                "{} {name} (PID {pid})",
                copy::pick(locale, "End process:", "结束进程：")
            ),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Plan {
    Uninstall(software::SoftwareSelectionPlanV1),
    LeftoverChoices(Vec<software::SoftwareLeftoverCandidateV1>),
    Leftover(software::SoftwareLeftoverPlanV1),
    KeepAwake(KeepAwakePlan),
    Startup(StartupTogglePlanV1),
    Update(SoftwareUpdatePlanV1),
    Process(ProcessTerminationPlanV1),
}

#[derive(Clone, Debug)]
pub struct Prepared {
    pub intent: Intent,
    pub plan: Plan,
    pub digest: String,
}

#[derive(Clone, Debug)]
pub struct ActionResult {
    pub intent: Intent,
    pub code: String,
    pub reverse: Option<Intent>,
    pub uninstall: Option<software::SoftwareActionOutcomeV1>,
}

#[derive(Clone, Debug)]
pub struct AwakeSnapshot {
    pub state: String,
    pub endpoint: String,
    pub token: String,
    pub owner: String,
    pub remaining_ms: u64,
}

struct AwakeOwner {
    service: Option<KeepAwakeService>,
    session: Option<KeepAwakeSession>,
    production_allowed: bool,
}

impl Default for AwakeOwner {
    fn default() -> Self {
        Self {
            service: None,
            session: None,
            production_allowed: true,
        }
    }
}

/// Production is closed over core's trusted functions. Tests inject this
/// boundary directly; environment variables cannot replace an executor.
pub trait ActionExecutor: Send + Sync {
    fn prepare(&self, intent: Intent) -> Result<Prepared, String>;
    fn execute(&self, prepared: &Prepared, confirmed: bool) -> Result<ActionResult, String>;
    fn execute_cancellable(
        &self,
        prepared: &Prepared,
        confirmed: bool,
        cancel: &Arc<FlagCancelObserver>,
    ) -> Result<ActionResult, String> {
        if cancel.is_cancel_requested() {
            return Err("canceled_before_start".to_string());
        }
        self.execute(prepared, confirmed)
    }
    fn query_awake(&self) -> Result<Option<AwakeSnapshot>, String> {
        Ok(None)
    }
    fn shutdown(&self) -> Result<(), String> {
        Ok(())
    }
}

pub struct CoreExecutor {
    awake: Mutex<AwakeOwner>,
    process: Option<Mutex<ProcessOwner>>,
    hive_only: bool,
}

struct ProcessOwner {
    probe: Box<dyn ProcessProbe + Send>,
    killer: Box<dyn ProcessKiller + Send>,
    audit: Box<dyn ProcessActionAudit + Send>,
}

impl Default for CoreExecutor {
    fn default() -> Self {
        Self {
            awake: Mutex::new(AwakeOwner::default()),
            hive_only: false,
            process: Some(Mutex::new(ProcessOwner {
                probe: Box::new(status::HostProcessControl),
                killer: Box::new(status::HostProcessControl),
                audit: Box::new(status::FileProcessActionAudit::production()),
            })),
        }
    }
}

impl CoreExecutor {
    pub(crate) fn for_startup_hive() -> Self {
        Self {
            awake: Mutex::new(AwakeOwner {
                production_allowed: false,
                ..AwakeOwner::default()
            }),
            process: None,
            hive_only: true,
        }
    }

    fn check_intent(&self, intent: &Intent) -> Result<(), String> {
        if self.hive_only
            && !matches!(
                intent,
                Intent::Startup {
                    source: StartupSource::Hive(_),
                    ..
                }
            )
        {
            return Err("unbacked_session".into());
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn with_awake(service: KeepAwakeService) -> Self {
        Self {
            awake: Mutex::new(AwakeOwner {
                service: Some(service),
                session: None,
                production_allowed: false,
            }),
            process: None,
            hive_only: false,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_process(
        probe: Box<dyn ProcessProbe + Send>,
        killer: Box<dyn ProcessKiller + Send>,
        audit: Box<dyn ProcessActionAudit + Send>,
    ) -> Self {
        Self {
            hive_only: false,
            awake: Mutex::new(AwakeOwner {
                production_allowed: false,
                ..AwakeOwner::default()
            }),
            process: Some(Mutex::new(ProcessOwner {
                probe,
                killer,
                audit,
            })),
        }
    }
}

impl ActionExecutor for CoreExecutor {
    fn prepare(&self, intent: Intent) -> Result<Prepared, String> {
        self.check_intent(&intent)?;
        let (plan, digest) = match &intent {
            Intent::Leftovers(row) => return row.prepare(),
            Intent::Uninstall(row) => {
                let inventory = software::inventory_software(row.source(), None)
                    .map_err(|_| "software_revalidation_unavailable")?;
                let (plan, digest) =
                    row.preview(&inventory, crate::software_review::unix_now_ms())?;
                (Plan::Uninstall(plan), digest)
            }
            Intent::Process { pid, name } => {
                let process = self
                    .process
                    .as_ref()
                    .ok_or("unbacked_session")?
                    .lock()
                    .map_err(|_| "worker_unavailable")?;
                let live = process
                    .probe
                    .lookup(*pid)
                    .map_err(|error| error.code())?
                    .ok_or("invalid_identity")?;
                if live.pid != *pid || !live.name.eq_ignore_ascii_case(name) {
                    return Err("invalid_identity".into());
                }
                let plan = status::bind_process_termination_plan(
                    live.pid,
                    live.creation_time_filetime,
                    &live.name,
                )
                .map_err(|error| error.code())?;
                let preview = status::preview_bound_process(&plan, process.probe.as_ref())
                    .map_err(|error| error.code())?;
                (Plan::Process(plan), preview.digest)
            }
            Intent::KeepAwake { enabled } => {
                let mut owner = self
                    .awake
                    .lock()
                    .map_err(|_| "worker_unavailable".to_string())?;
                let duration = Duration::from_secs(3_600);
                let plan = if let Some(session) = &owner.session {
                    if *enabled {
                        session.plan_start(duration)
                    } else {
                        session.plan_stop()
                    }
                } else if *enabled {
                    if owner.service.is_none() && !owner.production_allowed {
                        return Err("keep_awake_unavailable".to_string());
                    }
                    let service = owner
                        .service
                        .get_or_insert_with(KeepAwakeService::production);
                    service.plan_start(&format!("devsweep-gpui-{}", std::process::id()), duration)
                } else {
                    return Err("keep_awake_unavailable".to_string());
                }
                .map_err(|error| error.code().to_string())?;
                let preview = if let Some(session) = &owner.session {
                    session.preview(&plan)
                } else {
                    owner
                        .service
                        .as_ref()
                        .ok_or("worker_unavailable")?
                        .preview(&plan)
                }
                .map_err(|error| error.code().to_string())?;
                (Plan::KeepAwake(plan), preview.digest)
            }
            Intent::Startup {
                source,
                id,
                enabled,
                ..
            } => {
                let plan = match source {
                    StartupSource::Host => software::plan_startup_toggle(id, *enabled),
                    StartupSource::Hive(path) => software::plan_startup_hive(path, id, *enabled),
                }
                .map_err(|error| error.code().to_string())?;
                let preview = match source {
                    StartupSource::Host => software::preview_startup_toggle(&plan),
                    StartupSource::Hive(path) => software::preview_startup_hive(path, &plan),
                }
                .map_err(|error| error.code().to_string())?;
                (Plan::Startup(plan), preview.digest)
            }
            Intent::Update { row } => {
                let program = OsStr::new("winget");
                let plan = software::plan_software_update(
                    program,
                    &row.id,
                    &row.source,
                    &row.available_version,
                )
                .map_err(|error| error.code().to_string())?;
                if plan.installed_version != row.installed_version {
                    return Err("version_changed".to_string());
                }
                let preview = software::preview_software_update(program, &plan)
                    .map_err(|error| error.code().to_string())?;
                (Plan::Update(plan), preview.digest)
            }
        };
        Ok(Prepared {
            intent,
            plan,
            digest,
        })
    }

    fn execute(&self, prepared: &Prepared, confirmed: bool) -> Result<ActionResult, String> {
        self.execute_cancellable(prepared, confirmed, &Arc::default())
    }

    fn execute_cancellable(
        &self,
        prepared: &Prepared,
        confirmed: bool,
        cancel: &Arc<FlagCancelObserver>,
    ) -> Result<ActionResult, String> {
        if !confirmed {
            return Err("confirmation_required".to_string());
        }
        if cancel.is_cancel_requested() {
            return Err("canceled_before_start".to_string());
        }
        self.check_intent(&prepared.intent)?;
        let (code, reverse) =
            match (&prepared.intent, &prepared.plan) {
                (Intent::Leftovers(row), Plan::Leftover(plan)) => (
                    row.execute(
                        plan,
                        &prepared.digest,
                        confirmed,
                        cancel,
                        software::execute_software_leftover_plan,
                    )?,
                    None,
                ),
                (Intent::Uninstall(row), Plan::Uninstall(plan)) => {
                    return row.execute(
                        plan,
                        &prepared.digest,
                        confirmed,
                        Arc::clone(cancel),
                        |request| software::SoftwareExecutor::default().execute(request),
                    );
                }
                (Intent::Process { pid, name }, Plan::Process(plan)) => {
                    if *pid != plan.pid || !name.eq_ignore_ascii_case(&plan.name) {
                        return Err("invalid_plan".into());
                    }
                    let mut owner = self
                        .process
                        .as_ref()
                        .ok_or("unbacked_session")?
                        .lock()
                        .map_err(|_| "worker_unavailable")?;
                    let ProcessOwner {
                        probe,
                        killer,
                        audit,
                    } = &mut *owner;
                    let report = status::terminate_bound_process_audited_cancellable(
                        plan,
                        &prepared.digest,
                        confirmed,
                        probe.as_ref(),
                        killer.as_mut(),
                        audit.as_mut(),
                        cancel.as_ref(),
                    )
                    .map_err(|error| error.code().to_string())?;
                    (report.outcome, None)
                }
                (Intent::KeepAwake { enabled }, Plan::KeepAwake(plan)) => {
                    let mut owner = self
                        .awake
                        .lock()
                        .map_err(|_| "worker_unavailable".to_string())?;
                    let status = if let Some(session) = &owner.session {
                        if *enabled {
                            session.execute_start(plan, &prepared.digest, confirmed)
                        } else {
                            session.execute_stop(plan, &prepared.digest, confirmed)
                        }
                    } else if *enabled {
                        let service = owner.service.take().ok_or("keep_awake_unavailable")?;
                        let session =
                            KeepAwakeSession::start(service, plan, &prepared.digest, confirmed)
                                .map_err(|error| error.code().to_string())?;
                        let result = session.status();
                        owner.session = Some(session);
                        result
                    } else {
                        return Err("keep_awake_unavailable".to_string());
                    }
                    .map_err(|error| error.code().to_string())?;
                    (status.state, None)
                }
                (
                    Intent::Startup {
                        source,
                        id,
                        name,
                        enabled,
                    },
                    Plan::Startup(plan),
                ) => {
                    let report = match source {
                        StartupSource::Host => {
                            software::execute_startup_toggle(plan, &prepared.digest, confirmed)
                        }
                        StartupSource::Hive(path) => {
                            software::execute_startup_hive(path, plan, &prepared.digest, confirmed)
                                .map(|report| report.toggle)
                        }
                    }
                    .map_err(|error| error.code().to_string())?;
                    let code = match report.outcome {
                        software::StartupStrictOutcome::Succeeded => "succeeded",
                        software::StartupStrictOutcome::Failed => "failed",
                        software::StartupStrictOutcome::Unknown => "unknown",
                        software::StartupStrictOutcome::AlreadyInState => "already_in_state",
                    };
                    let reverse = (report.outcome == software::StartupStrictOutcome::Succeeded)
                        .then(|| Intent::Startup {
                            source: source.clone(),
                            id: id.clone(),
                            name: name.clone(),
                            enabled: !enabled,
                        });
                    (
                        report.error_code.unwrap_or_else(|| code.to_string()),
                        reverse,
                    )
                }
                (Intent::Update { .. }, Plan::Update(plan)) => {
                    let report = software::execute_software_update_cancellable(
                        OsStr::new("winget"),
                        plan,
                        &prepared.digest,
                        confirmed,
                        cancel.as_ref(),
                    )
                    .map_err(|error| error.code().to_string())?;
                    let code = match report.outcome {
                        software::SoftwareUpdateOutcome::Updated => "updated",
                        software::SoftwareUpdateOutcome::RestartRequired => "restart_required",
                        software::SoftwareUpdateOutcome::Failed => "failed",
                        software::SoftwareUpdateOutcome::Unknown => "unknown",
                    };
                    (
                        report
                            .error_code
                            .map_or_else(|| code.to_string(), |error| format!("{code}:{error}")),
                        None,
                    )
                }
                _ => return Err("invalid_plan".to_string()),
            };
        Ok(ActionResult {
            intent: prepared.intent.clone(),
            code,
            reverse,
            uninstall: None,
        })
    }

    fn query_awake(&self) -> Result<Option<AwakeSnapshot>, String> {
        let owner = self
            .awake
            .lock()
            .map_err(|_| "worker_unavailable".to_string())?;
        let Some(session) = &owner.session else {
            return Ok(None);
        };
        let status = session.status().map_err(|error| error.code().to_string())?;
        Ok(Some(AwakeSnapshot {
            state: status.state,
            endpoint: session.endpoint().to_string(),
            token: session.token().to_string(),
            owner: session.owner().to_string(),
            remaining_ms: status.remaining_ms,
        }))
    }

    fn shutdown(&self) -> Result<(), String> {
        let mut owner = match self.awake.lock() {
            Ok(owner) => owner,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(session) = owner.session.as_mut() {
            session
                .shutdown()
                .map(|_| ())
                .map_err(|error| error.code().to_string())
        } else if let Some(service) = owner.service.as_ref() {
            service.shutdown().map_err(|error| error.code().to_string())
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Idle,
    Preparing,
    Choosing,
    Ready,
    Running,
    Finished,
}

#[derive(Default)]
pub struct HostReview {
    pub generation: u64,
    pub phase: Phase,
    pub prepared: Vec<Prepared>,
    pub intents: Vec<Intent>,
    pub results: Vec<ActionResult>,
    pub error: Option<String>,
}

impl HostReview {
    pub fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.prepared.clear();
        self.intents.clear();
        self.results.clear();
        self.error = None;
        self.phase = Phase::Idle;
    }

    pub fn begin(&mut self, intents: Vec<Intent>) -> u64 {
        self.invalidate();
        self.intents = intents;
        self.phase = Phase::Preparing;
        self.generation
    }

    pub fn apply_preview(
        &mut self,
        generation: u64,
        result: Result<Vec<Prepared>, String>,
    ) -> bool {
        if self.generation != generation || self.phase != Phase::Preparing {
            return false;
        }
        match result {
            Ok(prepared) if matches!(prepared.as_slice(), [Prepared { intent: Intent::Leftovers(Leftovers { candidate: None, .. }), plan: Plan::LeftoverChoices(_), digest }] if digest.is_empty()) =>
            {
                self.prepared = prepared;
                self.phase = Phase::Choosing;
            }
            Ok(prepared)
                if !prepared.is_empty()
                    && prepared.iter().all(|item| valid_digest(&item.digest)) =>
            {
                self.prepared = prepared;
                self.phase = Phase::Ready;
            }
            Ok(_) => {
                self.error = Some("invalid_preview".to_string());
                self.phase = Phase::Finished;
            }
            Err(error) => {
                self.error = Some(error);
                self.phase = Phase::Finished;
            }
        }
        true
    }

    pub fn take_confirmed(&mut self, confirmed: bool) -> Option<Vec<Prepared>> {
        if !confirmed || self.phase != Phase::Ready || self.prepared.is_empty() {
            return None;
        }
        // A delayed preview delivery must not release the execution worker's lock.
        self.generation = self.generation.wrapping_add(1);
        self.phase = Phase::Running;
        Some(std::mem::take(&mut self.prepared))
    }

    pub fn apply_results(&mut self, generation: u64, results: Vec<ActionResult>) -> bool {
        if self.generation != generation || self.phase != Phase::Running {
            return false;
        }
        self.results = results;
        self.phase = Phase::Finished;
        true
    }

    pub fn lines(&self, locale: Locale) -> Vec<String> {
        if self.phase == Phase::Idle {
            return Vec::new();
        }
        let mut lines = vec![
            match self.phase {
                Phase::Preparing => {
                    copy::pick(locale, "Checking the live plan…", "正在检查实时计划…")
                }
                Phase::Choosing => copy::pick(
                    locale,
                    "Choose a directory to review. Name matches can include personal files.",
                    "选择要审阅的目录。名称匹配可能包含个人文件。",
                ),
                Phase::Ready => copy::pick(
                    locale,
                    "Review these changes, then confirm.",
                    "审阅以下更改后再确认。",
                ),
                Phase::Running => copy::pick(
                    locale,
                    "Running the reviewed item. Wait for its result.",
                    "正在执行已审阅项目，请等待结果。",
                ),
                _ => copy::pick(locale, "Action results", "动作结果"),
            }
            .to_string(),
        ];
        lines.extend(self.intents.iter().map(|intent| intent.label(locale)));
        for item in &self.prepared {
            if let Plan::LeftoverChoices(choices) = &item.plan {
                if choices.is_empty() {
                    lines.push(
                        copy::pick(
                            locale,
                            "No matching directories found.",
                            "未找到匹配的目录。",
                        )
                        .into(),
                    );
                }
                lines.push(copy::pick(locale, "Registry residue: the CLI supports backup and conditional restore of its DevSweepResidue fixture.", "注册表残留：CLI 支持备份 DevSweepResidue 夹具，并在状态匹配时恢复。").into());
            }
            if let Plan::Leftover(_) = &item.plan {
                lines.push(
                    copy::pick(
                        locale,
                        "Confirm only after checking this exact directory and its contents.",
                        "请先核对这个目录及其内容，再确认移入回收站。",
                    )
                    .into(),
                );
            }
            if let Plan::Uninstall(_) = &item.plan {
                lines.push(copy::pick(locale, "Windows will remove this app for the current user. This action cannot be undone.", "Windows 将为当前用户卸载此应用，无法撤销。").into());
            }
            if let Plan::Process(plan) = &item.plan {
                lines.push(format!(
                    "{} · PID {} · {} {}",
                    plan.name,
                    plan.pid,
                    copy::pick(locale, "Creation FILETIME:", "创建时间 FILETIME："),
                    plan.creation_time_filetime
                ));
                lines.push(
                    copy::pick(
                        locale,
                        status::PROCESS_TERMINATION_DATA_LOSS_TEXT,
                        "结束此进程可能丢失未保存的工作和内存中的数据。",
                    )
                    .to_string(),
                );
            }
        }
        lines.extend(
            self.prepared
                .iter()
                .filter(|item| !item.digest.is_empty())
                .map(|item| format!("{}…", item.digest.chars().take(23).collect::<String>())),
        );
        lines.extend(self.results.iter().map(|result| {
            format!(
                "{} · {}",
                result.intent.label(locale),
                result_label(locale, &result.code)
            )
        }));
        for outcome in self
            .results
            .iter()
            .filter_map(|result| result.uninstall.as_ref())
        {
            lines.extend(uninstall::evidence_lines(outcome, locale));
        }
        if let Some(error) = &self.error {
            lines.push(result_label(locale, error));
        }
        lines
    }
}

pub fn run_previews(
    executor: &dyn ActionExecutor,
    intents: Vec<Intent>,
    cancel: &FlagCancelObserver,
) -> Result<Vec<Prepared>, String> {
    intents
        .into_iter()
        .map(|intent| {
            if cancel.is_cancel_requested() {
                return Err("canceled_before_start".to_string());
            }
            executor.prepare(intent)
        })
        .collect()
}

pub fn run_actions(
    executor: Arc<dyn ActionExecutor>,
    prepared: Vec<Prepared>,
    cancel: &Arc<FlagCancelObserver>,
) -> Vec<ActionResult> {
    prepared
        .into_iter()
        .map(|item| {
            if cancel.is_cancel_requested() {
                return ActionResult {
                    intent: item.intent,
                    code: "canceled_before_start".to_string(),
                    reverse: None,
                    uninstall: None,
                };
            }
            executor
                .execute_cancellable(&item, true, cancel)
                .unwrap_or_else(|code| ActionResult {
                    intent: item.intent,
                    code,
                    reverse: None,
                    uninstall: None,
                })
        })
        .collect()
}

fn valid_digest(digest: &str) -> bool {
    digest.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

pub fn result_label(locale: Locale, code: &str) -> String {
    let label = match code.split_once(':').map_or(code, |(kind, _)| kind) {
        "succeeded" | "updated" => copy::pick(locale, "Completed", "已完成"),
        "removed" => copy::pick(locale, "Uninstalled", "已卸载"),
        "leftover_audit_unknown" => copy::pick(
            locale,
            "The final move could not be verified in history",
            "无法通过历史记录验证最终移动结果",
        ),
        "leftover_moved" => copy::pick(locale, "Moved to Recycle Bin", "已移入回收站"),
        "leftover_missing" => copy::pick(
            locale,
            "Skipped because the directory is gone",
            "已跳过，目录已不存在",
        ),
        "leftover_protected" => copy::pick(
            locale,
            "Skipped because the directory is protected",
            "已跳过，目录受保护",
        ),
        "leftover_unsafe" => copy::pick(
            locale,
            "Skipped because the path is unsafe",
            "已跳过，路径不安全",
        ),
        "leftover_skipped" => copy::pick(locale, "Skipped the directory", "已跳过该目录"),
        "leftover_failed" => copy::pick(
            locale,
            "The directory could not be moved",
            "目录未能移入回收站",
        ),
        "stale_candidate" | "candidate_changed" | "digest_mismatch" => copy::pick(
            locale,
            "The directory changed. Review it again.",
            "目录已变化，请重新审阅。",
        ),
        "uninstall_not_succeeded" => copy::pick(
            locale,
            "The software history has no matching successful uninstall",
            "软件历史中未找到对应的成功卸载记录",
        ),
        "protection_unavailable" => copy::pick(
            locale,
            "Directory protection could not be verified",
            "无法验证目录保护状态",
        ),
        "still_present" => copy::pick(locale, "The app is still installed", "应用仍已安装"),
        "msi_context_unsupported" => copy::pick(
            locale,
            "Only a unique, unmanaged installation for this user can be removed here",
            "此处只能卸载当前用户唯一的非托管安装",
        ),
        "msi_authority_unavailable" => copy::pick(
            locale,
            "Windows could not verify this installation and its restrictions",
            "Windows 无法验证这项安装及其限制",
        ),
        "conflicting_identity" => copy::pick(
            locale,
            "Conflicting installation identities",
            "安装身份存在冲突",
        ),
        "arp_settings_only" => copy::pick(
            locale,
            "Manage this registry entry in Windows Installed apps",
            "请在 Windows 已安装的应用中管理此注册表条目",
        ),
        "software_not_eligible" => copy::pick(
            locale,
            "This app cannot be uninstalled here",
            "此处无法卸载该应用",
        ),
        "software_revalidation_unavailable" => copy::pick(
            locale,
            "Windows could not verify the selected app",
            "Windows 无法验证所选应用",
        ),
        "terminated" => copy::pick(locale, "Process termination requested", "已请求结束进程"),
        "active" => copy::pick(locale, "Keep awake active", "正在保持唤醒"),
        "stopped" => copy::pick(locale, "Keep awake stopped", "已停止保持唤醒"),
        "expired" => copy::pick(locale, "Keep awake lease expired", "保持唤醒租期已结束"),
        "already_in_state" => copy::pick(
            locale,
            "Already in this state; no change",
            "已处于此状态，未更改",
        ),
        "restart_required" => copy::pick(locale, "Restart required", "需要重启"),
        "failed" | "verification_failed" | "registry_write_failed" => copy::pick(
            locale,
            "Failed; review the current state",
            "执行失败，请检查当前状态",
        ),
        "stale_confirmation"
        | "startup_state_changed"
        | "startup_entry_changed"
        | "version_changed"
        | "invalid_identity"
        | "provider_changed" => copy::pick(
            locale,
            "The host changed. Preview again.",
            "主机状态已变化，请重新预览。",
        ),
        "canceled_before_start" => copy::pick(locale, "Canceled before dispatch", "已在派发前取消"),
        "requires_administrator" => copy::pick(
            locale,
            "Administrator permission required; unavailable here",
            "需要管理员权限，此处不可用",
        ),
        "protected" => copy::pick(locale, "Protected; no change", "受保护，未更改"),
        "identity_unavailable" => copy::pick(
            locale,
            "The process could not be inspected or ended without elevation",
            "无法查询或结束此进程，不会请求提权",
        ),
        "fixture_read_only" => copy::pick(
            locale,
            "Display fixture; execution is unavailable",
            "显示夹具不可执行主机动作",
        ),
        "unbacked_session" => copy::pick(
            locale,
            "Host actions are unavailable in this session",
            "此会话不可执行主机动作",
        ),
        "probe_failed" | "network_unavailable" | "package_not_found" | "unsupported_source" => {
            copy::pick(
                locale,
                "Update provider unavailable; refresh and review its source",
                "更新工具或来源不可用，请刷新并检查来源",
            )
        }
        "provider_missing" => copy::pick(
            locale,
            "Microsoft App Installer is unavailable",
            "Microsoft 应用安装程序不可用",
        ),
        "unsupported_provider" => copy::pick(
            locale,
            "The update tool version, capabilities, source identity or file layout is unsupported",
            "更新工具版本、能力、来源身份或文件布局不受支持",
        ),
        "source_agreement_pending" => copy::pick(
            locale,
            "The source requires an agreement; no update started",
            "来源要求协议确认，未开始更新",
        ),
        "audit_unavailable" => copy::pick(
            locale,
            "History could not be saved; no action started",
            "无法保存历史，动作未开始",
        ),
        "keep_awake_acquire_failed" => copy::pick(
            locale,
            "The OS did not acquire the keep-awake request",
            "操作系统未获取保持唤醒请求",
        ),
        "keep_awake_clear_failed" => copy::pick(
            locale,
            "The OS could not clear the request; state is unknown",
            "操作系统未能清除请求，状态未知",
        ),
        "unknown_after_dispatch" => copy::pick(
            locale,
            "Result unknown after dispatch; no automatic retry",
            "派发后结果未知，不会自动重试",
        ),
        _ => copy::pick(
            locale,
            "Unavailable or unknown; refresh before retrying",
            "不可用或结果未知，重试前请刷新",
        ),
    };
    format!("{label} ({code})")
}

#[cfg(test)]
mod tests {
    use super::*;
    use devsweep_core::status::{FakeClock, RecordingBackend};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    #[derive(Default)]
    struct FakeExecutor {
        calls: AtomicUsize,
        stale: AtomicBool,
        cancel_after_first: Option<Arc<FlagCancelObserver>>,
    }

    fn update(id: &str) -> Intent {
        Intent::Update {
            row: SoftwareUpdateRowV1 {
                id: id.into(),
                name: id.into(),
                installed_version: "1".into(),
                available_version: "2".into(),
                source: "winget".into(),
                name_truncated: false,
                matched_software_ids: Vec::new(),
            },
        }
    }

    impl ActionExecutor for FakeExecutor {
        fn prepare(&self, intent: Intent) -> Result<Prepared, String> {
            Ok(Prepared {
                intent,
                plan: Plan::Update(SoftwareUpdatePlanV1 {
                    version: 1,
                    package_id: "fixture".into(),
                    source: "winget".into(),
                    installed_version: "1".into(),
                    available_version: "2".into(),
                    provider_fingerprint: format!("sha256:{}", "a".repeat(64)),
                }),
                digest: format!("sha256:{}", "b".repeat(64)),
            })
        }
        fn execute(&self, prepared: &Prepared, confirmed: bool) -> Result<ActionResult, String> {
            if !confirmed {
                return Err("confirmation_required".into());
            }
            if self.stale.load(Ordering::SeqCst) {
                return Err("stale_confirmation".into());
            }
            self.calls.fetch_add(1, Ordering::SeqCst);
            if let Some(cancel) = &self.cancel_after_first {
                cancel.request_cancel();
            }
            Ok(ActionResult {
                intent: prepared.intent.clone(),
                code: if prepared.intent.id() == "bad" {
                    "failed"
                } else {
                    "updated"
                }
                .into(),
                reverse: None,
                uninstall: None,
            })
        }
    }

    #[test]
    fn shared_review_requires_live_digest_and_confirmation_and_rejects_old_delivery() {
        let fake = FakeExecutor::default();
        let mut review = HostReview::default();
        assert!(review.take_confirmed(true).is_none());
        let generation = review.begin(vec![update("a")]);
        let prepared = fake.prepare(update("a")).expect("prepare");
        assert!(review.apply_preview(generation, Ok(vec![prepared.clone()])));
        assert!(review.take_confirmed(false).is_none());
        assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
        review.invalidate();
        assert!(review.prepared.is_empty());
        assert!(!review.apply_preview(generation, Ok(vec![prepared.clone()])));
        assert!(review.take_confirmed(true).is_none());
        let newer = review.begin(vec![update("bad")]);
        assert!(review.apply_preview(
            newer,
            Ok(vec![fake.prepare(update("bad")).expect("prepare")])
        ));
        assert!(review.take_confirmed(true).is_some());
        assert!(!review.apply_results(generation, Vec::new()));
        assert_eq!(review.phase, Phase::Running);
        fake.stale.store(true, Ordering::SeqCst);
        assert_eq!(
            fake.execute(&prepared, true).expect_err("stale"),
            "stale_confirmation"
        );
        assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
        review.invalidate();
        let invalid = review.begin(vec![update("a")]);
        let mut prepared = prepared;
        prepared.digest.clear();
        review.apply_preview(invalid, Ok(vec![prepared]));
        assert_eq!(review.phase, Phase::Finished);
        assert!(review.take_confirmed(true).is_none());
    }

    #[test]
    fn per_item_failure_and_cancel_never_dispatch_remaining_items() {
        let fake = Arc::new(FakeExecutor::default());
        let cancel = Arc::new(FlagCancelObserver::new());
        let prepared = run_previews(fake.as_ref(), vec![update("good"), update("bad")], &cancel)
            .expect("preview");
        let results = run_actions(fake.clone(), prepared, &cancel);
        assert_eq!(
            results
                .iter()
                .map(|item| item.code.as_str())
                .collect::<Vec<_>>(),
            ["updated", "failed"]
        );
        let fake = Arc::new(FakeExecutor {
            cancel_after_first: Some(cancel.clone()),
            ..Default::default()
        });
        let prepared = run_previews(fake.as_ref(), vec![update("good"), update("bad")], &cancel)
            .expect("preview");
        let results = run_actions(fake.clone(), prepared, &cancel);
        assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
        assert_eq!(results[1].code, "canceled_before_start");
    }

    #[test]
    fn keep_awake_fake_service_confirms_reuses_endpoint_and_releases_on_drop() {
        let clock = Arc::new(FakeClock::new());
        let (backend, log) = RecordingBackend::pair();
        let executor = CoreExecutor::with_awake(KeepAwakeService::new(
            clock.clone(),
            Box::new(backend),
            true,
        ));
        let start = executor
            .prepare(Intent::KeepAwake { enabled: true })
            .expect("start preview");
        assert!(executor.execute(&start, false).is_err());
        assert!(log.lock().expect("log").is_empty());
        // A failed first confirmation consumes no lease. Preview anew.
        let start = executor
            .prepare(Intent::KeepAwake { enabled: true })
            .expect("new start preview");
        assert_eq!(
            executor.execute(&start, true).expect("start").code,
            "active"
        );
        let active = executor.query_awake().expect("query").expect("session");
        let stop = executor
            .prepare(Intent::KeepAwake { enabled: false })
            .expect("stop preview");
        assert_eq!(executor.execute(&stop, true).expect("stop").code, "stopped");
        let stopped = executor.query_awake().expect("query").expect("session");
        assert_eq!(active.endpoint, stopped.endpoint);
        assert_eq!(active.token, stopped.token);
        let restart = executor
            .prepare(Intent::KeepAwake { enabled: true })
            .expect("restart preview");
        executor.execute(&restart, true).expect("restart");
        let stale = executor
            .prepare(Intent::KeepAwake { enabled: false })
            .expect("stop preview");
        clock.advance(Duration::from_secs(3_601));
        assert!(executor.execute(&stale, true).is_err());
        drop(executor);
        let log = log.lock().expect("log");
        assert_eq!(
            log.iter()
                .filter(|call| matches!(call, devsweep_core::status::BackendCall::Acquire(_)))
                .count(),
            2
        );
        assert_eq!(
            log.iter()
                .filter(|call| matches!(call, devsweep_core::status::BackendCall::Release(_)))
                .count(),
            2
        );
    }

    #[test]
    fn cli_stop_changes_core_generation_and_refuses_the_old_gui_review() {
        use devsweep_core::status::{BackendCall, KeepAwakeVerb, query_keep_awake};
        let (backend, log) = RecordingBackend::pair();
        let executor = CoreExecutor::with_awake(KeepAwakeService::new(
            Arc::new(FakeClock::new()),
            Box::new(backend),
            true,
        ));
        let start = executor
            .prepare(Intent::KeepAwake { enabled: true })
            .expect("start preview");
        executor.execute(&start, true).expect("fake acquire");
        let stop = executor
            .prepare(Intent::KeepAwake { enabled: false })
            .expect("GUI stop preview");
        let connection = executor.query_awake().expect("query").expect("session");
        let status = query_keep_awake(
            &connection.endpoint,
            KeepAwakeVerb::Stop,
            &connection.token,
            &connection.owner,
        )
        .expect("CLI stops the fake session");
        assert_eq!(status.state, "stopped");
        assert_eq!(
            executor.execute(&stop, true).expect_err("old GUI review"),
            "stale_confirmation"
        );
        let log = log.lock().expect("backend calls");
        assert_eq!(
            log.iter()
                .filter(|call| matches!(call, BackendCall::Release(_)))
                .count(),
            1,
            "stale GUI confirmation must not release again"
        );
    }

    #[test]
    fn keep_awake_acquire_and_clear_failures_never_claim_active_or_stopped() {
        struct FailingBackend {
            fail_acquire: bool,
        }
        impl devsweep_core::status::AwakeBackend for FailingBackend {
            fn acquire(&mut self, _: &str) -> Result<(), devsweep_core::status::KeepAwakeError> {
                if self.fail_acquire {
                    Err(devsweep_core::status::KeepAwakeError::BackendFailed {
                        operation: "acquire",
                        message: "fixture".into(),
                    })
                } else {
                    Ok(())
                }
            }
            fn release(&mut self, _: &str) -> Result<(), devsweep_core::status::KeepAwakeError> {
                Err(devsweep_core::status::KeepAwakeError::BackendFailed {
                    operation: "clear",
                    message: "fixture".into(),
                })
            }
        }
        for fail_acquire in [true, false] {
            let service = KeepAwakeService::new(
                Arc::new(FakeClock::new()),
                Box::new(FailingBackend { fail_acquire }),
                true,
            );
            let executor = CoreExecutor::with_awake(service);
            let plan = executor
                .prepare(Intent::KeepAwake { enabled: true })
                .expect("preview");
            let result = executor.execute(&plan, true);
            if fail_acquire {
                assert_eq!(
                    result.expect_err("acquire fails"),
                    "keep_awake_acquire_failed"
                );
                assert!(executor.query_awake().expect("no lease").is_none());
                assert!(
                    executor
                        .prepare(Intent::KeepAwake { enabled: true })
                        .is_err(),
                    "fake session cannot fall back to OS backend"
                );
            } else {
                assert_eq!(result.expect("start").code, "active");
                let plan = executor
                    .prepare(Intent::KeepAwake { enabled: false })
                    .expect("stop preview");
                assert_eq!(
                    executor.execute(&plan, true).expect_err("clear fails"),
                    "keep_awake_clear_failed"
                );
                assert!(
                    executor.query_awake().is_err(),
                    "clear failure is unknown, not stopped"
                );
            }
        }
    }

    #[test]
    fn startup_changed_hive_identity_refuses_saved_digest_without_writing() {
        use software::{
            SoftwareStartupLocation, StartupFixtureClass, StartupFixtureEntryV1,
            StartupFixtureHiveV1,
        };
        let path =
            std::env::temp_dir().join(format!("devsweep-host-stale-{}.json", std::process::id()));
        let mut hive = StartupFixtureHiveV1 {
            version: 1,
            entries: vec![StartupFixtureEntryV1 {
                name: "Fixture".into(),
                location: SoftwareStartupLocation::CurrentUserRun,
                class: StartupFixtureClass::Ordinary,
                run_value: b"before".to_vec(),
                approval: None,
            }],
        };
        std::fs::write(&path, serde_json::to_vec(&hive).expect("hive")).expect("write");
        let id =
            software::startup_fixture_entry_id(SoftwareStartupLocation::CurrentUserRun, "Fixture");
        let executor = CoreExecutor::default();
        let plan = executor
            .prepare(Intent::Startup {
                source: StartupSource::Hive(path.clone()),
                id,
                name: "Fixture".into(),
                enabled: false,
            })
            .expect("preview");
        let before = std::fs::read(&path).expect("read");
        assert!(executor.execute(&plan, false).is_err());
        assert_eq!(std::fs::read(&path).expect("read"), before);
        hive.entries[0].run_value = b"changed".to_vec();
        let changed = serde_json::to_vec(&hive).expect("changed");
        std::fs::write(&path, &changed).expect("change fixture identity");
        assert_eq!(
            executor.execute(&plan, true).expect_err("stale identity"),
            "startup_entry_changed"
        );
        assert_eq!(std::fs::read(&path).expect("read"), changed);
        std::fs::remove_file(path).expect("remove owned fixture");
    }

    #[test]
    fn hive_only_executor_has_no_native_owner_and_refuses_other_intents() {
        let executor = CoreExecutor::for_startup_hive();
        assert!(executor.process.is_none());
        assert!(!executor.awake.lock().expect("owner").production_allowed);
        for intent in [
            Intent::Process {
                pid: 42,
                name: "fixture.exe".into(),
            },
            Intent::KeepAwake { enabled: true },
            update("fixture"),
            Intent::Startup {
                source: StartupSource::Host,
                id: "fixture".into(),
                name: "fixture".into(),
                enabled: true,
            },
        ] {
            assert_eq!(
                executor
                    .prepare(intent.clone())
                    .expect_err("non-hive prepare"),
                "unbacked_session"
            );
            let prepared = FakeExecutor::default()
                .prepare(intent)
                .expect("fixture plan");
            assert_eq!(
                executor
                    .execute(&prepared, true)
                    .expect_err("non-hive execute"),
                "unbacked_session"
            );
        }
    }

    #[test]
    fn shared_review_has_bilingual_action_digest_and_result_projection() {
        let fake = FakeExecutor::default();
        let mut review = HostReview::default();
        let generation = review.begin(vec![update("App")]);
        review.apply_preview(
            generation,
            Ok(vec![fake.prepare(update("App")).expect("preview")]),
        );
        assert!(
            review
                .lines(Locale::En)
                .join("\n")
                .contains("Review these changes")
        );
        assert!(
            review
                .lines(Locale::ZhCn)
                .join("\n")
                .contains("审阅以下更改")
        );
        assert!(review.lines(Locale::En).join("\n").contains("sha256:"));
        review.take_confirmed(true);
        review.apply_results(
            review.generation,
            vec![ActionResult {
                intent: update("App"),
                code: "failed".into(),
                reverse: None,
                uninstall: None,
            }],
        );
        assert!(review.lines(Locale::En).join("\n").contains("Failed"));
        assert!(review.lines(Locale::ZhCn).join("\n").contains("执行失败"));
        let view = crate::source_lf(include_str!("view/host_review.rs"));
        assert!(view.contains("host-action-confirm") && view.contains("on_a11y_action"));
        let software = include_str!("view/page_workbench.rs");
        let status = include_str!("view/page_status.rs");
        assert!(software.contains("self.host_review_bar(t, cx)"));
        assert!(status.contains("self.host_review_bar(t, cx)"));
    }
}
