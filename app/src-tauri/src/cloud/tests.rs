//! The cloud client against an in-memory cloud (no network). Devices are in-memory
//! databases, like the simulation in `sync.rs`.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use kosmotheka_domain::{
    Change, CopyInput, DeviceKey, EditionInput, Entity, Format, Hlc, Kind, Operation, ValidBook,
};
use serde_json::{Map, Value, json};

use super::*;
use crate::catalog;
use crate::permissions;

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
    /// Accounts the claim gave access to, and the person each one is.
    members: Vec<String>,
    persons: BTreeMap<String, String>,
    /// A server that lies, or has cut an account: what it answers `access_withdrawn` with, per account.
    forced_cut: BTreeMap<String, Vec<SyncOperation>>,
    /// A server whose proofs carry only the revocations, not the earlier operations of whoever signed them.
    no_chain: bool,
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
    /// Cloud libraries whose pull fails with this code.
    failing_pulls: BTreeMap<String, &'static str>,
    /// A server that makes no invitations (it has no address for their links).
    without_invitations: bool,
    /// Where the links of this server live.
    link_base: String,
    /// What `GET /v1/server` says the links start with, if the server says (an older one does not).
    published_base: Option<String>,
    invitations: Vec<FakeInvitation>,
    /// Creating an invitation fails with this code of the server.
    refuse_invitations: Option<&'static str>,
    /// Every call to an invitation route, whichever it is: what a device that must not ask never adds to.
    invitation_calls: usize,
    accepts: usize,
    /// The catalogues `GET /v1/server` says the server looks books up in (7.8).
    metadata: Vec<String>,
    /// What the book route answers, in order; the code is the server's own error.
    book_answers: std::collections::VecDeque<std::result::Result<Option<Metadata>, &'static str>>,
    /// What the title search answers, in order, and every search it was asked: kind, text and language.
    tmdb_answers: std::collections::VecDeque<
        std::result::Result<Vec<crate::metadata::Candidate>, &'static str>,
    >,
    tmdb_asks: Vec<(Kind, String, String)>,
    /// The same for the video game search: what it answers, in order, and the texts it was asked.
    rawg_answers: std::collections::VecDeque<
        std::result::Result<Vec<crate::metadata::Candidate>, &'static str>,
    >,
    rawg_asks: Vec<String>,
    /// The ISBNs the book route was asked, and how many times `GET /v1/server` was read for the catalogues.
    book_asks: Vec<String>,
    offer_checks: usize,
    /// Reading what the server offers fails.
    offer_fails: bool,
    /// The server and the token of every peek, to see where a link was taken.
    peeks: Vec<(String, String)>,
}

