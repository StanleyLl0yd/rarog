use rarog_accessibility::{AccessibilityError, AccessibilityLimits, AccessibilityTreeState};
use rarog_engine::{RenderOptions, RenderSession};
use rarog_process::ProcessTopology;
use rarog_types::Size;
use rarog_url::WebUrl;

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

const STORAGE_STATE: &str = include_str!("../../rarog-storage/src/state.rs");
const WORKER_IDENTITY: &str = include_str!("../../rarog-workers/src/identity.rs");
const WORKER_MESSAGE: &str = include_str!("../../rarog-workers/src/message.rs");
const SERVICE_WORKER: &str = include_str!("../../rarog-workers/src/service_worker.rs");
const WEBSOCKET: &str = include_str!("../../rarog-websocket/src/lib.rs");
const WEBSOCKET_QUEUE: &str = include_str!("../../rarog-websocket/src/queue.rs");
const HOST: &str = include_str!("../../rarog-host/src/lib.rs");
const MEDIA: &str = include_str!("../../rarog-media/src/lib.rs");
const MEDIA_ADAPTER: &str = include_str!("../../rarog-media-adapter/src/lib.rs");
const CANVAS: &str = include_str!("../../rarog-canvas/src/lib.rs");
const WEBGL: &str = include_str!("../../rarog-webgl/src/lib.rs");
const ACCESSIBILITY: &str = include_str!("../../rarog-accessibility/src/lib.rs");

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
    assert!(R5_BACKLOG.contains("R5 completion does not begin or claim R6 compatibility qualification"));
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
            "portable R5 semantic crate {name} depends on Windows-native authority"
        );
    }
}

#[test]
fn r5_exit_storage_identity_origin_isolation_and_limits_are_explicit() {
    let mut topology = ProcessTopology::try_new(1).unwrap();
    let app = web_url("https://app.example.com/");
    let cdn = web_url("https://cdn.example.com/");
    let site = topology.assign_site(app.site_identity().unwrap()).unwrap();
    let first_storage = topology.ensure_storage_process().unwrap();
    assert_ne!(site.process().get(), first_storage.process().get());

    topology
        .retire_storage_process(first_storage.process())
        .unwrap();
    let replacement = topology.ensure_storage_process().unwrap();
    assert_ne!(first_storage.process(), replacement.process());

    assert_eq!(app.site_identity().unwrap(), cdn.site_identity().unwrap());
    assert_ne!(app.origin().unwrap(), cdn.origin().unwrap());
    assert!(STORAGE_STATE.contains("pub struct StorageLimits"));
    assert!(STORAGE_STATE.contains("origins: HashMap<Origin, OriginStorage>"));
    assert!(STORAGE_STATE.contains("StorageErrorKind::OriginLimitExceeded"));
    assert!(STORAGE_STATE.contains("StorageErrorKind::EntryLimitExceeded"));
    assert!(STORAGE_STATE.contains("StorageErrorKind::OriginByteLimitExceeded"));
    assert!(STORAGE_STATE.contains("StorageErrorKind::TotalByteLimitExceeded"));
}

#[test]
fn r5_exit_worker_and_service_worker_contracts_are_bounded_and_fail_closed() {
    assert!(WORKER_IDENTITY.contains("pub struct WorkerLimits"));
    assert!(WORKER_IDENTITY.contains("WorkerErrorKind::WorkerLimitExceeded"));
    assert!(WORKER_IDENTITY.contains("WorkerErrorKind::UnknownWorker"));
    assert!(WORKER_IDENTITY.contains("WorkerLifecycleState::Closing"));

    assert!(WORKER_MESSAGE.contains("pub struct WorkerMessageLimits"));
    assert!(WORKER_MESSAGE.contains("QueueMessageLimitExceeded"));
    assert!(WORKER_MESSAGE.contains("QueueByteLimitExceeded"));
    assert!(WORKER_MESSAGE.contains("DeliveryUnavailable"));

    assert!(SERVICE_WORKER.contains("pub struct ServiceWorkerLimits"));
    assert!(SERVICE_WORKER.contains("OriginMismatch"));
    assert!(SERVICE_WORKER.contains("RegistrationLimitExceeded"));
    assert!(SERVICE_WORKER.contains("VersionLimitExceeded"));
    assert!(SERVICE_WORKER.contains("UnknownVersion"));
}

