//! Bounded, versioned IPC frames and typed message payloads.
//!
//! This module validates bounded frames and session state. Stream transport is
//! deliberately small and does not itself authorize capabilities or isolate a process.

use std::{error::Error, fmt};

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;
const MAGIC: &[u8; 4] = b"PHIP";
const HEADER_LENGTH: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageKind {
    Hello = 1,
    RunSource = 2,
    RunBytecode = 3,
    Stdout = 4,
    Stderr = 5,
    CapabilityRequest = 6,
    CapabilityResponse = 7,
    Completed = 8,
    Error = 9,
    Cancel = 10,
}

impl TryFrom<u8> for MessageKind {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Hello),
            2 => Ok(Self::RunSource),
            3 => Ok(Self::RunBytecode),
            4 => Ok(Self::Stdout),
            5 => Ok(Self::Stderr),
            6 => Ok(Self::CapabilityRequest),
            7 => Ok(Self::CapabilityResponse),
            8 => Ok(Self::Completed),
            9 => Ok(Self::Error),
            10 => Ok(Self::Cancel),
            _ => Err(ProtocolError::new("Unknown IPC message kind")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CapabilityId {
    ConsoleRead = 1,
    FileRead = 2,
    FileWrite = 3,
    NetworkConnect = 4,
    Clock = 5,
    Random = 6,
}

impl TryFrom<u8> for CapabilityId {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::ConsoleRead),
            2 => Ok(Self::FileRead),
            3 => Ok(Self::FileWrite),
            4 => Ok(Self::NetworkConnect),
            5 => Ok(Self::Clock),
            6 => Ok(Self::Random),
            _ => Err(ProtocolError::new("Unknown IPC capability identifier")),
        }
    }
}

/// A validated, message-specific IPC payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Hello { minimum_version: u16, maximum_version: u16 },
    RunSource(String),
    RunBytecode(Vec<u8>),
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    CapabilityRequest { capability: CapabilityId },
    CapabilityResponse { granted: bool },
    Completed,
    Error { code: u16, message: String },
    Cancel,
}

impl Message {
    pub const fn kind(&self) -> MessageKind {
        match self {
            Self::Hello { .. } => MessageKind::Hello,
            Self::RunSource(_) => MessageKind::RunSource,
            Self::RunBytecode(_) => MessageKind::RunBytecode,
            Self::Stdout(_) => MessageKind::Stdout,
            Self::Stderr(_) => MessageKind::Stderr,
            Self::CapabilityRequest { .. } => MessageKind::CapabilityRequest,
            Self::CapabilityResponse { .. } => MessageKind::CapabilityResponse,
            Self::Completed => MessageKind::Completed,
            Self::Error { .. } => MessageKind::Error,
            Self::Cancel => MessageKind::Cancel,
        }
    }

    fn encode_payload(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut payload = Vec::new();
        match self {
            Self::Hello { minimum_version, maximum_version } => {
                validate_version_range(*minimum_version, *maximum_version)?;
                payload.extend_from_slice(&minimum_version.to_le_bytes());
                payload.extend_from_slice(&maximum_version.to_le_bytes());
            }
            Self::RunSource(source) => {
                if source.is_empty() {
                    return Err(ProtocolError::new("Source request must not be empty"));
                }
                payload.extend_from_slice(source.as_bytes());
            }
            Self::RunBytecode(bytecode) => {
                if bytecode.is_empty() {
                    return Err(ProtocolError::new("Bytecode request must not be empty"));
                }
                payload.extend_from_slice(bytecode);
            }
            Self::Stdout(bytes) | Self::Stderr(bytes) => payload.extend_from_slice(bytes),
            Self::CapabilityRequest { capability } => payload.push(*capability as u8),
            Self::CapabilityResponse { granted } => payload.push(u8::from(*granted)),
            Self::Completed | Self::Cancel => {}
            Self::Error { code, message } => {
                if message.is_empty() {
                    return Err(ProtocolError::new("IPC error message must not be empty"));
                }
                payload.extend_from_slice(&code.to_le_bytes());
                payload.extend_from_slice(message.as_bytes());
            }
        }
        if payload.len() > MAX_PAYLOAD_BYTES {
            return Err(ProtocolError::new("IPC payload exceeds the frame limit"));
        }
        Ok(payload)
    }

