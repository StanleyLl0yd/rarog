use rarog_process::StorageProcessId;
use rarog_url::Origin;
use std::collections::HashMap;
use std::fmt;

pub const DEFAULT_MAX_ORIGINS: usize = 1024;
pub const DEFAULT_MAX_ENTRIES_PER_ORIGIN: usize = 4096;
pub const DEFAULT_MAX_KEY_BYTES: usize = 16 * 1024;
pub const DEFAULT_MAX_VALUE_BYTES: usize = 1024 * 1024;
pub const DEFAULT_MAX_ORIGIN_BYTES: usize = 16 * 1024 * 1024;
pub const DEFAULT_MAX_TOTAL_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageLimits {
    pub max_origins: usize,
    pub max_entries_per_origin: usize,
    pub max_key_bytes: usize,
    pub max_value_bytes: usize,
    pub max_origin_bytes: usize,
    pub max_total_bytes: usize,
}

impl StorageLimits {
    pub fn is_valid(self) -> bool {
        self.max_origins > 0
            && self.max_entries_per_origin > 0
            && self.max_key_bytes > 0
            && self.max_value_bytes > 0
            && self.max_origin_bytes > 0
            && self.max_total_bytes > 0
            && self.max_key_bytes <= self.max_origin_bytes
            && self.max_value_bytes <= self.max_origin_bytes
            && self.max_origin_bytes <= self.max_total_bytes
    }
}

