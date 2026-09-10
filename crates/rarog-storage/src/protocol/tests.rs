use super::*;
use crate::state::{StorageLimits, StorageProcessState};
use rarog_process::{ProcessTopology, StorageProcessId};
use rarog_url::{Origin, WebUrl};

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
        .assign_site(
            WebUrl::parse("https://example.org/")
                .unwrap()
                .site_identity()
                .unwrap(),
        )
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

fn queue(process: StorageProcessId) -> StorageRequestQueue {
    StorageRequestQueue::try_new(process, storage_limits(), request_limits()).unwrap()
}

fn execute_next(
    queue: &mut StorageRequestQueue,
    storage: &mut StorageProcessState,
) -> StorageResponsePayload {
    let request = queue.take_next().unwrap();
    let response = execute_storage_request(storage, &request).unwrap();
    queue.complete(response).unwrap()
}

#[test]
fn typed_request_response_round_trips_all_operations() {
    let process = process();
    let origin = origin("https://example.com/");
    let mut storage = StorageProcessState::try_new(process, storage_limits()).unwrap();
    let mut queue = queue(process);

    queue
        .enqueue(
            origin.clone(),
            StorageCommand::Put {
                key: String::from("key"),
                value: b"value".to_vec(),
            },
        )
        .unwrap();
    assert_eq!(
        execute_next(&mut queue, &mut storage),
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
    assert_eq!(
        execute_next(&mut queue, &mut storage),
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
    assert_eq!(
        execute_next(&mut queue, &mut storage),
        StorageResponsePayload::Removed(true)
    );

    queue.enqueue(origin, StorageCommand::Clear).unwrap();
    assert_eq!(
        execute_next(&mut queue, &mut storage),
        StorageResponsePayload::Cleared(false)
    );
    assert_eq!(queue.pending_requests(), 0);
    assert_eq!(queue.tracked_bytes(), 0);
}

#[test]
fn pending_count_and_byte_limits_apply_before_enqueue() {
    let process = process();
    let origin = origin("https://example.com/");
    let mut count = StorageRequestQueue::try_new(
        process,
        storage_limits(),
        StorageRequestLimits {
            max_request_bytes: 64,
            max_pending_requests: 1,
            max_pending_bytes: 64,
        },
    )
    .unwrap();
    count
        .enqueue(
            origin.clone(),
            StorageCommand::Get {
                key: String::from("a"),
            },
        )
        .unwrap();
    let before = count.tracked_bytes();
    assert_eq!(
        count
            .enqueue(
                origin.clone(),
                StorageCommand::Get {
                    key: String::from("b"),
                },
            )
            .unwrap_err()
            .kind,
        StorageProtocolErrorKind::PendingRequestLimitExceeded
    );
    assert_eq!(count.pending_requests(), 1);
    assert_eq!(count.tracked_bytes(), before);

    let mut bytes = StorageRequestQueue::try_new(
        process,
        storage_limits(),
        StorageRequestLimits {
            max_request_bytes: 30,
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
    let mut queue = queue(process);
    assert_eq!(
        queue
            .enqueue(origin("data:text/plain,hello"), StorageCommand::Clear)
            .unwrap_err()
            .kind,
        StorageProtocolErrorKind::OpaqueOrigin
    );
    assert_eq!(
        queue
            .enqueue(
                origin("https://example.com/"),
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
    let mut queue = queue(process);

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
    let mut queue = queue(process);
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
    let mut storage = StorageProcessState::try_new(process, storage_limits()).unwrap();
    let mut queue = queue(process);
    let id = queue
        .enqueue(
            origin("https://example.com/"),
            StorageCommand::Get {
                key: String::from("key"),
            },
        )
        .unwrap();
    let request = queue.take_next().unwrap();
    let valid = execute_storage_request(&mut storage, &request).unwrap();
    let mismatched = StorageResponse::new(
        id,
        process,
        StorageOperationKind::Put,
        StorageResponsePayload::Written,
    );

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