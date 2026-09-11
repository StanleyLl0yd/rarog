mod queue;
pub use queue::*;

use rarog_url::{Origin, WebUrl};
use std::fmt;
use std::num::NonZeroU64;

pub const DEFAULT_MAX_WEBSOCKET_URL_BYTES: usize = 64 * 1024;
pub const DEFAULT_MAX_WEBSOCKET_SUBPROTOCOLS: usize = 32;
pub const DEFAULT_MAX_WEBSOCKET_SUBPROTOCOL_BYTES: usize = 256;
pub const DEFAULT_MAX_WEBSOCKET_MESSAGE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WebSocketLimits {
    pub max_url_bytes: usize,
    pub max_subprotocols: usize,
    pub max_subprotocol_bytes: usize,
    pub max_message_bytes: usize,
}

impl WebSocketLimits {
    pub fn is_valid(self) -> bool {
        self.max_url_bytes > 0
            && self.max_subprotocols > 0
            && self.max_subprotocol_bytes > 0
            && self.max_message_bytes > 0
    }
}

impl Default for WebSocketLimits {
    fn default() -> Self {
        Self {
            max_url_bytes: DEFAULT_MAX_WEBSOCKET_URL_BYTES,
            max_subprotocols: DEFAULT_MAX_WEBSOCKET_SUBPROTOCOLS,
            max_subprotocol_bytes: DEFAULT_MAX_WEBSOCKET_SUBPROTOCOL_BYTES,
            max_message_bytes: DEFAULT_MAX_WEBSOCKET_MESSAGE_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebSocketErrorKind {
    InvalidLimits,
    InvalidUrl,
    InvalidScheme,
    FragmentNotAllowed,
    UrlLimitExceeded,
    SubprotocolCountLimitExceeded,
    SubprotocolBytesLimitExceeded,
    InvalidSubprotocol,
    DuplicateSubprotocol,
    MessageLimitExceeded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebSocketError {
    pub kind: WebSocketErrorKind,
    pub message: String,
}

impl WebSocketError {
    fn new(kind: WebSocketErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for WebSocketError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for WebSocketError {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WebSocketUrl {
    url: WebUrl,
}

impl WebSocketUrl {
    pub fn parse(input: &str, limits: WebSocketLimits) -> Result<Self, WebSocketError> {
        validate_limits(limits)?;
        if input.len() > limits.max_url_bytes {
            return Err(WebSocketError::new(
                WebSocketErrorKind::UrlLimitExceeded,
                format!(
                    "WebSocket URL requires {} bytes; limit is {}",
                    input.len(),
                    limits.max_url_bytes
                ),
            ));
        }
        let url = WebUrl::parse(input).map_err(|error| {
            WebSocketError::new(
                WebSocketErrorKind::InvalidUrl,
                format!("invalid WebSocket URL: {error}"),
            )
        })?;
        Self::from_web_url(url, limits)
    }

    pub fn from_web_url(url: WebUrl, limits: WebSocketLimits) -> Result<Self, WebSocketError> {
        validate_limits(limits)?;
        if url.as_str().len() > limits.max_url_bytes {
            return Err(WebSocketError::new(
                WebSocketErrorKind::UrlLimitExceeded,
                format!(
                    "source WebSocket URL requires {} bytes; limit is {}",
                    url.as_str().len(),
                    limits.max_url_bytes
                ),
            ));
        }
        if url.fragment().is_some() {
            return Err(WebSocketError::new(
                WebSocketErrorKind::FragmentNotAllowed,
                "WebSocket URLs must not contain a fragment",
            ));
        }

        let normalized = match url.scheme() {
            "ws" | "wss" => url,
            "http" => rewrite_scheme(url, "ws")?,
            "https" => rewrite_scheme(url, "wss")?,
            scheme => {
                return Err(WebSocketError::new(
                    WebSocketErrorKind::InvalidScheme,
                    format!("unsupported WebSocket URL scheme {scheme:?}"),
                ));
            }
        };

        if normalized.as_str().len() > limits.max_url_bytes {
            return Err(WebSocketError::new(
                WebSocketErrorKind::UrlLimitExceeded,
                format!(
                    "canonical WebSocket URL requires {} bytes; limit is {}",
                    normalized.as_str().len(),
                    limits.max_url_bytes
                ),
            ));
        }
        Ok(Self { url: normalized })
    }

    pub fn as_web_url(&self) -> &WebUrl {
        &self.url
    }

    pub fn as_str(&self) -> &str {
        self.url.as_str()
    }

    pub fn is_secure(&self) -> bool {
        self.url.scheme() == "wss"
    }

    pub fn resource_name(&self) -> String {
        let path = if self.url.path().is_empty() {
            "/"
        } else {
            self.url.path()
        };
        match self.url.query() {
            Some(query) => format!("{path}?{query}"),
            None => path.to_owned(),
        }
    }
}

impl fmt::Display for WebSocketUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

fn rewrite_scheme(url: WebUrl, target_scheme: &str) -> Result<WebUrl, WebSocketError> {
    let serialized = url.as_str();
    let source_scheme_len = url.scheme().len();
    let rewritten = format!("{target_scheme}{}", &serialized[source_scheme_len..]);
    WebUrl::parse(&rewritten).map_err(|error| {
        WebSocketError::new(
            WebSocketErrorKind::InvalidUrl,
            format!("failed to normalize WebSocket URL scheme: {error}"),
        )
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebSocketSubprotocol {
    value: String,
}

impl WebSocketSubprotocol {
    fn parse(value: &str, limits: WebSocketLimits) -> Result<Self, WebSocketError> {
        if value.len() > limits.max_subprotocol_bytes {
            return Err(WebSocketError::new(
                WebSocketErrorKind::SubprotocolBytesLimitExceeded,
                format!(
                    "WebSocket subprotocol requires {} bytes; limit is {}",
                    value.len(),
                    limits.max_subprotocol_bytes
                ),
            ));
        }
        if value.is_empty() || !value.bytes().all(is_http_token_byte) {
            return Err(WebSocketError::new(
                WebSocketErrorKind::InvalidSubprotocol,
                "WebSocket subprotocol must be a non-empty HTTP token",
            ));
        }
        Ok(Self {
            value: value.to_owned(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for WebSocketSubprotocol {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebSocketProtocols {
    values: Vec<WebSocketSubprotocol>,
}

impl WebSocketProtocols {
    pub fn try_new(values: &[&str], limits: WebSocketLimits) -> Result<Self, WebSocketError> {
        validate_limits(limits)?;
        if values.len() > limits.max_subprotocols {
            return Err(WebSocketError::new(
                WebSocketErrorKind::SubprotocolCountLimitExceeded,
                format!(
                    "WebSocket handshake requests {} subprotocols; limit is {}",
                    values.len(),
                    limits.max_subprotocols
                ),
            ));
        }

        let mut protocols = Vec::with_capacity(values.len());
        for value in values {
            if protocols
                .iter()
                .any(|existing: &WebSocketSubprotocol| existing.as_str() == *value)
            {
                return Err(WebSocketError::new(
                    WebSocketErrorKind::DuplicateSubprotocol,
                    format!("duplicate WebSocket subprotocol {value:?}"),
                ));
            }
            protocols.push(WebSocketSubprotocol::parse(value, limits)?);
        }
        Ok(Self { values: protocols })
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &WebSocketSubprotocol> {
        self.values.iter()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebSocketHandshakeIntent {
    url: WebSocketUrl,
    protocols: WebSocketProtocols,
}

impl WebSocketHandshakeIntent {
    pub fn try_new(
        url: &str,
        protocols: &[&str],
        limits: WebSocketLimits,
    ) -> Result<Self, WebSocketError> {
        validate_limits(limits)?;
        Ok(Self {
            url: WebSocketUrl::parse(url, limits)?,
            protocols: WebSocketProtocols::try_new(protocols, limits)?,
        })
    }

    pub fn from_url(
        url: WebSocketUrl,
        protocols: &[&str],
        limits: WebSocketLimits,
    ) -> Result<Self, WebSocketError> {
        validate_limits(limits)?;
        if url.as_str().len() > limits.max_url_bytes {
            return Err(WebSocketError::new(
                WebSocketErrorKind::UrlLimitExceeded,
                format!(
                    "canonical WebSocket URL requires {} bytes; limit is {}",
                    url.as_str().len(),
                    limits.max_url_bytes
                ),
            ));
        }
        Ok(Self {
            url,
            protocols: WebSocketProtocols::try_new(protocols, limits)?,
        })
    }

    pub fn url(&self) -> &WebSocketUrl {
        &self.url
    }

    pub fn protocols(&self) -> &WebSocketProtocols {
        &self.protocols
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WebSocketTransportTicket(NonZeroU64);

impl WebSocketTransportTicket {
    pub const fn new(value: NonZeroU64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebSocketTransportErrorKind {
    InvalidTicket,
    Backend,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebSocketTransportError {
    pub kind: WebSocketTransportErrorKind,
    pub message: String,
}

impl WebSocketTransportError {
    pub fn new(kind: WebSocketTransportErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for WebSocketTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for WebSocketTransportError {}

pub const MAX_WEBSOCKET_CLOSE_REASON_BYTES: usize = 123;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WebSocketCloseLimits {
    pub max_reason_bytes: usize,
}

impl WebSocketCloseLimits {
    pub fn is_valid(self) -> bool {
        self.max_reason_bytes > 0 && self.max_reason_bytes <= MAX_WEBSOCKET_CLOSE_REASON_BYTES
    }
}

impl Default for WebSocketCloseLimits {
    fn default() -> Self {
        Self {
            max_reason_bytes: MAX_WEBSOCKET_CLOSE_REASON_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebSocketCloseErrorKind {
    InvalidLimits,
    InvalidCode,
    ReasonRequiresCode,
    ReasonLimitExceeded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebSocketCloseError {
    pub kind: WebSocketCloseErrorKind,
    pub message: String,
}

impl WebSocketCloseError {
    fn new(kind: WebSocketCloseErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for WebSocketCloseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for WebSocketCloseError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebSocketCloseIntent {
    code: Option<u16>,
    reason: String,
}

impl WebSocketCloseIntent {
    pub fn try_new(
        code: Option<u16>,
        reason: &str,
        limits: WebSocketCloseLimits,
    ) -> Result<Self, WebSocketCloseError> {
        if !limits.is_valid() {
            return Err(WebSocketCloseError::new(
                WebSocketCloseErrorKind::InvalidLimits,
                "WebSocket close reason limit must be between 1 and 123 bytes",
            ));
        }
        if code.is_none() && !reason.is_empty() {
            return Err(WebSocketCloseError::new(
                WebSocketCloseErrorKind::ReasonRequiresCode,
                "WebSocket close reason requires an explicit close code",
            ));
        }
        if let Some(code) = code {
            if code != 1000 && !(3000..=4999).contains(&code) {
                return Err(WebSocketCloseError::new(
                    WebSocketCloseErrorKind::InvalidCode,
                    format!("WebSocket local close code {code} is not allowed"),
                ));
            }
        }
        if reason.len() > limits.max_reason_bytes {
            return Err(WebSocketCloseError::new(
                WebSocketCloseErrorKind::ReasonLimitExceeded,
                format!(
                    "WebSocket close reason requires {} UTF-8 bytes; limit is {}",
                    reason.len(),
                    limits.max_reason_bytes
                ),
            ));
        }
        Ok(Self {
            code,
            reason: reason.to_owned(),
        })
    }

    pub fn empty() -> Self {
        Self {
            code: None,
            reason: String::new(),
        }
    }

    pub fn code(&self) -> Option<u16> {
        self.code
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebSocketTransportCloseStart {
    Started,
    Backpressure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebSocketTransportClosePoll {
    Pending,
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebSocketTransportSend {
    Accepted,
    Backpressure,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WebSocketTransportReceive {
    Pending,
    Message(WebSocketMessage),
}

pub trait WebSocketTransport {
    fn start(
        &mut self,
        handshake: WebSocketHandshakeIntent,
        client_origin: Origin,
    ) -> Result<WebSocketTransportTicket, WebSocketTransportError>;

    fn send(
        &mut self,
        ticket: WebSocketTransportTicket,
        message: &WebSocketMessage,
    ) -> Result<WebSocketTransportSend, WebSocketTransportError>;

    fn receive(
        &mut self,
        ticket: WebSocketTransportTicket,
        max_message_bytes: usize,
    ) -> Result<WebSocketTransportReceive, WebSocketTransportError>;

    fn begin_close(
        &mut self,
        ticket: WebSocketTransportTicket,
        close: &WebSocketCloseIntent,
    ) -> Result<WebSocketTransportCloseStart, WebSocketTransportError>;

    fn poll_close(
        &mut self,
        ticket: WebSocketTransportTicket,
    ) -> Result<WebSocketTransportClosePoll, WebSocketTransportError>;

    fn abort(&mut self, ticket: WebSocketTransportTicket) -> Result<(), WebSocketTransportError>;
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WebSocketReadyState {
    #[default]
    Connecting = 0,
    Open = 1,
    Closing = 2,
    Closed = 3,
}

impl WebSocketReadyState {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebSocketLifecycleErrorKind {
    InvalidTransition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebSocketLifecycleError {
    pub kind: WebSocketLifecycleErrorKind,
    pub message: String,
}

impl WebSocketLifecycleError {
    fn invalid_transition(from: WebSocketReadyState, to: WebSocketReadyState) -> Self {
        Self {
            kind: WebSocketLifecycleErrorKind::InvalidTransition,
            message: format!("invalid WebSocket lifecycle transition from {from:?} to {to:?}"),
        }
    }
}

impl fmt::Display for WebSocketLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for WebSocketLifecycleError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebSocketLifecycle {
    state: WebSocketReadyState,
}

impl WebSocketLifecycle {
    pub fn new() -> Self {
        Self {
            state: WebSocketReadyState::Connecting,
        }
    }

    pub fn state(&self) -> WebSocketReadyState {
        self.state
    }

    pub fn mark_open(&mut self) -> Result<(), WebSocketLifecycleError> {
        match self.state {
            WebSocketReadyState::Connecting => {
                self.state = WebSocketReadyState::Open;
                Ok(())
            }
            WebSocketReadyState::Open => Ok(()),
            state => Err(WebSocketLifecycleError::invalid_transition(
                state,
                WebSocketReadyState::Open,
            )),
        }
    }

    pub fn begin_closing(&mut self) -> Result<(), WebSocketLifecycleError> {
        match self.state {
            WebSocketReadyState::Connecting | WebSocketReadyState::Open => {
                self.state = WebSocketReadyState::Closing;
                Ok(())
            }
            WebSocketReadyState::Closing => Ok(()),
            WebSocketReadyState::Closed => Err(WebSocketLifecycleError::invalid_transition(
                WebSocketReadyState::Closed,
                WebSocketReadyState::Closing,
            )),
        }
    }

    pub fn mark_closed(&mut self) -> Result<(), WebSocketLifecycleError> {
        match self.state {
            WebSocketReadyState::Closed => Ok(()),
            WebSocketReadyState::Connecting
            | WebSocketReadyState::Open
            | WebSocketReadyState::Closing => {
                self.state = WebSocketReadyState::Closed;
                Ok(())
            }
        }
    }
}

impl Default for WebSocketLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebSocketMessageKind {
    Text,
    Binary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum WebSocketMessagePayload {
    Text(String),
    Binary(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebSocketMessage {
    payload: WebSocketMessagePayload,
}

impl WebSocketMessage {
    pub fn text(value: &str, limits: WebSocketLimits) -> Result<Self, WebSocketError> {
        validate_limits(limits)?;
        validate_message_len(value.len(), limits)?;
        Ok(Self {
            payload: WebSocketMessagePayload::Text(value.to_owned()),
        })
    }

    pub fn binary(value: &[u8], limits: WebSocketLimits) -> Result<Self, WebSocketError> {
        validate_limits(limits)?;
        validate_message_len(value.len(), limits)?;
        Ok(Self {
            payload: WebSocketMessagePayload::Binary(value.to_vec()),
        })
    }

    pub fn kind(&self) -> WebSocketMessageKind {
        match &self.payload {
            WebSocketMessagePayload::Text(_) => WebSocketMessageKind::Text,
            WebSocketMessagePayload::Binary(_) => WebSocketMessageKind::Binary,
        }
    }

    pub fn len(&self) -> usize {
        match &self.payload {
            WebSocketMessagePayload::Text(value) => value.len(),
            WebSocketMessagePayload::Binary(value) => value.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_text(&self) -> Option<&str> {
        match &self.payload {
            WebSocketMessagePayload::Text(value) => Some(value),
            WebSocketMessagePayload::Binary(_) => None,
        }
    }

    pub fn as_binary(&self) -> Option<&[u8]> {
        match &self.payload {
            WebSocketMessagePayload::Text(_) => None,
            WebSocketMessagePayload::Binary(value) => Some(value),
        }
    }
}

fn validate_limits(limits: WebSocketLimits) -> Result<(), WebSocketError> {
    if limits.is_valid() {
        Ok(())
    } else {
        Err(WebSocketError::new(
            WebSocketErrorKind::InvalidLimits,
            "WebSocket limits must all be non-zero",
        ))
    }
}

fn validate_message_len(len: usize, limits: WebSocketLimits) -> Result<(), WebSocketError> {
    if len <= limits.max_message_bytes {
        Ok(())
    } else {
        Err(WebSocketError::new(
            WebSocketErrorKind::MessageLimitExceeded,
            format!(
                "WebSocket message requires {len} bytes; limit is {}",
                limits.max_message_bytes
            ),
        ))
    }
}

fn is_http_token_byte(byte: u8) -> bool {
    matches!(
        byte,
        b'!' | b'#'
            | b'$'
            | b'%'
            | b'&'
            | b'\''
            | b'*'
            | b'+'
            | b'-'
            | b'.'
            | b'^'
            | b'_'
            | b'`'
            | b'|'
            | b'~'
            | b'0'..=b'9'
            | b'A'..=b'Z'
            | b'a'..=b'z'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> WebSocketLimits {
        WebSocketLimits {
            max_url_bytes: 128,
            max_subprotocols: 3,
            max_subprotocol_bytes: 12,
            max_message_bytes: 4,
        }
    }

    #[test]
    fn close_intent_validates_code_and_utf8_reason_before_copy() {
        let limits = WebSocketCloseLimits::default();
        let normal = WebSocketCloseIntent::try_new(Some(1000), "done", limits).unwrap();
        assert_eq!(normal.code(), Some(1000));
        assert_eq!(normal.reason(), "done");
        assert_eq!(WebSocketCloseIntent::empty().code(), None);

        assert_eq!(
            WebSocketCloseIntent::try_new(None, "reason", limits)
                .unwrap_err()
                .kind,
            WebSocketCloseErrorKind::ReasonRequiresCode
        );
        assert_eq!(
            WebSocketCloseIntent::try_new(Some(1001), "", limits)
                .unwrap_err()
                .kind,
            WebSocketCloseErrorKind::InvalidCode
        );
        assert!(WebSocketCloseIntent::try_new(Some(3000), "", limits).is_ok());
        assert!(WebSocketCloseIntent::try_new(Some(4999), "", limits).is_ok());

        let mut tiny = limits;
        tiny.max_reason_bytes = 3;
        assert_eq!(
            WebSocketCloseIntent::try_new(Some(1000), "éé", tiny)
                .unwrap_err()
                .kind,
            WebSocketCloseErrorKind::ReasonLimitExceeded
        );
        tiny.max_reason_bytes = MAX_WEBSOCKET_CLOSE_REASON_BYTES + 1;
        assert_eq!(
            WebSocketCloseIntent::try_new(Some(1000), "", tiny)
                .unwrap_err()
                .kind,
            WebSocketCloseErrorKind::InvalidLimits
        );
    }

    #[test]
    fn lifecycle_never_revives_after_terminal_close() {
        let mut lifecycle = WebSocketLifecycle::new();
        assert_eq!(lifecycle.state(), WebSocketReadyState::Connecting);
        lifecycle.mark_open().unwrap();
        assert_eq!(lifecycle.state(), WebSocketReadyState::Open);
        lifecycle.begin_closing().unwrap();
        assert_eq!(lifecycle.state(), WebSocketReadyState::Closing);
        lifecycle.mark_closed().unwrap();
        lifecycle.mark_closed().unwrap();
        assert_eq!(lifecycle.state(), WebSocketReadyState::Closed);
        assert_eq!(
            lifecycle.mark_open().unwrap_err().kind,
            WebSocketLifecycleErrorKind::InvalidTransition
        );
        assert_eq!(
            lifecycle.begin_closing().unwrap_err().kind,
            WebSocketLifecycleErrorKind::InvalidTransition
        );

        let mut connecting = WebSocketLifecycle::new();
        connecting.begin_closing().unwrap();
        connecting.mark_closed().unwrap();
        assert_eq!(connecting.state(), WebSocketReadyState::Closed);
    }

    #[test]
    fn constructor_schemes_normalize_to_websocket_schemes() {
        let ws = WebSocketUrl::parse("http://Example.com/chat?q=1", limits()).unwrap();
        assert_eq!(ws.as_str(), "ws://example.com/chat?q=1");
        assert!(!ws.is_secure());

        let wss = WebSocketUrl::parse("https://Example.com/chat", limits()).unwrap();
        assert_eq!(wss.as_str(), "wss://example.com/chat");
        assert!(wss.is_secure());

        assert_eq!(
            WebSocketUrl::parse("ws://example.com/", limits())
                .unwrap()
                .as_str(),
            "ws://example.com/"
        );
        assert_eq!(
            WebSocketUrl::parse("wss://example.com/", limits())
                .unwrap()
                .as_str(),
            "wss://example.com/"
        );
    }

    #[test]
    fn invalid_scheme_and_fragment_are_rejected() {
        let scheme = WebSocketUrl::parse("ftp://example.com/socket", limits()).unwrap_err();
        assert_eq!(scheme.kind, WebSocketErrorKind::InvalidScheme);

        let fragment =
            WebSocketUrl::parse("wss://example.com/socket#fragment", limits()).unwrap_err();
        assert_eq!(fragment.kind, WebSocketErrorKind::FragmentNotAllowed);
    }

    #[test]
    fn websocket_url_is_bounded_before_and_after_canonicalization() {
        let mut small = limits();
        small.max_url_bytes = 8;
        let raw = WebSocketUrl::parse("wss://example.com/", small).unwrap_err();
        assert_eq!(raw.kind, WebSocketErrorKind::UrlLimitExceeded);

        let mut canonical = limits();
        canonical.max_url_bytes = 24;
        let encoded = WebSocketUrl::parse("ws://example.com/a b c", canonical).unwrap_err();
        assert_eq!(encoded.kind, WebSocketErrorKind::UrlLimitExceeded);
    }

    #[test]
    fn from_web_url_bounds_source_before_scheme_rewrite() {
        let source = WebUrl::parse("https://example.com/a-long-path").unwrap();
        let mut small = limits();
        small.max_url_bytes = 16;
        let error = WebSocketUrl::from_web_url(source, small).unwrap_err();
        assert_eq!(error.kind, WebSocketErrorKind::UrlLimitExceeded);
    }

    #[test]
    fn resource_name_contains_path_and_query_but_not_fragment() {
        let url = WebSocketUrl::parse("wss://example.com/chat/room?q=one", limits()).unwrap();
        assert_eq!(url.resource_name(), "/chat/room?q=one");
    }

    #[test]
    fn subprotocols_preserve_order_and_validate_http_tokens() {
        let protocols =
            WebSocketProtocols::try_new(&["chat", "super.v2", "x_y"], limits()).unwrap();
        assert_eq!(
            protocols
                .iter()
                .map(WebSocketSubprotocol::as_str)
                .collect::<Vec<_>>(),
            vec!["chat", "super.v2", "x_y"]
        );

        for invalid in ["", "two words", "a,b", "ümlaut"] {
            let error = WebSocketProtocols::try_new(&[invalid], limits()).unwrap_err();
            assert_eq!(error.kind, WebSocketErrorKind::InvalidSubprotocol);
        }
    }

    #[test]
    fn duplicate_protocols_are_rejected_case_sensitively() {
        let duplicate = WebSocketProtocols::try_new(&["chat", "chat"], limits()).unwrap_err();
        assert_eq!(duplicate.kind, WebSocketErrorKind::DuplicateSubprotocol);

        let distinct = WebSocketProtocols::try_new(&["chat", "Chat"], limits()).unwrap();
        assert_eq!(distinct.len(), 2);
    }

    #[test]
    fn protocol_count_and_bytes_are_bounded() {
        let count = WebSocketProtocols::try_new(&["a", "b", "c", "d"], limits()).unwrap_err();
        assert_eq!(
            count.kind,
            WebSocketErrorKind::SubprotocolCountLimitExceeded
        );

        let bytes = WebSocketProtocols::try_new(&["this-is-longer"], limits()).unwrap_err();
        assert_eq!(
            bytes.kind,
            WebSocketErrorKind::SubprotocolBytesLimitExceeded
        );
    }

    #[test]
    fn handshake_intent_owns_only_validated_semantic_data() {
        let handshake = WebSocketHandshakeIntent::try_new(
            "https://example.com/socket?room=1",
            &["chat", "super.v2"],
            limits(),
        )
        .unwrap();
        assert_eq!(handshake.url().as_str(), "wss://example.com/socket?room=1");
        assert_eq!(handshake.protocols().len(), 2);
    }

    #[test]
    fn ready_state_values_match_websocket_semantics() {
        assert_eq!(
            WebSocketReadyState::default(),
            WebSocketReadyState::Connecting
        );
        assert_eq!(WebSocketReadyState::Connecting.as_u8(), 0);
        assert_eq!(WebSocketReadyState::Open.as_u8(), 1);
        assert_eq!(WebSocketReadyState::Closing.as_u8(), 2);
        assert_eq!(WebSocketReadyState::Closed.as_u8(), 3);
    }

    #[test]
    fn text_messages_are_bounded_by_utf8_bytes() {
        let message = WebSocketMessage::text("éé", limits()).unwrap();
        assert_eq!(message.kind(), WebSocketMessageKind::Text);
        assert_eq!(message.len(), 4);
        assert_eq!(message.as_text(), Some("éé"));
        assert_eq!(message.as_binary(), None);

        let error = WebSocketMessage::text("ééé", limits()).unwrap_err();
        assert_eq!(error.kind, WebSocketErrorKind::MessageLimitExceeded);
    }

    #[test]
    fn binary_messages_are_bounded_before_copy() {
        let message = WebSocketMessage::binary(&[1, 2, 3, 4], limits()).unwrap();
        assert_eq!(message.kind(), WebSocketMessageKind::Binary);
        assert_eq!(message.len(), 4);
        assert_eq!(message.as_binary(), Some(&[1, 2, 3, 4][..]));
        assert_eq!(message.as_text(), None);

        let error = WebSocketMessage::binary(&[1, 2, 3, 4, 5], limits()).unwrap_err();
        assert_eq!(error.kind, WebSocketErrorKind::MessageLimitExceeded);
    }

    #[test]
    fn zero_limits_are_rejected() {
        let invalid = WebSocketLimits {
            max_url_bytes: 0,
            ..limits()
        };
        let error = WebSocketUrl::parse("ws://example.com/", invalid).unwrap_err();
        assert_eq!(error.kind, WebSocketErrorKind::InvalidLimits);
    }
}
