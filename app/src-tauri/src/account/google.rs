//! «Continuar con Google» from the app's side (D77, docs/architecture/accounts.md). The app starts with a secret
//! that never leaves it (the verifier; the server only learns its hash), the system browser goes through the
//! server and Google, and what comes back by `kosmotheka://auth/google` is a one-use code that is worth nothing
//! without that secret. The way back only counts if it is the attempt this app started and is still waiting for.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{AccountUser, ResetContext, client, react_to_account_error, server, store_session};
use crate::cloud::invitations::Secret;
use crate::db::Database;
use crate::error::{Error, Result};
use crate::ops::{self, read_state, write_state};
use crate::{lan, links::GoogleOutcome, links::GoogleReturn};

const PENDING: &str = "google_pending";

/// As long as the server keeps an attempt alive.
const LIFETIME_SECS: u64 = 600;

/// What the app is waiting for: its secret, and, once the way back arrived, the code (kept so that a second factor
/// can be asked for without starting over).
#[derive(Serialize, Deserialize)]
struct Pending {
    attempt: String,
    verifier: Secret,
    server: String,
    expires_at: u64,
    code: Option<Secret>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

fn load(database: &Database) -> Result<Option<Pending>> {
    let conn = database.lock()?;
    Ok(read_state(&conn, PENDING)?.and_then(|raw| serde_json::from_str(&raw).ok()))
}

fn save(database: &Database, pending: &Pending) -> Result<()> {
    write_state(
        &*database.lock()?,
        PENDING,
        Some(&serde_json::to_string(pending)?),
    )
}

pub fn forget(conn: &rusqlite::Connection) -> Result<()> {
    write_state(conn, PENDING, None)
}

/// The person gave up (or a new attempt starts): whatever way back comes later is not an attempt of this app.
pub fn cancel(database: &Database) -> Result<()> {
    forget(&*database.lock()?)
}

/// Starts an attempt: returns the address the system browser has to open. The server is the account's, never one a
/// link names.
pub async fn start(database: &Database, locale: &str) -> Result<String> {
    let base = server(&*database.lock()?)?;
    let verifier: [u8; 32] = rand::rng().random();
    let verifier = URL_SAFE_NO_PAD.encode(verifier);
    let hash = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let http = client::build_client()?;
    let started = client::google_start(&http, &base, &hash, locale).await?;
    // The server says where the browser goes; it is not trusted to name anything but a web address.
    if !opens_in_a_browser(&started.url, &base) {
        return Err(Error::account("google_failed"));
    }
    save(
        database,
        &Pending {
            attempt: started.attempt,
            verifier: Secret::new(verifier),
            server: base,
            expires_at: now() + LIFETIME_SECS,
            code: None,
        },
    )?;
    Ok(started.url)
}

/// A web address the system may open for the person: `https` (or `http` where the server itself is `http`), with a host
/// and no credentials. Not a file, a script, another app's scheme, nor this app's own.
fn opens_in_a_browser(address: &str, server: &str) -> bool {
    let Ok(url) = url::Url::parse(address) else {
        return false;
    };
    let allowed = match url.scheme() {
        "https" => true,
        "http" => server.starts_with("http://"),
        _ => false,
    };
    allowed && url.host().is_some() && url.username().is_empty() && url.password().is_none()
}

/// The reasons the server's page gives for not coming back with a code, and no other: what the way back says is
/// shown only if the app knows it.
fn known_reason(code: &str) -> &'static str {
    [
        "google_cancelled",
        "google_failed",
        "google_email_not_verified",
        "google_other_account",
        "registration_closed",
        "account_blocked",
    ]
    .into_iter()
    .find(|known| *known == code)
    .unwrap_or("google_failed")
}

/// What came back by the link. `None`: it is not for an attempt this app is waiting for, and nothing was asked.
pub async fn complete(
    database: &Database,
    reset: ResetContext<'_>,
    back: GoogleReturn,
) -> Result<Option<AccountUser>> {
    let Some(mut pending) = load(database)? else {
        return Ok(None);
    };
    // Only the attempt this app started and is waiting for: a way back made by somebody else (their own attempt, their
    // own Google) is not this person's, and is not even answered.
    if back.attempt != pending.attempt || pending.server != server(&*database.lock()?)? {
        return Ok(None);
    }
    if pending.expires_at < now() {
        cancel(database)?;
        return Err(Error::account("google_expired"));
    }
    match back.outcome {
        GoogleOutcome::Error(code) => {
            cancel(database)?;
            Err(Error::account(known_reason(&code)))
        }
        GoogleOutcome::Code(code) => {
            pending.code = Some(code);
            save(database, &pending)?;
            finish(database, reset, None).await.map(Some)
        }
    }
}

