use rarog_broker::{CapabilityClass, CapabilityErrorKind};
use rarog_fetch::{
    FetchError, FetchRequest, NetworkCapability, NetworkPoll, NetworkRequest, NetworkTicket,
};
use rarog_host::{HostControlErrorKind, HostControlPlane, HostLimits, NavigationTransitionKind};
use rarog_ipc::{
    EndpointRole, IPC_PROTOCOL_VERSION, IPC_WIRE_HEADER_BYTES, IpcChannel, IpcEnvelope,
    IpcErrorKind, IpcLimits, MessageKind, decode_wire_frame, decode_wire_header, encode_wire_frame,
};
use rarog_platform::{
    ClipboardError, ClipboardLimits, ClipboardText, PlatformClipboardService, PlatformHost,
    PlatformService,
};
use rarog_platform_windows::{WindowsPlatformError, WindowsPlatformHost};
use rarog_process::ProcessTopologyErrorKind;
use rarog_url::{SiteIdentity, WebUrl};
use std::num::NonZeroU64;
use std::sync::Mutex;

const R4_BACKLOG: &str = include_str!("../../../docs/R4-BACKLOG.md");
const R4_EXIT: &str = include_str!("../../../docs/R4-EXIT.md");
const CI: &str = include_str!("../../../.github/workflows/ci.yml");
const ROOT_CARGO: &str = include_str!("../../../Cargo.toml");
const WINDOWS_NATIVE_CARGO: &str = include_str!("../../rarog-platform-windows-native/Cargo.toml");

fn limits(max_site_processes: usize, max_navigation_contexts: usize) -> HostLimits {
    HostLimits {
        max_site_processes,
        ipc: IpcLimits {
            max_message_bytes: 64,
            max_queued_messages: 4,
            max_queued_bytes: 128,
        },
        max_capabilities: 32,
        max_network_operations: 16,
        max_navigation_contexts,
        max_navigation_context_url_bytes: 512,
        max_service_worker_fetch_dispatches: 4,
        max_websocket_connections: 4,
    }
}

fn site(url: &str) -> SiteIdentity {
    WebUrl::parse(url).unwrap().site_identity().unwrap()
}

fn network_request(url: &str) -> NetworkRequest {
    let url = WebUrl::parse(url).unwrap();
    let origin = WebUrl::parse("https://app.example.com/")
        .unwrap()
        .origin()
        .unwrap();
    FetchRequest::new(url, origin).network_request()
}

#[derive(Default)]
struct FixtureNetwork {
    starts: usize,
    polls: usize,
    cancels: usize,
    last_ticket: Option<NetworkTicket>,
}

impl NetworkCapability for FixtureNetwork {
    fn start(&mut self, _request: NetworkRequest) -> Result<NetworkTicket, FetchError> {
        self.starts += 1;
        let ticket = NetworkTicket::new(NonZeroU64::new(700 + self.starts as u64).unwrap());
        self.last_ticket = Some(ticket);
        Ok(ticket)
    }

    fn poll(&mut self, _ticket: NetworkTicket) -> Result<NetworkPoll, FetchError> {
        self.polls += 1;
        Ok(NetworkPoll::Pending)
    }

    fn cancel(&mut self, _ticket: NetworkTicket) -> Result<(), FetchError> {
        self.cancels += 1;
        Ok(())
    }
}

#[derive(Default)]
struct ClipboardState {
    reads: usize,
    writes: usize,
    text: Option<ClipboardText>,
}

struct FixtureClipboard {
    limits: ClipboardLimits,
    state: Mutex<ClipboardState>,
}

impl FixtureClipboard {
    fn new() -> Self {
        Self {
            limits: ClipboardLimits { max_text_bytes: 64 },
            state: Mutex::new(ClipboardState::default()),
        }
    }

    fn reads(&self) -> usize {
        self.state.lock().unwrap().reads
    }

    fn writes(&self) -> usize {
        self.state.lock().unwrap().writes
    }
}