    fn decode_payload(kind: MessageKind, payload: &[u8]) -> Result<Self, ProtocolError> {
        match kind {
            MessageKind::Hello => {
                require_length(payload, 4, "Hello")?;
                let minimum_version = u16::from_le_bytes([payload[0], payload[1]]);
                let maximum_version = u16::from_le_bytes([payload[2], payload[3]]);
                validate_version_range(minimum_version, maximum_version)?;
                Ok(Self::Hello { minimum_version, maximum_version })
            }
            MessageKind::RunSource => {
                if payload.is_empty() {
                    return Err(ProtocolError::new("Source request must not be empty"));
                }
                let source = std::str::from_utf8(payload)
                    .map_err(|_| ProtocolError::new("Source request must be valid UTF-8"))?;
                Ok(Self::RunSource(source.to_owned()))
            }
            MessageKind::RunBytecode => {
                if payload.is_empty() {
                    return Err(ProtocolError::new("Bytecode request must not be empty"));
                }
                Ok(Self::RunBytecode(payload.to_vec()))
            }
            MessageKind::Stdout => Ok(Self::Stdout(payload.to_vec())),
            MessageKind::Stderr => Ok(Self::Stderr(payload.to_vec())),
            MessageKind::CapabilityRequest => {
                require_length(payload, 1, "Capability request")?;
                Ok(Self::CapabilityRequest { capability: CapabilityId::try_from(payload[0])? })
            }
            MessageKind::CapabilityResponse => {
                require_length(payload, 1, "Capability response")?;
                match payload[0] {
                    0 => Ok(Self::CapabilityResponse { granted: false }),
                    1 => Ok(Self::CapabilityResponse { granted: true }),
                    _ => Err(ProtocolError::new("Capability response must be zero or one")),
                }
            }
            MessageKind::Completed => {
                require_length(payload, 0, "Completed")?;
                Ok(Self::Completed)
            }
            MessageKind::Error => {
                if payload.len() < 3 {
                    return Err(ProtocolError::new("IPC error payload requires a code and message"));
                }
                let code = u16::from_le_bytes([payload[0], payload[1]]);
                let message = std::str::from_utf8(&payload[2..])
                    .map_err(|_| ProtocolError::new("IPC error message must be valid UTF-8"))?;
                if message.is_empty() {
                    return Err(ProtocolError::new("IPC error message must not be empty"));
                }
                Ok(Self::Error { code, message: message.to_owned() })
            }
            MessageKind::Cancel => {
                require_length(payload, 0, "Cancel")?;
                Ok(Self::Cancel)
            }
        }
    }
}

fn validate_version_range(minimum: u16, maximum: u16) -> Result<(), ProtocolError> {
    if minimum == 0 || minimum > maximum {
        return Err(ProtocolError::new("Invalid IPC protocol version range"));
    }
    if minimum > PROTOCOL_VERSION || maximum < PROTOCOL_VERSION {
        return Err(ProtocolError::new("No mutually supported IPC protocol version"));
    }
    Ok(())
}

