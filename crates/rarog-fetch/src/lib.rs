use rarog_url::{Origin, WebUrl};
use std::fmt;
use std::num::NonZeroU64;

pub const DEFAULT_MAX_HEADERS: usize = 256;
pub const DEFAULT_MAX_HEADER_BYTES: usize = 64 * 1024;
pub const DEFAULT_MAX_REQUEST_BODY_BYTES: usize = 16 * 1024 * 1024;
pub const DEFAULT_MAX_RESPONSE_BODY_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FetchLimits {
    pub max_headers: usize,
    pub max_header_bytes: usize,
    pub max_request_body_bytes: usize,
    pub max_response_body_bytes: usize,
}

impl FetchLimits {
    pub fn is_valid(self) -> bool {
        self.max_headers > 0
            && self.max_header_bytes > 0
            && self.max_request_body_bytes > 0
            && self.max_response_body_bytes > 0
    }
}

impl Default for FetchLimits {
    fn default() -> Self {
        Self {
            max_headers: DEFAULT_MAX_HEADERS,
            max_header_bytes: DEFAULT_MAX_HEADER_BYTES,
            max_request_body_bytes: DEFAULT_MAX_REQUEST_BODY_BYTES,
            max_response_body_bytes: DEFAULT_MAX_RESPONSE_BODY_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FetchErrorKind {
    InvalidLimits,
    InvalidMethod,
    ForbiddenMethod,
    InvalidHeaderName,
    InvalidHeaderValue,
    HeaderCountLimitExceeded,
    HeaderBytesLimitExceeded,
    RequestBodyLimitExceeded,
    ResponseBodyLimitExceeded,
    InvalidStatus,
    Network,
    Policy,
    InvalidNetworkTicket,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FetchError {
    pub kind: FetchErrorKind,
    pub message: String,
}

impl FetchError {
    pub fn new(kind: FetchErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self::new(FetchErrorKind::Network, message)
    }

    pub fn policy(message: impl Into<String>) -> Self {
        Self::new(FetchErrorKind::Policy, message)
    }
}

impl fmt::Display for FetchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for FetchError {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FetchMethod(String);

impl FetchMethod {
    pub fn try_new(value: impl Into<String>) -> Result<Self, FetchError> {
        let value = value.into();
        if value.is_empty() || !value.bytes().all(is_http_token_byte) {
            return Err(FetchError::new(
                FetchErrorKind::InvalidMethod,
                "fetch method must be a non-empty HTTP token",
            ));
        }
        if ["CONNECT", "TRACE", "TRACK"]
            .into_iter()
            .any(|method| value.eq_ignore_ascii_case(method))
        {
            return Err(FetchError::new(
                FetchErrorKind::ForbiddenMethod,
                "fetch forbids CONNECT, TRACE and TRACK methods",
            ));
        }
        let canonical = ["DELETE", "GET", "HEAD", "OPTIONS", "POST", "PUT"]
            .into_iter()
            .find(|method| value.eq_ignore_ascii_case(method));
        let normalized = match canonical {
            Some(method) if value != method => method.to_owned(),
            _ => value,
        };
        Ok(Self(normalized))
    }

    pub fn get() -> Self {
        Self(String::from("GET"))
    }

    pub fn head() -> Self {
        Self(String::from("HEAD"))
    }

    pub fn post() -> Self {
        Self(String::from("POST"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn permits_body(&self) -> bool {
        !matches!(self.as_str(), "GET" | "HEAD")
    }
}

impl fmt::Display for FetchMethod {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    name: String,
    value: String,
}

impl Header {
    pub fn try_new(name: impl Into<String>, value: impl Into<String>) -> Result<Self, FetchError> {
        let name = normalize_header_name(name.into())?;
        let value = normalize_header_value(value.into())?;
        Ok(Self { name, value })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    fn byte_len(&self) -> usize {
        self.name.len().saturating_add(self.value.len())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeaderList {
    entries: Vec<Header>,
    bytes: usize,
    max_headers: usize,
    max_bytes: usize,
}

impl HeaderList {
    pub fn try_new(max_headers: usize, max_bytes: usize) -> Result<Self, FetchError> {
        if max_headers == 0 || max_bytes == 0 {
            return Err(FetchError::new(
                FetchErrorKind::InvalidLimits,
                "header limits must be non-zero",
            ));
        }
        Ok(Self {
            entries: Vec::new(),
            bytes: 0,
            max_headers,
            max_bytes,
        })
    }

    pub fn from_fetch_limits(limits: FetchLimits) -> Result<Self, FetchError> {
        Self::try_new(limits.max_headers, limits.max_header_bytes)
    }

    pub fn append(
        &mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), FetchError> {
        if self.entries.len() >= self.max_headers {
            return Err(FetchError::new(
                FetchErrorKind::HeaderCountLimitExceeded,
                format!("header count limit {} exceeded", self.max_headers),
            ));
        }
        let header = Header::try_new(name, value)?;
        let bytes = self.bytes.checked_add(header.byte_len()).ok_or_else(|| {
            FetchError::new(
                FetchErrorKind::HeaderBytesLimitExceeded,
                "header byte count overflow",
            )
        })?;
        if bytes > self.max_bytes {
            return Err(FetchError::new(
                FetchErrorKind::HeaderBytesLimitExceeded,
                format!("headers require {bytes} bytes; limit is {}", self.max_bytes),
            ));
        }
        self.entries.push(header);
        self.bytes = bytes;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn byte_len(&self) -> usize {
        self.bytes
    }

    pub fn iter(&self) -> impl Iterator<Item = &Header> {
        self.entries.iter()
    }

    pub fn get_first(&self, name: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name))
            .map(Header::value)
    }

    pub fn values<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.entries
            .iter()
            .filter(move |header| header.name.eq_ignore_ascii_case(name))
            .map(Header::value)
    }

    pub fn remove(&mut self, name: &str) -> usize {
        let mut removed = 0usize;
        let mut removed_bytes = 0usize;
        self.entries.retain(|header| {
            if header.name.eq_ignore_ascii_case(name) {
                removed = removed.saturating_add(1);
                removed_bytes = removed_bytes.saturating_add(header.byte_len());
                false
            } else {
                true
            }
        });
        self.bytes = self.bytes.saturating_sub(removed_bytes);
        removed
    }
}

impl Default for HeaderList {
    fn default() -> Self {
        Self::from_fetch_limits(FetchLimits::default()).expect("default fetch limits are valid")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestMode {
    SameOrigin,
    Cors,
    NoCors,
    Navigate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialsMode {
    Omit,
    SameOrigin,
    Include,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RedirectMode {
    Follow,
    Error,
    Manual,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestDestination {
    Empty,
    Document,
    Script,
    Style,
    Image,
    Font,
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FetchRequest {
    url: WebUrl,
    origin: Origin,
    method: FetchMethod,
    headers: HeaderList,
    body: Option<Vec<u8>>,
    mode: RequestMode,
    credentials: CredentialsMode,
    redirect: RedirectMode,
    destination: RequestDestination,
    limits: FetchLimits,
}

impl FetchRequest {
    pub fn try_new(url: WebUrl, origin: Origin, limits: FetchLimits) -> Result<Self, FetchError> {
        if !limits.is_valid() {
            return Err(FetchError::new(
                FetchErrorKind::InvalidLimits,
                "fetch limits must be non-zero",
            ));
        }
        Ok(Self {
            url: url.without_fragment(),
            origin,
            method: FetchMethod::get(),
            headers: HeaderList::from_fetch_limits(limits)?,
            body: None,
            mode: RequestMode::Cors,
            credentials: CredentialsMode::SameOrigin,
            redirect: RedirectMode::Follow,
            destination: RequestDestination::Empty,
            limits,
        })
    }

    pub fn new(url: WebUrl, origin: Origin) -> Self {
        Self::try_new(url, origin, FetchLimits::default()).expect("default fetch limits are valid")
    }

    pub fn url(&self) -> &WebUrl {
        &self.url
    }

    pub fn origin(&self) -> &Origin {
        &self.origin
    }

    pub fn method(&self) -> &FetchMethod {
        &self.method
    }

    pub fn headers(&self) -> &HeaderList {
        &self.headers
    }

    pub fn headers_mut(&mut self) -> &mut HeaderList {
        &mut self.headers
    }

    pub fn body(&self) -> Option<&[u8]> {
        self.body.as_deref()
    }

    pub fn mode(&self) -> RequestMode {
        self.mode
    }

    pub fn credentials(&self) -> CredentialsMode {
        self.credentials
    }

    pub fn redirect(&self) -> RedirectMode {
        self.redirect
    }

    pub fn destination(&self) -> &RequestDestination {
        &self.destination
    }

    pub fn limits(&self) -> FetchLimits {
        self.limits
    }

    pub fn set_method(&mut self, method: FetchMethod) -> Result<(), FetchError> {
        if !method.permits_body() && self.body.is_some() {
            return Err(FetchError::new(
                FetchErrorKind::InvalidMethod,
                "GET and HEAD requests cannot carry a fetch body",
            ));
        }
        self.method = method;
        Ok(())
    }

    pub fn set_body(&mut self, body: Option<Vec<u8>>) -> Result<(), FetchError> {
        if body.is_some() && !self.method.permits_body() {
            return Err(FetchError::new(
                FetchErrorKind::InvalidMethod,
                "GET and HEAD requests cannot carry a fetch body",
            ));
        }
        if let Some(body) = &body
            && body.len() > self.limits.max_request_body_bytes
        {
            return Err(FetchError::new(
                FetchErrorKind::RequestBodyLimitExceeded,
                format!(
                    "request body requires {} bytes; limit is {}",
                    body.len(),
                    self.limits.max_request_body_bytes
                ),
            ));
        }
        self.body = body;
        Ok(())
    }

    pub fn set_mode(&mut self, mode: RequestMode) {
        self.mode = mode;
    }

    pub fn set_credentials(&mut self, credentials: CredentialsMode) {
        self.credentials = credentials;
    }

    pub fn set_redirect(&mut self, redirect: RedirectMode) {
        self.redirect = redirect;
    }

    pub fn set_destination(&mut self, destination: RequestDestination) {
        self.destination = destination;
    }

    pub fn network_request(&self) -> NetworkRequest {
        NetworkRequest {
            url: self.url.clone(),
            method: self.method.clone(),
            headers: self.headers.clone(),
            body: self.body.clone(),
            max_response_body_bytes: self.limits.max_response_body_bytes,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkRequest {
    url: WebUrl,
    method: FetchMethod,
    headers: HeaderList,
    body: Option<Vec<u8>>,
    max_response_body_bytes: usize,
}

impl NetworkRequest {
    pub fn url(&self) -> &WebUrl {
        &self.url
    }

    pub fn method(&self) -> &FetchMethod {
        &self.method
    }

    pub fn headers(&self) -> &HeaderList {
        &self.headers
    }

    pub fn body(&self) -> Option<&[u8]> {
        self.body.as_deref()
    }

    pub fn max_response_body_bytes(&self) -> usize {
        self.max_response_body_bytes
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FetchResponse {
    url: Option<WebUrl>,
    status: u16,
    headers: HeaderList,
    body: Vec<u8>,
}

impl FetchResponse {
    pub fn try_new(
        url: Option<WebUrl>,
        status: u16,
        headers: HeaderList,
        body: Vec<u8>,
        max_body_bytes: usize,
    ) -> Result<Self, FetchError> {
        if !(100..=599).contains(&status) {
            return Err(FetchError::new(
                FetchErrorKind::InvalidStatus,
                format!("network response status {status} is outside 100..=599"),
            ));
        }
        if max_body_bytes == 0 {
            return Err(FetchError::new(
                FetchErrorKind::InvalidLimits,
                "response body limit must be non-zero",
            ));
        }
        if body.len() > max_body_bytes {
            return Err(FetchError::new(
                FetchErrorKind::ResponseBodyLimitExceeded,
                format!(
                    "response body requires {} bytes; limit is {max_body_bytes}",
                    body.len()
                ),
            ));
        }
        Ok(Self {
            url: url.map(|url| url.without_fragment()),
            status,
            headers,
            body,
        })
    }

    pub fn url(&self) -> Option<&WebUrl> {
        self.url.as_ref()
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn headers(&self) -> &HeaderList {
        &self.headers
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn is_success(&self) -> bool {
        (200..=299).contains(&self.status)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkTicket(NonZeroU64);

impl NetworkTicket {
    pub const fn new(value: NonZeroU64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetworkPoll {
    Pending,
    Complete(FetchResponse),
}

pub trait NetworkCapability {
    fn start(&mut self, request: NetworkRequest) -> Result<NetworkTicket, FetchError>;

    fn poll(&mut self, ticket: NetworkTicket) -> Result<NetworkPoll, FetchError>;

    fn cancel(&mut self, ticket: NetworkTicket) -> Result<(), FetchError>;
}

fn normalize_header_name(mut name: String) -> Result<String, FetchError> {
    if name.is_empty() || !name.bytes().all(is_http_token_byte) {
        return Err(FetchError::new(
            FetchErrorKind::InvalidHeaderName,
            "header name must be a non-empty HTTP token",
        ));
    }
    name.make_ascii_lowercase();
    Ok(name)
}

fn normalize_header_value(value: String) -> Result<String, FetchError> {
    if value.bytes().any(|byte| matches!(byte, 0 | b'\r' | b'\n')) {
        return Err(FetchError::new(
            FetchErrorKind::InvalidHeaderValue,
            "header value cannot contain NUL, CR or LF",
        ));
    }
    Ok(value
        .trim_matches(|character| matches!(character, ' ' | '\t'))
        .to_owned())
}

fn is_http_token_byte(byte: u8) -> bool {
    matches!(
        byte,
        b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' | b'^' | b'_'
            | b'`' | b'|' | b'~'
            | b'0'..=b'9'
            | b'A'..=b'Z'
            | b'a'..=b'z'
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn request() -> FetchRequest {
        let url = WebUrl::parse("https://api.example.com/data#client-fragment").unwrap();
        let origin = WebUrl::parse("https://app.example.com/")
            .unwrap()
            .origin()
            .unwrap();
        FetchRequest::new(url, origin)
    }

    #[test]
    fn request_strips_fragment_and_keeps_policy_out_of_network_projection() {
        let mut request = request();
        request.set_mode(RequestMode::SameOrigin);
        request.set_credentials(CredentialsMode::Include);
        request.set_redirect(RedirectMode::Manual);
        request.set_destination(RequestDestination::Script);
        request
            .headers_mut()
            .append("Accept", " text/javascript ")
            .unwrap();
        let network = request.network_request();
        assert_eq!(network.url().as_str(), "https://api.example.com/data");
        assert_eq!(network.method().as_str(), "GET");
        assert_eq!(
            network.headers().get_first("accept"),
            Some("text/javascript")
        );
    }

    #[test]
    fn methods_are_normalized_and_forbidden_methods_are_rejected() {
        assert_eq!(FetchMethod::try_new("post").unwrap().as_str(), "POST");
        assert_eq!(FetchMethod::try_new("gEt").unwrap().as_str(), "GET");
        assert_eq!(FetchMethod::try_new("PATCH").unwrap().as_str(), "PATCH");
        assert_eq!(FetchMethod::try_new("pAtCh").unwrap().as_str(), "pAtCh");
        assert_eq!(
            FetchMethod::try_new("TrAcE").unwrap_err().kind,
            FetchErrorKind::ForbiddenMethod
        );
        assert_eq!(
            FetchMethod::try_new("bad method").unwrap_err().kind,
            FetchErrorKind::InvalidMethod
        );
    }

    #[test]
    fn get_and_head_reject_request_bodies() {
        let mut request = request();
        assert_eq!(
            request.set_body(Some(vec![1])).unwrap_err().kind,
            FetchErrorKind::InvalidMethod
        );
        request.set_method(FetchMethod::post()).unwrap();
        request.set_body(Some(vec![1, 2, 3])).unwrap();
        assert_eq!(request.body(), Some(&[1, 2, 3][..]));
        assert_eq!(
            request.set_method(FetchMethod::head()).unwrap_err().kind,
            FetchErrorKind::InvalidMethod
        );
    }

    #[test]
    fn headers_preserve_duplicates_and_enforce_normalization_and_limits() {
        let mut headers = HeaderList::try_new(2, 64).unwrap();
        headers.append("X-Test", " one ").unwrap();
        headers.append("x-test", "two").unwrap();
        assert_eq!(headers.get_first("X-TEST"), Some("one"));
        assert_eq!(
            headers.values("x-test").collect::<Vec<_>>(),
            vec!["one", "two"]
        );
        assert_eq!(
            headers.append("third", "value").unwrap_err().kind,
            FetchErrorKind::HeaderCountLimitExceeded
        );
        assert_eq!(headers.remove("x-test"), 2);
        assert!(headers.is_empty());
        assert_eq!(
            Header::try_new("bad name", "x").unwrap_err().kind,
            FetchErrorKind::InvalidHeaderName
        );
        assert_eq!(
            Header::try_new("x-test", "line\r\nbreak").unwrap_err().kind,
            FetchErrorKind::InvalidHeaderValue
        );
    }

    #[test]
    fn header_removal_updates_byte_accounting_without_rescanning_survivors() {
        let mut headers = HeaderList::try_new(4, 64).unwrap();
        headers.append("X-Test", "one").unwrap();
        headers.append("Keep", "value").unwrap();
        headers.append("x-test", "two").unwrap();
        let expected = "keep".len() + "value".len();

        assert_eq!(headers.remove("X-TEST"), 2);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers.get_first("keep"), Some("value"));
        assert_eq!(headers.byte_len(), expected);
    }

    #[test]
    fn request_and_response_body_limits_are_enforced() {
        let limits = FetchLimits {
            max_headers: 4,
            max_header_bytes: 64,
            max_request_body_bytes: 2,
            max_response_body_bytes: 3,
        };
        let url = WebUrl::parse("https://example.com/").unwrap();
        let origin = url.origin().unwrap();
        let mut request = FetchRequest::try_new(url.clone(), origin, limits).unwrap();
        request.set_method(FetchMethod::post()).unwrap();
        assert_eq!(
            request.set_body(Some(vec![1, 2, 3])).unwrap_err().kind,
            FetchErrorKind::RequestBodyLimitExceeded
        );
        assert_eq!(
            FetchResponse::try_new(Some(url), 200, HeaderList::default(), vec![1, 2, 3, 4], 3)
                .unwrap_err()
                .kind,
            FetchErrorKind::ResponseBodyLimitExceeded
        );
    }

    #[test]
    fn response_validates_status_and_strips_url_fragment() {
        let url = WebUrl::parse("https://example.com/path#fragment").unwrap();
        let response =
            FetchResponse::try_new(Some(url), 204, HeaderList::default(), Vec::new(), 1024)
                .unwrap();
        assert_eq!(response.url().unwrap().as_str(), "https://example.com/path");
        assert!(response.is_success());
        assert_eq!(
            FetchResponse::try_new(None, 99, HeaderList::default(), Vec::new(), 1024)
                .unwrap_err()
                .kind,
            FetchErrorKind::InvalidStatus
        );
    }

    struct FixtureNetwork {
        next: u64,
        pending: BTreeMap<NetworkTicket, NetworkRequest>,
    }

    impl FixtureNetwork {
        fn new() -> Self {
            Self {
                next: 1,
                pending: BTreeMap::new(),
            }
        }
    }

    impl NetworkCapability for FixtureNetwork {
        fn start(&mut self, request: NetworkRequest) -> Result<NetworkTicket, FetchError> {
            let raw = NonZeroU64::new(self.next).ok_or_else(|| {
                FetchError::new(
                    FetchErrorKind::InvalidNetworkTicket,
                    "network ticket exhausted",
                )
            })?;
            self.next = self.next.checked_add(1).ok_or_else(|| {
                FetchError::new(
                    FetchErrorKind::InvalidNetworkTicket,
                    "network ticket exhausted",
                )
            })?;
            let ticket = NetworkTicket::new(raw);
            self.pending.insert(ticket, request);
            Ok(ticket)
        }

        fn poll(&mut self, ticket: NetworkTicket) -> Result<NetworkPoll, FetchError> {
            let request = self.pending.remove(&ticket).ok_or_else(|| {
                FetchError::new(
                    FetchErrorKind::InvalidNetworkTicket,
                    "unknown network ticket",
                )
            })?;
            let response = FetchResponse::try_new(
                Some(request.url().clone()),
                200,
                HeaderList::default(),
                b"ok".to_vec(),
                request.max_response_body_bytes(),
            )?;
            Ok(NetworkPoll::Complete(response))
        }

        fn cancel(&mut self, ticket: NetworkTicket) -> Result<(), FetchError> {
            self.pending.remove(&ticket).map(|_| ()).ok_or_else(|| {
                FetchError::new(
                    FetchErrorKind::InvalidNetworkTicket,
                    "unknown network ticket",
                )
            })
        }
    }

    #[test]
    fn network_capability_is_object_safe_and_receives_transport_projection() {
        fn execute(
            capability: &mut dyn NetworkCapability,
            request: &FetchRequest,
        ) -> FetchResponse {
            let ticket = capability.start(request.network_request()).unwrap();
            match capability.poll(ticket).unwrap() {
                NetworkPoll::Complete(response) => response,
                NetworkPoll::Pending => panic!("fixture completes immediately"),
            }
        }

        let mut network = FixtureNetwork::new();
        let response = execute(&mut network, &request());
        assert_eq!(response.status(), 200);
        assert_eq!(response.body(), b"ok");
    }
}
