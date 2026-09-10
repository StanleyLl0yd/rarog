use super::model::{
    StorageCommand, StorageOperationKind, StorageProtocolError, StorageProtocolErrorKind,
    StorageRequest, StorageRequestId, StorageRequestLimits, StorageResponse,
    StorageResponsePayload,
};
use crate::state::{StorageLimits, StorageProcessState};
use rarog_process::StorageProcessId;
use rarog_url::Origin;
use std::collections::{HashMap, VecDeque};
use std::num::NonZeroU64;

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
        let id = StorageRequestId::try_new(raw.get())?;
        if self.pending.contains_key(&id) {
            return Err(StorageProtocolError::new(
                StorageProtocolErrorKind::AccountingOverflow,
                "storage request allocator reused a live identity",
            ));
        }

        self.pending
            .try_reserve(1)
            .map_err(|_| allocation_error())?;
        self.queued.try_reserve(1).map_err(|_| allocation_error())?;

        let kind = command.kind();
        self.pending.insert(
            id,
            PendingRequest {
                process: self.process,
                kind,
                accounted_bytes,
            },
        );
        self.queued.push_back(StorageRequest {
            id,
            process: self.process,
            origin,
            command,
            accounted_bytes,
        });
        self.tracked_bytes = next_bytes;
        Ok(id)
    }

    pub fn take_next(&mut self) -> Option<StorageRequest> {
        self.queued.pop_front()
    }

    pub fn cancel(&mut self, id: StorageRequestId) -> Result<bool, StorageProtocolError> {
        let Some(pending) = self.pending.get(&id).copied() else {
            return Ok(false);
        };
        let next_bytes = self.checked_release(pending.accounted_bytes)?;
        let removed = self.pending.remove(&id);
        debug_assert!(removed.is_some());
        if let Some(position) = self.queued.iter().position(|request| request.id == id) {
            let _ = self.queued.remove(position);
        }
        self.tracked_bytes = next_bytes;
        Ok(true)
    }

    pub fn complete(
        &mut self,
        response: StorageResponse,
    ) -> Result<StorageResponsePayload, StorageProtocolError> {
        let pending = self
            .pending
            .get(&response.request())
            .copied()
            .ok_or_else(|| {
                StorageProtocolError::new(
                    StorageProtocolErrorKind::UnknownRequest,
                    format!(
                        "unknown, cancelled or completed storage request {}",
                        response.request()
                    ),
                )
            })?;
        if response.process() != self.process || response.process() != pending.process {
            return Err(StorageProtocolError::new(
                StorageProtocolErrorKind::WrongProcess,
                "storage response process identity does not match Host request authority",
            ));
        }
        if response.kind() != pending.kind
            || !payload_matches_kind(response.payload(), pending.kind)
        {
            return Err(StorageProtocolError::new(
                StorageProtocolErrorKind::ResponseMismatch,
                "storage response operation does not match the pending request",
            ));
        }

        let next_bytes = self.checked_release(pending.accounted_bytes)?;
        let removed = self.pending.remove(&response.request());
        debug_assert!(removed.is_some());
        self.tracked_bytes = next_bytes;
        Ok(response.into_payload())
    }

    fn checked_release(&self, bytes: usize) -> Result<usize, StorageProtocolError> {
        self.tracked_bytes.checked_sub(bytes).ok_or_else(|| {
            StorageProtocolError::new(
                StorageProtocolErrorKind::AccountingOverflow,
                "storage pending-byte accounting underflow",
            )
        })
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
        let bytes = origin
            .ascii_serialization()
            .len()
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
    if request.process() != storage.process() {
        return Err(StorageProtocolError::new(
            StorageProtocolErrorKind::WrongProcess,
            "storage request targets a different Storage-process identity",
        ));
    }
    if request.origin().is_opaque() {
        return Err(StorageProtocolError::new(
            StorageProtocolErrorKind::OpaqueOrigin,
            "opaque origins cannot execute persistent storage requests",
        ));
    }

    let payload = match request.command() {
        StorageCommand::Get { key } => {
            let value = storage
                .get(request.origin(), key)
                .map(try_owned_bytes)
                .transpose()?;
            StorageResponsePayload::Value(value)
        }
        StorageCommand::Put { key, value } => {
            storage.put(request.origin(), key, value)?;
            StorageResponsePayload::Written
        }
        StorageCommand::Remove { key } => {
            StorageResponsePayload::Removed(storage.remove(request.origin(), key)?)
        }
        StorageCommand::Clear => {
            StorageResponsePayload::Cleared(storage.clear_origin(request.origin())?)
        }
    };

    Ok(StorageResponse::new(
        request.id(),
        request.process(),
        request.kind(),
        payload,
    ))
}

fn payload_matches_kind(payload: &StorageResponsePayload, kind: StorageOperationKind) -> bool {
    matches!(
        (payload, kind),
        (StorageResponsePayload::Value(_), StorageOperationKind::Get)
            | (StorageResponsePayload::Written, StorageOperationKind::Put)
            | (
                StorageResponsePayload::Removed(_),
                StorageOperationKind::Remove
            )
            | (
                StorageResponsePayload::Cleared(_),
                StorageOperationKind::Clear
            )
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
