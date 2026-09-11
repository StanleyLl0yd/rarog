use rarog_broker::CapabilityClass;
use rarog_host::{
    HostControlErrorKind, HostControlPlane, HostLimits, NavigationContextCapability,
    WebSocketConnectionId,
};
use rarog_url::{Origin, WebUrl};
use rarog_websocket::{
    WebSocketHandshakeIntent, WebSocketLimits, WebSocketMessage, WebSocketTransport,
    WebSocketTransportError, WebSocketTransportErrorKind, WebSocketTransportReceive,
    WebSocketTransportSend, WebSocketTransportTicket,
};
use std::num::NonZeroU64;

fn host_with_websocket_limit(max_websocket_connections: usize) -> HostControlPlane {
    let limits = HostLimits {
        max_site_processes: 4,
        max_capabilities: 16,
        max_navigation_contexts: 8,
        max_websocket_connections,
        ..HostLimits::default()
    };
    HostControlPlane::try_new(limits).unwrap()
}

fn open_context(host: &mut HostControlPlane, url: &str) -> rarog_host::NavigationContextId {
    host.open_navigation_context(&WebUrl::parse(url).unwrap())
        .unwrap()
        .context()
}

fn network_capability(
    host: &mut HostControlPlane,
    context: rarog_host::NavigationContextId,
) -> NavigationContextCapability {
    host.grant_navigation_context_network_capability(context)
        .unwrap()
}

fn handshake() -> WebSocketHandshakeIntent {
    WebSocketHandshakeIntent::try_new(
        "wss://socket.other.example/realtime",
        &["chat.v1"],
        WebSocketLimits::default(),
    )
    .unwrap()
}

#[derive(Debug)]
struct FixtureTransport {
    next_ticket: u64,
    fixed_ticket: Option<u64>,
    starts: usize,
    aborts: Vec<u64>,
    origins: Vec<Origin>,
    fail_abort: bool,
}

impl Default for FixtureTransport {
    fn default() -> Self {
        Self {
            next_ticket: 1,
            fixed_ticket: None,
            starts: 0,
            aborts: Vec::new(),
            origins: Vec::new(),
            fail_abort: false,
        }
    }
}

impl FixtureTransport {
    fn with_fixed_ticket(ticket: u64) -> Self {
        Self {
            fixed_ticket: Some(ticket),
            ..Self::default()
        }
    }
}

impl WebSocketTransport for FixtureTransport {
    fn start(
        &mut self,
        _handshake: WebSocketHandshakeIntent,
        client_origin: Origin,
    ) -> Result<WebSocketTransportTicket, WebSocketTransportError> {
        self.starts += 1;
        self.origins.push(client_origin);
        let raw = match self.fixed_ticket {
            Some(ticket) => ticket,
            None => {
                let ticket = self.next_ticket;
                self.next_ticket += 1;
                ticket
            }
        };
        Ok(WebSocketTransportTicket::new(NonZeroU64::new(raw).unwrap()))
    }

    fn send(
        &mut self,
        _ticket: WebSocketTransportTicket,
        _message: &WebSocketMessage,
    ) -> Result<WebSocketTransportSend, WebSocketTransportError> {
        Ok(WebSocketTransportSend::Accepted)
    }

    fn receive(
        &mut self,
        _ticket: WebSocketTransportTicket,
        _max_message_bytes: usize,
    ) -> Result<WebSocketTransportReceive, WebSocketTransportError> {
        Ok(WebSocketTransportReceive::Pending)
    }

    fn abort(&mut self, ticket: WebSocketTransportTicket) -> Result<(), WebSocketTransportError> {
        if self.fail_abort {
            return Err(WebSocketTransportError::new(
                WebSocketTransportErrorKind::Backend,
                "fixture abort failed",
            ));
        }
        self.aborts.push(ticket.get());
        Ok(())
    }
}

fn start(
    host: &mut HostControlPlane,
    capability: NavigationContextCapability,
    transport: &mut FixtureTransport,
) -> WebSocketConnectionId {
    host.start_navigation_context_websocket(capability, handshake(), transport)
        .unwrap()
}

