//! HTTP calls to the sync server's account API (docs/architecture/accounts.md).
//! Bodies are `snake_case`, as the server expects: the wire structs below are named
//! field for field after the JSON, so no `serde` rename is needed to read or write
//! them. The UI-facing types in `mod.rs` are camelCase, like every other command
//! output, and are built from these by hand.

use std::time::Duration;

use reqwest::{Method, StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::{Error, Result};

/// This device's token, sent on every authenticated call.
pub struct Auth<'a> {
    pub device_id: &'a str,
    pub token: &'a str,
}

pub fn build_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!(
            "Kosmotheka/",
            env!("CARGO_PKG_VERSION"),
            " (+https://kosmotheka.app)"
        ))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| Error::account("server_unavailable"))
}

/// For a connection that stays open (the live notices): no total timeout, which would
/// cut the stream, only one for connecting.
pub fn build_streaming_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!(
            "Kosmotheka/",
            env!("CARGO_PKG_VERSION"),
            " (+https://kosmotheka.app)"
        ))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| Error::account("server_unavailable"))
}

#[derive(Deserialize)]
struct ErrorBody {
    error: String,
    /// Only `access_withdrawn` carries them: the signed operations that withdrew this device (D98).
    #[serde(default)]
    proofs: Vec<Value>,
}

/// Maps a failed response to the error the UI sees. Pure and unit-tested without
/// any network: the status, the raw `Retry-After` header text (if present and it
/// parsed), and the response body text.
pub fn map_response_error(status: StatusCode, retry_after: Option<&str>, body: &str) -> Error {
    let retry_after = || retry_after.and_then(|value| value.trim().parse::<u64>().ok());
    if let Ok(parsed) = serde_json::from_str::<ErrorBody>(body) {
        if status == StatusCode::FORBIDDEN && parsed.error == "access_withdrawn" {
            // What does not parse as an operation is dropped: it could prove nothing anyway.
            let proofs = parsed
                .proofs
                .into_iter()
                .filter_map(|proof| serde_json::from_value(proof).ok())
                .collect();
            return Error::AccessWithdrawn(proofs);
        }
        return if status == StatusCode::TOO_MANY_REQUESTS {
            Error::Account {
                code: parsed.error,
                retry_after: retry_after(),
            }
        } else {
            Error::account(&parsed.error)
        };
    }
    match status {
        StatusCode::UNPROCESSABLE_ENTITY => Error::account("invalid_input"),
        StatusCode::TOO_MANY_REQUESTS => Error::Account {
            code: "too_many_requests".to_owned(),
            retry_after: retry_after(),
        },
        _ => Error::account("server_unavailable"),
    }
}

pub(crate) async fn send(
    client: &reqwest::Client,
    method: Method,
    url: &str,
    auth: Option<Auth<'_>>,
    body: Option<&Value>,
) -> Result<reqwest::Response> {
    let mut request = client.request(method, url);
    if let Some(auth) = auth {
        request = request
            .header("X-Device-Id", auth.device_id)
            .bearer_auth(auth.token);
    }
    if let Some(body) = body {
        request = request.json(body);
    }
    let response = request
        .send()
        .await
        .map_err(|_| Error::account("server_unavailable"))?;
    if response.status().is_success() {
        return Ok(response);
    }
    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(map_response_error(status, retry_after.as_deref(), &body))
}

pub(crate) async fn send_json<T: for<'de> Deserialize<'de>>(
    client: &reqwest::Client,
    method: Method,
    url: &str,
    auth: Option<Auth<'_>>,
    body: Option<&Value>,
) -> Result<T> {
    let response = send(client, method, url, auth, body).await?;
    response
        .json()
        .await
        .map_err(|_| Error::account("server_unavailable"))
}

