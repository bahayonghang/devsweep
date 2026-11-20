use super::*;
use crate::copy;
use devsweep_core::optimize::{
    MaintenanceActionClass, MaintenanceBatchPreview, MaintenanceBatchReport,
    MaintenanceExecutionError, MaintenanceExecutor, MaintenancePlanV2, OptimizePlanError,
    catalogue_entry,
};

#[derive(Default)]
pub(crate) struct BatchState {
    pub selected: Vec<String>,
    pub guidance_open: bool,
    pub review: Option<(MaintenancePlanV2, MaintenanceBatchPreview)>,
    pub report: Option<MaintenanceBatchReport>,
    pub error: Option<BatchError>,
    generation: u64,
    pending: Option<u64>,
    running: Option<(Arc<FlagCancelObserver>, Arc<AtomicUsize>)>,
    executor: Option<Arc<MaintenanceExecutor>>,
    pub fixture: bool,
}

#[derive(Clone, Copy)]
pub(crate) enum BatchError {
    Stale,
    Audit,
    Worker,
    Unknown,
    Invalid,
}
impl BatchError {
    pub(crate) fn label(self, locale: Locale) -> &'static str {
        match self {
            Self::Stale => copy::pick(
                locale,
                "The live plan changed. Review it again.",
                "实时计划已变更，请重新审阅。",
            ),
            Self::Audit => copy::pick(
                locale,
                "Audit could not be saved or read. Check history before retrying.",
                "无法写入或读取审计。重试前请核对历史。",
            ),
            Self::Worker => copy::pick(
                locale,
                "The review worker could not start.",
                "审阅工作线程无法启动。",
            ),
            Self::Unknown => copy::pick(
                locale,
                "Execution was interrupted. Check history for recorded outcomes.",
                "执行中断，请在历史中核对已记录的结果。",
            ),
            Self::Invalid => copy::pick(
                locale,
                "The maintenance plan is unavailable.",
                "维护计划不可用。",
            ),
        }
    }
}

impl BatchState {
    pub(super) fn new(backed: bool) -> Self {
        if !backed {
            return Self::default();
        }
        #[cfg(feature = "optimize-fixtures")]
        if std::env::var_os("DEVSWEEP_OPTIMIZE_FIXTURE").is_some() {
            let executor = devsweep_core::optimize::optimize_audit_v1_path()
                .ok()
                .map(|path| Arc::new(MaintenanceExecutor::isolated_batch_fixture(&path)));
            return Self {
                executor,
                fixture: true,
                ..Self::default()
            };
        }
        Self {
            executor: Some(Arc::new(MaintenanceExecutor::default())),
            ..Self::default()
        }
    }
    pub(crate) fn running(&self) -> bool {
        self.running.is_some()
    }
    pub(crate) fn reviewed(&self) -> bool {
        self.review.is_some()
    }
    pub(crate) fn previewing(&self) -> bool {
        self.pending.is_some() && !self.running()
    }
    pub(crate) fn current(&self) -> Option<usize> {
        self.running
            .as_ref()
            .map(|(_, current)| current.load(Ordering::Acquire))
    }
    fn reset_review(&mut self) {
        if self.running() {
            return;
        }
        self.generation = self.generation.wrapping_add(1);
        self.review = None;
        self.pending = None;
        self.report = None;
        self.error = None;
    }
    pub(crate) fn cancel(&self) {
        if let Some((flag, _)) = &self.running {
            flag.request_cancel()
        }
    }
}

pub(crate) enum BatchJob {
    Preview {
        generation: u64,
        plan: MaintenancePlanV2,
        executor: Arc<MaintenanceExecutor>,
    },
    Execute {
        generation: u64,
        plan: MaintenancePlanV2,
        digest: String,
        executor: Arc<MaintenanceExecutor>,
        cancel: Arc<FlagCancelObserver>,
        progress: Arc<AtomicUsize>,
    },
}
pub(crate) enum BatchDelivery {
    Review(MaintenancePlanV2, MaintenanceBatchPreview),
    Result(MaintenanceBatchReport),
}
impl BatchJob {
    pub(super) fn failure(&self, panic: bool) -> JobResult {
        let (generation, error) = match self {
            Self::Preview { generation, .. } => (*generation, BatchError::Worker),
            Self::Execute { generation, .. } => (
                *generation,
                if panic {
                    BatchError::Unknown
                } else {
                    BatchError::Worker
                },
            ),
        };
        JobResult::OptimizeBatch {
            generation,
            result: Err(error),
        }
    }
    pub(super) fn run(self) -> JobResult {
        let (generation, result) = match self {
            Self::Preview {
                generation,
                plan,
                executor,
            } => {
                let result = executor
                    .preview_batch(&plan)
                    .map(|preview| BatchDelivery::Review(plan, preview))
                    .map_err(|_| BatchError::Invalid);
                (generation, result)
            }
            Self::Execute {
                generation,
                plan,
                digest,
                executor,
                cancel,
                progress,
            } => {
                let report = executor.execute_batch(
                    &plan,
                    &digest,
                    true,
                    Some(cancel.as_ref()),
                    |index, _| progress.store(index, Ordering::Release),
                );
                (
                    generation,
                    report
                        .map(BatchDelivery::Result)
                        .map_err(|error| match error {
                            MaintenanceExecutionError::Plan(OptimizePlanError::DigestMismatch) => {
                                BatchError::Stale
                            }
                            MaintenanceExecutionError::Audit(_) => BatchError::Audit,
                            _ => BatchError::Invalid,
                        }),
                )
            }
        };
        JobResult::OptimizeBatch { generation, result }
    }
}