/// The small version of what the server keeps: no hash, the token is here, and the rules are only the ones
/// the client is tested against (the server's own are in its tests, 7.6.1).
struct FakeInvitation {
    id: String,
    cloud: String,
    member: String,
    kind: invitations::InvitationKind,
    email: Option<String>,
    token: String,
    state: invitations::InvitationState,
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
    /// The id and key of the device signed in with this account, for `claim`.
    device: Option<(String, String)>,
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
            device: None,
        }
    }

    /// A server that looks books up in Google Books for its accounts, answering these in order.
    fn with_google_books(&self, answers: Vec<std::result::Result<Option<Metadata>, &'static str>>) {
        let mut state = self.0.lock().unwrap();
        state.metadata = vec!["google_books".to_owned()];
        state.book_answers = answers.into();
    }

    /// A server that searches films and series in TMDB, answering these in order.
    fn with_tmdb(
        &self,
        answers: Vec<std::result::Result<Vec<crate::metadata::Candidate>, &'static str>>,
    ) {
        let mut state = self.0.lock().unwrap();
        state.metadata.push("tmdb".to_owned());
        state.tmdb_answers = answers.into();
    }

    /// A server that searches video games in RAWG, answering these in order.
    fn with_rawg(
        &self,
        answers: Vec<std::result::Result<Vec<crate::metadata::Candidate>, &'static str>>,
    ) {
        let mut state = self.0.lock().unwrap();
        state.metadata.push("rawg".to_owned());
        state.rawg_answers = answers.into();
    }

    fn rawg_asks(&self) -> Vec<String> {
        self.0.lock().unwrap().rawg_asks.clone()
    }

    fn tmdb_asks(&self) -> Vec<(Kind, String, String)> {
        self.0.lock().unwrap().tmdb_asks.clone()
    }

    fn book_asks(&self) -> Vec<String> {
        self.0.lock().unwrap().book_asks.clone()
    }

    /// A server that makes no invitations.
    fn without_invitations(self) -> Self {
        self.0.lock().unwrap().without_invitations = true;
        self
    }

    /// Creating an invitation fails with this code, after the person is in the catalog.
    fn refuse_invitations(&self, code: &'static str) {
        self.0.lock().unwrap().refuse_invitations = Some(code);
    }

    fn accepts(&self) -> usize {
        self.0.lock().unwrap().accepts
    }

    fn invitation_calls(&self) -> usize {
        self.0.lock().unwrap().invitation_calls
    }

    fn peeks(&self) -> Vec<(String, String)> {
        self.0.lock().unwrap().peeks.clone()
    }

    /// A server that publishes where its links start.
    fn publishing_base(self, base: &str) -> Self {
        self.0.lock().unwrap().published_base = Some(base.to_owned());
        self
    }

    fn with_link_base(self, base: &str) -> Self {
        self.0.lock().unwrap().link_base = base.to_owned();
        self
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

    /// Pulling this cloud library fails with `Error::Cloud(code)`.
    fn fail_pulls_of(&self, cloud: &str, code: &'static str) {
        self.0
            .lock()
            .unwrap()
            .failing_pulls
            .insert(cloud.to_owned(), code);
    }

    /// A server that lies: the id it gives its cloud device does not derive from the key it gives.
    fn lying(self) -> Self {
        self.0.lock().unwrap().lying = true;
        self
    }

    /// The server answers `access_withdrawn` to this account about this library with these proofs, true or not.
    fn cut(&self, cloud: &str, account: &str, proofs: Vec<SyncOperation>) {
        self.0
            .lock()
            .unwrap()
            .libraries
            .get_mut(cloud)
            .unwrap()
            .forced_cut
            .insert(account.to_owned(), proofs);
    }

    /// The server's proofs carry only the revocations (what the first version of the cut sent).
    fn without_chain(&self, cloud: &str) {
        self.0
            .lock()
            .unwrap()
            .libraries
            .get_mut(cloud)
            .unwrap()
            .no_chain = true;
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

impl FakeApi {
    /// The account signs in with this database's device (its id and key), as the server would know it.
    fn on(mut self, database: &Database) -> Self {
        let conn = database.lock().unwrap();
        self.device = Some((
            ops::device_id(&conn).unwrap(),
            ops::signing_key(&conn).unwrap().public_key(),
        ));
        self
    }
}

impl FakeLibrary {
    fn has_access(&self, account: &str) -> bool {
        self.owner == account || self.members.iter().any(|member| member == account)
    }

    /// The small version of the server's two cuts (D98, D71): a revocation in the log names the device, or the
    /// account's person has the access withdrawn. The proofs are the revocations that name the device.
    fn cut_for(
        &self,
        account: &str,
        device: Option<&str>,
        vector: Option<&VersionVector>,
    ) -> Option<Vec<SyncOperation>> {
        if let Some(forced) = self.forced_cut.get(account) {
            return Some(forced.clone());
        }
        let revoking = |id: &str| -> Vec<SyncOperation> {
            self.operations
                .values()
                .filter(|synced| {
                    synced.operation.changes.iter().any(|change| {
                        change.entity == Entity::Revocation
                            && change.action == kosmotheka_domain::Action::Create
                            && change.fields.get("device_id").and_then(Value::as_str) == Some(id)
                    })
                })
                .cloned()
                .collect()
        };
        let mut proofs = device.map(revoking).unwrap_or_default();
        if let Some(vector) = vector.filter(|_| !self.no_chain) {
            // With a vector (a pull): what the author of each revocation did before it and the client lacks.
            let mut chain: Vec<SyncOperation> = Vec::new();
            for proof in &proofs {
                let author = &proof.operation.hlc.device;
                chain.extend(
                    self.operations
                        .values()
                        .filter(|synced| {
                            let hlc = synced.operation.hlc.to_string();
                            &synced.operation.hlc.device == author
                                && synced.operation.hlc < proof.operation.hlc
                                && vector.get(author).is_none_or(|seen| hlc > *seen)
                        })
                        .cloned(),
                );
            }
            chain.extend(proofs);
            chain.sort_by(|a, b| a.operation.hlc.cmp(&b.operation.hlc));
            chain.dedup_by(|a, b| a.operation.id == b.operation.id);
            proofs = chain;
        }
        let person_cut = self.persons.get(account).is_some_and(|person| {
            self.operations
                .values()
                .flat_map(|synced| {
                    synced
                        .operation
                        .changes
                        .iter()
                        .map(|c| (&synced.operation.hlc, c))
                })
                .filter(|(_, change)| change.entity == Entity::Member && &change.id == person)
                .filter_map(|(hlc, change)| {
                    Some((hlc, change.fields.get("access_withdrawn")?.as_bool()?))
                })
                .max_by(|a, b| a.0.cmp(b.0))
                .is_some_and(|(_, withdrawn)| withdrawn)
        });
        (person_cut || !proofs.is_empty()).then_some(proofs)
    }

    /// The small version of the server's rule (D96): the person of the device in the log, if it is registered
    /// with this key and that key was vouched. The last writer of each field wins, as in the catalog.
    fn vouches_for(&self, id: &str, key: &str) -> Option<String> {
        let (registered, vouched, person) = self.device_fields(id);
        (registered.as_deref() == Some(key) && vouched.as_deref() == Some(key))
            .then_some(person)
            .flatten()
    }

    /// `public_key`, `vouched_key` and `member_id` of a device as the log leaves them.
    fn device_fields(&self, id: &str) -> (Option<String>, Option<String>, Option<String>) {
        let mut changes: Vec<(&Hlc, &Change)> = self
            .operations
            .values()
            .flat_map(|synced| {
                synced
                    .operation
                    .changes
                    .iter()
                    .map(|c| (&synced.operation.hlc, c))
            })
            .filter(|(_, change)| change.entity == Entity::Device && change.id == id)
            .collect();
        changes.sort_by(|a, b| a.0.cmp(b.0));
        let last = |field: &str| {
            changes
                .iter()
                .rev()
                .find_map(|(_, change)| change.fields.get(field))
                .and_then(Value::as_str)
                .map(str::to_owned)
        };
        (last("public_key"), last("vouched_key"), last("member_id"))
    }

    /// What the real cloud signs for itself, once it has registered and the library names it: an operation
    /// with its own key, chained to its previous one, dated after everything in the log.
    fn cloud_writes(&mut self, intent: &str, changes: Vec<Change>) -> bool {
        let (Some(identity), true) = (self.identity.as_ref(), self.registered) else {
            return false;
        };
        let Some(local) = self.local.clone() else {
            return false;
        };
        let newest = self
            .operations
            .values()
            .map(|synced| synced.operation.hlc.millis)
            .max()
            .unwrap_or_default();
        let previous = self
            .operations
            .values()
            .filter(|synced| synced.operation.hlc.device == identity.id)
            .max_by(|a, b| a.operation.hlc.cmp(&b.operation.hlc))
            .map(|synced| synced.operation.id.clone());
        let schema_version = self.operations.values().next().unwrap().schema_version;
        let mut operation = Operation {
            id: ops::new_id(),
            library: local,
            hlc: Hlc {
                millis: newest + 1,
                counter: 0,
                device: identity.id.clone(),
            },
            intent: intent.into(),
            changes,
            previous,
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
        true
    }

    /// Whether this person is in the catalog the log builds: a member row was created with this id.
    fn knows_member(&self, member: &str) -> bool {
        self.operations.values().any(|synced| {
            synced.operation.changes.iter().any(|change| {
                change.entity == Entity::Member
                    && change.action == kosmotheka_domain::Action::Create
                    && change.id == member
            })
        })
    }

    /// The account note the cloud writes when it links an account with its person (D76).
    fn note_account(&mut self, account: &str, member: &str) {
        let change = Change::update(
            Entity::Member,
            member,
            json!({ "account_id": format!("account-{account}") }),
        );
        self.cloud_writes("member.link_account", vec![change]);
    }

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
                members: Vec::new(),
                persons: BTreeMap::new(),
                forced_cut: BTreeMap::new(),
                no_chain: false,
                local: None,
                operations: BTreeMap::new(),
                identity,
                registered: false,
            },
        );
        Ok(listed)
    }

    async fn claim(&self) -> Result<Claimed> {
        let mut state = self.cloud.0.lock().unwrap();
        let Some((id, key)) = &self.device else {
            return Ok(Claimed::default());
        };
        let mut claimed = Claimed::default();
        for (cloud, library) in state.libraries.iter_mut() {
            if library.local.is_none() {
                continue;
            }
            if !library.persons.contains_key(&self.account)
                && let Some(person) = library.vouches_for(id, key)
            {
                // The owner's row only gets its person; anyone else's row is made.
                if !library.has_access(&self.account) {
                    library.members.push(self.account.clone());
                }
                library.persons.insert(self.account.clone(), person.clone());
                library.note_account(&self.account, &person);
                claimed.linked.push(cloud.clone());
            }
            // The second step: this account is a person here, and the device registered itself with this
            // key and belongs to nobody yet.
            let Some(person) = library.persons.get(&self.account).cloned() else {
                continue;
            };
            let (registered, _, owner) = library.device_fields(id);
            // The account is a person here and this device, registered with this key, is someone else's.
            if registered.as_deref() == Some(key.as_str())
                && owner.as_ref().is_some_and(|owner| *owner != person)
            {
                claimed.conflicts.push(cloud.clone());
            }
            if registered.as_deref() == Some(key.as_str())
                && owner.is_none()
                && library.cloud_writes(
                    "device.link_account",
                    vec![Change::update(
                        Entity::Device,
                        id,
                        json!({ "member_id": person, "vouched_key": key }),
                    )],
                )
            {
                claimed.devices.push(cloud.clone());
            }
        }
        Ok(claimed)
    }

    async fn list_libraries(&self) -> Result<Listing> {
        let state = self.cloud.0.lock().unwrap();
        let device = self.device.as_ref().map(|(id, _)| id.as_str());
        let mut listing = Listing::default();
        for (id, library) in state
            .libraries
            .iter()
            .filter(|(_, l)| l.has_access(&self.account))
        {
            if library.cut_for(&self.account, device, None).is_some() {
                listing.withdrawn.push(id.clone());
                continue;
            }
            listing.libraries.push(CloudLibrary {
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
            });
        }
        Ok(listing)
    }

    async fn pull(
        &self,
        cloud: &str,
        _schema_version: i64,
        vector: &VersionVector,
    ) -> Result<Pulled> {
        let mut state = self.cloud.0.lock().unwrap();
        state.pulls += 1;
        state
            .libraries
            .get(cloud)
            .filter(|library| library.has_access(&self.account))
            .ok_or(Error::Cloud("not_found"))?;
        match state.failing_pulls.get(cloud) {
            Some(&"access_withdrawn") => return Err(Error::AccessWithdrawn(Vec::new())),
            Some(code) => return Err(Error::Cloud(code)),
            None => {}
        }
        let library = &state.libraries[cloud];
        if let Some(proofs) = library.cut_for(
            &self.account,
            self.device.as_ref().map(|(id, _)| id.as_str()),
            Some(vector),
        ) {
            return Err(Error::AccessWithdrawn(proofs));
        }
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
            .filter(|library| library.has_access(&self.account))
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
            .filter(|library| library.has_access(&self.account))
            .ok_or(Error::Cloud("not_found"))?;
        if let Some(proofs) = library.cut_for(
            &self.account,
            self.device.as_ref().map(|(id, _)| id.as_str()),
            None,
        ) {
            return Err(Error::AccessWithdrawn(proofs));
        }
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

    async fn offers_invitations(&self) -> Result<bool> {
        Ok(!self.cloud.0.lock().unwrap().without_invitations)
    }

    async fn offers_metadata(&self, provider: &str) -> Result<bool> {
        let mut state = self.cloud.0.lock().unwrap();
        state.offer_checks += 1;
        if state.offer_fails {
            return Err(Error::account("server_unavailable"));
        }
        Ok(state.metadata.iter().any(|offered| offered == provider))
    }

    async fn rawg_search(&self, query: &str) -> Result<Vec<crate::metadata::Candidate>> {
        let mut state = self.cloud.0.lock().unwrap();
        state.rawg_asks.push(query.to_owned());
        match state.rawg_answers.pop_front() {
            Some(Ok(candidates)) => Ok(candidates),
            Some(Err(code)) => Err(Error::account(code)),
            None => Err(Error::account("server_unavailable")),
        }
    }

    async fn tmdb_search(
        &self,
        kind: Kind,
        query: &str,
        language: &str,
    ) -> Result<Vec<crate::metadata::Candidate>> {
        let mut state = self.cloud.0.lock().unwrap();
        state
            .tmdb_asks
            .push((kind, query.to_owned(), language.to_owned()));
        match state.tmdb_answers.pop_front() {
            Some(Ok(candidates)) => Ok(candidates),
            Some(Err(code)) => Err(Error::account(code)),
            None => Err(Error::account("server_unavailable")),
        }
    }

    async fn google_books_isbn(&self, isbn: &Isbn) -> Result<Option<Metadata>> {
        let mut state = self.cloud.0.lock().unwrap();
        state.book_asks.push(isbn.as_str().to_owned());
        match state.book_answers.pop_front() {
            Some(Ok(book)) => Ok(book),
            Some(Err(code)) => Err(Error::account(code)),
            None => Err(Error::account("server_unavailable")),
        }
    }

    async fn invitation_base(&self) -> Result<Option<String>> {
        self.cloud.0.lock().unwrap().invitation_calls += 1;
        Ok(self.cloud.0.lock().unwrap().published_base.clone())
    }

    async fn create_invitation(
        &self,
        cloud: &str,
        member: &str,
        kind: invitations::InvitationKind,
        email: Option<&str>,
    ) -> Result<invitations::CreatedInvitation> {
        let mut state = self.cloud.0.lock().unwrap();
        state.invitation_calls += 1;
        if let Some(code) = state.refuse_invitations {
            return Err(Error::account(code));
        }
        let library = state
            .libraries
            .get(cloud)
            .filter(|library| library.has_access(&self.account))
            .ok_or(Error::Cloud("not_found"))?;
        // The id is compared with the catalog of this library, as the real server does (D84).
        if !library.knows_member(member) {
            return Err(Error::account("invitation_person_unsuitable"));
        }
        let token = format!("{:A>43}", state.invitations.len() + 1);
        let base = if state.link_base.is_empty() {
            "https://cloud.example"
        } else {
            &state.link_base
        };
        let link = (kind == invitations::InvitationKind::Link)
            .then(|| invitations::Secret::new(format!("{base}/i/{token}")));
        let id = ops::new_id();
        state.invitations.push(FakeInvitation {
            id: id.clone(),
            cloud: cloud.to_owned(),
            member: member.to_owned(),
            kind,
            email: email.map(str::to_owned),
            token,
            state: invitations::InvitationState::Pending,
        });
        Ok(invitations::CreatedInvitation {
            id,
            kind,
            member_id: member.to_owned(),
            expires_at: "2099-01-01T00:00:00+00:00".to_owned(),
            link,
        })
    }

    async fn list_invitations(&self, cloud: &str) -> Result<Vec<invitations::InvitationItem>> {
        let mut state = self.cloud.0.lock().unwrap();
        state.invitation_calls += 1;
        Ok(state
            .invitations
            .iter()
            .filter(|invitation| invitation.cloud == cloud)
            .map(|invitation| invitations::InvitationItem {
                id: invitation.id.clone(),
                kind: invitation.kind,
                member_id: invitation.member.clone(),
                state: invitation.state,
                created_at: "2098-12-31T00:00:00+00:00".to_owned(),
                expires_at: "2099-01-01T00:00:00+00:00".to_owned(),
                accepted_at: None,
            })
            .collect())
    }

    async fn revoke_invitation(&self, cloud: &str, invitation: &str) -> Result<()> {
        let mut state = self.cloud.0.lock().unwrap();
        state.invitation_calls += 1;
        let found = state
            .invitations
            .iter_mut()
            .find(|found| found.cloud == cloud && found.id == invitation)
            .ok_or(Error::Cloud("not_found"))?;
        found.state = invitations::InvitationState::Revoked;
        Ok(())
    }

    async fn peek_invitation(&self, token: &str) -> Result<invitations::InvitationPreview> {
        let mut state = self.cloud.0.lock().unwrap();
        state.invitation_calls += 1;
        state.peeks.push((self.server.clone(), token.to_owned()));
        let found = state
            .invitations
            .iter()
            .find(|found| found.token == token)
            .ok_or(Error::Cloud("invitation_not_found"))?;
        match found.state {
            invitations::InvitationState::Pending => Ok(invitations::InvitationPreview {
                library_name: "Family".to_owned(),
                kind: found.kind,
                expires_at: "2099-01-01T00:00:00+00:00".to_owned(),
            }),
            invitations::InvitationState::Accepted => Err(Error::account("invitation_used")),
            _ => Err(Error::account("invitation_revoked")),
        }
    }

    async fn accept_invitation(&self, token: &str) -> Result<String> {
        let mut state = self.cloud.0.lock().unwrap();
        state.accepts += 1;
        state.invitation_calls += 1;
        let index = state
            .invitations
            .iter()
            .position(|found| found.token == token)
            .ok_or(Error::Cloud("invitation_not_found"))?;
        let (cloud, member, email) = {
            let found = &state.invitations[index];
            match found.state {
                invitations::InvitationState::Pending => {}
                invitations::InvitationState::Accepted => {
                    return Err(Error::account("invitation_used"));
                }
                _ => return Err(Error::account("invitation_revoked")),
            }
            (
                found.cloud.clone(),
                found.member.clone(),
                found.email.clone(),
            )
        };
        if email.is_some_and(|email| email != format!("{}@example.com", self.account)) {
            return Err(Error::account("invitation_not_for_this_account"));
        }
        state.invitations[index].state = invitations::InvitationState::Accepted;
        let library = state.libraries.get_mut(&cloud).unwrap();
        if !library.has_access(&self.account) {
            library.members.push(self.account.clone());
        }
        library.persons.insert(self.account.clone(), member.clone());
        library.note_account(&self.account, &member);
        Ok(cloud)
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

mod invitations_tests;
mod links_tests;
mod metadata_tests;
mod own_key_tests;
mod rawg_tests;
mod tmdb_tests;

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

/// 7.5.2b-2 (D76, D96): Hugo's phone, assigned and vouched, has the library over the LAN; his own account
/// claims it, is let in, adopts the cloud link and converges with the laptop that uploaded.
#[tokio::test]
async fn a_second_account_whose_device_is_assigned_and_vouched_syncs_the_library_it_has_over_the_lan()
 {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib) = laptop_admin_phone_reader();
    let tersoal = cloud.api("tersoal").on(&laptop);
    // The laptop vouches for the old assignments at the end of its first sessions.
    upload_settled(&laptop, &tersoal, &lib).await;
    let c = cloud.only_library();
    assert!(
        laptop
            .lock()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM devices WHERE library_id = ?1 AND member_id IS NOT NULL AND vouched_key IS NULL",
                [&lib],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            == 0
    );
    assert_eq!(
        link_of(&phone.lock().unwrap(), &lib)
            .unwrap()
            .and_then(|l| l.cloud_library),
        None
    );
    let hugo = cloud.api("hugo").on(&phone);
    assert!(
        hugo.list_libraries().await.unwrap().libraries.is_empty(),
        "no access before the claim"
    );

    let reports = sync_all(&phone, &hugo).await.unwrap();

    assert_eq!(reports.len(), 1);
    assert_eq!(link(&phone, &lib).0.as_deref(), Some(c.as_str()));
    sync_library(&laptop, &tersoal, &lib).await.unwrap();
    sync_library(&phone, &hugo, &lib).await.unwrap();
    assert_converged(&[&laptop, &phone], &lib);
}

/// A new phone of an account that is already Hugo, an editor: one session and it has the library, its person
/// and its role (D76, D96); what it adds is applied and reaches everybody.
#[tokio::test]
async fn a_new_phone_of_a_linked_account_gets_the_library_its_person_and_its_role_in_one_session() {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib) = laptop_admin_phone_reader();
    let phone_id = ops::device_id(&phone.lock().unwrap()).unwrap();
    let hugo_member: String = laptop
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
        &hugo_member,
        Some("editor"),
        false,
    )
    .unwrap();
    lan_sync(&laptop, &phone, &lib);
    let tersoal = cloud.api("tersoal").on(&laptop);
    upload_settled(&laptop, &tersoal, &lib).await;
    // Hugo's first phone claims its person and syncs.
    let hugo = cloud.api("hugo").on(&phone);
    sync_all(&phone, &hugo).await.unwrap();
    sync_library(&laptop, &tersoal, &lib).await.unwrap();

    // The new phone: nothing of the library, a fresh device of the same account.
    let new = Database::in_memory();
    let hugos_new = cloud.api("hugo").on(&new);
    let reports = sync_all(&new, &hugos_new).await.unwrap();

    assert!(
        reports
            .iter()
            .any(|r| r.library == lib && r.outcome == Outcome::Downloaded)
    );
    // The report says who the phone is by the end of the session, for the summary of the download.
    assert_eq!(
        reports
            .iter()
            .find(|r| r.library == lib)
            .and_then(|r| r.me.clone()),
        Some(Joined {
            person: "Hugo".into(),
            role: Some("editor".into())
        })
    );
    let new_id = ops::device_id(&new.lock().unwrap()).unwrap();
    let person: Option<String> = new
        .lock()
        .unwrap()
        .query_row(
            "SELECT member_id FROM devices WHERE library_id = ?1 AND id = ?2",
            [&lib, &new_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        person.as_deref(),
        Some(hugo_member.as_str()),
        "the cloud said it is Hugo's"
    );
    assert_eq!(
        people::list_people(&new.lock().unwrap(), &lib)
            .unwrap()
            .my_role
            .as_deref(),
        Some("editor")
    );
    // A change of the new phone is applied everywhere.
    add(&new, &lib, "The Farthest Shore");
    sync_library(&new, &hugos_new, &lib).await.unwrap();
    sync_library(&laptop, &tersoal, &lib).await.unwrap();
    sync_library(&phone, &hugo, &lib).await.unwrap();
    sync_library(&new, &hugos_new, &lib).await.unwrap();
    for device in [&laptop, &phone, &new] {
        assert!(
            catalog::list_books(
                &device.lock().unwrap(),
                &lib,
                &catalog::BookFilter::default()
            )
            .unwrap()
            .iter()
            .any(|b| b.title == "The Farthest Shore")
        );
    }
    assert_converged(&[&laptop, &phone, &new], &lib);

    // Who linked each device (D76, D96): the cloud only for the new phone; by hand, the administrator.
    let new_id = ops::device_id(&new.lock().unwrap()).unwrap();
    let linked = |device: &Database| -> Vec<(String, bool, Option<String>)> {
        let mut found: Vec<_> = people::list_people(&device.lock().unwrap(), &lib)
            .unwrap()
            .people
            .into_iter()
            .flat_map(|p| p.devices)
            .map(|d| (d.id, d.linked_by_cloud, d.linked_by))
            .collect();
        found.sort();
        found
    };
    for device in [&laptop, &new] {
        let found = linked(device);
        let of = |id: &str| found.iter().find(|d| d.0 == id).cloned().unwrap();
        assert!(of(&new_id).1, "the new phone was linked by the cloud");
        assert_eq!(of(&new_id).2, None);
        assert!(!of(&phone_id).1, "the first phone was linked by hand");
        assert_eq!(of(&phone_id).2.as_deref(), Some("Tersoal"));
    }
    let hugo_has_account = people::list_people(&laptop.lock().unwrap(), &lib)
        .unwrap()
        .people
        .iter()
        .map(|p| (p.name.clone(), p.has_account))
        .collect::<Vec<_>>();
    assert!(hugo_has_account.contains(&("Hugo".into(), true)));

    // Hostile: a library that names as its cloud a device whose key does not derive its id has no cloud
    // device, so nothing is «by the cloud» whoever signed (rule 12).
    laptop
        .lock()
        .unwrap()
        .execute(
            "UPDATE libraries SET cloud_device_id = ?2 WHERE id = ?1",
            [&lib, &new_id],
        )
        .unwrap();
    assert!(linked(&laptop).iter().all(|d| !d.1));
}

/// D58: the only administrator loses her only phone. A new one and her account are enough.
#[tokio::test]
async fn the_only_administrator_with_a_new_phone_and_her_account_is_an_administrator_again() {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib) = laptop_admin_phone_reader();
    let tersoal = cloud.api("tersoal").on(&laptop);
    upload_settled(&laptop, &tersoal, &lib).await;
    // Her session with the cloud: she is let in as her person.
    sync_all(&laptop, &tersoal).await.unwrap();
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
    // The laptop is gone: only the cloud and a new device remain.
    let new = Database::in_memory();
    let api = cloud.api("tersoal").on(&new);

    sync_all(&new, &api).await.unwrap();

    assert_eq!(
        people::list_people(&new.lock().unwrap(), &lib)
            .unwrap()
            .my_role
            .as_deref(),
        Some("admin")
    );
    people::set_role(&mut new.lock().unwrap(), &lib, &hugo, Some("editor"), false).unwrap();
}

