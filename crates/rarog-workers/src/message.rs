use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

use rarog_scheduler::TaskId;

use crate::identity::{WorkerError, WorkerId, WorkerLifecycleState, WorkerOwner, WorkerRegistry};

static NEXT_MESSAGE_SCOPE: AtomicU64 = AtomicU64::new(1);

pub const DEFAULT_MAX_WORKER_MESSAGE_BYTES: usize = 256 * 1024;
pub const DEFAULT_MAX_WORKER_MESSAGE_ITEMS: usize = 4096;
pub const DEFAULT_MAX_WORKER_MESSAGE_DEPTH: usize = 32;
pub const HARD_MAX_WORKER_MESSAGE_DEPTH: usize = 64;
pub const DEFAULT_MAX_QUEUED_WORKER_MESSAGES: usize = 64;
pub const DEFAULT_MAX_QUEUED_WORKER_MESSAGE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkerMessageLimits {
    pub max_message_bytes: usize,
    pub max_message_items: usize,
    pub max_message_depth: usize,
    pub max_queued_messages: usize,
    pub max_queued_bytes: usize,
}

impl WorkerMessageLimits {
    pub fn is_valid(self) -> bool {
        self.max_message_bytes > 0
            && self.max_message_items > 0
            && self.max_message_depth > 0
            && self.max_message_depth <= HARD_MAX_WORKER_MESSAGE_DEPTH
            && self.max_queued_messages > 0
            && self.max_queued_bytes > 0
            && self.max_message_bytes <= self.max_queued_bytes
    }
}

