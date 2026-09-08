use super::{
    EndpointRole, IPC_PROTOCOL_VERSION, IpcEnvelope, IpcError, IpcErrorKind, IpcLimits,
    MessageKind, RequestId,
};

pub const IPC_WIRE_MAGIC: [u8; 4] = *b"RIPC";
pub const IPC_WIRE_HEADER_BYTES: usize = 24;

const ROLE_HOST: u8 = 1;
const ROLE_SITE: u8 = 2;
const KIND_REQUEST: u8 = 1;
const KIND_RESPONSE: u8 = 2;
const KIND_EVENT: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IpcWireHeader {
    version: u16,
    source: EndpointRole,
    destination: EndpointRole,
    kind: MessageKind,
    payload_len: usize,
}

impl IpcWireHeader {
    pub fn version(self) -> u16 {
        self.version
    }

    pub fn source(self) -> EndpointRole {
        self.source
    }

    pub fn destination(self) -> EndpointRole {
        self.destination
    }

    pub fn kind(self) -> MessageKind {
        self.kind
    }

    pub fn payload_len(self) -> usize {
        self.payload_len
    }

    pub fn into_envelope(
        self,
        payload: Vec<u8>,
        limits: IpcLimits,
    ) -> Result<IpcEnvelope, IpcError> {
        if payload.len() != self.payload_len {
            return Err(IpcError::new(
                IpcErrorKind::TruncatedWireFrame,
                format!(
                    "IPC wire payload length {} does not match declared length {}",
                    payload.len(),
                    self.payload_len
                ),
            ));
        }
        IpcEnvelope::try_new(
            self.version,
            self.source,
            self.destination,
            self.kind,
            payload,
            limits,
        )
    }
}

pub fn encode_wire_header(
    envelope: &IpcEnvelope,
    limits: IpcLimits,
) -> Result<[u8; IPC_WIRE_HEADER_BYTES], IpcError> {
    IpcEnvelope::try_new(
        envelope.version(),
        envelope.source(),
        envelope.destination(),
        envelope.kind(),
        envelope.payload().to_vec(),
        limits,
    )?;

    let payload_len = u32::try_from(envelope.payload().len()).map_err(|_| {
        IpcError::new(
            IpcErrorKind::MessageTooLarge,
            "IPC payload length cannot be represented by the wire format",
        )
    })?;
    let (kind, request_id) = match envelope.kind() {
        MessageKind::Request(id) => (KIND_REQUEST, id.get()),
        MessageKind::Response(id) => (KIND_RESPONSE, id.get()),
        MessageKind::Event => (KIND_EVENT, 0),
    };

    let mut header = [0u8; IPC_WIRE_HEADER_BYTES];
    header[0..4].copy_from_slice(&IPC_WIRE_MAGIC);
    header[4..6].copy_from_slice(&envelope.version().to_le_bytes());
    header[6] = encode_role(envelope.source());
    header[7] = encode_role(envelope.destination());
    header[8] = kind;
    header[12..20].copy_from_slice(&request_id.to_le_bytes());
    header[20..24].copy_from_slice(&payload_len.to_le_bytes());
    Ok(header)
}

