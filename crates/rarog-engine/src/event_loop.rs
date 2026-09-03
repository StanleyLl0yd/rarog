use std::fmt;

use rarog_scheduler::{
    EventLoopScheduler, MicrotaskId, ScheduledMicrotask, ScheduledTask, SchedulerError,
    SchedulerLimits, SchedulerStep, TaskId, TaskSource, WorkId,
};

use crate::{IncrementalReport, RenderError, RenderSession};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineEventLoopError {
    Scheduler(SchedulerError),
    Render(RenderError),
}

impl fmt::Display for EngineEventLoopError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scheduler(error) => write!(formatter, "{error}"),
            Self::Render(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for EngineEventLoopError {}

impl From<SchedulerError> for EngineEventLoopError {
    fn from(error: SchedulerError) -> Self {
        Self::Scheduler(error)
    }
}

impl From<RenderError> for EngineEventLoopError {
    fn from(error: RenderError) -> Self {
        Self::Render(error)
    }
}

#[derive(Debug)]
pub enum EngineEventLoopStep<T, M> {
    Task(ScheduledTask<T>),
    Microtask(ScheduledMicrotask<M>),
    RenderCheckpoint(IncrementalReport),
}

impl<T, M> EngineEventLoopStep<T, M> {
    pub fn work_id(&self) -> Option<WorkId> {
        match self {
            Self::Task(task) => Some(WorkId::Task(task.id)),
            Self::Microtask(microtask) => Some(WorkId::Microtask(microtask.id)),
            Self::RenderCheckpoint(_) => None,
        }
    }
}

#[derive(Debug)]
pub struct EngineEventLoop<T, M> {
    scheduler: EventLoopScheduler<T, M>,
}

impl<T, M> EngineEventLoop<T, M> {
    pub fn new(limits: SchedulerLimits) -> Result<Self, SchedulerError> {
        Ok(Self {
            scheduler: EventLoopScheduler::new(limits)?,
        })
    }

    pub fn queue_task(
        &mut self,
        source: TaskSource,
        payload: T,
    ) -> Result<TaskId, SchedulerError> {
        self.scheduler.queue_task(source, payload)
    }

    pub fn queue_microtask(&mut self, payload: M) -> Result<MicrotaskId, SchedulerError> {
        self.scheduler.queue_microtask(payload)
    }

    pub fn request_microtask_checkpoint(&mut self) {
        self.scheduler.request_microtask_checkpoint();
    }

    pub fn next_step(
        &mut self,
        session: &mut RenderSession,
    ) -> Result<Option<EngineEventLoopStep<T, M>>, EngineEventLoopError> {
        let Some(step) = self.scheduler.next_step()? else {
            return Ok(None);
        };
        Ok(Some(match step {
            SchedulerStep::Task(task) => EngineEventLoopStep::Task(task),
            SchedulerStep::Microtask(microtask) => EngineEventLoopStep::Microtask(microtask),
            SchedulerStep::MicrotaskCheckpointComplete => {
                EngineEventLoopStep::RenderCheckpoint(session.update()?)
            }
        }))
    }

    pub fn complete(&mut self, work: WorkId) -> Result<(), SchedulerError> {
        self.scheduler.complete(work)
    }

    pub fn cancel_task(&mut self, task: TaskId) -> bool {
        self.scheduler.cancel_task(task)
    }

    pub fn active_work(&self) -> Option<WorkId> {
        self.scheduler.active_work()
    }

    pub fn checkpoint_due(&self) -> bool {
        self.scheduler.checkpoint_due()
    }

    pub fn pending_task_count(&self) -> usize {
        self.scheduler.pending_task_count()
    }

    pub fn pending_microtask_count(&self) -> usize {
        self.scheduler.pending_microtask_count()
    }
}