/// A library that was never uploaded: nothing happens in the cloud, and nothing changes here.
#[tokio::test]
async fn a_library_that_was_never_uploaded_stays_as_it_is_in_a_session() {
    let cloud = FakeCloud::default();
    let (device, library) = device_with_family();
    let before = log_ids(&device, &library);

    let reports = sync_all(&device, &cloud.api("ana").on(&device))
        .await
        .unwrap();

    assert!(reports.is_empty());
    assert_eq!(cloud.calls(), (0, 0));
    assert_eq!(log_ids(&device, &library), before);
}

/// A new phone of an account that is not linked with any person gets nothing.
#[tokio::test]
async fn a_new_phone_of_an_account_that_is_not_linked_gets_nothing() {
    let cloud = FakeCloud::default();
    let (laptop, _phone, lib) = laptop_admin_phone_reader();
    upload_settled(&laptop, &cloud.api("tersoal").on(&laptop), &lib).await;
    let new = Database::in_memory();
    let own = ops::active_library(&new.lock().unwrap()).unwrap();
    let before = log_ids(&new, &own);
    let (pulls, pushes) = cloud.calls();

    let reports = sync_all(&new, &cloud.api("mallory").on(&new))
        .await
        .unwrap();

    assert!(reports.is_empty());
    assert_eq!(cloud.calls(), (pulls, pushes));
    assert!(!library_exists_here(&new, &lib));
    assert_eq!(log_ids(&new, &own), before);
}

