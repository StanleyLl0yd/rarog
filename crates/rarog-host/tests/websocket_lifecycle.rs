use rarog_host::{
    HostControlErrorKind, HostControlPlane, HostLimits, NavigationContextCapability,
    WebSocketCloseProgress, WebSocketOutboundFlush,
};
use rarog_url::{Origin, WebUrl};
use rarog_websocket::{
    WebSocketCloseErrorKind, WebSocketCloseIntent, WebSocketCloseLimits, WebSocketHandshakeIntent,
    WebSocketLimits, WebSocketMessage, WebSocketQueueLimits, WebSocketReadyState,
    WebSocketTransport, WebSocketTransportClosePoll, WebSocketTransportCloseStart,
    WebSocketTransportError, WebSocketTransportErrorKind, WebSocketTransportReceive,
    WebSocketTransportSend, WebSocketTransportTicket,
};
use std::num::NonZeroU64;

fn host_with_limit(max_websocket_connections: usize) -> HostControlPlane {
    HostControlPlane::try_new(HostLimits {
        max_site_processes: 8,
        max_capabilities: 32,
        max_navigation_contexts: 16,
        max_websocket_connections,
        websocket_queues: WebSocketQueueLimits {
            max_outbound_messages: 4,
            max_outbound_bytes: 64,
            max_inbound_messages: 4,
            max_inbound_bytes: 64,
        },
        ..HostLimits::default()
    })
    .unwrap()
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
        "wss://socket.example/realtime",
        &["chat.v1"],
        WebSocketLimits::default(),
    )
    .unwrap()
}

fn message(value: &str) -> WebSocketMessage {
    WebSocketMessage::text(value, WebSocketLimits::default()).unwrap()
}

