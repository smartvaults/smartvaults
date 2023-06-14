// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::SocketAddr;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use bdk::bitcoin::psbt::PartiallySignedTransaction;
use bdk::bitcoin::{Address, Network, PrivateKey, Txid, XOnlyPublicKey};
use bdk::blockchain::Blockchain;
use bdk::blockchain::ElectrumBlockchain;
use bdk::database::{MemoryDatabase, SqliteDatabase};
use bdk::electrum_client::Client as ElectrumClient;
use bdk::miniscript::Descriptor;
use bdk::signer::{SignerContext, SignerWrapper};
use bdk::{Balance, FeeRate, SyncOptions, TransactionDetails, Wallet};
use coinstr_core::bips::bip39::Mnemonic;
use coinstr_core::reserves::{ProofError, ProofOfReserves};
use coinstr_core::signer::{SharedSigner, Signer};
use coinstr_core::types::{KeeChain, Keychain, Seed, WordCount};
use coinstr_core::util::{extract_public_keys, Serde};
use coinstr_core::{Amount, ApprovedProposal, CompletedProposal, Policy, Proposal};
use futures_util::future::{AbortHandle, Abortable};
use nostr_sdk::nips::nip04;
use nostr_sdk::nips::nip06::FromMnemonic;
use nostr_sdk::secp256k1::SecretKey;
use nostr_sdk::{
    nips, Client, Contact, Event, EventBuilder, EventId, Filter, Keys, Kind, Metadata, Options,
    Relay, RelayMessage, RelayPoolNotification, Result, Tag, TagKind, Timestamp, Url,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::constants::{
    APPROVED_PROPOSAL_EXPIRATION, APPROVED_PROPOSAL_KIND, COINSTR_ACCOUNT_INDEX,
    COMPLETED_PROPOSAL_KIND, POLICY_KIND, PROPOSAL_KIND, SHARED_KEY_KIND, SHARED_SIGNERS_KIND,
    SIGNERS_KIND,
};
use crate::db::model::{GetDetailedPolicyResult, GetNotificationsResult, GetPolicyResult};
use crate::db::store::{GetApprovedProposals, Transactions};
use crate::db::Store;
use crate::types::backup::PolicyBackup;
use crate::util::encryption::{EncryptionWithKeys, EncryptionWithKeysError};
use crate::{thread, util};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Keechain(#[from] coinstr_core::types::keychain::Error),
    #[error(transparent)]
    Dir(#[from] util::dir::Error),
    #[error(transparent)]
    Bdk(#[from] bdk::Error),
    #[error(transparent)]
    Electrum(#[from] bdk::electrum_client::Error),
    #[error(transparent)]
    Client(#[from] nostr_sdk::client::Error),
    #[error(transparent)]
    Keys(#[from] nostr_sdk::key::Error),
    #[error(transparent)]
    EventId(#[from] nostr_sdk::event::id::Error),
    #[error(transparent)]
    EventBuilder(#[from] nostr_sdk::event::builder::Error),
    #[error(transparent)]
    NIP04(#[from] nostr_sdk::nips::nip04::Error),
    #[error(transparent)]
    Policy(#[from] coinstr_core::policy::Error),
    #[error(transparent)]
    Proposal(#[from] coinstr_core::proposal::Error),
    #[error(transparent)]
    Secp256k1(#[from] coinstr_core::bitcoin::secp256k1::Error),
    #[error(transparent)]
    EncryptionWithKeys(#[from] EncryptionWithKeysError),
    #[error(transparent)]
    Util(#[from] coinstr_core::util::Error),
    #[error(transparent)]
    NIP06(#[from] nostr_sdk::nips::nip06::Error),
    #[error(transparent)]
    BIP32(#[from] coinstr_core::bitcoin::util::bip32::Error),
    #[error(transparent)]
    Proof(#[from] ProofError),
    #[error(transparent)]
    Signer(#[from] coinstr_core::signer::Error),
    #[error(transparent)]
    Store(#[from] crate::db::Error),
    #[error("password not match")]
    PasswordNotMatch,
    #[error("not enough public keys")]
    NotEnoughPublicKeys,
    #[error("shared keys not found")]
    SharedKeysNotFound,
    #[error("policy not found")]
    PolicyNotFound,
    #[error("proposal not found")]
    ProposalNotFound,
    #[error("unexpected proposal")]
    UnexpectedProposal,
    #[error("approved proposal/s not found")]
    ApprovedProposalNotFound,
    #[error("electrum endpoint not set")]
    ElectrumEndpointNotSet,
    #[error("signer not found")]
    SignerNotFound,
    #[error("signer ID not found")]
    SignerIdNotFound,
    #[error("public key not found")]
    PublicKeyNotFound,
    #[error("{0}")]
    Generic(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Notification {
    NewPolicy(EventId),
    NewProposal(EventId),
    /* NewApproval {
        proposal_id: EventId,
        approval_id: EventId,
    } */
}

impl Serde for Notification {}

#[derive(Debug, Clone)]
pub enum Message {
    Notification(Notification),
    WalletSyncCompleted(EventId),
}

fn coinstr_signer(seed: Seed, network: Network) -> Result<(EventId, Signer), Error> {
    let signer_id = EventId::from_slice(&[
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ])?;
    let signer = Signer::from_seed("Coinstr", None, seed, Some(COINSTR_ACCOUNT_INDEX), network)?;
    Ok((signer_id, signer))
}

/// Coinstr
#[derive(Debug, Clone)]
pub struct Coinstr {
    network: Network,
    keechain: KeeChain,
    client: Client,
    endpoint: Arc<RwLock<Option<String>>>,
    pub db: Store,
}

impl Coinstr {
    /// Open keychain
    pub fn open<P, S, PSW>(
        base_path: P,
        name: S,
        get_password: PSW,
        network: Network,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        S: Into<String>,
        PSW: FnOnce() -> Result<String>,
    {
        let base_path = base_path.as_ref();

        // Open keychain
        let file_path: PathBuf = util::dir::get_keychain_file(base_path, network, name)?;
        let mut keechain: KeeChain = KeeChain::open(file_path, get_password)?;
        let passphrase: Option<String> = keechain.keychain.get_passphrase(0);
        keechain.keychain.apply_passphrase(passphrase);

        // Get nostr keys
        let keys = Keys::from_mnemonic(
            keechain.keychain.seed.mnemonic().to_string(),
            keechain.keychain.seed.passphrase(),
        )?;

        // Open db
        let db = Store::open(
            util::dir::user_db(base_path, network, keys.public_key())?,
            util::dir::timechain_db(base_path, network)?,
            &keys,
            network,
        )?;

        // Load wallets
        db.load_wallets()?;

        let opts = Options::new()
            .wait_for_connection(false)
            .wait_for_send(false)
            .wait_for_subscription(false);

        Ok(Self {
            network,
            keechain,
            client: Client::with_opts(&keys, opts),
            endpoint: Arc::new(RwLock::new(None)),
            db,
        })
    }

    /// Generate keychain
    pub fn generate<P, S, PSW, PASSP>(
        base_path: P,
        name: S,
        get_password: PSW,
        word_count: WordCount,
        get_passphrase: PASSP,
        network: Network,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        S: Into<String>,
        PSW: FnOnce() -> Result<String>,
        PASSP: FnOnce() -> Result<Option<String>>,
    {
        let base_path = base_path.as_ref();

        // Generate keychain
        let file_path: PathBuf = util::dir::get_keychain_file(base_path, network, name)?;
        let mut keechain: KeeChain =
            KeeChain::generate(file_path, get_password, word_count, || Ok(None))?;
        let passphrase: Option<String> =
            get_passphrase().map_err(|e| Error::Generic(e.to_string()))?;
        if let Some(passphrase) = passphrase {
            keechain.keychain.add_passphrase(&passphrase);
            keechain.save()?;
            keechain.keychain.apply_passphrase(Some(passphrase));
        }

        // Get nostr keys
        let keys = Keys::from_mnemonic(
            keechain.keychain.seed.mnemonic().to_string(),
            keechain.keychain.seed.passphrase(),
        )?;

        // Open db
        let db = Store::open(
            util::dir::user_db(base_path, network, keys.public_key())?,
            util::dir::timechain_db(base_path, network)?,
            &keys,
            network,
        )?;

        // Load wallets
        db.load_wallets()?;

        let opts = Options::new()
            .wait_for_connection(false)
            .wait_for_send(false)
            .wait_for_subscription(false);

        Ok(Self {
            network,
            keechain,
            client: Client::with_opts(&keys, opts),
            endpoint: Arc::new(RwLock::new(None)),
            db,
        })
    }

    /// Restore keychain
    pub fn restore<P, S, PSW, M, PASSP>(
        base_path: P,
        name: S,
        get_password: PSW,
        get_mnemonic: M,
        get_passphrase: PASSP,
        network: Network,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        S: Into<String>,
        PSW: FnOnce() -> Result<String>,
        M: FnOnce() -> Result<Mnemonic>,
        PASSP: FnOnce() -> Result<Option<String>>,
    {
        let base_path = base_path.as_ref();

        // Restore keychain
        let file_path: PathBuf = util::dir::get_keychain_file(base_path, network, name)?;
        let mut keechain: KeeChain = KeeChain::restore(file_path, get_password, get_mnemonic)?;
        let passphrase: Option<String> =
            get_passphrase().map_err(|e| Error::Generic(e.to_string()))?;
        if let Some(passphrase) = passphrase {
            keechain.keychain.add_passphrase(&passphrase);
            keechain.save()?;
            keechain.keychain.apply_passphrase(Some(passphrase));
        }

        // Get nostr keys
        let keys = Keys::from_mnemonic(
            keechain.keychain.seed.mnemonic().to_string(),
            keechain.keychain.seed.passphrase(),
        )?;

        // Open db
        let db = Store::open(
            util::dir::user_db(base_path, network, keys.public_key())?,
            util::dir::timechain_db(base_path, network)?,
            &keys,
            network,
        )?;

        // Load wallets
        db.load_wallets()?;

        let opts = Options::new()
            .wait_for_connection(false)
            .wait_for_send(false)
            .wait_for_subscription(false);

        Ok(Self {
            network,
            keechain,
            client: Client::with_opts(&keys, opts),
            endpoint: Arc::new(RwLock::new(None)),
            db,
        })
    }

    pub fn list_keychains<P>(base_path: P, network: Network) -> Result<Vec<String>, Error>
    where
        P: AsRef<Path>,
    {
        Ok(util::dir::get_keychains_list(base_path, network)?)
    }

    /// Save keychain
    pub fn save(&self) -> Result<(), Error> {
        Ok(self.keechain.save()?)
    }

    /// Check keychain password
    pub fn check_password<S>(&self, password: S) -> bool
    where
        S: Into<String>,
    {
        self.keechain.check_password(password)
    }

    /// Rename keychain file
    pub fn rename<S>(&mut self, new_name: S) -> Result<(), Error>
    where
        S: Into<String>,
    {
        Ok(self.keechain.rename(new_name)?)
    }

    /// Change keychain password
    pub fn change_password<NPSW>(&mut self, get_new_password: NPSW) -> Result<(), Error>
    where
        NPSW: FnOnce() -> Result<String>,
    {
        Ok(self.keechain.change_password(get_new_password)?)
    }

    /// Permanent delete the keychain
    pub fn wipe<S>(&self, password: S) -> Result<(), Error>
    where
        S: Into<String>,
    {
        if self.check_password(password) {
            Ok(self.keechain.wipe()?)
        } else {
            Err(Error::PasswordNotMatch)
        }
    }

    pub fn keychain(&self) -> Keychain {
        self.keechain.keychain.clone()
    }

    pub fn keys(&self) -> Keys {
        self.client.keys()
    }

    pub fn network(&self) -> Network {
        self.network
    }

    pub async fn add_relay<S>(&self, url: S, proxy: Option<SocketAddr>) -> Result<(), Error>
    where
        S: Into<String>,
    {
        Ok(self.client.add_relay(url, proxy).await?)
    }

    pub async fn connect(&self) {
        self.client.connect().await;
    }

    /// Add relays
    /// Connect
    /// Rebroadcast stored events
    pub async fn add_relays_and_connect<S>(&self, relays: Vec<S>) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let relays = relays.into_iter().map(|r| (r, None)).collect();
        self.client.add_relays(relays).await?;
        self.client.connect().await;
        self.rebroadcast_all_events().await?;
        Ok(())
    }

    pub async fn remove_relay<S>(&self, url: S) -> Result<(), Error>
    where
        S: Into<String>,
    {
        Ok(self.client.remove_relay(url).await?)
    }

    pub async fn relays(&self) -> HashMap<Url, Relay> {
        self.client.relays().await
    }

    pub async fn shutdown(self) -> Result<(), Error> {
        Ok(self.client.shutdown().await?)
    }

    async fn send_event(&self, event: Event) -> Result<EventId, Error> {
        self.db.save_event(&event)?;
        Ok(self.client.send_event(event).await?)
    }

    pub fn set_electrum_endpoint<S>(&self, endpoint: S)
    where
        S: Into<String>,
    {
        let mut e = self.endpoint.write();
        *e = Some(endpoint.into());
    }

    pub fn electrum_endpoint(&self) -> Result<String, Error> {
        let endpoint = self.endpoint.read();
        endpoint.clone().ok_or(Error::ElectrumEndpointNotSet)
    }

    pub fn block_height(&self) -> u32 {
        self.db.block_height()
    }

    pub fn get_contacts(&self) -> Result<BTreeMap<XOnlyPublicKey, Metadata>, Error> {
        Ok(self.db.get_contacts_with_metadata()?)
    }

    pub async fn add_contact(&self, public_key: XOnlyPublicKey) -> Result<(), Error> {
        let mut contacts: Vec<Contact> = self
            .db
            .get_contacts_public_keys()?
            .into_iter()
            .map(|p| Contact::new::<String>(p, None, None))
            .collect();
        contacts.push(Contact::new::<String>(public_key, None, None));
        self.client.set_contact_list(contacts).await?;
        self.db.save_contact(public_key)?;
        Ok(())
    }

    pub async fn remove_contact(&self, public_key: XOnlyPublicKey) -> Result<(), Error> {
        let contacts: Vec<Contact> = self
            .db
            .get_contacts_public_keys()?
            .into_iter()
            .filter(|p| p != &public_key)
            .map(|p| Contact::new::<String>(p, None, None))
            .collect();
        self.client.set_contact_list(contacts).await?;
        self.db.delete_contact(public_key)?;
        Ok(())
    }

    pub fn get_policy_by_id(&self, policy_id: EventId) -> Result<Policy, Error> {
        Ok(self.db.get_policy(policy_id)?.policy)
    }

    pub fn get_proposal_by_id(&self, proposal_id: EventId) -> Result<(EventId, Proposal), Error> {
        Ok(self.db.get_proposal(proposal_id)?)
    }

    pub fn get_completed_proposal_by_id(
        &self,
        completed_proposal_id: EventId,
    ) -> Result<(EventId, CompletedProposal), Error> {
        Ok(self.db.get_completed_proposal(completed_proposal_id)?)
    }

    pub async fn delete_policy_by_id(
        &self,
        policy_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(), Error> {
        // Get nostr pubkeys and shared keys
        let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id)?;
        let shared_keys: Keys = self.db.get_shared_key(policy_id)?;

        // Get all events linked to the policy
        let filter = Filter::new().event(policy_id);
        let events = self.client.get_events_of(vec![filter], timeout).await?;

        let mut tags: Vec<Tag> = nostr_pubkeys
            .into_iter()
            .map(|p| Tag::PubKey(p, None))
            .collect();
        tags.push(Tag::Event(policy_id, None, None));
        events
            .into_iter()
            .for_each(|e| tags.push(Tag::Event(e.id, None, None)));

        let event = EventBuilder::new(Kind::EventDeletion, "", &tags).to_event(&shared_keys)?;
        self.send_event(event).await?;

        self.db.delete_policy(policy_id)?;

        Ok(())
    }

    pub async fn delete_proposal_by_id(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(), Error> {
        // Get the proposal
        let filter = Filter::new().id(proposal_id).kind(PROPOSAL_KIND);
        let events = self.client.get_events_of(vec![filter], timeout).await?;
        let proposal_event = events.first().ok_or(Error::ProposalNotFound)?;
        let policy_id =
            util::extract_first_event_id(proposal_event).ok_or(Error::PolicyNotFound)?;

        // Get shared key
        let shared_keys = self.db.get_shared_key(policy_id)?;

        // Extract `p` tags from proposal event to notify users about proposal deletion
        let mut tags: Vec<Tag> = util::extract_tags_by_kind(proposal_event, TagKind::P)
            .into_iter()
            .cloned()
            .collect();

        // Get all events linked to the proposal
        /* let filter = Filter::new().event(proposal_id);
        let events = self.client.get_events_of(vec![filter], timeout).await?; */

        tags.push(Tag::Event(proposal_id, None, None));
        /* let mut ids: Vec<EventId> = vec![proposal_id];

        for event in events.into_iter() {
            if event.kind != COMPLETED_PROPOSAL_KIND {
                ids.push(event.id);
            }
        } */

        let event = EventBuilder::new(Kind::EventDeletion, "", &tags).to_event(&shared_keys)?;
        self.send_event(event).await?;

        self.db.delete_proposal(proposal_id)?;

        Ok(())
    }

    pub async fn delete_completed_proposal_by_id(
        &self,
        completed_proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(), Error> {
        // Get the completed proposal
        let filter = Filter::new()
            .id(completed_proposal_id)
            .kind(COMPLETED_PROPOSAL_KIND);
        let events = self.client.get_events_of(vec![filter], timeout).await?;
        let proposal_event = events.first().ok_or(Error::ProposalNotFound)?;
        let policy_id = util::extract_tags_by_kind(proposal_event, TagKind::E)
            .get(1)
            .map(|t| {
                if let Tag::Event(event_id, ..) = t {
                    Some(event_id)
                } else {
                    None
                }
            })
            .ok_or(Error::PolicyNotFound)?
            .ok_or(Error::PolicyNotFound)?;

        // Get shared key
        let shared_keys = self.db.get_shared_key(*policy_id)?;

        // Extract `p` tags from proposal event to notify users about proposal deletion
        let mut tags: Vec<Tag> = util::extract_tags_by_kind(proposal_event, TagKind::P)
            .into_iter()
            .cloned()
            .collect();

        tags.push(Tag::Event(completed_proposal_id, None, None));

        let event = EventBuilder::new(Kind::EventDeletion, "", &tags).to_event(&shared_keys)?;
        self.send_event(event).await?;

        self.db.delete_completed_proposal(completed_proposal_id)?;

        Ok(())
    }

    pub async fn delete_signer_by_id(
        &self,
        signer_id: EventId,
        _timeout: Option<Duration>,
    ) -> Result<(), Error> {
        let keys = self.client.keys();

        // TODO: delete shared signers and notify the interested users (tag pubkey in deletion event)

        let event = EventBuilder::new(Kind::EventDeletion, "", &[]).to_event(&keys)?;
        self.send_event(event).await?;

        self.db.delete_signer(signer_id)?;

        Ok(())
    }

    pub fn get_policies(&self) -> Result<BTreeMap<EventId, GetPolicyResult>, Error> {
        Ok(self.db.get_policies()?)
    }

    pub fn get_detailed_policies(
        &self,
    ) -> Result<BTreeMap<EventId, GetDetailedPolicyResult>, Error> {
        Ok(self.db.get_detailed_policies()?)
    }

    pub fn get_proposals(&self) -> Result<BTreeMap<EventId, (EventId, Proposal)>, Error> {
        Ok(self.db.get_proposals()?)
    }

    pub fn get_completed_proposals(
        &self,
    ) -> Result<BTreeMap<EventId, (EventId, CompletedProposal)>, Error> {
        Ok(self.db.completed_proposals()?)
    }

    pub fn wallet<S>(
        &self,
        policy_id: EventId,
        descriptor: S,
    ) -> Result<Wallet<SqliteDatabase>, Error>
    where
        S: Into<String>,
    {
        let db: SqliteDatabase = self.db.get_wallet_db(policy_id)?;
        let wallet = Wallet::new(&descriptor.into(), None, self.network, db)?;
        Ok(wallet)
    }

    fn memory_wallet<S>(&self, descriptor: S) -> Result<Wallet<MemoryDatabase>, Error>
    where
        S: Into<String>,
    {
        let db = MemoryDatabase::new();
        Ok(Wallet::new(&descriptor.into(), None, self.network, db)?)
    }

    pub async fn save_policy<S>(
        &self,
        name: S,
        description: S,
        descriptor: S,
        custom_pubkeys: Option<Vec<XOnlyPublicKey>>,
    ) -> Result<EventId, Error>
    where
        S: Into<String>,
    {
        let keys = self.client.keys();
        let descriptor = descriptor.into();

        let nostr_pubkeys: Vec<XOnlyPublicKey> = match custom_pubkeys {
            Some(pubkeys) => pubkeys,
            None => extract_public_keys(&descriptor)?,
        };

        if nostr_pubkeys.len() < 2 {
            return Err(Error::NotEnoughPublicKeys);
        }

        // Generate a shared key
        let shared_key = Keys::generate();
        let policy = Policy::from_desc_or_policy(name, description, descriptor)?;

        // Compose the event
        let content: String = policy.encrypt_with_keys(&shared_key)?;
        let tags: Vec<Tag> = nostr_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        // Publish policy with `shared_key` so every owner can delete it
        let policy_event = EventBuilder::new(POLICY_KIND, content, &tags).to_event(&shared_key)?;
        let policy_id = policy_event.id;

        // Publish the shared key
        for pubkey in nostr_pubkeys.iter() {
            let encrypted_shared_key = nips::nip04::encrypt(
                &keys.secret_key()?,
                pubkey,
                shared_key.secret_key()?.display_secret().to_string(),
            )?;
            let event: Event = EventBuilder::new(
                SHARED_KEY_KIND,
                encrypted_shared_key,
                &[
                    Tag::Event(policy_id, None, None),
                    Tag::PubKey(*pubkey, None),
                ],
            )
            .to_event(&keys)?;
            let event_id: EventId = self.send_event(event).await?;
            log::info!("Published shared key for {pubkey} at event {event_id}");
        }

        // Publish the event
        self.send_event(policy_event).await?;

        // Cache policy
        self.db.save_shared_key(policy_id, shared_key)?;
        self.db.save_policy(policy_id, policy, nostr_pubkeys)?;

        Ok(policy_id)
    }

    /// Make a spending proposal
    pub async fn spend<S>(
        &self,
        policy_id: EventId,
        address: Address,
        amount: Amount,
        description: S,
        fee_rate: FeeRate,
    ) -> Result<(EventId, Proposal), Error>
    where
        S: Into<String>,
    {
        // Get policy and shared keys
        let policy: Policy = self.get_policy_by_id(policy_id)?;
        let shared_keys: Keys = self.db.get_shared_key(policy_id)?;

        let description: &str = &description.into();

        // Build spending proposal
        let wallet: Wallet<SqliteDatabase> =
            self.wallet(policy_id, &policy.descriptor.to_string())?;
        let proposal = policy.spend(wallet, address, amount, description, fee_rate)?;

        if let Proposal::Spending {
            amount,
            description,
            ..
        } = &proposal
        {
            // Compose the event
            let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id)?;
            let mut tags: Vec<Tag> = nostr_pubkeys
                .iter()
                .map(|p| Tag::PubKey(*p, None))
                .collect();
            tags.push(Tag::Event(policy_id, None, None));
            let content: String = proposal.encrypt_with_keys(&shared_keys)?;
            // Publish proposal with `shared_key` so every owner can delete it
            let event = EventBuilder::new(PROPOSAL_KIND, content, &tags).to_event(&shared_keys)?;
            let proposal_id = self.send_event(event).await?;

            // Send DM msg
            let sender = self.client.keys().public_key();
            let mut msg = String::from("New spending proposal:\n");
            msg.push_str(&format!(
                "- Amount: {} sat\n",
                util::format::big_number(*amount)
            ));
            msg.push_str(&format!("- Description: {description}"));
            for pubkey in nostr_pubkeys.into_iter() {
                if sender != pubkey {
                    self.client.send_direct_msg(pubkey, &msg).await?;
                }
            }

            // Cache proposal
            self.db
                .save_proposal(proposal_id, policy_id, proposal.clone())?;

            Ok((proposal_id, proposal))
        } else {
            Err(Error::UnexpectedProposal)
        }
    }

    fn is_internal_key<S>(&self, descriptor: S) -> Result<bool, Error>
    where
        S: Into<String>,
    {
        let descriptor = descriptor.into();
        let keys = self.client.keys();
        Ok(
            descriptor.starts_with(&format!("tr({}", keys.normalized_public_key()?))
                || descriptor.starts_with(&format!("tr({}", keys.public_key())),
        )
    }

    pub async fn approve(
        &self,
        proposal_id: EventId,
    ) -> Result<(EventId, ApprovedProposal), Error> {
        // Get proposal and policy
        let (policy_id, proposal) = self.get_proposal_by_id(proposal_id)?;
        let policy: Policy = self.get_policy_by_id(policy_id)?;

        // Sign PSBT
        // Custom signer
        let keys = self.client.keys();
        let signer = SignerWrapper::new(
            PrivateKey::new(keys.secret_key()?, self.network),
            SignerContext::Tap {
                is_internal_key: self.is_internal_key(policy.descriptor.to_string())?,
            },
        );
        let seed: Seed = self.keechain.keychain.seed();
        let approved_proposal = proposal.approve(&seed, vec![signer], self.network)?;

        // Get shared keys
        let shared_keys: Keys = self.db.get_shared_key(policy_id)?;

        // Compose the event
        let content = approved_proposal.encrypt_with_keys(&shared_keys)?;
        let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id)?;
        let mut tags: Vec<Tag> = nostr_pubkeys
            .into_iter()
            .map(|p| Tag::PubKey(p, None))
            .collect();
        tags.push(Tag::Event(proposal_id, None, None));
        tags.push(Tag::Event(policy_id, None, None));
        tags.push(Tag::Expiration(
            Timestamp::now().add(APPROVED_PROPOSAL_EXPIRATION),
        ));

        let event = EventBuilder::new(APPROVED_PROPOSAL_KIND, content, &tags).to_event(&keys)?;
        let timestamp = event.created_at;

        // Publish the event
        let event_id = self.send_event(event).await?;

        // Cache approved proposal
        self.db.save_approved_proposal(
            proposal_id,
            keys.public_key(),
            event_id,
            approved_proposal.clone(),
            timestamp,
        )?;

        Ok((event_id, approved_proposal))
    }

    pub async fn approve_with_signed_psbt(
        &self,
        proposal_id: EventId,
        signed_psbt: PartiallySignedTransaction,
    ) -> Result<(EventId, ApprovedProposal), Error> {
        let keys = self.client.keys();

        // Get proposal and policy
        let (policy_id, proposal) = self.get_proposal_by_id(proposal_id)?;

        let approved_proposal = proposal.approve_with_signed_psbt(signed_psbt)?;

        // Get shared keys
        let shared_keys: Keys = self.db.get_shared_key(policy_id)?;

        // Compose the event
        let content = approved_proposal.encrypt_with_keys(&shared_keys)?;
        let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id)?;
        let mut tags: Vec<Tag> = nostr_pubkeys
            .into_iter()
            .map(|p| Tag::PubKey(p, None))
            .collect();
        tags.push(Tag::Event(proposal_id, None, None));
        tags.push(Tag::Event(policy_id, None, None));
        tags.push(Tag::Expiration(
            Timestamp::now().add(APPROVED_PROPOSAL_EXPIRATION),
        ));

        let event = EventBuilder::new(APPROVED_PROPOSAL_KIND, content, &tags).to_event(&keys)?;
        let timestamp = event.created_at;

        // Publish the event
        let event_id = self.send_event(event).await?;

        // Cache approved proposal
        self.db.save_approved_proposal(
            proposal_id,
            keys.public_key(),
            event_id,
            approved_proposal.clone(),
            timestamp,
        )?;

        Ok((event_id, approved_proposal))
    }

    #[cfg(feature = "hwi")]
    pub async fn approve_with_hwi_signer(
        &self,
        proposal_id: EventId,
        signer: Signer,
    ) -> Result<(EventId, ApprovedProposal), Error> {
        let keys = self.client.keys();

        // Get proposal and policy
        let (policy_id, proposal) = self.get_proposal_by_id(proposal_id)?;

        let approved_proposal = proposal.approve_with_hwi_signer(signer, self.network)?;

        // Get shared keys
        let shared_keys: Keys = self.db.get_shared_key(policy_id)?;

        // Compose the event
        let content = approved_proposal.encrypt_with_keys(&shared_keys)?;
        let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id)?;
        let mut tags: Vec<Tag> = nostr_pubkeys
            .into_iter()
            .map(|p| Tag::PubKey(p, None))
            .collect();
        tags.push(Tag::Event(proposal_id, None, None));
        tags.push(Tag::Event(policy_id, None, None));
        tags.push(Tag::Expiration(
            Timestamp::now().add(APPROVED_PROPOSAL_EXPIRATION),
        ));

        let event = EventBuilder::new(APPROVED_PROPOSAL_KIND, content, &tags).to_event(&keys)?;
        let timestamp = event.created_at;

        // Publish the event
        let event_id = self.send_event(event).await?;

        // Cache approved proposal
        self.db.save_approved_proposal(
            proposal_id,
            keys.public_key(),
            event_id,
            approved_proposal.clone(),
            timestamp,
        )?;

        Ok((event_id, approved_proposal))
    }

    pub async fn finalize(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<CompletedProposal, Error> {
        // Get PSBTs
        let GetApprovedProposals {
            policy_id,
            proposal,
            approved_proposals,
        } = self.db.get_approved_proposals_by_id(proposal_id)?;

        let shared_keys = self.db.get_shared_key(policy_id)?;
        let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id)?;

        // Finalize proposal
        let completed_proposal: CompletedProposal =
            proposal.finalize(approved_proposals, self.network)?;

        // Broadcast
        if let CompletedProposal::Spending { tx, .. } = &completed_proposal {
            let endpoint = self.electrum_endpoint()?;
            let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&endpoint)?);
            blockchain.broadcast(tx)?;
            self.db.schedule_for_sync(policy_id)?;
        }

        // Compose the event
        let content: String = completed_proposal.encrypt_with_keys(&shared_keys)?;
        let mut tags: Vec<Tag> = nostr_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        tags.push(Tag::Event(proposal_id, None, None));
        tags.push(Tag::Event(policy_id, None, None));
        let event =
            EventBuilder::new(COMPLETED_PROPOSAL_KIND, content, &tags).to_event(&shared_keys)?;

        // Publish the event
        let event_id = self.send_event(event).await?;

        // Delete the proposal
        if let Err(e) = self.delete_proposal_by_id(proposal_id, timeout).await {
            log::error!("Impossibe to delete proposal {proposal_id}: {e}");
        }

        // Cache
        self.db.delete_proposal(proposal_id)?;
        self.db
            .save_completed_proposal(event_id, policy_id, completed_proposal.clone())?;

        Ok(completed_proposal)
    }

    pub async fn new_proof_proposal<S>(
        &self,
        policy_id: EventId,
        message: S,
    ) -> Result<(EventId, Proposal, EventId), Error>
    where
        S: Into<String>,
    {
        let message: &str = &message.into();

        // Get policy and shared keys
        let policy: Policy = self.get_policy_by_id(policy_id)?;
        let shared_keys = self.db.get_shared_key(policy_id)?;

        // Build proposal
        let wallet: Wallet<SqliteDatabase> =
            self.wallet(policy_id, &policy.descriptor.to_string())?;
        let proposal = policy.proof_of_reserve(wallet, message)?;

        // Compose the event
        let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id)?;
        let mut tags: Vec<Tag> = nostr_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        tags.push(Tag::Event(policy_id, None, None));
        let content = proposal.encrypt_with_keys(&shared_keys)?;
        // Publish proposal with `shared_key` so every owner can delete it
        let event = EventBuilder::new(PROPOSAL_KIND, content, &tags).to_event(&shared_keys)?;
        let proposal_id = self.send_event(event).await?;

        // Send DM msg
        let sender = self.client.keys().public_key();
        let mut msg = String::from("New Proof of Reserve request:\n");
        msg.push_str(&format!("- Message: {message}"));
        for pubkey in nostr_pubkeys.into_iter() {
            if sender != pubkey {
                self.client.send_direct_msg(pubkey, &msg).await?;
            }
        }

        // Cache proposal
        self.db
            .save_proposal(proposal_id, policy_id, proposal.clone())?;

        Ok((proposal_id, proposal, policy_id))
    }

    pub fn verify_proof(&self, proposal: CompletedProposal) -> Result<u64, Error> {
        if let CompletedProposal::ProofOfReserve {
            message,
            descriptor,
            psbt,
            ..
        } = proposal
        {
            let endpoint = self.electrum_endpoint()?;
            let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&endpoint)?);
            let wallet = self.memory_wallet(descriptor.to_string())?;
            wallet.sync(&blockchain, SyncOptions::default())?;
            Ok(wallet.verify_proof(&psbt, message, None)?)
        } else {
            Err(Error::UnexpectedProposal)
        }
    }

    pub fn verify_proof_by_id(&self, proposal_id: EventId) -> Result<u64, Error> {
        let (_policy_id, proposal) = self.get_completed_proposal_by_id(proposal_id)?;
        self.verify_proof(proposal)
    }

    pub async fn save_signer(&self, signer: Signer) -> Result<EventId, Error> {
        let keys = self.client.keys();

        // Compose the event
        let content: String = signer.encrypt_with_keys(&keys)?;

        // Compose signer event
        let event = EventBuilder::new(SIGNERS_KIND, content, &[]).to_event(&keys)?;

        // Publish the event
        let signer_id = self.send_event(event).await?;

        // Save signer in db
        self.db.save_signer(signer_id, signer)?;

        Ok(signer_id)
    }

    pub fn get_signers(&self) -> Result<BTreeMap<EventId, Signer>, Error> {
        let mut signers = self.db.get_signers()?;
        let (signer_id, signer) = coinstr_signer(self.keechain.keychain.seed(), self.network)?;
        signers.insert(signer_id, signer);
        Ok(signers)
    }

    pub fn search_signer_by_descriptor(
        &self,
        descriptor: Descriptor<String>,
    ) -> Result<Signer, Error> {
        let descriptor: String = descriptor.to_string();
        for signer in self.db.get_signers()?.into_values() {
            let signer_descriptor = signer.descriptor_public_key()?.to_string();
            if descriptor.contains(&signer_descriptor) {
                return Ok(signer);
            }
        }
        let (_, signer) = coinstr_signer(self.keechain.keychain.seed(), self.network)?;
        Ok(signer)
    }

    fn sync_with_timechain(&self, sender: Sender<Option<Message>>) -> AbortHandle {
        let this = self.clone();
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let timechain_sync = async move {
            let blockchain: ElectrumBlockchain;
            loop {
                match this.electrum_endpoint() {
                    Ok(endpoint) => match ElectrumClient::new(&endpoint) {
                        Ok(client) => {
                            blockchain = ElectrumBlockchain::from(client);
                            break;
                        }
                        Err(e) => {
                            log::error!("Impossible to connect to electrum server: {e}");
                            thread::sleep(Duration::from_secs(10)).await;
                        }
                    },
                    Err(_) => {
                        log::warn!("Waiting for an electrum endpoint");
                        thread::sleep(Duration::from_secs(3)).await;
                    }
                }
            }

            loop {
                if let Err(e) = this
                    .db
                    .sync_with_timechain(&blockchain, Some(&sender), false)
                {
                    log::error!("Impossible to sync wallets: {e}");
                }
                thread::sleep(Duration::from_secs(3)).await;
            }
        };

        let future = Abortable::new(timechain_sync, abort_registration);
        thread::spawn(async {
            let _ = future.await;
            log::debug!("Exited from wallet sync thread");
        });

        abort_handle
    }

    fn handle_pending_events(&self, sender: Sender<Option<Message>>) -> AbortHandle {
        let this = self.clone();
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let fut = async move {
            loop {
                match this.db.get_pending_events() {
                    Ok(events) => {
                        for event in events.into_iter() {
                            let event_id = event.id;
                            match this.handle_event(event).await {
                                Ok(notification) => {
                                    sender.try_send(notification).ok();
                                }
                                Err(e) => {
                                    log::error!(
                                        "Impossible to handle pending event {event_id}: {e}"
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => log::error!("Impossible to get pending events: {e}"),
                }
                thread::sleep(Duration::from_secs(30)).await;
            }
        };

        let future = Abortable::new(fut, abort_registration);
        thread::spawn(async {
            let _ = future.await;
            log::debug!("Exited from pending events handler thread");
        });

        abort_handle
    }

    pub fn sync(&self) -> Receiver<Option<Message>> {
        let (sender, receiver) = mpsc::channel::<Option<Message>>(1024);
        let this = self.clone();
        thread::spawn(async move {
            // Sync timechain
            let timechain_sync: AbortHandle = this.sync_with_timechain(sender.clone());

            // Pending events handler
            let pending_event_handler = this.handle_pending_events(sender.clone());

            let keys = this.client.keys();

            let base_filter = Filter::new().kinds(vec![
                POLICY_KIND,
                PROPOSAL_KIND,
                APPROVED_PROPOSAL_KIND,
                COMPLETED_PROPOSAL_KIND,
                SHARED_KEY_KIND,
                SIGNERS_KIND,
                SHARED_SIGNERS_KIND,
                Kind::EventDeletion,
            ]);

            for (relay_url, relay) in this.client.relays().await {
                let last_sync: Timestamp = match this.db.get_last_relay_sync(&relay_url) {
                    Ok(ts) => ts,
                    Err(e) => {
                        log::error!("Impossible to get last relay sync: {e}");
                        Timestamp::from(0)
                    }
                };
                let author_filter = base_filter
                    .clone()
                    .author(keys.public_key().to_string())
                    .since(last_sync);
                let pubkey_filter = base_filter
                    .clone()
                    .pubkey(keys.public_key())
                    .since(last_sync);
                let contacts_filters = Filter::new()
                    .author(keys.public_key().to_string())
                    .kind(Kind::ContactList)
                    .since(last_sync);
                if let Err(e) = relay
                    .subscribe(vec![author_filter, pubkey_filter, contacts_filters], false)
                    .await
                {
                    log::error!("Impossible to subscribe to {relay_url}: {e}");
                }
            }

            let _ = this
                .client
                .handle_notifications(|notification| async {
                    match notification {
                        RelayPoolNotification::Event(_, event) => {
                            let event_id = event.id;
                            if event.is_expired() {
                                log::warn!("Event {event_id} expired");
                            } else {
                                match this.handle_event(event).await {
                                    Ok(notification) => {
                                        sender.try_send(notification).ok();
                                    }
                                    Err(e) => {
                                        log::error!("Impossible to handle event {event_id}: {e}");
                                    }
                                }
                            }
                        }
                        RelayPoolNotification::Message(relay_url, relay_msg) => {
                            if let RelayMessage::EndOfStoredEvents(subscription_id) = relay_msg {
                                log::debug!("Received new EOSE for {relay_url} with subid {subscription_id}");
                                if let Ok(relay) = this.client.relay(&relay_url).await {
                                    let subscription = relay.subscription().await;
                                    if subscription.id() == subscription_id {
                                        if let Err(e) = this
                                            .db
                                            .save_last_relay_sync(&relay_url, Timestamp::now())
                                        {
                                            log::error!("Impossible to save last relay sync: {e}");
                                        }
                                    }
                                }
                            }
                        }
                        RelayPoolNotification::Shutdown => {
                            log::debug!("Received shutdown msg");
                            timechain_sync.abort();
                            pending_event_handler.abort();
                        }
                    }

                    Ok(())
                })
                .await;
            log::debug!("Exited from nostr sync thread");
        });
        receiver
    }

    async fn handle_event(&self, event: Event) -> Result<Option<Message>> {
        if let Err(e) = self.db.save_event(&event) {
            log::error!("Impossible to save event {}: {e}", event.id);
        }

        if event.kind == SHARED_KEY_KIND {
            let policy_id = util::extract_first_event_id(&event).ok_or(Error::PolicyNotFound)?;
            if !self.db.shared_key_exists_for_policy(policy_id)? {
                let keys = self.client.keys();
                let content =
                    nips::nip04::decrypt(&keys.secret_key()?, &event.pubkey, &event.content)?;
                let sk = SecretKey::from_str(&content)?;
                let shared_key = Keys::new(sk);
                self.db.save_shared_key(policy_id, shared_key)?;
            }
        } else if event.kind == POLICY_KIND && !self.db.policy_exists(event.id)? {
            if let Ok(shared_key) = self.db.get_shared_key(event.id) {
                let policy = Policy::decrypt_with_keys(&shared_key, &event.content)?;
                let mut nostr_pubkeys: Vec<XOnlyPublicKey> = Vec::new();
                for tag in event.tags.iter() {
                    if let Tag::PubKey(pubkey, ..) = tag {
                        nostr_pubkeys.push(*pubkey);
                    }
                }
                if nostr_pubkeys.is_empty() {
                    log::error!("Policy {} not contains any nostr pubkey", event.id);
                } else {
                    self.db.save_policy(event.id, policy, nostr_pubkeys)?;
                    let notification = Notification::NewPolicy(event.id);
                    self.db.save_notification(notification)?;
                    return Ok(Some(Message::Notification(notification)));
                }
            } else {
                self.db.save_pending_event(&event)?;
            }
        } else if event.kind == PROPOSAL_KIND && !self.db.proposal_exists(event.id)? {
            if let Some(policy_id) = util::extract_first_event_id(&event) {
                if let Ok(shared_key) = self.db.get_shared_key(policy_id) {
                    let proposal = Proposal::decrypt_with_keys(&shared_key, &event.content)?;
                    self.db.save_proposal(event.id, policy_id, proposal)?;
                    let notification = Notification::NewProposal(event.id);
                    self.db.save_notification(notification)?;
                    return Ok(Some(Message::Notification(notification)));
                } else {
                    self.db.save_pending_event(&event)?;
                }
            } else {
                log::error!("Impossible to find policy id in proposal {}", event.id);
            }
        } else if event.kind == APPROVED_PROPOSAL_KIND {
            if let Some(proposal_id) = util::extract_first_event_id(&event) {
                if let Some(Tag::Event(policy_id, ..)) =
                    util::extract_tags_by_kind(&event, TagKind::E).get(1)
                {
                    if let Ok(shared_key) = self.db.get_shared_key(*policy_id) {
                        let approved_proposal =
                            ApprovedProposal::decrypt_with_keys(&shared_key, &event.content)?;
                        self.db.save_approved_proposal(
                            proposal_id,
                            event.pubkey,
                            event.id,
                            approved_proposal,
                            event.created_at,
                        )?;
                    } else {
                        self.db.save_pending_event(&event)?;
                    }
                } else {
                    log::error!("Impossible to find policy id in proposal {}", event.id);
                }
            } else {
                log::error!(
                    "Impossible to find proposal id in approved proposal {}",
                    event.id
                );
            }
        } else if event.kind == COMPLETED_PROPOSAL_KIND {
            if let Some(proposal_id) = util::extract_first_event_id(&event) {
                self.db.delete_proposal(proposal_id)?;
                if let Some(Tag::Event(policy_id, ..)) =
                    util::extract_tags_by_kind(&event, TagKind::E).get(1)
                {
                    // Schedule policy for sync if the event was created in the last 60 secs
                    if event.created_at.add(Duration::from_secs(60)) >= Timestamp::now() {
                        self.db.schedule_for_sync(*policy_id)?;
                    }

                    if let Ok(shared_key) = self.db.get_shared_key(*policy_id) {
                        let completed_proposal =
                            CompletedProposal::decrypt_with_keys(&shared_key, &event.content)?;
                        self.db.save_completed_proposal(
                            event.id,
                            *policy_id,
                            completed_proposal,
                        )?;
                    } else {
                        self.db.save_pending_event(&event)?;
                    }
                } else {
                    log::error!(
                        "Impossible to find policy id in completed proposal {}",
                        event.id
                    );
                }
            }
        } else if event.kind == SIGNERS_KIND {
            let keys = self.client.keys();
            let signer = Signer::decrypt_with_keys(&keys, event.content)?;
            self.db.save_signer(event.id, signer)?;
        } else if event.kind == SHARED_SIGNERS_KIND {
            let public_key =
                util::extract_first_public_key(&event).ok_or(Error::PublicKeyNotFound)?;
            let keys = self.client.keys();
            if event.pubkey == keys.public_key() {
                let signer_id =
                    util::extract_first_event_id(&event).ok_or(Error::SignerIdNotFound)?;
                self.db
                    .save_my_shared_signer(signer_id, event.id, public_key)?;
            } else {
                let shared_signer =
                    nip04::decrypt(&keys.secret_key()?, &public_key, event.content)?;
                let shared_signer = SharedSigner::from_json(shared_signer)?;
                self.db
                    .save_shared_signer(event.id, event.pubkey, shared_signer)?;
            }
        } else if event.kind == Kind::EventDeletion {
            for tag in event.tags.iter() {
                if let Tag::Event(event_id, ..) = tag {
                    self.db.delete_generic_event_id(*event_id)?;
                }
            }
        } else if event.kind == Kind::ContactList {
            let mut contacts = HashSet::new();
            for tag in event.tags.into_iter() {
                if let Tag::ContactList { pk, .. } = tag {
                    contacts.insert(pk);
                }
            }
            self.db.save_contacts(contacts)?;
        }

        Ok(None)
    }

    pub fn get_balance(&self, policy_id: EventId) -> Option<Balance> {
        self.db.get_balance(policy_id)
    }

    pub fn get_txs_with_descriptions(&self, policy_id: EventId) -> Option<Transactions> {
        self.db.get_txs_with_descriptions(policy_id)
    }

    pub fn get_last_unused_address(&self, policy_id: EventId) -> Option<Address> {
        self.db.get_last_unused_address(policy_id)
    }

    pub fn get_total_balance(&self) -> Result<(Balance, bool), Error> {
        Ok(self.db.get_total_balance()?)
    }

    pub fn get_all_transactions(&self) -> Result<Vec<(TransactionDetails, Option<String>)>, Error> {
        Ok(self.db.get_all_transactions()?)
    }

    pub fn get_tx(&self, txid: Txid) -> Option<(TransactionDetails, Option<String>)> {
        self.db.get_tx(txid)
    }

    pub async fn rebroadcast_all_events(&self) -> Result<(), Error> {
        let events: Vec<Event> = self.db.get_events()?;
        for event in events.into_iter() {
            self.client.send_event(event).await?;
        }
        Ok(())
    }

    pub async fn republish_shared_key_for_policy(&self, policy_id: EventId) -> Result<(), Error> {
        let keys = self.client.keys();
        let shared_key = self.db.get_shared_key(policy_id)?;
        let pubkeys = self.db.get_nostr_pubkeys(policy_id)?;
        // Publish the shared key
        for pubkey in pubkeys.iter() {
            let encrypted_shared_key = nips::nip04::encrypt(
                &keys.secret_key()?,
                pubkey,
                shared_key.secret_key()?.display_secret().to_string(),
            )?;
            let event: Event = EventBuilder::new(
                SHARED_KEY_KIND,
                encrypted_shared_key,
                &[
                    Tag::Event(policy_id, None, None),
                    Tag::PubKey(*pubkey, None),
                ],
            )
            .to_event(&keys)?;
            let event_id: EventId = self.send_event(event).await?;
            log::info!("Published shared key for {pubkey} at event {event_id}");
        }
        Ok(())
    }

    pub fn export_policy_backup(&self, policy_id: EventId) -> Result<PolicyBackup, Error> {
        let GetPolicyResult { policy, .. } = self.db.get_policy(policy_id)?;
        let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id)?;
        Ok(PolicyBackup::new(
            policy.name,
            policy.description,
            policy.descriptor,
            nostr_pubkeys,
        ))
    }

    pub fn save_policy_backup<P>(&self, policy_id: EventId, path: P) -> Result<(), Error>
    where
        P: AsRef<Path>,
    {
        let backup = self.export_policy_backup(policy_id)?;
        backup.save(path)?;
        Ok(())
    }

    pub async fn share_signer(
        &self,
        public_key: XOnlyPublicKey,
        signer_id: EventId,
        shared_signer: SharedSigner,
    ) -> Result<EventId, Error> {
        let keys = self.client.keys();
        let content = nip04::encrypt(&keys.secret_key()?, &public_key, shared_signer.as_json())?;
        let tags = &[
            Tag::Event(signer_id, None, None),
            Tag::PubKey(public_key, None),
        ];
        let event = EventBuilder::new(SHARED_SIGNERS_KIND, content, tags).to_event(&keys)?;
        Ok(self.client.send_event(event).await?)
    }

    pub async fn revoke_shared_signer(&self, shared_signer_id: EventId) -> Result<(), Error> {
        let keys = self.client.keys();
        let public_key = self
            .db
            .get_public_key_for_my_shared_signer(shared_signer_id)?;
        let tags = &[
            Tag::PubKey(public_key, None),
            Tag::Event(shared_signer_id, None, None),
        ];
        let event = EventBuilder::new(Kind::EventDeletion, "", tags).to_event(&keys)?;
        self.send_event(event).await?;
        Ok(())
    }

    pub fn get_notifications(&self) -> Result<Vec<GetNotificationsResult>, Error> {
        Ok(self.db.get_notifications()?)
    }

    pub fn count_unseen_notifications(&self) -> Result<usize, Error> {
        Ok(self.db.count_unseen_notifications()?)
    }

    pub fn mark_all_notifications_as_seen(&self) -> Result<(), Error> {
        Ok(self.db.mark_all_notifications_as_seen()?)
    }

    pub fn mark_notification_as_seen(&self, notification: Notification) -> Result<(), Error> {
        Ok(self.db.mark_notification_as_seen(notification)?)
    }
}
