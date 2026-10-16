//! A library is founded by one key (D97, rule 1 of `spec/README.md`): its id derives from the public key of
//! the device that founded it and a nonce, so only whoever holds that key can found it, whatever date they put
//! on the operation. The same derivation as the cloud device's id (D95, rule 12), with a nonce.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

/// The 16 random bytes that go with the key, as the `founding_nonce` field writes them.
pub const NONCE_BYTES: usize = 16;

/// SHA-256 of the 32 bytes of the public key followed by the 16 bytes of the nonce, the first 16 bytes with the
/// version nibble set to 8 and the variant bits to 10, as a lowercase UUID. `None` when `public_key` is not 32
/// bytes or `nonce` is not 16 (both base64url, no padding).
pub fn library_id(public_key: &str, nonce: &str) -> Option<String> {
    let key = URL_SAFE_NO_PAD.decode(public_key).ok()?;
    let nonce = URL_SAFE_NO_PAD.decode(nonce).ok()?;
    if key.len() != 32 || nonce.len() != NONCE_BYTES {
        return None;
    }
    let mut hash = Sha256::new();
    hash.update(&key);
    hash.update(&nonce);
    let digest = hash.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Some(uuid::Uuid::from_bytes(bytes).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w";
    const NONCE: &str = "AAECAwQFBgcICQoLDA0ODw";

    #[test]
    fn the_id_comes_from_the_key_and_the_nonce() {
        // The same vector as in the PHP test and in spec/README.md.
        let id = library_id(KEY, NONCE).unwrap();
        assert_eq!(id, "2c33de53-4834-8a05-b014-c42dc60bd4e1");
        assert_eq!(library_id(KEY, NONCE), library_id(KEY, NONCE));
        assert_ne!(
            library_id(KEY, NONCE),
            library_id(KEY, "AAECAwQFBgcICQoLDA0ODg")
        );
        assert_ne!(
            library_id("jojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w", NONCE),
            library_id(KEY, NONCE)
        );
    }

    #[test]
    fn what_is_not_a_key_and_a_nonce_gives_no_id() {
        assert_eq!(library_id("short", NONCE), None);
        assert_eq!(library_id(KEY, "short"), None);
        assert_eq!(library_id(KEY, ""), None);
        assert_eq!(library_id("not base64 !", NONCE), None);
        // 17 bytes.
        assert_eq!(library_id(KEY, "AAECAwQFBgcICQoLDA0ODxA"), None);
    }
}
