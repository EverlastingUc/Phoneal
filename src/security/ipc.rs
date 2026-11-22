//! Bounded, versioned IPC frame primitives for the future sandbox broker.
//!
//! This module encodes and decodes frames only. It does not create a process,
//! open a pipe/socket, authorize capabilities, or validate message-specific payloads.

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
        output.push(0); // Reserved flags; version 1 requires zero.
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
        Ok(Self {
            kind,
            request_id,
            payload: payload.to_vec(),
        })
    }
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
    fn round_trips_every_message_kind() {
        let kinds = [
            MessageKind::Hello,
            MessageKind::RunSource,
            MessageKind::RunBytecode,
            MessageKind::Stdout,
            MessageKind::Stderr,
            MessageKind::CapabilityRequest,
            MessageKind::CapabilityResponse,
            MessageKind::Completed,
            MessageKind::Error,
            MessageKind::Cancel,
        ];
        for (index, kind) in kinds.into_iter().enumerate() {
            let frame = Frame::new(kind, index as u64 + 1, b"payload".to_vec()).unwrap();
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
    fn rejects_unknown_message_kinds() {
        let frame = Frame::new(MessageKind::Hello, 1, Vec::new()).unwrap();
        let mut encoded = frame.encode().unwrap();
        encoded[6] = 255;
        assert!(Frame::decode(&encoded).unwrap_err().message.contains("Unknown IPC message"));
    }
}
