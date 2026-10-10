//! The cloud client against an in-memory cloud (no network). Devices are in-memory
//! databases, like the simulation in `sync.rs`.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use kosmotheka_domain::{
    Change, CopyInput, DeviceKey, EditionInput, Entity, Format, Hlc, Operation, ValidBook,
};
use serde_json::{Map, Value, json};

use super::*;
use crate::catalog;

// --- The fake cloud --------------------------------------------------------

/// The cloud's own device in a fake library: a key pair, and the id that key gives (D95).
struct FakeIdentity {
    seed: u8,
    id: String,
    public_key: String,
}

impl FakeIdentity {
    fn new(seed: u8) -> Self {
        let public_key = DeviceKey::from_seed([seed; 32]).public_key();
        Self {
            seed,
            id: kosmotheka_domain::cloud_device_id(&public_key).unwrap(),
            public_key,
        }
    }
}

struct FakeLibrary {
    owner: String,
    local: Option<String>,
    operations: BTreeMap<String, SyncOperation>,
    identity: Option<FakeIdentity>,
    /// Whether the cloud has registered itself in the log.
    registered: bool,
}

#[derive(Default)]
struct CloudState {
    libraries: BTreeMap<String, FakeLibrary>,
    created: usize,
    pulls: usize,
    pushes: usize,
    links: usize,
    /// How many operations one push request carries, as the real client cuts them;
    /// zero for all at once.
    batch: usize,
    /// A server with no secret: its libraries get no cloud device.
    without_identity: bool,
    /// A server that names a cloud device whose id is not the one its key gives.
    lying: bool,
}

/// Enforces only what the client is tested against: membership by account,
/// operations unique per cloud library, a cloud library that takes only the
/// local id its link operation names, and an operation that waits for the previous
/// one of its author (D87).
#[derive(Clone, Default)]
struct FakeCloud(Arc<Mutex<CloudState>>);

struct FakeApi {
    cloud: FakeCloud,
    account: String,
    server: String,
}

impl FakeCloud {
    fn api(&self, account: &str) -> FakeApi {
        self.api_on(account, "https://cloud.example")
    }

    fn api_on(&self, account: &str, server: &str) -> FakeApi {
        FakeApi {
            cloud: self.clone(),
            account: account.to_owned(),
            server: server.to_owned(),
        }
    }

    fn created(&self) -> usize {
        self.0.lock().unwrap().created
    }

    fn links(&self) -> usize {
        self.0.lock().unwrap().links
    }

    /// A server with an empty secret (D18): it gives its libraries no cloud device.
    fn without_identity(self) -> Self {
        self.0.lock().unwrap().without_identity = true;
        self
    }

    /// A server that lies: the id it gives its cloud device does not derive from the key it gives.
    fn lying(self) -> Self {
        self.0.lock().unwrap().lying = true;
        self
    }

    fn identity_of(&self, cloud: &str) -> Option<(String, String)> {
        let state = self.0.lock().unwrap();
        let identity = state.libraries[cloud].identity.as_ref()?;
        Some((identity.id.clone(), identity.public_key.clone()))
    }

    fn push_in_batches_of(&self, batch: usize) {
        self.0.lock().unwrap().batch = batch;
    }

    fn calls(&self) -> (usize, usize) {
        let state = self.0.lock().unwrap();
        (state.pulls, state.pushes)
    }

    /// The ids the devices wrote; the cloud's own operations (its registration) are not among them.
    fn operation_ids(&self, cloud: &str) -> Vec<String> {
        let state = self.0.lock().unwrap();
        let library = &state.libraries[cloud];
        let cloud_device = library
            .identity
            .as_ref()
            .map(|identity| identity.id.as_str());
        library
            .operations
            .iter()
            .filter(|(_, synced)| Some(synced.operation.hlc.device.as_str()) != cloud_device)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// The operations the cloud signed itself, oldest first.
    fn cloud_operations(&self, cloud: &str) -> Vec<Operation> {
        let state = self.0.lock().unwrap();
        let library = &state.libraries[cloud];
        let Some(identity) = &library.identity else {
            return Vec::new();
        };
        let mut own: Vec<Operation> = library
            .operations
            .values()
            .filter(|synced| synced.operation.hlc.device == identity.id)
            .map(|synced| synced.operation.clone())
            .collect();
        own.sort_by(|a, b| a.hlc.cmp(&b.hlc));
        own
    }

    fn only_library(&self) -> String {
        let state = self.0.lock().unwrap();
        assert_eq!(state.libraries.len(), 1);
        state.libraries.keys().next().unwrap().clone()
    }

    /// What a hostile or broken server could do: store anything, unchecked.
    fn inject(&self, cloud: &str, operations: Vec<SyncOperation>) {
        let mut state = self.0.lock().unwrap();
        let library = state.libraries.get_mut(cloud).unwrap();
        for synced in operations {
            library
                .operations
                .insert(synced.operation.id.clone(), synced);
        }
    }
}

impl FakeLibrary {
    /// What the real server does at the end of the push that names its cloud device (D95): the device
    /// registers itself, signed with its own key, with the newest HLC of the log.
    fn register_the_cloud(&mut self, server: &str) {
        let Some(identity) = self.identity.as_ref().filter(|_| !self.registered) else {
            return;
        };
        let names_it = self.operations.values().any(|synced| {
            synced.operation.changes.iter().any(|change| {
                change.entity == Entity::Library
                    && change.fields.get("cloud_device_id").and_then(Value::as_str)
                        == Some(identity.id.as_str())
            })
        });
        let (Some(local), true) = (self.local.clone(), names_it) else {
            return;
        };
        let newest = self
            .operations
            .values()
            .map(|synced| synced.operation.hlc.millis)
            .max()
            .unwrap_or_default();
        let schema_version = self.operations.values().next().unwrap().schema_version;
        let mut operation = Operation {
            id: ops::new_id(),
            library: local,
            hlc: Hlc {
                millis: newest + 1,
                counter: 0,
                device: identity.id.clone(),
            },
            intent: "device.register".into(),
            changes: vec![Change::create(
                Entity::Device,
                &identity.id,
                json!({ "public_key": identity.public_key, "name": server.trim_start_matches("https://") }),
            )],
            previous: None,
            signature: None,
        };
        operation.signature = Some(DeviceKey::from_seed([identity.seed; 32]).sign(&operation));
        self.operations.insert(
            operation.id.clone(),
            SyncOperation {
                schema_version,
                operation,
            },
        );
        self.registered = true;
    }
}

fn is_link_to(synced: &SyncOperation, cloud: &str) -> bool {
    synced.operation.changes.iter().any(|change| {
        change.entity == Entity::Library
            && change
                .fields
                .get("cloud_library_id")
                .and_then(Value::as_str)
                == Some(cloud)
    })
}

impl CloudApi for FakeApi {
    fn server(&self) -> &str {
        &self.server
    }

