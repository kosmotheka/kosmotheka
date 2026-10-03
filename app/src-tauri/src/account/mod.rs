//! The account client (D77, D81, hito 7.1): talks to the sync server's account
//! API. Full contract in `docs/architecture/accounts.md`.
//!
//! State kept in `local_state` (never synced, never in backups, never logged):
//! which server, the device token, a cache of the signed-in user, the last email
//! used to sign in (to prefill the form after signing out) and a notice the UI
//! shows once for something learnt in the background (`device_removed` or
//! `signed_out_password_changed`).

pub mod client;

use std::net::Ipv4Addr;
use std::path::Path;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::covers::Covers;
use crate::db::Database;
use crate::device_reset::DataDir;
use crate::error::{Error, Result};
use crate::lan::service::LanService;
use crate::ops::{self, read_state, write_state};
use crate::{device_reset, lan};

pub const OFFICIAL_SERVER: &str = "https://api.kosmotheka.app";

const ACCOUNT_SERVER: &str = "account_server";
const ACCOUNT_TOKEN: &str = "account_token";
const ACCOUNT_USER: &str = "account_user";
const ACCOUNT_EMAIL: &str = "account_email";
const ACCOUNT_NOTICE: &str = "account_notice";

/// Everything a device reset needs besides the database, bundled so operation
/// functions take one argument instead of three. Cheap to copy: it only holds
/// references.
#[derive(Clone, Copy)]
pub struct ResetContext<'a> {
    pub covers: &'a Covers,
    pub lan: &'a LanService,
    pub data_dir: &'a Path,
}

/// The same three things as Tauri state, bundled so commands take one state
/// argument instead of three (clippy caps a function at seven).
#[derive(Clone)]
pub struct ResetDeps {
    pub covers: Covers,
    pub lan: LanService,
    pub data_dir: DataDir,
}

impl ResetDeps {
    pub fn new(covers: Covers, lan: LanService, data_dir: DataDir) -> Self {
        Self {
            covers,
            lan,
            data_dir,
        }
    }

