//! The real HTTP client for the cloud routes (docs/architecture/cloud.md), on top of
//! the account client: same `reqwest` client, same device-token headers.

use reqwest::Method;
use serde::Deserialize;
use serde_json::{Value, json};

use super::invitations::{CreatedInvitation, InvitationItem, InvitationKind, InvitationPreview};
use super::{Claimed, CloudApi, CloudLibrary, Listing, Pulled, PushReport};
use crate::account::{
    self,
    client::{Auth, build_client, send_empty, send_json},
};
use crate::db::Database;
use crate::error::{Error, Result};
use crate::sync::{SyncOperation, VersionVector};

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