    async fn create_library(&self) -> Result<CloudLibrary> {
        let mut state = self.cloud.0.lock().unwrap();
        let id = ops::new_id();
        state.created += 1;
        let mut identity = (!state.without_identity)
            .then(|| FakeIdentity::new(100 + u8::try_from(state.created).unwrap()));
        if let Some(identity) = identity.as_mut().filter(|_| state.lying) {
            identity.id = ops::new_id();
        }
        let listed = CloudLibrary {
            id: id.clone(),
            local_library_id: None,
            cloud_device_id: identity.as_ref().map(|identity| identity.id.clone()),
            cloud_public_key: identity
                .as_ref()
                .map(|identity| identity.public_key.clone()),
        };
        state.libraries.insert(
            id,
            FakeLibrary {
                owner: self.account.clone(),
                local: None,
                operations: BTreeMap::new(),
                identity,
                registered: false,
            },
        );
        Ok(listed)
    }

    async fn list_libraries(&self) -> Result<Vec<CloudLibrary>> {
        let state = self.cloud.0.lock().unwrap();
        Ok(state
            .libraries
            .iter()
            .filter(|(_, library)| library.owner == self.account)
            .map(|(id, library)| CloudLibrary {
                id: id.clone(),
                local_library_id: library.local.clone(),
                cloud_device_id: library
                    .identity
                    .as_ref()
                    .map(|identity| identity.id.clone()),
                cloud_public_key: library
                    .identity
                    .as_ref()
                    .map(|identity| identity.public_key.clone()),
            })
            .collect())
    }

    async fn pull(
        &self,
        cloud: &str,
        _schema_version: i64,
        vector: &VersionVector,
    ) -> Result<Pulled> {
        let mut state = self.cloud.0.lock().unwrap();
        state.pulls += 1;
        let library = state
            .libraries
            .get(cloud)
            .filter(|library| library.owner == self.account)
            .ok_or(Error::Cloud("not_found"))?;
        let mut operations: Vec<SyncOperation> = library
            .operations
            .values()
            .filter(|synced| {
                let hlc = synced.operation.hlc.to_string();
                vector
                    .get(&synced.operation.hlc.device)
                    .is_none_or(|seen| hlc > *seen)
            })
            .cloned()
            .collect();
        operations.sort_by(|a, b| a.operation.hlc.cmp(&b.operation.hlc));
        let mut own = VersionVector::new();
        for synced in library.operations.values() {
            let slot = own.entry(synced.operation.hlc.device.clone()).or_default();
            *slot = (*slot).clone().max(synced.operation.hlc.to_string());
        }
        Ok(Pulled {
            operations,
            vector: own,
        })
    }

    async fn link(&self, cloud: &str, link: &SyncOperation) -> Result<()> {
        let mut state = self.cloud.0.lock().unwrap();
        state.links += 1;
        let library = state
            .libraries
            .get_mut(cloud)
            .filter(|library| library.owner == self.account)
            .ok_or(Error::Cloud("not_found"))?;
        if !is_link_to(link, cloud) {
            return Err(Error::Cloud("invalid_request"));
        }
        match &library.local {
            Some(local) if *local != link.operation.library => Err(Error::Cloud("already_linked")),
            _ => {
                library.local = Some(link.operation.library.clone());
                Ok(())
            }
        }
    }