impl Session {
    pub(crate) fn batch(&self) -> &BatchState {
        &self.optimize_batch
    }
    fn batch_surface(&self) -> bool {
        self.shell.mode() == Mode::Optimize
            && self.shell.surface() == Surface::Mode
            && !self.shell.menu_open()
            && !self.shell.screen_covered()
            && self.clean.native_fixture().is_none()
    }
    pub(crate) fn toggle_batch_item(&mut self, id: &str) {
        if !self.batch_surface()
            || self.optimize_batch.running()
            || !catalogue_entry(id)
                .is_some_and(|e| e.action_class == MaintenanceActionClass::Execute)
        {
            return;
        }
        let state = &mut self.optimize_batch;
        state.reset_review();
        if let Some(index) = state.selected.iter().position(|other| other == id) {
            state.selected.remove(index);
        } else {
            state.selected.push(id.to_string());
        }
    }
    pub(crate) fn toggle_batch_guidance(&mut self) {
        self.optimize_batch.guidance_open = !self.optimize_batch.guidance_open;
        self.sync_optimize_rows();
    }
    pub(crate) fn review_batch(&mut self) {
        if !self.batch_surface() || self.host_job.is_some() {
            return;
        }
        let state = &mut self.optimize_batch;
        if state.running() || state.previewing() {
            return;
        }
        let (Ok(plan), Some(executor)) = (
            MaintenancePlanV2::new(&state.selected),
            state.executor.clone(),
        ) else {
            return;
        };
        state.reset_review();
        state.pending = Some(state.generation);
        self.jobs
            .push(BackgroundJob::OptimizeBatch(BatchJob::Preview {
                generation: state.generation,
                plan,
                executor,
            }));
    }
    pub(crate) fn confirm_batch(&mut self, generation: u64) {
        if !self.batch_surface() || self.host_job.is_some() {
            return;
        }
        let state = &mut self.optimize_batch;
        if state.generation != generation || state.pending.is_some() {
            return;
        }
        let Some(executor) = state.executor.clone() else {
            return;
        };
        let Some((plan, preview)) = state.review.take() else {
            return;
        };
        let cancel = Arc::new(FlagCancelObserver::new());
        let progress = Arc::new(AtomicUsize::new(0));
        state.pending = Some(generation);
        state.running = Some((cancel.clone(), progress.clone()));
        self.jobs
            .push(BackgroundJob::OptimizeBatch(BatchJob::Execute {
                generation,
                plan,
                digest: preview.digest,
                executor,
                cancel,
                progress,
            }));
    }
    pub(crate) fn batch_generation(&self) -> u64 {
        self.optimize_batch.generation
    }
    pub(crate) fn cancel_batch(&self, generation: u64) {
        if self.batch_surface() && self.batch_generation() == generation {
            self.optimize_batch.cancel();
        }
    }
    pub(super) fn apply_batch(
        &mut self,
        generation: u64,
        result: Result<BatchDelivery, BatchError>,
    ) -> bool {
        let state = &mut self.optimize_batch;
        if state.pending != Some(generation) {
            return false;
        }
        let finished_execution = state.running();
        state.pending = None;
        state.running = None;
        match result {
            Ok(BatchDelivery::Review(plan, preview)) => state.review = Some((plan, preview)),
            Ok(BatchDelivery::Result(report)) => state.report = Some(report),
            Err(error) => state.error = Some(error),
        }
        if finished_execution && self.shell.surface() == Surface::History {
            self.refresh_history_from_store();
        }
        true
    }
    pub(super) fn leave_batch(&mut self) {
        if !self.optimize_batch.running()
            && (self.optimize_batch.previewing() || self.optimize_batch.reviewed())
        {
            self.optimize_batch.reset_review()
        }
    }
}

#[cfg(test)]
mod tests;
