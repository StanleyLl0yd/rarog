use crate::state::{StorageError, StorageErrorKind, StorageLimits, StorageProcessState};
use rarog_process::StorageProcessId;
use rarog_url::Origin;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::num::NonZeroU64;

pub const DEFAULT_MAX_STORAGE_REQUEST_BYTES: usize = 1024 * 1024;
pub const DEFAULT_MAX_PENDING_STORAGE_REQUESTS: usize = 256;
pub const DEFAULT_MAX_PENDING_STORAGE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageRequestLimits {
    pub max_request_bytes: usize,
    pub max_pending_requests: usize,
    pub max_pending_bytes: usize,
}

impl StorageRequestLimits {
    pub fn is_valid(self) -> bool {
        self.max_request_bytes > 0
            && self.max_pending_requests > 0
            && self.max_pending_bytes > 0
            && self.max_request_bytes <= self.max_pending_bytes
    }
}

impl Default for StorageRequestLimits {
    fn default() -> Self {
        Self {
            max_request_bytes: DEFAULT_MAX_STORAGE_REQUEST_BYTES,
            max_pending_requests: DEFAULT_MAX_PENDING_STORAGE_REQUESTS,
            max_pending_bytes: DEFAULT_MAX_PENDING_STORAGE_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageProtocolErrorKind {
    InvalidLimits,
    InvalidRequestId,
    RequestIdentitySpaceExhausted,
    OpaqueOrigin,
    KeyTooLarge,
    ValueTooLarge,
    RequestTooLarge,
    PendingRequestLimitExceeded,
    PendingByteLimitExceeded,
    UnknownRequest,
    WrongProcess,
    ResponseMismatch,
    AccountingOverflow,
    AllocationFailed,
    Storage(StorageErrorKind),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageProtocolError {
    pub kind: StorageProtocolErrorKind,
    pub message: String,
}

impl StorageProtocolError {
    fn new(kind: StorageProtocolErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for StorageProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for StorageProtocolError {}

impl From<StorageError> for StorageProtocolError {
    fn from(error: StorageError) -> Self {
        Self::new(StorageProtocolErrorKind::Storage(error.kind), error.message)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StorageRequestId(NonZeroU64);

impl StorageRequestId {
    pub fn try_new(raw: u64) -> Result<Self, StorageProtocolError> {
        NonZeroU64::new(raw).map(Self).ok_or_else(|| {
            StorageProtocolError::new(
                StorageProtocolErrorKind::InvalidRequestId,
                "storage request identity must be non-zero",
            )
        })
    }

    pub fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Display for StorageRequestId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "storage-request:{}", self.get())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageOperationKind {
    Get,
    Put,
    Remove,
    Clear,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageCommand {
    Get { key: String },
    Put { key: String, value: Vec<u8> },
    Remove { key: String },
    Clear,
}

impl StorageCommand {
    pub fn kind(&self) -> StorageOperationKind {
        match self {
            Self::Get { .. } => StorageOperationKind::Get,
            Self::Put { .. } => StorageOperationKind::Put,
            Self::Remove { .. } => StorageOperationKind::Remove,
            Self::Clear => StorageOperationKind::Clear,
        }
    }

    fn key_len(&self) -> usize {
        match self {
            Self::Get { key } | Self::Put { key, .. } | Self::Remove { key } => key.len(),
            Self::Clear => 0,
        }
    }

    fn value_len(&self) -> usize {
        match self {
            Self::Put { value, .. } => value.len(),
            _ => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageRequest {
    id: StorageRequestId,
    process: StorageProcessId,
    origin: Origin,
    command: StorageCommand,
    accounted_bytes: usize,
}

impl StorageRequest {
    pub fn id(&self) -> StorageRequestId {
        self.id
    }

    pub fn process(&self) -> StorageProcessId {
        self.process
    }

    pub fn origin(&self) -> &Origin {
        &self.origin
    }

    pub fn command(&self) -> &StorageCommand {
        &self.command
    }

    pub fn kind(&self) -> StorageOperationKind {
        self.command.kind()
    }

    pub fn accounted_bytes(&self) -> usize {
        self.accounted_bytes
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageResponsePayload {
    Value(Option<Vec<u8>>),
    Written,
    Removed(bool),
    Cleared(bool),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageResponse {
    request: StorageRequestId,
    process: StorageProcessId,
    kind: StorageOperationKind,
    payload: StorageResponsePayload,
}

impl StorageResponse {
    pub fn request(&self) -> StorageRequestId {
        self.request
    }

    pub fn process(&self) -> StorageProcessId {
        self.process
    }

    pub fn kind(&self) -> StorageOperationKind {
        self.kind
    }

    pub fn payload(&self) -> &StorageResponsePayload {
        &self.payload
    }

    pub fn into_payload(self) -> StorageResponsePayload {
        self.payload
    }
}

#[derive(Clone, Copy, Debug)]
struct PendingRequest {
    process: StorageProcessId,
    kind: StorageOperationKind,
    accounted_bytes: usize,
}

#[derive(Debug)]
pub struct StorageRequestQueue {
    process: StorageProcessId,
    storage_limits: StorageLimits,
    limits: StorageRequestLimits,
    next_request: Option<NonZeroU64>,
    pending: HashMap<StorageRequestId, PendingRequest>,
    queued: VecDeque<StorageRequest>,
    tracked_bytes: usize,
}

impl StorageRequestQueue {
    pub fn try_new(
        process: StorageProcessId,
        storage_limits: StorageLimits,
        limits: StorageRequestLimits,
    ) -> Result<Self, StorageProtocolError> {
        if !storage_limits.is_valid() || !limits.is_valid() {
            return Err(StorageProtocolError::new(
                StorageProtocolErrorKind::InvalidLimits,
                "storage request and storage-state limits must be valid",
            ));
        }
        Ok(Self {
            process,
            storage_limits,
            limits,
            next_request: NonZeroU64::new(1),
            pending: HashMap::new(),
            queued: VecDeque::new(),
            tracked_bytes: 0,
        })
    }

    pub fn process(&self) -> StorageProcessId {
        self.process
    }

    pub fn pending_requests(&self) -> usize {
        self.pending.len()
    }

    pub fn queued_requests(&self) -> usize {
        self.queued.len()
    }

    pub fn tracked_bytes(&self) -> usize {
        self.tracked_bytes
    }

    pub fn enqueue(
        &mut self,
        origin: Origin,
        command: StorageCommand,
    ) -> Result<StorageRequestId, StorageProtocolError> {
        let accounted_bytes = self.validate_and_measure(&origin, &command)?;
        if self.pending.len() >= self.limits.max_pending_requests {
            return Err(StorageProtocolError::new(
                StorageProtocolErrorKind::PendingRequestLimitExceeded,
                format!(
                    "storage pending-request limit {} reached",
                    self.limits.max_pending_requests
                ),
            ));
        }
        let next_bytes = self
            .tracked_bytes
            .checked_add(accounted_bytes)
            .ok_or_else(|| {
                StorageProtocolError::new(
                    StorageProtocolErrorKind::AccountingOverflow,
                    "storage pending-byte accounting overflow",
                )
            })?;
        if next_bytes > self.limits.max_pending_bytes {
            return Err(StorageProtocolError::new(
                StorageProtocolErrorKind::PendingByteLimitExceeded,
                format!(
                    "storage pending requests would require {next_bytes} bytes; limit is {}",
                    self.limits.max_pending_bytes
                ),
            ));
        }

        let raw = self.next_request.ok_or_else(|| {
            StorageProtocolError::new(
                StorageProtocolErrorKind::RequestIdentitySpaceExhausted,
                "storage request identity space is exhausted",
            )
        })?;
        self.next_request = NonZeroU64::new(raw.get().wrapping_add(1));
        let id = StorageRequestId(raw);
        if self.pending.contains_key(&id) {
            return Err(StorageProtocolError::new(
                StorageProtocolErrorKind::AccountingOverflow,
                "storage request allocator reused a live identity",
            ));
        }

        self.pending
            .try_reserve(1)
            .map_err(|_| allocation_error())?;
        self.queued
            .try_reserve(1)
            .map_err(|_| allocation_error())?;

        let kind = command.kind();
        let request = StorageRequest {
            id,
            process: self.process,
            origin,
            command,
            accounted_bytes,
        };
        self.pending.insert(
            id,
            PendingRequest {
                process: self.process,
                kind,
                accounted_bytes,
            },
        );
        self.queued.push_back(request);
        self.tracked_bytes = next_bytes;
        Ok(id)
    }

    pub fn take_next(&mut self) -> Option<StorageRequest> {
        self.queued.pop_front()
    }

    pub fn cancel(&mut self, id: StorageRequestId) -> Result<bool, StorageProtocolError> {
        let Some(pending) = self.pending.remove(&id) else {
            return Ok(false);
        };
        if let Some(position) = self.queued.iter().position(|request| request.id == id) {
            self.queued.remove(position);
        }
        self.tracked_bytes = self
            .tracked_bytes
            .checked_sub(pending.accounted_bytes)
            .ok_or_else(|| {
                StorageProtocolError::new(
                    StorageProtocolErrorKind::AccountingOverflow,
                    "storage pending-byte accounting underflow",
                )
            })?;
        Ok(true)
    }

    pub fn complete(
        &mut self,
        response: StorageResponse,
    ) -> Result<StorageResponsePayload, StorageProtocolError> {
        let pending = self.pending.get(&response.request).copied().ok_or_else(|| {
            StorageProtocolError::new(
                StorageProtocolErrorKind::UnknownRequest,
                format!("unknown, cancelled or completed storage request {}", response.request),
            )
        })?;
        if response.process != self.process || response.process != pending.process {
            return Err(StorageProtocolError::new(
                StorageProtocolErrorKind::WrongProcess,
                "storage response process identity does not match Host request authority",
            ));
        }
        if response.kind != pending.kind || !payload_matches_kind(&response.payload, pending.kind) {
            return Err(StorageProtocolError::new(
                StorageProtocolErrorKind::ResponseMismatch,
                "storage response operation does not match the pending request",
            ));
        }

        self.pending.remove(&response.request);
        self.tracked_bytes = self
            .tracked_bytes
            .checked_sub(pending.accounted_bytes)
            .ok_or_else(|| {
                StorageProtocolError::new(
                    StorageProtocolErrorKind::AccountingOverflow,
                    "storage pending-byte accounting underflow",
                )
            })?;
        Ok(response.payload)
    }

    fn validate_and_measure(
        &self,
        origin: &Origin,
        command: &StorageCommand,
    ) -> Result<usize, StorageProtocolError> {
        if origin.is_opaque() {
            return Err(StorageProtocolError::new(
                StorageProtocolErrorKind::OpaqueOrigin,
                "opaque origins do not receive persistent Storage-process requests",
            ));
        }
        let key_len = command.key_len();
        let value_len = command.value_len();
        if key_len > self.storage_limits.max_key_bytes {
            return Err(StorageProtocolError::new(
                StorageProtocolErrorKind::KeyTooLarge,
                format!(
                    "storage request key requires {key_len} bytes; limit is {}",
                    self.storage_limits.max_key_bytes
                ),
            ));
        }
        if value_len > self.storage_limits.max_value_bytes {
            return Err(StorageProtocolError::new(
                StorageProtocolErrorKind::ValueTooLarge,
                format!(
                    "storage request value requires {value_len} bytes; limit is {}",
                    self.storage_limits.max_value_bytes
                ),
            ));
        }
        let origin_len = origin.ascii_serialization().len();
        let bytes = origin_len
            .checked_add(key_len)
            .and_then(|bytes| bytes.checked_add(value_len))
            .ok_or_else(|| {
                StorageProtocolError::new(
                    StorageProtocolErrorKind::AccountingOverflow,
                    "storage request payload accounting overflow",
                )
            })?;
        if bytes > self.limits.max_request_bytes {
            return Err(StorageProtocolError::new(
                StorageProtocolErrorKind::RequestTooLarge,
                format!(
                    "storage request requires {bytes} accounted bytes; limit is {}",
                    self.limits.max_request_bytes
                ),
            ));
        }
        Ok(bytes)
    }
}

pub fn execute_storage_request(
    storage: &mut StorageProcessState,
    request: &StorageRequest,
) -> Result<StorageResponse, StorageProtocolError> {
    if request.process != storage.process() {
        return Err(StorageProtocolError::new(
            StorageProtocolErrorKind::WrongProcess,
            "storage request targets a different Storage-process identity",
        ));
    }
    if request.origin.is_opaque() {
        return Err(StorageProtocolError::new(
            StorageProtocolErrorKind::OpaqueOrigin,
            "opaque origins cannot execute persistent storage requests",
        ));
    }

    let payload = match &request.command {
        StorageCommand::Get { key } => {
            let value = storage
                .get(&request.origin, key)
                .map(try_owned_bytes)
                .transpose()?;
            StorageResponsePayload::Value(value)
        }
        StorageCommand::Put { key, value } => {
            storage.put(&request.origin, key, value)?;
            StorageResponsePayload::Written
        }
        StorageCommand::Remove { key } => {
            StorageResponsePayload::Removed(storage.remove(&request.origin, key)?)
        }
        StorageCommand::Clear => {
            StorageResponsePayload::Cleared(storage.clear_origin(&request.origin)?)
        }
    };

    Ok(StorageResponse {
        request: request.id,
        process: request.process,
        kind: request.kind(),
        payload,
    })
}

fn payload_matches_kind(payload: &StorageResponsePayload, kind: StorageOperationKind) -> bool {
    matches!(
        (payload, kind),
        (StorageResponsePayload::Value(_), StorageOperationKind::Get)
            | (StorageResponsePayload::Written, StorageOperationKind::Put)
            | (StorageResponsePayload::Removed(_), StorageOperationKind::Remove)
            | (StorageResponsePayload::Cleared(_), StorageOperationKind::Clear)
    )
}

fn try_owned_bytes(value: &[u8]) -> Result<Vec<u8>, StorageProtocolError> {
    let mut owned = Vec::new();
    owned
        .try_reserve_exact(value.len())
        .map_err(|_| allocation_error())?;
    owned.extend_from_slice(value);
    Ok(owned)
}

fn allocation_error() -> StorageProtocolError {
    StorageProtocolError::new(
        StorageProtocolErrorKind::AllocationFailed,
        "storage request allocation failed within configured bounds",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_process::ProcessTopology;
    use rarog_url::WebUrl;

    fn origin(url: &str) -> Origin {
        WebUrl::parse(url).unwrap().origin().unwrap()
    }

    fn process() -> StorageProcessId {
        ProcessTopology::try_new(1)
            .unwrap()
            .ensure_storage_process()
            .unwrap()
            .process()
    }

    fn different_process() -> StorageProcessId {
        let mut topology = ProcessTopology::try_new(1).unwrap();
        topology
            .assign_site(WebUrl::parse("https://example.org/").unwrap().site_identity().unwrap())
            .unwrap();
        topology.ensure_storage_process().unwrap().process()
    }

    fn storage_limits() -> StorageLimits {
        StorageLimits {
            max_origins: 4,
            max_entries_per_origin: 8,
            max_key_bytes: 16,
            max_value_bytes: 32,
            max_origin_bytes: 128,
            max_total_bytes: 256,
        }
    }

    fn request_limits() -> StorageRequestLimits {
        StorageRequestLimits {
            max_request_bytes: 96,
            max_pending_requests: 4,
            max_pending_bytes: 192,
        }
    }

    #[test]
    fn typed_request_response_round_trips_all_operations() {
        let process = process();
        let origin = origin("https://example.com/");
        let mut storage = StorageProcessState::try_new(process, storage_limits()).unwrap();
        let mut queue = StorageRequestQueue::try_new(process, storage_limits(), request_limits())
            .unwrap();

        queue
            .enqueue(
                origin.clone(),
                StorageCommand::Put {
                    key: String::from("key"),
                    value: b"value".to_vec(),
                },
            )
            .unwrap();
        let request = queue.take_next().unwrap();
        let response = execute_storage_request(&mut storage, &request).unwrap();
        assert_eq!(
            queue.complete(response).unwrap(),
            StorageResponsePayload::Written
        );

        queue
            .enqueue(
                origin.clone(),
                StorageCommand::Get {
                    key: String::from("key"),
                },
            )
            .unwrap();
        let request = queue.take_next().unwrap();
        let response = execute_storage_request(&mut storage, &request).unwrap();
        assert_eq!(
            queue.complete(response).unwrap(),
            StorageResponsePayload::Value(Some(b"value".to_vec()))
        );

        queue
            .enqueue(
                origin.clone(),
                StorageCommand::Remove {
                    key: String::from("key"),
                },
            )
            .unwrap();
        let request = queue.take_next().unwrap();
        let response = execute_storage_request(&mut storage, &request).unwrap();
        assert_eq!(
            queue.complete(response).unwrap(),
            StorageResponsePayload::Removed(true)
        );

        queue.enqueue(origin, StorageCommand::Clear).unwrap();
        let request = queue.take_next().unwrap();
        let response = execute_storage_request(&mut storage, &request).unwrap();
        assert_eq!(
            queue.complete(response).unwrap(),
            StorageResponsePayload::Cleared(false)
        );
        assert_eq!(queue.pending_requests(), 0);
        assert_eq!(queue.tracked_bytes(), 0);
    }

    #[test]
    fn pending_count_and_byte_limits_apply_before_enqueue() {
        let process = process();
        let origin = origin("https://example.com/");
        let mut one = StorageRequestQueue::try_new(
            process,
            storage_limits(),
            StorageRequestLimits {
                max_request_bytes: 64,
                max_pending_requests: 1,
                max_pending_bytes: 64,
            },
        )
        .unwrap();
        one.enqueue(
            origin.clone(),
            StorageCommand::Get {
                key: String::from("a"),
            },
        )
        .unwrap();
        let before = one.tracked_bytes();
        assert_eq!(
            one.enqueue(
                origin.clone(),
                StorageCommand::Get {
                    key: String::from("b"),
                },
            )
            .unwrap_err()
            .kind,
            StorageProtocolErrorKind::PendingRequestLimitExceeded
        );
        assert_eq!(one.pending_requests(), 1);
        assert_eq!(one.tracked_bytes(), before);

        let mut bytes = StorageRequestQueue::try_new(
            process,
            storage_limits(),
            StorageRequestLimits {
                max_request_bytes: 64,
                max_pending_requests: 4,
                max_pending_bytes: 30,
            },
        )
        .unwrap();
        bytes
            .enqueue(
                origin.clone(),
                StorageCommand::Get {
                    key: String::from("a"),
                },
            )
            .unwrap();
        let before = bytes.tracked_bytes();
        assert_eq!(
            bytes
                .enqueue(
                    origin,
                    StorageCommand::Get {
                        key: String::from("b"),
                    },
                )
                .unwrap_err()
                .kind,
            StorageProtocolErrorKind::PendingByteLimitExceeded
        );
        assert_eq!(bytes.pending_requests(), 1);
        assert_eq!(bytes.tracked_bytes(), before);
    }

    #[test]
    fn oversized_and_opaque_requests_fail_without_tracking() {
        let process = process();
        let mut queue = StorageRequestQueue::try_new(process, storage_limits(), request_limits())
            .unwrap();
        let opaque = origin("data:text/plain,hello");
        assert_eq!(
            queue
                .enqueue(opaque, StorageCommand::Clear)
                .unwrap_err()
                .kind,
            StorageProtocolErrorKind::OpaqueOrigin
        );
        assert_eq!(queue.pending_requests(), 0);

        let exact = origin("https://example.com/");
        assert_eq!(
            queue
                .enqueue(
                    exact,
                    StorageCommand::Put {
                        key: String::from("key"),
                        value: vec![0; 33],
                    },
                )
                .unwrap_err()
                .kind,
            StorageProtocolErrorKind::ValueTooLarge
        );
        assert_eq!(queue.pending_requests(), 0);
        assert_eq!(queue.tracked_bytes(), 0);
    }

    #[test]
    fn completed_and_cancelled_request_ids_stay_stale() {
        let process = process();
        let origin = origin("https://example.com/");
        let mut storage = StorageProcessState::try_new(process, storage_limits()).unwrap();
        let mut queue = StorageRequestQueue::try_new(process, storage_limits(), request_limits())
            .unwrap();

        let first = queue
            .enqueue(origin.clone(), StorageCommand::Clear)
            .unwrap();
        let request = queue.take_next().unwrap();
        let response = execute_storage_request(&mut storage, &request).unwrap();
        let replay = response.clone();
        queue.complete(response).unwrap();
        assert_eq!(
            queue.complete(replay).unwrap_err().kind,
            StorageProtocolErrorKind::UnknownRequest
        );

        let second = queue.enqueue(origin, StorageCommand::Clear).unwrap();
        assert_ne!(first, second);
        let request = queue.take_next().unwrap();
        let response = execute_storage_request(&mut storage, &request).unwrap();
        assert!(queue.cancel(second).unwrap());
        assert_eq!(
            queue.complete(response).unwrap_err().kind,
            StorageProtocolErrorKind::UnknownRequest
        );
        assert_eq!(queue.pending_requests(), 0);
        assert_eq!(queue.tracked_bytes(), 0);
    }

    #[test]
    fn wrong_storage_process_fails_before_state_access() {
        let process = process();
        let wrong = different_process();
        assert_ne!(process, wrong);
        let origin = origin("https://example.com/");
        let mut queue = StorageRequestQueue::try_new(process, storage_limits(), request_limits())
            .unwrap();
        queue
            .enqueue(
                origin.clone(),
                StorageCommand::Put {
                    key: String::from("key"),
                    value: b"value".to_vec(),
                },
            )
            .unwrap();
        let request = queue.take_next().unwrap();
        let mut wrong_storage = StorageProcessState::try_new(wrong, storage_limits()).unwrap();

        assert_eq!(
            execute_storage_request(&mut wrong_storage, &request)
                .unwrap_err()
                .kind,
            StorageProtocolErrorKind::WrongProcess
        );
        assert_eq!(wrong_storage.get(&origin, "key"), None);
        assert_eq!(queue.pending_requests(), 1);
    }

    #[test]
    fn mismatched_response_does_not_consume_pending_authority() {
        let process = process();
        let origin = origin("https://example.com/");
        let mut storage = StorageProcessState::try_new(process, storage_limits()).unwrap();
        let mut queue = StorageRequestQueue::try_new(process, storage_limits(), request_limits())
            .unwrap();
        let id = queue
            .enqueue(
                origin,
                StorageCommand::Get {
                    key: String::from("key"),
                },
            )
            .unwrap();
        let request = queue.take_next().unwrap();
        let valid = execute_storage_request(&mut storage, &request).unwrap();
        let mismatched = StorageResponse {
            request: id,
            process,
            kind: StorageOperationKind::Put,
            payload: StorageResponsePayload::Written,
        };

        assert_eq!(
            queue.complete(mismatched).unwrap_err().kind,
            StorageProtocolErrorKind::ResponseMismatch
        );
        assert_eq!(queue.pending_requests(), 1);
        assert_eq!(
            queue.complete(valid).unwrap(),
            StorageResponsePayload::Value(None)
        );
        assert_eq!(queue.pending_requests(), 0);
    }
}