    async fn push(
        &self,
        cloud: &str,
        _schema_version: i64,
        operations: &[SyncOperation],
    ) -> Result<PushReport> {
        let mut state = self.cloud.0.lock().unwrap();
        state.pushes += 1;
        let batch = state.batch;
        let library = state
            .libraries
            .get_mut(cloud)
            .filter(|library| library.owner == self.account)
            .ok_or(Error::Cloud("not_found"))?;
        let mut report = PushReport::default();
        let size = if batch == 0 { usize::MAX } else { batch };
        for request in operations.chunks(size) {
            if library.local.is_none()
                && let Some(link) = request.iter().find(|synced| is_link_to(synced, cloud))
            {
                library.local = Some(link.operation.library.clone());
            }
            for synced in request {
                let operation = &synced.operation;
                let follows = operation
                    .previous
                    .as_ref()
                    .is_none_or(|previous| library.operations.contains_key(previous));
                if library.operations.contains_key(&operation.id) {
                    report.known += 1;
                } else if follows && library.local.as_deref() == Some(operation.library.as_str()) {
                    library
                        .operations
                        .insert(operation.id.clone(), synced.clone());
                    report.received += 1;
                } else {
                    report.rejected += 1;
                }
            }
        }
        library.register_the_cloud(&self.server);
        Ok(report)
    }
}

// --- Devices and helpers ---------------------------------------------------

fn edition(title: &str) -> ValidBook {
    ValidBook {
        edition: EditionInput {
            title: title.into(),
            authors: vec!["Ursula K. Le Guin".into()],
            ..EditionInput::default()
        }
        .validate()
        .unwrap(),
        copy: CopyInput {
            owner: None,
            format: Format::Paperback,
            quantity: 1,
            location: None,
            acquired_on: None,
            platform: None,
            seasons: None,
        }
        .validate()
        .unwrap(),
    }
}

fn add(device: &Database, library: &str, title: &str) {
    catalog::create_book(&mut device.lock().unwrap(), library, &edition(title), None).unwrap();
}

/// The library a fresh device was born with (D85): random id, nothing in it yet.
fn first_library(device: &Database) -> String {
    ops::active_library(&device.lock().unwrap()).unwrap()
}

/// Creates a library with a known id on this device. Two devices that never synced
/// having the same id stands for a copied or hostile one: libraries are otherwise
/// created with a random id on one device and travel to the others.
fn create_with_id(device: &Database, id: &str) {
    let mut conn = device.lock().unwrap();
    let tx = conn.transaction().unwrap();
    libraries::create_with_id(&tx, id, "").unwrap();
    tx.commit().unwrap();
}

/// A device with a new library of its own (random id) holding one book.
fn device_with_family() -> (Database, String) {
    let device = Database::in_memory();
    let library = libraries::create_library(&mut device.lock().unwrap(), "Family").unwrap();
    add(&device, &library, "A Wizard of Earthsea");
    (device, library)
}

type State = Vec<(String, Vec<Map<String, Value>>)>;

fn state(device: &Database, library: &str) -> State {
    let conn = device.lock().unwrap();
    let mut state = Vec::new();
    for entity in sync::ORDER {
        let rows = sync::entity_rows(&conn, library, entity).unwrap();
        if !rows.is_empty() {
            state.push((entity.table().to_owned(), rows));
        }
    }
    state
}

fn log_ids(device: &Database, library: &str) -> Vec<String> {
    let conn = device.lock().unwrap();
    let mut ids: Vec<String> = sync::library_log(&conn, library)
        .unwrap()
        .into_iter()
        .map(|operation| operation.id)
        .collect();
    ids.sort();
    ids
}

/// Same rows, same log, and rebuilding the library from its log changes nothing.
fn assert_converged(devices: &[&Database], library: &str) {
    let first = (state(devices[0], library), log_ids(devices[0], library));
    for other in &devices[1..] {
        assert_eq!(
            (state(other, library), log_ids(other, library)),
            first,
            "devices diverged"
        );
    }
    for device in devices {
        let before = state(device, library);
        let conn = device.lock().unwrap();
        let tx = conn.unchecked_transaction().unwrap();
        crate::permissions::rebuild(&tx, library).unwrap();
        tx.rollback().unwrap();
        drop(conn);
        assert_eq!(
            state(device, library),
            before,
            "rebuild changed the library"
        );
    }
}

fn pull_lan(to: &Database, from: &Database, libraries: &[String]) {
    let mine = sync::version_vectors(&to.lock().unwrap(), libraries).unwrap();
    let missing = sync::operations_since(&from.lock().unwrap(), &mine).unwrap();
    sync::merge(&mut to.lock().unwrap(), missing).unwrap();
}

/// What a LAN session does for the shared libraries: both ways.
fn lan_sync(a: &Database, b: &Database, library: &str) {
    let libraries = [library.to_owned()];
    pull_lan(a, b, &libraries);
    pull_lan(b, a, &libraries);
}

fn link(device: &Database, library: &str) -> (Option<String>, Option<String>) {
    let link = link_of(&device.lock().unwrap(), library).unwrap().unwrap();
    (link.cloud_library, link.cloud_server)
}

fn unwrap_err<T: std::fmt::Debug>(result: Result<T>) -> Error {
    result.expect_err("expected an error")
}

/// Uploads, and runs the session after it: the cloud registers itself at the end of the push that names it
/// (D95), and the uploader hears of it in the next one.
async fn upload_settled<A: CloudApi>(device: &Database, api: &A, library: &str) {
    upload(device, api, library).await.unwrap();
    sync_library(device, api, library).await.unwrap();
}

// --- Tests -----------------------------------------------------------------

#[tokio::test]
async fn uploading_links_the_library_and_sends_its_whole_log() {
    let cloud = FakeCloud::default();
    let api = cloud.api("ana");
    let (ana, library) = device_with_family();

    let report = upload(&ana, &api, &library).await.unwrap();

    let c = cloud.only_library();
    assert_eq!(
        link(&ana, &library),
        (Some(c.clone()), Some(api.server.clone()))
    );
    let log = sync::library_log(&ana.lock().unwrap(), &library).unwrap();
    let links: Vec<_> = log
        .iter()
        .filter(|op| op.intent == "library.cloud")
        .collect();
    assert_eq!(links.len(), 1);
    assert_eq!(
        links[0].changes,
        vec![Change::update(
            Entity::Library,
            &library,
            json!({ "cloud_library_id": c, "cloud_server": api.server, "cloud_device_id": cloud.identity_of(&c).unwrap().0 })
        )]
    );
    assert_eq!(cloud.operation_ids(&c), log_ids(&ana, &library));
    assert_eq!(
        (report.outcome, report.sent, report.rejected),
        (Outcome::Synced, log.len(), 0)
    );
}

#[tokio::test]
async fn a_long_log_is_pushed_in_an_order_where_every_previous_goes_first() {
    // The client pushes the log in HLC order and cuts it into consecutive batches
    // of PUSH_BATCH, so a batch boundary never separates an operation from the
    // previous one of its author that the cloud has yet to store (D87).
    let cloud = FakeCloud::default();
    let api = cloud.api("ana");
    let (ana, library) = device_with_family();
    for n in 0..300 {
        add(&ana, &library, &format!("Book {n}"));
    }

    let report = upload(&ana, &api, &library).await.unwrap();

    let log = sync::library_log(&ana.lock().unwrap(), &library).unwrap();
    assert!(log.len() > 300);
    let mut seen = std::collections::BTreeSet::new();
    for operation in &log {
        if let Some(previous) = &operation.previous {
            assert!(
                seen.contains(previous),
                "{} comes before its previous",
                operation.id
            );
        }
        seen.insert(operation.id.clone());
    }
    assert_eq!((report.sent, report.rejected), (log.len(), 0));
}

#[tokio::test]
async fn a_library_bigger_than_one_push_goes_up_whole_the_first_time() {
    // The link is the newest operation, so it travels in the last request, and the
    // cloud takes nothing before the link: the client links first, on its own call.
    let cloud = FakeCloud::default();
    cloud.push_in_batches_of(100);
    let api = cloud.api("ana");
    let (ana, library) = device_with_family();
    for n in 0..300 {
        add(&ana, &library, &format!("Book {n}"));
    }

    let report = upload(&ana, &api, &library).await.unwrap();

    let c = cloud.only_library();
    let log = sync::library_log(&ana.lock().unwrap(), &library).unwrap();
    assert_eq!((report.sent, report.rejected), (log.len(), 0));
    assert_eq!(cloud.operation_ids(&c), log_ids(&ana, &library));
    assert_eq!(cloud.links(), 1);
}

#[tokio::test]
async fn a_library_already_in_the_cloud_is_not_linked_again() {
    let cloud = FakeCloud::default();
    let api = cloud.api("ana");
    let (ana, library) = device_with_family();
    upload(&ana, &api, &library).await.unwrap();

    add(&ana, &library, "The Tombs of Atuan");
    sync_library(&ana, &api, &library).await.unwrap();

    assert_eq!(cloud.links(), 1);
}

#[tokio::test]
async fn uploading_twice_is_refused() {
    let cloud = FakeCloud::default();
    let api = cloud.api("ana");
    let (ana, library) = device_with_family();
    upload(&ana, &api, &library).await.unwrap();

    let error = unwrap_err(upload(&ana, &api, &library).await);

    assert!(matches!(error, Error::Cloud("already_linked")), "{error:?}");
    assert_eq!(cloud.created(), 1);
}

#[tokio::test]
async fn an_unsigned_history_is_not_uploaded() {
    let cloud = FakeCloud::default();
    let api = cloud.api("ana");
    let (ana, library) = device_with_family();
    ana.lock()
        .unwrap()
        .execute(
            "UPDATE operations SET signature = NULL WHERE library_id = ?1 AND intent = 'book.add'",
            [&library],
        )
        .unwrap();
    let before = log_ids(&ana, &library);

    let error = unwrap_err(upload(&ana, &api, &library).await);

    assert!(
        matches!(error, Error::Cloud("unsigned_history")),
        "{error:?}"
    );
    assert_eq!(cloud.created(), 0);
    assert_eq!(log_ids(&ana, &library), before);
    assert_eq!(link(&ana, &library), (None, None));
}

/// The laptop is Tersoal's (administrator), the phone Hugo's (reader), in the
/// laptop's first library, which the phone receives over the LAN.
fn laptop_admin_phone_reader() -> (Database, Database, String) {
    let laptop = Database::in_memory();
    let phone = Database::in_memory();
    let lib = first_library(&laptop);
    add(&laptop, &lib, "Dune");
    lan_sync(&laptop, &phone, &lib);
    add(&phone, &lib, "Tehanu");
    lan_sync(&laptop, &phone, &lib);
    let laptop_id = ops::device_id(&laptop.lock().unwrap()).unwrap();
    let phone_id = ops::device_id(&phone.lock().unwrap()).unwrap();
    let (admin, reader) = (ops::new_id(), ops::new_id());
    {
        let mut conn = laptop.lock().unwrap();
        let tx = conn.transaction().unwrap();
        ops::record(
            &tx,
            &lib,
            "member.roles",
            vec![
                Change::create(
                    Entity::Member,
                    &admin,
                    json!({ "name": "Tersoal", "role": "admin" }),
                ),
                Change::create(
                    Entity::Member,
                    &reader,
                    json!({ "name": "Hugo", "role": "reader" }),
                ),
                Change::update(Entity::Device, &laptop_id, json!({ "member_id": admin })),
                Change::update(Entity::Device, &phone_id, json!({ "member_id": reader })),
            ],
        )
        .unwrap();
        tx.commit().unwrap();
    }
    lan_sync(&laptop, &phone, &lib);
    (laptop, phone, lib)
}

#[tokio::test]
async fn a_session_with_the_cloud_seals_a_library_that_had_roles_and_no_seal_and_sends_the_seal() {
    let cloud = FakeCloud::default();
    let api = cloud.api("tersoal");
    let (laptop, phone, lib) = laptop_admin_phone_reader();
    let seals = |device: &Database| -> Vec<String> {
        device
            .lock()
            .unwrap()
            .prepare("SELECT id FROM operations WHERE library_id = ?1 AND intent = 'library.seal'")
            .unwrap()
            .query_map([&lib], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    };
    upload(&laptop, &api, &lib).await.unwrap();

    sync_library(&laptop, &api, &lib).await.unwrap();
    sync_library(&laptop, &api, &lib).await.unwrap();

    let sealed = seals(&laptop);
    assert_eq!(sealed.len(), 1, "once, and not again in the next session");
    assert!(
        cloud
            .operation_ids(&cloud.only_library())
            .contains(&sealed[0])
    );
    // The reader's device has sessions too, and seals nothing.
    lan_sync(&laptop, &phone, &lib);
    assert_eq!(seals(&phone), sealed);
    assert_converged(&[&laptop, &phone], &lib);
}

#[tokio::test]
async fn a_reader_cannot_upload_but_the_administrator_can() {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib) = laptop_admin_phone_reader();
    let before = log_ids(&phone, &lib);

    let error = unwrap_err(upload(&phone, &cloud.api("hugo"), &lib).await);

    assert!(matches!(error, Error::NotAllowed), "{error:?}");
    assert_eq!(cloud.created(), 0);
    assert_eq!(log_ids(&phone, &lib), before);
    assert_eq!(link(&phone, &lib), (None, None));

    upload(&laptop, &cloud.api("tersoal"), &lib).await.unwrap();
    assert_eq!(cloud.created(), 1);
}

/// CON-009 (D76): «Can share» is for the home network; going up to the cloud is for whoever administers.
#[tokio::test]
async fn an_editor_who_may_share_cannot_upload_and_the_server_receives_nothing() {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib) = laptop_admin_phone_reader();
    let phone_id = ops::device_id(&phone.lock().unwrap()).unwrap();
    let hugo: String = laptop
        .lock()
        .unwrap()
        .query_row(
            "SELECT member_id FROM devices WHERE library_id = ?1 AND id = ?2",
            [&lib, &phone_id],
            |row| row.get(0),
        )
        .unwrap();
    people::set_role(
        &mut laptop.lock().unwrap(),
        &lib,
        &hugo,
        Some("editor"),
        true,
    )
    .unwrap();
    lan_sync(&laptop, &phone, &lib);
    let before = log_ids(&phone, &lib);
    // Signed in, so that only the role decides.
    for device in [&laptop, &phone] {
        ops::write_state(&device.lock().unwrap(), "account_token", Some("token")).unwrap();
    }
    let statuses = status(&phone).unwrap();
    let mine = statuses.iter().find(|s| s.library == lib).unwrap();
    assert!(mine.may_share, "he still shares over the home network");
    assert!(!mine.can_upload);

