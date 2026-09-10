use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_WORKER_SCOPE: AtomicU64 = AtomicU64::new(1);

pub const DEFAULT_MAX_WORKERS: usize = 256;
pub const DEFAULT_MAX_CHILDREN_PER_OWNER: usize = 32;
pub const DEFAULT_MAX_WORKER_DEPTH: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkerLimits {
    pub max_workers: usize,
    pub max_children_per_owner: usize,
    pub max_depth: usize,
}

impl WorkerLimits {
    pub fn is_valid(self) -> bool {
        self.max_workers > 0 && self.max_children_per_owner > 0 && self.max_depth > 0
    }
}

impl Default for WorkerLimits {
    fn default() -> Self {
        Self {
            max_workers: DEFAULT_MAX_WORKERS,
            max_children_per_owner: DEFAULT_MAX_CHILDREN_PER_OWNER,
            max_depth: DEFAULT_MAX_WORKER_DEPTH,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerId {
    scope: NonZeroU64,
    serial: NonZeroU64,
}

impl WorkerId {
    pub fn try_from_parts(scope: u64, serial: u64) -> Result<Self, WorkerError> {
        let scope = NonZeroU64::new(scope).ok_or_else(|| {
            WorkerError::new(
                WorkerErrorKind::InvalidWorkerId,
                "worker registry scope must be non-zero",
            )
        })?;
        let serial = NonZeroU64::new(serial).ok_or_else(|| {
            WorkerError::new(
                WorkerErrorKind::InvalidWorkerId,
                "worker serial must be non-zero",
            )
        })?;
        Ok(Self { scope, serial })
    }

    pub fn scope(self) -> u64 {
        self.scope.get()
    }

    pub fn serial(self) -> u64 {
        self.serial.get()
    }
}

impl fmt::Display for WorkerId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "worker:{}:{}", self.scope(), self.serial())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum WorkerOwner<O> {
    Root(O),
    Worker(WorkerId),
}

impl<O> WorkerOwner<O> {
    pub fn root(&self) -> Option<&O> {
        match self {
            Self::Root(owner) => Some(owner),
            Self::Worker(_) => None,
        }
    }

    pub fn parent_worker(&self) -> Option<WorkerId> {
        match self {
            Self::Root(_) => None,
            Self::Worker(worker) => Some(*worker),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerLifecycleState {
    Created,
    Running,
    Closing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerErrorKind {
    InvalidLimits,
    InvalidWorkerId,
    IdentitySpaceExhausted,
    WorkerLimitExceeded,
    ChildLimitExceeded,
    DepthLimitExceeded,
    UnknownWorker,
    ParentNotRunning,
    InvalidLifecycleTransition,
    InconsistentState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkerError {
    pub kind: WorkerErrorKind,
    pub message: String,
}

impl WorkerError {
    fn new(kind: WorkerErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for WorkerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for WorkerError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkerRetirement {
    worker: WorkerId,
    retired_workers: usize,
}

impl WorkerRetirement {
    pub fn worker(self) -> WorkerId {
        self.worker
    }

    pub fn retired_workers(self) -> usize {
        self.retired_workers
    }

    pub fn retired_descendants(self) -> usize {
        self.retired_workers.saturating_sub(1)
    }
}

#[derive(Debug)]
struct WorkerIdAllocator {
    scope: NonZeroU64,
    next_serial: u64,
}

impl WorkerIdAllocator {
    fn new() -> Result<Self, WorkerError> {
        let scope = NEXT_WORKER_SCOPE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| {
                WorkerError::new(
                    WorkerErrorKind::IdentitySpaceExhausted,
                    "worker registry identity space is exhausted",
                )
            })?;
        let scope = NonZeroU64::new(scope).ok_or_else(|| {
            WorkerError::new(
                WorkerErrorKind::IdentitySpaceExhausted,
                "worker registry identity space is exhausted",
            )
        })?;
        Ok(Self {
            scope,
            next_serial: 1,
        })
    }

    fn allocate(&mut self) -> Result<WorkerId, WorkerError> {
        let serial = NonZeroU64::new(self.next_serial).ok_or_else(|| {
            WorkerError::new(
                WorkerErrorKind::IdentitySpaceExhausted,
                "worker identity space is exhausted",
            )
        })?;
        self.next_serial = self.next_serial.checked_add(1).unwrap_or(0);
        Ok(WorkerId {
            scope: self.scope,
            serial,
        })
    }
}

#[derive(Debug)]
struct WorkerRecord<O> {
    owner: WorkerOwner<O>,
    state: WorkerLifecycleState,
    depth: usize,
    children: BTreeSet<WorkerId>,
}

#[derive(Debug)]
pub struct WorkerRegistry<O> {
    limits: WorkerLimits,
    allocator: WorkerIdAllocator,
    workers: BTreeMap<WorkerId, WorkerRecord<O>>,
}

impl<O: Eq> WorkerRegistry<O> {
    pub fn try_new(limits: WorkerLimits) -> Result<Self, WorkerError> {
        if !limits.is_valid() {
            return Err(WorkerError::new(
                WorkerErrorKind::InvalidLimits,
                "worker limits must all be non-zero",
            ));
        }
        Ok(Self {
            limits,
            allocator: WorkerIdAllocator::new()?,
            workers: BTreeMap::new(),
        })
    }

    pub fn with_default_limits() -> Result<Self, WorkerError> {
        Self::try_new(WorkerLimits::default())
    }

    pub fn limits(&self) -> WorkerLimits {
        self.limits
    }

    pub fn live_workers(&self) -> usize {
        self.workers.len()
    }

    pub fn create_root(&mut self, owner: O) -> Result<WorkerId, WorkerError> {
        self.ensure_worker_capacity()?;
        let direct_children = self
            .workers
            .values()
            .filter(|record| {
                matches!(&record.owner, WorkerOwner::Root(candidate) if candidate == &owner)
            })
            .count();
        self.ensure_child_capacity(direct_children)?;

        let id = self.allocator.allocate()?;
        self.insert_worker(
            id,
            WorkerRecord {
                owner: WorkerOwner::Root(owner),
                state: WorkerLifecycleState::Created,
                depth: 1,
                children: BTreeSet::new(),
            },
        )?;
        Ok(id)
    }

    pub fn create_child(&mut self, parent: WorkerId) -> Result<WorkerId, WorkerError> {
        self.ensure_worker_capacity()?;
        let parent_record = self.require_worker(parent)?;
        if parent_record.state != WorkerLifecycleState::Running {
            return Err(WorkerError::new(
                WorkerErrorKind::ParentNotRunning,
                format!("parent {parent} is not running"),
            ));
        }
        self.ensure_child_capacity(parent_record.children.len())?;
        let depth = parent_record.depth.checked_add(1).ok_or_else(|| {
            WorkerError::new(
                WorkerErrorKind::DepthLimitExceeded,
                "worker ownership depth overflowed",
            )
        })?;
        if depth > self.limits.max_depth {
            return Err(WorkerError::new(
                WorkerErrorKind::DepthLimitExceeded,
                format!(
                    "worker ownership depth {depth} exceeds configured limit {}",
                    self.limits.max_depth
                ),
            ));
        }

        let id = self.allocator.allocate()?;
        self.insert_worker(
            id,
            WorkerRecord {
                owner: WorkerOwner::Worker(parent),
                state: WorkerLifecycleState::Created,
                depth,
                children: BTreeSet::new(),
            },
        )?;
        let parent_record = self.workers.get_mut(&parent).ok_or_else(|| {
            WorkerError::new(
                WorkerErrorKind::InconsistentState,
                format!("parent {parent} disappeared while creating child {id}"),
            )
        })?;
        if !parent_record.children.insert(id) {
            self.workers.remove(&id);
            return Err(WorkerError::new(
                WorkerErrorKind::InconsistentState,
                format!("parent {parent} already tracked child {id}"),
            ));
        }
        Ok(id)
    }

    pub fn owner(&self, worker: WorkerId) -> Result<&WorkerOwner<O>, WorkerError> {
        Ok(&self.require_worker(worker)?.owner)
    }

    pub fn state(&self, worker: WorkerId) -> Result<WorkerLifecycleState, WorkerError> {
        Ok(self.require_worker(worker)?.state)
    }

    pub fn depth(&self, worker: WorkerId) -> Result<usize, WorkerError> {
        Ok(self.require_worker(worker)?.depth)
    }

    pub fn child_count(&self, worker: WorkerId) -> Result<usize, WorkerError> {
        Ok(self.require_worker(worker)?.children.len())
    }

    pub fn mark_running(&mut self, worker: WorkerId) -> Result<(), WorkerError> {
        let record = self.require_worker_mut(worker)?;
        if record.state != WorkerLifecycleState::Created {
            return Err(Self::invalid_transition(
                worker,
                record.state,
                WorkerLifecycleState::Running,
            ));
        }
        record.state = WorkerLifecycleState::Running;
        Ok(())
    }

    pub fn begin_close(&mut self, worker: WorkerId) -> Result<usize, WorkerError> {
        let state = self.state(worker)?;
        if state == WorkerLifecycleState::Closing {
            return Err(Self::invalid_transition(
                worker,
                state,
                WorkerLifecycleState::Closing,
            ));
        }
        let subtree = self.subtree_ids(worker)?;
        for id in &subtree {
            let record = self.workers.get_mut(id).ok_or_else(|| {
                WorkerError::new(
                    WorkerErrorKind::InconsistentState,
                    format!("worker {id} disappeared while closing {worker}"),
                )
            })?;
            record.state = WorkerLifecycleState::Closing;
        }
        Ok(subtree.len())
    }

    pub fn retire(&mut self, worker: WorkerId) -> Result<WorkerRetirement, WorkerError> {
        let state = self.state(worker)?;
        if state != WorkerLifecycleState::Closing {
            return Err(Self::invalid_transition(
                worker,
                state,
                WorkerLifecycleState::Closing,
            ));
        }
        let retired_workers = self.remove_subtree(worker)?;
        Ok(WorkerRetirement {
            worker,
            retired_workers,
        })
    }

    pub fn retire_root_owner(&mut self, owner: &O) -> Result<usize, WorkerError> {
        let roots = self
            .workers
            .iter()
            .filter_map(|(worker, record)| {
                matches!(&record.owner, WorkerOwner::Root(candidate) if candidate == owner)
                    .then_some(*worker)
            })
            .collect::<Vec<_>>();
        let mut retired = 0usize;
        for root in roots {
            retired = retired
                .checked_add(self.remove_subtree(root)?)
                .ok_or_else(|| {
                    WorkerError::new(
                        WorkerErrorKind::InconsistentState,
                        "retired worker count overflowed",
                    )
                })?;
        }
        Ok(retired)
    }

    fn ensure_worker_capacity(&self) -> Result<(), WorkerError> {
        if self.workers.len() >= self.limits.max_workers {
            return Err(WorkerError::new(
                WorkerErrorKind::WorkerLimitExceeded,
                format!("worker limit {} reached", self.limits.max_workers),
            ));
        }
        Ok(())
    }

    fn ensure_child_capacity(&self, current: usize) -> Result<(), WorkerError> {
        if current >= self.limits.max_children_per_owner {
            return Err(WorkerError::new(
                WorkerErrorKind::ChildLimitExceeded,
                format!(
                    "worker child limit {} reached for owner",
                    self.limits.max_children_per_owner
                ),
            ));
        }
        Ok(())
    }

    fn insert_worker(&mut self, id: WorkerId, record: WorkerRecord<O>) -> Result<(), WorkerError> {
        if self.workers.contains_key(&id) {
            return Err(WorkerError::new(
                WorkerErrorKind::InconsistentState,
                format!("worker allocator reused live identity {id}"),
            ));
        }
        self.workers.insert(id, record);
        Ok(())
    }

    fn require_worker(&self, worker: WorkerId) -> Result<&WorkerRecord<O>, WorkerError> {
        self.workers.get(&worker).ok_or_else(|| {
            WorkerError::new(
                WorkerErrorKind::UnknownWorker,
                format!("unknown or retired worker {worker}"),
            )
        })
    }

    fn require_worker_mut(
        &mut self,
        worker: WorkerId,
    ) -> Result<&mut WorkerRecord<O>, WorkerError> {
        self.workers.get_mut(&worker).ok_or_else(|| {
            WorkerError::new(
                WorkerErrorKind::UnknownWorker,
                format!("unknown or retired worker {worker}"),
            )
        })
    }

    fn subtree_ids(&self, root: WorkerId) -> Result<Vec<WorkerId>, WorkerError> {
        self.require_worker(root)?;
        let mut stack = vec![root];
        let mut subtree = Vec::new();
        while let Some(worker) = stack.pop() {
            let record = self.workers.get(&worker).ok_or_else(|| {
                WorkerError::new(
                    WorkerErrorKind::InconsistentState,
                    format!("worker ownership tree references missing {worker}"),
                )
            })?;
            subtree.push(worker);
            for child in record.children.iter().rev() {
                stack.push(*child);
            }
        }
        Ok(subtree)
    }

    fn remove_subtree(&mut self, root: WorkerId) -> Result<usize, WorkerError> {
        let parent = self.require_worker(root)?.owner.parent_worker();
        if let Some(parent) = parent {
            let parent_record = self.workers.get(&parent).ok_or_else(|| {
                WorkerError::new(
                    WorkerErrorKind::InconsistentState,
                    format!("worker {root} references missing parent {parent}"),
                )
            })?;
            if !parent_record.children.contains(&root) {
                return Err(WorkerError::new(
                    WorkerErrorKind::InconsistentState,
                    format!("parent {parent} does not reference child {root}"),
                ));
            }
        }

        let subtree = self.subtree_ids(root)?;
        if let Some(parent) = parent {
            let parent_record = self.workers.get_mut(&parent).ok_or_else(|| {
                WorkerError::new(
                    WorkerErrorKind::InconsistentState,
                    format!("worker {root} parent {parent} disappeared during retirement"),
                )
            })?;
            parent_record.children.remove(&root);
        }
        for worker in subtree.iter().rev() {
            self.workers.remove(worker);
        }
        Ok(subtree.len())
    }

    fn invalid_transition(
        worker: WorkerId,
        current: WorkerLifecycleState,
        requested: WorkerLifecycleState,
    ) -> WorkerError {
        WorkerError::new(
            WorkerErrorKind::InvalidLifecycleTransition,
            format!("worker {worker} cannot transition from {current:?} to {requested:?}"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(max_workers: usize, max_children: usize, max_depth: usize) -> WorkerLimits {
        WorkerLimits {
            max_workers,
            max_children_per_owner: max_children,
            max_depth,
        }
    }

    fn registry() -> WorkerRegistry<u64> {
        WorkerRegistry::try_new(limits(16, 4, 4)).unwrap()
    }

    #[test]
    fn limits_reject_zero_values() {
        for invalid in [limits(0, 1, 1), limits(1, 0, 1), limits(1, 1, 0)] {
            let error = WorkerRegistry::<u64>::try_new(invalid).unwrap_err();
            assert_eq!(error.kind, WorkerErrorKind::InvalidLimits);
        }
    }

    #[test]
    fn worker_id_rejects_zero_parts() {
        let scope_error = WorkerId::try_from_parts(0, 1).unwrap_err();
        assert_eq!(scope_error.kind, WorkerErrorKind::InvalidWorkerId);
        let serial_error = WorkerId::try_from_parts(1, 0).unwrap_err();
        assert_eq!(serial_error.kind, WorkerErrorKind::InvalidWorkerId);
    }

    #[test]
    fn root_workers_are_created_with_explicit_owner_and_depth() {
        let mut registry = registry();
        let worker = registry.create_root(7).unwrap();
        assert!(worker.scope() > 0);
        assert_eq!(worker.serial(), 1);
        assert_eq!(registry.owner(worker).unwrap(), &WorkerOwner::Root(7));
        assert_eq!(
            registry.state(worker).unwrap(),
            WorkerLifecycleState::Created
        );
        assert_eq!(registry.depth(worker).unwrap(), 1);
    }

    #[test]
    fn parent_must_be_running_before_it_can_create_children() {
        let mut registry = registry();
        let parent = registry.create_root(1).unwrap();
        let error = registry.create_child(parent).unwrap_err();
        assert_eq!(error.kind, WorkerErrorKind::ParentNotRunning);
        registry.mark_running(parent).unwrap();
        let child = registry.create_child(parent).unwrap();
        assert_eq!(registry.owner(child).unwrap(), &WorkerOwner::Worker(parent));
        assert_eq!(registry.depth(child).unwrap(), 2);
    }

    #[test]
    fn live_worker_limit_fails_before_allocating_another_worker() {
        let mut registry = WorkerRegistry::try_new(limits(1, 1, 1)).unwrap();
        let first = registry.create_root(1).unwrap();
        let error = registry.create_root(2).unwrap_err();
        assert_eq!(error.kind, WorkerErrorKind::WorkerLimitExceeded);
        registry.begin_close(first).unwrap();
        registry.retire(first).unwrap();
        let replacement = registry.create_root(2).unwrap();
        assert_eq!(replacement.scope(), first.scope());
        assert_eq!(replacement.serial(), 2);
    }

    #[test]
    fn direct_children_per_owner_are_bounded() {
        let mut registry = WorkerRegistry::try_new(limits(8, 1, 3)).unwrap();
        registry.create_root(1).unwrap();
        let error = registry.create_root(1).unwrap_err();
        assert_eq!(error.kind, WorkerErrorKind::ChildLimitExceeded);
        registry.create_root(2).unwrap();
    }

    #[test]
    fn parent_child_count_is_bounded_independently() {
        let mut registry = WorkerRegistry::try_new(limits(8, 1, 3)).unwrap();
        let parent = registry.create_root(1).unwrap();
        registry.mark_running(parent).unwrap();
        registry.create_child(parent).unwrap();
        let error = registry.create_child(parent).unwrap_err();
        assert_eq!(error.kind, WorkerErrorKind::ChildLimitExceeded);
    }

    #[test]
    fn ownership_depth_is_bounded() {
        let mut registry = WorkerRegistry::try_new(limits(8, 4, 2)).unwrap();
        let root = registry.create_root(1).unwrap();
        registry.mark_running(root).unwrap();
        let child = registry.create_child(root).unwrap();
        registry.mark_running(child).unwrap();
        let error = registry.create_child(child).unwrap_err();
        assert_eq!(error.kind, WorkerErrorKind::DepthLimitExceeded);
    }

    #[test]
    fn lifecycle_transitions_are_explicit() {
        let mut registry = registry();
        let worker = registry.create_root(1).unwrap();
        registry.mark_running(worker).unwrap();
        let error = registry.mark_running(worker).unwrap_err();
        assert_eq!(error.kind, WorkerErrorKind::InvalidLifecycleTransition);
        assert_eq!(registry.begin_close(worker).unwrap(), 1);
        let error = registry.begin_close(worker).unwrap_err();
        assert_eq!(error.kind, WorkerErrorKind::InvalidLifecycleTransition);
        let retirement = registry.retire(worker).unwrap();
        assert_eq!(retirement.worker(), worker);
        assert_eq!(retirement.retired_workers(), 1);
        assert_eq!(retirement.retired_descendants(), 0);
        assert_eq!(registry.live_workers(), 0);
        assert_eq!(
            registry.state(worker).unwrap_err().kind,
            WorkerErrorKind::UnknownWorker
        );
    }

    #[test]
    fn retirement_requires_closing_state() {
        let mut registry = registry();
        let worker = registry.create_root(1).unwrap();
        let error = registry.retire(worker).unwrap_err();
        assert_eq!(error.kind, WorkerErrorKind::InvalidLifecycleTransition);
        assert_eq!(registry.live_workers(), 1);
    }

    #[test]
    fn closing_parent_cascades_to_descendants_and_blocks_restart() {
        let mut registry = registry();
        let root = registry.create_root(1).unwrap();
        registry.mark_running(root).unwrap();
        let child = registry.create_child(root).unwrap();
        registry.mark_running(child).unwrap();
        let grandchild = registry.create_child(child).unwrap();

        assert_eq!(registry.begin_close(root).unwrap(), 3);
        for worker in [root, child, grandchild] {
            assert_eq!(
                registry.state(worker).unwrap(),
                WorkerLifecycleState::Closing
            );
        }
        let error = registry.mark_running(grandchild).unwrap_err();
        assert_eq!(error.kind, WorkerErrorKind::InvalidLifecycleTransition);

        let retirement = registry.retire(root).unwrap();
        assert_eq!(retirement.retired_workers(), 3);
        assert_eq!(retirement.retired_descendants(), 2);
        assert_eq!(registry.live_workers(), 0);
    }

    #[test]
    fn retiring_child_releases_parent_child_budget() {
        let mut registry = WorkerRegistry::try_new(limits(8, 1, 3)).unwrap();
        let parent = registry.create_root(1).unwrap();
        registry.mark_running(parent).unwrap();
        let child = registry.create_child(parent).unwrap();
        registry.begin_close(child).unwrap();
        registry.retire(child).unwrap();
        assert_eq!(registry.child_count(parent).unwrap(), 0);
        let replacement = registry.create_child(parent).unwrap();
        assert_ne!(replacement, child);
    }

    #[test]
    fn retiring_root_owner_is_scoped_and_cascades() {
        let mut registry = registry();
        let first = registry.create_root(1).unwrap();
        registry.mark_running(first).unwrap();
        let child = registry.create_child(first).unwrap();
        let other = registry.create_root(2).unwrap();

        assert_eq!(registry.retire_root_owner(&1).unwrap(), 2);
        assert_eq!(registry.live_workers(), 1);
        assert_eq!(
            registry.state(first).unwrap_err().kind,
            WorkerErrorKind::UnknownWorker
        );
        assert_eq!(
            registry.state(child).unwrap_err().kind,
            WorkerErrorKind::UnknownWorker
        );
        assert_eq!(
            registry.state(other).unwrap(),
            WorkerLifecycleState::Created
        );
    }

    #[test]
    fn retired_worker_identity_is_never_reused() {
        let mut registry = registry();
        let first = registry.create_root(1).unwrap();
        registry.begin_close(first).unwrap();
        registry.retire(first).unwrap();
        let second = registry.create_root(1).unwrap();
        assert_ne!(first, second);
        assert_eq!(first.scope(), second.scope());
        assert_eq!(first.serial(), 1);
        assert_eq!(second.serial(), 2);
    }

    #[test]
    fn independent_registries_do_not_alias_worker_ids() {
        let mut first = registry();
        let mut second = registry();
        let first_worker = first.create_root(1).unwrap();
        let second_worker = second.create_root(1).unwrap();
        assert_ne!(first_worker, second_worker);
        assert_ne!(first_worker.scope(), second_worker.scope());
        assert_eq!(first_worker.serial(), 1);
        assert_eq!(second_worker.serial(), 1);
    }

    #[test]
    fn identity_allocator_fails_closed_after_last_serial() {
        let mut allocator = WorkerIdAllocator {
            scope: NonZeroU64::new(7).unwrap(),
            next_serial: u64::MAX,
        };
        let last = allocator.allocate().unwrap();
        assert_eq!(last.scope(), 7);
        assert_eq!(last.serial(), u64::MAX);
        let error = allocator.allocate().unwrap_err();
        assert_eq!(error.kind, WorkerErrorKind::IdentitySpaceExhausted);
    }
}
