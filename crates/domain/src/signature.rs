//! Operation signatures (D56): each device signs its operations with Ed25519 and
//! every other device checks them. The signed bytes are the operation without its
//! signature in canonical JSON (RFC 8785), so any implementation can reproduce them.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde_json::Value;

use crate::operation::Operation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SignatureError {
    #[error("the operation is not signed")]
    Missing,
    #[error("the public key is not a valid Ed25519 key")]
    BadKey,
    #[error("the signature is malformed")]
    Malformed,
    #[error("the signature does not match the operation and key")]
    Mismatch,
}

/// This device's signing key. Built from 32 random bytes kept by the app.
pub struct DeviceKey(SigningKey);

impl DeviceKey {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self(SigningKey::from_bytes(&seed))
    }

    /// The public key other devices verify with, as base64url without padding.
    pub fn public_key(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0.verifying_key().as_bytes())
    }

    /// Signs `operation` as it stands; any signature it carries is ignored.
    pub fn sign(&self, operation: &Operation) -> String {
        URL_SAFE_NO_PAD.encode(self.0.sign(&canonical_bytes(operation)).to_bytes())
    }

    /// Signs an arbitrary message (for example a login challenge), not an operation.
    pub fn sign_bytes(&self, message: &[u8]) -> String {
        URL_SAFE_NO_PAD.encode(self.0.sign(message).to_bytes())
    }
}

/// Checks the operation's signature against the author's public key.
pub fn verify(operation: &Operation, public_key: &str) -> Result<(), SignatureError> {
    let signature = operation
        .signature
        .as_deref()
        .ok_or(SignatureError::Missing)?;
    let key: [u8; 32] = URL_SAFE_NO_PAD
        .decode(public_key)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(SignatureError::BadKey)?;
    let key = VerifyingKey::from_bytes(&key).map_err(|_| SignatureError::BadKey)?;
    let signature: [u8; 64] = URL_SAFE_NO_PAD
        .decode(signature)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(SignatureError::Malformed)?;
    key.verify_strict(
        &canonical_bytes(operation),
        &Signature::from_bytes(&signature),
    )
    .map_err(|_| SignatureError::Mismatch)
}