impl PlatformClipboardService for FixtureClipboard {
    fn limits(&self) -> ClipboardLimits {
        self.limits
    }

    fn read_text(&self) -> Result<Option<ClipboardText>, ClipboardError> {
        let mut state = self.state.lock().unwrap();
        state.reads += 1;
        Ok(state.text.clone())
    }

    fn write_text(&self, text: &ClipboardText) -> Result<(), ClipboardError> {
        let mut state = self.state.lock().unwrap();
        state.writes += 1;
        state.text = Some(text.clone());
        Ok(())
    }
}

#[test]
fn r4_exit_manifest_ci_and_native_boundary_are_complete() {
    assert!(R4_BACKLOG.contains("Status: **complete**."));
    assert!(
        !R4_BACKLOG
            .lines()
            .any(|line| line.trim_start().starts_with("- [ ]")),
        "R4 backlog contains an unchecked milestone item"
    );
    assert!(R4_EXIT.contains("Status: **complete**."));
    assert!(CI.contains("R4 Windows sandbox gate"));
    assert!(CI.contains("R4 Windows IPC transport gate"));
    assert_eq!(CI.matches("name: R4 exit gate").count(), 2);
    assert!(ROOT_CARGO.contains("unsafe_code = \"forbid\""));
    assert!(WINDOWS_NATIVE_CARGO.contains("unsafe_code = \"allow\""));
    assert!(WINDOWS_NATIVE_CARGO.contains("unsafe_op_in_unsafe_fn = \"deny\""));
}

#[test]
fn r4_exit_schemeful_site_assignment_is_bounded_and_fail_closed() {
    let mut host = HostControlPlane::try_new(limits(3, 8)).unwrap();
    let first_site = site("https://a.example.com/");
    let same_site = site("https://b.example.com/path");
    let cross_scheme = site("http://example.com/");
    let other_site = site("https://example.org/");

    let first = host.ensure_site(first_site.clone()).unwrap();
    let same = host.ensure_site(same_site).unwrap();
    let scheme = host.ensure_site(cross_scheme).unwrap();
    let other = host.ensure_site(other_site).unwrap();

    assert_eq!(first.process(), same.process());
    assert_ne!(first.process(), scheme.process());
    assert_ne!(first.process(), other.process());
    assert_ne!(scheme.process(), other.process());
    assert_eq!(host.active_site_processes(), 3);

    let mut bounded = HostControlPlane::try_new(limits(1, 4)).unwrap();
    let retained_site = site("https://example.com/");
    let retained = bounded.ensure_site(retained_site.clone()).unwrap();
    let error = bounded
        .ensure_site(site("https://example.org/"))
        .unwrap_err();
    assert_eq!(
        error.kind,
        HostControlErrorKind::Process(ProcessTopologyErrorKind::ProcessLimitExceeded)
    );
    assert_eq!(bounded.active_site_processes(), 1);
    assert_eq!(
        bounded.process_for_site(&retained_site),
        Some(retained.process())
    );
}

#[test]
fn r4_exit_opaque_identity_is_environment_owned() {
    let url = WebUrl::parse("data:text/html,rarog").unwrap();
    let inherited = url.site_identity().unwrap();
    let independently_created = url.site_identity().unwrap();
    assert_ne!(inherited, independently_created);

    let mut host = HostControlPlane::try_new(limits(2, 8)).unwrap();
    let first = host
        .open_navigation_context_to_site(inherited.clone())
        .unwrap();
    let second = host
        .open_navigation_context_to_site(inherited.clone())
        .unwrap();
    let inherited_process = host.process_for_site(&inherited).unwrap();

    assert_ne!(first.context(), second.context());
    assert_eq!(host.active_site_processes(), 1);
    assert_eq!(host.process_for_site(&inherited), Some(inherited_process));

    host.open_navigation_context_to_site(independently_created.clone())
        .unwrap();
    assert_ne!(
        host.process_for_site(&independently_created).unwrap(),
        inherited_process
    );
}