fn library_exists_here(device: &Database, library: &str) -> bool {
    libraries::library_exists(&device.lock().unwrap(), library).unwrap()
}

// --- The cloud cuts the access (D98, D71) -----------------------------------------------------------

/// The laptop (an administrator) uploaded; Hugo's phone has the library, is let in by its account, and the two
/// have everything the other has. Returns the cloud library id and Hugo's member id.
async fn laptop_and_hugos_phone_in_sync(
    cloud: &FakeCloud,
) -> (Database, Database, String, String, String) {
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
    let tersoal = cloud.api("tersoal").on(&laptop);
    upload_settled(&laptop, &tersoal, &lib).await;
    sync_all(&phone, &cloud.api("hugo").on(&phone))
        .await
        .unwrap();
    sync_library(&laptop, &tersoal, &lib).await.unwrap();
    sync_all(&phone, &cloud.api("hugo").on(&phone))
        .await
        .unwrap();
    (laptop, phone, lib, hugo, cloud.only_library())
}

fn titles(device: &Database, library: &str) -> Vec<String> {
    let mut titles: Vec<String> = catalog::list_books(
        &device.lock().unwrap(),
        library,
        &catalog::BookFilter::default(),
    )
    .unwrap()
    .into_iter()
    .map(|book| book.title)
    .collect();
    titles.sort();
    titles
}