/// The operation without its signature, as canonical JSON (RFC 8785): object keys
/// sorted by UTF-16 code units, no whitespace, and strings escaped the way
/// JavaScript's `JSON.stringify` does. Operations only hold strings, integers,
/// booleans and nulls, so number formatting never needs more than `to_string`.
pub fn canonical_bytes(operation: &Operation) -> Vec<u8> {
    let mut value = serde_json::to_value(operation).expect("an operation always serializes");
    if let Value::Object(map) = &mut value {
        map.remove("signature");
    }
    let mut out = String::new();
    write_canonical(&value, &mut out);
    out.into_bytes()
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => write_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            out.push('{');
            for (i, key) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(key, out);
                out.push(':');
                write_canonical(&map[key], out);
            }
            out.push('}');
        }
    }
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;
    use crate::operation::Change;
    use crate::schema::Entity;

    const CASES: &str = include_str!("../../../spec/conformance/signatures.json");

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Case {
        name: String,
        operation: Operation,
        public_key: String,
        canonical: String,
        expect: Expect,
    }

    #[derive(Deserialize)]
    #[serde(tag = "result", rename_all = "snake_case")]
    enum Expect {
        Valid,
        Invalid { code: String },
    }

    fn code(error: SignatureError) -> &'static str {
        match error {
            SignatureError::Missing => "missing",
            SignatureError::BadKey => "bad_key",
            SignatureError::Malformed => "malformed",
            SignatureError::Mismatch => "mismatch",
        }
    }

    #[test]
    fn conformance_cases() {
        let cases: Vec<Case> = serde_json::from_str(CASES).expect("signature cases must parse");
        assert!(!cases.is_empty());
        for case in cases {
            let canonical = String::from_utf8(canonical_bytes(&case.operation)).unwrap();
            assert_eq!(canonical, case.canonical, "{}: canonical bytes", case.name);
            let result = verify(&case.operation, &case.public_key).map_err(code);
            match case.expect {
                Expect::Valid => assert_eq!(result, Ok(()), "{}", case.name),
                Expect::Invalid { code } => {
                    assert_eq!(result, Err(code.as_str()), "{}", case.name)
                }
            }
        }
    }

    #[test]
    fn sign_bytes_verifies_with_the_public_key() {
        let device = DeviceKey::from_seed([9; 32]);
        let message = b"kosmotheka-login\nchallenge\ndevice-id";
        let signature = device.sign_bytes(message);
        let key: [u8; 32] = URL_SAFE_NO_PAD
            .decode(device.public_key())
            .unwrap()
            .try_into()
            .unwrap();
        let key = VerifyingKey::from_bytes(&key).unwrap();
        let signature_bytes: [u8; 64] = URL_SAFE_NO_PAD
            .decode(signature)
            .unwrap()
            .try_into()
            .unwrap();
        assert!(
            key.verify_strict(message, &Signature::from_bytes(&signature_bytes))
                .is_ok()
        );
    }

    #[test]
    fn a_signature_verifies_with_its_own_key_only() {
        let alice = DeviceKey::from_seed([7; 32]);
        let bob = DeviceKey::from_seed([8; 32]);
        let mut op = Operation {
            id: "01997c7e-0000-7000-8000-000000000001".into(),
            library: "01997c7e-0000-7000-8000-00000000aaaa".into(),
            hlc: "0001727222222222-00000000-01997c7e-0000-7000-8000-0000000000ff"
                .parse()
                .unwrap(),
            intent: "member.add".into(),
            changes: vec![Change::create(
                Entity::Member,
                "01997c7e-0000-7000-8000-000000000002",
                json!({ "name": "Ana" }),
            )],
            previous: None,
            signature: None,
        };
        assert_eq!(
            verify(&op, &alice.public_key()),
            Err(SignatureError::Missing)
        );
        op.signature = Some(alice.sign(&op));
        assert_eq!(verify(&op, &alice.public_key()), Ok(()));
        assert_eq!(
            verify(&op, &bob.public_key()),
            Err(SignatureError::Mismatch)
        );
        // Signing again with the signature in place gives the same bytes: it is left out.
        assert_eq!(alice.sign(&op), op.signature.clone().unwrap());
    }

    #[test]
    fn the_previous_operation_is_covered_by_the_signature() {
        let key = DeviceKey::from_seed([7; 32]);
        let mut op = Operation {
            id: "01997c7e-0000-7000-8000-000000000001".into(),
            library: "01997c7e-0000-7000-8000-00000000aaaa".into(),
            hlc: "0001727222222222-00000000-01997c7e-0000-7000-8000-0000000000ff"
                .parse()
                .unwrap(),
            intent: "member.add".into(),
            changes: vec![Change::create(
                Entity::Member,
                "01997c7e-0000-7000-8000-000000000002",
                json!({ "name": "Ana" }),
            )],
            previous: Some("01997c7e-0000-7000-8000-000000000000".into()),
            signature: None,
        };
        op.signature = Some(key.sign(&op));
        assert_eq!(verify(&op, &key.public_key()), Ok(()));

        // A modified app cannot point a signed operation at another chain.
        op.previous = None;
        assert_eq!(
            verify(&op, &key.public_key()),
            Err(SignatureError::Mismatch)
        );
        op.previous = Some("01997c7e-0000-7000-8000-000000000009".into());
        assert_eq!(
            verify(&op, &key.public_key()),
            Err(SignatureError::Mismatch)
        );
    }

    /// Prints the cases in spec/conformance/signatures.json. Run it by hand only
    /// when the format changes: `cargo test -p kosmotheka-domain -- --ignored --nocapture print_signature_cases`.
    #[test]
    #[ignore]
    fn print_signature_cases() {
        // Test keys only, made from fixed bytes: never use them for anything real.
        let author = DeviceKey::from_seed([1; 32]);
        let stranger = DeviceKey::from_seed([2; 32]);
        let op = |id: &str, fields: Value| Operation {
            id: id.into(),
            library: "01997c7e-0000-7000-8000-00000000aaaa".into(),
            hlc: "0001727222222222-00000003-01997c7e-0000-7000-8000-0000000000ff"
                .parse()
                .unwrap(),
            intent: "publisher.add".into(),
            changes: vec![Change::create(
                Entity::Publisher,
                "01997c7e-0000-7000-8000-000000000061",
                fields,
            )],
            previous: None,
            signature: None,
        };
        let signed = |mut o: Operation, key: &DeviceKey| {
            o.signature = Some(key.sign(&o));
            o
        };
        let plain = signed(
            op(
                "01997c7e-0000-7000-8000-000000000301",
                json!({ "name": "Minotauro" }),
            ),
            &author,
        );
        let tricky = signed(
            op(
                "01997c7e-0000-7000-8000-000000000302",
                json!({ "name": "«El \"mesías\"» de Dune\n\t/ñ €\u{1}" }),
            ),
            &author,
        );
        let mut tampered = plain.clone();
        tampered.changes[0]
            .fields
            .insert("name".into(), json!("Minotauro (2)"));
        let by_stranger = signed(
            op(
                "01997c7e-0000-7000-8000-000000000303",
                json!({ "name": "Minotauro" }),
            ),
            &stranger,
        );
        // Same operation, pointed at another previous after signing.
        let mut rechained = plain.clone();
        rechained.previous = Some("01997c7e-0000-7000-8000-000000000300".into());
        let chained = signed(
            Operation {
                previous: Some("01997c7e-0000-7000-8000-000000000300".into()),
                ..op(
                    "01997c7e-0000-7000-8000-000000000305",
                    json!({ "name": "Minotauro" }),
                )
            },
            &author,
        );
        let mut malformed = plain.clone();
        malformed.signature = Some("not-a-signature".into());
        let unsigned = op(
            "01997c7e-0000-7000-8000-000000000304",
            json!({ "name": "Minotauro" }),
        );
        let case = |name: &str, o: &Operation, key: &str, expect: Value| {
            json!({
                "name": name,
                "operation": o,
                "publicKey": key,
                "canonical": String::from_utf8(canonical_bytes(o)).unwrap(),
                "expect": expect,
            })
        };
        let key = author.public_key();
        let cases = json!([
            case(
                "signed by its author",
                &plain,
                &key,
                json!({ "result": "valid" })
            ),
            case(
                "quotes, escapes and non-ASCII text",
                &tricky,
                &key,
                json!({ "result": "valid" })
            ),
            case(
                "changed after signing",
                &tampered,
                &key,
                json!({ "result": "invalid", "code": "mismatch" })
            ),
            case(
                "signed with a previous operation",
                &chained,
                &key,
                json!({ "result": "valid" })
            ),
            case(
                "previous changed after signing",
                &rechained,
                &key,
                json!({ "result": "invalid", "code": "mismatch" })
            ),
            case(
                "signed by another device",
                &by_stranger,
                &key,
                json!({ "result": "invalid", "code": "mismatch" })
            ),
            case(
                "signature that is not Ed25519",
                &malformed,
                &key,
                json!({ "result": "invalid", "code": "malformed" })
            ),
            case(
                "public key that is not Ed25519",
                &plain,
                "AAAA",
                json!({ "result": "invalid", "code": "bad_key" })
            ),
            case(
                "not signed",
                &unsigned,
                &key,
                json!({ "result": "invalid", "code": "missing" })
            ),
        ]);
        println!("{}", serde_json::to_string_pretty(&cases).unwrap());
    }

    #[test]
    fn the_items_of_an_array_are_separated_by_commas() {
        let op = Operation {
            id: "01997c7e-0000-7000-8000-000000000001".into(),
            library: "01997c7e-0000-7000-8000-00000000aaaa".into(),
            hlc: "0000000000000001-00000000-dev".parse().unwrap(),
            intent: "tag.add".into(),
            changes: vec![
                Change::delete(Entity::Tag, "t1"),
                Change::delete(Entity::Tag, "t2"),
                Change::delete(Entity::Tag, "t3"),
            ],
            previous: None,
            signature: None,
        };
        let change = |id| format!(r#"{{"action":"delete","entity":"tag","id":"{id}"}}"#);
        let expected = format!(
            r#"{{"changes":[{},{},{}],"hlc":"0000000000000001-00000000-dev","id":"01997c7e-0000-7000-8000-000000000001","intent":"tag.add","library":"01997c7e-0000-7000-8000-00000000aaaa","previous":null}}"#,
            change("t1"),
            change("t2"),
            change("t3")
        );
        assert_eq!(String::from_utf8(canonical_bytes(&op)).unwrap(), expected);
    }
}
