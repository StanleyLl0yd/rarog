use std::fmt;

use rarog_scheduler::{
    EventLoopScheduler, MicrotaskId, SchedulerError, SchedulerLimits, SchedulerStep, TaskId,
    TaskSource, WorkId,
};
use rarog_script::{
    EvaluationOutcome, RealmId, ScriptError, ScriptRealmLimits, ScriptRuntime, ScriptSource,
};

use crate::identity::{WorkerError, WorkerId, WorkerLifecycleState, WorkerRegistry};
use crate::message::{
    WorkerMessage, WorkerMessageDiscard, WorkerMessageError, WorkerMessageId, WorkerMessageMailbox,
};

const WORKER_MESSAGE_TASK_SOURCE: TaskSource = TaskSource::Other(0x574d_5347);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkerExecutionError {
    Worker(WorkerError),
    WorkerNotRunning {
        worker: WorkerId,
        state: WorkerLifecycleState,
    },
    Scheduler(SchedulerError),
    Script(ScriptError),
    Message(WorkerMessageError),
    InvalidMessageDelivery,
}

impl fmt::Display for WorkerExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Worker(error) => error.fmt(formatter),
            Self::WorkerNotRunning { worker, state } => {
                write!(formatter, "worker {worker} is not running ({state:?})")
            }
            Self::Scheduler(error) => error.fmt(formatter),
            Self::Script(error) => error.fmt(formatter),
            Self::Message(error) => error.fmt(formatter),
            Self::InvalidMessageDelivery => formatter
                .write_str("worker message delivery token is not active for this execution"),
        }
    }
}

impl std::error::Error for WorkerExecutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Worker(error) => Some(error),
            Self::WorkerNotRunning { .. } => None,
            Self::Scheduler(error) => Some(error),
            Self::Script(error) => Some(error),
            Self::Message(error) => Some(error),
            Self::InvalidMessageDelivery => None,
        }
    }
}

impl From<WorkerError> for WorkerExecutionError {
    fn from(error: WorkerError) -> Self {
        Self::Worker(error)
    }
}

impl From<SchedulerError> for WorkerExecutionError {
    fn from(error: SchedulerError) -> Self {
        Self::Scheduler(error)
    }
}

impl From<ScriptError> for WorkerExecutionError {
    fn from(error: ScriptError) -> Self {
        Self::Script(error)
    }
}

impl From<WorkerMessageError> for WorkerExecutionError {
    fn from(error: WorkerMessageError) -> Self {
        Self::Message(error)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OwnedScriptSource {
    text: String,
}

impl OwnedScriptSource {
    fn try_new(text: &str, max_source_bytes: usize) -> Result<Self, WorkerExecutionError> {
        ScriptSource::new(text).ensure_byte_limit(max_source_bytes)?;
        Ok(Self {
            text: text.to_owned(),
        })
    }

    fn source(&self) -> ScriptSource<'_> {
        ScriptSource::new(&self.text)
    }
}

#[derive(Debug, PartialEq, Eq)]
enum WorkerTask {
    Script(OwnedScriptSource),
    Message(WorkerMessageId),
}

#[derive(Debug, PartialEq, Eq)]
pub struct WorkerMessageDelivery {
    worker: WorkerId,
    task: TaskId,
    message: WorkerMessageId,
}

impl WorkerMessageDelivery {
    pub fn worker(&self) -> WorkerId {
        self.worker
    }

    pub fn task(&self) -> TaskId {
        self.task
    }

    pub fn message(&self) -> WorkerMessageId {
        self.message
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum WorkerExecutionStep {
    Task {
        id: TaskId,
        source: TaskSource,
        outcome: EvaluationOutcome,
    },
    Microtask {
        id: MicrotaskId,
        outcome: EvaluationOutcome,
    },
    Message {
        delivery: WorkerMessageDelivery,
    },
    MicrotaskCheckpointComplete,
}

pub struct WorkerExecution<'runtime, R: ScriptRuntime + ?Sized> {
    worker: WorkerId,
    runtime: &'runtime mut R,
    realm: RealmId,
    realm_live: bool,
    max_source_bytes: usize,
    scheduler: EventLoopScheduler<WorkerTask, OwnedScriptSource>,
}

impl<'runtime, R: ScriptRuntime + ?Sized> WorkerExecution<'runtime, R> {
    pub fn new<O: Eq>(
        worker: WorkerId,
        registry: &WorkerRegistry<O>,
        runtime: &'runtime mut R,
        realm_limits: ScriptRealmLimits,
        scheduler_limits: SchedulerLimits,
    ) -> Result<Self, WorkerExecutionError> {
        ensure_running(registry, worker)?;
        let scheduler = EventLoopScheduler::new(scheduler_limits)?;
        let realm = runtime.create_realm(realm_limits)?.id();
        Ok(Self {
            worker,
            runtime,
            realm,
            realm_live: true,
            max_source_bytes: realm_limits.max_source_bytes(),
            scheduler,
        })
    }

