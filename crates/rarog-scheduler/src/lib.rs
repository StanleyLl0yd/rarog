use std::collections::VecDeque;
use std::fmt;
use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SCHEDULER_SCOPE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SchedulerLimits {
    max_tasks: NonZeroUsize,
    max_microtasks: NonZeroUsize,
}

impl SchedulerLimits {
    pub fn try_new(max_tasks: usize, max_microtasks: usize) -> Result<Self, SchedulerError> {
        let max_tasks = NonZeroUsize::new(max_tasks).ok_or(SchedulerError::InvalidLimits)?;
        let max_microtasks =
            NonZeroUsize::new(max_microtasks).ok_or(SchedulerError::InvalidLimits)?;
        Ok(Self {
            max_tasks,
            max_microtasks,
        })
    }

    pub fn max_tasks(self) -> usize {
        self.max_tasks.get()
    }

    pub fn max_microtasks(self) -> usize {
        self.max_microtasks.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId {
    scope: NonZeroU64,
    serial: NonZeroU64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MicrotaskId {
    scope: NonZeroU64,
    serial: NonZeroU64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorkId {
    Task(TaskId),
    Microtask(MicrotaskId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TaskSource {
    DomManipulation,
    Networking,
    UserInteraction,
    Timer,
    Other(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchedulerError {
    InvalidLimits,
    IdentitySpaceExhausted,
    TaskQueueFull,
    MicrotaskQueueFull,
    WorkInProgress(WorkId),
    NoActiveWork,
    WrongCompletion { expected: WorkId, actual: WorkId },
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => formatter.write_str("scheduler limits must be non-zero"),
            Self::IdentitySpaceExhausted => {
                formatter.write_str("scheduler identity space is exhausted")
            }
            Self::TaskQueueFull => formatter.write_str("scheduler task queue limit exceeded"),
            Self::MicrotaskQueueFull => {
                formatter.write_str("scheduler microtask queue limit exceeded")
            }
            Self::WorkInProgress(work) => {
                write!(formatter, "scheduler work {work:?} is still in progress")
            }
            Self::NoActiveWork => formatter.write_str("scheduler has no active work"),
            Self::WrongCompletion { expected, actual } => write!(
                formatter,
                "scheduler expected completion for {expected:?}, got {actual:?}"
            ),
        }
    }
}

impl std::error::Error for SchedulerError {}

#[derive(Debug)]
pub struct ScheduledTask<T> {
    pub id: TaskId,
    pub source: TaskSource,
    pub payload: T,
}

#[derive(Debug)]
pub struct ScheduledMicrotask<M> {
    pub id: MicrotaskId,
    pub payload: M,
}

#[derive(Debug)]
pub enum SchedulerStep<T, M> {
    Task(ScheduledTask<T>),
    Microtask(ScheduledMicrotask<M>),
    MicrotaskCheckpointComplete,
}

impl<T, M> SchedulerStep<T, M> {
    pub fn work_id(&self) -> Option<WorkId> {
        match self {
            Self::Task(task) => Some(WorkId::Task(task.id)),
            Self::Microtask(microtask) => Some(WorkId::Microtask(microtask.id)),
            Self::MicrotaskCheckpointComplete => None,
        }
    }
}

#[derive(Debug)]
struct QueuedTask<T> {
    id: TaskId,
    source: TaskSource,
    payload: T,
}

#[derive(Debug)]
struct QueuedMicrotask<M> {
    id: MicrotaskId,
    payload: M,
}

#[derive(Debug)]
pub struct EventLoopScheduler<T, M> {
    limits: SchedulerLimits,
    scope: NonZeroU64,
    next_serial: u64,
    tasks: VecDeque<QueuedTask<T>>,
    microtasks: VecDeque<QueuedMicrotask<M>>,
    active: Option<WorkId>,
    checkpoint_due: bool,
}

impl<T, M> EventLoopScheduler<T, M> {
    pub fn new(limits: SchedulerLimits) -> Result<Self, SchedulerError> {
        let scope = NEXT_SCHEDULER_SCOPE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| SchedulerError::IdentitySpaceExhausted)?;
        let scope = NonZeroU64::new(scope).ok_or(SchedulerError::IdentitySpaceExhausted)?;
        Ok(Self {
            limits,
            scope,
            next_serial: 1,
            tasks: VecDeque::new(),
            microtasks: VecDeque::new(),
            active: None,
            checkpoint_due: false,
        })
    }

    pub fn queue_task(&mut self, source: TaskSource, payload: T) -> Result<TaskId, SchedulerError> {
        if self.outstanding_task_count() >= self.limits.max_tasks() {
            return Err(SchedulerError::TaskQueueFull);
        }
        let serial = self.allocate_serial()?;
        let id = TaskId {
            scope: self.scope,
            serial,
        };
        self.tasks.push_back(QueuedTask {
            id,
            source,
            payload,
        });
        Ok(id)
    }

    pub fn queue_microtask(&mut self, payload: M) -> Result<MicrotaskId, SchedulerError> {
        if self.outstanding_microtask_count() >= self.limits.max_microtasks() {
            return Err(SchedulerError::MicrotaskQueueFull);
        }
        let serial = self.allocate_serial()?;
        let id = MicrotaskId {
            scope: self.scope,
            serial,
        };
        self.microtasks.push_back(QueuedMicrotask { id, payload });
        Ok(id)
    }

    pub fn request_microtask_checkpoint(&mut self) {
        self.checkpoint_due = true;
    }

    pub fn next_step(&mut self) -> Result<Option<SchedulerStep<T, M>>, SchedulerError> {
        if let Some(active) = self.active {
            return Err(SchedulerError::WorkInProgress(active));
        }

        if self.checkpoint_due {
            if let Some(microtask) = self.microtasks.pop_front() {
                let id = WorkId::Microtask(microtask.id);
                self.active = Some(id);
                return Ok(Some(SchedulerStep::Microtask(ScheduledMicrotask {
                    id: microtask.id,
                    payload: microtask.payload,
                })));
            }
            self.checkpoint_due = false;
            return Ok(Some(SchedulerStep::MicrotaskCheckpointComplete));
        }

        let Some(task) = self.tasks.pop_front() else {
            return Ok(None);
        };
        let id = WorkId::Task(task.id);
        self.active = Some(id);
        Ok(Some(SchedulerStep::Task(ScheduledTask {
            id: task.id,
            source: task.source,
            payload: task.payload,
        })))
    }

    pub fn complete(&mut self, work: WorkId) -> Result<(), SchedulerError> {
        let Some(active) = self.active else {
            return Err(SchedulerError::NoActiveWork);
        };
        if active != work {
            return Err(SchedulerError::WrongCompletion {
                expected: active,
                actual: work,
            });
        }
        self.active = None;
        if matches!(work, WorkId::Task(_)) {
            self.checkpoint_due = true;
        }
        Ok(())
    }

    pub fn cancel_task(&mut self, task: TaskId) -> bool {
        let Some(position) = self.tasks.iter().position(|queued| queued.id == task) else {
            return false;
        };
        self.tasks.remove(position);
        true
    }

    pub fn active_work(&self) -> Option<WorkId> {
        self.active
    }

    pub fn checkpoint_due(&self) -> bool {
        self.checkpoint_due
    }

    pub fn pending_task_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn pending_microtask_count(&self) -> usize {
        self.microtasks.len()
    }

    fn allocate_serial(&mut self) -> Result<NonZeroU64, SchedulerError> {
        let serial =
            NonZeroU64::new(self.next_serial).ok_or(SchedulerError::IdentitySpaceExhausted)?;
        self.next_serial = self.next_serial.checked_add(1).unwrap_or(0);
        Ok(serial)
    }

    fn outstanding_task_count(&self) -> usize {
        self.tasks.len() + usize::from(matches!(self.active, Some(WorkId::Task(_))))
    }

    fn outstanding_microtask_count(&self) -> usize {
        self.microtasks.len() + usize::from(matches!(self.active, Some(WorkId::Microtask(_))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scheduler() -> EventLoopScheduler<&'static str, &'static str> {
        EventLoopScheduler::new(SchedulerLimits::try_new(8, 8).unwrap()).unwrap()
    }

    #[test]
    fn schedulers_do_not_alias_work_ids() {
        let mut first = scheduler();
        let mut second = scheduler();
        let first_id = first.queue_task(TaskSource::Other(1), "a").unwrap();
        let second_id = second.queue_task(TaskSource::Other(1), "a").unwrap();
        assert_ne!(first_id, second_id);
    }

    #[test]
    fn tasks_are_fifo() {
        let mut scheduler = scheduler();
        let first = scheduler
            .queue_task(TaskSource::Networking, "first")
            .unwrap();
        let second = scheduler
            .queue_task(TaskSource::UserInteraction, "second")
            .unwrap();

        let SchedulerStep::Task(task) = scheduler.next_step().unwrap().unwrap() else {
            panic!("expected first task");
        };
        assert_eq!(task.id, first);
        assert_eq!(task.payload, "first");
        scheduler.complete(WorkId::Task(task.id)).unwrap();
        assert!(matches!(
            scheduler.next_step().unwrap().unwrap(),
            SchedulerStep::MicrotaskCheckpointComplete
        ));

        let SchedulerStep::Task(task) = scheduler.next_step().unwrap().unwrap() else {
            panic!("expected second task");
        };
        assert_eq!(task.id, second);
        assert_eq!(task.payload, "second");
    }

    #[test]
    fn task_completion_drains_microtasks_before_the_next_task() {
        let mut scheduler = scheduler();
        let first = scheduler
            .queue_task(TaskSource::DomManipulation, "first")
            .unwrap();
        scheduler
            .queue_task(TaskSource::DomManipulation, "second")
            .unwrap();

        let step = scheduler.next_step().unwrap().unwrap();
        assert_eq!(step.work_id(), Some(WorkId::Task(first)));
        scheduler.queue_microtask("microtask").unwrap();
        scheduler.complete(WorkId::Task(first)).unwrap();

        let SchedulerStep::Microtask(microtask) = scheduler.next_step().unwrap().unwrap() else {
            panic!("expected microtask");
        };
        assert_eq!(microtask.payload, "microtask");
        scheduler.complete(WorkId::Microtask(microtask.id)).unwrap();
        assert!(matches!(
            scheduler.next_step().unwrap().unwrap(),
            SchedulerStep::MicrotaskCheckpointComplete
        ));
        assert!(matches!(
            scheduler.next_step().unwrap().unwrap(),
            SchedulerStep::Task(_)
        ));
    }

    #[test]
    fn microtasks_queued_during_checkpoint_join_the_same_checkpoint() {
        let mut scheduler = scheduler();
        let task = scheduler
            .queue_task(TaskSource::DomManipulation, "task")
            .unwrap();
        scheduler.next_step().unwrap().unwrap();
        scheduler.queue_microtask("first").unwrap();
        scheduler.complete(WorkId::Task(task)).unwrap();

        let SchedulerStep::Microtask(first) = scheduler.next_step().unwrap().unwrap() else {
            panic!("expected first microtask");
        };
        scheduler.queue_microtask("second").unwrap();
        scheduler.complete(WorkId::Microtask(first.id)).unwrap();

        let SchedulerStep::Microtask(second) = scheduler.next_step().unwrap().unwrap() else {
            panic!("expected second microtask");
        };
        assert_eq!(second.payload, "second");
        scheduler.complete(WorkId::Microtask(second.id)).unwrap();
        assert!(matches!(
            scheduler.next_step().unwrap().unwrap(),
            SchedulerStep::MicrotaskCheckpointComplete
        ));
    }

    #[test]
    fn explicit_checkpoint_drains_microtasks_without_a_task() {
        let mut scheduler = scheduler();
        let microtask = scheduler.queue_microtask("microtask").unwrap();
        assert!(scheduler.next_step().unwrap().is_none());
        scheduler.request_microtask_checkpoint();
        let step = scheduler.next_step().unwrap().unwrap();
        assert_eq!(step.work_id(), Some(WorkId::Microtask(microtask)));
    }

    #[test]
    fn active_work_blocks_advancing_the_scheduler() {
        let mut scheduler = scheduler();
        let task = scheduler
            .queue_task(TaskSource::Networking, "task")
            .unwrap();
        scheduler.next_step().unwrap().unwrap();
        assert_eq!(
            scheduler.next_step().unwrap_err(),
            SchedulerError::WorkInProgress(WorkId::Task(task))
        );
    }

    #[test]
    fn wrong_completion_does_not_clear_active_work() {
        let mut scheduler = scheduler();
        let first = scheduler
            .queue_task(TaskSource::Networking, "first")
            .unwrap();
        let second = scheduler
            .queue_task(TaskSource::Networking, "second")
            .unwrap();
        scheduler.next_step().unwrap().unwrap();
        assert_eq!(
            scheduler.complete(WorkId::Task(second)).unwrap_err(),
            SchedulerError::WrongCompletion {
                expected: WorkId::Task(first),
                actual: WorkId::Task(second),
            }
        );
        assert_eq!(scheduler.active_work(), Some(WorkId::Task(first)));
    }

    #[test]
    fn queued_tasks_can_be_cancelled_without_touching_active_work() {
        let mut scheduler = scheduler();
        let active = scheduler.queue_task(TaskSource::Timer, "active").unwrap();
        let queued = scheduler.queue_task(TaskSource::Timer, "queued").unwrap();
        scheduler.next_step().unwrap().unwrap();
        assert!(!scheduler.cancel_task(active));
        assert!(scheduler.cancel_task(queued));
        assert_eq!(scheduler.pending_task_count(), 0);
        assert_eq!(scheduler.active_work(), Some(WorkId::Task(active)));
    }

    #[test]
    fn queue_limits_include_active_work() {
        let limits = SchedulerLimits::try_new(1, 1).unwrap();
        let mut scheduler = EventLoopScheduler::new(limits).unwrap();
        let task = scheduler
            .queue_task(TaskSource::Networking, "task")
            .unwrap();
        scheduler.next_step().unwrap().unwrap();
        assert_eq!(
            scheduler
                .queue_task(TaskSource::Networking, "blocked")
                .unwrap_err(),
            SchedulerError::TaskQueueFull
        );
        scheduler.queue_microtask("microtask").unwrap();
        scheduler.complete(WorkId::Task(task)).unwrap();
        let SchedulerStep::Microtask(microtask) = scheduler.next_step().unwrap().unwrap() else {
            panic!("expected microtask");
        };
        assert_eq!(
            scheduler.queue_microtask("blocked").unwrap_err(),
            SchedulerError::MicrotaskQueueFull
        );
        scheduler.complete(WorkId::Microtask(microtask.id)).unwrap();
    }

    #[test]
    fn identity_exhaustion_is_reported_without_reusing_ids() {
        let mut scheduler = scheduler();
        scheduler.next_serial = u64::MAX;
        let last = scheduler.queue_task(TaskSource::Other(7), "last").unwrap();
        assert_eq!(last.serial.get(), u64::MAX);
        assert_eq!(
            scheduler
                .queue_task(TaskSource::Other(7), "overflow")
                .unwrap_err(),
            SchedulerError::IdentitySpaceExhausted
        );
    }

    #[test]
    fn limits_reject_zero_capacity() {
        assert_eq!(
            SchedulerLimits::try_new(0, 1).unwrap_err(),
            SchedulerError::InvalidLimits
        );
        assert_eq!(
            SchedulerLimits::try_new(1, 0).unwrap_err(),
            SchedulerError::InvalidLimits
        );
    }
}
