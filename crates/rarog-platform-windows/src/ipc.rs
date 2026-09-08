use rarog_host::{HostControlPlane, SiteLease};
#[cfg(target_os = "windows")]
use rarog_ipc::{EndpointRole, IPC_WIRE_HEADER_BYTES, decode_wire_header, encode_wire_header};
use rarog_ipc::{IpcEnvelope, IpcError, IpcErrorKind, IpcLimits};
use std::fmt;

pub const DEFAULT_MAX_WINDOWS_IPC_ENDPOINT_BYTES: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsIpcEndpointName(String);

impl WindowsIpcEndpointName {
    pub fn try_new(value: impl Into<String>) -> Result<Self, WindowsIpcError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > DEFAULT_MAX_WINDOWS_IPC_ENDPOINT_BYTES
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err(WindowsIpcError::new(
                WindowsIpcErrorKind::InvalidEndpointName,
                format!(
                    "Windows IPC endpoint names must be 1..={} ASCII [A-Za-z0-9._-] bytes",
                    DEFAULT_MAX_WINDOWS_IPC_ENDPOINT_BYTES
                ),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsIpcErrorKind {
    UnsupportedTarget,
    InvalidEndpointName,
    UnknownSiteProcess,
    InvalidEnvelopeDirection,
    Io,
    Ipc(IpcErrorKind),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsIpcError {
    pub kind: WindowsIpcErrorKind,
    pub message: String,
}

impl WindowsIpcError {
    fn new(kind: WindowsIpcErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn unsupported_target() -> Self {
        Self::new(
            WindowsIpcErrorKind::UnsupportedTarget,
            "Windows IPC transport is unavailable on this target",
        )
    }

    #[cfg(target_os = "windows")]
    fn io(error: std::io::Error) -> Self {
        Self::new(
            WindowsIpcErrorKind::Io,
            format!("Windows IPC transport failed: {error}"),
        )
    }
}

impl fmt::Display for WindowsIpcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for WindowsIpcError {}

impl From<IpcError> for WindowsIpcError {
    fn from(error: IpcError) -> Self {
        Self::new(WindowsIpcErrorKind::Ipc(error.kind), error.message)
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug)]
pub struct WindowsIpcListener {
    lease: SiteLease,
    limits: IpcLimits,
    listener: interprocess::local_socket::Listener,
}

#[cfg(not(target_os = "windows"))]
#[derive(Debug)]
pub struct WindowsIpcListener;

#[cfg(target_os = "windows")]
impl WindowsIpcListener {
    pub fn bind(
        host: &HostControlPlane,
        lease: SiteLease,
        endpoint: &WindowsIpcEndpointName,
        limits: IpcLimits,
    ) -> Result<Self, WindowsIpcError> {
        use interprocess::local_socket::{GenericNamespaced, ListenerOptions, prelude::*};

        if host.site_for_process(lease.process()).is_none() {
            return Err(WindowsIpcError::new(
                WindowsIpcErrorKind::UnknownSiteProcess,
                "Windows IPC listener requires a live Host Site-process lease",
            ));
        }
        let name = endpoint
            .as_str()
            .to_ns_name::<GenericNamespaced>()
            .map_err(WindowsIpcError::io)?;
        let listener = ListenerOptions::new()
            .name(name)
            .create_sync()
            .map_err(WindowsIpcError::io)?;
        Ok(Self {
            lease,
            limits,
            listener,
        })
    }

    pub fn accept(&self) -> Result<WindowsIpcConnection, WindowsIpcError> {
        use interprocess::local_socket::prelude::*;

        let stream = self.listener.accept().map_err(WindowsIpcError::io)?;
        Ok(WindowsIpcConnection {
            lease: self.lease,
            limits: self.limits,
            stream,
        })
    }
}

#[cfg(not(target_os = "windows"))]
impl WindowsIpcListener {
    pub fn bind(
        _host: &HostControlPlane,
        _lease: SiteLease,
        _endpoint: &WindowsIpcEndpointName,
        _limits: IpcLimits,
    ) -> Result<Self, WindowsIpcError> {
        Err(WindowsIpcError::unsupported_target())
    }

    pub fn accept(&self) -> Result<WindowsIpcConnection, WindowsIpcError> {
        Err(WindowsIpcError::unsupported_target())
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug)]
pub struct WindowsIpcConnection {
    lease: SiteLease,
    limits: IpcLimits,
    stream: interprocess::local_socket::Stream,
}

#[cfg(not(target_os = "windows"))]
#[derive(Debug)]
pub struct WindowsIpcConnection;

#[cfg(target_os = "windows")]
impl WindowsIpcConnection {
    pub fn site_lease(&self) -> SiteLease {
        self.lease
    }

    pub fn receive_from_site(&mut self) -> Result<IpcEnvelope, WindowsIpcError> {
        let envelope = read_envelope(&mut self.stream, self.limits)?;
        validate_direction(&envelope, EndpointRole::Site)?;
        Ok(envelope)
    }

    pub fn send_from_host(&mut self, envelope: &IpcEnvelope) -> Result<(), WindowsIpcError> {
        validate_direction(envelope, EndpointRole::Host)?;
        write_envelope(&mut self.stream, envelope, self.limits)
    }
}

#[cfg(not(target_os = "windows"))]
impl WindowsIpcConnection {
    pub fn site_lease(&self) -> SiteLease {
        panic!("Windows IPC connection cannot exist on a non-Windows target")
    }

    pub fn receive_from_site(&mut self) -> Result<IpcEnvelope, WindowsIpcError> {
        Err(WindowsIpcError::unsupported_target())
    }

    pub fn send_from_host(&mut self, _envelope: &IpcEnvelope) -> Result<(), WindowsIpcError> {
        Err(WindowsIpcError::unsupported_target())
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug)]
pub struct WindowsIpcSiteStream {
    limits: IpcLimits,
    stream: interprocess::local_socket::Stream,
}

#[cfg(not(target_os = "windows"))]
#[derive(Debug)]
pub struct WindowsIpcSiteStream;

#[cfg(target_os = "windows")]
impl WindowsIpcSiteStream {
    pub fn connect(
        endpoint: &WindowsIpcEndpointName,
        limits: IpcLimits,
    ) -> Result<Self, WindowsIpcError> {
        use interprocess::local_socket::{GenericNamespaced, Stream, prelude::*};

        let name = endpoint
            .as_str()
            .to_ns_name::<GenericNamespaced>()
            .map_err(WindowsIpcError::io)?;
        let stream = Stream::connect(name).map_err(WindowsIpcError::io)?;
        Ok(Self { limits, stream })
    }

    pub fn send_to_host(&mut self, envelope: &IpcEnvelope) -> Result<(), WindowsIpcError> {
        validate_direction(envelope, EndpointRole::Site)?;
        write_envelope(&mut self.stream, envelope, self.limits)
    }

    pub fn receive_from_host(&mut self) -> Result<IpcEnvelope, WindowsIpcError> {
        let envelope = read_envelope(&mut self.stream, self.limits)?;
        validate_direction(&envelope, EndpointRole::Host)?;
        Ok(envelope)
    }
}

#[cfg(not(target_os = "windows"))]
impl WindowsIpcSiteStream {
    pub fn connect(
        _endpoint: &WindowsIpcEndpointName,
        _limits: IpcLimits,
    ) -> Result<Self, WindowsIpcError> {
        Err(WindowsIpcError::unsupported_target())
    }

    pub fn send_to_host(&mut self, _envelope: &IpcEnvelope) -> Result<(), WindowsIpcError> {
        Err(WindowsIpcError::unsupported_target())
    }

    pub fn receive_from_host(&mut self) -> Result<IpcEnvelope, WindowsIpcError> {
        Err(WindowsIpcError::unsupported_target())
    }
}

#[cfg(target_os = "windows")]
fn validate_direction(
    envelope: &IpcEnvelope,
    expected_source: EndpointRole,
) -> Result<(), WindowsIpcError> {
    if envelope.source() == expected_source && envelope.destination() == expected_source.peer() {
        Ok(())
    } else {
        Err(WindowsIpcError::new(
            WindowsIpcErrorKind::InvalidEnvelopeDirection,
            format!(
                "Windows IPC endpoint expected {expected_source:?} -> {:?} envelope",
                expected_source.peer()
            ),
        ))
    }
}

#[cfg(target_os = "windows")]
fn read_envelope(
    stream: &mut interprocess::local_socket::Stream,
    limits: IpcLimits,
) -> Result<IpcEnvelope, WindowsIpcError> {
    use std::io::Read;

    let mut header_bytes = [0u8; IPC_WIRE_HEADER_BYTES];
    stream
        .read_exact(&mut header_bytes)
        .map_err(WindowsIpcError::io)?;
    let header = decode_wire_header(&header_bytes, limits)?;
    let mut payload = vec![0u8; header.payload_len()];
    stream
        .read_exact(&mut payload)
        .map_err(WindowsIpcError::io)?;
    Ok(header.into_envelope(payload, limits)?)
}

#[cfg(target_os = "windows")]
fn write_envelope(
    stream: &mut interprocess::local_socket::Stream,
    envelope: &IpcEnvelope,
    limits: IpcLimits,
) -> Result<(), WindowsIpcError> {
    use std::io::Write;

    let header = encode_wire_header(envelope, limits)?;
    stream.write_all(&header).map_err(WindowsIpcError::io)?;
    stream
        .write_all(envelope.payload())
        .map_err(WindowsIpcError::io)?;
    stream.flush().map_err(WindowsIpcError::io)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "windows")]
    use rarog_ipc::{MessageKind, RequestId};
    use rarog_url::WebUrl;

    fn limits() -> IpcLimits {
        IpcLimits {
            max_message_bytes: 64,
            max_queued_messages: 4,
            max_queued_bytes: 128,
        }
    }

    fn live_lease(host: &mut HostControlPlane) -> SiteLease {
        host.ensure_site(
            WebUrl::parse("https://example.com/")
                .unwrap()
                .site_identity()
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn windows_ipc_endpoint_names_are_bounded_and_local_tokens() {
        assert_eq!(
            WindowsIpcEndpointName::try_new("").unwrap_err().kind,
            WindowsIpcErrorKind::InvalidEndpointName
        );
        assert_eq!(
            WindowsIpcEndpointName::try_new("../remote")
                .unwrap_err()
                .kind,
            WindowsIpcErrorKind::InvalidEndpointName
        );
        assert_eq!(
            WindowsIpcEndpointName::try_new("x".repeat(DEFAULT_MAX_WINDOWS_IPC_ENDPOINT_BYTES + 1))
                .unwrap_err()
                .kind,
            WindowsIpcErrorKind::InvalidEndpointName
        );
        assert_eq!(
            WindowsIpcEndpointName::try_new("rarog-r4.site_1")
                .unwrap()
                .as_str(),
            "rarog-r4.site_1"
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn windows_ipc_transport_is_explicitly_unsupported_off_windows() {
        let mut host = HostControlPlane::with_default_limits().unwrap();
        let lease = live_lease(&mut host);
        let endpoint = WindowsIpcEndpointName::try_new("rarog-r4-portability").unwrap();

        assert_eq!(
            WindowsIpcListener::bind(&host, lease, &endpoint, limits())
                .unwrap_err()
                .kind,
            WindowsIpcErrorKind::UnsupportedTarget
        );
        assert_eq!(
            WindowsIpcSiteStream::connect(&endpoint, limits())
                .unwrap_err()
                .kind,
            WindowsIpcErrorKind::UnsupportedTarget
        );
    }

    #[cfg(target_os = "windows")]
    fn unique_endpoint(label: &str) -> WindowsIpcEndpointName {
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT: AtomicU64 = AtomicU64::new(1);
        WindowsIpcEndpointName::try_new(format!(
            "rarog-r4-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
        .unwrap()
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_ipc_round_trip_preserves_host_owned_site_binding() {
        let mut host = HostControlPlane::with_default_limits().unwrap();
        let lease = live_lease(&mut host);
        let endpoint = unique_endpoint("roundtrip");
        let listener = WindowsIpcListener::bind(&host, lease, &endpoint, limits()).unwrap();
        let site_endpoint = endpoint.clone();

        let site = std::thread::spawn(move || {
            let mut stream = WindowsIpcSiteStream::connect(&site_endpoint, limits()).unwrap();
            let request_id = RequestId::try_new(17).unwrap();
            let request =
                IpcEnvelope::request(EndpointRole::Site, request_id, b"ping".to_vec(), limits())
                    .unwrap();
            stream.send_to_host(&request).unwrap();
            let response = stream.receive_from_host().unwrap();
            assert_eq!(response.kind(), MessageKind::Response(request_id));
            assert_eq!(response.payload(), b"pong");
        });

        let mut connection = listener.accept().unwrap();
        assert_eq!(connection.site_lease().process(), lease.process());
        let request = connection.receive_from_site().unwrap();
        let request_id = request.kind().request_id().unwrap();
        assert_eq!(request.payload(), b"ping");

        let response =
            IpcEnvelope::response(EndpointRole::Host, request_id, b"pong".to_vec(), limits())
                .unwrap();
        connection.send_from_host(&response).unwrap();
        site.join().unwrap();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_ipc_rejects_self_asserted_role_without_changing_binding() {
        let mut host = HostControlPlane::with_default_limits().unwrap();
        let lease = live_lease(&mut host);
        let endpoint = unique_endpoint("forged-role");
        let listener = WindowsIpcListener::bind(&host, lease, &endpoint, limits()).unwrap();
        let site_endpoint = endpoint.clone();

        let site = std::thread::spawn(move || {
            let mut stream = WindowsIpcSiteStream::connect(&site_endpoint, limits()).unwrap();
            let forged =
                IpcEnvelope::event(EndpointRole::Host, b"forged".to_vec(), limits()).unwrap();
            write_envelope(&mut stream.stream, &forged, limits()).unwrap();
        });

        let mut connection = listener.accept().unwrap();
        assert_eq!(connection.site_lease().process(), lease.process());
        assert_eq!(
            connection.receive_from_site().unwrap_err().kind,
            WindowsIpcErrorKind::InvalidEnvelopeDirection
        );
        assert_eq!(connection.site_lease().process(), lease.process());
        site.join().unwrap();
    }
}
