//! The six-digit pairing number (D88), the way to pair by hand instead of the long code. The number never travels:
//! both devices run SPAKE2 with it, so only one that knows it reaches the same key, and each confirms the other with
//! that key. The device showing the number also confirms the fingerprint of the TLS certificate it serves with, so
//! whoever enters the number knows it talks to the screen it read it from, and pins that certificate from then on.

use hmac::{Hmac, KeyInit, Mac};
use rand::RngExt;
use sha2::Sha256;
use spake2::{Ed25519Group, Identity, Password, Spake2};

/// Tries per pairing window: each one is a whole exchange with the device, never a guess checked at home, so three
/// leave a stranger one chance in about three hundred thousand.
pub const PIN_ATTEMPTS: u8 = 3;

const JOINING: &[u8] = b"kosmotheka/pin/joining";
const SHOWING: &[u8] = b"kosmotheka/pin/showing";

/// A fresh number, from 000000 to 999999.
pub fn new_pin() -> String {
    format!("{:06}", rand::rng().random_range(0..1_000_000u32))
}

/// The joining device's half of the exchange: its message, and the state that turns the answer into the key.
pub fn join(pin: &str) -> (Spake2<Ed25519Group>, Vec<u8>) {
    Spake2::<Ed25519Group>::start_a(
        &Password::new(pin.as_bytes()),
        &Identity::new(JOINING),
        &Identity::new(SHOWING),
    )
}

/// The showing device's answer to `message`: the key and its own message. None when the message is malformed.
pub fn answer(pin: &str, message: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let (state, outbound) = Spake2::<Ed25519Group>::start_b(
        &Password::new(pin.as_bytes()),
        &Identity::new(JOINING),
        &Identity::new(SHOWING),
    );
    Some((state.finish(message).ok()?, outbound))
}

/// What the showing device sends with its answer: proof of the key, bound to the certificate it serves with.
pub fn showing_confirmation(key: &[u8], fingerprint: &str) -> Vec<u8> {
    tag(key, &[b"showing", fingerprint.as_bytes()])
}

/// What the joining device sends back: proof of the key, for this certificate and this device.
pub fn joining_confirmation(key: &[u8], fingerprint: &str, device_id: &str) -> Vec<u8> {
    tag(key, &[b"joining", fingerprint.as_bytes(), device_id.as_bytes()])
}

/// Whether `given` is `expected`, compared without stopping at the first difference, so timing says nothing.
pub fn confirms(expected: &[u8], given: &[u8]) -> bool {
    expected.len() == given.len()
        && expected.iter().zip(given).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn tag(key: &[u8], parts: &[&[u8]]) -> Vec<u8> {
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(key).expect("HMAC takes a key of any length");
    for part in parts {
        mac.update(&(part.len() as u64).to_be_bytes());
        mac.update(part);
    }
    mac.finalize().into_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exchange(joining_pin: &str, showing_pin: &str) -> (Vec<u8>, Vec<u8>) {
        let (state, message) = join(joining_pin);
        let (showing_key, reply) = answer(showing_pin, &message).unwrap();
        (state.finish(&reply).unwrap(), showing_key)
    }

    #[test]
    fn the_same_number_gives_the_same_key() {
        let (joining, showing) = exchange("482913", "482913");

        assert_eq!(joining, showing);
        assert!(confirms(
            &showing_confirmation(&showing, "fp"),
            &showing_confirmation(&joining, "fp")
        ));
    }

    #[test]
    fn another_number_gives_another_key_and_nothing_confirms() {
        let (joining, showing) = exchange("482914", "482913");

        assert_ne!(joining, showing);
        assert!(!confirms(
            &showing_confirmation(&showing, "fp"),
            &showing_confirmation(&joining, "fp")
        ));
        assert!(!confirms(
            &joining_confirmation(&showing, "fp", "phone"),
            &joining_confirmation(&joining, "fp", "phone")
        ));
    }

    #[test]
    fn a_confirmation_holds_for_its_certificate_and_device_only() {
        let (key, _) = exchange("000001", "000001");

        assert!(!confirms(
            &showing_confirmation(&key, "fp"),
            &showing_confirmation(&key, "other-fp")
        ));
        assert!(!confirms(
            &joining_confirmation(&key, "fp", "phone"),
            &joining_confirmation(&key, "fp", "laptop")
        ));
        assert!(!confirms(
            &showing_confirmation(&key, "fp"),
            &joining_confirmation(&key, "fp", "")
        ));
    }

    #[test]
    fn numbers_have_six_digits() {
        for _ in 0..200 {
            let pin = new_pin();
            assert_eq!(pin.len(), 6);
            assert!(pin.chars().all(|c| c.is_ascii_digit()));
        }
    }
}
