use crate::state::{StorageError, StorageErrorKind};
use rarog_process::StorageProcessId;
use rarog_url::Origin;
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
    pub(super) fn new(kind: StorageProtocolErrorKind, message: impl Into<String>) -> Self {
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

    pub(super) fn key_len(&self) -> usize {
        match self {
            Self::Get { key } | Self::Put { key, .. } | Self::Remove { key } => key.len(),
            Self::Clear => 0,
        }
    }

    pub(super) fn value_len(&self) -> usize {
        match self {
            Self::Put { value, .. } => value.len(),
            _ => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageRequest {
    pub(super) id: StorageRequestId,
    pub(super) process: StorageProcessId,
    pub(super) origin: Origin,
    pub(super) command: StorageCommand,
    pub(super) accounted_bytes: usize,
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
    pub(super) fn new(
        request: StorageRequestId,
        process: StorageProcessId,
        kind: StorageOperationKind,
        payload: StorageResponsePayload,
    ) -> Self {
        Self {
            request,
            process,
            kind,
            payload,
        }
    }

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