fn require_length(payload: &[u8], expected: usize, label: &str) -> Result<(), ProtocolError> {
    if payload.len() != expected {
        return Err(ProtocolError::new(format!(
            "{label} payload must be exactly {expected} bytes"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub kind: MessageKind,
    pub request_id: u64,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn new(kind: MessageKind, request_id: u64, payload: impl Into<Vec<u8>>) -> Result<Self, ProtocolError> {
        let payload = payload.into();
        if payload.len() > MAX_PAYLOAD_BYTES {
            return Err(ProtocolError::new("IPC payload exceeds the frame limit"));
        }
        Ok(Self { kind, request_id, payload })
    }

    pub fn from_message(request_id: u64, message: &Message) -> Result<Self, ProtocolError> {
        Self::new(message.kind(), request_id, message.encode_payload()?)
    }

    pub fn message(&self) -> Result<Message, ProtocolError> {
        Message::decode_payload(self.kind, &self.payload)
    }

    pub fn decode_message(bytes: &[u8]) -> Result<(Self, Message), ProtocolError> {
        let frame = Self::decode(bytes)?;
        let message = frame.message()?;
        Ok((frame, message))
    }

    pub fn encode(&self) -> Result<Vec<u8>, ProtocolError> {
        if self.payload.len() > MAX_PAYLOAD_BYTES {
            return Err(ProtocolError::new("IPC payload exceeds the frame limit"));
        }
        let payload_length = u32::try_from(self.payload.len())
            .map_err(|_| ProtocolError::new("IPC payload length cannot be represented"))?;
        let mut output = Vec::with_capacity(HEADER_LENGTH + self.payload.len());
        output.extend_from_slice(MAGIC);
        output.extend_from_slice(&PROTOCOL_VERSION.to_le_bytes());
        output.push(self.kind as u8);
        output.push(0);
        output.extend_from_slice(&self.request_id.to_le_bytes());
        output.extend_from_slice(&payload_length.to_le_bytes());
        output.extend_from_slice(&crc32(&self.payload).to_le_bytes());
        output.extend_from_slice(&self.payload);
        Ok(output)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() < HEADER_LENGTH {
            return Err(ProtocolError::new("Truncated IPC frame header"));
        }
        if bytes.len() > HEADER_LENGTH + MAX_PAYLOAD_BYTES {
            return Err(ProtocolError::new("IPC frame exceeds the size limit"));
        }
        if bytes.get(..4) != Some(MAGIC.as_slice()) {
            return Err(ProtocolError::new("Invalid IPC frame magic"));
        }
        let version = u16::from_le_bytes(bytes[4..6].try_into().expect("header version is present"));
        if version != PROTOCOL_VERSION {
            return Err(ProtocolError::new(format!("Unsupported IPC protocol version {version}")));
        }
        let kind = MessageKind::try_from(bytes[6])?;
        if bytes[7] != 0 {
            return Err(ProtocolError::new("Reserved IPC frame flags must be zero"));
        }
        let request_id = u64::from_le_bytes(bytes[8..16].try_into().expect("header request ID is present"));
        let payload_length = u32::from_le_bytes(bytes[16..20].try_into().expect("header payload length is present")) as usize;
        let expected_crc = u32::from_le_bytes(bytes[20..24].try_into().expect("header checksum is present"));
        if payload_length > MAX_PAYLOAD_BYTES || payload_length != bytes.len() - HEADER_LENGTH {
            return Err(ProtocolError::new("IPC payload length does not match the frame"));
        }
        let payload = &bytes[HEADER_LENGTH..];
        if crc32(payload) != expected_crc {
            return Err(ProtocolError::new("IPC payload checksum mismatch"));
        }
        Ok(Self { kind, request_id, payload: payload.to_vec() })
    }
}


/// The peer expected on the other end of an IPC session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointRole {
    Parent,
    Worker,
}

/// Stateful protocol validation for one endpoint.
///
/// The caller must invoke `begin_request` before sending a request from the
/// parent and `complete_request` after the worker sends a terminal response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    role: EndpointRole,
    handshaken: bool,
    active_request: Option<u64>,
    last_request_id: u64,
}

impl Session {
    pub const fn new(role: EndpointRole) -> Self {
        Self {
            role,
            handshaken: false,
            active_request: None,
            last_request_id: 0,
        }
    }

    pub const fn is_handshaken(&self) -> bool {
        self.handshaken
    }

    pub const fn active_request_id(&self) -> Option<u64> {
        self.active_request
    }

    /// Records a parent request before its frame is written to the transport.
    pub fn begin_request(&mut self, request_id: u64) -> Result<(), ProtocolError> {
        if self.role != EndpointRole::Parent {
            return Err(ProtocolError::new("Only the parent may begin a request"));
        }
        if !self.handshaken {
            return Err(ProtocolError::new("IPC handshake is incomplete"));
        }
        if self.active_request.is_some() {
            return Err(ProtocolError::new("An IPC request is already active"));
        }
        if request_id == 0 || request_id <= self.last_request_id {
            return Err(ProtocolError::new("IPC request ID must increase monotonically"));
        }
        self.last_request_id = request_id;
        self.active_request = Some(request_id);
        Ok(())
    }

    /// Validates the next peer frame against handshake and request state.
    pub fn accept(&mut self, frame: &Frame) -> Result<Message, ProtocolError> {
        let message = frame.message()?;
        if !self.handshaken {
            if message.kind() != MessageKind::Hello || frame.request_id != 0 {
                return Err(ProtocolError::new("IPC peer must begin with a request-zero Hello"));
            }
            self.handshaken = true;
            return Ok(message);
        }
        if message.kind() == MessageKind::Hello {
            return Err(ProtocolError::new("Duplicate IPC Hello message"));
        }

        match self.role {
            EndpointRole::Parent => self.accept_worker_message(frame, message),
            EndpointRole::Worker => self.accept_parent_message(frame, message),
        }
    }

    /// Marks a worker request complete after its terminal frame has been sent.
    pub fn complete_request(&mut self, request_id: u64) -> Result<(), ProtocolError> {
        if self.role != EndpointRole::Worker {
            return Err(ProtocolError::new("Only the worker completes an IPC request"));
        }
        self.require_active_request(request_id)?;
        self.active_request = None;
        Ok(())
    }

    fn accept_worker_message(
        &mut self,
        frame: &Frame,
        message: Message,
    ) -> Result<Message, ProtocolError> {
        self.require_active_request(frame.request_id)?;
        if !matches!(
            message,
            Message::Stdout(_) | Message::Stderr(_) | Message::CapabilityRequest { .. }
                | Message::Completed | Message::Error { .. }
        ) {
            return Err(ProtocolError::new("Unexpected worker-to-parent IPC message"));
        }
        if matches!(message, Message::Completed | Message::Error { .. }) {
            self.active_request = None;
        }
        Ok(message)
    }

    fn accept_parent_message(
        &mut self,
        frame: &Frame,
        message: Message,
    ) -> Result<Message, ProtocolError> {
        match message {
            Message::RunSource(_) | Message::RunBytecode(_) => {
                if self.active_request.is_some() {
                    return Err(ProtocolError::new("Worker already has an active IPC request"));
                }
                if frame.request_id == 0 || frame.request_id <= self.last_request_id {
                    return Err(ProtocolError::new("IPC request ID must increase monotonically"));
                }
                self.last_request_id = frame.request_id;
                self.active_request = Some(frame.request_id);
            }
            Message::CapabilityResponse { .. } | Message::Cancel => {
                self.require_active_request(frame.request_id)?;
            }
            _ => return Err(ProtocolError::new("Unexpected parent-to-worker IPC message")),
        }
        Ok(message)
    }

    fn require_active_request(&self, request_id: u64) -> Result<(), ProtocolError> {
        if request_id == 0 || self.active_request != Some(request_id) {
            return Err(ProtocolError::new("IPC message has no matching active request"));
        }
        Ok(())
    }
}


pub fn read_frame<R: std::io::Read>(reader: &mut R) -> Result<Option<Frame>, ProtocolError> {
    use std::io::Read;
    let mut header = [0_u8; HEADER_LENGTH];
    loop {
        match reader.read(&mut header[..1]) {
            Ok(0) => return Ok(None),
            Ok(1) => break,
            Ok(_) => unreachable!("one-byte read buffer"),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(ProtocolError::new(format!("Cannot read IPC header: {error}"))),
        }
    }
    reader.read_exact(&mut header[1..])
        .map_err(|error| ProtocolError::new(format!("Truncated IPC frame header: {error}")))?;
    if &header[..4] != MAGIC {
        return Err(ProtocolError::new("Invalid IPC frame magic"));
    }
    let payload_length = u32::from_le_bytes(header[16..20].try_into().expect("header length checked")) as usize;
    if payload_length > MAX_PAYLOAD_BYTES {
        return Err(ProtocolError::new("IPC frame exceeds the size limit"));
    }
    let mut bytes = Vec::with_capacity(HEADER_LENGTH + payload_length);
    bytes.extend_from_slice(&header);
    bytes.resize(HEADER_LENGTH + payload_length, 0);
    reader.read_exact(&mut bytes[HEADER_LENGTH..])
        .map_err(|error| ProtocolError::new(format!("Truncated IPC frame payload: {error}")))?;
    Frame::decode(&bytes).map(Some)
}

pub fn write_frame<W: std::io::Write>(writer: &mut W, frame: &Frame) -> Result<(), ProtocolError> {
    use std::io::Write;
    let bytes = frame.encode()?;
    writer.write_all(&bytes)
        .and_then(|()| writer.flush())
        .map_err(|error| ProtocolError::new(format!("Cannot write IPC frame: {error}")))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolError {
    pub message: String,
}

impl ProtocolError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ProtocolError {}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn stream_transport_round_trips_consecutive_frames() {
        let first = Frame::from_message(0, &Message::Hello { minimum_version: 1, maximum_version: 1 }).unwrap();
        let second = Frame::from_message(1, &Message::RunSource("print(1);".to_owned())).unwrap();
        let mut stream = Vec::new();
        write_frame(&mut stream, &first).unwrap();
        write_frame(&mut stream, &second).unwrap();
        let mut input = stream.as_slice();
        assert_eq!(read_frame(&mut input).unwrap(), Some(first));
        assert_eq!(read_frame(&mut input).unwrap(), Some(second));
        assert_eq!(read_frame(&mut input).unwrap(), None);
    }

    #[test]
    fn stream_transport_rejects_truncated_header_and_payload() {
        let mut short_header = b"PHIP".to_vec();
        assert!(read_frame(&mut short_header.as_slice()).is_err());
        let frame = Frame::from_message(1, &Message::RunSource("x".to_owned())).unwrap();
        let mut encoded = frame.encode().unwrap();
        encoded.pop();
        assert!(read_frame(&mut encoded.as_slice()).is_err());
    }

    #[test]
    fn stream_transport_rejects_oversized_declared_payload_before_allocation() {
        let mut header = [0_u8; HEADER_LENGTH];
        header[..4].copy_from_slice(MAGIC);
        header[4..6].copy_from_slice(&PROTOCOL_VERSION.to_le_bytes());
        header[6] = MessageKind::Completed as u8;
        header[16..20].copy_from_slice(&((MAX_PAYLOAD_BYTES as u32) + 1).to_le_bytes());
        assert!(read_frame(&mut header.as_slice()).is_err());
    }

    #[test]
    fn typed_messages_round_trip_through_frames() {
        let messages = [
            Message::Hello { minimum_version: 1, maximum_version: 1 },
            Message::RunSource("print(1);".to_owned()),
            Message::RunBytecode(b"PHNB".to_vec()),
            Message::Stdout(b"output".to_vec()),
            Message::Stderr(b"error".to_vec()),
            Message::CapabilityRequest { capability: CapabilityId::FileRead },
            Message::CapabilityResponse { granted: true },
            Message::Completed,
            Message::Error { code: 7, message: "failed".to_owned() },
            Message::Cancel,
        ];
        for (index, message) in messages.into_iter().enumerate() {
            let frame = Frame::from_message(index as u64 + 1, &message).unwrap();
            let encoded = frame.encode().unwrap();
            let (decoded_frame, decoded_message) = Frame::decode_message(&encoded).unwrap();
            assert_eq!(decoded_frame.request_id, index as u64 + 1);
            assert_eq!(decoded_message, message);
        }
    }

    #[test]
    fn rejects_invalid_message_payload_shapes() {
        let cases = [
            (MessageKind::Hello, vec![1, 0]),
            (MessageKind::Hello, vec![2, 0, 1, 0]),
            (MessageKind::RunSource, vec![0xFF]),
            (MessageKind::RunSource, Vec::new()),
            (MessageKind::RunBytecode, Vec::new()),
            (MessageKind::CapabilityRequest, vec![99]),
            (MessageKind::CapabilityResponse, vec![2]),
            (MessageKind::Completed, vec![0]),
            (MessageKind::Cancel, vec![0]),
            (MessageKind::Error, vec![1, 0]),
            (MessageKind::Error, vec![1, 0, 0xFF]),
        ];
        for (kind, payload) in cases {
            let frame = Frame::new(kind, 1, payload).unwrap();
            assert!(frame.message().is_err(), "unexpectedly accepted {kind:?}");
        }
    }

    #[test]
    fn rejects_empty_error_and_source_messages_when_encoding() {
        assert!(Frame::from_message(1, &Message::RunSource(String::new())).is_err());
        assert!(Frame::from_message(
            1,
            &Message::Error { code: 1, message: String::new() }
        ).is_err());
    }

    #[test]
    fn round_trips_raw_frames_for_every_message_kind() {
        for value in 1..=10 {
            let kind = MessageKind::try_from(value).unwrap();
            let frame = Frame::new(kind, u64::from(value), b"payload".to_vec()).unwrap();
            assert_eq!(Frame::decode(&frame.encode().unwrap()).unwrap(), frame);
        }
    }

    #[test]
    fn rejects_bad_magic_version_flags_and_checksum() {
        let frame = Frame::new(MessageKind::Hello, 1, b"hello".to_vec()).unwrap();
        let encoded = frame.encode().unwrap();

        let mut bad_magic = encoded.clone();
        bad_magic[0] = b'X';
        assert!(Frame::decode(&bad_magic).unwrap_err().message.contains("magic"));

        let mut bad_version = encoded.clone();
        bad_version[4] = 99;
        assert!(Frame::decode(&bad_version).unwrap_err().message.contains("version"));

        let mut bad_flags = encoded.clone();
        bad_flags[7] = 1;
        assert!(Frame::decode(&bad_flags).unwrap_err().message.contains("flags"));

        let mut bad_checksum = encoded;
        *bad_checksum.last_mut().unwrap() ^= 0xFF;
        assert!(Frame::decode(&bad_checksum).unwrap_err().message.contains("checksum"));
    }

    #[test]
    fn rejects_truncated_oversized_and_trailing_frames() {
        let frame = Frame::new(MessageKind::Hello, 1, b"hello".to_vec()).unwrap();
        let encoded = frame.encode().unwrap();
        assert!(Frame::decode(&encoded[..encoded.len() - 1]).unwrap_err().message.contains("length"));
        let mut trailing = encoded;
        trailing.push(0);
        assert!(Frame::decode(&trailing).unwrap_err().message.contains("length"));
        assert!(Frame::new(MessageKind::Hello, 1, vec![0; MAX_PAYLOAD_BYTES + 1]).is_err());
    }


    #[test]
    fn parent_session_requires_handshake_and_monotonic_requests() {
        let hello = Frame::from_message(
            0,
            &Message::Hello { minimum_version: 1, maximum_version: 1 },
        ).unwrap();
        let mut session = Session::new(EndpointRole::Parent);
        assert!(session.begin_request(1).is_err());
        assert_eq!(session.accept(&hello).unwrap().kind(), MessageKind::Hello);
        assert!(session.begin_request(1).is_ok());
        assert!(session.begin_request(2).is_err());

        let completed = Frame::from_message(1, &Message::Completed).unwrap();
        assert_eq!(session.accept(&completed).unwrap(), Message::Completed);
        assert!(session.begin_request(1).is_err());
        assert!(session.begin_request(2).is_ok());
    }

    #[test]
    fn parent_session_rejects_unmatched_and_out_of_order_worker_frames() {
        let mut session = Session::new(EndpointRole::Parent);
        let hello = Frame::from_message(0, &Message::Hello {
            minimum_version: 1,
            maximum_version: 1,
        }).unwrap();
        session.accept(&hello).unwrap();

        let unsolicited = Frame::from_message(4, &Message::Stdout(b"x".to_vec())).unwrap();
        assert!(session.accept(&unsolicited).is_err());
        session.begin_request(5).unwrap();
        let wrong_id = Frame::from_message(4, &Message::Completed).unwrap();
        assert!(session.accept(&wrong_id).is_err());
        let duplicate_hello = Frame::from_message(0, &Message::Hello {
            minimum_version: 1,
            maximum_version: 1,
        }).unwrap();
        assert!(session.accept(&duplicate_hello).is_err());
    }

    #[test]
    fn worker_session_tracks_requests_and_rejects_replay() {
        let mut session = Session::new(EndpointRole::Worker);
        let hello = Frame::from_message(0, &Message::Hello {
            minimum_version: 1,
            maximum_version: 1,
        }).unwrap();
        session.accept(&hello).unwrap();

        let run = Frame::from_message(7, &Message::RunSource("print(1);".to_owned())).unwrap();
        assert_eq!(session.accept(&run).unwrap(), Message::RunSource("print(1);".to_owned()));
        let cancel = Frame::from_message(7, &Message::Cancel).unwrap();
        assert_eq!(session.accept(&cancel).unwrap(), Message::Cancel);
        session.complete_request(7).unwrap();
        assert!(session.accept(&run).is_err());

        let next = Frame::from_message(8, &Message::RunBytecode(vec![1, 2, 3])).unwrap();
        assert_eq!(session.accept(&next).unwrap(), Message::RunBytecode(vec![1, 2, 3]));
        let wrong_response = Frame::from_message(7, &Message::CapabilityResponse { granted: true }).unwrap();
        assert!(session.accept(&wrong_response).is_err());
    }

    #[test]
    fn sessions_reject_wrong_direction_and_handshake_order() {
        let mut worker = Session::new(EndpointRole::Worker);
        let run_before_hello = Frame::from_message(1, &Message::RunSource("x".to_owned())).unwrap();
        assert!(worker.accept(&run_before_hello).is_err());

        let hello = Frame::from_message(0, &Message::Hello {
            minimum_version: 1,
            maximum_version: 1,
        }).unwrap();
        worker.accept(&hello).unwrap();
        let output = Frame::from_message(1, &Message::Stdout(vec![])).unwrap();
        assert!(worker.accept(&output).is_err());

        let mut parent = Session::new(EndpointRole::Parent);
        parent.accept(&hello).unwrap();
        let capability_response = Frame::from_message(1, &Message::CapabilityResponse { granted: false }).unwrap();
        assert!(parent.accept(&capability_response).is_err());
    }

    #[test]
    fn rejects_unknown_message_kinds() {
        let frame = Frame::new(MessageKind::Hello, 1, Vec::new()).unwrap();
        let mut encoded = frame.encode().unwrap();
        encoded[6] = 255;
        assert!(Frame::decode(&encoded).unwrap_err().message.contains("Unknown IPC message"));
    }
}