    pub fn worker(&self) -> WorkerId {
        self.worker
    }

    pub fn realm(&self) -> RealmId {
        self.realm
    }

    pub fn pending_task_count(&self) -> usize {
        self.scheduler.pending_task_count()
    }

    pub fn pending_microtask_count(&self) -> usize {
        self.scheduler.pending_microtask_count()
    }

    pub fn queue_task<O: Eq>(
        &mut self,
        registry: &WorkerRegistry<O>,
        source: TaskSource,
        script: &str,
    ) -> Result<TaskId, WorkerExecutionError> {
        ensure_running(registry, self.worker)?;
        let payload = OwnedScriptSource::try_new(script, self.max_source_bytes)?;
        Ok(self
            .scheduler
            .queue_task(source, WorkerTask::Script(payload))?)
    }

    pub fn queue_microtask<O: Eq>(
        &mut self,
        registry: &WorkerRegistry<O>,
        script: &str,
    ) -> Result<MicrotaskId, WorkerExecutionError> {
        ensure_running(registry, self.worker)?;
        let payload = OwnedScriptSource::try_new(script, self.max_source_bytes)?;
        Ok(self.scheduler.queue_microtask(payload)?)
    }

    pub fn request_microtask_checkpoint<O: Eq>(
        &mut self,
        registry: &WorkerRegistry<O>,
    ) -> Result<(), WorkerExecutionError> {
        ensure_running(registry, self.worker)?;
        self.scheduler.request_microtask_checkpoint();
        Ok(())
    }

    pub fn next_step<O: Eq>(
        &mut self,
        registry: &WorkerRegistry<O>,
    ) -> Result<Option<WorkerExecutionStep>, WorkerExecutionError> {
        ensure_running(registry, self.worker)?;
        let Some(step) = self.scheduler.next_step()? else {
            return Ok(None);
        };
        match step {
            SchedulerStep::Task(task) => match task.payload {
                WorkerTask::Script(source) => {
                    let work = WorkId::Task(task.id);
                    let evaluation = self.runtime.evaluate(self.realm, source.source());
                    self.scheduler.complete(work)?;
                    let outcome = evaluation?;
                    Ok(Some(WorkerExecutionStep::Task {
                        id: task.id,
                        source: task.source,
                        outcome,
                    }))
                }
                WorkerTask::Message(message) => Ok(Some(WorkerExecutionStep::Message {
                    delivery: WorkerMessageDelivery {
                        worker: self.worker,
                        task: task.id,
                        message,
                    },
                })),
            },
            SchedulerStep::Microtask(microtask) => {
                let work = WorkId::Microtask(microtask.id);
                let evaluation = self
                    .runtime
                    .evaluate(self.realm, microtask.payload.source());
                self.scheduler.complete(work)?;
                let outcome = evaluation?;
                Ok(Some(WorkerExecutionStep::Microtask {
                    id: microtask.id,
                    outcome,
                }))
            }
            SchedulerStep::MicrotaskCheckpointComplete => {
                Ok(Some(WorkerExecutionStep::MicrotaskCheckpointComplete))
            }
        }
    }

    pub fn schedule_next_message<O: Eq>(
        &mut self,
        registry: &WorkerRegistry<O>,
        mailbox: &mut WorkerMessageMailbox,
    ) -> Result<Option<TaskId>, WorkerExecutionError> {
        ensure_running(registry, self.worker)?;
        let Some(message) = mailbox.next_for_worker_delivery(registry, self.worker)? else {
            return Ok(None);
        };
        let task = self
            .scheduler
            .queue_task(WORKER_MESSAGE_TASK_SOURCE, WorkerTask::Message(message))?;
        if let Err(error) = mailbox.mark_scheduled(self.worker, message, task) {
            if !self.scheduler.cancel_task(task) {
                return Err(WorkerExecutionError::InvalidMessageDelivery);
            }
            return Err(error.into());
        }
        Ok(Some(task))
    }