/// An operation signed by `device` itself, as a modified app would make it.
fn signed_by(
    device: &Database,
    library: &str,
    intent: &str,
    changes: Vec<Change>,
) -> SyncOperation {
    let conn = device.lock().unwrap();
    let id = ops::device_id(&conn).unwrap();
    let mut operation = Operation {
        id: ops::new_id(),
        library: library.to_owned(),
        hlc: Hlc {
            millis: 9_000_000_000_000,
            counter: 0,
            device: id.clone(),
        },
        intent: intent.into(),
        changes,
        previous: ops::latest_of(&conn, library, &id).unwrap(),
        signature: None,
    };
    operation.signature = Some(ops::signing_key(&conn).unwrap().sign(&operation));
    SyncOperation {
        schema_version: ops::schema_version(&conn).unwrap(),
        operation,
    }
}

/// D98: echar a una persona is proved to her phone by the signed operation, and the phone removes the library.
#[tokio::test]
async fn a_phone_of_a_person_who_was_withdrawn_removes_the_library_with_the_proof_and_it_does_not_come_back()
 {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib, hugo, c) = laptop_and_hugos_phone_in_sync(&cloud).await;
    let api = cloud.api("hugo").on(&phone);
    people::withdraw_person(&mut laptop.lock().unwrap(), &lib, &hugo).unwrap();
    sync_library(&laptop, &cloud.api("tersoal").on(&laptop), &lib)
        .await
        .unwrap();

    let reports = sync_all(&phone, &api).await.unwrap();

    assert!(
        reports
            .iter()
            .any(|r| r.library == lib && r.outcome == Outcome::Removed),
        "{reports:?}"
    );
    assert!(!library_exists_here(&phone, &lib));
    // The next session does not bring it down again, and the laptop keeps it.
    let again = sync_all(&phone, &api).await.unwrap();
    assert!(again.iter().all(|r| r.outcome != Outcome::Downloaded));
    assert!(!library_exists_here(&phone, &lib));
    assert!(library_exists_here(&laptop, &lib));
    let _ = c;
}

/// A device withdrawn alone (D71) gets the same answer from the cloud and removes the library the same way.
#[tokio::test]
async fn a_device_whose_access_was_withdrawn_removes_the_library_with_the_proof() {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib, _hugo, _c) = laptop_and_hugos_phone_in_sync(&cloud).await;
    let phone_id = ops::device_id(&phone.lock().unwrap()).unwrap();
    libraries::revoke_access(&mut laptop.lock().unwrap(), &lib, &phone_id).unwrap();
    sync_library(&laptop, &cloud.api("tersoal").on(&laptop), &lib)
        .await
        .unwrap();

    let reports = sync_all(&phone, &cloud.api("hugo").on(&phone))
        .await
        .unwrap();

    assert!(reports.iter().any(|r| r.outcome == Outcome::Removed));
    assert!(!library_exists_here(&phone, &lib));
}