#[test]
fn r5_exit_websocket_authority_lifecycle_and_queues_are_bounded() {
    assert!(WEBSOCKET.contains("pub struct WebSocketLimits"));
    assert!(WEBSOCKET.contains("pub trait WebSocketTransport"));
    assert!(WEBSOCKET.contains("WebSocketTransportTicket(NonZeroU64)"));
    assert!(WEBSOCKET.contains("pub enum WebSocketReadyState"));

    assert!(WEBSOCKET_QUEUE.contains("pub struct WebSocketQueueLimits"));
    assert!(WEBSOCKET_QUEUE.contains("OutboundMessageLimitExceeded"));
    assert!(WEBSOCKET_QUEUE.contains("OutboundByteLimitExceeded"));
    assert!(WEBSOCKET_QUEUE.contains("InboundMessageLimitExceeded"));
    assert!(WEBSOCKET_QUEUE.contains("InboundByteLimitExceeded"));

    assert!(HOST.contains(
        "authorize_navigation_context_capability_class(capability, CapabilityClass::Network)"
    ));
    assert!(HOST.contains("quarantine_websocket_connections_for_process(process)"));
    assert!(HOST.contains("pending_websocket_aborts"));
}

#[test]
fn r5_exit_media_contract_keeps_backend_state_out_of_semantic_authority() {
    assert!(MEDIA.contains("pub struct MediaLimits"));
    assert!(MEDIA.contains("pub struct MediaRegistry"));
    assert!(MEDIA.contains("ResourceLimitExceeded"));
    assert!(MEDIA.contains("PlaybackLimitExceeded"));
    assert!(MEDIA.contains("UnknownResource(MediaResourceId)"));

    assert!(MEDIA_ADAPTER.contains("pub trait MediaDemuxer"));
    assert!(MEDIA_ADAPTER.contains("pub trait MediaDecoder"));
    assert!(MEDIA_ADAPTER.contains("pub trait MediaOutput"));
    assert!(MEDIA_ADAPTER.contains("pub struct MediaDemuxTicket(NonZeroU64)"));
    assert!(MEDIA_ADAPTER.contains("pub struct MediaDecoderTicket(NonZeroU64)"));
    assert!(MEDIA_ADAPTER.contains("pub struct MediaOutputTicket(NonZeroU64)"));
}

#[test]
fn r5_exit_canvas_webgl_contract_bounds_resources_and_context_loss() {
    assert!(CANVAS.contains("pub struct CanvasLimits"));
    assert!(CANVAS.contains("SurfacePixelLimitExceeded"));
    assert!(CANVAS.contains("TotalPixelLimitExceeded"));
    assert!(CANVAS.contains("CanvasExternalContextLease"));

    assert!(WEBGL.contains("pub struct WebGlLimits"));
    assert!(WEBGL.contains("ContextLost(WebGlContextId)"));
    assert!(WEBGL.contains("pub fn lose_context"));
    assert!(WEBGL.contains("self.retire_resources(context)?"));
    assert!(WEBGL.contains("canvas.acquire_external_context(surface)?"));
    assert!(WEBGL.contains("canvas.release_external_context(lease)?"));
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
    assert!(ACCESSIBILITY.contains("pub struct AccessibilityLimits"));
    assert!(ACCESSIBILITY.contains("source_generation: u64"));
    assert!(ACCESSIBILITY.contains("IdentityLimitExceeded"));
    assert!(ACCESSIBILITY.contains("TotalNameByteLimitExceeded"));

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
