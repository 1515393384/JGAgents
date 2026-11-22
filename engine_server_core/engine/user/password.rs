use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use crate::core::error::EngineError;
use crate::core::ids::{decode_hex, encode_hex};

type HmacSha256 = Hmac<Sha256>;

const ALGORITHM: &str = "pbkdf2_sha256";
const DEFAULT_ITERATIONS: u32 = 120_000;
const MIN_ITERATIONS: u32 = 10_000;
const MAX_ITERATIONS: u32 = 2_000_000;
const SALT_BYTES: usize = 16;
const DK_BYTES: usize = 32;

pub fn hash_password(password: &str) -> Result<String, EngineError> {
    let mut salt = [0u8; SALT_BYTES];
    getrandom::fill(&mut salt).map_err(|e| {
        tracing::error!(error = %e, "csprng failed generating password salt");
        EngineError::internal("failed to process password")
    })?;

    let dk = pbkdf2_hmac_sha256(password.as_bytes(), &salt, DEFAULT_ITERATIONS);
    Ok(format!(
        "{ALGORITHM}${DEFAULT_ITERATIONS}${}${}",
        encode_hex(&salt),
        encode_hex(&dk)
    ))
}

pub fn verify_password(password: &str, stored: &str) -> bool {
    if stored.is_empty() {
        return false;
    }

    let mut parts = stored.split('$');
    let Some(alg) = parts.next() else {
        return false;
    };
    let Some(iter_raw) = parts.next() else {
        return false;
    };
    let Some(salt_hex) = parts.next() else {
        return false;
    };
    let Some(dk_hex) = parts.next() else {
        return false;
    };
    if parts.next().is_some() || alg != ALGORITHM {
        return false;
    }

    let Ok(iterations) = iter_raw.parse::<u32>() else {
        return false;
    };
    if !(MIN_ITERATIONS..=MAX_ITERATIONS).contains(&iterations) {
        return false;
    }

    let Some(salt) = decode_hex(salt_hex) else {
        return false;
    };
    let Some(expected_dk) = decode_hex(dk_hex) else {
        return false;
    };
    if salt.len() < 8 || expected_dk.len() != DK_BYTES {
        return false;
    }

    let computed = pbkdf2_hmac_sha256(password.as_bytes(), &salt, iterations);
    constant_time_eq(&computed, &expected_dk)
}

fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32) -> [u8; DK_BYTES] {
    let mut mac = HmacSha256::new_from_slice(password)
        .expect("HMAC-SHA256 accepts keys of any length");
    mac.update(salt);
    mac.update(&1u32.to_be_bytes());
    let mut u = mac.finalize().into_bytes();

    let mut out = [0u8; DK_BYTES];
    out.copy_from_slice(&u);

    for _ in 1..iterations {
        let mut next_mac = HmacSha256::new_from_slice(password)
            .expect("HMAC-SHA256 accepts keys of any length");
        next_mac.update(&u);
        u = next_mac.finalize().into_bytes();
        for (dst, src) in out.iter_mut().zip(u.iter()) {
            *dst ^= *src;
        }
    }

    out
}

pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::{hash_password, verify_password};

    #[test]
    fn pbkdf2_hash_and_verify_work_with_salts() {
        let h1 = hash_password("correct-horse-battery-staple").expect("hash 1");
        let h2 = hash_password("correct-horse-battery-staple").expect("hash 2");
        assert_ne!(h1, h2, "unique random salt per hash");
        assert!(verify_password("correct-horse-battery-staple", &h1));
        assert!(verify_password("correct-horse-battery-staple", &h2));
        assert!(!verify_password("wrong-password-123456", &h1));
        assert!(!verify_password("", ""));
        assert!(!verify_password("anything", "malformed$hash"));
    }
}