#[test]
fn websocket_open_requires_exact_network_authority_before_backend_start() {
    let mut host = host_with_websocket_limit(4);
    let context = open_context(&mut host, "https://app.example/page");
    let wrong = host
        .grant_navigation_context_capability(context, CapabilityClass::Clipboard)
        .unwrap();
    let mut transport = FixtureTransport::default();

    let error = host
        .start_navigation_context_websocket(wrong, handshake(), &mut transport)
        .unwrap_err();
    assert_eq!(
        error.kind,
        HostControlErrorKind::InvalidNavigationContextCapabilityAuthority
    );
    assert_eq!(transport.starts, 0);

    let network = network_capability(&mut host, context);
    let connection = start(&mut host, network, &mut transport);
    assert_eq!(host.active_websocket_connections(), 1);
    assert_eq!(connection.serial(), 1);
    assert_eq!(transport.starts, 1);
    assert_eq!(
        transport.origins,
        vec![
            WebUrl::parse("https://app.example/")
                .unwrap()
                .origin()
                .unwrap()
        ]
    );
}

#[test]
fn site_only_context_without_host_origin_cannot_touch_backend() {
    let mut host = host_with_websocket_limit(4);
    let site = WebUrl::parse("https://app.example/")
        .unwrap()
        .site_identity()
        .unwrap();
    let context = host
        .open_navigation_context_to_site(site)
        .unwrap()
        .context();
    let network = network_capability(&mut host, context);
    let mut transport = FixtureTransport::default();

    let error = host
        .start_navigation_context_websocket(network, handshake(), &mut transport)
        .unwrap_err();
    assert_eq!(
        error.kind,
        HostControlErrorKind::WebSocketClientOriginUnavailable
    );
    assert_eq!(transport.starts, 0);
    assert_eq!(host.tracked_websocket_connections(), 0);
}

#[test]
fn connection_ids_are_scoped_and_wrong_context_authority_is_rejected() {
    let mut first_host = host_with_websocket_limit(4);
    let first_context = open_context(&mut first_host, "https://first.example/");
    let first_capability = network_capability(&mut first_host, first_context);
    let mut first_transport = FixtureTransport::default();
    let first_connection = start(&mut first_host, first_capability, &mut first_transport);

    let mut second_host = host_with_websocket_limit(4);
    let second_context = open_context(&mut second_host, "https://second.example/");
    let second_capability = network_capability(&mut second_host, second_context);
    let mut second_transport = FixtureTransport::default();
    let second_connection = start(&mut second_host, second_capability, &mut second_transport);

    assert_ne!(first_connection.scope(), second_connection.scope());
    assert_eq!(first_connection.serial(), second_connection.serial());
    let foreign = second_host
        .revoke_navigation_context_websocket(second_capability, first_connection)
        .unwrap_err();
    assert_eq!(
        foreign.kind,
        HostControlErrorKind::InvalidWebSocketConnectionAuthority
    );
    assert_eq!(second_host.active_websocket_connections(), 1);

    let third_context = open_context(&mut second_host, "https://third.example/");
    let third_capability = network_capability(&mut second_host, third_context);
    let wrong_context = second_host
        .revoke_navigation_context_websocket(third_capability, second_connection)
        .unwrap_err();
    assert_eq!(
        wrong_context.kind,
        HostControlErrorKind::InvalidWebSocketConnectionAuthority
    );
    assert_eq!(second_host.active_websocket_connections(), 1);
}

