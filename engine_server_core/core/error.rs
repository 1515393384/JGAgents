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

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{}: {message}", self.as_str())]
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

    pub fn bad_frame(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::BadFrame, message)
    }

    pub fn unsupported_version(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::UnsupportedVersion, message)
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

    pub fn canceled(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Canceled, message)
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