    pub fn message_for_delivery<'mailbox, O: Eq>(
        &self,
        registry: &WorkerRegistry<O>,
        mailbox: &'mailbox WorkerMessageMailbox,
        delivery: &WorkerMessageDelivery,
    ) -> Result<&'mailbox WorkerMessage, WorkerExecutionError> {
        ensure_running(registry, self.worker)?;
        self.validate_message_delivery(delivery)?;
        Ok(mailbox.message_for_delivery(registry, self.worker, delivery.message, delivery.task)?)
    }

    pub fn complete_message(
        &mut self,
        mailbox: &mut WorkerMessageMailbox,
        delivery: WorkerMessageDelivery,
    ) -> Result<(), WorkerExecutionError> {
        self.validate_message_delivery(&delivery)?;
        mailbox.complete_delivery(self.worker, delivery.message, delivery.task)?;
        self.scheduler.complete(WorkId::Task(delivery.task))?;
        Ok(())
    }

    pub fn shutdown_with_mailbox(
        mut self,
        mailbox: &mut WorkerMessageMailbox,
    ) -> Result<WorkerMessageDiscard, WorkerExecutionError> {
        let discarded = mailbox.discard_for_worker(self.worker);
        self.destroy_realm()?;
        Ok(discarded)
    }

    fn validate_message_delivery(
        &self,
        delivery: &WorkerMessageDelivery,
    ) -> Result<(), WorkerExecutionError> {
        if delivery.worker != self.worker
            || self.scheduler.active_work() != Some(WorkId::Task(delivery.task))
        {
            return Err(WorkerExecutionError::InvalidMessageDelivery);
        }
        Ok(())
    }

    pub fn shutdown(mut self) -> Result<(), WorkerExecutionError> {
        self.destroy_realm()
    }

    fn destroy_realm(&mut self) -> Result<(), WorkerExecutionError> {
        if !self.realm_live {
            return Ok(());
        }
        self.runtime.destroy_realm(self.realm)?;
        self.realm_live = false;
        Ok(())
    }
}

fn ensure_running<O: Eq>(
    registry: &WorkerRegistry<O>,
    worker: WorkerId,
) -> Result<(), WorkerExecutionError> {
    let state = registry.state(worker)?;
    if state != WorkerLifecycleState::Running {
        return Err(WorkerExecutionError::WorkerNotRunning { worker, state });
    }
    Ok(())
}