pub fn decode_wire_header(header: &[u8], limits: IpcLimits) -> Result<IpcWireHeader, IpcError> {
    if header.len() != IPC_WIRE_HEADER_BYTES {
        return Err(IpcError::new(
            IpcErrorKind::TruncatedWireFrame,
            format!(
                "IPC wire header requires {IPC_WIRE_HEADER_BYTES} bytes; received {}",
                header.len()
            ),
        ));
    }
    if header[0..4] != IPC_WIRE_MAGIC {
        return Err(IpcError::new(
            IpcErrorKind::InvalidWireMagic,
            "IPC wire magic does not match the Rarog protocol",
        ));
    }
    if header[9..12] != [0, 0, 0] {
        return Err(IpcError::new(
            IpcErrorKind::InvalidWireEncoding,
            "IPC wire reserved header bytes must be zero",
        ));
    }

    let version = u16::from_le_bytes([header[4], header[5]]);
    if version != IPC_PROTOCOL_VERSION {
        return Err(IpcError::new(
            IpcErrorKind::UnsupportedVersion,
            format!("unsupported IPC protocol version {version}; expected {IPC_PROTOCOL_VERSION}"),
        ));
    }

    let source = decode_role(header[6])?;
    let destination = decode_role(header[7])?;
    if destination != source.peer() {
        return Err(IpcError::new(
            IpcErrorKind::InvalidRoute,
            "IPC wire route must cross the Host/Site boundary",
        ));
    }

    let request_raw = u64::from_le_bytes(header[12..20].try_into().expect("fixed header range"));
    let kind = match header[8] {
        KIND_REQUEST => MessageKind::Request(RequestId::try_new(request_raw)?),
        KIND_RESPONSE => MessageKind::Response(RequestId::try_new(request_raw)?),
        KIND_EVENT if request_raw == 0 => MessageKind::Event,
        KIND_EVENT => {
            return Err(IpcError::new(
                IpcErrorKind::InvalidWireEncoding,
                "IPC event wire frames cannot carry a request identity",
            ));
        }
        _ => {
            return Err(IpcError::new(
                IpcErrorKind::InvalidWireEncoding,
                "IPC wire message kind is unknown",
            ));
        }
    };

    let payload_u32 = u32::from_le_bytes(header[20..24].try_into().expect("fixed header range"));
    let payload_len = usize::try_from(payload_u32).map_err(|_| {
        IpcError::new(
            IpcErrorKind::MessageTooLarge,
            "IPC wire payload length is not representable on this target",
        )
    })?;
    if !limits.is_valid() {
        return Err(IpcError::new(
            IpcErrorKind::InvalidLimits,
            "IPC limits must be non-zero and message bytes cannot exceed queued bytes",
        ));
    }
    if payload_len > limits.max_message_bytes {
        return Err(IpcError::new(
            IpcErrorKind::MessageTooLarge,
            format!(
                "IPC wire payload declares {payload_len} bytes; limit is {}",
                limits.max_message_bytes
            ),
        ));
    }

    Ok(IpcWireHeader {
        version,
        source,
        destination,
        kind,
        payload_len,
    })
}

pub fn encode_wire_frame(envelope: &IpcEnvelope, limits: IpcLimits) -> Result<Vec<u8>, IpcError> {
    let header = encode_wire_header(envelope, limits)?;
    let capacity = IPC_WIRE_HEADER_BYTES
        .checked_add(envelope.payload().len())
        .ok_or_else(|| {
            IpcError::new(
                IpcErrorKind::MessageTooLarge,
                "IPC wire frame length overflow",
            )
        })?;
    let mut frame = Vec::with_capacity(capacity);
    frame.extend_from_slice(&header);
    frame.extend_from_slice(envelope.payload());
    Ok(frame)
}

pub fn decode_wire_frame(frame: &[u8], limits: IpcLimits) -> Result<IpcEnvelope, IpcError> {
    if frame.len() < IPC_WIRE_HEADER_BYTES {
        return Err(IpcError::new(
            IpcErrorKind::TruncatedWireFrame,
            "IPC wire frame is shorter than its fixed header",
        ));
    }
    let header = decode_wire_header(&frame[..IPC_WIRE_HEADER_BYTES], limits)?;
    let expected = IPC_WIRE_HEADER_BYTES
        .checked_add(header.payload_len())
        .ok_or_else(|| {
            IpcError::new(
                IpcErrorKind::MessageTooLarge,
                "IPC wire frame length overflow",
            )
        })?;
    if frame.len() < expected {
        return Err(IpcError::new(
            IpcErrorKind::TruncatedWireFrame,
            format!(
                "IPC wire frame requires {expected} bytes; received {}",
                frame.len()
            ),
        ));
    }
    if frame.len() > expected {
        return Err(IpcError::new(
            IpcErrorKind::TrailingWireBytes,
            format!(
                "IPC wire frame contains {} trailing bytes",
                frame.len() - expected
            ),
        ));
    }

    header.into_envelope(frame[IPC_WIRE_HEADER_BYTES..expected].to_vec(), limits)
}

