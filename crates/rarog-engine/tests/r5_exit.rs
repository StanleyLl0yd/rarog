use rarog_accessibility::{AccessibilityError, AccessibilityLimits, AccessibilityTreeState};
use rarog_canvas::{CanvasLimits, CanvasRegistry};
use rarog_engine::{RenderOptions, RenderSession};
use rarog_media::{
    MediaError, MediaLimits, MediaRegistry, MediaResourceDescriptor, MediaStreamDescriptor, MediaTime,
};
use rarog_process::ProcessTopology;
use rarog_storage::{StorageErrorKind, StorageLimits, StorageProcessState};
use rarog_types::Size;
use rarog_url::WebUrl;
use rarog_webgl::{
    WebGlContextLossReason, WebGlContextState, WebGlError, WebGlLimits, WebGlRegistry,
};
use rarog_websocket::{
    WebSocketLifecycle, WebSocketLifecycleErrorKind, WebSocketLimits, WebSocketMessage,
    WebSocketMessageQueues, WebSocketQueueErrorKind, WebSocketQueueLimits, WebSocketReadyState,
};
use rarog_workers::{
    ServiceWorkerError, ServiceWorkerLimits, ServiceWorkerRegistry, WorkerErrorKind, WorkerLimits,
    WorkerMessageError, WorkerMessageLimits, WorkerMessageMailbox, WorkerMessageValue,
    WorkerRegistry,
};

const R5_BACKLOG: &str = include_str!("../../../docs/R5-BACKLOG.md");
const R5_EXIT: &str = include_str!("../../../docs/R5-EXIT.md");
const CI: &str = include_str!("../../../.github/workflows/ci.yml");
const STORAGE_CARGO: &str = include_str!("../../rarog-storage/Cargo.toml");
const WORKERS_CARGO: &str = include_str!("../../rarog-workers/Cargo.toml");
const WEBSOCKET_CARGO: &str = include_str!("../../rarog-websocket/Cargo.toml");
const MEDIA_CARGO: &str = include_str!("../../rarog-media/Cargo.toml");
const CANVAS_CARGO: &str = include_str!("../../rarog-canvas/Cargo.toml");
const WEBGL_CARGO: &str = include_str!("../../rarog-webgl/Cargo.toml");
const ACCESSIBILITY_CARGO: &str = include_str!("../../rarog-accessibility/Cargo.toml");

fn web_url(value: &str) -> WebUrl {
    WebUrl::parse(value).unwrap()
}

#[test]
fn r5_exit_manifest_ci_and_stop_boundary_are_wired() {
    let selected_work = R5_BACKLOG.split("## G — Milestone exit").next().unwrap();
    assert!(
        !selected_work
            .lines()
            .any(|line| line.trim_start().starts_with("- [ ]")),
        "R5 selected workstreams contain an unchecked item"
    );
    assert!(R5_EXIT.contains("R5 exit is an architecture/correctness gate"));
    assert!(R5_EXIT.contains("WebDriver/BiDi"));
    assert!(R5_BACKLOG.contains("Do not begin R6 compatibility qualification"));
    assert_eq!(CI.matches("name: R5 exit gate").count(), 2);
    assert_eq!(
        CI.matches("cargo test --locked -p rarog-engine --test r5_exit")
            .count(),
        2
    );

    for (name, manifest) in [
        ("storage", STORAGE_CARGO),
        ("workers", WORKERS_CARGO),
        ("websocket", WEBSOCKET_CARGO),
        ("media", MEDIA_CARGO),
        ("canvas", CANVAS_CARGO),
        ("webgl", WEBGL_CARGO),
        ("accessibility", ACCESSIBILITY_CARGO),
    ] {
        assert!(
            !manifest.contains("rarog-platform-windows")
                && !manifest.contains("windows-sys")
                && !manifest.contains("windows-core"),
            "portable R5 semantic crate {name} depends on a Windows-native authority crate"
        );
    }
}