#[test]
fn r4_exit_navigation_context_ids_and_last_reference_lifetime_are_monotonic() {
    let mut host = HostControlPlane::try_new(limits(1, 2)).unwrap();
    let target = WebUrl::parse("https://example.com/").unwrap();
    let target_site = target.site_identity().unwrap();
    let first = host.open_navigation_context(&target).unwrap();
    let second = host.open_navigation_context(&target).unwrap();
    let first_process = host.process_for_site(&target_site).unwrap();

    assert!(second.context().get() > first.context().get());
    assert_eq!(
        host.open_navigation_context(&target).unwrap_err().kind,
        HostControlErrorKind::NavigationContextLimitExceeded
    );

    let first_close = host.close_navigation_context(first.context()).unwrap();
    assert!(!first_close.source_retired());
    assert_eq!(host.active_site_processes(), 1);

    let second_close = host.close_navigation_context(second.context()).unwrap();
    assert!(second_close.source_retired());
    assert_eq!(host.active_site_processes(), 0);

    let replacement = host.open_navigation_context(&target).unwrap();
    let replacement_process = host.process_for_site(&target_site).unwrap();
    assert!(replacement.context().get() > second.context().get());
    assert_ne!(replacement_process, first_process);
}

#[test]
fn r4_exit_one_slot_cross_site_replacement_revokes_stale_authority() {
    let mut host = HostControlPlane::try_new(limits(1, 4)).unwrap();
    let source_url = WebUrl::parse("https://example.com/").unwrap();
    let source_site = source_url.site_identity().unwrap();
    let context = host.open_navigation_context(&source_url).unwrap();
    let old_process = host.process_for_site(&source_site).unwrap();
    let stale = host
        .grant_navigation_context_network_capability(context.context())
        .unwrap();

    let target_url = WebUrl::parse("https://example.org/").unwrap();
    let target_site = target_url.site_identity().unwrap();
    let transition = host
        .navigate_navigation_context(context.context(), &target_url)
        .unwrap();
    let replacement_process = host.process_for_site(&target_site).unwrap();

    assert_eq!(
        transition.kind(),
        NavigationTransitionKind::CrossSiteReplacement
    );
    assert!(transition.source_retired());
    assert_ne!(old_process, replacement_process);
    assert!(host.process_for_site(&source_site).is_none());

    let mut network = FixtureNetwork::default();
    assert_eq!(
        host.start_navigation_context_network_operation(
            stale,
            network_request("https://example.org/data"),
            &mut network,
        )
        .unwrap_err()
        .kind,
        HostControlErrorKind::InvalidNavigationContextCapabilityAuthority
    );
    assert_eq!(network.starts, 0);

    let mut shared = HostControlPlane::try_new(limits(1, 4)).unwrap();
    let first = shared
        .open_navigation_context(&WebUrl::parse("https://a.example.com/").unwrap())
        .unwrap();
    let second = shared
        .open_navigation_context(&WebUrl::parse("https://b.example.com/").unwrap())
        .unwrap();
    assert_eq!(
        shared
            .navigate_navigation_context(
                first.context(),
                &WebUrl::parse("https://example.org/").unwrap(),
            )
            .unwrap_err()
            .kind,
        HostControlErrorKind::Process(ProcessTopologyErrorKind::ProcessLimitExceeded)
    );
    assert!(shared.navigation_context(first.context()).is_ok());
    assert!(shared.navigation_context(second.context()).is_ok());
    assert_eq!(shared.active_site_processes(), 1);
}