impl<R: ScriptRuntime + ?Sized> Drop for WorkerExecution<'_, R> {
    fn drop(&mut self) {
        let _ = self.destroy_realm();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use rarog_script::{
        RealmIdAllocator, RootedValueId, RootedValueIdAllocator, ScriptCompletion,
        ScriptDiagnostic, ScriptDiagnosticLevel, ScriptErrorKind, ScriptException, ScriptRealm,
    };

    use super::*;
    use crate::identity::{WorkerErrorKind, WorkerLimits};
    use crate::message::{WorkerMessageLimits, WorkerMessageValue};

    struct FixtureRealm {
        limits: ScriptRealmLimits,
        roots: RootedValueIdAllocator,
    }

    struct FixtureRuntime {
        realm_ids: RealmIdAllocator,
        realms: BTreeMap<RealmId, FixtureRealm>,
        evaluations: Vec<String>,
        destroyed: Vec<RealmId>,
    }

    impl FixtureRuntime {
        fn new() -> Self {
            Self {
                realm_ids: RealmIdAllocator::new().unwrap(),
                realms: BTreeMap::new(),
                evaluations: Vec::new(),
                destroyed: Vec::new(),
            }
        }

        fn state_mut(&mut self, realm: RealmId) -> Result<&mut FixtureRealm, ScriptError> {
            self.realms.get_mut(&realm).ok_or_else(|| {
                ScriptError::new(ScriptErrorKind::InvalidRealm, "fixture realm is not live")
            })
        }
    }

    impl ScriptRuntime for FixtureRuntime {
        fn create_realm(&mut self, limits: ScriptRealmLimits) -> Result<ScriptRealm, ScriptError> {
            let realm = self.realm_ids.allocate()?;
            let roots = RootedValueIdAllocator::new(realm)?;
            self.realms.insert(realm, FixtureRealm { limits, roots });
            Ok(ScriptRealm::new(realm))
        }

        fn evaluate(
            &mut self,
            realm: RealmId,
            source: ScriptSource<'_>,
        ) -> Result<EvaluationOutcome, ScriptError> {
            let limit = self
                .realms
                .get(&realm)
                .ok_or_else(|| {
                    ScriptError::new(ScriptErrorKind::InvalidRealm, "fixture realm is not live")
                })?
                .limits
                .max_source_bytes();
            source.ensure_byte_limit(limit)?;
            self.evaluations.push(source.text().to_owned());
            if source.text() == "backend error" {
                return Err(ScriptError::new(
                    ScriptErrorKind::Backend,
                    "fixture backend error",
                ));
            }
            let value = self.state_mut(realm)?.roots.allocate()?;
            let completion = if source.text() == "throw fixture" {
                ScriptCompletion::Throw(ScriptException {
                    value,
                    message: Some(String::from("fixture throw")),
                    stack: None,
                })
            } else {
                ScriptCompletion::Normal(value)
            };
            Ok(EvaluationOutcome {
                completion,
                diagnostics: vec![ScriptDiagnostic::new(
                    ScriptDiagnosticLevel::Warning,
                    source.text(),
                )],
            })
        }

        fn duplicate_root(&mut self, value: RootedValueId) -> Result<RootedValueId, ScriptError> {
            self.state_mut(value.realm())?.roots.allocate()
        }

        fn release_root(&mut self, value: RootedValueId) -> Result<(), ScriptError> {
            self.realms.get(&value.realm()).map(|_| ()).ok_or_else(|| {
                ScriptError::new(ScriptErrorKind::InvalidRealm, "fixture realm is not live")
            })
        }

        fn destroy_realm(&mut self, realm: RealmId) -> Result<(), ScriptError> {
            if self.realms.remove(&realm).is_none() {
                return Err(ScriptError::new(
                    ScriptErrorKind::InvalidRealm,
                    "fixture realm is not live",
                ));
            }
            self.destroyed.push(realm);
            Ok(())
        }
    }

    fn running_worker() -> (WorkerRegistry<u8>, WorkerId) {
        let mut registry = WorkerRegistry::try_new(WorkerLimits::default()).unwrap();
        let worker = registry.create_root(1).unwrap();
        registry.mark_running(worker).unwrap();
        (registry, worker)
    }

    fn realm_limits(max_source_bytes: usize) -> ScriptRealmLimits {
        ScriptRealmLimits::try_new(max_source_bytes, 16).unwrap()
    }

    fn scheduler_limits(max_tasks: usize, max_microtasks: usize) -> SchedulerLimits {
        SchedulerLimits::try_new(max_tasks, max_microtasks).unwrap()
    }

    #[test]
    fn execution_is_bound_to_worker_and_owns_realm_lifetime() {
        let (registry, worker) = running_worker();
        let mut runtime = FixtureRuntime::new();
        let execution = WorkerExecution::new(
            worker,
            &registry,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(4, 4),
        )
        .unwrap();
        let realm = execution.realm();
        assert_eq!(execution.worker(), worker);
        drop(execution);
        assert!(runtime.realms.is_empty());
        assert_eq!(runtime.destroyed, vec![realm]);
    }

    #[test]
    fn queued_script_source_is_owned_after_enqueue() {
        let (registry, worker) = running_worker();
        let mut runtime = FixtureRuntime::new();
        let mut execution = WorkerExecution::new(
            worker,
            &registry,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(4, 4),
        )
        .unwrap();
        let mut source = String::from("original");
        execution
            .queue_task(&registry, TaskSource::Other(1), &source)
            .unwrap();
        source.clear();
        source.push_str("changed");

        let Some(WorkerExecutionStep::Task { outcome, .. }) =
            execution.next_step(&registry).unwrap()
        else {
            panic!("expected worker task");
        };
        assert_eq!(outcome.diagnostics[0].message, "original");
    }

    #[test]
    fn task_completion_runs_microtask_checkpoint_before_next_task() {
        let (registry, worker) = running_worker();
        let mut runtime = FixtureRuntime::new();
        let mut execution = WorkerExecution::new(
            worker,
            &registry,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(4, 4),
        )
        .unwrap();
        execution
            .queue_task(&registry, TaskSource::Other(1), "first task")
            .unwrap();
        execution.queue_microtask(&registry, "microtask").unwrap();
        execution
            .queue_task(&registry, TaskSource::Other(1), "second task")
            .unwrap();

        assert!(matches!(
            execution.next_step(&registry).unwrap(),
            Some(WorkerExecutionStep::Task { .. })
        ));
        assert!(matches!(
            execution.next_step(&registry).unwrap(),
            Some(WorkerExecutionStep::Microtask { .. })
        ));
        assert!(matches!(
            execution.next_step(&registry).unwrap(),
            Some(WorkerExecutionStep::MicrotaskCheckpointComplete)
        ));
        assert!(matches!(
            execution.next_step(&registry).unwrap(),
            Some(WorkerExecutionStep::Task { .. })
        ));
    }

    #[test]
    fn explicit_checkpoint_runs_microtasks_without_a_task() {
        let (registry, worker) = running_worker();
        let mut runtime = FixtureRuntime::new();
        let mut execution = WorkerExecution::new(
            worker,
            &registry,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(4, 4),
        )
        .unwrap();
        execution.queue_microtask(&registry, "microtask").unwrap();
        assert!(execution.next_step(&registry).unwrap().is_none());
        execution.request_microtask_checkpoint(&registry).unwrap();
        assert!(matches!(
            execution.next_step(&registry).unwrap(),
            Some(WorkerExecutionStep::Microtask { .. })
        ));
    }

    #[test]
    fn scheduler_queue_limits_remain_authoritative() {
        let (registry, worker) = running_worker();
        let mut runtime = FixtureRuntime::new();
        let mut execution = WorkerExecution::new(
            worker,
            &registry,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(1, 1),
        )
        .unwrap();
        execution
            .queue_task(&registry, TaskSource::Other(1), "first")
            .unwrap();
        assert_eq!(
            execution
                .queue_task(&registry, TaskSource::Other(1), "blocked")
                .unwrap_err(),
            WorkerExecutionError::Scheduler(SchedulerError::TaskQueueFull)
        );
        execution
            .queue_microtask(&registry, "first microtask")
            .unwrap();
        assert_eq!(
            execution.queue_microtask(&registry, "blocked").unwrap_err(),
            WorkerExecutionError::Scheduler(SchedulerError::MicrotaskQueueFull)
        );
    }

    #[test]
    fn source_limit_is_enforced_before_payload_is_queued() {
        let (registry, worker) = running_worker();
        let mut runtime = FixtureRuntime::new();
        let mut execution = WorkerExecution::new(
            worker,
            &registry,
            &mut runtime,
            realm_limits(4),
            scheduler_limits(4, 4),
        )
        .unwrap();
        let error = execution
            .queue_task(&registry, TaskSource::Other(1), "12345")
            .unwrap_err();
        let WorkerExecutionError::Script(error) = error else {
            panic!("expected script limit error");
        };
        assert_eq!(error.kind, ScriptErrorKind::ResourceLimit);
        assert_eq!(execution.pending_task_count(), 0);
    }

    #[test]
    fn backend_evaluation_error_does_not_wedge_scheduler() {
        let (registry, worker) = running_worker();
        let mut runtime = FixtureRuntime::new();
        let mut execution = WorkerExecution::new(
            worker,
            &registry,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(4, 4),
        )
        .unwrap();
        execution
            .queue_task(&registry, TaskSource::Other(1), "backend error")
            .unwrap();
        execution
            .queue_task(&registry, TaskSource::Other(1), "next")
            .unwrap();

        let error = execution.next_step(&registry).unwrap_err();
        let WorkerExecutionError::Script(error) = error else {
            panic!("expected script backend error");
        };
        assert_eq!(error.kind, ScriptErrorKind::Backend);
        assert!(matches!(
            execution.next_step(&registry).unwrap(),
            Some(WorkerExecutionStep::MicrotaskCheckpointComplete)
        ));
        assert!(matches!(
            execution.next_step(&registry).unwrap(),
            Some(WorkerExecutionStep::Task { .. })
        ));
    }

    #[test]
    fn script_throw_is_returned_as_a_completion() {
        let (registry, worker) = running_worker();
        let mut runtime = FixtureRuntime::new();
        let mut execution = WorkerExecution::new(
            worker,
            &registry,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(4, 4),
        )
        .unwrap();
        execution
            .queue_task(&registry, TaskSource::Other(1), "throw fixture")
            .unwrap();

        let Some(WorkerExecutionStep::Task { outcome, .. }) =
            execution.next_step(&registry).unwrap()
        else {
            panic!("expected worker task");
        };
        assert!(matches!(outcome.completion, ScriptCompletion::Throw(_)));
    }

    #[test]
    fn explicit_shutdown_destroys_the_exact_realm_once() {
        let (registry, worker) = running_worker();
        let mut runtime = FixtureRuntime::new();
        let execution = WorkerExecution::new(
            worker,
            &registry,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(4, 4),
        )
        .unwrap();
        let realm = execution.realm();
        execution.shutdown().unwrap();
        assert!(runtime.realms.is_empty());
        assert_eq!(runtime.destroyed, vec![realm]);
    }

    #[test]
    fn dropping_execution_discards_pending_work_without_evaluating_it() {
        let (registry, worker) = running_worker();
        let mut runtime = FixtureRuntime::new();
        let mut execution = WorkerExecution::new(
            worker,
            &registry,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(4, 4),
        )
        .unwrap();
        execution
            .queue_task(&registry, TaskSource::Other(1), "never evaluated")
            .unwrap();
        execution
            .queue_microtask(&registry, "also never evaluated")
            .unwrap();
        drop(execution);

        assert!(runtime.evaluations.is_empty());
        assert!(runtime.realms.is_empty());
        assert_eq!(runtime.destroyed.len(), 1);
    }

    #[test]
    fn execution_construction_requires_exact_running_registry_worker() {
        let mut registry = WorkerRegistry::<u8>::try_new(WorkerLimits::default()).unwrap();
        let worker = registry.create_root(1).unwrap();
        let mut runtime = FixtureRuntime::new();

        let error = match WorkerExecution::new(
            worker,
            &registry,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(4, 4),
        ) {
            Ok(_) => panic!("created execution for non-running worker"),
            Err(error) => error,
        };
        assert_eq!(
            error,
            WorkerExecutionError::WorkerNotRunning {
                worker,
                state: WorkerLifecycleState::Created,
            }
        );
        assert!(runtime.realms.is_empty());

        registry.mark_running(worker).unwrap();
        let foreign = WorkerRegistry::<u8>::try_new(WorkerLimits::default()).unwrap();
        let error = match WorkerExecution::new(
            worker,
            &foreign,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(4, 4),
        ) {
            Ok(_) => panic!("created execution through foreign registry"),
            Err(error) => error,
        };
        let WorkerExecutionError::Worker(error) = error else {
            panic!("expected exact-registry rejection");
        };
        assert_eq!(error.kind, WorkerErrorKind::UnknownWorker);
        assert!(runtime.realms.is_empty());
    }

    #[test]
    fn closing_worker_cannot_queue_checkpoint_or_drive_pending_script() {
        let (mut registry, worker) = running_worker();
        let mut runtime = FixtureRuntime::new();
        let mut execution = WorkerExecution::new(
            worker,
            &registry,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(4, 4),
        )
        .unwrap();
        execution
            .queue_task(&registry, TaskSource::Other(1), "must not run")
            .unwrap();

        registry.begin_close(worker).unwrap();
        assert_eq!(
            execution
                .queue_microtask(&registry, "must not queue")
                .unwrap_err(),
            WorkerExecutionError::WorkerNotRunning {
                worker,
                state: WorkerLifecycleState::Closing,
            }
        );
        assert_eq!(
            execution
                .request_microtask_checkpoint(&registry)
                .unwrap_err(),
            WorkerExecutionError::WorkerNotRunning {
                worker,
                state: WorkerLifecycleState::Closing,
            }
        );
        assert_eq!(
            execution.next_step(&registry).unwrap_err(),
            WorkerExecutionError::WorkerNotRunning {
                worker,
                state: WorkerLifecycleState::Closing,
            }
        );
        drop(execution);
        assert!(runtime.evaluations.is_empty());
    }

    #[test]
    fn retired_worker_cannot_drive_pending_script() {
        let (mut registry, worker) = running_worker();
        let mut runtime = FixtureRuntime::new();
        let mut execution = WorkerExecution::new(
            worker,
            &registry,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(4, 4),
        )
        .unwrap();
        execution
            .queue_task(&registry, TaskSource::Other(1), "must not run")
            .unwrap();

        registry.begin_close(worker).unwrap();
        registry.retire(worker).unwrap();
        let error = execution.next_step(&registry).unwrap_err();
        let WorkerExecutionError::Worker(error) = error else {
            panic!("expected retired-worker rejection");
        };
        assert_eq!(error.kind, WorkerErrorKind::UnknownWorker);
        drop(execution);
        assert!(runtime.evaluations.is_empty());
    }

    #[test]
    fn worker_message_delivery_runs_as_scheduler_task_and_retains_payload_until_completion() {
        let (registry, worker) = running_worker();
        let mut runtime = FixtureRuntime::new();
        let mut mailbox = WorkerMessageMailbox::try_new(WorkerMessageLimits::default()).unwrap();
        mailbox
            .send_from_root(
                &registry,
                &1,
                worker,
                &WorkerMessageValue::String(String::from("payload")),
            )
            .unwrap();
        let charged = mailbox.queued_bytes();
        let mut execution = WorkerExecution::new(
            worker,
            &registry,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(4, 4),
        )
        .unwrap();

        execution
            .schedule_next_message(&registry, &mut mailbox)
            .unwrap()
            .unwrap();
        assert_eq!(mailbox.queued_bytes(), charged);
        let Some(WorkerExecutionStep::Message { delivery }) =
            execution.next_step(&registry).unwrap()
        else {
            panic!("expected worker message task");
        };
        {
            let message = execution
                .message_for_delivery(&registry, &mailbox, &delivery)
                .unwrap();
            assert_eq!(
                message.payload().value(),
                &WorkerMessageValue::String(String::from("payload"))
            );
        }
        execution
            .queue_microtask(&registry, "after message")
            .unwrap();
        assert_eq!(mailbox.queued_bytes(), charged);
        execution.complete_message(&mut mailbox, delivery).unwrap();
        assert_eq!(mailbox.queued_messages(), 0);
        assert_eq!(mailbox.queued_bytes(), 0);
        assert!(matches!(
            execution.next_step(&registry).unwrap(),
            Some(WorkerExecutionStep::Microtask { .. })
        ));
        assert!(matches!(
            execution.next_step(&registry).unwrap(),
            Some(WorkerExecutionStep::MicrotaskCheckpointComplete)
        ));
    }

    #[test]
    fn message_scheduler_backpressure_leaves_mailbox_payload_pending() {
        let (registry, worker) = running_worker();
        let mut runtime = FixtureRuntime::new();
        let mut mailbox = WorkerMessageMailbox::with_default_limits().unwrap();
        mailbox
            .send_from_root(&registry, &1, worker, &WorkerMessageValue::Null)
            .unwrap();
        let mut execution = WorkerExecution::new(
            worker,
            &registry,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(1, 4),
        )
        .unwrap();
        execution
            .queue_task(&registry, TaskSource::Other(7), "script")
            .unwrap();

        assert_eq!(
            execution
                .schedule_next_message(&registry, &mut mailbox)
                .unwrap_err(),
            WorkerExecutionError::Scheduler(SchedulerError::TaskQueueFull)
        );
        assert_eq!(mailbox.queued_messages(), 1);
        assert!(matches!(
            execution.next_step(&registry).unwrap(),
            Some(WorkerExecutionStep::Task { .. })
        ));
        assert!(matches!(
            execution.next_step(&registry).unwrap(),
            Some(WorkerExecutionStep::MicrotaskCheckpointComplete)
        ));
        assert!(
            execution
                .schedule_next_message(&registry, &mut mailbox)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn only_the_oldest_worker_message_can_be_scheduled_until_it_completes() {
        let (registry, worker) = running_worker();
        let mut runtime = FixtureRuntime::new();
        let mut mailbox = WorkerMessageMailbox::with_default_limits().unwrap();
        let first = mailbox
            .send_from_root(
                &registry,
                &1,
                worker,
                &WorkerMessageValue::String(String::from("first")),
            )
            .unwrap();
        let second = mailbox
            .send_from_root(
                &registry,
                &1,
                worker,
                &WorkerMessageValue::String(String::from("second")),
            )
            .unwrap();
        let mut execution = WorkerExecution::new(
            worker,
            &registry,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(4, 4),
        )
        .unwrap();

        execution
            .schedule_next_message(&registry, &mut mailbox)
            .unwrap()
            .unwrap();
        assert!(
            execution
                .schedule_next_message(&registry, &mut mailbox)
                .unwrap()
                .is_none()
        );
        let Some(WorkerExecutionStep::Message { delivery }) =
            execution.next_step(&registry).unwrap()
        else {
            panic!("expected first message");
        };
        assert_eq!(delivery.message(), first);
        execution.complete_message(&mut mailbox, delivery).unwrap();
        assert!(
            execution
                .schedule_next_message(&registry, &mut mailbox)
                .unwrap()
                .is_some()
        );
        let Some(WorkerExecutionStep::MicrotaskCheckpointComplete) =
            execution.next_step(&registry).unwrap()
        else {
            panic!("expected first message checkpoint");
        };
        let Some(WorkerExecutionStep::Message { delivery }) =
            execution.next_step(&registry).unwrap()
        else {
            panic!("expected second message");
        };
        assert_eq!(delivery.message(), second);
    }

    #[test]
    fn lifecycle_revocation_blocks_selected_message_payload_and_cleanup_recovers_capacity() {
        let mut registry = WorkerRegistry::try_new(WorkerLimits::default()).unwrap();
        let parent = registry.create_root(1).unwrap();
        registry.mark_running(parent).unwrap();
        let child = registry.create_child(parent).unwrap();
        registry.mark_running(child).unwrap();
        let mut runtime = FixtureRuntime::new();
        let mut mailbox = WorkerMessageMailbox::with_default_limits().unwrap();
        mailbox
            .send_between_workers(&registry, child, parent, &WorkerMessageValue::Null)
            .unwrap();
        let mut execution = WorkerExecution::new(
            parent,
            &registry,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(4, 4),
        )
        .unwrap();
        execution
            .schedule_next_message(&registry, &mut mailbox)
            .unwrap()
            .unwrap();
        let Some(WorkerExecutionStep::Message { delivery }) =
            execution.next_step(&registry).unwrap()
        else {
            panic!("expected selected message");
        };
        registry.begin_close(child).unwrap();
        let error = execution
            .message_for_delivery(&registry, &mailbox, &delivery)
            .unwrap_err();
        assert_eq!(
            error,
            WorkerExecutionError::Message(WorkerMessageError::WorkerNotRunning {
                worker: child,
                state: WorkerLifecycleState::Closing,
            })
        );
        execution.complete_message(&mut mailbox, delivery).unwrap();
        assert_eq!(mailbox.queued_messages(), 0);
        assert_eq!(mailbox.queued_bytes(), 0);
    }

    #[test]
    fn dropped_execution_leaves_scheduled_payload_fail_closed_until_worker_cleanup() {
        let (registry, worker) = running_worker();
        let mut mailbox = WorkerMessageMailbox::with_default_limits().unwrap();
        mailbox
            .send_from_root(&registry, &1, worker, &WorkerMessageValue::Null)
            .unwrap();
        let mut runtime = FixtureRuntime::new();
        {
            let mut execution = WorkerExecution::new(
                worker,
                &registry,
                &mut runtime,
                realm_limits(64),
                scheduler_limits(4, 4),
            )
            .unwrap();
            execution
                .schedule_next_message(&registry, &mut mailbox)
                .unwrap()
                .unwrap();
        }
        let mut replacement = WorkerExecution::new(
            worker,
            &registry,
            &mut runtime,
            realm_limits(64),
            scheduler_limits(4, 4),
        )
        .unwrap();
        assert!(
            replacement
                .schedule_next_message(&registry, &mut mailbox)
                .unwrap()
                .is_none()
        );
        let discarded = mailbox.discard_for_worker(worker);
        assert_eq!(discarded.messages(), 1);
        assert_eq!(mailbox.queued_bytes(), 0);
        mailbox
            .send_from_root(&registry, &1, worker, &WorkerMessageValue::Null)
            .unwrap();
        assert!(
            replacement
                .schedule_next_message(&registry, &mut mailbox)
                .unwrap()
                .is_some()
        );
    }
}