fn encode_role(role: EndpointRole) -> u8 {
    match role {
        EndpointRole::Host => ROLE_HOST,
        EndpointRole::Site => ROLE_SITE,
    }
}

fn decode_role(value: u8) -> Result<EndpointRole, IpcError> {
    match value {
        ROLE_HOST => Ok(EndpointRole::Host),
        ROLE_SITE => Ok(EndpointRole::Site),
        _ => Err(IpcError::new(
            IpcErrorKind::InvalidWireEncoding,
            "IPC wire endpoint role is unknown",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> IpcLimits {
        IpcLimits {
            max_message_bytes: 64,
            max_queued_messages: 4,
            max_queued_bytes: 128,
        }
    }

    #[test]
    fn wire_round_trips_request_response_and_event() {
        let request = RequestId::try_new(7).unwrap();
        let envelopes = [
            IpcEnvelope::request(EndpointRole::Site, request, b"request".to_vec(), limits())
                .unwrap(),
            IpcEnvelope::response(EndpointRole::Host, request, b"response".to_vec(), limits())
                .unwrap(),
            IpcEnvelope::event(EndpointRole::Host, b"event".to_vec(), limits()).unwrap(),
        ];

        for envelope in envelopes {
            let frame = encode_wire_frame(&envelope, limits()).unwrap();
            assert_eq!(decode_wire_frame(&frame, limits()).unwrap(), envelope);
        }
    }

    #[test]
    fn wire_header_rejects_untrusted_fields_before_payload_allocation() {
        let envelope = IpcEnvelope::event(EndpointRole::Site, Vec::new(), limits()).unwrap();
        let mut header = encode_wire_header(&envelope, limits()).unwrap();

        header[0] ^= 0xff;
        assert_eq!(
            decode_wire_header(&header, limits()).unwrap_err().kind,
            IpcErrorKind::InvalidWireMagic
        );

        let mut header = encode_wire_header(&envelope, limits()).unwrap();
        header[6] = 99;
        assert_eq!(
            decode_wire_header(&header, limits()).unwrap_err().kind,
            IpcErrorKind::InvalidWireEncoding
        );

        let mut header = encode_wire_header(&envelope, limits()).unwrap();
        header[9] = 1;
        assert_eq!(
            decode_wire_header(&header, limits()).unwrap_err().kind,
            IpcErrorKind::InvalidWireEncoding
        );

        let mut header = encode_wire_header(&envelope, limits()).unwrap();
        header[20..24].copy_from_slice(&65u32.to_le_bytes());
        assert_eq!(
            decode_wire_header(&header, limits()).unwrap_err().kind,
            IpcErrorKind::MessageTooLarge
        );
    }

    #[test]
    fn event_correlation_and_frame_boundaries_fail_closed() {
        let envelope = IpcEnvelope::event(EndpointRole::Site, b"x".to_vec(), limits()).unwrap();
        let mut header = encode_wire_header(&envelope, limits()).unwrap();
        header[12..20].copy_from_slice(&1u64.to_le_bytes());
        assert_eq!(
            decode_wire_header(&header, limits()).unwrap_err().kind,
            IpcErrorKind::InvalidWireEncoding
        );

        let frame = encode_wire_frame(&envelope, limits()).unwrap();
        assert_eq!(
            decode_wire_frame(&frame[..frame.len() - 1], limits())
                .unwrap_err()
                .kind,
            IpcErrorKind::TruncatedWireFrame
        );

        let mut trailing = frame;
        trailing.push(0);
        assert_eq!(
            decode_wire_frame(&trailing, limits()).unwrap_err().kind,
            IpcErrorKind::TrailingWireBytes
        );
    }
}