#[test]
fn r4_exit_ipc_and_wire_protocol_are_bounded() {
    let limits = IpcLimits {
        max_message_bytes: 4,
        max_queued_messages: 1,
        max_queued_bytes: 4,
    };

    assert_eq!(
        IpcEnvelope::try_new(
            IPC_PROTOCOL_VERSION + 1,
            EndpointRole::Host,
            EndpointRole::Site,
            MessageKind::Event,
            Vec::new(),
            limits,
        )
        .unwrap_err()
        .kind,
        IpcErrorKind::UnsupportedVersion
    );
    assert_eq!(
        IpcEnvelope::try_new(
            IPC_PROTOCOL_VERSION,
            EndpointRole::Host,
            EndpointRole::Host,
            MessageKind::Event,
            Vec::new(),
            limits,
        )
        .unwrap_err()
        .kind,
        IpcErrorKind::InvalidRoute
    );
    assert_eq!(
        IpcEnvelope::event(EndpointRole::Host, vec![0; 5], limits)
            .unwrap_err()
            .kind,
        IpcErrorKind::MessageTooLarge
    );

    let mut channel = IpcChannel::try_new(limits).unwrap();
    channel
        .send(IpcEnvelope::event(EndpointRole::Host, vec![1, 2, 3, 4], limits).unwrap())
        .unwrap();
    assert_eq!(
        channel
            .send(IpcEnvelope::event(EndpointRole::Host, Vec::new(), limits).unwrap())
            .unwrap_err()
            .kind,
        IpcErrorKind::QueueMessageLimitExceeded
    );

    let envelope = IpcEnvelope::event(EndpointRole::Site, b"abc".to_vec(), limits).unwrap();
    let frame = encode_wire_frame(&envelope, limits).unwrap();
    assert_eq!(decode_wire_frame(&frame, limits).unwrap(), envelope);

    assert_eq!(
        decode_wire_frame(&frame[..frame.len() - 1], limits)
            .unwrap_err()
            .kind,
        IpcErrorKind::TruncatedWireFrame
    );
    let mut trailing = frame.clone();
    trailing.push(0);
    assert_eq!(
        decode_wire_frame(&trailing, limits).unwrap_err().kind,
        IpcErrorKind::TrailingWireBytes
    );

    let mut oversized_header = frame[..IPC_WIRE_HEADER_BYTES].to_vec();
    oversized_header[20..24].copy_from_slice(&5u32.to_le_bytes());
    assert_eq!(
        decode_wire_header(&oversized_header, limits)
            .unwrap_err()
            .kind,
        IpcErrorKind::MessageTooLarge
    );
}

#[test]
fn r4_exit_context_privileges_fail_before_backend_side_effects() {
    let mut host = HostControlPlane::try_new(limits(1, 8)).unwrap();
    let first = host
        .open_navigation_context(&WebUrl::parse("https://a.example.com/").unwrap())
        .unwrap();
    let second = host
        .open_navigation_context(&WebUrl::parse("https://b.example.com/").unwrap())
        .unwrap();
    let first_network = host
        .grant_navigation_context_network_capability(first.context())
        .unwrap();
    let second_network = host
        .grant_navigation_context_network_capability(second.context())
        .unwrap();

    let mut network = FixtureNetwork::default();
    let operation = host
        .start_navigation_context_network_operation(
            first_network,
            network_request("https://api.example.com/data"),
            &mut network,
        )
        .unwrap();

    assert_eq!(network.starts, 1);
    assert_ne!(operation.get(), network.last_ticket.unwrap().get());
    assert_eq!(
        host.poll_navigation_context_network_operation(second_network, operation, &mut network,)
            .unwrap_err()
            .kind,
        HostControlErrorKind::InvalidNetworkOperationAuthority
    );
    assert_eq!(network.polls, 0);

    let clipboard = FixtureClipboard::new();
    assert_eq!(
        host.read_navigation_context_clipboard_text(first_network, &clipboard)
            .unwrap_err()
            .kind,
        HostControlErrorKind::InvalidNavigationContextCapabilityAuthority
    );
    assert_eq!(clipboard.reads(), 0);

    let clipboard_capability = host
        .grant_navigation_context_capability(first.context(), CapabilityClass::Clipboard)
        .unwrap();
    let text = ClipboardText::try_new("Rarog", clipboard.limits()).unwrap();
    host.write_navigation_context_clipboard_text(clipboard_capability, &text, &clipboard)
        .unwrap();
    assert_eq!(clipboard.writes(), 1);

    host.close_navigation_context(first.context()).unwrap();
    assert_eq!(
        host.read_navigation_context_clipboard_text(clipboard_capability, &clipboard)
            .unwrap_err()
            .kind,
        HostControlErrorKind::InvalidNavigationContextCapabilityAuthority
    );
    assert_eq!(clipboard.reads(), 0);
}

