//! The id of a library's cloud device derives from its public key (D95, rule 12 of
//! `spec/README.md`), so the id an administrator writes in `library.cloud_device_id` is tied
//! to a key that nobody can forge: only whoever holds the key can sign as that id.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

/// SHA-256 of the 32 bytes of the public key, the first 16 bytes with the version nibble set to 8 and
/// the variant bits to 10, as a lowercase UUID. `None` when `public_key` (base64url, no padding) is not
/// 32 bytes.
pub fn cloud_device_id(public_key: &str) -> Option<String> {
    let key = URL_SAFE_NO_PAD.decode(public_key).ok()?;
    if key.len() != 32 {
        return None;
    }
    let digest = Sha256::digest(&key);
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Some(uuid::Uuid::from_bytes(bytes).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_id_comes_from_the_key() {
        // The same vector as in the PHP test and in spec/README.md.
        assert_eq!(
            cloud_device_id("iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w").as_deref(),
            Some("34750f98-bd59-8cfc-946d-a45aaabe933b")
        );
        assert_eq!(cloud_device_id("short"), None);
        assert_eq!(cloud_device_id("not base64 !"), None);
    }
}
