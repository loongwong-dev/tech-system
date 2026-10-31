//! 密码校验与 Argon2id 哈希。

use argon2::password_hash::{PasswordHasher, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};

const MIN_PASSWORD_LENGTH: usize = 10;
const MAX_PASSWORD_LENGTH: usize = 128;
const WEAK_PASSWORDS: &[&str] = &[
    "1234567890",
    "123456789012",
    "password123",
    "qwertyuiop",
    "1111111111",
    "letmein123",
    "iloveyou12",
    "admin12345",
    "welcome123",
];

/// 密码校验或哈希错误。
#[derive(Debug, thiserror::Error)]
pub(crate) enum PasswordError {
    /// 密码过短。
    #[error("密码长度不足")]
    TooShort,
    /// 密码过长。
    #[error("密码长度超过上限")]
    TooLong,
    /// 命中弱密码黑名单。
    #[error("密码过于常见")]
    Weak,
    /// Argon2 哈希失败。
    #[error("密码哈希失败")]
    Hash,
}

/// 按认证设计校验密码长度与弱密码黑名单。
pub(crate) fn validate_password(password: &str) -> Result<(), PasswordError> {
    let length = password.len();
    if length < MIN_PASSWORD_LENGTH {
        return Err(PasswordError::TooShort);
    }
    if length > MAX_PASSWORD_LENGTH {
        return Err(PasswordError::TooLong);
    }
    if WEAK_PASSWORDS.contains(&password.to_ascii_lowercase().as_str()) {
        return Err(PasswordError::Weak);
    }
    Ok(())
}

/// 使用 Argon2id（m=19MiB、t=2、p=1）计算 PHC 密码哈希。
///
/// Argon2 是 CPU 密集型操作，放到阻塞线程池执行，避免占用异步运行时。
pub(crate) async fn hash_password(password: String) -> Result<String, PasswordError> {
    tokio::task::spawn_blocking(move || hash_password_blocking(&password))
        .await
        .map_err(|_| PasswordError::Hash)?
}

fn hash_password_blocking(password: &str) -> Result<String, PasswordError> {
    let params = Params::new(19 * 1024, 2, 1, None).map_err(|_| PasswordError::Hash)?;
    let hasher = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let salt_bytes: [u8; 16] = rand::random();
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|_| PasswordError::Hash)?;
    hasher
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| PasswordError::Hash)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use argon2::Argon2;
    use argon2::password_hash::{PasswordHash, PasswordVerifier};

    use super::*;

    #[test]
    fn rejects_short_long_and_weak_passwords() {
        assert!(matches!(
            validate_password("short"),
            Err(PasswordError::TooShort)
        ));
        assert!(matches!(
            validate_password(&"a".repeat(MAX_PASSWORD_LENGTH + 1)),
            Err(PasswordError::TooLong)
        ));
        assert!(matches!(
            validate_password("password123"),
            Err(PasswordError::Weak)
        ));
        assert!(validate_password("a-long-enough-password").is_ok());
    }

    #[tokio::test]
    async fn hashes_with_argon2id_and_verifies() {
        let password = "a-long-enough-password".to_owned();
        let hash = hash_password(password.clone()).await.expect("哈希失败");

        assert!(hash.starts_with("$argon2id$"));
        let parsed = PasswordHash::new(&hash).expect("解析 PHC 失败");
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .expect("密码校验失败");
    }
}