/// Exchanges the code for this device's token. Asked again with the second factor's code if the account has one.
pub async fn finish(
    database: &Database,
    reset: ResetContext<'_>,
    totp_code: Option<String>,
) -> Result<AccountUser> {
    let pending = load(database)?.ok_or_else(|| Error::account("google_expired"))?;
    let code = pending
        .code
        .as_ref()
        .ok_or_else(|| Error::account("google_expired"))?;
    let http = client::build_client()?;
    let challenge = client::auth_challenge(&http, &pending.server).await?;
    let (device_id, device_name, public_key, signature) = {
        let conn = database.lock()?;
        let device_id = ops::device_id(&conn)?;
        let device_name = lan::device_name(&conn)?;
        let key = ops::signing_key(&conn)?;
        let message = format!("kosmotheka-login\n{challenge}\n{device_id}");
        let signature = key.sign_bytes(message.as_bytes());
        (device_id, device_name, key.public_key(), signature)
    };
    let result = client::google_finish(
        &http,
        &pending.server,
        &pending.attempt,
        code.expose(),
        pending.verifier.expose(),
        &device_id,
        &device_name,
        &public_key,
        &challenge,
        &signature,
        totp_code.as_deref(),
    )
    .await;
    if let Err(Error::Account { code, .. }) = &result
        && !keeps_waiting(code)
    {
        cancel(database)?;
    }
    let login = react_to_account_error(database, reset, result).await?;
    let user = AccountUser::from(login.user);
    store_session(&*database.lock()?, &login.token, &user, &user.email)?;
    cancel(database)?;
    Ok(user)
}