pub(crate) async fn send_empty(
    client: &reqwest::Client,
    method: Method,
    url: &str,
    auth: Option<Auth<'_>>,
    body: Option<&Value>,
) -> Result<()> {
    send(client, method, url, auth, body).await?;
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerInfoWire {
    pub name: String,
    pub api_version: i64,
    pub registration: bool,
    pub sign_in_with_google: bool,
    pub two_factor: bool,
    /// Whether the server has the live-notices hub (D75). Older servers omit it.
    #[serde(default)]
    pub events: bool,
    /// Whether the server makes invitations (D43): it needs a base address for their links. Older servers omit it.
    #[serde(default)]
    pub invitations: bool,
    /// What the links of this server's invitations start with (D79), if it says. Older servers omit it.
    #[serde(default)]
    pub invitation_base: Option<String>,
    /// The catalogues the server looks books up in for its accounts (7.8, D83). Older servers omit it.
    #[serde(default, deserialize_with = "strings_only")]
    pub metadata: Vec<String>,
}

/// A list of strings from a server that may send anything: what is not a string is dropped, and so is a value that
/// is not a list at all, so a wrong field here never breaks reading the rest of what the server offers.
fn strings_only<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Vec<String>, D::Error> {
    let value = Value::deserialize(deserializer)?;
    Ok(match value {
        Value::Array(items) => items
            .into_iter()
            .filter_map(|item| match item {
                Value::String(text) => Some(text),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    })
}

pub async fn server_info(client: &reqwest::Client, base: &str) -> Result<ServerInfoWire> {
    send_json(
        client,
        Method::GET,
        &format!("{base}/v1/server"),
        None,
        None,
    )
    .await
}

pub async fn register(
    client: &reqwest::Client,
    base: &str,
    email: &str,
    password: &str,
    name: &str,
    locale: &str,
) -> Result<()> {
    let body = json!({ "email": email, "password": password, "name": name, "locale": locale });
    send_empty(
        client,
        Method::POST,
        &format!("{base}/v1/account/register"),
        None,
        Some(&body),
    )
    .await
}

pub async fn confirm_email(
    client: &reqwest::Client,
    base: &str,
    email: &str,
    code: &str,
) -> Result<()> {
    let body = json!({ "email": email, "code": code });
    send_empty(
        client,
        Method::POST,
        &format!("{base}/v1/account/confirm"),
        None,
        Some(&body),
    )
    .await
}

pub async fn resend_code(client: &reqwest::Client, base: &str, email: &str) -> Result<()> {
    let body = json!({ "email": email, "purpose": "confirm_email" });
    send_empty(
        client,
        Method::POST,
        &format!("{base}/v1/account/code"),
        None,
        Some(&body),
    )
    .await
}

pub async fn forgot_password(client: &reqwest::Client, base: &str, email: &str) -> Result<()> {
    let body = json!({ "email": email });
    send_empty(
        client,
        Method::POST,
        &format!("{base}/v1/account/password/forgot"),
        None,
        Some(&body),
    )
    .await
}

pub async fn reset_password(
    client: &reqwest::Client,
    base: &str,
    email: &str,
    code: &str,
    password: &str,
    auth: Option<Auth<'_>>,
) -> Result<()> {
    let body = json!({ "email": email, "code": code, "password": password });
    send_empty(
        client,
        Method::POST,
        &format!("{base}/v1/account/password/reset"),
        auth,
        Some(&body),
    )
    .await
}

#[derive(Deserialize)]
struct ChallengeWire {
    challenge: String,
}

pub async fn auth_challenge(client: &reqwest::Client, base: &str) -> Result<String> {
    let response: ChallengeWire = send_json(
        client,
        Method::POST,
        &format!("{base}/v1/auth/challenge"),
        None,
        None,
    )
    .await?;
    Ok(response.challenge)
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserWire {
    pub id: String,
    pub email: String,
    pub name: String,
    pub locale: String,
}

#[derive(Deserialize)]
pub struct LoginResponseWire {
    pub token: String,
    pub user: UserWire,
}

#[allow(clippy::too_many_arguments)]
pub async fn login(
    client: &reqwest::Client,
    base: &str,
    email: &str,
    password: &str,
    device_id: &str,
    device_name: &str,
    public_key: &str,
    challenge: &str,
    signature: &str,
    totp_code: Option<&str>,
) -> Result<LoginResponseWire> {
    let mut body = json!({
        "email": email,
        "password": password,
        "device_id": device_id,
        "device_name": device_name,
        "public_key": public_key,
        "challenge": challenge,
        "signature": signature,
    });
    if let Some(code) = totp_code {
        body["totp_code"] = json!(code);
    }
    send_json(
        client,
        Method::POST,
        &format!("{base}/v1/auth/login"),
        None,
        Some(&body),
    )
    .await
}

#[derive(Deserialize)]
pub struct GoogleStartWire {
    pub attempt: String,
    pub url: String,
}

/// Opens an attempt of «Continuar con Google» (D77): the server only learns the hash of the secret the app keeps.
pub async fn google_start(
    client: &reqwest::Client,
    base: &str,
    verifier_hash: &str,
    locale: &str,
) -> Result<GoogleStartWire> {
    let body = json!({ "verifier_hash": verifier_hash, "locale": locale });
    send_json(
        client,
        Method::POST,
        &format!("{base}/v1/auth/google/start"),
        None,
        Some(&body),
    )
    .await
}

/// Exchanges the code Google's way back brought, with the secret that never left, and the proof of this device.
#[allow(clippy::too_many_arguments)]
pub async fn google_finish(
    client: &reqwest::Client,
    base: &str,
    attempt: &str,
    code: &str,
    verifier: &str,
    device_id: &str,
    device_name: &str,
    public_key: &str,
    challenge: &str,
    signature: &str,
    totp_code: Option<&str>,
) -> Result<LoginResponseWire> {
    let mut body = json!({
        "attempt": attempt,
        "code": code,
        "verifier": verifier,
        "device_id": device_id,
        "device_name": device_name,
        "public_key": public_key,
        "challenge": challenge,
        "signature": signature,
    });
    if let Some(code) = totp_code {
        body["totp_code"] = json!(code);
    }
    send_json(
        client,
        Method::POST,
        &format!("{base}/v1/auth/google/finish"),
        None,
        Some(&body),
    )
    .await
}

pub async fn logout(
    client: &reqwest::Client,
    base: &str,
    device_id: &str,
    token: &str,
) -> Result<()> {
    send_empty(
        client,
        Method::POST,
        &format!("{base}/v1/auth/logout"),
        Some(Auth { device_id, token }),
        None,
    )
    .await
}

#[derive(Debug, Clone, Deserialize)]
pub struct AccountDetailsWire {
    pub id: String,
    pub email: String,
    pub name: String,
    pub locale: String,
    pub has_password: bool,
    pub created_at: String,
    pub two_factor_enabled: bool,
    pub backup_codes_left: i64,
}

pub async fn get_account(
    client: &reqwest::Client,
    base: &str,
    device_id: &str,
    token: &str,
) -> Result<AccountDetailsWire> {
    send_json(
        client,
        Method::GET,
        &format!("{base}/v1/account"),
        Some(Auth { device_id, token }),
        None,
    )
    .await
}

pub async fn update_account(
    client: &reqwest::Client,
    base: &str,
    device_id: &str,
    token: &str,
    name: Option<&str>,
    locale: Option<&str>,
) -> Result<AccountDetailsWire> {
    let mut body = json!({});
    if let Some(name) = name {
        body["name"] = json!(name);
    }
    if let Some(locale) = locale {
        body["locale"] = json!(locale);
    }
    send_json(
        client,
        Method::PATCH,
        &format!("{base}/v1/account"),
        Some(Auth { device_id, token }),
        Some(&body),
    )
    .await
}

pub async fn change_password(
    client: &reqwest::Client,
    base: &str,
    device_id: &str,
    token: &str,
    current_password: &str,
    new_password: &str,
) -> Result<()> {
    let body = json!({ "current_password": current_password, "new_password": new_password });
    send_empty(
        client,
        Method::POST,
        &format!("{base}/v1/account/password"),
        Some(Auth { device_id, token }),
        Some(&body),
    )
    .await
}

pub async fn delete_account(
    client: &reqwest::Client,
    base: &str,
    device_id: &str,
    token: &str,
    password: &str,
) -> Result<()> {
    let body = json!({ "password": password });
    send_empty(
        client,
        Method::DELETE,
        &format!("{base}/v1/account"),
        Some(Auth { device_id, token }),
        Some(&body),
    )
    .await
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceWire {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub last_seen_at: Option<String>,
    pub signed_in: bool,
    pub removed: bool,
    pub can_restore: bool,
    pub current: bool,
}

pub async fn list_devices(
    client: &reqwest::Client,
    base: &str,
    device_id: &str,
    token: &str,
) -> Result<Vec<DeviceWire>> {
    send_json(
        client,
        Method::GET,
        &format!("{base}/v1/account/devices"),
        Some(Auth { device_id, token }),
        None,
    )
    .await
}

pub async fn remove_device(
    client: &reqwest::Client,
    base: &str,
    device_id: &str,
    token: &str,
    target: &str,
) -> Result<()> {
    send_empty(
        client,
        Method::DELETE,
        &format!("{base}/v1/account/devices/{target}"),
        Some(Auth { device_id, token }),
        None,
    )
    .await
}

pub async fn restore_device(
    client: &reqwest::Client,
    base: &str,
    device_id: &str,
    token: &str,
    target: &str,
) -> Result<()> {
    send_empty(
        client,
        Method::POST,
        &format!("{base}/v1/account/devices/{target}/restore"),
        Some(Auth { device_id, token }),
        None,
    )
    .await
}

#[derive(Debug, Clone, Deserialize)]
pub struct TwoFactorSetupWire {
    pub secret: String,
    pub otpauth_uri: String,
}

pub async fn two_factor_setup(
    client: &reqwest::Client,
    base: &str,
    device_id: &str,
    token: &str,
) -> Result<TwoFactorSetupWire> {
    send_json(
        client,
        Method::POST,
        &format!("{base}/v1/account/two-factor/setup"),
        Some(Auth { device_id, token }),
        None,
    )
    .await
}

#[derive(Deserialize)]
struct BackupCodesWire {
    backup_codes: Vec<String>,
}

pub async fn two_factor_enable(
    client: &reqwest::Client,
    base: &str,
    device_id: &str,
    token: &str,
    code: &str,
) -> Result<Vec<String>> {
    let body = json!({ "code": code });
    let response: BackupCodesWire = send_json(
        client,
        Method::POST,
        &format!("{base}/v1/account/two-factor/enable"),
        Some(Auth { device_id, token }),
        Some(&body),
    )
    .await?;
    Ok(response.backup_codes)
}

pub async fn two_factor_disable(
    client: &reqwest::Client,
    base: &str,
    device_id: &str,
    token: &str,
    password: &str,
    code: &str,
) -> Result<()> {
    let body = json!({ "password": password, "code": code });
    send_empty(
        client,
        Method::DELETE,
        &format!("{base}/v1/account/two-factor"),
        Some(Auth { device_id, token }),
        Some(&body),
    )
    .await
}

pub async fn regenerate_backup_codes(
    client: &reqwest::Client,
    base: &str,
    device_id: &str,
    token: &str,
    password: &str,
    code: &str,
) -> Result<Vec<String>> {
    let body = json!({ "password": password, "code": code });
    let response: BackupCodesWire = send_json(
        client,
        Method::POST,
        &format!("{base}/v1/account/two-factor/backup-codes"),
        Some(Auth { device_id, token }),
        Some(&body),
    )
    .await?;
    Ok(response.backup_codes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_server_code_is_passed_through() {
        let error = map_response_error(
            StatusCode::UNAUTHORIZED,
            None,
            r#"{"error":"invalid_credentials"}"#,
        );
        assert!(
            matches!(error, Error::Account { code, retry_after: None } if code == "invalid_credentials")
        );
    }

    #[test]
    fn unprocessable_entity_becomes_invalid_input() {
        let error = map_response_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            None,
            r#"{"violations":[]}"#,
        );
        assert!(matches!(error, Error::Account { code, .. } if code == "invalid_input"));
    }

    #[test]
    fn too_many_requests_carries_the_retry_after_header() {
        let error = map_response_error(
            StatusCode::TOO_MANY_REQUESTS,
            Some("42"),
            r#"{"error":"too_many_requests"}"#,
        );
        assert!(matches!(
            error,
            Error::Account { code, retry_after: Some(42) } if code == "too_many_requests"
        ));
    }

    #[test]
    fn too_many_requests_with_an_unparsable_header_has_no_retry_after() {
        let error = map_response_error(StatusCode::TOO_MANY_REQUESTS, Some("soon"), "not json");
        assert!(matches!(
            error,
            Error::Account { code, retry_after: None } if code == "too_many_requests"
        ));
    }

    #[test]
    fn a_server_error_with_no_json_body_is_unavailable() {
        let error =
            map_response_error(StatusCode::INTERNAL_SERVER_ERROR, None, "<html>oops</html>");
        assert!(matches!(error, Error::Account { code, .. } if code == "server_unavailable"));
    }

    #[test]
    fn a_non_json_body_elsewhere_is_also_unavailable() {
        let error = map_response_error(StatusCode::BAD_GATEWAY, None, "");
        assert!(matches!(error, Error::Account { code, .. } if code == "server_unavailable"));
    }
}