#[test]
fn r5_exit_storage_identity_origin_isolation_and_limits_fail_closed() {
    let mut topology = ProcessTopology::try_new(1).unwrap();
    let site_url = web_url("https://app.example.com/");
    let site = topology.assign_site(site_url.site_identity().unwrap()).unwrap();
    let first_storage = topology.ensure_storage_process().unwrap();
    assert_ne!(site.process().get(), first_storage.process().get());

    topology
        .retire_storage_process(first_storage.process())
        .unwrap();
    let replacement = topology.ensure_storage_process().unwrap();
    assert_ne!(first_storage.process(), replacement.process());

    let limits = StorageLimits {
        max_origins: 2,
        max_entries_per_origin: 1,
        max_key_bytes: 8,
        max_value_bytes: 8,
        max_origin_bytes: 16,
        max_total_bytes: 32,
    };
    let mut storage = StorageProcessState::try_new(replacement.process(), limits).unwrap();
    let app = web_url("https://app.example.com/");
    let cdn = web_url("https://cdn.example.com/");
    assert_eq!(app.site_identity().unwrap(), cdn.site_identity().unwrap());
    let app_origin = app.origin().unwrap();
    let cdn_origin = cdn.origin().unwrap();
    assert_ne!(app_origin, cdn_origin);

    storage.put(&app_origin, "key", b"value").unwrap();
    assert_eq!(storage.get(&app_origin, "key"), Some(b"value".as_slice()));
    assert_eq!(storage.get(&cdn_origin, "key"), None);

    let error = storage.put(&app_origin, "second", b"x").unwrap_err();
    assert_eq!(error.kind, StorageErrorKind::EntryLimitExceeded);
    let error = storage.put(&cdn_origin, "key", b"012345678").unwrap_err();
    assert_eq!(error.kind, StorageErrorKind::ValueTooLarge);
}

#[test]
fn r5_exit_workers_and_service_workers_bound_lifecycle_and_stale_identity() {
    let mut workers = WorkerRegistry::try_new(WorkerLimits {
        max_workers: 1,
        max_children_per_owner: 1,
        max_depth: 1,
    })
    .unwrap();
    let owner = 7_u64;
    let worker = workers.create_root(owner).unwrap();
    workers.mark_running(worker).unwrap();
    assert_eq!(workers.live_workers(), 1);
    assert_eq!(
        workers.create_root(owner).unwrap_err().kind,
        WorkerErrorKind::WorkerLimitExceeded
    );

    let mut mailbox = WorkerMessageMailbox::try_new(WorkerMessageLimits {
        max_message_bytes: 64,
        max_message_items: 8,
        max_message_depth: 4,
        max_queued_messages: 1,
        max_queued_bytes: 64,
    })
    .unwrap();
    mailbox
        .send_from_root(&workers, &owner, worker, &WorkerMessageValue::Null)
        .unwrap();
    assert_eq!(mailbox.queued_messages(), 1);
    assert_eq!(
        mailbox
            .send_from_root(&workers, &owner, worker, &WorkerMessageValue::Null)
            .unwrap_err(),
        WorkerMessageError::QueueMessageLimitExceeded
    );

    workers.begin_close(worker).unwrap();
    workers.retire(worker).unwrap();
    assert_eq!(
        workers.state(worker).unwrap_err().kind,
        WorkerErrorKind::UnknownWorker
    );
    assert!(matches!(
        mailbox
            .send_from_root(&workers, &owner, worker, &WorkerMessageValue::Null)
            .unwrap_err(),
        WorkerMessageError::Worker(error) if error.kind == WorkerErrorKind::UnknownWorker
    ));

    let mut service_workers = ServiceWorkerRegistry::try_new(ServiceWorkerLimits {
        max_registrations: 1,
        max_registrations_per_origin: 1,
        max_versions: 2,
        max_url_bytes: 256,
    })
    .unwrap();
    let page = web_url("https://app.example.com/index.html");
    let origin = page.origin().unwrap();
    let update = service_workers
        .register(
            &origin,
            &web_url("https://app.example.com/app/"),
            &web_url("https://app.example.com/sw.js"),
        )
        .unwrap();
    assert_eq!(service_workers.registration_count(), 1);
    assert_eq!(service_workers.version_count(), 1);
    assert!(service_workers.version(update.version()).is_ok());

    let cross_origin = service_workers
        .register(
            &origin,
            &web_url("https://app.example.com/other/"),
            &web_url("https://cdn.example.com/sw.js"),
        )
        .unwrap_err();
    assert_eq!(cross_origin, ServiceWorkerError::OriginMismatch);

    let foreign_registry = ServiceWorkerRegistry::with_default_limits().unwrap();
    assert_eq!(
        foreign_registry.version(update.version()).unwrap_err(),
        ServiceWorkerError::UnknownVersion
    );
}