    let error = unwrap_err(upload(&phone, &cloud.api("hugo"), &lib).await);

    assert!(matches!(error, Error::NotAllowed), "{error:?}");
    assert_eq!(cloud.created(), 0, "the server never hears of it");
    assert_eq!(log_ids(&phone, &lib), before);
    assert_eq!(link(&phone, &lib), (None, None));
    let statuses = status(&laptop).unwrap();
    assert!(
        statuses
            .iter()
            .find(|s| s.library == lib)
            .unwrap()
            .can_upload
    );
}

#[tokio::test]
async fn a_second_device_of_the_account_downloads_the_library_and_converges() {
    let cloud = FakeCloud::default();
    let (ana, library) = device_with_family();
    upload_settled(&ana, &cloud.api("ana"), &library).await;
    let phone = Database::in_memory();

    let reports = sync_all(&phone, &cloud.api("ana")).await.unwrap();

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].outcome, Outcome::Downloaded);
    assert_eq!(reports[0].sent, 0);
    assert_converged(&[&ana, &phone], &library);
    assert_eq!(link(&phone, &library), link(&ana, &library));

    // And what the second device adds goes back the same way.
    add(&phone, &library, "The Dispossessed");
    sync_library(&phone, &cloud.api("ana"), &library)
        .await
        .unwrap();
    let reports = sync_all(&ana, &cloud.api("ana")).await.unwrap();
    assert_eq!(reports[0].outcome, Outcome::Synced);
    assert_eq!(reports[0].received, 2); // the book and the phone's key
    assert_converged(&[&ana, &phone], &library);
}

