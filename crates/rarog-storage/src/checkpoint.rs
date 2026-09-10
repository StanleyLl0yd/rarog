use crate::state::{
    MAX_PERSISTENT_ORIGIN_IDENTITY_BYTES, StorageError, StorageErrorKind, StorageProcessState,
    validate_persistent_origin_identity,
};
use rarog_url::{Origin, UrlHost};
use std::cmp::Ordering;
use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr};

const STORAGE_CHECKPOINT_MAGIC: &[u8; 8] = b"RAROGST1";
pub const STORAGE_CHECKPOINT_VERSION: u16 = 1;
pub const DEFAULT_MAX_STORAGE_CHECKPOINT_BYTES: usize = 320 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageCheckpointLimits {
    pub max_checkpoint_bytes: usize,
}

impl StorageCheckpointLimits {
    pub fn is_valid(self) -> bool {
        self.max_checkpoint_bytes >= STORAGE_CHECKPOINT_MAGIC.len() + 2 + 4
    }
}

impl Default for StorageCheckpointLimits {
    fn default() -> Self {
        Self {
            max_checkpoint_bytes: DEFAULT_MAX_STORAGE_CHECKPOINT_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageCheckpointErrorKind {
    InvalidLimits,
    CheckpointTooLarge,
    InvalidMagic,
    UnsupportedVersion,
    Truncated,
    InvalidUtf8,
    InvalidHostTag,
    NonCanonicalOrder,
    TrailingData,
    LengthOverflow,
    AllocationFailed,
    Storage(StorageErrorKind),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageCheckpointError {
    pub kind: StorageCheckpointErrorKind,
    pub message: String,
}

impl StorageCheckpointError {
    fn new(kind: StorageCheckpointErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    fn storage(kind: StorageErrorKind, message: impl Into<String>) -> Self {
        Self::new(StorageCheckpointErrorKind::Storage(kind), message)
    }
}

impl fmt::Display for StorageCheckpointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for StorageCheckpointError {}

impl From<StorageError> for StorageCheckpointError {
    fn from(error: StorageError) -> Self {
        Self::new(
            StorageCheckpointErrorKind::Storage(error.kind),
            error.message,
        )
    }
}

pub fn encode_storage_checkpoint(
    storage: &StorageProcessState,
    limits: StorageCheckpointLimits,
) -> Result<Vec<u8>, StorageCheckpointError> {
    validate_limits(limits)?;

    let mut origins = Vec::new();
    origins
        .try_reserve_exact(storage.origins.len())
        .map_err(|_| allocation_error())?;
    for origin in storage.origins.keys() {
        validate_persistent_origin_identity(origin)?;
        origins.push(origin);
    }
    origins.sort_by(|left, right| compare_origins(left, right));

    let origin_count = u32::try_from(origins.len()).map_err(|_| length_overflow())?;
    let mut output = Vec::new();
    append(&mut output, STORAGE_CHECKPOINT_MAGIC, limits)?;
    append(
        &mut output,
        &STORAGE_CHECKPOINT_VERSION.to_be_bytes(),
        limits,
    )?;
    append(&mut output, &origin_count.to_be_bytes(), limits)?;

    for origin in origins {
        encode_origin(&mut output, origin, limits)?;
        let origin_storage = storage.origins.get(origin).ok_or_else(|| {
            StorageCheckpointError::new(
                StorageCheckpointErrorKind::NonCanonicalOrder,
                "storage origin disappeared during checkpoint encoding",
            )
        })?;
        let mut keys = Vec::new();
        keys.try_reserve_exact(origin_storage.entries.len())
            .map_err(|_| allocation_error())?;
        keys.extend(origin_storage.entries.keys());
        keys.sort();

        let entry_count = u32::try_from(keys.len()).map_err(|_| length_overflow())?;
        append(&mut output, &entry_count.to_be_bytes(), limits)?;
        for key in keys {
            let value = origin_storage.entries.get(key).ok_or_else(|| {
                StorageCheckpointError::new(
                    StorageCheckpointErrorKind::NonCanonicalOrder,
                    "storage entry disappeared during checkpoint encoding",
                )
            })?;
            encode_length_prefixed(&mut output, key.as_bytes(), limits)?;
            encode_length_prefixed(&mut output, value, limits)?;
        }
    }

    Ok(output)
}

pub fn restore_storage_checkpoint(
    storage: &mut StorageProcessState,
    checkpoint: &[u8],
    limits: StorageCheckpointLimits,
) -> Result<(), StorageCheckpointError> {
    validate_limits(limits)?;
    if checkpoint.len() > limits.max_checkpoint_bytes {
        return Err(StorageCheckpointError::new(
            StorageCheckpointErrorKind::CheckpointTooLarge,
            format!(
                "storage checkpoint requires {} bytes; limit is {}",
                checkpoint.len(),
                limits.max_checkpoint_bytes
            ),
        ));
    }

    let mut cursor = CheckpointCursor::new(checkpoint);
    if cursor.take(STORAGE_CHECKPOINT_MAGIC.len())? != STORAGE_CHECKPOINT_MAGIC {
        return Err(StorageCheckpointError::new(
            StorageCheckpointErrorKind::InvalidMagic,
            "storage checkpoint magic does not match",
        ));
    }
    let version = cursor.read_u16()?;
    if version != STORAGE_CHECKPOINT_VERSION {
        return Err(StorageCheckpointError::new(
            StorageCheckpointErrorKind::UnsupportedVersion,
            format!("unsupported storage checkpoint version {version}"),
        ));
    }

    let storage_limits = storage.limits();
    let origin_count = cursor.read_u32()? as usize;
    if origin_count > storage_limits.max_origins {
        return Err(StorageCheckpointError::storage(
            StorageErrorKind::OriginLimitExceeded,
            format!(
                "storage checkpoint declares {origin_count} origins; limit is {}",
                storage_limits.max_origins
            ),
        ));
    }

    let mut candidate = StorageProcessState::try_new(storage.process(), storage_limits)?;
    let mut previous_origin: Option<Origin> = None;
    for _ in 0..origin_count {
        let origin = decode_origin(&mut cursor)?;
        validate_persistent_origin_identity(&origin)?;
        if previous_origin
            .as_ref()
            .is_some_and(|previous| compare_origins(previous, &origin) != Ordering::Less)
        {
            return Err(StorageCheckpointError::new(
                StorageCheckpointErrorKind::NonCanonicalOrder,
                "storage checkpoint origins are duplicated or not in canonical order",
            ));
        }

        let entry_count = cursor.read_u32()? as usize;
        if entry_count > storage_limits.max_entries_per_origin {
            return Err(StorageCheckpointError::storage(
                StorageErrorKind::EntryLimitExceeded,
                format!(
                    "storage checkpoint declares {entry_count} entries for one origin; limit is {}",
                    storage_limits.max_entries_per_origin
                ),
            ));
        }

        let mut previous_key: Option<String> = None;
        for _ in 0..entry_count {
            let key = cursor
                .read_bounded_string(storage_limits.max_key_bytes, StorageErrorKind::KeyTooLarge)?;
            if previous_key
                .as_ref()
                .is_some_and(|previous| previous >= &key)
            {
                return Err(StorageCheckpointError::new(
                    StorageCheckpointErrorKind::NonCanonicalOrder,
                    "storage checkpoint keys are duplicated or not in canonical order",
                ));
            }
            let value = cursor.read_bounded_bytes(
                storage_limits.max_value_bytes,
                StorageErrorKind::ValueTooLarge,
            )?;
            candidate.put(&origin, &key, &value)?;
            previous_key = Some(key);
        }
        previous_origin = Some(origin);
    }

    if cursor.remaining() != 0 {
        return Err(StorageCheckpointError::new(
            StorageCheckpointErrorKind::TrailingData,
            format!(
                "storage checkpoint has {} unexpected trailing bytes",
                cursor.remaining()
            ),
        ));
    }

    *storage = candidate;
    Ok(())
}

fn validate_limits(limits: StorageCheckpointLimits) -> Result<(), StorageCheckpointError> {
    if limits.is_valid() {
        Ok(())
    } else {
        Err(StorageCheckpointError::new(
            StorageCheckpointErrorKind::InvalidLimits,
            "storage checkpoint byte limit is too small for the checkpoint header",
        ))
    }
}

fn encode_origin(
    output: &mut Vec<u8>,
    origin: &Origin,
    limits: StorageCheckpointLimits,
) -> Result<(), StorageCheckpointError> {
    let Origin::Tuple { scheme, host, port } = origin else {
        return Err(StorageCheckpointError::storage(
            StorageErrorKind::OpaqueOrigin,
            "opaque origins cannot be persisted in storage checkpoints",
        ));
    };
    encode_length_prefixed(output, scheme.as_bytes(), limits)?;
    match host {
        UrlHost::Domain(domain) => {
            append(output, &[0], limits)?;
            encode_length_prefixed(output, domain.as_bytes(), limits)?;
        }
        UrlHost::Ipv4(address) => {
            append(output, &[1], limits)?;
            append(output, &address.octets(), limits)?;
        }
        UrlHost::Ipv6(address) => {
            append(output, &[2], limits)?;
            append(output, &address.octets(), limits)?;
        }
    }
    append(output, &port.to_be_bytes(), limits)
}

fn decode_origin(cursor: &mut CheckpointCursor<'_>) -> Result<Origin, StorageCheckpointError> {
    let scheme = cursor.read_bounded_string(
        MAX_PERSISTENT_ORIGIN_IDENTITY_BYTES,
        StorageErrorKind::OriginIdentityTooLarge,
    )?;
    let host = match cursor.read_u8()? {
        0 => UrlHost::Domain(cursor.read_bounded_string(
            MAX_PERSISTENT_ORIGIN_IDENTITY_BYTES,
            StorageErrorKind::OriginIdentityTooLarge,
        )?),
        1 => {
            let bytes = cursor.take(4)?;
            UrlHost::Ipv4(Ipv4Addr::from([bytes[0], bytes[1], bytes[2], bytes[3]]))
        }
        2 => {
            let bytes = cursor.take(16)?;
            let mut octets = [0_u8; 16];
            octets.copy_from_slice(bytes);
            UrlHost::Ipv6(Ipv6Addr::from(octets))
        }
        tag => {
            return Err(StorageCheckpointError::new(
                StorageCheckpointErrorKind::InvalidHostTag,
                format!("invalid storage checkpoint host tag {tag}"),
            ));
        }
    };
    let port = cursor.read_u16()?;
    Ok(Origin::Tuple { scheme, host, port })
}

fn compare_origins(left: &Origin, right: &Origin) -> Ordering {
    match (left, right) {
        (
            Origin::Tuple {
                scheme: left_scheme,
                host: left_host,
                port: left_port,
            },
            Origin::Tuple {
                scheme: right_scheme,
                host: right_host,
                port: right_port,
            },
        ) => left_scheme
            .cmp(right_scheme)
            .then_with(|| compare_hosts(left_host, right_host))
            .then_with(|| left_port.cmp(right_port)),
        (Origin::Tuple { .. }, Origin::Opaque(_)) => Ordering::Less,
        (Origin::Opaque(_), Origin::Tuple { .. }) => Ordering::Greater,
        (Origin::Opaque(left), Origin::Opaque(right)) => left.cmp(right),
    }
}

fn compare_hosts(left: &UrlHost, right: &UrlHost) -> Ordering {
    let rank = |host: &UrlHost| match host {
        UrlHost::Domain(_) => 0_u8,
        UrlHost::Ipv4(_) => 1_u8,
        UrlHost::Ipv6(_) => 2_u8,
    };
    rank(left)
        .cmp(&rank(right))
        .then_with(|| match (left, right) {
            (UrlHost::Domain(left), UrlHost::Domain(right)) => left.cmp(right),
            (UrlHost::Ipv4(left), UrlHost::Ipv4(right)) => left.octets().cmp(&right.octets()),
            (UrlHost::Ipv6(left), UrlHost::Ipv6(right)) => left.octets().cmp(&right.octets()),
            _ => Ordering::Equal,
        })
}

fn encode_length_prefixed(
    output: &mut Vec<u8>,
    bytes: &[u8],
    limits: StorageCheckpointLimits,
) -> Result<(), StorageCheckpointError> {
    let length = u32::try_from(bytes.len()).map_err(|_| length_overflow())?;
    append(output, &length.to_be_bytes(), limits)?;
    append(output, bytes, limits)
}

fn append(
    output: &mut Vec<u8>,
    bytes: &[u8],
    limits: StorageCheckpointLimits,
) -> Result<(), StorageCheckpointError> {
    let next = output
        .len()
        .checked_add(bytes.len())
        .ok_or_else(length_overflow)?;
    if next > limits.max_checkpoint_bytes {
        return Err(StorageCheckpointError::new(
            StorageCheckpointErrorKind::CheckpointTooLarge,
            format!(
                "storage checkpoint would require {next} bytes; limit is {}",
                limits.max_checkpoint_bytes
            ),
        ));
    }
    output
        .try_reserve(bytes.len())
        .map_err(|_| allocation_error())?;
    output.extend_from_slice(bytes);
    Ok(())
}

struct CheckpointCursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> CheckpointCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], StorageCheckpointError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or_else(length_overflow)?;
        let bytes = self.bytes.get(self.position..end).ok_or_else(|| {
            StorageCheckpointError::new(
                StorageCheckpointErrorKind::Truncated,
                "storage checkpoint ended before the declared field length",
            )
        })?;
        self.position = end;
        Ok(bytes)
    }

    fn read_u8(&mut self) -> Result<u8, StorageCheckpointError> {
        Ok(self.take(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, StorageCheckpointError> {
        let bytes = self.take(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, StorageCheckpointError> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_bounded_string(
        &mut self,
        limit: usize,
        limit_kind: StorageErrorKind,
    ) -> Result<String, StorageCheckpointError> {
        let bytes = self.read_bounded_bytes(limit, limit_kind)?;
        String::from_utf8(bytes).map_err(|_| {
            StorageCheckpointError::new(
                StorageCheckpointErrorKind::InvalidUtf8,
                "storage checkpoint string field is not valid UTF-8",
            )
        })
    }

    fn read_bounded_bytes(
        &mut self,
        limit: usize,
        limit_kind: StorageErrorKind,
    ) -> Result<Vec<u8>, StorageCheckpointError> {
        let length = self.read_u32()? as usize;
        if length > limit {
            return Err(StorageCheckpointError::storage(
                limit_kind,
                format!("storage checkpoint field requires {length} bytes; limit is {limit}"),
            ));
        }
        let source = self.take(length)?;
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(length)
            .map_err(|_| allocation_error())?;
        owned.extend_from_slice(source);
        Ok(owned)
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.position)
    }
}

fn allocation_error() -> StorageCheckpointError {
    StorageCheckpointError::new(
        StorageCheckpointErrorKind::AllocationFailed,
        "storage checkpoint allocation failed within configured bounds",
    )
}

fn length_overflow() -> StorageCheckpointError {
    StorageCheckpointError::new(
        StorageCheckpointErrorKind::LengthOverflow,
        "storage checkpoint length accounting overflow",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::StorageLimits;
    use rarog_process::{ProcessTopology, StorageProcessId};
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
            max_origins: 4,
            max_entries_per_origin: 8,
            max_key_bytes: 16,
            max_value_bytes: 32,
            max_origin_bytes: 128,
            max_total_bytes: 256,
        }
    }

    fn checkpoint_limits() -> StorageCheckpointLimits {
        StorageCheckpointLimits {
            max_checkpoint_bytes: 4096,
        }
    }

    #[test]
    fn checkpoint_round_trip_preserves_exact_origins_entries_and_accounting() {
        let process = process();
        let first = origin("https://a.example.com/");
        let second = origin("https://b.example.com:8443/");
        let mut source = StorageProcessState::try_new(process, limits()).unwrap();
        source.put(&second, "z", b"last").unwrap();
        source.put(&first, "b", b"two").unwrap();
        source.put(&first, "a", b"one").unwrap();
        let bytes = encode_storage_checkpoint(&source, checkpoint_limits()).unwrap();

        let mut restored = StorageProcessState::try_new(process, limits()).unwrap();
        restore_storage_checkpoint(&mut restored, &bytes, checkpoint_limits()).unwrap();

        assert_eq!(restored.origin_count(), 2);
        assert_eq!(restored.total_bytes(), source.total_bytes());
        assert_eq!(restored.origin_bytes(&first), source.origin_bytes(&first));
        assert_eq!(restored.get(&first, "a"), Some(b"one".as_slice()));
        assert_eq!(restored.get(&first, "b"), Some(b"two".as_slice()));
        assert_eq!(restored.get(&second, "z"), Some(b"last".as_slice()));
    }

    #[test]
    fn checkpoint_encoding_is_deterministic_across_insertion_order() {
        let process = process();
        let first = origin("https://a.example/");
        let second = origin("https://b.example/");
        let mut left = StorageProcessState::try_new(process, limits()).unwrap();
        left.put(&second, "z", b"2").unwrap();
        left.put(&first, "a", b"1").unwrap();
        let mut right = StorageProcessState::try_new(process, limits()).unwrap();
        right.put(&first, "a", b"1").unwrap();
        right.put(&second, "z", b"2").unwrap();

        assert_eq!(
            encode_storage_checkpoint(&left, checkpoint_limits()).unwrap(),
            encode_storage_checkpoint(&right, checkpoint_limits()).unwrap()
        );
    }

    #[test]
    fn restore_keeps_target_process_identity() {
        let first_process = process();
        let mut topology = ProcessTopology::try_new(1).unwrap();
        topology
            .assign_site(
                WebUrl::parse("https://site.example/")
                    .unwrap()
                    .site_identity()
                    .unwrap(),
            )
            .unwrap();
        let second_process = topology.ensure_storage_process().unwrap().process();
        assert_ne!(first_process, second_process);
        let exact = origin("https://example.com/");
        let mut source = StorageProcessState::try_new(first_process, limits()).unwrap();
        source.put(&exact, "key", b"value").unwrap();
        let checkpoint = encode_storage_checkpoint(&source, checkpoint_limits()).unwrap();
        let mut target = StorageProcessState::try_new(second_process, limits()).unwrap();

        restore_storage_checkpoint(&mut target, &checkpoint, checkpoint_limits()).unwrap();
        assert_eq!(target.process(), second_process);
        assert_eq!(target.get(&exact, "key"), Some(b"value".as_slice()));
    }

    #[test]
    fn malformed_truncated_version_and_trailing_data_fail_without_mutation() {
        let process = process();
        let exact = origin("https://example.com/");
        let mut source = StorageProcessState::try_new(process, limits()).unwrap();
        source.put(&exact, "key", b"new").unwrap();
        let valid = encode_storage_checkpoint(&source, checkpoint_limits()).unwrap();

        let cases = [
            {
                let mut bytes = valid.clone();
                bytes[0] ^= 0xff;
                bytes
            },
            valid[..valid.len() - 1].to_vec(),
            {
                let mut bytes = valid.clone();
                bytes[8..10].copy_from_slice(&(STORAGE_CHECKPOINT_VERSION + 1).to_be_bytes());
                bytes
            },
            {
                let mut bytes = valid.clone();
                bytes.push(0);
                bytes
            },
        ];

        for bytes in cases {
            let mut target = StorageProcessState::try_new(process, limits()).unwrap();
            target.put(&exact, "key", b"old").unwrap();
            assert!(
                restore_storage_checkpoint(&mut target, &bytes, checkpoint_limits()).is_err()
            );
            assert_eq!(target.get(&exact, "key"), Some(b"old".as_slice()));
            assert_eq!(target.origin_count(), 1);
        }
    }

    #[test]
    fn checkpoint_size_limit_is_enforced_before_decode_or_encode_growth() {
        let process = process();
        let exact = origin("https://example.com/");
        let mut storage = StorageProcessState::try_new(process, limits()).unwrap();
        storage.put(&exact, "key", &[7; 32]).unwrap();
        let tiny = StorageCheckpointLimits {
            max_checkpoint_bytes: STORAGE_CHECKPOINT_MAGIC.len() + 2 + 4,
        };
        assert_eq!(
            encode_storage_checkpoint(&storage, tiny).unwrap_err().kind,
            StorageCheckpointErrorKind::CheckpointTooLarge
        );

        let oversized = vec![0_u8; tiny.max_checkpoint_bytes + 1];
        assert_eq!(
            restore_storage_checkpoint(&mut storage, &oversized, tiny)
                .unwrap_err()
                .kind,
            StorageCheckpointErrorKind::CheckpointTooLarge
        );
    }

    #[test]
    fn restore_quota_failure_is_transactional() {
        let process = process();
        let exact = origin("https://example.com/");
        let mut source = StorageProcessState::try_new(process, limits()).unwrap();
        source.put(&exact, "a", b"12345678").unwrap();
        source.put(&exact, "b", b"12345678").unwrap();
        let checkpoint = encode_storage_checkpoint(&source, checkpoint_limits()).unwrap();
        let tight = StorageLimits {
            max_value_bytes: 12,
            max_origin_bytes: 12,
            max_total_bytes: 12,
            ..limits()
        };
        let mut target = StorageProcessState::try_new(process, tight).unwrap();
        target.put(&exact, "old", b"value").unwrap();
        let before = target.total_bytes();

        assert!(
            restore_storage_checkpoint(&mut target, &checkpoint, checkpoint_limits()).is_err()
        );
        assert_eq!(target.get(&exact, "old"), Some(b"value".as_slice()));
        assert_eq!(target.get(&exact, "a"), None);
        assert_eq!(target.total_bytes(), before);
    }
}