/// A server that lies cannot make anybody lose a library: four ways to say «access withdrawn» without a proof
/// this device accepts, and the library stays whole.
#[tokio::test]
async fn a_server_that_lies_about_the_access_removes_nothing() {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib, _hugo, c) = laptop_and_hugos_phone_in_sync(&cloud).await;
    let phone_id = ops::device_id(&phone.lock().unwrap()).unwrap();
    let before_titles = titles(&phone, &lib);
    let before_log = log_ids(&phone, &lib);

    // A revocation of this phone signed by an administrator, in another library.
    let (stranger, other) = device_with_family();
    libraries::revoke_access(&mut stranger.lock().unwrap(), &other, &phone_id).unwrap();
    let elsewhere: Vec<SyncOperation> = sync::operations_since(
        &stranger.lock().unwrap(),
        &LibraryVectors::from([(other.clone(), VersionVector::new())]),
    )
    .unwrap()
    .into_iter()
    .filter(|synced| synced.operation.intent == "library.revoke")
    .collect();
    assert_eq!(elsewhere.len(), 1);
    // A revocation of this phone signed by a reader (itself): not allowed, so it is only a request.
    let by_a_reader = signed_by(
        &phone,
        &lib,
        "library.revoke",
        vec![Change::create(
            Entity::Revocation,
            ops::new_id(),
            json!({ "device_id": phone_id, "device_name": "me" }),
        )],
    );
    // A genuine revocation signed by the administrator, but of another device.
    libraries::revoke_access(
        &mut laptop.lock().unwrap(),
        &lib,
        "01997c7e-0000-7000-8000-0000000000ee",
    )
    .unwrap();
    let schema_version = ops::schema_version(&laptop.lock().unwrap()).unwrap();
    let log = sync::library_log(&laptop.lock().unwrap(), &lib).unwrap();
    let of_another: Vec<SyncOperation> = log
        .into_iter()
        .filter(|op| op.intent == "library.revoke")
        .map(|operation| SyncOperation {
            schema_version,
            operation,
        })
        .collect();
    assert_eq!(of_another.len(), 1);

    for (name, proofs) in [
        ("no proofs", Vec::new()),
        ("proofs of another library", elsewhere),
        ("a revocation signed by a reader", vec![by_a_reader]),
        ("a revocation of another device", of_another),
    ] {
        cloud.cut(&c, "hugo", proofs);

        let reports = sync_all(&phone, &cloud.api("hugo").on(&phone))
            .await
            .unwrap();

        assert!(
            reports
                .iter()
                .any(|r| r.library == lib && r.outcome == Outcome::AccessWithdrawn),
            "{name}: {reports:?}"
        );
        assert!(
            library_exists_here(&phone, &lib),
            "{name}: the library stays"
        );
        assert_eq!(titles(&phone, &lib), before_titles, "{name}");
        assert_eq!(log_ids(&phone, &lib), before_log, "{name}");
    }
}

/// The administrator did things the phone never received, and the cut stops it from receiving them: the cloud
/// sends, with the revocation, the earlier operations of whoever signed it that the phone lacks (D87), so the
/// withdrawal chains and the library is removed.
#[tokio::test]
async fn a_proof_that_comes_with_the_earlier_operations_of_the_administrator_removes_the_library() {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib, hugo, _c) = laptop_and_hugos_phone_in_sync(&cloud).await;
    add(&laptop, &lib, "Something the phone never received");
    people::withdraw_person(&mut laptop.lock().unwrap(), &lib, &hugo).unwrap();
    sync_library(&laptop, &cloud.api("tersoal").on(&laptop), &lib)
        .await
        .unwrap();

    let reports = sync_all(&phone, &cloud.api("hugo").on(&phone))
        .await
        .unwrap();

    assert!(
        reports.iter().any(|r| r.outcome == Outcome::Removed),
        "{reports:?}"
    );
    assert!(!library_exists_here(&phone, &lib));
}

/// The fail-safe: the server does not send the earlier operations, the revocation cannot chain, and nothing is
/// removed. The library stays, reported as withdrawn.
#[tokio::test]
async fn a_proof_without_the_earlier_operations_it_depends_on_removes_nothing() {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib, hugo, c) = laptop_and_hugos_phone_in_sync(&cloud).await;
    add(&laptop, &lib, "Something the phone never received");
    people::withdraw_person(&mut laptop.lock().unwrap(), &lib, &hugo).unwrap();
    sync_library(&laptop, &cloud.api("tersoal").on(&laptop), &lib)
        .await
        .unwrap();
    cloud.without_chain(&c);

    let reports = sync_all(&phone, &cloud.api("hugo").on(&phone))
        .await
        .unwrap();

    assert!(
        reports
            .iter()
            .any(|r| r.outcome == Outcome::AccessWithdrawn),
        "{reports:?}"
    );
    assert!(library_exists_here(&phone, &lib));
}

/// A fifth way to lie: earlier operations that claim to be the administrator's but are signed by somebody else,
/// next to the real revocation. They are refused, the revocation finds its `previous` missing, and nothing goes.
#[tokio::test]
async fn forged_earlier_operations_do_not_make_the_device_remove_anything() {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib, hugo, c) = laptop_and_hugos_phone_in_sync(&cloud).await;
    add(&laptop, &lib, "Something the phone never received");
    people::withdraw_person(&mut laptop.lock().unwrap(), &lib, &hugo).unwrap();
    sync_library(&laptop, &cloud.api("tersoal").on(&laptop), &lib)
        .await
        .unwrap();
    let laptop_id = ops::device_id(&laptop.lock().unwrap()).unwrap();
    let schema_version = ops::schema_version(&laptop.lock().unwrap()).unwrap();
    let log = sync::library_log(&laptop.lock().unwrap(), &lib).unwrap();
    // The real revocation, and, in place of the laptop's real earlier operation, a copy re-signed by the phone.
    let key = ops::signing_key(&phone.lock().unwrap()).unwrap();
    let mut proofs = Vec::new();
    for mut operation in log.into_iter().filter(|op| op.hlc.device == laptop_id) {
        if operation.intent == "book.add" {
            operation.signature = Some(key.sign(&operation));
        }
        if operation.intent == "member.withdraw" || operation.intent == "book.add" {
            proofs.push(SyncOperation {
                schema_version,
                operation,
            });
        }
    }
    assert!(proofs.len() >= 2, "{}", proofs.len());
    let before_titles = titles(&phone, &lib);
    cloud.cut(&c, "hugo", proofs);

    let reports = sync_all(&phone, &cloud.api("hugo").on(&phone))
        .await
        .unwrap();

    assert!(
        reports
            .iter()
            .any(|r| r.outcome == Outcome::AccessWithdrawn),
        "{reports:?}"
    );
    assert!(library_exists_here(&phone, &lib));
    assert_eq!(titles(&phone, &lib), before_titles);
}