#[tokio::test]
async fn a_library_with_the_same_local_id_that_was_never_linked_is_not_merged() {
    // Two installs have a library under the same id (made on purpose: it stands for
    // a copied or hostile id, since a random one never repeats). One uploads it; the
    // other, in the same account, never linked its own.
    let repeated = ops::new_id();
    let cloud = FakeCloud::default();
    let tanakas = Database::in_memory();
    create_with_id(&tanakas, &repeated);
    add(&tanakas, &repeated, "Dune");
    upload(&tanakas, &cloud.api("ana"), &repeated)
        .await
        .unwrap();
    let jhonny = Database::in_memory();
    create_with_id(&jhonny, &repeated);
    add(&jhonny, &repeated, "The Hobbit");
    let (state_before, log_before) = (state(&jhonny, &repeated), log_ids(&jhonny, &repeated));
    let (_, pushes_before) = cloud.calls();

    let reports = sync_all(&jhonny, &cloud.api("ana")).await.unwrap();

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].outcome, Outcome::LibraryConflict);
    assert_eq!((reports[0].received, reports[0].sent), (0, 0));
    assert_eq!(state(&jhonny, &repeated), state_before);
    assert_eq!(log_ids(&jhonny, &repeated), log_before);
    assert_eq!(link(&jhonny, &repeated), (None, None));
    assert_eq!(cloud.calls().1, pushes_before);
}

