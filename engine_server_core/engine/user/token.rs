use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use crate::core::error::EngineError;
use crate::core::ids::{decode_hex, encode_hex, TokenHash};

type HmacSha256 = Hmac<Sha256>;

const PEPPER_BYTES: usize = 32;
const TOKEN_BYTES: usize = 32;

#[derive(Clone)]
pub struct TokenPepper([u8; PEPPER_BYTES]);

impl TokenPepper {
    pub fn ephemeral() -> Result<Self, EngineError> {
        let mut key = [0u8; PEPPER_BYTES];
        getrandom::fill(&mut key).map_err(|e| {
            tracing::error!(error = %e, "csprng failed creating pepper");
            EngineError::internal("failed to initialize token key")
        })?;
        Ok(Self(key))
    }

    pub fn from_bytes(bytes: [u8; PEPPER_BYTES]) -> Self {
        Self(bytes)
    }

    pub fn load_or_init(path: &Path) -> Result<Self, EngineError> {
        if path.exists() {
            let raw = fs::read_to_string(path).map_err(|e| {
                tracing::error!(path = %path.display(), error = %e, "failed reading token pepper");
                EngineError::internal("failed to load token key")
            })?;
            let decoded = decode_hex(&raw).ok_or_else(|| {
                tracing::error!(path = %path.display(), "invalid hex in token pepper file");
                EngineError::internal("invalid token key format")
            })?;
            if decoded.len() != PEPPER_BYTES {
                tracing::error!(
                    path = %path.display(),
                    len = decoded.len(),
                    "unexpected token pepper length"
                );
                return Err(EngineError::internal("invalid token key length"));
            }
            let mut key = [0u8; PEPPER_BYTES];
            key.copy_from_slice(&decoded);
            return Ok(Self(key));
        }

        let pepper = Self::ephemeral()?;
        let hex = encode_hex(&pepper.0);
        write_restricted_file(path, hex.as_bytes())?;
        Ok(pepper)
    }

    pub fn hash_token(&self, plain: &str) -> TokenHash {
        let mut mac = HmacSha256::new_from_slice(&self.0)
            .expect("HMAC-SHA256 accepts fixed 32-byte key");
        mac.update(plain.as_bytes());
        let tag = mac.finalize().into_bytes();
        TokenHash::from_hex(encode_hex(&tag))
    }

    pub fn hash_session_token(&self, plain: &str) -> TokenHash {
        let mut mac = HmacSha256::new_from_slice(&self.0)
            .expect("HMAC-SHA256 accepts fixed 32-byte key");
        mac.update(b"session:");
        mac.update(plain.as_bytes());
        let tag = mac.finalize().into_bytes();
        TokenHash::from_hex(encode_hex(&tag))
    }
}

pub fn generate_token() -> Result<(String, String), EngineError> {
    let mut raw = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut raw).map_err(|e| {
        tracing::error!(error = %e, "csprng failed generating bearer token");
        EngineError::internal("failed to generate access token")
    })?;
    let hex = encode_hex(&raw);
    let plain = format!("jg_{hex}");
    let hint = format!("jg_{}", &hex[..4]);
    Ok((plain, hint))
}

pub fn generate_session_token() -> Result<String, EngineError> {
    let mut raw = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut raw).map_err(|e| {
        tracing::error!(error = %e, "csprng failed generating session token");
        EngineError::internal("failed to generate session token")
    })?;
    Ok(format!("sess_{}", encode_hex(&raw)))
}

pub fn write_restricted_file(path: &Path, content: &[u8]) -> Result<(), EngineError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                tracing::error!(dir = %parent.display(), error = %e, "failed creating parent dir");
                EngineError::internal("failed to prepare credential directory")
            })?;
        }
    }

    let mut opts = OpenOptions::new();
    opts.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }

    let mut file = opts.open(path).map_err(|e| {
        tracing::error!(path = %path.display(), error = %e, "failed opening credential file");
        EngineError::internal("failed to write credential file")
    })?;

    file.write_all(content).map_err(|e| {
        tracing::error!(path = %path.display(), error = %e, "failed writing credential file");
        EngineError::internal("failed to write credential file")
    })?;
    file.sync_all().map_err(|e| {
        tracing::error!(path = %path.display(), error = %e, "failed syncing credential file");
        EngineError::internal("failed to flush credential file")
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| {
            tracing::error!(path = %path.display(), error = %e, "failed setting 0600 permissions");
            EngineError::internal("failed to restrict credential file permissions")
        })?;
    }

    Ok(())
}