#[test]
fn r5_exit_websocket_lifecycle_and_queue_accounting_are_bounded() {
    let message_limits = WebSocketLimits {
        max_url_bytes: 256,
        max_subprotocols: 4,
        max_subprotocol_bytes: 32,
        max_message_bytes: 4,
    };
    let mut queues = WebSocketMessageQueues::try_new(WebSocketQueueLimits {
        max_outbound_messages: 1,
        max_outbound_bytes: 4,
        max_inbound_messages: 1,
        max_inbound_bytes: 4,
    })
    .unwrap();
    queues
        .enqueue_outbound(WebSocketMessage::text("four", message_limits).unwrap())
        .unwrap();
    let snapshot = queues.snapshot();
    assert_eq!(snapshot.outbound_messages(), 1);
    assert_eq!(snapshot.outbound_bytes(), 4);
    assert_eq!(
        queues
            .enqueue_outbound(WebSocketMessage::text("x", message_limits).unwrap())
            .unwrap_err()
            .kind,
        WebSocketQueueErrorKind::OutboundMessageLimitExceeded
    );
    queues.complete_outbound().unwrap();
    assert_eq!(queues.snapshot().outbound_bytes(), 0);

    queues
        .enqueue_inbound(WebSocketMessage::binary(&[1, 2, 3, 4], message_limits).unwrap())
        .unwrap();
    assert_eq!(queues.inbound_receive_limit().unwrap(), None);
    assert_eq!(queues.take_inbound().unwrap().unwrap().len(), 4);
    assert_eq!(queues.snapshot().inbound_bytes(), 0);

    let mut lifecycle = WebSocketLifecycle::new();
    lifecycle.mark_open().unwrap();
    lifecycle.begin_closing().unwrap();
    lifecycle.mark_closed().unwrap();
    assert_eq!(lifecycle.state(), WebSocketReadyState::Closed);
    assert_eq!(
        lifecycle.mark_open().unwrap_err().kind,
        WebSocketLifecycleErrorKind::InvalidTransition
    );
}

#[test]
fn r5_exit_media_semantics_own_bounded_resources_not_backend_state() {
    let limits = MediaLimits {
        max_resources: 1,
        max_streams: 1,
        max_streams_per_resource: 1,
        max_playbacks: 1,
        max_audio_channels: 2,
        max_audio_sample_rate_hz: 48_000,
        max_video_width: 1_920,
        max_video_height: 1_080,
    };
    let descriptor = MediaResourceDescriptor::try_new(
        MediaTime::from_micros(1_000_000),
        &[MediaStreamDescriptor::Audio {
            channels: 2,
            sample_rate_hz: 48_000,
        }],
        limits,
    )
    .unwrap();
    let mut media = MediaRegistry::try_new(limits).unwrap();
    let resource = media.create_resource(descriptor.clone()).unwrap();
    assert!(matches!(
        media.create_resource(descriptor).unwrap_err(),
        MediaError::ResourceLimitExceeded { .. }
    ));

    let stream = media.resource(resource).unwrap().streams()[0];
    let playback = media.create_playback(resource, &[stream]).unwrap();
    assert_eq!(media.snapshot().resources(), 1);
    assert_eq!(media.snapshot().streams(), 1);
    assert_eq!(media.snapshot().playbacks(), 1);
    assert_eq!(
        media.retire_resource(resource).unwrap_err(),
        MediaError::ResourceInUse(resource)
    );
    assert_eq!(media.playback(playback).unwrap().resource(), resource);

    let foreign = MediaRegistry::try_new(limits).unwrap();
    assert_eq!(
        foreign.resource(resource).unwrap_err(),
        MediaError::UnknownResource(resource)
    );
}