#[tokio::test]
async fn a_new_device_of_the_account_downloads_its_first_library() {
    // D85: Kenji's laptop uploads the library it was born with. His new phone was
    // born with another one, of another id, so nothing clashes: it gets his.
    let cloud = FakeCloud::default();
    let laptop = Database::in_memory();
    let library = first_library(&laptop);
    add(&laptop, &library, "Dune");
    upload_settled(&laptop, &cloud.api("kenji"), &library).await;
    let phone = Database::in_memory();
    assert_ne!(first_library(&phone), library);

    let reports = sync_all(&phone, &cloud.api("kenji")).await.unwrap();

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].outcome, Outcome::Downloaded);
    assert!(
        reports
            .iter()
            .all(|report| report.outcome != Outcome::LibraryConflict)
    );
    assert_converged(&[&laptop, &phone], &library);
    assert_eq!(link(&phone, &library), link(&laptop, &library));
}

#[tokio::test]
async fn a_family_device_that_got_the_library_over_the_lan_before_the_upload_joins_it() {
    // The phone had the library from the laptop over the LAN; then the laptop
    // uploads it, and the phone, away from home, only reaches the cloud.
    let cloud = FakeCloud::default();
    let (laptop, library) = device_with_family();
    let phone = Database::in_memory();
    lan_sync(&phone, &laptop, &library);
    add(&phone, &library, "The Dispossessed");
    lan_sync(&phone, &laptop, &library);
    upload(&laptop, &cloud.api("ana"), &library).await.unwrap();
    add(&phone, &library, "Tehanu");

    let reports = sync_all(&phone, &cloud.api("ana")).await.unwrap();

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].outcome, Outcome::Synced);
    assert_eq!(link(&phone, &library), link(&laptop, &library));
    sync_all(&laptop, &cloud.api("ana")).await.unwrap();
    assert_converged(&[&laptop, &phone], &library);
}

#[tokio::test]
async fn adopting_a_link_brings_what_the_author_wrote_before_it() {
    // The laptop edits after the last LAN sync and before uploading: the phone
    // must not jump over those operations when it adopts the link.
    let cloud = FakeCloud::default();
    let (laptop, library) = device_with_family();
    let phone = Database::in_memory();
    lan_sync(&phone, &laptop, &library);
    add(&laptop, &library, "The Dispossessed");
    add(&laptop, &library, "Tehanu");
    upload(&laptop, &cloud.api("ana"), &library).await.unwrap();

    let reports = sync_all(&phone, &cloud.api("ana")).await.unwrap();

    assert_eq!(reports[0].outcome, Outcome::Synced);
    assert_eq!(link(&phone, &library), link(&laptop, &library));
    sync_all(&laptop, &cloud.api("ana")).await.unwrap();
    assert_converged(&[&laptop, &phone], &library);
}

/// A phone that has the family library from the laptop, which uploaded it.
async fn phone_with_uploaded_library() -> (FakeCloud, Database, String) {
    let cloud = FakeCloud::default();
    let (laptop, library) = device_with_family();
    let phone = Database::in_memory();
    lan_sync(&phone, &laptop, &library);
    // What pairing records: the laptop is a paired peer for this library.
    {
        let conn = phone.lock().unwrap();
        conn.execute(
            "INSERT INTO known_servers (device_id, name, addresses, fingerprint, secret)
             VALUES ('laptop', 'Laptop', '[]', 'f', 's')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO library_peers (library_id, device_id) VALUES (?1, 'laptop')",
            [&library],
        )
        .unwrap();
    }
    upload(&laptop, &cloud.api("ana"), &library).await.unwrap();
    (cloud, phone, library)
}

#[tokio::test]
async fn a_library_left_here_is_not_downloaded_again() {
    let (cloud, phone, library) = phone_with_uploaded_library().await;
    libraries::leave_library(&mut phone.lock().unwrap(), &library).unwrap();

    let reports = sync_all(&phone, &cloud.api("ana")).await.unwrap();

    assert_eq!(reports[0].outcome, Outcome::DeletedHere);
    assert!(!libraries::library_exists(&phone.lock().unwrap(), &library).unwrap());
}

#[tokio::test]
async fn a_library_removed_by_a_revocation_is_not_downloaded_again() {
    let (cloud, phone, library) = phone_with_uploaded_library().await;
    libraries::remove_revoked(&mut phone.lock().unwrap(), &library, "laptop").unwrap();

    let reports = sync_all(&phone, &cloud.api("ana")).await.unwrap();

    assert_eq!(reports[0].outcome, Outcome::DeletedHere);
    assert!(!libraries::library_exists(&phone.lock().unwrap(), &library).unwrap());
}

