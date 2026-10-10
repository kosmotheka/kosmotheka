//! The real HTTP client for the cloud routes (docs/architecture/cloud.md), on top of
//! the account client: same `reqwest` client, same device-token headers.

use reqwest::Method;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{Claimed, CloudApi, CloudLibrary, Listing, Pulled, PushReport};
use crate::account::{self, client::Auth, client::build_client, client::send_json};
use crate::db::Database;
use crate::error::{Error, Result};
use crate::sync::{SyncOperation, VersionVector};

/// The server accepts at most this many operations per push.
const PUSH_BATCH: usize = 1000;

pub struct HttpCloud {
    client: reqwest::Client,
    base: String,
    device_id: String,
    token: String,
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
            device_id,
            token,
        })
    }

    fn auth(&self) -> Option<Auth<'_>> {
        Some(Auth {
            device_id: &self.device_id,
            token: &self.token,
        })
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
