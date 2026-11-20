use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    BadFrame,
    UnsupportedVersion,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    InvalidInput,
    Budget,
    Canceled,
    Internal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineError {
    pub code: ErrorCode,
    pub message: String,
}

impl EngineError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Conflict, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unauthorized, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Forbidden, message)
    }

    pub fn budget(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Budget, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, message)
    }

    pub fn http_status(&self) -> u16 {
        match self.code {
            ErrorCode::BadFrame | ErrorCode::InvalidInput => 400,
            ErrorCode::UnsupportedVersion => 426,
            ErrorCode::Unauthorized => 401,
            ErrorCode::Forbidden => 403,
            ErrorCode::NotFound => 404,
            ErrorCode::Conflict => 409,
            ErrorCode::Budget => 429,
            ErrorCode::Canceled => 499,
            ErrorCode::Internal => 500,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self.code {
            ErrorCode::BadFrame => "BadFrame",
            ErrorCode::UnsupportedVersion => "UnsupportedVersion",
            ErrorCode::Unauthorized => "Unauthorized",
            ErrorCode::Forbidden => "Forbidden",
            ErrorCode::NotFound => "NotFound",
            ErrorCode::Conflict => "Conflict",
            ErrorCode::InvalidInput => "InvalidInput",
            ErrorCode::Budget => "Budget",
            ErrorCode::Canceled => "Canceled",
            ErrorCode::Internal => "Internal",
        }
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.as_str(), self.message)
    }
}

impl std::error::Error for EngineError {}