#[tokio::test]
async fn a_link_signed_by_a_reader_of_the_family_is_not_adopted() {
    // Hugo reads. With a modified app he links the family library to a cloud
    // library of his own; his device is registered in the library, but a link is
    // for administrators (D73), so the laptop keeps it as a request, unlinked.
    let cloud = FakeCloud::default();
    let (laptop, phone, lib) = laptop_admin_phone_reader();
    let api = cloud.api("ana");
    let hugos = api.create_library().await.unwrap().id;
    cloud
        .0
        .lock()
        .unwrap()
        .libraries
        .get_mut(&hugos)
        .unwrap()
        .local = Some(lib.clone());
    ops::as_request(None, || {
        let mut conn = phone.lock().unwrap();
        let tx = conn.transaction()?;
        ops::record(
            &tx,
            &lib,
            "library.cloud",
            vec![Change::update(
                Entity::Library,
                &lib,
                json!({ "cloud_library_id": hugos, "cloud_server": api.server() }),
            )],
        )?;
        tx.commit()?;
        Ok(())
    })
    .unwrap();
    let everything = sync::operations_since(
        &phone.lock().unwrap(),
        &LibraryVectors::from([(lib.clone(), VersionVector::new())]),
    )
    .unwrap();
    cloud.inject(&hugos, everything);
    let state_before = state(&laptop, &lib);

    let reports = sync_all(&laptop, &api).await.unwrap();

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].outcome, Outcome::LibraryConflict);
    assert_eq!(link(&laptop, &lib), (None, None));
    assert_eq!(state(&laptop, &lib), state_before);
}

#[tokio::test]
async fn operations_of_another_library_from_the_cloud_are_dropped() {
    let cloud = FakeCloud::default();
    let (ana, library) = device_with_family();
    upload(&ana, &cloud.api("ana"), &library).await.unwrap();
    let c = cloud.only_library();
    // A perfectly valid, signed library of somebody else, planted in `c`.
    let (stranger, other) = device_with_family();
    let planted = sync::operations_since(
        &stranger.lock().unwrap(),
        &LibraryVectors::from([(other.clone(), VersionVector::new())]),
    )
    .unwrap();
    assert!(!planted.is_empty());
    cloud.inject(&c, planted.clone());

    // An existing device that pulls it...
    let report = sync_library(&ana, &cloud.api("ana"), &library)
        .await
        .unwrap();
    assert_eq!(report.rejected, planted.len());
    assert!(!libraries::library_known(&ana.lock().unwrap(), &other).unwrap());

    // ...and a new one that downloads the library.
    let phone = Database::in_memory();
    let reports = sync_all(&phone, &cloud.api("ana")).await.unwrap();
    assert_eq!(reports[0].outcome, Outcome::Downloaded);
    assert_eq!(reports[0].rejected, planted.len());
    assert!(!libraries::library_known(&phone.lock().unwrap(), &other).unwrap());
    assert_converged(&[&ana, &phone], &library);
}

#[tokio::test]
async fn a_library_that_arrives_without_its_link_is_removed_again() {
    // A cloud library whose log lacks the link operation: not a properly linked one.
    let cloud = FakeCloud::default();
    let api = cloud.api("ana");
    let (stranger, other) = device_with_family();
    let c = api.create_library().await.unwrap().id;
    let all = sync::operations_since(
        &stranger.lock().unwrap(),
        &LibraryVectors::from([(other.clone(), VersionVector::new())]),
    )
    .unwrap();
    {
        let mut state = cloud.0.lock().unwrap();
        state.libraries.get_mut(&c).unwrap().local = Some(other.clone());
    }
    cloud.inject(&c, all);
    let phone = Database::in_memory();

    let reports = sync_all(&phone, &api).await.unwrap();

    assert_eq!(reports[0].outcome, Outcome::NotLinked);
    assert!(!libraries::library_known(&phone.lock().unwrap(), &other).unwrap());
}

/// The device edits offline for a «week»; `cloud_first` picks which way it meets
/// the world first.
async fn a_week_offline_then_both_paths(cloud_first: bool) {
    let cloud = FakeCloud::default();
    let (laptop, library) = device_with_family();
    upload_settled(&laptop, &cloud.api("ana"), &library).await;
    // The phone only ever talks over the LAN; the tablet only to the cloud.
    let phone = Database::in_memory();
    lan_sync(&laptop, &phone, &library);
    let tablet = Database::in_memory();
    sync_all(&tablet, &cloud.api("ana")).await.unwrap();
    assert_converged(&[&laptop, &phone, &tablet], &library);

    for title in ["Dune", "Tehanu", "The Dispossessed", "Lavinia"] {
        add(&laptop, &library, title);
    }
    add(&phone, &library, "Gaudy Night");
    add(&tablet, &library, "Kindred");

    if cloud_first {
        sync_library(&laptop, &cloud.api("ana"), &library)
            .await
            .unwrap();
        lan_sync(&laptop, &phone, &library);
    } else {
        lan_sync(&laptop, &phone, &library);
        sync_library(&laptop, &cloud.api("ana"), &library)
            .await
            .unwrap();
    }
    // The tablet's book reached the laptop with the first cloud round (or this one);
    // a second round carries what came over the LAN to the tablet.
    sync_library(&tablet, &cloud.api("ana"), &library)
        .await
        .unwrap();
    sync_library(&laptop, &cloud.api("ana"), &library)
        .await
        .unwrap();
    lan_sync(&laptop, &phone, &library);
    sync_library(&tablet, &cloud.api("ana"), &library)
        .await
        .unwrap();

    assert_converged(&[&laptop, &phone, &tablet], &library);
    // Nothing lost, nothing duplicated: 1 + 4 + 1 + 1 books, and the cloud holds
    // each operation once.
    let titles = state(&laptop, &library)
        .into_iter()
        .find(|(table, _)| table == "editions")
        .unwrap()
        .1
        .len();
    assert_eq!(titles, 7);
    let mut devices_log = log_ids(&laptop, &library);
    for own in cloud.cloud_operations(&cloud.only_library()) {
        devices_log.retain(|id| *id != own.id);
    }
    assert_eq!(cloud.operation_ids(&cloud.only_library()), devices_log);
}