#[test]
fn r4_exit_process_loss_revokes_authority_before_fresh_recovery() {
    let host_limits = limits(1, 4);
    let mut host = HostControlPlane::try_new(host_limits).unwrap();
    let site_identity = site("https://example.com/");
    let context = host
        .open_navigation_context_to_site(site_identity.clone())
        .unwrap();
    let process = host.process_for_site(&site_identity).unwrap();
    let network_capability = host
        .grant_navigation_context_network_capability(context.context())
        .unwrap();
    let clipboard_capability = host
        .grant_capability(process, CapabilityClass::Clipboard)
        .unwrap();

    host.enqueue_from_host(
        process,
        IpcEnvelope::event(EndpointRole::Host, b"stale".to_vec(), host_limits.ipc).unwrap(),
    )
    .unwrap();

    let mut network = FixtureNetwork::default();
    let operation = host
        .start_navigation_context_network_operation(
            network_capability,
            network_request("https://api.example.com/data"),
            &mut network,
        )
        .unwrap();

    let loss = host.process_lost(process).unwrap();
    assert_eq!(loss.process(), process);
    assert_eq!(loss.revoked_capabilities(), 2);
    assert_eq!(loss.revoked_network_operations(), 1);
    assert_eq!(loss.invalidated_navigation_contexts(), 1);
    assert_eq!(host.active_site_processes(), 0);
    assert_eq!(host.active_capabilities(), 0);
    assert_eq!(host.active_network_operations(), 0);
    assert_eq!(host.pending_network_cancellations(), 1);
    assert_eq!(host.tracked_network_operations(), 1);
    assert_eq!(host.active_navigation_contexts(), 0);
    assert_eq!(
        host.navigation_context(context.context()).unwrap_err().kind,
        HostControlErrorKind::UnknownNavigationContext
    );
    assert_eq!(
        host.authorize_capability(
            process,
            clipboard_capability.id(),
            CapabilityClass::Clipboard,
        )
        .unwrap_err()
        .kind,
        HostControlErrorKind::UnknownSiteProcess
    );
    assert_eq!(
        host.poll_navigation_context_network_operation(
            network_capability,
            operation,
            &mut network,
        )
        .unwrap_err()
        .kind,
        HostControlErrorKind::InvalidNavigationContextCapabilityAuthority
    );
    assert_eq!(network.polls, 0);
    assert_eq!(
        host.cancel_pending_network_operations(&mut network)
            .unwrap(),
        1
    );
    assert_eq!(network.cancels, 1);
    assert_eq!(host.tracked_network_operations(), 0);

    let replacement = host.recover_site(site_identity).unwrap().process();
    assert_ne!(replacement, process);
    assert_eq!(host.queued_for_site(replacement).unwrap(), 0);
    assert_eq!(
        host.authorize_capability(
            replacement,
            clipboard_capability.id(),
            CapabilityClass::Clipboard,
        )
        .unwrap_err()
        .kind,
        HostControlErrorKind::Capability(CapabilityErrorKind::UnknownCapability)
    );
}

#[test]
fn r4_exit_windows_platform_exposes_sandbox_only_on_windows() {
    if WindowsPlatformHost::target_available() {
        let host = WindowsPlatformHost::try_new().unwrap();
        assert!(
            host.capabilities()
                .supports(PlatformService::SandboxProcess)
        );
    } else {
        assert_eq!(
            WindowsPlatformHost::try_new().unwrap_err(),
            WindowsPlatformError::UnsupportedTarget
        );
    }
}