    pub fn context(&self) -> ResetContext<'_> {
        ResetContext {
            covers: &self.covers,
            lan: &self.lan,
            data_dir: &self.data_dir.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountUser {
    pub id: String,
    pub email: String,
    pub name: String,
    pub locale: String,
}

impl From<client::UserWire> for AccountUser {
    fn from(wire: client::UserWire) -> Self {
        Self {
            id: wire.id,
            email: wire.email,
            name: wire.name,
            locale: wire.locale,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatus {
    pub server: String,
    pub official: bool,
    pub signed_in: bool,
    pub user: Option<AccountUser>,
    pub email_hint: Option<String>,
    pub notice: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub name: String,
    pub api_version: i64,
    pub registration: bool,
    pub sign_in_with_google: bool,
    pub two_factor: bool,
}

impl From<client::ServerInfoWire> for ServerInfo {
    fn from(wire: client::ServerInfoWire) -> Self {
        Self {
            name: wire.name,
            api_version: wire.api_version,
            registration: wire.registration,
            sign_in_with_google: wire.sign_in_with_google,
            two_factor: wire.two_factor,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDetails {
    pub id: String,
    pub email: String,
    pub name: String,
    pub locale: String,
    pub has_password: bool,
    pub created_at: String,
    pub two_factor_enabled: bool,
    pub backup_codes_left: i64,
}

impl From<client::AccountDetailsWire> for AccountDetails {
    fn from(wire: client::AccountDetailsWire) -> Self {
        Self {
            id: wire.id,
            email: wire.email,
            name: wire.name,
            locale: wire.locale,
            has_password: wire.has_password,
            created_at: wire.created_at,
            two_factor_enabled: wire.two_factor_enabled,
            backup_codes_left: wire.backup_codes_left,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDevice {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub last_seen_at: Option<String>,
    pub signed_in: bool,
    pub removed: bool,
    pub can_restore: bool,
    pub current: bool,
}

impl From<client::DeviceWire> for AccountDevice {
    fn from(wire: client::DeviceWire) -> Self {
        Self {
            id: wire.id,
            name: wire.name,
            created_at: wire.created_at,
            last_seen_at: wire.last_seen_at,
            signed_in: wire.signed_in,
            removed: wire.removed,
            can_restore: wire.can_restore,
            current: wire.current,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorSetup {
    pub secret: String,
    pub otpauth_uri: String,
}

impl From<client::TwoFactorSetupWire> for TwoFactorSetup {
    fn from(wire: client::TwoFactorSetupWire) -> Self {
        Self {
            secret: wire.secret,
            otpauth_uri: wire.otpauth_uri,
        }
    }
}

// --- Local state ---------------------------------------------------------

pub(crate) fn server(conn: &Connection) -> Result<String> {
    Ok(read_state(conn, ACCOUNT_SERVER)?.unwrap_or_else(|| OFFICIAL_SERVER.to_owned()))
}

fn cached_user(conn: &Connection) -> Result<Option<AccountUser>> {
    Ok(read_state(conn, ACCOUNT_USER)?.and_then(|raw| serde_json::from_str(&raw).ok()))
}

pub(crate) fn is_signed_in(conn: &Connection) -> Result<bool> {
    Ok(read_state(conn, ACCOUNT_TOKEN)?.is_some())
}

fn store_session(conn: &Connection, token: &str, user: &AccountUser, email: &str) -> Result<()> {
    write_state(conn, ACCOUNT_TOKEN, Some(token))?;
    write_state(conn, ACCOUNT_USER, Some(&serde_json::to_string(user)?))?;
    write_state(conn, ACCOUNT_EMAIL, Some(email))?;
    Ok(())
}

fn store_user(conn: &Connection, user: &AccountUser) -> Result<()> {
    write_state(conn, ACCOUNT_USER, Some(&serde_json::to_string(user)?))
}

/// Sign-out never changes the identity (D81): the device id, the signing key and
/// the email hint all survive.
fn clear_session(conn: &Connection) -> Result<()> {
    write_state(conn, ACCOUNT_TOKEN, None)?;
    write_state(conn, ACCOUNT_USER, None)
}

fn check_password_length(password: &str) -> Result<()> {
    if password.len() < 8 {
        return Err(Error::account("password_short"));
    }
    if password.len() > 4096 {
        return Err(Error::account("password_long"));
    }
    Ok(())
}

/// Local only, no network: what the account screens show right away.
pub fn account_status(database: &Database) -> Result<AccountStatus> {
    let conn = database.lock()?;
    let server = server(&conn)?;
    Ok(AccountStatus {
        official: server == OFFICIAL_SERVER,
        server,
        signed_in: is_signed_in(&conn)?,
        user: cached_user(&conn)?,
        email_hint: read_state(&conn, ACCOUNT_EMAIL)?,
        notice: read_state(&conn, ACCOUNT_NOTICE)?,
    })
}

pub fn dismiss_account_notice(database: &Database) -> Result<()> {
    write_state(&*database.lock()?, ACCOUNT_NOTICE, None)
}

/// Trims, strips a trailing slash, and checks the URL is one this app will talk
/// plaintext HTTP to only when it is safe to (loopback, a private network, the
/// Android emulator's host alias, or a debug build). Pure, so it is unit-tested
/// without touching `local_state`.
fn validate_server_url(raw: &str) -> Result<String> {
    let trimmed = raw.trim().trim_end_matches('/');
    let url = Url::parse(trimmed).map_err(|_| Error::account("invalid_server"))?;
    if !matches!(url.path(), "" | "/") || url.query().is_some() {
        return Err(Error::account("invalid_server"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(Error::account("invalid_server"));
    }
    match url.scheme() {
        "https" => {}
        "http" if cfg!(debug_assertions) || is_allowed_http_host(&url) => {}
        _ => return Err(Error::account("invalid_server")),
    }
    Ok(trimmed.to_owned())
}

fn is_allowed_http_host(url: &Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(domain)) => domain == "localhost",
        Some(url::Host::Ipv4(ip)) => {
            ip.is_loopback() || ip.is_private() || ip == Ipv4Addr::new(10, 0, 2, 2)
        }
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    }
}

/// `None` goes back to the official server. Rejected with `signed_in` while a
/// session is open: sign out first (changing server mid-session would silently
/// point an open token at another server).
pub fn set_server(database: &Database, url: Option<String>) -> Result<()> {
    let conn = database.lock()?;
    if is_signed_in(&conn)? {
        return Err(Error::account("signed_in"));
    }
    match url {
        None => write_state(&conn, ACCOUNT_SERVER, None),
        Some(raw) => {
            let normalized = validate_server_url(&raw)?;
            write_state(&conn, ACCOUNT_SERVER, Some(&normalized))
        }
    }
}

pub(crate) fn authed_context(database: &Database) -> Result<(String, String, String)> {
    let conn = database.lock()?;
    let token = read_state(&conn, ACCOUNT_TOKEN)?.ok_or_else(|| Error::account("not_signed_in"))?;
    Ok((server(&conn)?, ops::device_id(&conn)?, token))
}

fn optional_auth(database: &Database) -> Result<(String, Option<(String, String)>)> {
    let conn = database.lock()?;
    let base = server(&conn)?;
    let auth = match read_state(&conn, ACCOUNT_TOKEN)? {
        Some(token) => Some((ops::device_id(&conn)?, token)),
        None => None,
    };
    Ok((base, auth))
}

/// Central reaction to the special 401 codes an authenticated call (or a login)
/// can answer with (D81, D50): resets the device, clears a session the server
/// ended, or just clears a dead token. Always hands the original error back.
pub(crate) async fn react_to_account_error<T>(
    database: &Database,
    reset: ResetContext<'_>,
    result: Result<T>,
) -> Result<T> {
    if let Err(Error::Account { code, .. }) = &result {
        match code.as_str() {
            "device_removed" => {
                device_reset::reset(database, reset.covers, reset.lan, reset.data_dir).await?;
                write_state(&*database.lock()?, ACCOUNT_NOTICE, Some("device_removed"))?;
            }
            "signed_out_password_changed" => {
                let conn = database.lock()?;
                clear_session(&conn)?;
                write_state(&conn, ACCOUNT_NOTICE, Some("signed_out_password_changed"))?;
            }
            "invalid_token" => {
                clear_session(&*database.lock()?)?;
            }
            _ => {}
        }
    }
    result
}

// --- Operations ------------------------------------------------------------

pub async fn server_info(database: &Database) -> Result<ServerInfo> {
    let base = server(&*database.lock()?)?;
    let client = client::build_client()?;
    Ok(client::server_info(&client, &base).await?.into())
}

pub async fn register(
    database: &Database,
    email: &str,
    password: &str,
    name: &str,
    locale: &str,
) -> Result<()> {
    check_password_length(password)?;
    let base = server(&*database.lock()?)?;
    let client = client::build_client()?;
    client::register(&client, &base, email, password, name, locale).await
}

pub async fn confirm_email(database: &Database, email: &str, code: &str) -> Result<()> {
    let base = server(&*database.lock()?)?;
    let client = client::build_client()?;
    client::confirm_email(&client, &base, email, code).await
}

pub async fn resend_code(database: &Database, email: &str) -> Result<()> {
    let base = server(&*database.lock()?)?;
    let client = client::build_client()?;
    client::resend_code(&client, &base, email).await
}

pub async fn forgot_password(database: &Database, email: &str) -> Result<()> {
    let base = server(&*database.lock()?)?;
    let client = client::build_client()?;
    client::forgot_password(&client, &base, email).await
}

pub async fn reset_password(
    database: &Database,
    reset: ResetContext<'_>,
    email: &str,
    code: &str,
    password: &str,
) -> Result<()> {
    check_password_length(password)?;
    let (base, auth) = optional_auth(database)?;
    let client = client::build_client()?;
    let auth_ref = auth
        .as_ref()
        .map(|(device_id, token)| client::Auth { device_id, token });
    let result = client::reset_password(&client, &base, email, code, password, auth_ref).await;
    react_to_account_error(database, reset, result).await
}

/// Challenge, sign with this device's key, log in: `two_factor_required` and
/// `invalid_two_factor_code` pass straight through (the UI asks for the code and
/// calls this again, which asks for a fresh challenge, since each one is single use).
pub async fn sign_in(
    database: &Database,
    reset: ResetContext<'_>,
    email: &str,
    password: &str,
    totp_code: Option<String>,
) -> Result<AccountUser> {
    check_password_length(password)?;
    let base = server(&*database.lock()?)?;
    let client = client::build_client()?;
    let challenge = client::auth_challenge(&client, &base).await?;
    let (device_id, device_name, public_key, signature) = {
        let conn = database.lock()?;
        let device_id = ops::device_id(&conn)?;
        let device_name = lan::device_name(&conn)?;
        let key = ops::signing_key(&conn)?;
        let message = format!("kosmotheka-login\n{challenge}\n{device_id}");
        let signature = key.sign_bytes(message.as_bytes());
        (device_id, device_name, key.public_key(), signature)
    };
    let result = client::login(
        &client,
        &base,
        email,
        password,
        &device_id,
        &device_name,
        &public_key,
        &challenge,
        &signature,
        totp_code.as_deref(),
    )
    .await;
    let login = react_to_account_error(database, reset, result).await?;
    let user = AccountUser::from(login.user);
    store_session(&*database.lock()?, &login.token, &user, email)?;
    Ok(user)
}

/// Whatever the server answers (including no network at all), the session is
/// forgotten: signing out never changes the identity (D81).
pub async fn sign_out(database: &Database) -> Result<()> {
    if let Ok((base, device_id, token)) = authed_context(database)
        && let Ok(client) = client::build_client()
    {
        let _ = client::logout(&client, &base, &device_id, &token).await;
    }
    clear_session(&*database.lock()?)
}

pub async fn get_account(database: &Database, reset: ResetContext<'_>) -> Result<AccountDetails> {
    let (base, device_id, token) = authed_context(database)?;
    let client = client::build_client()?;
    let result = client::get_account(&client, &base, &device_id, &token).await;
    let details = react_to_account_error(database, reset, result).await?;
    let details: AccountDetails = details.into();
    let user = AccountUser {
        id: details.id.clone(),
        email: details.email.clone(),
        name: details.name.clone(),
        locale: details.locale.clone(),
    };
    store_user(&*database.lock()?, &user)?;
    Ok(details)
}

pub async fn update_account(
    database: &Database,
    reset: ResetContext<'_>,
    name: Option<String>,
    locale: Option<String>,
) -> Result<AccountDetails> {
    let (base, device_id, token) = authed_context(database)?;
    let client = client::build_client()?;
    let result = client::update_account(
        &client,
        &base,
        &device_id,
        &token,
        name.as_deref(),
        locale.as_deref(),
    )
    .await;
    let details = react_to_account_error(database, reset, result).await?;
    let details: AccountDetails = details.into();
    let user = AccountUser {
        id: details.id.clone(),
        email: details.email.clone(),
        name: details.name.clone(),
        locale: details.locale.clone(),
    };
    store_user(&*database.lock()?, &user)?;
    Ok(details)
}

pub async fn change_password(
    database: &Database,
    reset: ResetContext<'_>,
    current_password: &str,
    new_password: &str,
) -> Result<()> {
    check_password_length(new_password)?;
    let (base, device_id, token) = authed_context(database)?;
    let client = client::build_client()?;
    let result = client::change_password(
        &client,
        &base,
        &device_id,
        &token,
        current_password,
        new_password,
    )
    .await;
    react_to_account_error(database, reset, result).await
}

/// On success, the account is gone: forget the session like a sign-out.
pub async fn delete_account(
    database: &Database,
    reset: ResetContext<'_>,
    password: &str,
) -> Result<()> {
    let (base, device_id, token) = authed_context(database)?;
    let client = client::build_client()?;
    let result = client::delete_account(&client, &base, &device_id, &token, password).await;
    let result = react_to_account_error(database, reset, result).await;
    result?;
    clear_session(&*database.lock()?)
}

pub async fn account_devices(
    database: &Database,
    reset: ResetContext<'_>,
) -> Result<Vec<AccountDevice>> {
    let (base, device_id, token) = authed_context(database)?;
    let client = client::build_client()?;
    let result = client::list_devices(&client, &base, &device_id, &token).await;
    let devices = react_to_account_error(database, reset, result).await?;
    Ok(devices.into_iter().map(Into::into).collect())
}

pub async fn remove_account_device(
    database: &Database,
    reset: ResetContext<'_>,
    id: &str,
) -> Result<()> {
    let (base, device_id, token) = authed_context(database)?;
    let client = client::build_client()?;
    let result = client::remove_device(&client, &base, &device_id, &token, id).await;
    react_to_account_error(database, reset, result).await
}

pub async fn restore_account_device(
    database: &Database,
    reset: ResetContext<'_>,
    id: &str,
) -> Result<()> {
    let (base, device_id, token) = authed_context(database)?;
    let client = client::build_client()?;
    let result = client::restore_device(&client, &base, &device_id, &token, id).await;
    react_to_account_error(database, reset, result).await
}

pub async fn two_factor_setup(
    database: &Database,
    reset: ResetContext<'_>,
) -> Result<TwoFactorSetup> {
    let (base, device_id, token) = authed_context(database)?;
    let client = client::build_client()?;
    let result = client::two_factor_setup(&client, &base, &device_id, &token).await;
    Ok(react_to_account_error(database, reset, result)
        .await?
        .into())
}

pub async fn two_factor_enable(
    database: &Database,
    reset: ResetContext<'_>,
    code: &str,
) -> Result<Vec<String>> {
    let (base, device_id, token) = authed_context(database)?;
    let client = client::build_client()?;
    let result = client::two_factor_enable(&client, &base, &device_id, &token, code).await;
    react_to_account_error(database, reset, result).await
}

pub async fn two_factor_disable(
    database: &Database,
    reset: ResetContext<'_>,
    password: &str,
    code: &str,
) -> Result<()> {
    let (base, device_id, token) = authed_context(database)?;
    let client = client::build_client()?;
    let result =
        client::two_factor_disable(&client, &base, &device_id, &token, password, code).await;
    react_to_account_error(database, reset, result).await
}

pub async fn regenerate_backup_codes(
    database: &Database,
    reset: ResetContext<'_>,
    password: &str,
    code: &str,
) -> Result<Vec<String>> {
    let (base, device_id, token) = authed_context(database)?;
    let client = client::build_client()?;
    let result =
        client::regenerate_backup_codes(&client, &base, &device_id, &token, password, code).await;
    react_to_account_error(database, reset, result).await
}

/// "Dejar este dispositivo a otra persona" (D81): drops this installation from
/// the account (best-effort: the token and key are destroyed by the reset either
/// way, so a stale server-side record cannot be used) and resets the device. No
/// notice is stored: the user did this on purpose.
pub async fn give_device_away(database: &Database, reset: ResetContext<'_>) -> Result<()> {
    if let Ok((base, device_id, token)) = authed_context(database)
        && let Ok(client) = client::build_client()
    {
        let _ = client::remove_device(&client, &base, &device_id, &token, &device_id).await;
    }
    device_reset::reset(database, reset.covers, reset.lan, reset.data_dir).await
}

/// At startup, for a device signed into an account: learns in the background
/// whether it was removed elsewhere. The caller ignores network errors; a
/// `device_removed` answer still goes through `react_to_account_error` and resets.
pub async fn refresh_account_on_start(
    database: &Database,
    reset: ResetContext<'_>,
) -> Result<Option<AccountDetails>> {
    let signed_in = matches!(database.lock(), Ok(conn) if is_signed_in(&conn).unwrap_or(false));
    if !signed_in {
        return Ok(None);
    }
    get_account(database, reset).await.map(Some)
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::extract::State;
    use axum::routing::post;
    use axum::{Json, Router};
    use serde_json::{Value, json};
    use tokio::net::TcpListener;

    use super::*;
    use crate::lan::identity::Identity;

    #[test]
    fn accepts_https_anywhere_and_trims_a_trailing_slash() {
        assert_eq!(
            validate_server_url("https://api.kosmotheka.app/").unwrap(),
            "https://api.kosmotheka.app"
        );
    }

    #[test]
    fn rejects_a_path_query_or_credentials() {
        assert!(validate_server_url("https://example.com/v1").is_err());
        assert!(validate_server_url("https://example.com?x=1").is_err());
        assert!(validate_server_url("https://user:pass@example.com").is_err());
        assert!(validate_server_url("not a url").is_err());
    }

    #[test]
    fn http_is_rejected_for_a_public_host_outside_debug_builds() {
        // This crate is always built with debug assertions in `cargo test`, so the
        // pure host check is exercised directly instead.
        let url = Url::parse("http://example.com").unwrap();
        assert!(!is_allowed_http_host(&url));
        let url = Url::parse("http://localhost:1234").unwrap();
        assert!(is_allowed_http_host(&url));
        let url = Url::parse("http://127.0.0.1").unwrap();
        assert!(is_allowed_http_host(&url));
        let url = Url::parse("http://10.0.2.2").unwrap();
        assert!(is_allowed_http_host(&url));
        let url = Url::parse("http://192.168.1.5").unwrap();
        assert!(is_allowed_http_host(&url));
        let url = Url::parse("http://[::1]").unwrap();
        assert!(is_allowed_http_host(&url));
        let url = Url::parse("http://8.8.8.8").unwrap();
        assert!(!is_allowed_http_host(&url));
    }

    #[test]
    fn account_status_round_trips_through_local_state() {
        let database = Database::in_memory();
        let status = account_status(&database).unwrap();
        assert_eq!(status.server, OFFICIAL_SERVER);
        assert!(status.official);
        assert!(!status.signed_in);

        let user = AccountUser {
            id: "u1".into(),
            email: "ana@example.com".into(),
            name: "Ana".into(),
            locale: "es".into(),
        };
        store_session(&database.lock().unwrap(), "tok", &user, "ana@example.com").unwrap();
        write_state(
            &database.lock().unwrap(),
            ACCOUNT_NOTICE,
            Some("device_removed"),
        )
        .unwrap();

        let status = account_status(&database).unwrap();
        assert!(status.signed_in);
        assert_eq!(status.user, Some(user));
        assert_eq!(status.email_hint.as_deref(), Some("ana@example.com"));
        assert_eq!(status.notice.as_deref(), Some("device_removed"));

        dismiss_account_notice(&database).unwrap();
        assert_eq!(account_status(&database).unwrap().notice, None);
    }

    #[tokio::test]
    async fn sign_out_clears_token_and_user_but_keeps_identity_and_email_hint() {
        let database = Database::in_memory();
        let device_id_before = ops::device_id(&database.lock().unwrap()).unwrap();
        let key_before = ops::signing_key(&database.lock().unwrap())
            .unwrap()
            .public_key();
        let user = AccountUser {
            id: "u1".into(),
            email: "ana@example.com".into(),
            name: "Ana".into(),
            locale: "es".into(),
        };
        store_session(&database.lock().unwrap(), "tok", &user, "ana@example.com").unwrap();

        sign_out(&database).await.unwrap();

        let status = account_status(&database).unwrap();
        assert!(!status.signed_in);
        assert_eq!(status.user, None);
        assert_eq!(status.email_hint.as_deref(), Some("ana@example.com"));
        assert_eq!(
            ops::device_id(&database.lock().unwrap()).unwrap(),
            device_id_before
        );
        assert_eq!(
            ops::signing_key(&database.lock().unwrap())
                .unwrap()
                .public_key(),
            key_before
        );
    }

    /// A tiny account server, enough to drive `sign_in` and the central 401
    /// handling without the network.
    struct Stub {
        address: SocketAddr,
        challenges: Arc<AtomicUsize>,
    }

    fn stub_router(behavior: &'static str, challenges: Arc<AtomicUsize>) -> Router {
        #[derive(Clone)]
        struct StubState {
            behavior: &'static str,
            challenges: Arc<AtomicUsize>,
        }
        async fn challenge(State(state): State<StubState>) -> Json<Value> {
            state.challenges.fetch_add(1, Ordering::SeqCst);
            Json(json!({ "challenge": "c", "expires_at": "later" }))
        }
        async fn login(
            State(state): State<StubState>,
            Json(body): Json<Value>,
        ) -> axum::response::Response {
            use axum::http::StatusCode;
            use axum::response::IntoResponse;
            match state.behavior {
                "ok" => Json(json!({
                    "token": "tok",
                    "user": { "id": "u1", "email": "ana@example.com", "name": "Ana", "locale": "es" }
                }))
                .into_response(),
                "two_factor" if body.get("totp_code").is_none() => {
                    (StatusCode::UNAUTHORIZED, Json(json!({ "error": "two_factor_required" })))
                        .into_response()
                }
                "two_factor" => Json(json!({
                    "token": "tok2",
                    "user": { "id": "u1", "email": "ana@example.com", "name": "Ana", "locale": "es" }
                }))
                .into_response(),
                "device_removed" => {
                    (StatusCode::UNAUTHORIZED, Json(json!({ "error": "device_removed" }))).into_response()
                }
                _ => (StatusCode::UNAUTHORIZED, Json(json!({ "error": "invalid_credentials" }))).into_response(),
            }
        }
        Router::new()
            .route("/v1/auth/challenge", post(challenge))
            .route("/v1/auth/login", post(login))
            .with_state(StubState {
                behavior,
                challenges,
            })
    }

    async fn start_stub(behavior: &'static str) -> Stub {
        let challenges = Arc::new(AtomicUsize::new(0));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = stub_router(behavior, challenges.clone());
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Stub {
            address,
            challenges,
        }
    }

    fn reset_context<'a>(
        covers: &'a Covers,
        lan: &'a LanService,
        data_dir: &'a Path,
    ) -> ResetContext<'a> {
        ResetContext {
            covers,
            lan,
            data_dir,
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sign_in_asks_for_a_challenge_and_stores_the_session() {
        let stub = start_stub("ok").await;
        let database = Database::in_memory();
        write_state(
            &database.lock().unwrap(),
            ACCOUNT_SERVER,
            Some(&format!("http://{}", stub.address)),
        )
        .unwrap();
        let dir =
            std::env::temp_dir().join(format!("library-account-signin-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let covers = Covers::new(dir.join("covers")).unwrap();
        let lan = LanService::new(database.clone(), Identity::generate().unwrap());

        let user = sign_in(
            &database,
            reset_context(&covers, &lan, &dir),
            "ana@example.com",
            "password123",
            None,
        )
        .await
        .unwrap();

        assert_eq!(user.email, "ana@example.com");
        assert_eq!(stub.challenges.load(Ordering::SeqCst), 1);
        let status = account_status(&database).unwrap();
        assert!(status.signed_in);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn two_factor_required_asks_for_a_new_challenge_on_retry() {
        let stub = start_stub("two_factor").await;
        let database = Database::in_memory();
        write_state(
            &database.lock().unwrap(),
            ACCOUNT_SERVER,
            Some(&format!("http://{}", stub.address)),
        )
        .unwrap();
        let dir = std::env::temp_dir().join(format!("library-account-2fa-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let covers = Covers::new(dir.join("covers")).unwrap();
        let lan = LanService::new(database.clone(), Identity::generate().unwrap());

        let first = sign_in(
            &database,
            reset_context(&covers, &lan, &dir),
            "ana@example.com",
            "password123",
            None,
        )
        .await;
        assert!(matches!(first, Err(Error::Account{code, ..}) if code == "two_factor_required"));
        assert_eq!(stub.challenges.load(Ordering::SeqCst), 1);

        let second = sign_in(
            &database,
            reset_context(&covers, &lan, &dir),
            "ana@example.com",
            "password123",
            Some("123456".into()),
        )
        .await
        .unwrap();
        assert_eq!(second.email, "ana@example.com");
        assert_eq!(
            stub.challenges.load(Ordering::SeqCst),
            2,
            "the retry asks for a brand new challenge"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn device_removed_on_login_resets_the_device_and_leaves_a_notice() {
        let stub = start_stub("device_removed").await;
        let database = Database::in_memory();
        write_state(
            &database.lock().unwrap(),
            ACCOUNT_SERVER,
            Some(&format!("http://{}", stub.address)),
        )
        .unwrap();
        let device_id_before = ops::device_id(&database.lock().unwrap()).unwrap();
        let key_before = ops::signing_key(&database.lock().unwrap())
            .unwrap()
            .public_key();
        let dir =
            std::env::temp_dir().join(format!("library-account-removed-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let covers = Covers::new(dir.join("covers")).unwrap();
        let lan = LanService::new(database.clone(), Identity::generate().unwrap());

        let result = sign_in(
            &database,
            reset_context(&covers, &lan, &dir),
            "ana@example.com",
            "password123",
            None,
        )
        .await;

        assert!(matches!(result, Err(Error::Account{code, ..}) if code == "device_removed"));
        let device_id_after = ops::device_id(&database.lock().unwrap()).unwrap();
        let key_after = ops::signing_key(&database.lock().unwrap())
            .unwrap()
            .public_key();
        assert_ne!(device_id_before, device_id_after);
        assert_ne!(key_before, key_after);
        let fresh = Database::in_memory();
        let fresh_devices: i64 = database
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM devices", [], |r| r.get(0))
            .unwrap();
        let empty_devices: i64 = fresh
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM devices", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            fresh_devices, empty_devices,
            "libraries are gone, like a fresh database"
        );
        assert_eq!(
            account_status(&database).unwrap().notice.as_deref(),
            Some("device_removed")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn signed_out_password_changed_clears_the_session_and_sets_the_notice() {
        async fn behavior(State(_): State<()>) -> axum::response::Response {
            use axum::http::StatusCode;
            use axum::response::IntoResponse;
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "signed_out_password_changed" })),
            )
                .into_response()
        }
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new()
            .route("/v1/account", axum::routing::get(behavior))
            .with_state(());
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let database = Database::in_memory();
        write_state(
            &database.lock().unwrap(),
            ACCOUNT_SERVER,
            Some(&format!("http://{address}")),
        )
        .unwrap();
        let user = AccountUser {
            id: "u1".into(),
            email: "ana@example.com".into(),
            name: "Ana".into(),
            locale: "es".into(),
        };
        store_session(&database.lock().unwrap(), "tok", &user, "ana@example.com").unwrap();
        let dir =
            std::env::temp_dir().join(format!("library-account-pwchange-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let covers = Covers::new(dir.join("covers")).unwrap();
        let lan = LanService::new(database.clone(), Identity::generate().unwrap());

        let result = get_account(&database, reset_context(&covers, &lan, &dir)).await;

        assert!(
            matches!(result, Err(Error::Account{code, ..}) if code == "signed_out_password_changed")
        );
        let status = account_status(&database).unwrap();
        assert!(!status.signed_in);
        assert_eq!(
            status.notice.as_deref(),
            Some("signed_out_password_changed")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