#[tokio::test]
async fn a_week_offline_syncs_with_the_cloud_and_then_a_lan_peer() {
    a_week_offline_then_both_paths(true).await;
}

#[tokio::test]
async fn a_week_offline_syncs_with_a_lan_peer_and_then_the_cloud() {
    a_week_offline_then_both_paths(false).await;
}

#[tokio::test]
async fn a_library_linked_to_another_server_is_skipped() {
    let one = FakeCloud::default();
    let (ana, library) = device_with_family();
    upload(&ana, &one.api_on("ana", "https://one.example"), &library)
        .await
        .unwrap();
    let two = FakeCloud::default();
    let calls_before = (one.calls(), two.calls());
    let api = two.api_on("ana", "https://two.example");

    let report = sync_library(&ana, &api, &library).await.unwrap();
    let reports = sync_all(&ana, &api).await.unwrap();

    assert_eq!(report.outcome, Outcome::OtherServer);
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].outcome, Outcome::OtherServer);
    assert_eq!((one.calls(), two.calls()), calls_before);
}

#[tokio::test]
async fn a_library_the_account_no_longer_lists_is_skipped_and_reported() {
    let cloud = FakeCloud::default();
    let (ana, library) = device_with_family();
    upload(&ana, &cloud.api("ana"), &library).await.unwrap();
    // The same server, but this account no longer sees that cloud library.
    let reports = sync_all(&ana, &cloud.api("somebody-else")).await.unwrap();

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].outcome, Outcome::MissingInCloud);
}

#[tokio::test]
async fn uploading_names_the_cloud_and_its_registration_comes_back_with_its_key() {
    let cloud = FakeCloud::default();
    let api = cloud.api("ana");
    let (ana, library) = device_with_family();

    upload_settled(&ana, &api, &library).await;

    let c = cloud.only_library();
    let (id, key) = cloud.identity_of(&c).unwrap();
    assert_eq!(
        kosmotheka_domain::cloud_device_id(&key).as_deref(),
        Some(id.as_str())
    );
    // The library row names it, and the device row (its registration, signed by the cloud) holds its key.
    let conn = ana.lock().unwrap();
    let named: Option<String> = conn
        .query_row(
            "SELECT cloud_device_id FROM libraries WHERE id = ?1",
            [&library],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(named.as_deref(), Some(id.as_str()));
    let registered: String = conn
        .query_row(
            "SELECT public_key FROM devices WHERE library_id = ?1 AND id = ?2",
            [&library, &id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(registered, key);
    let own = cloud.cloud_operations(&c);
    assert_eq!(own.len(), 1);
    kosmotheka_domain::verify(&own[0], &key).unwrap();
    assert_eq!(
        ops::status_of(&conn, &own[0].id).unwrap(),
        ops::Status::Applied
    );
    // The library trusts it as its cloud: the id and the registered key agree.
    assert_eq!(
        crate::permissions::SqlView::new(&conn, &library).cloud_device_for_tests(),
        Some(id)
    );
}

#[tokio::test]
async fn a_server_with_no_secret_gets_the_upload_it_always_had() {
    let cloud = FakeCloud::default().without_identity();
    let api = cloud.api("ana");
    let (ana, library) = device_with_family();

    upload_settled(&ana, &api, &library).await;

    let c = cloud.only_library();
    let log = sync::library_log(&ana.lock().unwrap(), &library).unwrap();
    let link = log.iter().find(|op| op.intent == "library.cloud").unwrap();
    assert_eq!(
        link.changes,
        vec![Change::update(
            Entity::Library,
            &library,
            json!({ "cloud_library_id": c, "cloud_server": api.server })
        )]
    );
    assert!(cloud.cloud_operations(&c).is_empty());
}

#[tokio::test]
async fn a_server_whose_cloud_id_does_not_come_from_its_key_is_not_named() {
    let cloud = FakeCloud::default().lying();
    let api = cloud.api("ana");
    let (ana, library) = device_with_family();

    upload_settled(&ana, &api, &library).await;

    let conn = ana.lock().unwrap();
    let named: Option<String> = conn
        .query_row(
            "SELECT cloud_device_id FROM libraries WHERE id = ?1",
            [&library],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(named, None);
    assert!(
        link_of(&conn, &library)
            .unwrap()
            .unwrap()
            .cloud_library
            .is_some(),
        "it still goes up"
    );
}

#[tokio::test]
async fn taking_the_cloud_away_after_uploading_fences_its_registration() {
    let cloud = FakeCloud::default();
    let api = cloud.api("ana");
    let (ana, library) = device_with_family();
    upload_settled(&ana, &api, &library).await;
    let c = cloud.only_library();
    let (id, _) = cloud.identity_of(&c).unwrap();
    let registration = cloud.cloud_operations(&c)[0].id.clone();

    crate::people::remove_cloud_device(&mut ana.lock().unwrap(), &library).unwrap();

    let conn = ana.lock().unwrap();
    let mark: String = conn
        .query_row(
            "SELECT through FROM device_fences WHERE library_id = ?1 AND device_id = ?2",
            [&library, &id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        mark, registration,
        "what the administrator had of the cloud is its registration"
    );
    let named: Option<String> = conn
        .query_row(
            "SELECT cloud_device_id FROM libraries WHERE id = ?1",
            [&library],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(named, None);
}
