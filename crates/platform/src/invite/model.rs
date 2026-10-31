//! 邀请码格式、规范化与哈希。

use sha2::{Digest, Sha256};

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const CODE_LENGTH: usize = 10;

/// 生成 10 字符 Crockford Base32 邀请码。
pub(crate) fn generate_code() -> String {
    let bytes: [u8; CODE_LENGTH] = rand::random();
    bytes
        .iter()
        .map(|byte| char::from(ALPHABET[usize::from(byte & 0x1f)]))
        .collect()
}

/// 规范化用户输入：大写、去分隔符、按 Crockford 规则映射易混字符。
///
/// 返回 `None` 表示格式不合法。
pub(crate) fn normalize_code(input: &str) -> Option<String> {
    let mut normalized = String::with_capacity(CODE_LENGTH);
    for character in input.chars() {
        if character == '-' || character.is_whitespace() {
            continue;
        }
        let normalized_character = match character.to_ascii_uppercase() {
            'I' | 'L' => '1',
            'O' => '0',
            other => other,
        };
        if !normalized_character.is_ascii() {
            return None;
        }
        if !ALPHABET.contains(&(normalized_character as u8)) {
            return None;
        }
        normalized.push(normalized_character);
    }

    (normalized.len() == CODE_LENGTH).then_some(normalized)
}

/// 计算邀请码的 SHA-256 哈希。
pub(crate) fn hash_code(code: &str) -> [u8; 32] {
    let digest = Sha256::digest(code.as_bytes());
    digest.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_code_is_crockford_and_ten_characters() {
        let code = generate_code();
        assert_eq!(code.len(), CODE_LENGTH);
        assert!(code.bytes().all(|byte| ALPHABET.contains(&byte)));
    }

    #[test]
    fn normalize_accepts_lowercase_separators_and_confusables() {
        assert_eq!(normalize_code("0123-456789"), Some("0123456789".to_owned()));
        assert_eq!(
            normalize_code("o1il-o1il-o1"),
            Some("0111011101".to_owned())
        );
    }

    #[test]
    fn normalize_rejects_wrong_length_and_unknown_characters() {
        assert_eq!(normalize_code("short"), None);
        assert_eq!(normalize_code("012345678U"), None);
        assert_eq!(normalize_code("Ā123456789"), None);
    }

    #[test]
    fn hash_is_32_bytes() {
        assert_eq!(hash_code("0123456789").len(), 32);
    }
}
