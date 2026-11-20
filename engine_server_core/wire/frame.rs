use crate::core::error::EngineError;
use crate::wire::code::{validate_version, FrameKind, WIRE_MAGIC, WIRE_VERSION_1};

pub const HEADER_LEN: usize = 18;
pub const MAX_PAYLOAD_LEN: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameHeader {
    pub version: u8,
    pub kind: FrameKind,
    pub flags: u16,
    pub stream_id: u64,
    pub payload_len: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub header: FrameHeader,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn new(kind: FrameKind, stream_id: u64, flags: u16, payload: Vec<u8>) -> Result<Self, EngineError> {
        if payload.len() > MAX_PAYLOAD_LEN {
            return Err(EngineError::bad_frame("frame payload exceeds max size"));
        }
        Ok(Self {
            header: FrameHeader {
                version: WIRE_VERSION_1,
                kind,
                flags,
                stream_id,
                payload_len: payload.len() as u32,
            },
            payload,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, EngineError> {
        validate_version(self.header.version)?;
        if self.payload.len() > MAX_PAYLOAD_LEN || self.payload.len() != self.header.payload_len as usize {
            return Err(EngineError::bad_frame("invalid frame payload length"));
        }

        let mut buf = Vec::with_capacity(HEADER_LEN + self.payload.len());
        buf.extend_from_slice(&WIRE_MAGIC);
        buf.push(self.header.version);
        buf.push(self.header.kind.to_byte());
        buf.extend_from_slice(&self.header.flags.to_be_bytes());
        buf.extend_from_slice(&self.header.stream_id.to_be_bytes());
        buf.extend_from_slice(&self.header.payload_len.to_be_bytes());
        buf.extend_from_slice(&self.payload);
        Ok(buf)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, EngineError> {
        if bytes.len() < HEADER_LEN {
            return Err(EngineError::bad_frame("frame shorter than header"));
        }
        if bytes[0..2] != WIRE_MAGIC {
            return Err(EngineError::bad_frame("invalid wire magic"));
        }
        let version = bytes[2];
        validate_version(version)?;
        let kind = FrameKind::from_byte(bytes[3])?;
        let flags = u16::from_be_bytes([bytes[4], bytes[5]]);
        let stream_id = u64::from_be_bytes([
            bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13],
        ]);
        let payload_len = u32::from_be_bytes([bytes[14], bytes[15], bytes[16], bytes[17]]);
        let payload_usize = payload_len as usize;
        if payload_usize > MAX_PAYLOAD_LEN {
            return Err(EngineError::bad_frame("frame payload exceeds limit"));
        }
        if bytes.len() != HEADER_LEN + payload_usize {
            return Err(EngineError::bad_frame("frame length mismatch"));
        }

        Ok(Self {
            header: FrameHeader {
                version,
                kind,
                flags,
                stream_id,
                payload_len,
            },
            payload: bytes[HEADER_LEN..].to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::ErrorCode;

    #[test]
    fn frame_roundtrip_and_validation() {
        let frame = Frame::new(FrameKind::Delta, 42, 0x0001, b"token-chunk".to_vec())
            .expect("create frame");
        let encoded = frame.encode().expect("encode");
        let decoded = Frame::decode(&encoded).expect("decode");
        assert_eq!(decoded, frame);

        let mut bad_magic = encoded.clone();
        bad_magic[0] = 0x00;
        assert_eq!(
            Frame::decode(&bad_magic).expect_err("bad magic").code,
            ErrorCode::BadFrame
        );

        let mut bad_ver = encoded.clone();
        bad_ver[2] = 99;
        assert_eq!(
            Frame::decode(&bad_ver).expect_err("bad ver").code,
            ErrorCode::UnsupportedVersion
        );
    }
}
