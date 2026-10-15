//! What a device signs to sign in (D100): the origin of the server it is signing in to and what the signature is for,
//! besides the challenge and its own id. A signature made for one server, or for signing in with Google, is worth
//! nothing at another or for the other. `server/src/Security/DeviceProof.php` builds the same text.

use url::Url;

use crate::error::{Error, Result};

/// What the signature is for. Part of what is signed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofPurpose {
    Login,
    Google,
}

impl ProofPurpose {
    fn as_str(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Google => "google",
        }
    }
}

/// `scheme://host[:port]` of a server address, in lower case, with no path and the port only when it is not the
/// scheme's own: the one text the server also makes of its own address.
pub fn server_origin(base: &str) -> Result<String> {
    let url = Url::parse(base).map_err(|_| Error::account("invalid_server"))?;
    if !url.origin().is_tuple() {
        return Err(Error::account("invalid_server"));
    }
    Ok(url.origin().ascii_serialization())
}

/// The bytes of the proof, for a server at `base`.
pub fn proof_message(
    base: &str,
    purpose: ProofPurpose,
    challenge: &str,
    device_id: &str,
) -> Result<String> {
    Ok(format!(
        "kosmotheka-login-v2\n{}\n{}\n{challenge}\n{device_id}",
        server_origin(base)?,
        purpose.as_str(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_origin_is_written_one_way() {
        for (base, origin) in [
            ("https://api.kosmotheka.app", "https://api.kosmotheka.app"),
            ("https://API.Kosmotheka.app/", "https://api.kosmotheka.app"),
            (
                "https://api.kosmotheka.app:443",
                "https://api.kosmotheka.app",
            ),
            ("http://localhost:80", "http://localhost"),
            ("http://localhost:8095", "http://localhost:8095"),
            ("https://localhost:80", "https://localhost:80"),
        ] {
            assert_eq!(server_origin(base).unwrap(), origin, "{base}");
        }
    }

    /// The same vector the server's test holds (`DeviceProofTest`): if one changes, so must the other.
    #[test]
    fn the_message_names_the_server_and_the_purpose() {
        assert_eq!(
            proof_message(
                "https://api.kosmotheka.app/",
                ProofPurpose::Login,
                "a-challenge",
                "a-device"
            )
            .unwrap(),
            "kosmotheka-login-v2\nhttps://api.kosmotheka.app\nlogin\na-challenge\na-device"
        );
        assert_eq!(
            proof_message(
                "https://api.kosmotheka.app",
                ProofPurpose::Google,
                "a-challenge",
                "a-device"
            )
            .unwrap(),
            "kosmotheka-login-v2\nhttps://api.kosmotheka.app\ngoogle\na-challenge\na-device"
        );
    }

    #[test]
    fn a_signature_for_another_server_or_purpose_is_another_text() {
        let here = proof_message("https://a.example", ProofPurpose::Login, "c", "d").unwrap();
        assert_ne!(
            here,
            proof_message("https://b.example", ProofPurpose::Login, "c", "d").unwrap()
        );
        assert_ne!(
            here,
            proof_message("https://a.example", ProofPurpose::Google, "c", "d").unwrap()
        );
    }

    #[test]
    fn what_is_not_a_server_address_has_no_origin() {
        assert!(server_origin("not a url").is_err());
        assert!(server_origin("data:text/plain,hi").is_err());
    }
}
