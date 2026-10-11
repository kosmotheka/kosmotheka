//! The real HTTP client for the cloud routes (docs/architecture/cloud.md), on top of
//! the account client: same `reqwest` client, same device-token headers.

use reqwest::Method;
use serde::Deserialize;
use serde_json::{Value, json};

use super::invitations::{CreatedInvitation, InvitationItem, InvitationKind, InvitationPreview};
use super::{Claimed, CloudApi, CloudLibrary, Listing, Pulled, PushReport};
use crate::account::{
    self,
    client::{Auth, build_client, send, send_empty, send_json},
};
use crate::db::Database;
use crate::error::{Error, Result};
use crate::metadata::Metadata;
use crate::metadata::via_server::parse_answer;
use crate::sync::{SyncOperation, VersionVector};
use kosmotheka_domain::Isbn;

/// The most the app reads of the answer to one book (7.8): the fixed shape is far smaller.
const MAX_BOOK_ANSWER: usize = 64 * 1024;

/// The server accepts at most this many operations per push.
const PUSH_BATCH: usize = 1000;

pub struct HttpCloud {
    client: reqwest::Client,
    base: String,
    /// This device's id and token; none for the calls that need no session.
    session: Option<(String, String)>,
}

impl HttpCloud {
    /// Fails with `not_signed_in` when this device has no account session.
    pub fn connect(database: &Database) -> Result<Self> {
        let (base, device_id, token) =
            account::authed_context(database).map_err(|error| match error {
                Error::Account { code, .. } if code == "not_signed_in" => {
                    Error::Cloud("not_signed_in")
                }
                other => other,
            })?;
        Ok(Self {
            client: build_client()?,
            base,
            session: Some((device_id, token)),
        })
    }

    /// The server this device talks to, with no session: it can only ask what needs none (an invitation's
    /// preview, what the server offers). Never sends the token, even if there is one.
    pub fn anonymous(database: &Database) -> Result<Self> {
        Ok(Self {
            client: build_client()?,
            base: account::server(&*database.lock()?)?,
            session: None,
        })
    }

    #[cfg(test)]
    fn for_test(base: &str) -> Self {
        Self {
            client: build_client().unwrap(),
            base: base.to_owned(),
            session: Some(("device-1".to_owned(), "token-1".to_owned())),
        }
    }

    fn auth(&self) -> Option<Auth<'_>> {
        self.session
            .as_ref()
            .map(|(device_id, token)| Auth { device_id, token })
    }

    fn invitations_url(&self, path: &str) -> String {
        format!("{}/v1/cloud/invitations{path}", self.base)
    }

    fn url(&self, path: &str) -> String {
        format!("{}/v1/cloud/libraries{path}", self.base)
    }
}

/// The server's own codes for this API; the rest keep the account client's mapping
/// (signed out, removed device, unreachable).
fn cloud_error(error: Error) -> Error {
    match error {
        Error::Account { code, .. } if code == "not_found" => Error::Cloud("not_found"),
        Error::Account { code, .. } if code == "schema_version" => Error::Cloud("schema_version"),
        other => other,
    }
}