#[test]
fn quarantine_keeps_capacity_charged_until_backend_abort_succeeds() {
    let mut host = host_with_websocket_limit(1);
    let context = open_context(&mut host, "https://app.example/");
    let capability = network_capability(&mut host, context);
    let mut transport = FixtureTransport::default();
    let connection = start(&mut host, capability, &mut transport);

    host.revoke_navigation_context_websocket(capability, connection)
        .unwrap();
    assert_eq!(host.active_websocket_connections(), 0);
    assert_eq!(host.pending_websocket_aborts(), 1);
    assert_eq!(host.tracked_websocket_connections(), 1);

    let limit = host
        .start_navigation_context_websocket(capability, handshake(), &mut transport)
        .unwrap_err();
    assert_eq!(
        limit.kind,
        HostControlErrorKind::WebSocketConnectionLimitExceeded
    );
    assert_eq!(transport.starts, 1);

    transport.fail_abort = true;
    let failed_abort = host
        .abort_pending_websocket_connections(&mut transport)
        .unwrap_err();
    assert_eq!(
        failed_abort.kind,
        HostControlErrorKind::WebSocket(WebSocketTransportErrorKind::Backend)
    );
    assert_eq!(host.pending_websocket_aborts(), 1);
    assert_eq!(host.tracked_websocket_connections(), 1);

    transport.fail_abort = false;
    assert_eq!(
        host.abort_pending_websocket_connections(&mut transport)
            .unwrap(),
        1
    );
    assert_eq!(transport.aborts, vec![1]);
    assert_eq!(host.tracked_websocket_connections(), 0);
    let replacement = start(&mut host, capability, &mut transport);
    assert_eq!(replacement.serial(), 2);
    assert_eq!(transport.starts, 2);
}

#[test]
fn same_site_navigation_revokes_old_document_connection_even_when_capability_survives() {
    let mut host = host_with_websocket_limit(4);
    let context = open_context(&mut host, "https://app.example/first");
    let capability = network_capability(&mut host, context);
    let mut transport = FixtureTransport::default();
    let old = start(&mut host, capability, &mut transport);

    host.navigate_navigation_context(
        context,
        &WebUrl::parse("https://app.example/second").unwrap(),
    )
    .unwrap();

    assert_eq!(host.active_websocket_connections(), 0);
    assert_eq!(host.pending_websocket_aborts(), 1);
    let stale = host
        .revoke_navigation_context_websocket(capability, old)
        .unwrap_err();
    assert_eq!(
        stale.kind,
        HostControlErrorKind::InvalidWebSocketConnectionAuthority
    );

    let replacement = start(&mut host, capability, &mut transport);
    assert_eq!(replacement.serial(), old.serial() + 1);
    assert_eq!(transport.starts, 2);
}

#[test]
fn capability_revocation_and_process_loss_quarantine_backend_tickets() {
    let mut host = host_with_websocket_limit(4);
    let context = open_context(&mut host, "https://app.example/");
    let capability = network_capability(&mut host, context);
    let mut transport = FixtureTransport::default();
    let _ = start(&mut host, capability, &mut transport);

    host.revoke_navigation_context_capability(capability)
        .unwrap();
    assert_eq!(host.active_websocket_connections(), 0);
    assert_eq!(host.pending_websocket_aborts(), 1);
    host.abort_pending_websocket_connections(&mut transport)
        .unwrap();

    let context = open_context(&mut host, "https://other.example/");
    let capability = network_capability(&mut host, context);
    let _ = start(&mut host, capability, &mut transport);
    let snapshot = host.navigation_context(context).unwrap();
    let process = host.process_for_site(snapshot.site()).unwrap();
    host.process_lost(process).unwrap();

    assert_eq!(host.active_websocket_connections(), 0);
    assert_eq!(host.pending_websocket_aborts(), 1);
}

#[test]
fn backend_ticket_reuse_revokes_existing_host_authority() {
    let mut host = host_with_websocket_limit(4);
    let first_context = open_context(&mut host, "https://first.example/");
    let first_capability = network_capability(&mut host, first_context);
    let second_context = open_context(&mut host, "https://second.example/");
    let second_capability = network_capability(&mut host, second_context);
    let mut transport = FixtureTransport::with_fixed_ticket(7);
    let first = start(&mut host, first_capability, &mut transport);

    let reuse = host
        .start_navigation_context_websocket(second_capability, handshake(), &mut transport)
        .unwrap_err();
    assert_eq!(reuse.kind, HostControlErrorKind::InconsistentState);
    assert_eq!(host.active_websocket_connections(), 0);
    assert_eq!(host.pending_websocket_aborts(), 1);
    assert_eq!(host.tracked_websocket_connections(), 1);

    let stale = host
        .revoke_navigation_context_websocket(first_capability, first)
        .unwrap_err();
    assert_eq!(
        stale.kind,
        HostControlErrorKind::InvalidWebSocketConnectionAuthority
    );
    assert_eq!(transport.starts, 2);
}