impl Default for WorkerMessageLimits {
    fn default() -> Self {
        Self {
            max_message_bytes: DEFAULT_MAX_WORKER_MESSAGE_BYTES,
            max_message_items: DEFAULT_MAX_WORKER_MESSAGE_ITEMS,
            max_message_depth: DEFAULT_MAX_WORKER_MESSAGE_DEPTH,
            max_queued_messages: DEFAULT_MAX_QUEUED_WORKER_MESSAGES,
            max_queued_bytes: DEFAULT_MAX_QUEUED_WORKER_MESSAGE_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerMessageNumber(u64);

impl WorkerMessageNumber {
    pub fn from_f64(value: f64) -> Self {
        Self(value.to_bits())
    }

    pub fn to_f64(self) -> f64 {
        f64::from_bits(self.0)
    }

    pub fn bits(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkerMessageValue {
    Null,
    Boolean(bool),
    Number(WorkerMessageNumber),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<WorkerMessageValue>),
    Object(BTreeMap<String, WorkerMessageValue>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PayloadMetrics {
    bytes: usize,
    items: usize,
    depth: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkerMessagePayload {
    value: WorkerMessageValue,
    metrics: PayloadMetrics,
}

impl WorkerMessagePayload {
    pub fn value(&self) -> &WorkerMessageValue {
        &self.value
    }

    pub fn into_value(self) -> WorkerMessageValue {
        self.value
    }

    pub fn retained_bytes(&self) -> usize {
        self.metrics.bytes
    }

    pub fn item_count(&self) -> usize {
        self.metrics.items
    }

    pub fn depth(&self) -> usize {
        self.metrics.depth
    }

    fn measure(
        value: &WorkerMessageValue,
        limits: WorkerMessageLimits,
    ) -> Result<PayloadMetrics, WorkerMessageError> {
        measure_payload(value, limits)
    }

    fn clone_with_metrics(value: &WorkerMessageValue, metrics: PayloadMetrics) -> Self {
        Self {
            value: value.clone(),
            metrics,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerMessageId {
    scope: NonZeroU64,
    serial: NonZeroU64,
}

impl WorkerMessageId {
    pub fn scope(self) -> u64 {
        self.scope.get()
    }

    pub fn serial(self) -> u64 {
        self.serial.get()
    }
}

impl fmt::Display for WorkerMessageId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "worker-message:{}:{}",
            self.scope(),
            self.serial()
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerMessageEndpoint {
    RootOwnerOf(WorkerId),
    Worker(WorkerId),
}

impl WorkerMessageEndpoint {
    pub fn worker(self) -> WorkerId {
        match self {
            Self::RootOwnerOf(worker) | Self::Worker(worker) => worker,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkerMessage {
    id: WorkerMessageId,
    source: WorkerMessageEndpoint,
    destination: WorkerMessageEndpoint,
    payload: WorkerMessagePayload,
    scheduled_task: Option<TaskId>,
}

impl WorkerMessage {
    pub fn id(&self) -> WorkerMessageId {
        self.id
    }

    pub fn source(&self) -> WorkerMessageEndpoint {
        self.source
    }

    pub fn destination(&self) -> WorkerMessageEndpoint {
        self.destination
    }

    pub fn payload(&self) -> &WorkerMessagePayload {
        &self.payload
    }

    pub fn into_payload(self) -> WorkerMessagePayload {
        self.payload
    }

    fn involves(&self, worker: WorkerId) -> bool {
        self.source.worker() == worker || self.destination.worker() == worker
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkerMessageDiscard {
    messages: usize,
    bytes: usize,
}

impl WorkerMessageDiscard {
    pub fn messages(self) -> usize {
        self.messages
    }

    pub fn bytes(self) -> usize {
        self.bytes
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkerMessageError {
    InvalidLimits,
    Worker(WorkerError),
    WorkerNotRunning {
        worker: WorkerId,
        state: WorkerLifecycleState,
    },
    InvalidRoute,
    MessageTooLarge {
        bytes: usize,
        limit: usize,
    },
    MessageItemLimitExceeded {
        items: usize,
        limit: usize,
    },
    MessageDepthLimitExceeded {
        depth: usize,
        limit: usize,
    },
    QueueMessageLimitExceeded,
    QueueByteLimitExceeded,
    DeliveryUnavailable,
    IdentitySpaceExhausted,
}

impl fmt::Display for WorkerMessageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => formatter.write_str("worker message limits are invalid"),
            Self::Worker(error) => error.fmt(formatter),
            Self::WorkerNotRunning { worker, state } => {
                write!(formatter, "worker {worker} is not running ({state:?})")
            }
            Self::InvalidRoute => formatter.write_str("worker message route is not authorized"),
            Self::MessageTooLarge { bytes, limit } => {
                write!(
                    formatter,
                    "worker message requires {bytes} bytes; limit is {limit}"
                )
            }
            Self::MessageItemLimitExceeded { items, limit } => {
                write!(
                    formatter,
                    "worker message requires {items} items; limit is {limit}"
                )
            }
            Self::MessageDepthLimitExceeded { depth, limit } => {
                write!(
                    formatter,
                    "worker message depth {depth} exceeds limit {limit}"
                )
            }
            Self::QueueMessageLimitExceeded => {
                formatter.write_str("worker message queue count limit exceeded")
            }
            Self::QueueByteLimitExceeded => {
                formatter.write_str("worker message queue byte limit exceeded")
            }
            Self::DeliveryUnavailable => {
                formatter.write_str("worker message delivery is unavailable or stale")
            }
            Self::IdentitySpaceExhausted => {
                formatter.write_str("worker message identity space is exhausted")
            }
        }
    }
}

impl std::error::Error for WorkerMessageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Worker(error) => Some(error),
            _ => None,
        }
    }
}

impl From<WorkerError> for WorkerMessageError {
    fn from(error: WorkerError) -> Self {
        Self::Worker(error)
    }
}

#[derive(Debug)]
struct WorkerMessageIdAllocator {
    scope: NonZeroU64,
    next_serial: u64,
}

impl WorkerMessageIdAllocator {
    fn new() -> Result<Self, WorkerMessageError> {
        let scope = NEXT_MESSAGE_SCOPE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| WorkerMessageError::IdentitySpaceExhausted)?;
        let scope = NonZeroU64::new(scope).ok_or(WorkerMessageError::IdentitySpaceExhausted)?;
        Ok(Self {
            scope,
            next_serial: 1,
        })
    }

    fn allocate(&mut self) -> Result<WorkerMessageId, WorkerMessageError> {
        let serial =
            NonZeroU64::new(self.next_serial).ok_or(WorkerMessageError::IdentitySpaceExhausted)?;
        self.next_serial = self.next_serial.checked_add(1).unwrap_or(0);
        Ok(WorkerMessageId {
            scope: self.scope,
            serial,
        })
    }
}

#[derive(Debug)]
pub struct WorkerMessageMailbox {
    limits: WorkerMessageLimits,
    allocator: WorkerMessageIdAllocator,
    messages: VecDeque<WorkerMessage>,
    queued_bytes: usize,
}

impl WorkerMessageMailbox {
    pub fn try_new(limits: WorkerMessageLimits) -> Result<Self, WorkerMessageError> {
        if !limits.is_valid() {
            return Err(WorkerMessageError::InvalidLimits);
        }
        Ok(Self {
            limits,
            allocator: WorkerMessageIdAllocator::new()?,
            messages: VecDeque::new(),
            queued_bytes: 0,
        })
    }

    pub fn with_default_limits() -> Result<Self, WorkerMessageError> {
        Self::try_new(WorkerMessageLimits::default())
    }

    pub fn limits(&self) -> WorkerMessageLimits {
        self.limits
    }

    pub fn queued_messages(&self) -> usize {
        self.messages.len()
    }

    pub fn queued_bytes(&self) -> usize {
        self.queued_bytes
    }

    pub fn send_from_root<O: Eq>(
        &mut self,
        registry: &WorkerRegistry<O>,
        owner: &O,
        worker: WorkerId,
        value: &WorkerMessageValue,
    ) -> Result<WorkerMessageId, WorkerMessageError> {
        ensure_root_owner(registry, owner, worker)?;
        self.enqueue(
            WorkerMessageEndpoint::RootOwnerOf(worker),
            WorkerMessageEndpoint::Worker(worker),
            value,
        )
    }

    pub fn send_to_root<O: Eq>(
        &mut self,
        registry: &WorkerRegistry<O>,
        worker: WorkerId,
        value: &WorkerMessageValue,
    ) -> Result<WorkerMessageId, WorkerMessageError> {
        ensure_root_worker(registry, worker)?;
        self.enqueue(
            WorkerMessageEndpoint::Worker(worker),
            WorkerMessageEndpoint::RootOwnerOf(worker),
            value,
        )
    }

    pub fn send_between_workers<O: Eq>(
        &mut self,
        registry: &WorkerRegistry<O>,
        source: WorkerId,
        destination: WorkerId,
        value: &WorkerMessageValue,
    ) -> Result<WorkerMessageId, WorkerMessageError> {
        ensure_direct_worker_route(registry, source, destination)?;
        self.enqueue(
            WorkerMessageEndpoint::Worker(source),
            WorkerMessageEndpoint::Worker(destination),
            value,
        )
    }

    pub fn receive_for_root<O: Eq>(
        &mut self,
        registry: &WorkerRegistry<O>,
        owner: &O,
        worker: WorkerId,
    ) -> Result<Option<WorkerMessage>, WorkerMessageError> {
        if let Err(error) = ensure_root_owner(registry, owner, worker) {
            self.discard_for_worker(worker);
            return Err(error);
        }
        let destination = WorkerMessageEndpoint::RootOwnerOf(worker);
        let Some(message) = self.remove_first_for(destination) else {
            return Ok(None);
        };
        validate_retained_route(registry, &message)?;
        Ok(Some(message))
    }

    #[cfg(test)]
    fn receive_for_worker_for_test<O: Eq>(
        &mut self,
        registry: &WorkerRegistry<O>,
        worker: WorkerId,
    ) -> Result<Option<WorkerMessage>, WorkerMessageError> {
        if let Err(error) = ensure_running(registry, worker) {
            self.discard_for_worker(worker);
            return Err(error);
        }
        let destination = WorkerMessageEndpoint::Worker(worker);
        let Some(message) = self.remove_first_for(destination) else {
            return Ok(None);
        };
        validate_retained_route(registry, &message)?;
        Ok(Some(message))
    }

    pub(crate) fn next_for_worker_delivery<O: Eq>(
        &mut self,
        registry: &WorkerRegistry<O>,
        worker: WorkerId,
    ) -> Result<Option<WorkerMessageId>, WorkerMessageError> {
        ensure_running(registry, worker)?;
        let destination = WorkerMessageEndpoint::Worker(worker);
        let Some(position) = self
            .messages
            .iter()
            .position(|message| message.destination == destination)
        else {
            return Ok(None);
        };
        if self.messages[position].scheduled_task.is_some() {
            return Ok(None);
        }
        if let Err(error) = validate_retained_route(registry, &self.messages[position]) {
            self.remove_at(position);
            return Err(error);
        }
        Ok(Some(self.messages[position].id))
    }

    pub(crate) fn mark_scheduled(
        &mut self,
        worker: WorkerId,
        message: WorkerMessageId,
        task: TaskId,
    ) -> Result<(), WorkerMessageError> {
        let destination = WorkerMessageEndpoint::Worker(worker);
        let queued = self
            .messages
            .iter_mut()
            .find(|queued| queued.id == message && queued.destination == destination)
            .ok_or(WorkerMessageError::DeliveryUnavailable)?;
        if queued.scheduled_task.is_some() {
            return Err(WorkerMessageError::DeliveryUnavailable);
        }
        queued.scheduled_task = Some(task);
        Ok(())
    }

    pub(crate) fn message_for_delivery<O: Eq>(
        &self,
        registry: &WorkerRegistry<O>,
        worker: WorkerId,
        message: WorkerMessageId,
        task: TaskId,
    ) -> Result<&WorkerMessage, WorkerMessageError> {
        ensure_running(registry, worker)?;
        let destination = WorkerMessageEndpoint::Worker(worker);
        let queued = self
            .messages
            .iter()
            .find(|queued| queued.id == message && queued.destination == destination)
            .ok_or(WorkerMessageError::DeliveryUnavailable)?;
        if queued.scheduled_task != Some(task) {
            return Err(WorkerMessageError::DeliveryUnavailable);
        }
        validate_retained_route(registry, queued)?;
        Ok(queued)
    }

    pub(crate) fn complete_delivery(
        &mut self,
        worker: WorkerId,
        message: WorkerMessageId,
        task: TaskId,
    ) -> Result<(), WorkerMessageError> {
        let destination = WorkerMessageEndpoint::Worker(worker);
        let position = self
            .messages
            .iter()
            .position(|queued| {
                queued.id == message
                    && queued.destination == destination
                    && queued.scheduled_task == Some(task)
            })
            .ok_or(WorkerMessageError::DeliveryUnavailable)?;
        self.remove_at(position);
        Ok(())
    }

    pub fn discard_for_worker(&mut self, worker: WorkerId) -> WorkerMessageDiscard {
        let mut messages = 0usize;
        let mut bytes = 0usize;
        self.messages.retain(|message| {
            if message.involves(worker) {
                messages = messages
                    .checked_add(1)
                    .expect("worker message count must fit configured queue limits");
                bytes = bytes
                    .checked_add(message.payload.retained_bytes())
                    .expect("worker message bytes must fit configured queue limits");
                false
            } else {
                true
            }
        });
        self.queued_bytes = self
            .queued_bytes
            .checked_sub(bytes)
            .expect("worker message byte accounting must match retained messages");
        WorkerMessageDiscard { messages, bytes }
    }

    fn enqueue(
        &mut self,
        source: WorkerMessageEndpoint,
        destination: WorkerMessageEndpoint,
        value: &WorkerMessageValue,
    ) -> Result<WorkerMessageId, WorkerMessageError> {
        if self.messages.len() >= self.limits.max_queued_messages {
            return Err(WorkerMessageError::QueueMessageLimitExceeded);
        }
        let metrics = WorkerMessagePayload::measure(value, self.limits)?;
        let queued_bytes = self
            .queued_bytes
            .checked_add(metrics.bytes)
            .ok_or(WorkerMessageError::QueueByteLimitExceeded)?;
        if queued_bytes > self.limits.max_queued_bytes {
            return Err(WorkerMessageError::QueueByteLimitExceeded);
        }
        let id = self.allocator.allocate()?;
        let payload = WorkerMessagePayload::clone_with_metrics(value, metrics);
        self.messages.push_back(WorkerMessage {
            id,
            source,
            destination,
            payload,
            scheduled_task: None,
        });
        self.queued_bytes = queued_bytes;
        Ok(id)
    }

    fn remove_first_for(&mut self, destination: WorkerMessageEndpoint) -> Option<WorkerMessage> {
        let position = self
            .messages
            .iter()
            .position(|message| message.destination == destination)?;
        self.remove_at(position)
    }

    fn remove_at(&mut self, position: usize) -> Option<WorkerMessage> {
        let message = self.messages.remove(position)?;
        self.queued_bytes = self
            .queued_bytes
            .checked_sub(message.payload.retained_bytes())
            .expect("worker message byte accounting must match retained messages");
        Some(message)
    }
}

fn ensure_running<O: Eq>(
    registry: &WorkerRegistry<O>,
    worker: WorkerId,
) -> Result<(), WorkerMessageError> {
    let state = registry.state(worker)?;
    if state != WorkerLifecycleState::Running {
        return Err(WorkerMessageError::WorkerNotRunning { worker, state });
    }
    Ok(())
}

fn ensure_root_worker<O: Eq>(
    registry: &WorkerRegistry<O>,
    worker: WorkerId,
) -> Result<(), WorkerMessageError> {
    ensure_running(registry, worker)?;
    if matches!(registry.owner(worker)?, WorkerOwner::Root(_)) {
        Ok(())
    } else {
        Err(WorkerMessageError::InvalidRoute)
    }
}

fn ensure_root_owner<O: Eq>(
    registry: &WorkerRegistry<O>,
    owner: &O,
    worker: WorkerId,
) -> Result<(), WorkerMessageError> {
    ensure_running(registry, worker)?;
    match registry.owner(worker)? {
        WorkerOwner::Root(candidate) if candidate == owner => Ok(()),
        _ => Err(WorkerMessageError::InvalidRoute),
    }
}

fn ensure_direct_worker_route<O: Eq>(
    registry: &WorkerRegistry<O>,
    source: WorkerId,
    destination: WorkerId,
) -> Result<(), WorkerMessageError> {
    if source == destination {
        return Err(WorkerMessageError::InvalidRoute);
    }
    ensure_running(registry, source)?;
    ensure_running(registry, destination)?;
    let source_parent = registry.owner(source)?.parent_worker();
    let destination_parent = registry.owner(destination)?.parent_worker();
    if source_parent == Some(destination) || destination_parent == Some(source) {
        Ok(())
    } else {
        Err(WorkerMessageError::InvalidRoute)
    }
}

fn validate_retained_route<O: Eq>(
    registry: &WorkerRegistry<O>,
    message: &WorkerMessage,
) -> Result<(), WorkerMessageError> {
    match (message.source, message.destination) {
        (WorkerMessageEndpoint::RootOwnerOf(root), WorkerMessageEndpoint::Worker(worker))
            if root == worker =>
        {
            ensure_root_worker(registry, worker)
        }
        (WorkerMessageEndpoint::Worker(worker), WorkerMessageEndpoint::RootOwnerOf(root))
            if root == worker =>
        {
            ensure_root_worker(registry, worker)
        }
        (WorkerMessageEndpoint::Worker(source), WorkerMessageEndpoint::Worker(destination)) => {
            ensure_direct_worker_route(registry, source, destination)
        }
        _ => Err(WorkerMessageError::InvalidRoute),
    }
}

fn measure_payload(
    root: &WorkerMessageValue,
    limits: WorkerMessageLimits,
) -> Result<PayloadMetrics, WorkerMessageError> {
    let mut stack = vec![(root, 1usize)];
    let mut bytes = 0usize;
    let mut items = 0usize;
    let mut depth = 1usize;

    while let Some((value, current_depth)) = stack.pop() {
        if current_depth > limits.max_message_depth {
            return Err(WorkerMessageError::MessageDepthLimitExceeded {
                depth: current_depth,
                limit: limits.max_message_depth,
            });
        }
        depth = depth.max(current_depth);
        items = items
            .checked_add(1)
            .ok_or(WorkerMessageError::MessageItemLimitExceeded {
                items: usize::MAX,
                limit: limits.max_message_items,
            })?;
        ensure_item_limit(items, limits)?;
        bytes = add_payload_bytes(bytes, 1, limits)?;

        match value {
            WorkerMessageValue::Null => {}
            WorkerMessageValue::Boolean(_) => {
                bytes = add_payload_bytes(bytes, 1, limits)?;
            }
            WorkerMessageValue::Number(_) => {
                bytes = add_payload_bytes(bytes, std::mem::size_of::<u64>(), limits)?;
            }
            WorkerMessageValue::String(text) => {
                bytes = add_payload_bytes(bytes, text.len(), limits)?;
            }
            WorkerMessageValue::Bytes(data) => {
                bytes = add_payload_bytes(bytes, data.len(), limits)?;
            }
            WorkerMessageValue::Array(values) => {
                if !values.is_empty() {
                    let child_depth = current_depth.checked_add(1).ok_or(
                        WorkerMessageError::MessageDepthLimitExceeded {
                            depth: usize::MAX,
                            limit: limits.max_message_depth,
                        },
                    )?;
                    if child_depth > limits.max_message_depth {
                        return Err(WorkerMessageError::MessageDepthLimitExceeded {
                            depth: child_depth,
                            limit: limits.max_message_depth,
                        });
                    }
                    if values.len() > remaining_items(limits.max_message_items, items)? {
                        return Err(WorkerMessageError::MessageItemLimitExceeded {
                            items: requested_items(items, values.len()),
                            limit: limits.max_message_items,
                        });
                    }
                    stack.extend(values.iter().rev().map(|value| (value, child_depth)));
                }
            }
            WorkerMessageValue::Object(fields) => {
                if !fields.is_empty() {
                    let child_depth = current_depth.checked_add(1).ok_or(
                        WorkerMessageError::MessageDepthLimitExceeded {
                            depth: usize::MAX,
                            limit: limits.max_message_depth,
                        },
                    )?;
                    if child_depth > limits.max_message_depth {
                        return Err(WorkerMessageError::MessageDepthLimitExceeded {
                            depth: child_depth,
                            limit: limits.max_message_depth,
                        });
                    }
                    if fields.len() > remaining_items(limits.max_message_items, items)? {
                        return Err(WorkerMessageError::MessageItemLimitExceeded {
                            items: requested_items(items, fields.len()),
                            limit: limits.max_message_items,
                        });
                    }
                    for (key, value) in fields.iter().rev() {
                        items = items.checked_add(1).ok_or(
                            WorkerMessageError::MessageItemLimitExceeded {
                                items: usize::MAX,
                                limit: limits.max_message_items,
                            },
                        )?;
                        ensure_item_limit(items, limits)?;
                        bytes = add_payload_bytes(bytes, 1, limits)?;
                        bytes = add_payload_bytes(bytes, key.len(), limits)?;
                        stack.push((value, child_depth));
                    }
                }
            }
        }
    }

    Ok(PayloadMetrics {
        bytes,
        items,
        depth,
    })
}

fn remaining_items(limit: usize, used: usize) -> Result<usize, WorkerMessageError> {
    match limit.checked_sub(used) {
        Some(remaining) => Ok(remaining),
        None => Err(WorkerMessageError::MessageItemLimitExceeded { items: used, limit }),
    }
}

fn requested_items(current: usize, additional: usize) -> usize {
    match current.checked_add(additional) {
        Some(requested) => requested,
        None => usize::MAX,
    }
}

fn ensure_item_limit(items: usize, limits: WorkerMessageLimits) -> Result<(), WorkerMessageError> {
    if items > limits.max_message_items {
        Err(WorkerMessageError::MessageItemLimitExceeded {
            items,
            limit: limits.max_message_items,
        })
    } else {
        Ok(())
    }
}

fn add_payload_bytes(
    current: usize,
    additional: usize,
    limits: WorkerMessageLimits,
) -> Result<usize, WorkerMessageError> {
    let bytes = current
        .checked_add(additional)
        .ok_or(WorkerMessageError::MessageTooLarge {
            bytes: usize::MAX,
            limit: limits.max_message_bytes,
        })?;
    if bytes > limits.max_message_bytes {
        Err(WorkerMessageError::MessageTooLarge {
            bytes,
            limit: limits.max_message_bytes,
        })
    } else {
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{WorkerErrorKind, WorkerLimits};

    fn limits(
        max_message_bytes: usize,
        max_message_items: usize,
        max_message_depth: usize,
        max_queued_messages: usize,
        max_queued_bytes: usize,
    ) -> WorkerMessageLimits {
        WorkerMessageLimits {
            max_message_bytes,
            max_message_items,
            max_message_depth,
            max_queued_messages,
            max_queued_bytes,
        }
    }

    fn worker_tree() -> (WorkerRegistry<u8>, WorkerId, WorkerId, WorkerId) {
        let mut registry = WorkerRegistry::try_new(WorkerLimits::default()).unwrap();
        let parent = registry.create_root(7).unwrap();
        registry.mark_running(parent).unwrap();
        let first = registry.create_child(parent).unwrap();
        registry.mark_running(first).unwrap();
        let second = registry.create_child(parent).unwrap();
        registry.mark_running(second).unwrap();
        (registry, parent, first, second)
    }

    #[test]
    fn default_limits_are_bounded_and_valid() {
        let limits = WorkerMessageLimits::default();
        assert!(limits.is_valid());
        assert!(limits.max_message_depth <= HARD_MAX_WORKER_MESSAGE_DEPTH);
        assert!(limits.max_message_bytes <= limits.max_queued_bytes);
    }

    #[test]
    fn invalid_limits_fail_before_mailbox_creation() {
        assert_eq!(
            WorkerMessageMailbox::try_new(limits(8, 8, 0, 1, 8)).unwrap_err(),
            WorkerMessageError::InvalidLimits
        );
        assert_eq!(
            WorkerMessageMailbox::try_new(limits(8, 8, HARD_MAX_WORKER_MESSAGE_DEPTH + 1, 1, 8,))
                .unwrap_err(),
            WorkerMessageError::InvalidLimits
        );
        assert_eq!(
            WorkerMessageMailbox::try_new(limits(9, 8, 4, 1, 8)).unwrap_err(),
            WorkerMessageError::InvalidLimits
        );
    }

    #[test]
    fn structured_payload_is_owned_and_exact_number_bits_are_preserved() {
        let (registry, parent, child, _) = worker_tree();
        let mut mailbox = WorkerMessageMailbox::with_default_limits().unwrap();
        let mut fields = BTreeMap::new();
        fields.insert(
            String::from("text"),
            WorkerMessageValue::String(String::from("original")),
        );
        fields.insert(
            String::from("number"),
            WorkerMessageValue::Number(WorkerMessageNumber::from_f64(-0.0)),
        );
        let mut value = WorkerMessageValue::Object(fields);
        let id = mailbox
            .send_between_workers(&registry, parent, child, &value)
            .unwrap();
        let WorkerMessageValue::Object(fields) = &mut value else {
            panic!("expected object");
        };
        fields.insert(
            String::from("text"),
            WorkerMessageValue::String(String::from("changed")),
        );

        let message = mailbox
            .receive_for_worker_for_test(&registry, child)
            .unwrap()
            .unwrap();
        assert_eq!(message.id(), id);
        let WorkerMessageValue::Object(fields) = message.payload().value() else {
            panic!("expected object payload");
        };
        assert_eq!(
            fields.get("text"),
            Some(&WorkerMessageValue::String(String::from("original")))
        );
        let Some(WorkerMessageValue::Number(number)) = fields.get("number") else {
            panic!("expected number");
        };
        assert_eq!(number.bits(), (-0.0f64).to_bits());
    }

    #[test]
    fn root_owner_route_requires_the_exact_external_owner() {
        let mut registry = WorkerRegistry::try_new(WorkerLimits::default()).unwrap();
        let worker = registry.create_root(9).unwrap();
        registry.mark_running(worker).unwrap();
        let mut mailbox = WorkerMessageMailbox::with_default_limits().unwrap();

        let error = mailbox
            .send_from_root(&registry, &8, worker, &WorkerMessageValue::Null)
            .unwrap_err();
        assert_eq!(error, WorkerMessageError::InvalidRoute);
        assert_eq!(mailbox.queued_messages(), 0);

        mailbox
            .send_from_root(&registry, &9, worker, &WorkerMessageValue::Null)
            .unwrap();
        let inbound = mailbox
            .receive_for_worker_for_test(&registry, worker)
            .unwrap()
            .unwrap();
        assert_eq!(inbound.source(), WorkerMessageEndpoint::RootOwnerOf(worker));

        mailbox
            .send_to_root(&registry, worker, &WorkerMessageValue::Boolean(true))
            .unwrap();
        let outbound = mailbox
            .receive_for_root(&registry, &9, worker)
            .unwrap()
            .unwrap();
        assert_eq!(outbound.source(), WorkerMessageEndpoint::Worker(worker));
    }

    #[test]
    fn arbitrary_or_sibling_worker_routes_are_rejected() {
        let (registry, parent, first, second) = worker_tree();
        let mut mailbox = WorkerMessageMailbox::with_default_limits().unwrap();
        let error = mailbox
            .send_between_workers(&registry, first, second, &WorkerMessageValue::Null)
            .unwrap_err();
        assert_eq!(error, WorkerMessageError::InvalidRoute);
        let error = mailbox
            .send_between_workers(&registry, parent, parent, &WorkerMessageValue::Null)
            .unwrap_err();
        assert_eq!(error, WorkerMessageError::InvalidRoute);
        assert_eq!(mailbox.queued_messages(), 0);
    }

    #[test]
    fn payload_depth_items_and_bytes_are_bounded_before_enqueue() {
        let (registry, parent, child, _) = worker_tree();
        let mut mailbox = WorkerMessageMailbox::try_new(limits(8, 4, 2, 4, 16)).unwrap();

        let too_deep = WorkerMessageValue::Array(vec![WorkerMessageValue::Array(vec![
            WorkerMessageValue::Null,
        ])]);
        assert_eq!(
            mailbox
                .send_between_workers(&registry, parent, child, &too_deep)
                .unwrap_err(),
            WorkerMessageError::MessageDepthLimitExceeded { depth: 3, limit: 2 }
        );

        let too_many = WorkerMessageValue::Array(vec![
            WorkerMessageValue::Null,
            WorkerMessageValue::Null,
            WorkerMessageValue::Null,
            WorkerMessageValue::Null,
        ]);
        let error = mailbox
            .send_between_workers(&registry, parent, child, &too_many)
            .unwrap_err();
        assert!(matches!(
            error,
            WorkerMessageError::MessageItemLimitExceeded { limit: 4, .. }
        ));

        let too_large = WorkerMessageValue::String(String::from("12345678"));
        assert_eq!(
            mailbox
                .send_between_workers(&registry, parent, child, &too_large)
                .unwrap_err(),
            WorkerMessageError::MessageTooLarge { bytes: 9, limit: 8 }
        );
        assert_eq!(mailbox.queued_messages(), 0);
        assert_eq!(mailbox.queued_bytes(), 0);
    }

    #[test]
    fn message_and_byte_backpressure_recover_after_delivery() {
        let (registry, parent, child, _) = worker_tree();
        let mut mailbox = WorkerMessageMailbox::try_new(limits(3, 8, 4, 2, 6)).unwrap();
        let value = WorkerMessageValue::String(String::from("ab"));
        mailbox
            .send_between_workers(&registry, parent, child, &value)
            .unwrap();
        mailbox
            .send_between_workers(&registry, parent, child, &value)
            .unwrap();
        assert_eq!(mailbox.queued_messages(), 2);
        assert_eq!(mailbox.queued_bytes(), 6);
        assert_eq!(
            mailbox
                .send_between_workers(&registry, parent, child, &WorkerMessageValue::Null)
                .unwrap_err(),
            WorkerMessageError::QueueMessageLimitExceeded
        );

        mailbox
            .receive_for_worker_for_test(&registry, child)
            .unwrap();
        assert_eq!(mailbox.queued_messages(), 1);
        assert_eq!(mailbox.queued_bytes(), 3);
        mailbox
            .send_between_workers(&registry, parent, child, &value)
            .unwrap();
        assert_eq!(mailbox.queued_messages(), 2);
        assert_eq!(mailbox.queued_bytes(), 6);
    }

    #[test]
    fn global_byte_backpressure_is_explicit() {
        let (registry, parent, child, _) = worker_tree();
        let mut mailbox = WorkerMessageMailbox::try_new(limits(3, 8, 4, 4, 5)).unwrap();
        let value = WorkerMessageValue::String(String::from("ab"));
        mailbox
            .send_between_workers(&registry, parent, child, &value)
            .unwrap();
        assert_eq!(mailbox.queued_bytes(), 3);
        assert_eq!(
            mailbox
                .send_between_workers(&registry, child, parent, &value)
                .unwrap_err(),
            WorkerMessageError::QueueByteLimitExceeded
        );
        assert_eq!(mailbox.queued_messages(), 1);
        assert_eq!(mailbox.queued_bytes(), 3);
    }

    #[test]
    fn fifo_is_preserved_per_destination_with_interleaved_messages() {
        let (registry, parent, child, _) = worker_tree();
        let mut mailbox = WorkerMessageMailbox::with_default_limits().unwrap();
        let first = mailbox
            .send_between_workers(
                &registry,
                parent,
                child,
                &WorkerMessageValue::String(String::from("first")),
            )
            .unwrap();
        mailbox
            .send_between_workers(
                &registry,
                child,
                parent,
                &WorkerMessageValue::String(String::from("other direction")),
            )
            .unwrap();
        let second = mailbox
            .send_between_workers(
                &registry,
                parent,
                child,
                &WorkerMessageValue::String(String::from("second")),
            )
            .unwrap();

        assert_eq!(
            mailbox
                .receive_for_worker_for_test(&registry, child)
                .unwrap()
                .unwrap()
                .id(),
            first
        );
        assert_eq!(
            mailbox
                .receive_for_worker_for_test(&registry, child)
                .unwrap()
                .unwrap()
                .id(),
            second
        );
        assert_eq!(mailbox.queued_messages(), 1);
    }

    #[test]
    fn closing_source_message_is_discarded_before_delivery() {
        let (mut registry, parent, child, _) = worker_tree();
        let mut mailbox = WorkerMessageMailbox::with_default_limits().unwrap();
        mailbox
            .send_between_workers(&registry, child, parent, &WorkerMessageValue::Null)
            .unwrap();
        registry.begin_close(child).unwrap();

        assert_eq!(
            mailbox
                .receive_for_worker_for_test(&registry, parent)
                .unwrap_err(),
            WorkerMessageError::WorkerNotRunning {
                worker: child,
                state: WorkerLifecycleState::Closing,
            }
        );
        assert_eq!(mailbox.queued_messages(), 0);
        assert_eq!(mailbox.queued_bytes(), 0);
    }

    #[test]
    fn closing_destination_discards_its_inaccessible_pending_messages() {
        let (mut registry, parent, child, _) = worker_tree();
        let mut mailbox = WorkerMessageMailbox::with_default_limits().unwrap();
        mailbox
            .send_between_workers(&registry, parent, child, &WorkerMessageValue::Null)
            .unwrap();
        registry.begin_close(child).unwrap();

        assert_eq!(
            mailbox
                .receive_for_worker_for_test(&registry, child)
                .unwrap_err(),
            WorkerMessageError::WorkerNotRunning {
                worker: child,
                state: WorkerLifecycleState::Closing,
            }
        );
        assert_eq!(mailbox.queued_messages(), 0);
        assert_eq!(mailbox.queued_bytes(), 0);
    }

    #[test]
    fn foreign_or_retired_worker_identity_cannot_drive_mailbox() {
        let (mut registry, parent, child, _) = worker_tree();
        let foreign = WorkerRegistry::<u8>::try_new(WorkerLimits::default()).unwrap();
        let mut mailbox = WorkerMessageMailbox::with_default_limits().unwrap();

        let error = mailbox
            .send_between_workers(&foreign, parent, child, &WorkerMessageValue::Null)
            .unwrap_err();
        let WorkerMessageError::Worker(error) = error else {
            panic!("expected exact-registry rejection");
        };
        assert_eq!(error.kind, WorkerErrorKind::UnknownWorker);

        registry.begin_close(child).unwrap();
        registry.retire(child).unwrap();
        let error = mailbox
            .send_between_workers(&registry, parent, child, &WorkerMessageValue::Null)
            .unwrap_err();
        let WorkerMessageError::Worker(error) = error else {
            panic!("expected retired-worker rejection");
        };
        assert_eq!(error.kind, WorkerErrorKind::UnknownWorker);
        assert_eq!(mailbox.queued_messages(), 0);
    }

    #[test]
    fn explicit_discard_releases_only_messages_involving_the_worker() {
        let (registry, parent, first, second) = worker_tree();
        let mut mailbox = WorkerMessageMailbox::with_default_limits().unwrap();
        mailbox
            .send_between_workers(&registry, parent, first, &WorkerMessageValue::Null)
            .unwrap();
        mailbox
            .send_between_workers(
                &registry,
                parent,
                second,
                &WorkerMessageValue::Boolean(true),
            )
            .unwrap();
        let before = mailbox.queued_bytes();
        let discard = mailbox.discard_for_worker(first);
        assert_eq!(discard.messages(), 1);
        assert!(discard.bytes() > 0);
        assert_eq!(mailbox.queued_messages(), 1);
        assert_eq!(mailbox.queued_bytes(), before - discard.bytes());
        assert!(
            mailbox
                .receive_for_worker_for_test(&registry, second)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn mailbox_message_ids_do_not_alias_across_instances() {
        let (registry, parent, child, _) = worker_tree();
        let mut first = WorkerMessageMailbox::with_default_limits().unwrap();
        let mut second = WorkerMessageMailbox::with_default_limits().unwrap();
        let first_id = first
            .send_between_workers(&registry, parent, child, &WorkerMessageValue::Null)
            .unwrap();
        let second_id = second
            .send_between_workers(&registry, parent, child, &WorkerMessageValue::Null)
            .unwrap();
        assert_ne!(first_id, second_id);
    }
}