fn close(code: u16, reason: &str) -> WebSocketCloseIntent {
    WebSocketCloseIntent::try_new(Some(code), reason, WebSocketCloseLimits::default()).unwrap()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SendMode {
    Accepted,
    Backpressure,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReceiveMode {
    Pending,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CloseStartMode {
    Started,
    Backpressure,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClosePollMode {
    Pending,
    Closed,
    Error,
}

#[derive(Debug)]
struct FixtureTransport {
    next_ticket: u64,
    starts: usize,
    send_mode: SendMode,
    sent: Vec<String>,
    receive_mode: ReceiveMode,
    receive_calls: usize,
    close_start_mode: CloseStartMode,
    close_starts: Vec<(u64, Option<u16>, String)>,
    close_poll_mode: ClosePollMode,
    close_polls: usize,
    aborts: Vec<u64>,
}

impl Default for FixtureTransport {
    fn default() -> Self {
        Self {
            next_ticket: 1,
            starts: 0,
            send_mode: SendMode::Accepted,
            sent: Vec::new(),
            receive_mode: ReceiveMode::Pending,
            receive_calls: 0,
            close_start_mode: CloseStartMode::Started,
            close_starts: Vec::new(),
            close_poll_mode: ClosePollMode::Pending,
            close_polls: 0,
            aborts: Vec::new(),
        }
    }
}

impl WebSocketTransport for FixtureTransport {
    fn start(
        &mut self,
        _handshake: WebSocketHandshakeIntent,
        _client_origin: Origin,
    ) -> Result<WebSocketTransportTicket, WebSocketTransportError> {
        self.starts += 1;
        let ticket = WebSocketTransportTicket::new(NonZeroU64::new(self.next_ticket).unwrap());
        self.next_ticket += 1;
        Ok(ticket)
    }

    fn send(
        &mut self,
        _ticket: WebSocketTransportTicket,
        message: &WebSocketMessage,
    ) -> Result<WebSocketTransportSend, WebSocketTransportError> {
        match self.send_mode {
            SendMode::Accepted => {
                self.sent
                    .push(message.as_text().unwrap_or("<binary>").to_owned());
                Ok(WebSocketTransportSend::Accepted)
            }
            SendMode::Backpressure => Ok(WebSocketTransportSend::Backpressure),
            SendMode::Error => Err(WebSocketTransportError::new(
                WebSocketTransportErrorKind::Backend,
                "fixture send failure",
            )),
        }
    }

    fn receive(
        &mut self,
        _ticket: WebSocketTransportTicket,
        _max_message_bytes: usize,
    ) -> Result<WebSocketTransportReceive, WebSocketTransportError> {
        self.receive_calls += 1;
        match self.receive_mode {
            ReceiveMode::Pending => Ok(WebSocketTransportReceive::Pending),
            ReceiveMode::Error => Err(WebSocketTransportError::new(
                WebSocketTransportErrorKind::Backend,
                "fixture receive failure",
            )),
        }
    }

    fn begin_close(
        &mut self,
        ticket: WebSocketTransportTicket,
        close: &WebSocketCloseIntent,
    ) -> Result<WebSocketTransportCloseStart, WebSocketTransportError> {
        match self.close_start_mode {
            CloseStartMode::Started => {
                self.close_starts
                    .push((ticket.get(), close.code(), close.reason().to_owned()));
                Ok(WebSocketTransportCloseStart::Started)
            }
            CloseStartMode::Backpressure => Ok(WebSocketTransportCloseStart::Backpressure),
            CloseStartMode::Error => Err(WebSocketTransportError::new(
                WebSocketTransportErrorKind::Backend,
                "fixture begin-close failure",
            )),
        }
    }

    fn poll_close(
        &mut self,
        _ticket: WebSocketTransportTicket,
    ) -> Result<WebSocketTransportClosePoll, WebSocketTransportError> {
        self.close_polls += 1;
        match self.close_poll_mode {
            ClosePollMode::Pending => Ok(WebSocketTransportClosePoll::Pending),
            ClosePollMode::Closed => Ok(WebSocketTransportClosePoll::Closed),
            ClosePollMode::Error => Err(WebSocketTransportError::new(
                WebSocketTransportErrorKind::Backend,
                "fixture close-poll failure",
            )),
        }
    }

    fn abort(&mut self, ticket: WebSocketTransportTicket) -> Result<(), WebSocketTransportError> {
        self.aborts.push(ticket.get());
        Ok(())
    }
}

fn start(
    host: &mut HostControlPlane,
    capability: NavigationContextCapability,
    transport: &mut FixtureTransport,
) -> rarog_host::WebSocketConnectionId {
    host.start_navigation_context_websocket(capability, handshake(), transport)
        .unwrap()
}

#[test]
fn close_intent_validation_happens_before_any_close_backend_side_effect() {
    let mut host = host_with_limit(2);
    let context = open_context(&mut host, "https://app.example/");
    let capability = network_capability(&mut host, context);
    let mut transport = FixtureTransport::default();
    let connection = start(&mut host, capability, &mut transport);

    let error =
        WebSocketCloseIntent::try_new(Some(1001), "reserved", WebSocketCloseLimits::default())
            .unwrap_err();
    assert_eq!(error.kind, WebSocketCloseErrorKind::InvalidCode);
    assert!(transport.close_starts.is_empty());
    assert_eq!(
        host.websocket_ready_state(capability, connection).unwrap(),
        WebSocketReadyState::Open
    );
}

#[test]
fn close_request_is_idempotent_and_blocks_new_send_and_receive_work() {
    let mut host = host_with_limit(2);
    let context = open_context(&mut host, "https://app.example/");
    let capability = network_capability(&mut host, context);
    let mut transport = FixtureTransport::default();
    let connection = start(&mut host, capability, &mut transport);
    let intent = close(1000, "done");

    assert_eq!(
        host.begin_navigation_context_websocket_close(capability, connection, intent.clone())
            .unwrap(),
        WebSocketReadyState::Closing
    );
    assert_eq!(
        host.begin_navigation_context_websocket_close(capability, connection, intent)
            .unwrap(),
        WebSocketReadyState::Closing
    );
    assert_eq!(
        host.begin_navigation_context_websocket_close(capability, connection, close(3000, "other"))
            .unwrap_err()
            .kind,
        HostControlErrorKind::InvalidWebSocketLifecycleState
    );
    assert_eq!(
        host.queue_navigation_context_websocket_message(capability, connection, message("new"))
            .unwrap_err()
            .kind,
        HostControlErrorKind::InvalidWebSocketLifecycleState
    );
    assert_eq!(
        host.poll_navigation_context_websocket_message(capability, connection, &mut transport)
            .unwrap_err()
            .kind,
        HostControlErrorKind::InvalidWebSocketLifecycleState
    );
    assert_eq!(transport.receive_calls, 0);
    assert!(transport.close_starts.is_empty());
}

#[test]
fn closing_drains_existing_outbound_then_completes_without_quarantine() {
    let mut host = host_with_limit(1);
    let context = open_context(&mut host, "https://app.example/");
    let capability = network_capability(&mut host, context);
    let mut transport = FixtureTransport::default();
    let connection = start(&mut host, capability, &mut transport);
    host.queue_navigation_context_websocket_message(capability, connection, message("queued"))
        .unwrap();
    host.begin_navigation_context_websocket_close(capability, connection, close(1000, "done"))
        .unwrap();

    assert_eq!(
        host.progress_navigation_context_websocket_close(capability, connection, &mut transport)
            .unwrap(),
        WebSocketCloseProgress::Draining
    );
    assert!(transport.close_starts.is_empty());

    transport.send_mode = SendMode::Backpressure;
    assert_eq!(
        host.flush_navigation_context_websocket_message(capability, connection, &mut transport)
            .unwrap(),
        WebSocketOutboundFlush::Backpressure
    );
    assert_eq!(
        host.progress_navigation_context_websocket_close(capability, connection, &mut transport)
            .unwrap(),
        WebSocketCloseProgress::Draining
    );

    transport.send_mode = SendMode::Accepted;
    assert_eq!(
        host.flush_navigation_context_websocket_message(capability, connection, &mut transport)
            .unwrap(),
        WebSocketOutboundFlush::Accepted
    );
    assert_eq!(transport.sent, vec!["queued"]);

    transport.close_start_mode = CloseStartMode::Backpressure;
    assert_eq!(
        host.progress_navigation_context_websocket_close(capability, connection, &mut transport)
            .unwrap(),
        WebSocketCloseProgress::Backpressure
    );
    assert!(transport.close_starts.is_empty());

    transport.close_start_mode = CloseStartMode::Started;
    assert_eq!(
        host.progress_navigation_context_websocket_close(capability, connection, &mut transport)
            .unwrap(),
        WebSocketCloseProgress::Started
    );
    assert_eq!(
        transport.close_starts,
        vec![(1, Some(1000), "done".to_owned())]
    );

    assert_eq!(
        host.progress_navigation_context_websocket_close(capability, connection, &mut transport)
            .unwrap(),
        WebSocketCloseProgress::Pending
    );
    transport.close_poll_mode = ClosePollMode::Closed;
    assert_eq!(
        host.progress_navigation_context_websocket_close(capability, connection, &mut transport)
            .unwrap(),
        WebSocketCloseProgress::Closed
    );
    assert_eq!(host.active_websocket_connections(), 0);
    assert_eq!(host.pending_websocket_aborts(), 0);
    assert_eq!(host.tracked_websocket_connections(), 0);

    let polls = transport.close_polls;
    assert_eq!(
        host.progress_navigation_context_websocket_close(capability, connection, &mut transport)
            .unwrap_err()
            .kind,
        HostControlErrorKind::InvalidWebSocketConnectionAuthority
    );
    assert_eq!(transport.close_polls, polls);
}

#[test]
fn fatal_send_error_discards_queues_and_keeps_backend_cleanup_charged() {
    let mut host = host_with_limit(1);
    let context = open_context(&mut host, "https://app.example/");
    let capability = network_capability(&mut host, context);
    let mut transport = FixtureTransport::default();
    let connection = start(&mut host, capability, &mut transport);
    host.queue_navigation_context_websocket_message(capability, connection, message("queued"))
        .unwrap();
    transport.send_mode = SendMode::Error;

    assert_eq!(
        host.flush_navigation_context_websocket_message(capability, connection, &mut transport)
            .unwrap_err()
            .kind,
        HostControlErrorKind::WebSocket(WebSocketTransportErrorKind::Backend)
    );
    assert_eq!(host.active_websocket_connections(), 0);
    assert_eq!(host.pending_websocket_aborts(), 1);
    assert_eq!(host.tracked_websocket_connections(), 1);
    assert_eq!(
        host.websocket_queue_snapshot(capability, connection)
            .unwrap_err()
            .kind,
        HostControlErrorKind::InvalidWebSocketConnectionAuthority
    );

    let second_context = open_context(&mut host, "https://other.example/");
    let second_capability = network_capability(&mut host, second_context);
    assert_eq!(
        host.start_navigation_context_websocket(second_capability, handshake(), &mut transport)
            .unwrap_err()
            .kind,
        HostControlErrorKind::WebSocketConnectionLimitExceeded
    );
    assert_eq!(
        host.abort_pending_websocket_connections(&mut transport)
            .unwrap(),
        1
    );
    assert_eq!(host.pending_websocket_aborts(), 0);
    start(&mut host, second_capability, &mut transport);
}

#[test]
fn begin_close_backend_error_is_terminal_and_quarantined() {
    let mut host = host_with_limit(2);
    let context = open_context(&mut host, "https://app.example/");
    let capability = network_capability(&mut host, context);
    let mut transport = FixtureTransport::default();
    let connection = start(&mut host, capability, &mut transport);
    host.begin_navigation_context_websocket_close(capability, connection, close(1000, "done"))
        .unwrap();
    transport.close_start_mode = CloseStartMode::Error;

    assert_eq!(
        host.progress_navigation_context_websocket_close(capability, connection, &mut transport)
            .unwrap_err()
            .kind,
        HostControlErrorKind::WebSocket(WebSocketTransportErrorKind::Backend)
    );
    assert_eq!(host.active_websocket_connections(), 0);
    assert_eq!(host.pending_websocket_aborts(), 1);
    assert!(transport.close_starts.is_empty());
}

#[test]
fn fatal_receive_and_close_poll_errors_are_terminal() {
    let mut host = host_with_limit(2);
    let context = open_context(&mut host, "https://app.example/");
    let capability = network_capability(&mut host, context);
    let mut transport = FixtureTransport::default();
    let connection = start(&mut host, capability, &mut transport);
    transport.receive_mode = ReceiveMode::Error;
    assert_eq!(
        host.poll_navigation_context_websocket_message(capability, connection, &mut transport)
            .unwrap_err()
            .kind,
        HostControlErrorKind::WebSocket(WebSocketTransportErrorKind::Backend)
    );
    assert_eq!(host.pending_websocket_aborts(), 1);
    host.abort_pending_websocket_connections(&mut transport)
        .unwrap();

    let connection = start(&mut host, capability, &mut transport);
    host.begin_navigation_context_websocket_close(capability, connection, close(1000, "done"))
        .unwrap();
    transport.close_start_mode = CloseStartMode::Started;
    host.progress_navigation_context_websocket_close(capability, connection, &mut transport)
        .unwrap();
    transport.close_poll_mode = ClosePollMode::Error;
    assert_eq!(
        host.progress_navigation_context_websocket_close(capability, connection, &mut transport)
            .unwrap_err()
            .kind,
        HostControlErrorKind::WebSocket(WebSocketTransportErrorKind::Backend)
    );
    assert_eq!(host.active_websocket_connections(), 0);
    assert_eq!(host.pending_websocket_aborts(), 1);
}

#[test]
fn authority_revocation_during_closing_converges_on_one_cleanup_ticket() {
    let mut host = host_with_limit(2);
    let context = open_context(&mut host, "https://app.example/");
    let capability = network_capability(&mut host, context);
    let mut transport = FixtureTransport::default();
    let connection = start(&mut host, capability, &mut transport);
    host.begin_navigation_context_websocket_close(capability, connection, close(1000, "done"))
        .unwrap();

    host.revoke_capability(capability.id()).unwrap();
    assert_eq!(host.active_websocket_connections(), 0);
    assert_eq!(host.pending_websocket_aborts(), 1);
    assert_eq!(host.tracked_websocket_connections(), 1);
    assert_eq!(
        host.progress_navigation_context_websocket_close(capability, connection, &mut transport)
            .unwrap_err()
            .kind,
        HostControlErrorKind::InvalidNavigationContextCapabilityAuthority
    );
    assert_eq!(transport.close_polls, 0);
    assert_eq!(
        host.abort_pending_websocket_connections(&mut transport)
            .unwrap(),
        1
    );
    assert_eq!(transport.aborts.len(), 1);
}