/// A device that has the library over the LAN but belongs to nobody gets nothing from the cloud.
#[tokio::test]
async fn a_device_of_nobody_that_has_the_library_over_the_lan_gets_nothing_from_the_cloud() {
    let cloud = FakeCloud::default();
    let (laptop, _phone, lib) = laptop_admin_phone_reader();
    upload_settled(&laptop, &cloud.api("tersoal").on(&laptop), &lib).await;
    let tablet = Database::in_memory();
    lan_sync(&laptop, &tablet, &lib);
    {
        // It only registers itself: a device of nobody is a reader and may write no more.
        let mut conn = tablet.lock().unwrap();
        let tx = conn.transaction().unwrap();
        ops::ensure_registered(&tx, &lib).unwrap();
        tx.commit().unwrap();
    }
    lan_sync(&laptop, &tablet, &lib);
    let before = log_ids(&tablet, &lib);

    let c = cloud.only_library();
    let (pulls, pushes) = cloud.calls();

    let reports = sync_all(&tablet, &cloud.api("mallory").on(&tablet))
        .await
        .unwrap();

    // The LAN already told it the link, but the cloud does not know it: nothing is pulled or pushed.
    assert!(reports.iter().all(|r| r.outcome == Outcome::MissingInCloud));
    assert_eq!(cloud.calls(), (pulls, pushes));
    assert_eq!(log_ids(&tablet, &lib), before);
    let tablet_id = ops::device_id(&tablet.lock().unwrap()).unwrap();
    assert!(
        !cloud.operation_ids(&c).iter().any(|id| {
            sync::library_log(&tablet.lock().unwrap(), &lib)
                .unwrap()
                .iter()
                .any(|op| &op.id == id && op.hlc.device == tablet_id)
        }),
        "nothing of the tablet reached the cloud"
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
    // The phone registered itself in the library (D56) and sent only that.
    assert_eq!(reports[0].sent, 1);
    sync_library(&ana, &cloud.api("ana"), &library)
        .await
        .unwrap();
    assert_converged(&[&ana, &phone], &library);
    assert_eq!(link(&phone, &library), link(&ana, &library));

    // And what the second device adds goes back the same way.
    add(&phone, &library, "The Dispossessed");
    sync_library(&phone, &cloud.api("ana"), &library)
        .await
        .unwrap();
    let reports = sync_all(&ana, &cloud.api("ana")).await.unwrap();
    assert_eq!(reports[0].outcome, Outcome::Synced);
    assert_eq!(reports[0].received, 1); // the book: the phone's key came with its first session
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
    // The phone registered itself in the library; the laptop hears of it in its next session.
    sync_library(&laptop, &cloud.api("kenji"), &library)
        .await
        .unwrap();
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
    sync_library(&ana, &cloud.api("ana"), &library)
        .await
        .unwrap();
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
    // The tablet registered itself in the library; the others hear of it.
    sync_library(&laptop, &cloud.api("ana"), &library)
        .await
        .unwrap();
    lan_sync(&laptop, &phone, &library);
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

/// Three libraries of one account, uploaded from a laptop: their local ids, in order, and the laptop.
async fn three_uploaded_libraries(cloud: &FakeCloud) -> (Database, Vec<String>) {
    let laptop = Database::in_memory();
    let api = cloud.api("ana").on(&laptop);
    let mut libs = Vec::new();
    for name in ["One", "Two", "Three"] {
        let lib = libraries::create_library(&mut laptop.lock().unwrap(), name).unwrap();
        add(&laptop, &lib, &format!("Book of {name}"));
        upload_settled(&laptop, &api, &lib).await;
        libs.push(lib);
    }
    (laptop, libs)
}

/// DAT-009: one library that cannot be brought down does not stop the others from coming down.
#[tokio::test]
async fn a_library_that_fails_to_download_does_not_stop_the_others() {
    let cloud = FakeCloud::default();
    let (laptop, libs) = three_uploaded_libraries(&cloud).await;
    cloud.fail_pulls_of(&link(&laptop, &libs[1]).0.unwrap(), "invalid_request");
    let phone = Database::in_memory();

    let reports = sync_all(&phone, &cloud.api("ana").on(&phone))
        .await
        .unwrap();

    let outcome = |lib: &String| reports.iter().find(|r| &r.library == lib).unwrap();
    assert_eq!(outcome(&libs[0]).outcome, Outcome::Downloaded);
    assert_eq!(outcome(&libs[2]).outcome, Outcome::Downloaded);
    assert_eq!(outcome(&libs[1]).outcome, Outcome::Failed);
    assert_eq!(outcome(&libs[1]).error.as_deref(), Some("invalid_request"));
    let conn = phone.lock().unwrap();
    assert!(libraries::library_exists(&conn, &libs[0]).unwrap());
    assert!(!libraries::library_exists(&conn, &libs[1]).unwrap());
    assert!(libraries::library_exists(&conn, &libs[2]).unwrap());
}

/// DAT-009: the same for libraries this device already syncs.
#[tokio::test]
async fn a_linked_library_that_fails_to_sync_does_not_stop_the_others() {
    let cloud = FakeCloud::default();
    let (laptop, libs) = three_uploaded_libraries(&cloud).await;
    let api = cloud.api("ana").on(&laptop);
    cloud.fail_pulls_of(&link(&laptop, &libs[0]).0.unwrap(), "unsigned_history");
    add(&laptop, &libs[2], "Added after");

    let reports = sync_all(&laptop, &api).await.unwrap();

    let failed: Vec<_> = reports
        .iter()
        .filter(|r| r.outcome == Outcome::Failed)
        .collect();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].library, libs[0]);
    assert_eq!(failed[0].error.as_deref(), Some("unsigned_history"));
    let third = reports.iter().find(|r| r.library == libs[2]).unwrap();
    assert_eq!((third.outcome, third.sent > 0), (Outcome::Synced, true));
}

/// What belongs to the whole session still stops it: a version of the schema the server does not take.
#[tokio::test]
async fn an_error_of_the_session_still_stops_it() {
    let cloud = FakeCloud::default();
    let (laptop, libs) = three_uploaded_libraries(&cloud).await;
    cloud.fail_pulls_of(&link(&laptop, &libs[1]).0.unwrap(), "schema_version");

    let result = sync_all(&laptop, &cloud.api("ana").on(&laptop)).await;

    assert!(matches!(result, Err(Error::Cloud("schema_version"))));
}

/// DAT-009: a cut that arrives while a library comes down is a result, like for a library already here.
#[tokio::test]
async fn a_library_cut_while_it_comes_down_is_a_result_and_the_others_still_come_down() {
    let cloud = FakeCloud::default();
    let (laptop, libs) = three_uploaded_libraries(&cloud).await;
    cloud.fail_pulls_of(&link(&laptop, &libs[0]).0.unwrap(), "access_withdrawn");
    let phone = Database::in_memory();

    let reports = sync_all(&phone, &cloud.api("ana").on(&phone))
        .await
        .unwrap();

    let first = reports.iter().find(|r| r.library == libs[0]).unwrap();
    assert_eq!(first.outcome, Outcome::AccessWithdrawn);
    let conn = phone.lock().unwrap();
    assert!(!libraries::library_exists(&conn, &libs[0]).unwrap());
    assert!(libraries::library_exists(&conn, &libs[1]).unwrap());
    assert!(libraries::library_exists(&conn, &libs[2]).unwrap());
}

fn cloud_power(device: &Database, library: &str) -> bool {
    status(device)
        .unwrap()
        .iter()
        .find(|s| s.library == library)
        .unwrap()
        .cloud_power
}

/// D76: the box of the upload screen says the truth. Off, the link does not name the cloud; later the
/// switch of the library gives it the power (and the cloud registers itself), and takes it away again.
#[tokio::test]
async fn the_cloud_gets_the_power_only_if_the_upload_said_so_and_the_switch_changes_it() {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib) = laptop_admin_phone_reader();
    let api = cloud.api("tersoal").on(&laptop);

    upload_with(&laptop, &api, &lib, false).await.unwrap();
    sync_library(&laptop, &api, &lib).await.unwrap();

    assert!(!cloud_power(&laptop, &lib), "off: the link names nobody");
    assert!(cloud.cloud_operations(&cloud.only_library()).is_empty());

    set_cloud_power(&laptop, &api, &lib, true).await.unwrap();
    assert!(cloud_power(&laptop, &lib));
    sync_library(&laptop, &api, &lib).await.unwrap();
    assert!(
        !cloud.cloud_operations(&cloud.only_library()).is_empty(),
        "the cloud registers itself once named"
    );

    set_cloud_power(&laptop, &api, &lib, false).await.unwrap();
    assert!(!cloud_power(&laptop, &lib));
    lan_sync(&laptop, &phone, &lib);
    assert!(!cloud_power(&phone, &lib), "and the family hears of it");
}

