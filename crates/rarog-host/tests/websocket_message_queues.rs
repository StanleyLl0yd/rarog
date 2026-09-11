use rarog_host::{
    HostControlErrorKind, HostControlPlane, HostLimits, NavigationContextCapability,
    WebSocketInboundPoll, WebSocketOutboundFlush,
};
use rarog_url::{Origin, WebUrl};
use rarog_websocket::{
    WebSocketHandshakeIntent, WebSocketLimits, WebSocketMessage, WebSocketQueueErrorKind,
    WebSocketQueueLimits, WebSocketTransport, WebSocketTransportError, WebSocketTransportErrorKind,
    WebSocketTransportReceive, WebSocketTransportSend, WebSocketTransportTicket,
};
use std::collections::VecDeque;
use std::num::NonZeroU64;

fn host_with_queues(queue_limits: WebSocketQueueLimits) -> HostControlPlane {
    let limits = HostLimits {
        max_site_processes: 4,
        max_capabilities: 16,
        max_navigation_contexts: 8,
        max_websocket_connections: 8,
        websocket_queues: queue_limits,
        ..HostLimits::default()
    };
    HostControlPlane::try_new(limits).unwrap()
}

fn queue_limits() -> WebSocketQueueLimits {
    WebSocketQueueLimits {
        max_outbound_messages: 2,
        max_outbound_bytes: 5,
        max_inbound_messages: 2,
        max_inbound_bytes: 5,
    }
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
        &[],
        WebSocketLimits::default(),
    )
    .unwrap()
}

