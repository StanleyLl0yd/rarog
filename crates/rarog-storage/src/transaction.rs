use crate::state::{
    validate_persistent_origin_identity, StorageError, StorageErrorKind, StorageLimits,
    StorageProcessState,
};
use rarog_process::StorageProcessId;
use rarog_url::Origin;
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroU64;

pub const DEFAULT_MAX_STORAGE_TRANSACTIONS: usize = 256;
pub const DEFAULT_MAX_STORAGE_TRANSACTION_MUTATIONS: usize = 1024;
pub const DEFAULT_MAX_STORAGE_TRANSACTION_BYTES: usize = 4 * 1024 * 1024;
pub const DEFAULT_MAX_STAGED_STORAGE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageTransactionLimits {
    pub max_transactions: usize,
    pub max_mutations_per_transaction: usize,
    pub max_staged_bytes_per_transaction: usize,
    pub max_total_staged_bytes: usize,
}

impl StorageTransactionLimits {
    pub fn is_valid(self) -> bool {
        self.max_transactions > 0
            && self.max_mutations_per_transaction > 0
            && self.max_staged_bytes_per_transaction > 0
            && self.max_total_staged_bytes > 0
            && self.max_staged_bytes_per_transaction <= self.max_total_staged_bytes
    }
}

impl Default for StorageTransactionLimits {
    fn default() -> Self {
        Self {
            max_transactions: DEFAULT_MAX_STORAGE_TRANSACTIONS,
            max_mutations_per_transaction: DEFAULT_MAX_STORAGE_TRANSACTION_MUTATIONS,
            max_staged_bytes_per_transaction: DEFAULT_MAX_STORAGE_TRANSACTION_BYTES,
            max_total_staged_bytes: DEFAULT_MAX_STAGED_STORAGE_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageTransactionMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageDurabilityHint {
    Default,
    Strict,
    Relaxed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageTransactionState {
    Waiting,
    Active,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StorageTransactionId(NonZeroU64);

impl StorageTransactionId {
    pub fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Display for StorageTransactionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "storage-transaction:{}", self.get())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageTransactionErrorKind {
    InvalidLimits,
    TransactionIdentitySpaceExhausted,
    TransactionLimitExceeded,
    UnknownTransaction,
    TransactionNotActive,
    ReadOnlyMutation,
    MutationLimitExceeded,
    StagedByteLimitExceeded,
    TotalStagedByteLimitExceeded,
    WrongProcess,
    AccountingOverflow,
    AllocationFailed,
    Storage(StorageErrorKind),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageTransactionError {
    pub kind: StorageTransactionErrorKind,
    pub message: String,
}

impl StorageTransactionError {
    fn new(kind: StorageTransactionErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for StorageTransactionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for StorageTransactionError {}

impl From<StorageError> for StorageTransactionError {
    fn from(error: StorageError) -> Self {
        Self::new(StorageTransactionErrorKind::Storage(error.kind), error.message)
    }
}

#[derive(Debug)]
enum StorageMutation {
    Put { key: String, value: Vec<u8> },
    Remove { key: String },
    Clear,
}

#[derive(Debug)]
struct StorageTransaction {
    origin: Origin,
    mode: StorageTransactionMode,
    durability: StorageDurabilityHint,
    state: StorageTransactionState,
    mutations: Vec<StorageMutation>,
    staged_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageTransactionCommit {
    transaction: StorageTransactionId,
    durability: StorageDurabilityHint,
    mutated: bool,
}

impl StorageTransactionCommit {
    pub fn transaction(self) -> StorageTransactionId {
        self.transaction
    }

    pub fn durability(self) -> StorageDurabilityHint {
        self.durability
    }

    pub fn mutated(self) -> bool {
        self.mutated
    }
}

#[derive(Debug)]
pub struct StorageTransactionManager {
    process: StorageProcessId,
    storage_limits: StorageLimits,
    limits: StorageTransactionLimits,
    next_transaction: Option<NonZeroU64>,
    transactions: HashMap<StorageTransactionId, StorageTransaction>,
    total_staged_bytes: usize,
}

impl StorageTransactionManager {
    pub fn try_new(
        process: StorageProcessId,
        storage_limits: StorageLimits,
        limits: StorageTransactionLimits,
    ) -> Result<Self, StorageTransactionError> {
        if !storage_limits.is_valid() || !limits.is_valid() {
            return Err(StorageTransactionError::new(
                StorageTransactionErrorKind::InvalidLimits,
                "storage and transaction limits must be valid and non-zero",
            ));
        }
        Ok(Self {
            process,
            storage_limits,
            limits,
            next_transaction: NonZeroU64::new(1),
            transactions: HashMap::new(),
            total_staged_bytes: 0,
        })
    }

    pub fn process(&self) -> StorageProcessId {
        self.process
    }

    pub fn limits(&self) -> StorageTransactionLimits {
        self.limits
    }

    pub fn active_transactions(&self) -> usize {
        self.transactions.len()
    }

    pub fn total_staged_bytes(&self) -> usize {
        self.total_staged_bytes
    }

    pub fn begin(
        &mut self,
        origin: Origin,
        mode: StorageTransactionMode,
        durability: StorageDurabilityHint,
    ) -> Result<StorageTransactionId, StorageTransactionError> {
        validate_persistent_origin_identity(&origin)?;
        if self.transactions.len() >= self.limits.max_transactions {
            return Err(StorageTransactionError::new(
                StorageTransactionErrorKind::TransactionLimitExceeded,
                format!(
                    "storage transaction limit {} reached",
                    self.limits.max_transactions
                ),
            ));
        }
        self.transactions
            .try_reserve(1)
            .map_err(|_| allocation_error())?;

        let raw = self.next_transaction.ok_or_else(|| {
            StorageTransactionError::new(
                StorageTransactionErrorKind::TransactionIdentitySpaceExhausted,
                "storage transaction identity space is exhausted",
            )
        })?;
        let id = StorageTransactionId(raw);
        self.next_transaction = raw.get().checked_add(1).and_then(NonZeroU64::new);
        let state = if self.can_start_new(mode, &origin) {
            StorageTransactionState::Active
        } else {
            StorageTransactionState::Waiting
        };
        self.transactions.insert(
            id,
            StorageTransaction {
                origin,
                mode,
                durability,
                state,
                mutations: Vec::new(),
                staged_bytes: 0,
            },
        );
        Ok(id)
    }

    pub fn state(
        &self,
        id: StorageTransactionId,
    ) -> Result<StorageTransactionState, StorageTransactionError> {
        Ok(self.transaction(id)?.state)
    }

    pub fn origin(
        &self,
        id: StorageTransactionId,
    ) -> Result<&Origin, StorageTransactionError> {
        Ok(&self.transaction(id)?.origin)
    }

    pub fn mode(
        &self,
        id: StorageTransactionId,
    ) -> Result<StorageTransactionMode, StorageTransactionError> {
        Ok(self.transaction(id)?.mode)
    }

    pub fn durability(
        &self,
        id: StorageTransactionId,
    ) -> Result<StorageDurabilityHint, StorageTransactionError> {
        Ok(self.transaction(id)?.durability)
    }

    pub fn staged_mutations(
        &self,
        id: StorageTransactionId,
    ) -> Result<usize, StorageTransactionError> {
        Ok(self.transaction(id)?.mutations.len())
    }

    pub fn staged_bytes(
        &self,
        id: StorageTransactionId,
    ) -> Result<usize, StorageTransactionError> {
        Ok(self.transaction(id)?.staged_bytes)
    }

    pub fn read(
        &self,
        id: StorageTransactionId,
        storage: &StorageProcessState,
        key: &str,
    ) -> Result<Option<Vec<u8>>, StorageTransactionError> {
        self.authorize_storage(storage)?;
        self.validate_key(key)?;
        let transaction = self.active_transaction(id)?;
        for mutation in transaction.mutations.iter().rev() {
            match mutation {
                StorageMutation::Put {
                    key: staged_key,
                    value,
                } if staged_key == key => return try_owned_bytes(value).map(Some),
                StorageMutation::Remove { key: staged_key } if staged_key == key => {
                    return Ok(None);
                }
                StorageMutation::Clear => return Ok(None),
                _ => {}
            }
        }
        storage
            .get(&transaction.origin, key)
            .map(try_owned_bytes)
            .transpose()
    }

    pub fn stage_put(
        &mut self,
        id: StorageTransactionId,
        key: &str,
        value: &[u8],
    ) -> Result<(), StorageTransactionError> {
        self.validate_key(key)?;
        if value.len() > self.storage_limits.max_value_bytes {
            return Err(StorageTransactionError::new(
                StorageTransactionErrorKind::Storage(StorageErrorKind::ValueTooLarge),
                format!(
                    "storage value requires {} bytes; limit is {}",
                    value.len(),
                    self.storage_limits.max_value_bytes
                ),
            ));
        }
        let accounted_bytes = key
            .len()
            .checked_add(value.len())
            .ok_or_else(accounting_overflow)?;
        self.validate_mutation_budget(id, accounted_bytes)?;
        let key = try_owned_string(key)?;
        let value = try_owned_bytes(value)?;
        self.push_mutation(id, StorageMutation::Put { key, value }, accounted_bytes)
    }

    pub fn stage_remove(
        &mut self,
        id: StorageTransactionId,
        key: &str,
    ) -> Result<(), StorageTransactionError> {
        self.validate_key(key)?;
        let accounted_bytes = key.len();
        self.validate_mutation_budget(id, accounted_bytes)?;
        let key = try_owned_string(key)?;
        self.push_mutation(id, StorageMutation::Remove { key }, accounted_bytes)
    }

    pub fn stage_clear(
        &mut self,
        id: StorageTransactionId,
    ) -> Result<(), StorageTransactionError> {
        self.validate_mutation_budget(id, 0)?;
        self.push_mutation(id, StorageMutation::Clear, 0)
    }

    pub fn commit(
        &mut self,
        id: StorageTransactionId,
        storage: &mut StorageProcessState,
    ) -> Result<StorageTransactionCommit, StorageTransactionError> {
        self.authorize_storage(storage)?;
        let (mode, durability, mutated) = {
            let transaction = self.active_transaction(id)?;
            (
                transaction.mode,
                transaction.durability,
                !transaction.mutations.is_empty(),
            )
        };

        if mode == StorageTransactionMode::ReadOnly {
            self.finish(id)?;
            return Ok(StorageTransactionCommit {
                transaction: id,
                durability,
                mutated: false,
            });
        }

        let candidate = {
            let transaction = self.active_transaction(id)?;
            let mut candidate = storage.try_clone_for_transaction()?;
            let result = apply_mutations(&mut candidate, &transaction.origin, &transaction.mutations);
            match result {
                Ok(()) => Ok(candidate),
                Err(error) => Err(StorageTransactionError::from(error)),
            }
        };

        let candidate = match candidate {
            Ok(candidate) => candidate,
            Err(error) => {
                self.finish(id)?;
                return Err(error);
            }
        };

        self.finish(id)?;
        *storage = candidate;
        Ok(StorageTransactionCommit {
            transaction: id,
            durability,
            mutated,
        })
    }

    pub fn abort(
        &mut self,
        id: StorageTransactionId,
    ) -> Result<bool, StorageTransactionError> {
        self.transaction(id)?;
        self.finish(id)?;
        Ok(true)
    }

    fn transaction(
        &self,
        id: StorageTransactionId,
    ) -> Result<&StorageTransaction, StorageTransactionError> {
        self.transactions.get(&id).ok_or_else(|| {
            StorageTransactionError::new(
                StorageTransactionErrorKind::UnknownTransaction,
                format!("unknown, completed or aborted storage transaction {id}"),
            )
        })
    }

    fn active_transaction(
        &self,
        id: StorageTransactionId,
    ) -> Result<&StorageTransaction, StorageTransactionError> {
        let transaction = self.transaction(id)?;
        if transaction.state != StorageTransactionState::Active {
            return Err(StorageTransactionError::new(
                StorageTransactionErrorKind::TransactionNotActive,
                format!("storage transaction {id} is waiting for its exact-origin scope"),
            ));
        }
        Ok(transaction)
    }

    fn authorize_storage(
        &self,
        storage: &StorageProcessState,
    ) -> Result<(), StorageTransactionError> {
        if storage.process() != self.process {
            return Err(StorageTransactionError::new(
                StorageTransactionErrorKind::WrongProcess,
                "storage transaction manager and committed state belong to different Storage processes",
            ));
        }
        Ok(())
    }

    fn validate_key(&self, key: &str) -> Result<(), StorageTransactionError> {
        if key.len() > self.storage_limits.max_key_bytes {
            return Err(StorageTransactionError::new(
                StorageTransactionErrorKind::Storage(StorageErrorKind::KeyTooLarge),
                format!(
                    "storage key requires {} bytes; limit is {}",
                    key.len(),
                    self.storage_limits.max_key_bytes
                ),
            ));
        }
        Ok(())
    }

    fn validate_mutation_budget(
        &self,
        id: StorageTransactionId,
        accounted_bytes: usize,
    ) -> Result<(), StorageTransactionError> {
        let transaction = self.active_transaction(id)?;
        if transaction.mode != StorageTransactionMode::ReadWrite {
            return Err(StorageTransactionError::new(
                StorageTransactionErrorKind::ReadOnlyMutation,
                "read-only storage transaction cannot stage mutations",
            ));
        }
        if transaction.mutations.len() >= self.limits.max_mutations_per_transaction {
            return Err(StorageTransactionError::new(
                StorageTransactionErrorKind::MutationLimitExceeded,
                format!(
                    "storage transaction mutation limit {} reached",
                    self.limits.max_mutations_per_transaction
                ),
            ));
        }
        let transaction_bytes = transaction
            .staged_bytes
            .checked_add(accounted_bytes)
            .ok_or_else(accounting_overflow)?;
        if transaction_bytes > self.limits.max_staged_bytes_per_transaction {
            return Err(StorageTransactionError::new(
                StorageTransactionErrorKind::StagedByteLimitExceeded,
                format!(
                    "storage transaction would stage {transaction_bytes} bytes; limit is {}",
                    self.limits.max_staged_bytes_per_transaction
                ),
            ));
        }
        let total_bytes = self
            .total_staged_bytes
            .checked_add(accounted_bytes)
            .ok_or_else(accounting_overflow)?;
        if total_bytes > self.limits.max_total_staged_bytes {
            return Err(StorageTransactionError::new(
                StorageTransactionErrorKind::TotalStagedByteLimitExceeded,
                format!(
                    "storage transactions would stage {total_bytes} bytes; limit is {}",
                    self.limits.max_total_staged_bytes
                ),
            ));
        }
        Ok(())
    }

    fn push_mutation(
        &mut self,
        id: StorageTransactionId,
        mutation: StorageMutation,
        accounted_bytes: usize,
    ) -> Result<(), StorageTransactionError> {
        let transaction = self.transactions.get_mut(&id).ok_or_else(|| {
            StorageTransactionError::new(
                StorageTransactionErrorKind::UnknownTransaction,
                format!("unknown, completed or aborted storage transaction {id}"),
            )
        })?;
        transaction
            .mutations
            .try_reserve(1)
            .map_err(|_| allocation_error())?;
        let next_transaction_bytes = transaction
            .staged_bytes
            .checked_add(accounted_bytes)
            .ok_or_else(accounting_overflow)?;
        let next_total_bytes = self
            .total_staged_bytes
            .checked_add(accounted_bytes)
            .ok_or_else(accounting_overflow)?;
        transaction.mutations.push(mutation);
        transaction.staged_bytes = next_transaction_bytes;
        self.total_staged_bytes = next_total_bytes;
        Ok(())
    }

    fn finish(&mut self, id: StorageTransactionId) -> Result<(), StorageTransactionError> {
        let staged_bytes = self.transaction(id)?.staged_bytes;
        let next_total = self
            .total_staged_bytes
            .checked_sub(staged_bytes)
            .ok_or_else(accounting_overflow)?;
        if self.transactions.remove(&id).is_none() {
            return Err(StorageTransactionError::new(
                StorageTransactionErrorKind::UnknownTransaction,
                format!("unknown, completed or aborted storage transaction {id}"),
            ));
        }
        self.total_staged_bytes = next_total;
        self.refresh_waiting();
        Ok(())
    }

    fn can_start_new(&self, mode: StorageTransactionMode, origin: &Origin) -> bool {
        !self
            .transactions
            .values()
            .any(|older| transactions_conflict(older, mode, origin))
    }

    fn can_start_existing(
        &self,
        id: StorageTransactionId,
        mode: StorageTransactionMode,
        origin: &Origin,
    ) -> bool {
        !self.transactions.iter().any(|(older_id, older)| {
            older_id.get() < id.get() && transactions_conflict(older, mode, origin)
        })
    }

    fn refresh_waiting(&mut self) {
        let mut after = 0_u64;
        loop {
            let next = self
                .transactions
                .iter()
                .filter(|(id, transaction)| {
                    id.get() > after && transaction.state == StorageTransactionState::Waiting
                })
                .map(|(id, _)| *id)
                .min_by_key(|id| id.get());
            let Some(id) = next else {
                break;
            };
            let can_start = {
                let transaction = self.transactions.get(&id).expect("transaction id came from map");
                self.can_start_existing(id, transaction.mode, &transaction.origin)
            };
            if can_start {
                if let Some(transaction) = self.transactions.get_mut(&id) {
                    transaction.state = StorageTransactionState::Active;
                }
            }
            after = id.get();
        }
    }
}

fn transactions_conflict(
    older: &StorageTransaction,
    newer_mode: StorageTransactionMode,
    newer_origin: &Origin,
) -> bool {
    if older.origin != *newer_origin {
        return false;
    }
    match newer_mode {
        StorageTransactionMode::ReadOnly => older.mode == StorageTransactionMode::ReadWrite,
        StorageTransactionMode::ReadWrite => true,
    }
}

fn apply_mutations(
    storage: &mut StorageProcessState,
    origin: &Origin,
    mutations: &[StorageMutation],
) -> Result<(), StorageError> {
    for mutation in mutations {
        match mutation {
            StorageMutation::Put { key, value } => storage.put(origin, key, value)?,
            StorageMutation::Remove { key } => {
                storage.remove(origin, key)?;
            }
            StorageMutation::Clear => {
                storage.clear_origin(origin)?;
            }
        }
    }
    Ok(())
}

fn try_owned_string(value: &str) -> Result<String, StorageTransactionError> {
    let mut owned = String::new();
    owned
        .try_reserve_exact(value.len())
        .map_err(|_| allocation_error())?;
    owned.push_str(value);
    Ok(owned)
}

fn try_owned_bytes(value: &[u8]) -> Result<Vec<u8>, StorageTransactionError> {
    let mut owned = Vec::new();
    owned
        .try_reserve_exact(value.len())
        .map_err(|_| allocation_error())?;
    owned.extend_from_slice(value);
    Ok(owned)
}

fn allocation_error() -> StorageTransactionError {
    StorageTransactionError::new(
        StorageTransactionErrorKind::AllocationFailed,
        "storage transaction allocation failed within configured bounds",
    )
}

fn accounting_overflow() -> StorageTransactionError {
    StorageTransactionError::new(
        StorageTransactionErrorKind::AccountingOverflow,
        "storage transaction byte accounting overflow",
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

    fn storage_limits() -> StorageLimits {
        StorageLimits {
            max_origins: 4,
            max_entries_per_origin: 8,
            max_key_bytes: 16,
            max_value_bytes: 32,
            max_origin_bytes: 64,
            max_total_bytes: 96,
        }
    }

    fn transaction_limits() -> StorageTransactionLimits {
        StorageTransactionLimits {
            max_transactions: 8,
            max_mutations_per_transaction: 4,
            max_staged_bytes_per_transaction: 48,
            max_total_staged_bytes: 64,
        }
    }

    fn manager(process: StorageProcessId) -> StorageTransactionManager {
        StorageTransactionManager::try_new(process, storage_limits(), transaction_limits()).unwrap()
    }

    #[test]
    fn read_only_transactions_overlap_but_write_scope_waits() {
        let process = process();
        let shared = origin("https://example.com/");
        let mut manager = manager(process);
        let first = manager
            .begin(
                shared.clone(),
                StorageTransactionMode::ReadOnly,
                StorageDurabilityHint::Default,
            )
            .unwrap();
        let second = manager
            .begin(
                shared.clone(),
                StorageTransactionMode::ReadOnly,
                StorageDurabilityHint::Relaxed,
            )
            .unwrap();
        let writer = manager
            .begin(
                shared,
                StorageTransactionMode::ReadWrite,
                StorageDurabilityHint::Strict,
            )
            .unwrap();

        assert_eq!(manager.state(first).unwrap(), StorageTransactionState::Active);
        assert_eq!(manager.state(second).unwrap(), StorageTransactionState::Active);
        assert_eq!(manager.state(writer).unwrap(), StorageTransactionState::Waiting);
        assert_eq!(
            manager.stage_put(writer, "k", b"v").unwrap_err().kind,
            StorageTransactionErrorKind::TransactionNotActive
        );
        manager.abort(first).unwrap();
        assert_eq!(manager.state(writer).unwrap(), StorageTransactionState::Waiting);
        manager.abort(second).unwrap();
        assert_eq!(manager.state(writer).unwrap(), StorageTransactionState::Active);
    }

    #[test]
    fn older_writer_blocks_later_overlapping_readers_and_writers_only() {
        let process = process();
        let first_origin = origin("https://a.example/");
        let other_origin = origin("https://b.example/");
        let mut manager = manager(process);
        let writer = manager
            .begin(
                first_origin.clone(),
                StorageTransactionMode::ReadWrite,
                StorageDurabilityHint::Default,
            )
            .unwrap();
        let reader = manager
            .begin(
                first_origin.clone(),
                StorageTransactionMode::ReadOnly,
                StorageDurabilityHint::Default,
            )
            .unwrap();
        let later_writer = manager
            .begin(
                first_origin,
                StorageTransactionMode::ReadWrite,
                StorageDurabilityHint::Default,
            )
            .unwrap();
        let independent = manager
            .begin(
                other_origin,
                StorageTransactionMode::ReadWrite,
                StorageDurabilityHint::Default,
            )
            .unwrap();

        assert_eq!(manager.state(writer).unwrap(), StorageTransactionState::Active);
        assert_eq!(manager.state(reader).unwrap(), StorageTransactionState::Waiting);
        assert_eq!(manager.state(later_writer).unwrap(), StorageTransactionState::Waiting);
        assert_eq!(manager.state(independent).unwrap(), StorageTransactionState::Active);
        manager.abort(writer).unwrap();
        assert_eq!(manager.state(reader).unwrap(), StorageTransactionState::Active);
        assert_eq!(manager.state(later_writer).unwrap(), StorageTransactionState::Waiting);
        manager.abort(reader).unwrap();
        assert_eq!(manager.state(later_writer).unwrap(), StorageTransactionState::Active);
    }

    #[test]
    fn read_only_transaction_cannot_stage_mutation() {
        let process = process();
        let mut manager = manager(process);
        let transaction = manager
            .begin(
                origin("https://example.com/"),
                StorageTransactionMode::ReadOnly,
                StorageDurabilityHint::Default,
            )
            .unwrap();
        assert_eq!(
            manager.stage_put(transaction, "key", b"value").unwrap_err().kind,
            StorageTransactionErrorKind::ReadOnlyMutation
        );
        assert_eq!(manager.staged_mutations(transaction).unwrap(), 0);
    }

    #[test]
    fn staged_writes_are_read_your_writes_and_exact_origin_scoped() {
        let process = process();
        let first = origin("https://a.example/");
        let second = origin("https://b.example/");
        let mut storage = StorageProcessState::try_new(process, storage_limits()).unwrap();
        storage.put(&first, "key", b"one").unwrap();
        storage.put(&second, "key", b"two").unwrap();
        let mut manager = manager(process);
        let transaction = manager
            .begin(
                first.clone(),
                StorageTransactionMode::ReadWrite,
                StorageDurabilityHint::Default,
            )
            .unwrap();

        assert_eq!(manager.read(transaction, &storage, "key").unwrap(), Some(b"one".to_vec()));
        manager.stage_put(transaction, "key", b"changed").unwrap();
        assert_eq!(manager.read(transaction, &storage, "key").unwrap(), Some(b"changed".to_vec()));
        assert_eq!(storage.get(&second, "key"), Some(b"two".as_slice()));
        manager.stage_clear(transaction).unwrap();
        assert_eq!(manager.read(transaction, &storage, "key").unwrap(), None);
        manager.stage_put(transaction, "key", b"after-clear").unwrap();
        assert_eq!(manager.read(transaction, &storage, "key").unwrap(), Some(b"after-clear".to_vec()));
    }

    #[test]
    fn multi_mutation_commit_is_atomic_and_reports_durability_hint() {
        let process = process();
        let exact = origin("https://example.com/");
        let mut storage = StorageProcessState::try_new(process, storage_limits()).unwrap();
        storage.put(&exact, "old", b"value").unwrap();
        let mut manager = manager(process);
        let transaction = manager
            .begin(
                exact.clone(),
                StorageTransactionMode::ReadWrite,
                StorageDurabilityHint::Strict,
            )
            .unwrap();
        manager.stage_remove(transaction, "old").unwrap();
        manager.stage_put(transaction, "a", b"one").unwrap();
        manager.stage_put(transaction, "b", b"two").unwrap();

        assert_eq!(storage.get(&exact, "a"), None);
        let commit = manager.commit(transaction, &mut storage).unwrap();
        assert_eq!(commit.transaction(), transaction);
        assert_eq!(commit.durability(), StorageDurabilityHint::Strict);
        assert!(commit.mutated());
        assert_eq!(storage.get(&exact, "old"), None);
        assert_eq!(storage.get(&exact, "a"), Some(b"one".as_slice()));
        assert_eq!(storage.get(&exact, "b"), Some(b"two".as_slice()));
        assert_eq!(manager.active_transactions(), 0);
        assert_eq!(manager.total_staged_bytes(), 0);
    }

    #[test]
    fn failed_commit_aborts_and_preserves_committed_state() {
        let process = process();
        let exact = origin("https://example.com/");
        let limited = StorageLimits {
            max_total_bytes: 12,
            max_origin_bytes: 12,
            ..storage_limits()
        };
        let mut storage = StorageProcessState::try_new(process, limited).unwrap();
        storage.put(&exact, "base", b"old").unwrap();
        let mut manager = StorageTransactionManager::try_new(process, limited, transaction_limits()).unwrap();
        let transaction = manager
            .begin(
                exact.clone(),
                StorageTransactionMode::ReadWrite,
                StorageDurabilityHint::Default,
            )
            .unwrap();
        manager.stage_put(transaction, "a", b"1234").unwrap();
        manager.stage_put(transaction, "b", b"1234").unwrap();

        let error = manager.commit(transaction, &mut storage).unwrap_err();
        assert_eq!(
            error.kind,
            StorageTransactionErrorKind::Storage(StorageErrorKind::OriginByteLimitExceeded)
        );
        assert_eq!(storage.get(&exact, "base"), Some(b"old".as_slice()));
        assert_eq!(storage.get(&exact, "a"), None);
        assert_eq!(storage.get(&exact, "b"), None);
        assert_eq!(manager.active_transactions(), 0);
        assert_eq!(
            manager.state(transaction).unwrap_err().kind,
            StorageTransactionErrorKind::UnknownTransaction
        );
    }

    #[test]
    fn abort_discards_staged_state_and_transaction_id_stays_stale() {
        let process = process();
        let exact = origin("https://example.com/");
        let mut storage = StorageProcessState::try_new(process, storage_limits()).unwrap();
        let mut manager = manager(process);
        let first = manager
            .begin(
                exact.clone(),
                StorageTransactionMode::ReadWrite,
                StorageDurabilityHint::Default,
            )
            .unwrap();
        manager.stage_put(first, "key", b"value").unwrap();
        manager.abort(first).unwrap();
        assert_eq!(storage.get(&exact, "key"), None);
        assert_eq!(
            manager.stage_clear(first).unwrap_err().kind,
            StorageTransactionErrorKind::UnknownTransaction
        );
        let second = manager
            .begin(
                exact,
                StorageTransactionMode::ReadWrite,
                StorageDurabilityHint::Default,
            )
            .unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn mutation_and_staged_byte_limits_fail_before_tracking() {
        let process = process();
        let exact = origin("https://example.com/");
        let limits = StorageTransactionLimits {
            max_transactions: 2,
            max_mutations_per_transaction: 1,
            max_staged_bytes_per_transaction: 5,
            max_total_staged_bytes: 5,
        };
        let mut manager = StorageTransactionManager::try_new(process, storage_limits(), limits).unwrap();
        let transaction = manager
            .begin(
                exact,
                StorageTransactionMode::ReadWrite,
                StorageDurabilityHint::Default,
            )
            .unwrap();
        assert_eq!(
            manager.stage_put(transaction, "key", b"123").unwrap_err().kind,
            StorageTransactionErrorKind::StagedByteLimitExceeded
        );
        assert_eq!(manager.staged_mutations(transaction).unwrap(), 0);
        manager.stage_put(transaction, "k", b"1234").unwrap();
        let before = manager.total_staged_bytes();
        assert_eq!(
            manager.stage_clear(transaction).unwrap_err().kind,
            StorageTransactionErrorKind::MutationLimitExceeded
        );
        assert_eq!(manager.total_staged_bytes(), before);
    }

    #[test]
    fn wrong_process_is_rejected_before_commit_or_read() {
        let process = process();
        let mut topology = ProcessTopology::try_new(1).unwrap();
        topology
            .assign_site(WebUrl::parse("https://site.example/").unwrap().site_identity().unwrap())
            .unwrap();
        let other = topology.ensure_storage_process().unwrap().process();
        assert_ne!(process, other);
        let mut manager = manager(process);
        let transaction = manager
            .begin(
                origin("https://example.com/"),
                StorageTransactionMode::ReadWrite,
                StorageDurabilityHint::Default,
            )
            .unwrap();
        let mut wrong = StorageProcessState::try_new(other, storage_limits()).unwrap();
        assert_eq!(
            manager.read(transaction, &wrong, "key").unwrap_err().kind,
            StorageTransactionErrorKind::WrongProcess
        );
        assert_eq!(
            manager.commit(transaction, &mut wrong).unwrap_err().kind,
            StorageTransactionErrorKind::WrongProcess
        );
    }
}