#[test]
fn r5_exit_canvas_webgl_loss_retires_derived_resources_and_releases_canvas() {
    let mut canvas = CanvasRegistry::try_new(CanvasLimits {
        max_surfaces: 1,
        max_contexts: 1,
        max_pixels_per_surface: 4,
        max_total_pixels: 4,
        max_state_stack_depth: 1,
    })
    .unwrap();
    let surface = canvas.create_surface(2, 2).unwrap();
    assert!(canvas.create_surface(1, 1).is_err());

    let mut webgl = WebGlRegistry::try_new(WebGlLimits {
        max_contexts: 1,
        max_resources_per_context: 2,
        max_resources: 2,
        max_buffer_bytes: 4,
        max_total_buffer_bytes: 4,
        max_texture_dimension: 2,
        max_texture_pixels: 4,
        max_total_texture_pixels: 4,
    })
    .unwrap();
    let context = webgl.create_context(&mut canvas, surface).unwrap();
    let buffer = webgl.create_buffer(context, 4).unwrap();
    assert!(webgl.buffer(buffer).is_some());
    assert_eq!(webgl.total_buffer_bytes(), 4);
    assert!(matches!(
        webgl.create_buffer(context, 1).unwrap_err(),
        WebGlError::TotalBufferByteLimitExceeded { .. }
    ));

    assert!(
        webgl
            .lose_context(context, WebGlContextLossReason::DeviceReset)
            .unwrap()
    );
    assert_eq!(
        webgl.context(context).unwrap().state(),
        WebGlContextState::Lost(WebGlContextLossReason::DeviceReset)
    );
    assert_eq!(webgl.resource_count(), 0);
    assert_eq!(webgl.total_buffer_bytes(), 0);
    assert_eq!(
        webgl.create_buffer(context, 1).unwrap_err(),
        WebGlError::ContextLost(context)
    );

    webgl.destroy_context(&mut canvas, context).unwrap();
    assert!(canvas.surface(surface).unwrap().external_context().is_none());
    assert!(canvas.create_2d_context(surface).is_ok());
}

#[test]
fn r5_exit_accessibility_is_bounded_derived_engine_state() {
    let invalid = AccessibilityLimits {
        max_nodes: 2,
        max_identities: 1,
        max_dom_nodes_scanned: 2,
        max_fragments: 2,
        max_name_bytes_per_node: 8,
        max_total_name_bytes: 8,
    };
    assert_eq!(
        AccessibilityTreeState::try_new(invalid).unwrap_err(),
        AccessibilityError::InvalidLimits
    );

    let mut session =
        RenderSession::new("<button>Save</button>", RenderOptions::default()).unwrap();
    let before = session.accessibility_snapshot().unwrap();
    let document_generation = before.document_generation();
    let geometry_revision = before.geometry_revision();
    let root = before.tree().root();

    session
        .resize(Size {
            width: 640.0,
            height: 480.0,
        })
        .unwrap();
    let after = session.accessibility_snapshot().unwrap();
    assert_eq!(after.document_generation(), document_generation);
    assert!(after.geometry_revision() > geometry_revision);
    assert_eq!(after.tree().root(), root);
    assert!(session.accessibility_is_current());
}