fn text(value: &str) -> WebSocketMessage {
    WebSocketMessage::text(value, WebSocketLimits::default()).unwrap()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SendMode {
    Accepted,
    Backpressure,
    Error,
}

#[derive(Debug)]
struct FixtureTransport {
    next_ticket: u64,
    send_mode: SendMode,
    sent: Vec<String>,
    receive_calls: Vec<usize>,
    inbound: VecDeque<WebSocketMessage>,
    aborts: Vec<u64>,
}

impl Default for FixtureTransport {
    fn default() -> Self {
        Self {
            next_ticket: 1,
            send_mode: SendMode::Accepted,
            sent: Vec::new(),
            receive_calls: Vec::new(),
            inbound: VecDeque::new(),
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
                "fixture send failed",
            )),
        }
    }

    fn receive(
        &mut self,
        _ticket: WebSocketTransportTicket,
        max_message_bytes: usize,
    ) -> Result<WebSocketTransportReceive, WebSocketTransportError> {
        self.receive_calls.push(max_message_bytes);
        Ok(match self.inbound.pop_front() {
            Some(message) => WebSocketTransportReceive::Message(message),
            None => WebSocketTransportReceive::Pending,
        })
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
fn outbound_fifo_backpressure_and_errors_preserve_owned_messages() {
    let mut host = host_with_queues(queue_limits());
    let context = open_context(&mut host, "https://app.example/");
    let capability = network_capability(&mut host, context);
    let mut transport = FixtureTransport::default();
    let connection = start(&mut host, capability, &mut transport);

    host.queue_navigation_context_websocket_message(capability, connection, text("ab"))
        .unwrap();
    host.queue_navigation_context_websocket_message(capability, connection, text("cde"))
        .unwrap();
    let snapshot = host
        .websocket_queue_snapshot(capability, connection)
        .unwrap();
    assert_eq!(snapshot.outbound_messages(), 2);
    assert_eq!(snapshot.outbound_bytes(), 5);
    assert_eq!(
        host.queue_navigation_context_websocket_message(capability, connection, text("x"))
            .unwrap_err()
            .kind,
        HostControlErrorKind::WebSocketQueue(WebSocketQueueErrorKind::OutboundMessageLimitExceeded)
    );

    transport.send_mode = SendMode::Backpressure;
    assert_eq!(
        host.flush_navigation_context_websocket_message(capability, connection, &mut transport)
            .unwrap(),
        WebSocketOutboundFlush::Backpressure
    );
    assert_eq!(
        host.websocket_queue_snapshot(capability, connection)
            .unwrap(),
        snapshot
    );

    transport.send_mode = SendMode::Error;
    let error = host
        .flush_navigation_context_websocket_message(capability, connection, &mut transport)
        .unwrap_err();
    assert_eq!(
        error.kind,
        HostControlErrorKind::WebSocket(WebSocketTransportErrorKind::Backend)
    );
    assert_eq!(
        host.websocket_queue_snapshot(capability, connection)
            .unwrap(),
        snapshot
    );

    transport.send_mode = SendMode::Accepted;
    assert_eq!(
        host.flush_navigation_context_websocket_message(capability, connection, &mut transport)
            .unwrap(),
        WebSocketOutboundFlush::Accepted
    );
    assert_eq!(transport.sent, vec!["ab"]);
    let after = host
        .websocket_queue_snapshot(capability, connection)
        .unwrap();
    assert_eq!(after.outbound_messages(), 1);
    assert_eq!(after.outbound_bytes(), 3);
}

#[test]
fn inbound_queue_advertises_remaining_capacity_and_skips_backend_when_full() {
    let mut host = host_with_queues(queue_limits());
    let context = open_context(&mut host, "https://app.example/");
    let capability = network_capability(&mut host, context);
    let mut transport = FixtureTransport::default();
    let connection = start(&mut host, capability, &mut transport);
    transport.inbound.push_back(text("abc"));
    transport.inbound.push_back(text("de"));
    transport.inbound.push_back(text("z"));

    assert_eq!(
        host.poll_navigation_context_websocket_message(capability, connection, &mut transport)
            .unwrap(),
        WebSocketInboundPoll::Queued
    );
    assert_eq!(
        host.poll_navigation_context_websocket_message(capability, connection, &mut transport)
            .unwrap(),
        WebSocketInboundPoll::Queued
    );
    assert_eq!(transport.receive_calls, vec![5, 2]);
    assert_eq!(
        host.poll_navigation_context_websocket_message(capability, connection, &mut transport)
            .unwrap(),
        WebSocketInboundPoll::QueueFull
    );
    assert_eq!(transport.receive_calls, vec![5, 2]);

    assert_eq!(
        host.take_navigation_context_websocket_message(capability, connection)
            .unwrap()
            .unwrap()
            .as_text(),
        Some("abc")
    );
    assert_eq!(
        host.poll_navigation_context_websocket_message(capability, connection, &mut transport)
            .unwrap(),
        WebSocketInboundPoll::Queued
    );
    assert_eq!(transport.receive_calls, vec![5, 2, 3]);
}

#[test]
fn byte_full_inbound_budget_still_polls_for_zero_byte_messages() {
    let mut limits = queue_limits();
    limits.max_inbound_bytes = 1;
    let mut host = host_with_queues(limits);
    let context = open_context(&mut host, "https://app.example/");
    let capability = network_capability(&mut host, context);
    let mut transport = FixtureTransport::default();
    let connection = start(&mut host, capability, &mut transport);
    transport.inbound.push_back(text("x"));
    transport.inbound.push_back(text(""));
    transport.inbound.push_back(text("z"));

    assert_eq!(
        host.poll_navigation_context_websocket_message(capability, connection, &mut transport)
            .unwrap(),
        WebSocketInboundPoll::Queued
    );
    assert_eq!(
        host.poll_navigation_context_websocket_message(capability, connection, &mut transport)
            .unwrap(),
        WebSocketInboundPoll::Queued
    );
    assert_eq!(transport.receive_calls, vec![1, 0]);
    let snapshot = host
        .websocket_queue_snapshot(capability, connection)
        .unwrap();
    assert_eq!(snapshot.inbound_messages(), 2);
    assert_eq!(snapshot.inbound_bytes(), 1);
    assert_eq!(
        host.poll_navigation_context_websocket_message(capability, connection, &mut transport)
            .unwrap(),
        WebSocketInboundPoll::QueueFull
    );
    assert_eq!(transport.receive_calls, vec![1, 0]);
}

#[test]
fn oversized_backend_input_fails_closed_and_quarantines_connection() {
    let mut limits = queue_limits();
    limits.max_inbound_bytes = 3;
    let mut host = host_with_queues(limits);
    let context = open_context(&mut host, "https://app.example/");
    let capability = network_capability(&mut host, context);
    let mut transport = FixtureTransport::default();
    let connection = start(&mut host, capability, &mut transport);
    transport.inbound.push_back(text("toolarge"));

    let error = host
        .poll_navigation_context_websocket_message(capability, connection, &mut transport)
        .unwrap_err();
    assert_eq!(error.kind, HostControlErrorKind::InconsistentState);
    assert_eq!(transport.receive_calls, vec![3]);
    assert_eq!(host.active_websocket_connections(), 0);
    assert_eq!(host.pending_websocket_aborts(), 1);
    assert_eq!(
        host.websocket_queue_snapshot(capability, connection)
            .unwrap_err()
            .kind,
        HostControlErrorKind::InvalidWebSocketConnectionAuthority
    );
}

#[test]
fn exact_authority_and_connection_identity_isolate_queues_before_backend_access() {
    let mut host = host_with_queues(queue_limits());
    let first_context = open_context(&mut host, "https://first.example/");
    let first_capability = network_capability(&mut host, first_context);
    let second_context = open_context(&mut host, "https://second.example/");
    let second_capability = network_capability(&mut host, second_context);
    let mut transport = FixtureTransport::default();
    let first = start(&mut host, first_capability, &mut transport);
    let second = start(&mut host, second_capability, &mut transport);

    host.queue_navigation_context_websocket_message(first_capability, first, text("one"))
        .unwrap();
    host.queue_navigation_context_websocket_message(second_capability, second, text("two"))
        .unwrap();
    let before = transport.sent.len();
    let error = host
        .flush_navigation_context_websocket_message(second_capability, first, &mut transport)
        .unwrap_err();
    assert_eq!(
        error.kind,
        HostControlErrorKind::InvalidWebSocketConnectionAuthority
    );
    assert_eq!(transport.sent.len(), before);

    host.flush_navigation_context_websocket_message(first_capability, first, &mut transport)
        .unwrap();
    host.flush_navigation_context_websocket_message(second_capability, second, &mut transport)
        .unwrap();
    assert_eq!(transport.sent, vec!["one", "two"]);
}

#[test]
fn lifecycle_revocation_drops_queues_while_backend_ticket_stays_quarantined() {
    let mut host = host_with_queues(queue_limits());
    let context = open_context(&mut host, "https://app.example/first");
    let capability = network_capability(&mut host, context);
    let mut transport = FixtureTransport::default();
    let connection = start(&mut host, capability, &mut transport);
    host.queue_navigation_context_websocket_message(capability, connection, text("abc"))
        .unwrap();
    transport.inbound.push_back(text("de"));
    host.poll_navigation_context_websocket_message(capability, connection, &mut transport)
        .unwrap();
    let snapshot = host
        .websocket_queue_snapshot(capability, connection)
        .unwrap();
    assert_eq!(snapshot.outbound_bytes(), 3);
    assert_eq!(snapshot.inbound_bytes(), 2);

    host.navigate_navigation_context(
        context,
        &WebUrl::parse("https://app.example/second").unwrap(),
    )
    .unwrap();
    assert_eq!(host.active_websocket_connections(), 0);
    assert_eq!(host.pending_websocket_aborts(), 1);
    assert_eq!(
        host.websocket_queue_snapshot(capability, connection)
            .unwrap_err()
            .kind,
        HostControlErrorKind::InvalidWebSocketConnectionAuthority
    );
    assert_eq!(host.tracked_websocket_connections(), 1);
}