/// What the app says of an invitation token the server does not know: the same words for a wrong one and for one
/// it forgot, so the code tells nothing more than the answer did.
fn invitation_error(error: Error) -> Error {
    match error {
        Error::Account { code, .. } if code == "not_found" => Error::Cloud("invitation_not_found"),
        other => other,
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvitationListWire {
    invitations: Vec<InvitationItem>,
}

#[derive(Deserialize)]
struct AcceptedWire {
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreatedWire {
    id: String,
    #[serde(default)]
    cloud_device_id: Option<String>,
    #[serde(default)]
    cloud_public_key: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListedWire {
    id: String,
    local_library_id: Option<String>,
    #[serde(default)]
    cloud_device_id: Option<String>,
    #[serde(default)]
    cloud_public_key: Option<String>,
}

impl CloudApi for HttpCloud {
    fn server(&self) -> &str {
        &self.base
    }

    async fn create_library(&self) -> Result<CloudLibrary> {
        let created: CreatedWire =
            send_json(&self.client, Method::POST, &self.url(""), self.auth(), None)
                .await
                .map_err(cloud_error)?;
        Ok(CloudLibrary {
            id: created.id,
            local_library_id: None,
            cloud_device_id: created.cloud_device_id,
            cloud_public_key: created.cloud_public_key,
        })
    }

    async fn list_libraries(&self) -> Result<Listing> {
        // Older servers answer with the list alone; newer ones with the list and the ids they cut (D98).
        let answer: Value = send_json(&self.client, Method::GET, &self.url(""), self.auth(), None)
            .await
            .map_err(cloud_error)?;
        let (libraries, withdrawn) = match answer {
            Value::Array(_) => (answer, Value::Array(Vec::new())),
            Value::Object(mut object) => (
                object
                    .remove("libraries")
                    .unwrap_or(Value::Array(Vec::new())),
                object
                    .remove("withdrawn")
                    .unwrap_or(Value::Array(Vec::new())),
            ),
            _ => return Err(Error::account("server_unavailable")),
        };
        let listed: Vec<ListedWire> =
            serde_json::from_value(libraries).map_err(|_| Error::account("server_unavailable"))?;
        Ok(Listing {
            libraries: listed
                .into_iter()
                .map(|wire| CloudLibrary {
                    id: wire.id,
                    local_library_id: wire.local_library_id,
                    cloud_device_id: wire.cloud_device_id,
                    cloud_public_key: wire.cloud_public_key,
                })
                .collect(),
            withdrawn: serde_json::from_value(withdrawn).unwrap_or_default(),
        })
    }

    async fn claim(&self) -> Result<Claimed> {
        send_json(
            &self.client,
            Method::POST,
            &self.url("/claim"),
            self.auth(),
            None,
        )
        .await
        .map_err(cloud_error)
    }

    async fn pull(
        &self,
        cloud: &str,
        schema_version: i64,
        vector: &VersionVector,
    ) -> Result<Pulled> {
        let body = json!({ "schemaVersion": schema_version, "vector": vector });
        send_json(
            &self.client,
            Method::POST,
            &self.url(&format!("/{cloud}/pull")),
            self.auth(),
            Some(&body),
        )
        .await
        .map_err(cloud_error)
    }

    async fn link(&self, cloud: &str, link: &SyncOperation) -> Result<()> {
        let body = serde_json::to_value(link).map_err(|_| Error::Cloud("invalid_request"))?;
        let _linked: Value = send_json(
            &self.client,
            Method::POST,
            &self.url(&format!("/{cloud}/link")),
            self.auth(),
            Some(&body),
        )
        .await
        .map_err(cloud_error)?;
        Ok(())
    }

    async fn push(
        &self,
        cloud: &str,
        schema_version: i64,
        operations: &[SyncOperation],
    ) -> Result<PushReport> {
        let mut total = PushReport::default();
        for batch in operations.chunks(PUSH_BATCH) {
            let body = json!({ "schemaVersion": schema_version, "operations": batch });
            let report: PushReport = send_json(
                &self.client,
                Method::POST,
                &self.url(&format!("/{cloud}/push")),
                self.auth(),
                Some(&body),
            )
            .await
            .map_err(cloud_error)?;
            total.received += report.received;
            total.known += report.known;
            total.rejected += report.rejected;
        }
        Ok(total)
    }

    async fn offers_invitations(&self) -> Result<bool> {
        Ok(account::client::server_info(&self.client, &self.base)
            .await?
            .invitations)
    }

    async fn invitation_base(&self) -> Result<Option<String>> {
        Ok(account::client::server_info(&self.client, &self.base)
            .await?
            .invitation_base)
    }

    async fn offers_metadata(&self, provider: &str) -> Result<bool> {
        Ok(account::client::server_info(&self.client, &self.base)
            .await?
            .metadata
            .iter()
            .any(|offered| offered == provider))
    }

    async fn google_books_isbn(&self, isbn: &Isbn) -> Result<Option<Metadata>> {
        let mut response = send(
            &self.client,
            Method::GET,
            &format!(
                "{}/v1/metadata/google-books/isbn/{}",
                self.base,
                isbn.as_str()
            ),
            self.auth(),
            None,
        )
        .await?;
        // The fixed shape is small: a server that sends more than this is not sending that.
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| Error::account("server_unavailable"))?
        {
            if body.len() + chunk.len() > MAX_BOOK_ANSWER {
                return Err(Error::account("server_unavailable"));
            }
            body.extend_from_slice(&chunk);
        }
        let answer: Value =
            serde_json::from_slice(&body).map_err(|_| Error::account("server_unavailable"))?;
        parse_answer(&answer).map_err(|()| Error::account("server_unavailable"))
    }

    async fn create_invitation(
        &self,
        cloud: &str,
        member: &str,
        kind: InvitationKind,
        email: Option<&str>,
    ) -> Result<CreatedInvitation> {
        let mut body = json!({ "memberId": member, "kind": kind });
        if let Some(email) = email {
            body["email"] = json!(email);
        }
        send_json(
            &self.client,
            Method::POST,
            &self.url(&format!("/{cloud}/invitations")),
            self.auth(),
            Some(&body),
        )
        .await
        .map_err(cloud_error)
    }

    async fn list_invitations(&self, cloud: &str) -> Result<Vec<InvitationItem>> {
        let answer: InvitationListWire = send_json(
            &self.client,
            Method::GET,
            &self.url(&format!("/{cloud}/invitations")),
            self.auth(),
            None,
        )
        .await
        .map_err(cloud_error)?;
        Ok(answer.invitations)
    }

    async fn revoke_invitation(&self, cloud: &str, invitation: &str) -> Result<()> {
        send_empty(
            &self.client,
            Method::DELETE,
            &self.url(&format!("/{cloud}/invitations/{invitation}")),
            self.auth(),
            None,
        )
        .await
        .map_err(cloud_error)
    }

    async fn peek_invitation(&self, token: &str) -> Result<InvitationPreview> {
        // No session on purpose: whoever opens a link has not necessarily got an account yet.
        send_json(
            &self.client,
            Method::GET,
            &self.invitations_url(&format!("/{token}")),
            None,
            None,
        )
        .await
        .map_err(invitation_error)
    }

    async fn accept_invitation(&self, token: &str) -> Result<String> {
        let accepted: AcceptedWire = send_json(
            &self.client,
            Method::POST,
            &self.invitations_url(&format!("/{token}/accept")),
            self.auth(),
            None,
        )
        .await
        .map_err(invitation_error)?;
        Ok(accepted.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_servers_own_codes_become_cloud_errors() {
        assert!(matches!(
            cloud_error(Error::account("not_found")),
            Error::Cloud("not_found")
        ));
        assert!(matches!(
            cloud_error(Error::account("schema_version")),
            Error::Cloud("schema_version")
        ));
    }

    // --- Google Books through the server (7.8) -------------------------------------------------------------------

    use std::sync::{Arc, Mutex};

    use axum::Router;
    use axum::extract::{Path, State};
    use axum::http::{HeaderMap, StatusCode};
    use axum::response::IntoResponse;
    use axum::routing::get;
    use tokio::net::TcpListener;

    /// The path and the session headers of one request to the book route.
    type BookAsk = (String, Option<String>, Option<String>);

    #[derive(Clone)]
    struct Book {
        /// `ok`, `huge`, `text`, `missing`, `quota`.
        mode: &'static str,
        asked: Arc<Mutex<Vec<BookAsk>>>,
        /// What `GET /v1/server` says under `metadata`.
        offered: Value,
    }

    async fn book_route(
        State(stub): State<Book>,
        Path(isbn): Path<String>,
        headers: HeaderMap,
    ) -> axum::response::Response {
        let header = |name: &str| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
        };
        stub.asked
            .lock()
            .unwrap()
            .push((isbn, header("x-device-id"), header("authorization")));
        match stub.mode {
            "huge" => axum::Json(json!({
                "found": true, "title": "Inferno", "authors": [], "padding": "x".repeat(5_000_000),
            }))
            .into_response(),
            "text" => (StatusCode::OK, "<html>hello</html>").into_response(),
            "missing" => axum::Json(json!({ "found": false })).into_response(),
            "quota" => (
                StatusCode::SERVICE_UNAVAILABLE,
                axum::Json(json!({ "error": "metadata_quota_exhausted" })),
            )
                .into_response(),
            _ => axum::Json(json!({
                "found": true, "title": "Inferno", "authors": ["Dan Brown"], "extra": 1,
            }))
            .into_response(),
        }
    }

    async fn server_route(State(stub): State<Book>) -> axum::Json<Value> {
        axum::Json(json!({
            "name": "Kosmotheka", "api_version": 1, "registration": true,
            "sign_in_with_google": false, "two_factor": true, "invitations": true,
            "metadata": stub.offered,
        }))
    }

    async fn book_server(mode: &'static str, offered: Value) -> (HttpCloud, Book) {
        let stub = Book {
            mode,
            asked: Arc::default(),
            offered,
        };
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new()
            .route("/v1/server", get(server_route))
            .route("/v1/metadata/google-books/isbn/{isbn}", get(book_route))
            .with_state(stub.clone());
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (HttpCloud::for_test(&format!("http://{address}")), stub)
    }

    fn the_isbn() -> Isbn {
        Isbn::parse("978-84-08-17603-9").unwrap()
    }

    #[tokio::test]
    async fn the_book_route_gets_the_isbn_and_the_session_and_nothing_else() {
        let (api, stub) = book_server("ok", json!(["google_books"])).await;

        let found = api.google_books_isbn(&the_isbn()).await.unwrap().unwrap();

        assert_eq!(found.title.as_deref(), Some("Inferno"));
        assert_eq!(
            *stub.asked.lock().unwrap(),
            vec![(
                "9788408176039".to_owned(),
                Some("device-1".to_owned()),
                Some("Bearer token-1".to_owned())
            )]
        );
    }

    #[tokio::test]
    async fn a_server_that_says_no_book_is_none() {
        let (api, _) = book_server("missing", json!(["google_books"])).await;

        assert_eq!(api.google_books_isbn(&the_isbn()).await.unwrap(), None);
    }

    #[tokio::test]
    async fn an_enormous_answer_is_a_failure_not_a_form_full_of_rubbish() {
        let (api, _) = book_server("huge", json!(["google_books"])).await;

        assert!(api.google_books_isbn(&the_isbn()).await.is_err());
    }

    #[tokio::test]
    async fn an_answer_that_is_not_json_is_a_failure() {
        let (api, _) = book_server("text", json!(["google_books"])).await;

        assert!(api.google_books_isbn(&the_isbn()).await.is_err());
    }

    #[tokio::test]
    async fn the_servers_refusals_are_failures_with_its_code() {
        let (api, _) = book_server("quota", json!(["google_books"])).await;

        let error = api.google_books_isbn(&the_isbn()).await.unwrap_err();

        assert!(matches!(error, Error::Account { code, .. } if code == "metadata_quota_exhausted"));
    }

    #[tokio::test]
    async fn the_offer_is_read_from_the_servers_list() {
        let (offering, _) = book_server("ok", json!(["google_books"])).await;
        let (other, _) = book_server("ok", json!(["open_library"])).await;
        let (none, _) = book_server("ok", json!([])).await;

        assert!(offering.offers_metadata("google_books").await.unwrap());
        assert!(!other.offers_metadata("google_books").await.unwrap());
        assert!(!none.offers_metadata("google_books").await.unwrap());
    }

    #[tokio::test]
    async fn a_list_of_the_wrong_type_offers_nothing_and_breaks_nothing_else() {
        for offered in [
            json!(5),
            json!("google_books"),
            json!({ "google_books": true }),
            json!(null),
        ] {
            let (api, _) = book_server("ok", offered.clone()).await;

            assert!(
                !api.offers_metadata("google_books").await.unwrap(),
                "{offered}"
            );
            assert!(api.offers_invitations().await.unwrap(), "{offered}");
        }
    }

    #[tokio::test]
    async fn only_the_strings_of_a_mixed_list_count() {
        let (api, _) = book_server("ok", json!([1, null, ["x"], "google_books"])).await;

        assert!(api.offers_metadata("google_books").await.unwrap());
    }

    #[test]
    fn other_errors_keep_the_account_mapping() {
        assert!(matches!(
            cloud_error(Error::account("device_removed")),
            Error::Account { code, .. } if code == "device_removed"
        ));
        assert!(matches!(
            cloud_error(Error::account("server_unavailable")),
            Error::Account { code, .. } if code == "server_unavailable"
        ));
    }
}