/// The answers that leave the way back waiting: a second factor still to give, or no reach to the server. Anything
/// else the server said is an answer, and the attempt is over.
fn keeps_waiting(code: &str) -> bool {
    matches!(
        code,
        "two_factor_required"
            | "invalid_two_factor_code"
            | "server_unavailable"
            | "too_many_requests"
    )
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::routing::post;
    use axum::{Json, Router};
    use serde_json::{Value, json};
    use tokio::net::TcpListener;

    use super::*;
    use crate::account::{ACCOUNT_SERVER, account_status, clear_session};
    use crate::covers::Covers;
    use crate::lan::identity::Identity;
    use crate::lan::service::LanService;

    /// What the server stub was asked, in order: `(route, body)`.
    type Asked = Arc<Mutex<Vec<(String, Value)>>>;

    #[derive(Clone)]
    struct Stub {
        behavior: &'static str,
        asked: Asked,
        url: String,
    }

    async fn challenge(State(stub): State<Stub>) -> Json<Value> {
        stub.asked
            .lock()
            .unwrap()
            .push(("challenge".into(), Value::Null));
        Json(json!({ "challenge": "c", "expires_at": "later" }))
    }

    async fn start_route(State(stub): State<Stub>, Json(body): Json<Value>) -> Json<Value> {
        stub.asked.lock().unwrap().push(("start".into(), body));
        Json(json!({ "attempt": "ATTEMPT-1", "url": stub.url, "expires_at": "later" }))
    }

    async fn finish_route(
        State(stub): State<Stub>,
        Json(body): Json<Value>,
    ) -> axum::response::Response {
        let second_factor_given = body.get("totp_code").is_some();
        stub.asked.lock().unwrap().push(("finish".into(), body));
        let user = json!({ "id": "u1", "email": "ana@example.com", "name": "Ana", "locale": "es" });
        match stub.behavior {
            "two_factor" if !second_factor_given => (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "two_factor_required" })),
            )
                .into_response(),
            "invalid_code" => (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid_google_code" })),
            )
                .into_response(),
            _ => Json(json!({ "token": "tok", "user": user })).into_response(),
        }
    }

    struct World {
        database: Database,
        asked: Asked,
        dir: PathBuf,
        covers: Covers,
        lan: LanService,
    }

    impl World {
        fn context(&self) -> ResetContext<'_> {
            ResetContext {
                covers: &self.covers,
                lan: &self.lan,
                data_dir: Path::new(&self.dir),
            }
        }

        fn calls(&self) -> usize {
            self.asked.lock().unwrap().len()
        }

        fn asked_of(&self, route: &str) -> Vec<Value> {
            self.asked
                .lock()
                .unwrap()
                .iter()
                .filter(|(name, _)| name == route)
                .map(|(_, body)| body.clone())
                .collect()
        }
    }

    async fn world(behavior: &'static str, url: &str) -> World {
        let asked: Asked = Arc::default();
        let stub = Stub {
            behavior,
            asked: asked.clone(),
            url: url.to_owned(),
        };
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address: SocketAddr = listener.local_addr().unwrap();
        let app = Router::new()
            .route("/v1/auth/challenge", post(challenge))
            .route("/v1/auth/google/start", post(start_route))
            .route("/v1/auth/google/finish", post(finish_route))
            .with_state(stub);
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
        let dir = std::env::temp_dir().join(format!(
            "library-google-{}-{}",
            std::process::id(),
            ops::new_id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let covers = Covers::new(dir.join("covers")).unwrap();
        let lan = LanService::new(database.clone(), Identity::generate().unwrap());
        World {
            database,
            asked,
            dir,
            covers,
            lan,
        }
    }

    const BROWSER: &str = "https://api.kosmotheka.app/v1/auth/google/connect?attempt=ATTEMPT-1";

    fn back(attempt: &str, outcome: GoogleOutcome) -> GoogleReturn {
        GoogleReturn {
            attempt: attempt.to_owned(),
            outcome,
        }
    }

    fn code(text: &str) -> GoogleOutcome {
        GoogleOutcome::Code(Secret::new(text))
    }

    fn pending_of(world: &World) -> Option<Pending> {
        load(&world.database).unwrap()
    }

    fn reason(error: &Error) -> String {
        match error {
            Error::Account { code, .. } => code.clone(),
            other => panic!("unexpected error {other:?}"),
        }
    }

    // --- 3. A way back that is not the attempt this app started -----------------------------------------------------

    #[tokio::test(flavor = "multi_thread")]
    async fn a_way_back_with_no_attempt_under_way_asks_no_server_anything() {
        let world = world("ok", BROWSER).await;

        let done = complete(
            &world.database,
            world.context(),
            back("ATTEMPT-1", code("c0de")),
        )
        .await
        .unwrap();

        assert_eq!(done, None);
        assert_eq!(world.calls(), 0, "no server was asked");
        assert!(!account_status(&world.database).unwrap().signed_in);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_way_back_of_somebody_elses_attempt_is_ignored_and_the_own_attempt_stays() {
        let world = world("ok", BROWSER).await;
        start(&world.database, "es").await.unwrap();
        let calls_after_start = world.calls();

        // Jhonny started his own attempt, signed in with his Google and sent the victim the way back.
        let done = complete(
            &world.database,
            world.context(),
            back("JHONNYS-ATTEMPT", code("c0de")),
        )
        .await
        .unwrap();

        assert_eq!(done, None);
        assert_eq!(
            world.calls(),
            calls_after_start,
            "the server was not asked again"
        );
        assert!(
            !account_status(&world.database).unwrap().signed_in,
            "nobody was signed in"
        );
        let pending = pending_of(&world).expect("the own attempt is still waiting");
        assert_eq!(pending.attempt, "ATTEMPT-1");
        assert!(pending.code.is_none(), "and the foreign code was not kept");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_way_back_after_the_attempt_expired_asks_nothing_and_says_so() {
        let world = world("ok", BROWSER).await;
        start(&world.database, "es").await.unwrap();
        let mut pending = pending_of(&world).unwrap();
        pending.expires_at = now() - 1;
        save(&world.database, &pending).unwrap();
        let calls_after_start = world.calls();

        let done = complete(
            &world.database,
            world.context(),
            back("ATTEMPT-1", code("c0de")),
        )
        .await;

        assert_eq!(reason(&done.unwrap_err()), "google_expired");
        assert_eq!(world.calls(), calls_after_start);
        assert!(pending_of(&world).is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn after_cancelling_or_signing_out_a_way_back_is_not_an_attempt_any_more() {
        let world = world("ok", BROWSER).await;
        start(&world.database, "es").await.unwrap();
        cancel(&world.database).unwrap();
        let done = complete(
            &world.database,
            world.context(),
            back("ATTEMPT-1", code("c0de")),
        )
        .await
        .unwrap();
        assert_eq!(done, None);

        start(&world.database, "es").await.unwrap();
        clear_session(&world.database.lock().unwrap()).unwrap();
        let done = complete(
            &world.database,
            world.context(),
            back("ATTEMPT-1", code("c0de")),
        )
        .await
        .unwrap();
        assert_eq!(done, None);
        assert!(world.asked_of("finish").is_empty());
    }

    // --- The way itself ---------------------------------------------------------------------------------------------

    #[tokio::test(flavor = "multi_thread")]
    async fn the_secret_stays_in_the_app_and_the_server_only_learns_its_hash() {
        let world = world("ok", BROWSER).await;

        let url = start(&world.database, "es").await.unwrap();

        assert_eq!(url, BROWSER);
        let started = world.asked_of("start");
        let hash = started[0]["verifier_hash"].as_str().unwrap();
        let verifier = pending_of(&world).unwrap().verifier;
        assert_eq!(
            hash,
            URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.expose().as_bytes()))
        );
        assert!(
            !started[0].to_string().contains(verifier.expose()),
            "the secret itself was not sent"
        );

        let user = complete(
            &world.database,
            world.context(),
            back("ATTEMPT-1", code("c0de")),
        )
        .await
        .unwrap()
        .expect("the attempt this app started");
        assert_eq!(user.email, "ana@example.com");
        let finished = world.asked_of("finish");
        assert_eq!(finished[0]["verifier"], verifier.expose());
        assert_eq!(finished[0]["code"], "c0de");
        assert_eq!(finished[0]["attempt"], "ATTEMPT-1");
        assert!(
            finished[0]["signature"]
                .as_str()
                .is_some_and(|s| !s.is_empty())
        );
        assert!(account_status(&world.database).unwrap().signed_in);
        assert!(pending_of(&world).is_none(), "used: not kept");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_browser_is_only_sent_to_a_web_address() {
        for hostile in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "kosmotheka://auth/google?attempt=ATTEMPT-1&code=x",
            "ftp://example.org/x",
            "intent://scan/#Intent;scheme=zxing;end",
            "https://user:pass@example.org/x",
            "not a url",
        ] {
            let world = world("ok", hostile).await;

            let refused = start(&world.database, "es").await;

            assert_eq!(reason(&refused.unwrap_err()), "google_failed", "{hostile}");
            assert!(pending_of(&world).is_none(), "{hostile}");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_reason_the_server_gives_is_shown_only_if_it_is_one_of_its_own() {
        for (given, shown) in [
            ("google_cancelled", "google_cancelled"),
            ("registration_closed", "registration_closed"),
            ("account_blocked", "account_blocked"),
            ("google_email_not_verified", "google_email_not_verified"),
            ("google_other_account", "google_other_account"),
            ("device_removed", "google_failed"),
            ("anything_a_link_says", "google_failed"),
        ] {
            let world = world("ok", BROWSER).await;
            start(&world.database, "es").await.unwrap();

            let refused = complete(
                &world.database,
                world.context(),
                back("ATTEMPT-1", GoogleOutcome::Error(given.to_owned())),
            )
            .await;

            assert_eq!(reason(&refused.unwrap_err()), shown, "{given}");
            assert!(pending_of(&world).is_none());
            assert!(world.asked_of("finish").is_empty());
            assert!(!account_status(&world.database).unwrap().signed_in);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_account_with_a_second_factor_keeps_the_way_back_waiting_for_its_code() {
        let world = world("two_factor", BROWSER).await;
        start(&world.database, "es").await.unwrap();

        let first = complete(
            &world.database,
            world.context(),
            back("ATTEMPT-1", code("c0de")),
        )
        .await;
        assert_eq!(reason(&first.unwrap_err()), "two_factor_required");
        assert!(
            pending_of(&world).unwrap().code.is_some(),
            "kept for the retry"
        );
        assert!(!account_status(&world.database).unwrap().signed_in);

        let user = finish(&world.database, world.context(), Some("123456".into()))
            .await
            .unwrap();

        assert_eq!(user.email, "ana@example.com");
        assert_eq!(
            world.asked_of("challenge").len(),
            2,
            "a new challenge for the retry"
        );
        assert_eq!(world.asked_of("finish")[1]["totp_code"], "123456");
        assert!(pending_of(&world).is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_code_the_server_refuses_ends_the_attempt() {
        let world = world("invalid_code", BROWSER).await;
        start(&world.database, "es").await.unwrap();

        let refused = complete(
            &world.database,
            world.context(),
            back("ATTEMPT-1", code("c0de")),
        )
        .await;

        assert_eq!(reason(&refused.unwrap_err()), "invalid_google_code");
        assert!(pending_of(&world).is_none());
        assert!(!account_status(&world.database).unwrap().signed_in);
    }

    #[test]
    fn nothing_of_the_attempt_is_shown_by_debug() {
        let pending = Pending {
            attempt: "ATTEMPT-1".into(),
            verifier: Secret::new("the-secret-verifier"),
            server: "https://x".into(),
            expires_at: 1,
            code: Some(Secret::new("the-secret-code")),
        };
        let text = format!("{:?} {:?}", pending.verifier, pending.code);
        assert!(!text.contains("the-secret"));
        let returned = format!("{:?}", back("ATTEMPT-1", code("the-secret-code")));
        assert!(!returned.contains("the-secret"));
    }
}
