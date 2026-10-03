//! The pairing code: what one device shows (as a QR or as text) and the other enters.

use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};

const PREFIX: &str = "KOSMOTHEKA1:";
pub const PAIRING_WINDOW: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingCode {
    pub device_id: String,
    pub name: String,
    /// `ip:port` on each local network interface; the other device tries them in order.
    pub addresses: Vec<String>,
    /// sha256 of the server certificate: the only one the other device will accept.
    pub fingerprint: String,
    /// One-time secret proving the other device saw this code.
    pub token: String,
}

impl PairingCode {
    pub fn encode(&self) -> String {
        let json = serde_json::to_vec(self).expect("a pairing code always serializes");
        format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(json))
    }

    pub fn decode(text: &str) -> Option<Self> {
        let encoded = text.trim().strip_prefix(PREFIX)?;
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(encoded).ok()?).ok()
    }
}

/// An open pairing window on the serving device.
#[derive(Debug, Clone)]
pub struct PairingWindow {
    pub token: String,
    pub expires: Instant,
    /// What this device shares with whoever pairs through this window.
    pub libraries: Vec<String>,
}

impl PairingWindow {
    pub fn open(token: String, libraries: Vec<String>) -> Self {
        Self {
            token,
            expires: Instant::now() + PAIRING_WINDOW,
            libraries,
        }
    }

    pub fn accepts(&self, token: &str) -> bool {
        Instant::now() < self.expires && self.token == token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip_and_reject_garbage() {
        let code = PairingCode {
            device_id: "d".into(),
            name: "Portátil".into(),
            addresses: vec!["192.168.1.10:47300".into()],
            fingerprint: "ab".repeat(32),
            token: "t".into(),
        };
        let text = code.encode();

        assert!(text.starts_with(PREFIX));
        assert_eq!(PairingCode::decode(&format!("  {text}\n")), Some(code));
        assert_eq!(PairingCode::decode("KOSMOTHEKA1:not-base64!"), None);
        assert_eq!(PairingCode::decode("hello"), None);
    }

    #[test]
    fn window_accepts_only_its_token_while_open() {
        let window = PairingWindow::open("secret".into(), Vec::new());
        assert!(window.accepts("secret"));
        assert!(!window.accepts("other"));
        let expired = PairingWindow {
            expires: Instant::now().checked_sub(Duration::from_secs(1)).unwrap(),
            ..window
        };
        assert!(!expired.accepts("secret"));
    }
}
