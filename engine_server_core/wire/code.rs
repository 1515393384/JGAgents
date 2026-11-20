use crate::core::error::EngineError;

pub const WIRE_MAGIC: [u8; 2] = [0x4A, 0x47];
pub const WIRE_VERSION_1: u8 = 1;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameKind {
    Hello = 1,
    Open = 2,
    Delta = 3,
    Commit = 4,
    Control = 5,
    Pong = 6,
    Error = 7,
}

impl FrameKind {
    pub fn from_byte(raw: u8) -> Result<Self, EngineError> {
        match raw {
            1 => Ok(Self::Hello),
            2 => Ok(Self::Open),
            3 => Ok(Self::Delta),
            4 => Ok(Self::Commit),
            5 => Ok(Self::Control),
            6 => Ok(Self::Pong),
            7 => Ok(Self::Error),
            _ => Err(EngineError::bad_frame("unknown frame kind")),
        }
    }

    pub fn to_byte(self) -> u8 {
        self as u8
    }
}

pub fn validate_version(ver: u8) -> Result<(), EngineError> {
    if ver != WIRE_VERSION_1 {
        return Err(EngineError::unsupported_version("unsupported wire version"));
    }
    Ok(())
}