impl Default for StorageLimits {
    fn default() -> Self {
        Self {
            max_origins: DEFAULT_MAX_ORIGINS,
            max_entries_per_origin: DEFAULT_MAX_ENTRIES_PER_ORIGIN,
            max_key_bytes: DEFAULT_MAX_KEY_BYTES,
            max_value_bytes: DEFAULT_MAX_VALUE_BYTES,
            max_origin_bytes: DEFAULT_MAX_ORIGIN_BYTES,
            max_total_bytes: DEFAULT_MAX_TOTAL_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageErrorKind {
    InvalidLimits,
    OpaqueOrigin,
    KeyTooLarge,
    ValueTooLarge,
    OriginLimitExceeded,
    EntryLimitExceeded,
    OriginByteLimitExceeded,
    TotalByteLimitExceeded,
    AccountingOverflow,
    AllocationFailed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageError {
    pub kind: StorageErrorKind,
    pub message: String,
}

impl StorageError {
    fn new(kind: StorageErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for StorageError {}

#[derive(Debug, Default)]
struct OriginStorage {
    entries: HashMap<String, Vec<u8>>,
    bytes: usize,
}

#[derive(Debug)]
pub struct StorageProcessState {
    process: StorageProcessId,
    limits: StorageLimits,
    origins: HashMap<Origin, OriginStorage>,
    total_bytes: usize,
}

impl StorageProcessState {
    pub fn try_new(process: StorageProcessId, limits: StorageLimits) -> Result<Self, StorageError> {
        if !limits.is_valid() {
            return Err(StorageError::new(
                StorageErrorKind::InvalidLimits,
                "storage limits must be non-zero and nested byte limits must fit their parents",
            ));
        }
        Ok(Self {
            process,
            limits,
            origins: HashMap::new(),
            total_bytes: 0,
        })
    }

    pub fn process(&self) -> StorageProcessId {
        self.process
    }

    pub fn limits(&self) -> StorageLimits {
        self.limits
    }

    pub fn origin_count(&self) -> usize {
        self.origins.len()
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    pub fn entry_count(&self, origin: &Origin) -> usize {
        self.origins
            .get(origin)
            .map_or(0, |storage| storage.entries.len())
    }

    pub fn origin_bytes(&self, origin: &Origin) -> usize {
        self.origins.get(origin).map_or(0, |storage| storage.bytes)
    }

    pub fn get(&self, origin: &Origin, key: &str) -> Option<&[u8]> {
        self.origins
            .get(origin)
            .and_then(|storage| storage.entries.get(key))
            .map(Vec::as_slice)
    }

    pub fn put(&mut self, origin: &Origin, key: &str, value: &[u8]) -> Result<(), StorageError> {
        self.validate_persistent_origin(origin)?;
        if key.len() > self.limits.max_key_bytes {
            return Err(StorageError::new(
                StorageErrorKind::KeyTooLarge,
                format!(
                    "storage key requires {} bytes; limit is {}",
                    key.len(),
                    self.limits.max_key_bytes
                ),
            ));
        }
        if value.len() > self.limits.max_value_bytes {
            return Err(StorageError::new(
                StorageErrorKind::ValueTooLarge,
                format!(
                    "storage value requires {} bytes; limit is {}",
                    value.len(),
                    self.limits.max_value_bytes
                ),
            ));
        }

        let existing_storage = self.origins.get(origin);
        let existing_value = existing_storage.and_then(|storage| storage.entries.get(key));
        let existing_bytes = existing_value
            .map(|old| checked_entry_bytes(key.len(), old.len()))
            .transpose()?
            .unwrap_or(0);
        let new_bytes = checked_entry_bytes(key.len(), value.len())?;
        let new_origin = existing_storage.is_none();
        let new_entry = existing_value.is_none();

        if new_origin && self.origins.len() >= self.limits.max_origins {
            return Err(StorageError::new(
                StorageErrorKind::OriginLimitExceeded,
                format!("storage origin limit {} reached", self.limits.max_origins),
            ));
        }

        let current_entries = existing_storage.map_or(0, |storage| storage.entries.len());
        if new_entry && current_entries >= self.limits.max_entries_per_origin {
            return Err(StorageError::new(
                StorageErrorKind::EntryLimitExceeded,
                format!(
                    "storage entry limit {} reached for origin",
                    self.limits.max_entries_per_origin
                ),
            ));
        }

        let current_origin_bytes = existing_storage.map_or(0, |storage| storage.bytes);
        let next_origin_bytes = replaced_bytes(current_origin_bytes, existing_bytes, new_bytes)?;
        if next_origin_bytes > self.limits.max_origin_bytes {
            return Err(StorageError::new(
                StorageErrorKind::OriginByteLimitExceeded,
                format!(
                    "storage origin would require {next_origin_bytes} bytes; limit is {}",
                    self.limits.max_origin_bytes
                ),
            ));
        }

        let next_total_bytes = replaced_bytes(self.total_bytes, existing_bytes, new_bytes)?;
        if next_total_bytes > self.limits.max_total_bytes {
            return Err(StorageError::new(
                StorageErrorKind::TotalByteLimitExceeded,
                format!(
                    "storage process would require {next_total_bytes} bytes; limit is {}",
                    self.limits.max_total_bytes
                ),
            ));
        }

        let owned_key = try_owned_string(key)?;
        let owned_value = try_owned_bytes(value)?;

        if new_origin {
            self.origins
                .try_reserve(1)
                .map_err(|_| allocation_error())?;
            let mut storage = OriginStorage::default();
            storage
                .entries
                .try_reserve(1)
                .map_err(|_| allocation_error())?;
            storage.entries.insert(owned_key, owned_value);
            storage.bytes = next_origin_bytes;
            self.origins.insert(origin.clone(), storage);
        } else if let Some(storage) = self.origins.get_mut(origin) {
            if new_entry {
                storage
                    .entries
                    .try_reserve(1)
                    .map_err(|_| allocation_error())?;
            }
            storage.entries.insert(owned_key, owned_value);
            storage.bytes = next_origin_bytes;
        } else {
            return Err(StorageError::new(
                StorageErrorKind::AccountingOverflow,
                "storage origin disappeared during a single-threaded mutation",
            ));
        }

        self.total_bytes = next_total_bytes;
        Ok(())
    }

    pub fn remove(&mut self, origin: &Origin, key: &str) -> Result<bool, StorageError> {
        let Some(storage) = self.origins.get(origin) else {
            return Ok(false);
        };
        let Some(value) = storage.entries.get(key) else {
            return Ok(false);
        };
        let removed = checked_entry_bytes(key.len(), value.len())?;
        let next_origin_bytes = storage.bytes.checked_sub(removed).ok_or_else(|| {
            StorageError::new(
                StorageErrorKind::AccountingOverflow,
                "storage origin byte accounting underflow",
            )
        })?;
        let next_total_bytes = self.total_bytes.checked_sub(removed).ok_or_else(|| {
            StorageError::new(
                StorageErrorKind::AccountingOverflow,
                "storage process byte accounting underflow",
            )
        })?;

        let storage = self.origins.get_mut(origin).ok_or_else(|| {
            StorageError::new(
                StorageErrorKind::AccountingOverflow,
                "storage origin disappeared during a single-threaded mutation",
            )
        })?;
        if storage.entries.remove(key).is_none() {
            return Err(StorageError::new(
                StorageErrorKind::AccountingOverflow,
                "storage entry disappeared during a single-threaded mutation",
            ));
        }
        storage.bytes = next_origin_bytes;
        self.total_bytes = next_total_bytes;
        if storage.entries.is_empty() {
            self.origins.remove(origin);
        }
        Ok(true)
    }

    pub fn clear_origin(&mut self, origin: &Origin) -> Result<bool, StorageError> {
        let Some(storage) = self.origins.get(origin) else {
            return Ok(false);
        };
        let next_total_bytes = self.total_bytes.checked_sub(storage.bytes).ok_or_else(|| {
            StorageError::new(
                StorageErrorKind::AccountingOverflow,
                "storage process byte accounting underflow",
            )
        })?;
        self.origins.remove(origin);
        self.total_bytes = next_total_bytes;
        Ok(true)
    }

    fn validate_persistent_origin(&self, origin: &Origin) -> Result<(), StorageError> {
        if origin.is_opaque() {
            Err(StorageError::new(
                StorageErrorKind::OpaqueOrigin,
                "opaque origins do not receive persistent storage authority",
            ))
        } else {
            Ok(())
        }
    }
}

fn checked_entry_bytes(key_bytes: usize, value_bytes: usize) -> Result<usize, StorageError> {
    key_bytes.checked_add(value_bytes).ok_or_else(|| {
        StorageError::new(
            StorageErrorKind::AccountingOverflow,
            "storage entry byte accounting overflow",
        )
    })
}

fn replaced_bytes(current: usize, old: usize, new: usize) -> Result<usize, StorageError> {
    current
        .checked_sub(old)
        .and_then(|without_old| without_old.checked_add(new))
        .ok_or_else(|| {
            StorageError::new(
                StorageErrorKind::AccountingOverflow,
                "storage quota byte accounting overflow",
            )
        })
}

fn try_owned_string(value: &str) -> Result<String, StorageError> {
    let mut owned = String::new();
    owned
        .try_reserve_exact(value.len())
        .map_err(|_| allocation_error())?;
    owned.push_str(value);
    Ok(owned)
}

fn try_owned_bytes(value: &[u8]) -> Result<Vec<u8>, StorageError> {
    let mut owned = Vec::new();
    owned
        .try_reserve_exact(value.len())
        .map_err(|_| allocation_error())?;
    owned.extend_from_slice(value);
    Ok(owned)
}

fn allocation_error() -> StorageError {
    StorageError::new(
        StorageErrorKind::AllocationFailed,
        "storage allocation failed within the configured quota",
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

    fn limits() -> StorageLimits {
        StorageLimits {
            max_origins: 2,
            max_entries_per_origin: 2,
            max_key_bytes: 8,
            max_value_bytes: 16,
            max_origin_bytes: 20,
            max_total_bytes: 24,
        }
    }

    #[test]
    fn exact_origins_are_separate_even_when_they_are_same_site() {
        let first = origin("https://a.example.com/");
        let second = origin("https://b.example.com/");
        assert!(first.site().same_site(&second.site()));
        assert!(!first.same_origin(&second));

        let mut storage = StorageProcessState::try_new(process(), limits()).unwrap();
        storage.put(&first, "key", b"one").unwrap();
        storage.put(&second, "key", b"two").unwrap();

        assert_eq!(storage.get(&first, "key"), Some(b"one".as_slice()));
        assert_eq!(storage.get(&second, "key"), Some(b"two".as_slice()));
        assert_eq!(storage.origin_count(), 2);
    }

    #[test]
    fn opaque_origin_is_rejected_without_mutation() {
        let opaque = origin("data:text/plain,hello");
        let mut storage = StorageProcessState::try_new(process(), limits()).unwrap();

        let error = storage.put(&opaque, "key", b"value").unwrap_err();

        assert_eq!(error.kind, StorageErrorKind::OpaqueOrigin);
        assert_eq!(storage.origin_count(), 0);
        assert_eq!(storage.total_bytes(), 0);
    }

    #[test]
    fn replacement_quota_failure_is_atomic() {
        let origin = origin("https://example.com/");
        let mut storage = StorageProcessState::try_new(process(), limits()).unwrap();
        storage.put(&origin, "k", b"old").unwrap();
        let before_bytes = storage.total_bytes();

        let error = storage.put(&origin, "k", &[7; 17]).unwrap_err();

        assert_eq!(error.kind, StorageErrorKind::ValueTooLarge);
        assert_eq!(storage.get(&origin, "k"), Some(b"old".as_slice()));
        assert_eq!(storage.total_bytes(), before_bytes);
    }

    #[test]
    fn entry_origin_and_global_quotas_fail_before_mutation() {
        let first = origin("https://a.example/");
        let second = origin("https://b.example/");
        let third = origin("https://c.example/");
        let mut storage = StorageProcessState::try_new(process(), limits()).unwrap();

        storage.put(&first, "a", &[1; 8]).unwrap();
        storage.put(&first, "b", &[2; 8]).unwrap();
        let entry_error = storage.put(&first, "c", b"x").unwrap_err();
        assert_eq!(entry_error.kind, StorageErrorKind::EntryLimitExceeded);

        storage.put(&second, "a", b"x").unwrap();
        let origin_error = storage.put(&third, "a", b"x").unwrap_err();
        assert_eq!(origin_error.kind, StorageErrorKind::OriginLimitExceeded);
        assert_eq!(storage.origin_count(), 2);
        assert_eq!(storage.get(&third, "a"), None);
    }

    #[test]
    fn origin_byte_quota_failure_preserves_existing_value() {
        let origin = origin("https://example.com/");
        let mut storage = StorageProcessState::try_new(process(), limits()).unwrap();
        storage.put(&origin, "a", &[1; 8]).unwrap();
        storage.put(&origin, "b", &[2; 8]).unwrap();

        let error = storage.put(&origin, "a", &[3; 16]).unwrap_err();

        assert_eq!(error.kind, StorageErrorKind::OriginByteLimitExceeded);
        assert_eq!(storage.get(&origin, "a"), Some([1; 8].as_slice()));
        assert_eq!(storage.get(&origin, "b"), Some([2; 8].as_slice()));
    }

    #[test]
    fn total_byte_quota_failure_is_atomic() {
        let first = origin("https://a.example/");
        let second = origin("https://b.example/");
        let mut storage = StorageProcessState::try_new(process(), limits()).unwrap();
        storage.put(&first, "a", &[1; 10]).unwrap();
        storage.put(&second, "b", &[2; 10]).unwrap();
        let before = storage.total_bytes();

        let error = storage.put(&second, "b", &[3; 13]).unwrap_err();

        assert_eq!(error.kind, StorageErrorKind::TotalByteLimitExceeded);
        assert_eq!(storage.get(&second, "b"), Some([2; 10].as_slice()));
        assert_eq!(storage.total_bytes(), before);
    }

    #[test]
    fn removal_releases_origin_and_global_quota() {
        let first = origin("https://a.example/");
        let second = origin("https://b.example/");
        let third = origin("https://c.example/");
        let mut storage = StorageProcessState::try_new(process(), limits()).unwrap();
        storage.put(&first, "a", b"one").unwrap();
        storage.put(&second, "b", b"two").unwrap();

        assert!(storage.clear_origin(&first).unwrap());
        assert_eq!(storage.origin_count(), 1);
        storage.put(&third, "c", b"three").unwrap();
        assert_eq!(storage.origin_count(), 2);

        assert!(storage.remove(&second, "b").unwrap());
        assert_eq!(storage.entry_count(&second), 0);
        assert_eq!(storage.origin_count(), 1);
    }

    #[test]
    fn invalid_nested_limits_are_rejected() {
        let invalid = StorageLimits {
            max_origin_bytes: 4,
            max_value_bytes: 5,
            ..limits()
        };
        let error = StorageProcessState::try_new(process(), invalid).unwrap_err();
        assert_eq!(error.kind, StorageErrorKind::InvalidLimits);
    }
}
