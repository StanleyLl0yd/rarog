mod wire;
pub use wire::*;

use std::collections::VecDeque;
use std::fmt;
use std::num::NonZeroU64;

pub const IPC_PROTOCOL_VERSION: u16 = 1;
pub const DEFAULT_MAX_MESSAGE_BYTES: usize = 1024 * 1024;
pub const DEFAULT_MAX_QUEUED_MESSAGES: usize = 256;
pub const DEFAULT_MAX_QUEUED_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IpcLimits {
    pub max_message_bytes: usize,
    pub max_queued_messages: usize,
    pub max_queued_bytes: usize,
}

impl IpcLimits {
    pub fn is_valid(self) -> bool {
        self.max_message_bytes > 0
            && self.max_queued_messages > 0
            && self.max_queued_bytes > 0
            && self.max_message_bytes <= self.max_queued_bytes
    }
}

impl Default for IpcLimits {
    fn default() -> Self {
        Self {
            max_message_bytes: DEFAULT_MAX_MESSAGE_BYTES,
            max_queued_messages: DEFAULT_MAX_QUEUED_MESSAGES,
            max_queued_bytes: DEFAULT_MAX_QUEUED_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EndpointRole {
    Host,
    Site,
}

impl EndpointRole {
    pub fn peer(self) -> Self {
        match self {
            Self::Host => Self::Site,
            Self::Site => Self::Host,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RequestId(NonZeroU64);

impl RequestId {
    pub fn try_new(raw: u64) -> Result<Self, IpcError> {
        NonZeroU64::new(raw).map(Self).ok_or_else(|| {
            IpcError::new(
                IpcErrorKind::InvalidRequestId,
                "IPC request identity must be non-zero",
            )
        })
    }

    pub fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "request:{}", self.get())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKind {
    Request(RequestId),
    Response(RequestId),
    Event,
}

impl MessageKind {
    pub fn request_id(self) -> Option<RequestId> {
        match self {
            Self::Request(id) | Self::Response(id) => Some(id),
            Self::Event => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpcErrorKind {
    InvalidLimits,
    UnsupportedVersion,
    InvalidRoute,
    InvalidRequestId,
    MessageTooLarge,
    QueueMessageLimitExceeded,
    QueueByteLimitExceeded,
    Disconnected,
    InvalidWireMagic,
    InvalidWireEncoding,
    TruncatedWireFrame,
    TrailingWireBytes,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IpcError {
    pub kind: IpcErrorKind,
    pub message: String,
}

impl IpcError {
    fn new(kind: IpcErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for IpcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for IpcError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IpcEnvelope {
    version: u16,
    source: EndpointRole,
    destination: EndpointRole,
    kind: MessageKind,
    payload: Vec<u8>,
}

impl IpcEnvelope {
    pub fn try_new(
        version: u16,
        source: EndpointRole,
        destination: EndpointRole,
        kind: MessageKind,
        payload: Vec<u8>,
        limits: IpcLimits,
    ) -> Result<Self, IpcError> {
        validate_limits(limits)?;
        let envelope = Self {
            version,
            source,
            destination,
            kind,
            payload,
        };
        envelope.validate(limits)?;
        Ok(envelope)
    }

    pub fn request(
        source: EndpointRole,
        request: RequestId,
        payload: Vec<u8>,
        limits: IpcLimits,
    ) -> Result<Self, IpcError> {
        Self::try_new(
            IPC_PROTOCOL_VERSION,
            source,
            source.peer(),
            MessageKind::Request(request),
            payload,
            limits,
        )
    }

    pub fn response(
        source: EndpointRole,
        request: RequestId,
        payload: Vec<u8>,
        limits: IpcLimits,
    ) -> Result<Self, IpcError> {
        Self::try_new(
            IPC_PROTOCOL_VERSION,
            source,
            source.peer(),
            MessageKind::Response(request),
            payload,
            limits,
        )
    }

    pub fn event(
        source: EndpointRole,
        payload: Vec<u8>,
        limits: IpcLimits,
    ) -> Result<Self, IpcError> {
        Self::try_new(
            IPC_PROTOCOL_VERSION,
            source,
            source.peer(),
            MessageKind::Event,
            payload,
            limits,
        )
    }

    pub fn version(&self) -> u16 {
        self.version
    }

    pub fn source(&self) -> EndpointRole {
        self.source
    }

    pub fn destination(&self) -> EndpointRole {
        self.destination
    }

    pub fn kind(&self) -> MessageKind {
        self.kind
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn into_payload(self) -> Vec<u8> {
        self.payload
    }

    fn validate(&self, limits: IpcLimits) -> Result<(), IpcError> {
        if self.version != IPC_PROTOCOL_VERSION {
            return Err(IpcError::new(
                IpcErrorKind::UnsupportedVersion,
                format!(
                    "unsupported IPC protocol version {}; expected {}",
                    self.version, IPC_PROTOCOL_VERSION
                ),
            ));
        }
        if self.source == self.destination {
            return Err(IpcError::new(
                IpcErrorKind::InvalidRoute,
                "IPC route must cross the Host/Site boundary",
            ));
        }
        if self.payload.len() > limits.max_message_bytes {
            return Err(IpcError::new(
                IpcErrorKind::MessageTooLarge,
                format!(
                    "IPC payload requires {} bytes; limit is {}",
                    self.payload.len(),
                    limits.max_message_bytes
                ),
            ));
        }
        Ok(())
    }
}

#[derive(Debug)]
struct DirectionQueue {
    source: EndpointRole,
    destination: EndpointRole,
    limits: IpcLimits,
    messages: VecDeque<IpcEnvelope>,
    queued_bytes: usize,
}

impl DirectionQueue {
    fn new(source: EndpointRole, destination: EndpointRole, limits: IpcLimits) -> Self {
        Self {
            source,
            destination,
            limits,
            messages: VecDeque::new(),
            queued_bytes: 0,
        }
    }

    fn push(&mut self, envelope: IpcEnvelope) -> Result<(), IpcError> {
        envelope.validate(self.limits)?;
        if envelope.source != self.source || envelope.destination != self.destination {
            return Err(IpcError::new(
                IpcErrorKind::InvalidRoute,
                "IPC envelope route does not match the channel direction",
            ));
        }
        if self.messages.len() >= self.limits.max_queued_messages {
            return Err(IpcError::new(
                IpcErrorKind::QueueMessageLimitExceeded,
                format!(
                    "IPC queue message limit {} reached",
                    self.limits.max_queued_messages
                ),
            ));
        }
        let queued_bytes = self
            .queued_bytes
            .checked_add(envelope.payload.len())
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorKind::QueueByteLimitExceeded,
                    "IPC queued byte count overflow",
                )
            })?;
        if queued_bytes > self.limits.max_queued_bytes {
            return Err(IpcError::new(
                IpcErrorKind::QueueByteLimitExceeded,
                format!(
                    "IPC queue would require {queued_bytes} bytes; limit is {}",
                    self.limits.max_queued_bytes
                ),
            ));
        }

        self.queued_bytes = queued_bytes;
        self.messages.push_back(envelope);
        Ok(())
    }

    fn pop(&mut self) -> Option<IpcEnvelope> {
        let envelope = self.messages.pop_front()?;
        self.queued_bytes = self
            .queued_bytes
            .checked_sub(envelope.payload.len())
            .expect("IPC queue byte accounting must match retained messages");
        Some(envelope)
    }

    fn clear(&mut self) {
        self.messages.clear();
        self.queued_bytes = 0;
    }

    fn len(&self) -> usize {
        self.messages.len()
    }

    fn queued_bytes(&self) -> usize {
        self.queued_bytes
    }
}

#[derive(Debug)]
pub struct IpcChannel {
    connected: bool,
    host_to_site: DirectionQueue,
    site_to_host: DirectionQueue,
}

impl IpcChannel {
    pub fn try_new(limits: IpcLimits) -> Result<Self, IpcError> {
        validate_limits(limits)?;
        Ok(Self {
            connected: true,
            host_to_site: DirectionQueue::new(EndpointRole::Host, EndpointRole::Site, limits),
            site_to_host: DirectionQueue::new(EndpointRole::Site, EndpointRole::Host, limits),
        })
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn send(&mut self, envelope: IpcEnvelope) -> Result<(), IpcError> {
        self.require_connected()?;
        match envelope.source {
            EndpointRole::Host => self.host_to_site.push(envelope),
            EndpointRole::Site => self.site_to_host.push(envelope),
        }
    }

    pub fn receive(&mut self, endpoint: EndpointRole) -> Result<Option<IpcEnvelope>, IpcError> {
        self.require_connected()?;
        Ok(match endpoint {
            EndpointRole::Host => self.site_to_host.pop(),
            EndpointRole::Site => self.host_to_site.pop(),
        })
    }

    pub fn queued_messages_for(&self, endpoint: EndpointRole) -> usize {
        match endpoint {
            EndpointRole::Host => self.site_to_host.len(),
            EndpointRole::Site => self.host_to_site.len(),
        }
    }

    pub fn queued_bytes_for(&self, endpoint: EndpointRole) -> usize {
        match endpoint {
            EndpointRole::Host => self.site_to_host.queued_bytes(),
            EndpointRole::Site => self.host_to_site.queued_bytes(),
        }
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
        self.host_to_site.clear();
        self.site_to_host.clear();
    }

    fn require_connected(&self) -> Result<(), IpcError> {
        if self.connected {
            Ok(())
        } else {
            Err(IpcError::new(
                IpcErrorKind::Disconnected,
                "IPC channel is disconnected",
            ))
        }
    }
}

fn validate_limits(limits: IpcLimits) -> Result<(), IpcError> {
    if limits.is_valid() {
        Ok(())
    } else {
        Err(IpcError::new(
            IpcErrorKind::InvalidLimits,
            "IPC limits must be non-zero and max message bytes must fit the queue byte budget",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(max_message: usize, max_messages: usize, max_bytes: usize) -> IpcLimits {
        IpcLimits {
            max_message_bytes: max_message,
            max_queued_messages: max_messages,
            max_queued_bytes: max_bytes,
        }
    }

    #[test]
    fn default_limits_are_bounded_and_valid() {
        let limits = IpcLimits::default();
        assert!(limits.is_valid());
        assert_eq!(limits.max_message_bytes, DEFAULT_MAX_MESSAGE_BYTES);
        assert_eq!(limits.max_queued_messages, DEFAULT_MAX_QUEUED_MESSAGES);
        assert_eq!(limits.max_queued_bytes, DEFAULT_MAX_QUEUED_BYTES);
    }

    #[test]
    fn invalid_limits_fail_before_channel_creation() {
        let error = IpcChannel::try_new(limits(8, 1, 4)).unwrap_err();
        assert_eq!(error.kind, IpcErrorKind::InvalidLimits);
        let error = IpcChannel::try_new(limits(0, 1, 1)).unwrap_err();
        assert_eq!(error.kind, IpcErrorKind::InvalidLimits);
    }

    #[test]
    fn protocol_version_and_routes_are_validated() {
        let limits = limits(16, 4, 32);
        let request = RequestId::try_new(1).unwrap();
        let version_error = IpcEnvelope::try_new(
            IPC_PROTOCOL_VERSION + 1,
            EndpointRole::Host,
            EndpointRole::Site,
            MessageKind::Request(request),
            Vec::new(),
            limits,
        )
        .unwrap_err();
        assert_eq!(version_error.kind, IpcErrorKind::UnsupportedVersion);

        let route_error = IpcEnvelope::try_new(
            IPC_PROTOCOL_VERSION,
            EndpointRole::Site,
            EndpointRole::Site,
            MessageKind::Event,
            Vec::new(),
            limits,
        )
        .unwrap_err();
        assert_eq!(route_error.kind, IpcErrorKind::InvalidRoute);
    }

    #[test]
    fn request_identity_is_nonzero_and_correlation_is_preserved() {
        let zero = RequestId::try_new(0).unwrap_err();
        assert_eq!(zero.kind, IpcErrorKind::InvalidRequestId);

        let limits = limits(16, 4, 32);
        let request_id = RequestId::try_new(7).unwrap();
        let mut channel = IpcChannel::try_new(limits).unwrap();
        channel
            .send(
                IpcEnvelope::request(EndpointRole::Host, request_id, b"ping".to_vec(), limits)
                    .unwrap(),
            )
            .unwrap();

        let request = channel.receive(EndpointRole::Site).unwrap().unwrap();
        assert_eq!(request.kind(), MessageKind::Request(request_id));

        channel
            .send(
                IpcEnvelope::response(EndpointRole::Site, request_id, b"pong".to_vec(), limits)
                    .unwrap(),
            )
            .unwrap();
        let response = channel.receive(EndpointRole::Host).unwrap().unwrap();
        assert_eq!(response.kind(), MessageKind::Response(request_id));
        assert_eq!(response.kind().request_id(), Some(request_id));
    }

    #[test]
    fn oversized_payload_is_rejected_at_envelope_boundary() {
        let limits = limits(4, 2, 8);
        let error = IpcEnvelope::event(EndpointRole::Host, vec![0; 5], limits).unwrap_err();
        assert_eq!(error.kind, IpcErrorKind::MessageTooLarge);
    }

    #[test]
    fn queue_count_backpressure_is_explicit() {
        let limits = limits(4, 1, 8);
        let mut channel = IpcChannel::try_new(limits).unwrap();
        channel
            .send(IpcEnvelope::event(EndpointRole::Host, vec![1], limits).unwrap())
            .unwrap();
        let error = channel
            .send(IpcEnvelope::event(EndpointRole::Host, vec![2], limits).unwrap())
            .unwrap_err();
        assert_eq!(error.kind, IpcErrorKind::QueueMessageLimitExceeded);
        assert_eq!(channel.queued_messages_for(EndpointRole::Site), 1);
    }

    #[test]
    fn queue_byte_backpressure_is_explicit() {
        let limits = limits(4, 4, 5);
        let mut channel = IpcChannel::try_new(limits).unwrap();
        channel
            .send(IpcEnvelope::event(EndpointRole::Host, vec![1; 3], limits).unwrap())
            .unwrap();
        channel
            .send(IpcEnvelope::event(EndpointRole::Host, vec![2; 2], limits).unwrap())
            .unwrap();
        let error = channel
            .send(IpcEnvelope::event(EndpointRole::Host, vec![3], limits).unwrap())
            .unwrap_err();
        assert_eq!(error.kind, IpcErrorKind::QueueByteLimitExceeded);
        assert_eq!(channel.queued_bytes_for(EndpointRole::Site), 5);
    }

    #[test]
    fn channel_revalidates_envelopes_against_its_own_limits() {
        let loose = limits(8, 4, 16);
        let strict = limits(4, 4, 8);
        let envelope = IpcEnvelope::event(EndpointRole::Host, vec![0; 5], loose).unwrap();
        let mut channel = IpcChannel::try_new(strict).unwrap();

        let error = channel.send(envelope).unwrap_err();

        assert_eq!(error.kind, IpcErrorKind::MessageTooLarge);
        assert_eq!(channel.queued_messages_for(EndpointRole::Site), 0);
    }

    #[test]
    fn receive_updates_queue_byte_accounting() {
        let limits = limits(8, 4, 16);
        let mut channel = IpcChannel::try_new(limits).unwrap();
        channel
            .send(IpcEnvelope::event(EndpointRole::Site, vec![1; 6], limits).unwrap())
            .unwrap();
        assert_eq!(channel.queued_bytes_for(EndpointRole::Host), 6);

        let received = channel.receive(EndpointRole::Host).unwrap().unwrap();

        assert_eq!(received.payload().len(), 6);
        assert_eq!(channel.queued_bytes_for(EndpointRole::Host), 0);
    }

    #[test]
    fn disconnect_discards_queued_work_and_fails_closed() {
        let limits = limits(8, 4, 16);
        let mut channel = IpcChannel::try_new(limits).unwrap();
        channel
            .send(IpcEnvelope::event(EndpointRole::Host, vec![1; 4], limits).unwrap())
            .unwrap();

        channel.disconnect();

        assert!(!channel.is_connected());
        assert_eq!(channel.queued_messages_for(EndpointRole::Site), 0);
        assert_eq!(channel.queued_bytes_for(EndpointRole::Site), 0);
        let error = channel
            .send(IpcEnvelope::event(EndpointRole::Host, Vec::new(), limits).unwrap())
            .unwrap_err();
        assert_eq!(error.kind, IpcErrorKind::Disconnected);
        let error = channel.receive(EndpointRole::Site).unwrap_err();
        assert_eq!(error.kind, IpcErrorKind::Disconnected);
    }
}