/// Hostile: a reader (Hugo's phone, with a vouched device) cannot give or take the cloud's power, by the
/// switch or by a modified app calling the command: nothing is written.
#[tokio::test]
async fn a_reader_cannot_give_or_take_the_cloud_s_power() {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib) = laptop_admin_phone_reader();
    let tersoal = cloud.api("tersoal").on(&laptop);
    upload_settled(&laptop, &tersoal, &lib).await;
    let hugo = cloud.api("hugo").on(&phone);
    sync_all(&phone, &hugo).await.unwrap();
    assert!(cloud_power(&phone, &lib));
    let before = log_ids(&phone, &lib);

    let off = set_cloud_power(&phone, &hugo, &lib, false).await;
    let on = set_cloud_power(&phone, &hugo, &lib, true).await;

    assert!(matches!(off, Err(Error::NotAllowed)), "{off:?}");
    assert!(matches!(on, Err(Error::NotAllowed)), "{on:?}");
    assert!(cloud_power(&phone, &lib));
    assert_eq!(log_ids(&phone, &lib), before);
}

/// D76: the account is let into a library the device already had, as Hugo, a reader; it is said once.
#[tokio::test]
async fn being_let_into_a_library_by_the_cloud_is_told_once_and_not_to_who_uploaded_it() {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib) = laptop_admin_phone_reader();
    let tersoal = cloud.api("tersoal").on(&laptop);
    upload_settled(&laptop, &tersoal, &lib).await;
    let hugo = cloud.api("hugo").on(&phone);

    let first = sync_all(&phone, &hugo).await.unwrap();
    let again = sync_all(&phone, &hugo).await.unwrap();
    let uploader = sync_all(&laptop, &tersoal).await.unwrap();

    let joined = |reports: &[LibraryReport]| reports.iter().find_map(|r| r.joined.clone());
    assert_eq!(
        joined(&first),
        Some(Joined {
            person: "Hugo".into(),
            role: Some("reader".into())
        })
    );
    assert_eq!(joined(&again), None, "once");
    assert_eq!(joined(&uploader), None, "who uploaded it was not let in");
}

/// D98: the report of a library removed keeps its name, which the screen no longer has.
#[tokio::test]
async fn a_library_removed_by_the_cloud_keeps_its_name_in_the_report() {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib) = laptop_admin_phone_reader();
    libraries::rename_library(&mut laptop.lock().unwrap(), &lib, "Casa").unwrap();
    let tersoal = cloud.api("tersoal").on(&laptop);
    upload_settled(&laptop, &tersoal, &lib).await;
    let hugo = cloud.api("hugo").on(&phone);
    sync_all(&phone, &hugo).await.unwrap();
    let phone_id = ops::device_id(&phone.lock().unwrap()).unwrap();
    people::remove_device(&mut laptop.lock().unwrap(), &phone_id).unwrap();
    sync_library(&laptop, &tersoal, &lib).await.unwrap();

    let reports = sync_all(&phone, &hugo).await.unwrap();

    let removed = reports
        .iter()
        .find(|r| r.outcome == Outcome::Removed)
        .unwrap();
    assert_eq!(removed.library_name.as_deref(), Some("Casa"));
}

/// The upload screen asks Rust who administers and whether there are roles; it decides nothing itself.
#[tokio::test]
async fn the_status_says_who_administers_and_whether_the_library_has_roles() {
    let (laptop, phone, lib) = laptop_admin_phone_reader();
    let plain = Database::in_memory();
    let own = first_library(&plain);
    let of = |device: &Database, library: &str| {
        let status = status(device)
            .unwrap()
            .into_iter()
            .find(|s| s.library == library)
            .unwrap();
        (status.administers, status.has_roles, status.cloud_power)
    };

    assert_eq!(of(&laptop, &lib), (true, true, false));
    assert_eq!(of(&phone, &lib), (false, true, false), "a reader");
    assert_eq!(
        of(&plain, &own),
        (true, false, false),
        "no people, no roles"
    );
}

fn sign_in_as(device: &Database, user: &str) {
    crate::account::store_session(
        &device.lock().unwrap(),
        "token",
        &crate::account::AccountUser {
            id: user.into(),
            email: format!("{user}@example.com"),
            name: user.into(),
            locale: "en".into(),
        },
        &format!("{user}@example.com"),
    )
    .unwrap();
}

/// D76: the account is Tersoal but this phone is Hugo's. The app learns it from the claim, tells it once, and
/// asks again only if another account signs in or the phone changes hands; it never changes whose it is.
#[tokio::test]
async fn a_device_of_another_person_is_asked_about_once_per_account_and_person() {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib) = laptop_admin_phone_reader();
    let tersoal = cloud.api("tersoal").on(&laptop);
    upload_settled(&laptop, &tersoal, &lib).await;
    sync_all(&laptop, &tersoal).await.unwrap();
    sign_in_as(&phone, "u1");
    let hers = cloud.api("tersoal").on(&phone);
    let phone_id = ops::device_id(&phone.lock().unwrap()).unwrap();
    let owner_before = person_of(&phone, &lib, &phone_id);
    let open = |device: &Database| other_owner::pending(&device.lock().unwrap()).unwrap();

    sync_all(&phone, &hers).await.unwrap();
    let first = open(&phone);
    assert_eq!(first.len(), 1, "{first:?}");
    assert_eq!(
        (first[0].person.as_str(), first[0].role.as_deref()),
        ("Hugo", Some("reader"))
    );
    assert_eq!(
        person_of(&phone, &lib, &phone_id),
        owner_before,
        "nothing changes"
    );

    other_owner::answer(&phone.lock().unwrap(), &lib).unwrap();
    assert!(open(&phone).is_empty());
    sync_all(&phone, &hers).await.unwrap();
    assert!(open(&phone).is_empty(), "not asked again");

    sign_in_as(&phone, "u2");
    assert!(
        open(&phone).is_empty(),
        "the answer was for u1: nothing is open until a claim says it"
    );
    sync_all(&phone, &hers).await.unwrap();
    assert_eq!(open(&phone).len(), 1, "another account is asked again");

    // The phone changes hands: Laura's now. The old question is no longer the question.
    let laura = ops::new_id();
    {
        let mut conn = laptop.lock().unwrap();
        let tx = conn.transaction().unwrap();
        ops::record(
            &tx,
            &lib,
            "member.add",
            vec![Change::create(
                Entity::Member,
                &laura,
                json!({ "name": "Laura", "role": "editor" }),
            )],
        )
        .unwrap();
        ops::record(
            &tx,
            &lib,
            "device.assign",
            vec![Change::update(
                Entity::Device,
                &phone_id,
                permissions::assignment_fields(&tx, &lib, &phone_id, Some(&laura)).unwrap(),
            )],
        )
        .unwrap();
        tx.commit().unwrap();
    }
    sync_all(&laptop, &tersoal).await.unwrap();
    sync_all(&phone, &hers).await.unwrap();
    let again = open(&phone);
    assert_eq!(again.len(), 1);
    assert_eq!(again[0].person, "Laura");
}

fn person_of(device: &Database, library: &str, id: &str) -> Option<String> {
    permissions::person_of_device(&device.lock().unwrap(), library, id).unwrap()
}
