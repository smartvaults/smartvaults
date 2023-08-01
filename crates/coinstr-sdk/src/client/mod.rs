// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use bdk::bitcoin::psbt::PartiallySignedTransaction;
use bdk::bitcoin::{Address, Network, OutPoint, PrivateKey, Txid, XOnlyPublicKey};
use bdk::blockchain::Blockchain;
use bdk::blockchain::ElectrumBlockchain;
use bdk::database::SqliteDatabase;
use bdk::electrum_client::Client as ElectrumClient;
use bdk::miniscript::Descriptor;
use bdk::signer::{SignerContext, SignerWrapper};
use bdk::wallet::AddressIndex;
use bdk::{Balance, TransactionDetails, Wallet};
use coinstr_core::bips::bip39::Mnemonic;
use coinstr_core::reserves::{ProofError, ProofOfReserves};
use coinstr_core::signer::{coinstr_signer, SharedSigner, Signer};
use coinstr_core::types::{KeeChain, Keychain, Seed, WordCount};
use coinstr_core::util::Serde;
use coinstr_core::{Amount, ApprovedProposal, CompletedProposal, FeeRate, Policy, Proposal};

use nostr_sdk::nips::nip04;
use nostr_sdk::nips::nip06::FromMnemonic;
use nostr_sdk::nips::nip46::{Message as NIP46Message, Request as NIP46Request};
use nostr_sdk::prelude::NostrConnectURI;
use nostr_sdk::{
    nips, Client, ClientMessage, Contact, Event, EventBuilder, EventId, Keys, Kind, Metadata,
    Options, Relay, RelayPoolNotification, Result, Tag, TagKind, Timestamp, Url,
};
use tokio::sync::broadcast::{self, Sender};

mod label;
mod sync;

use crate::config::Config;
use crate::constants::{
    APPROVED_PROPOSAL_EXPIRATION, APPROVED_PROPOSAL_KIND, COMPLETED_PROPOSAL_KIND,
    CONNECT_SEND_TIMEOUT, POLICY_KIND, PROPOSAL_KIND, SEND_TIMEOUT, SHARED_KEY_KIND,
    SHARED_SIGNERS_KIND, SIGNERS_KIND,
};
use crate::db::model::{
    GetAddress, GetAllSigners, GetApprovedProposalResult, GetApprovedProposals,
    GetCompletedProposal, GetDetailedPolicyResult, GetNotificationsResult, GetPolicy, GetProposal,
    GetSharedSignerResult, GetUtxo, NostrConnectRequest,
};
use crate::db::store::{Store, Transactions};
use crate::types::{Notification, PolicyBackup};
use crate::util;
use crate::util::encryption::{EncryptionWithKeys, EncryptionWithKeysError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Keechain(#[from] coinstr_core::types::keechain::Error),
    #[error(transparent)]
    Keychain(#[from] coinstr_core::types::keychain::Error),
    #[error(transparent)]
    Dir(#[from] util::dir::Error),
    #[error(transparent)]
    Bdk(#[from] bdk::Error),
    #[error(transparent)]
    Electrum(#[from] bdk::electrum_client::Error),
    #[error(transparent)]
    Url(#[from] nostr_sdk::url::ParseError),
    #[error(transparent)]
    Client(#[from] nostr_sdk::client::Error),
    #[error(transparent)]
    Keys(#[from] nostr_sdk::key::Error),
    #[error(transparent)]
    EventId(#[from] nostr_sdk::event::id::Error),
    #[error(transparent)]
    EventBuilder(#[from] nostr_sdk::event::builder::Error),
    #[error(transparent)]
    Relay(#[from] nostr_sdk::relay::Error),
    #[error(transparent)]
    Policy(#[from] coinstr_core::policy::Error),
    #[error(transparent)]
    Proposal(#[from] coinstr_core::proposal::Error),
    #[error(transparent)]
    Secp256k1(#[from] coinstr_core::bitcoin::secp256k1::Error),
    #[error(transparent)]
    EncryptionWithKeys(#[from] EncryptionWithKeysError),
    #[error(transparent)]
    NIP04(#[from] nostr_sdk::nips::nip04::Error),
    #[error(transparent)]
    NIP06(#[from] nostr_sdk::nips::nip06::Error),
    #[error(transparent)]
    NIP46(#[from] nostr_sdk::nips::nip46::Error),
    #[error(transparent)]
    BIP32(#[from] coinstr_core::bitcoin::util::bip32::Error),
    #[error(transparent)]
    Proof(#[from] ProofError),
    #[error(transparent)]
    Signer(#[from] coinstr_core::signer::Error),
    #[error(transparent)]
    Config(#[from] crate::config::Error),
    #[error(transparent)]
    Store(#[from] crate::db::Error),
    #[error(transparent)]
    Label(#[from] crate::types::label::Error),
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
    #[error("signer not found")]
    SignerNotFound,
    #[error("signer ID not found")]
    SignerIdNotFound,
    #[error("public key not found")]
    PublicKeyNotFound,
    #[error("signer already shared")]
    SignerAlreadyShared,
    #[error("signer descriptor already exists")]
    SignerDescriptorAlreadyExists,
    #[error("nostr connect request already approved")]
    NostrConnectRequestAlreadyApproved,
    #[error("impossible to generate nostr connect response")]
    CantGenerateNostrConnectResponse,
    #[error("invalid fee rate")]
    InvalidFeeRate,
    #[error("impossible to delete a not owned event")]
    TryingToDeleteNotOwnedEvent,
    #[error("{0}")]
    Generic(String),
}

#[derive(Debug, Clone)]
pub enum Message {
    Notification(Notification),
    WalletSyncCompleted(EventId),
    BlockHeightUpdated,
}

/// Coinstr
#[derive(Debug, Clone)]
pub struct Coinstr {
    network: Network,
    keechain: KeeChain,
    client: Client,
    config: Config,
    pub db: Store,
    syncing: Arc<AtomicBool>,
    sync_channel: Sender<Option<Message>>,
}

impl Coinstr {
    async fn new<P>(base_path: P, keechain: KeeChain, network: Network) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let base_path = base_path.as_ref();

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

        let (sender, _) = broadcast::channel::<Option<Message>>(1024);

        let this = Self {
            network,
            keechain,
            client: Client::with_opts(&keys, opts),
            config: Config::try_from_file(base_path, network)?,
            db,
            syncing: Arc::new(AtomicBool::new(false)),
            sync_channel: sender,
        };

        this.init().await?;

        Ok(this)
    }

    /// Open keychain
    pub async fn open<P, S, PSW>(
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

        Self::new(base_path, keechain, network).await
    }

    /// Generate keychain
    pub async fn generate<P, S, PSW, PASSP>(
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

        Self::new(base_path, keechain, network).await
    }

    /// Restore keychain
    pub async fn restore<P, S, PSW, M, PASSP>(
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

        Self::new(base_path, keechain, network).await
    }

    pub fn list_keychains<P>(base_path: P, network: Network) -> Result<Vec<String>, Error>
    where
        P: AsRef<Path>,
    {
        Ok(util::dir::get_keychains_list(base_path, network)?)
    }

    async fn init(&self) -> Result<(), Error> {
        self.restore_relays().await?;
        self.client.connect().await;
        self.sync();
        Ok(())
    }

    /// Get keychain name
    pub fn name(&self) -> Option<String> {
        self.keechain.name()
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
    pub fn rename<S>(&self, new_name: S) -> Result<(), Error>
    where
        S: Into<String>,
    {
        Ok(self.keechain.rename(new_name)?)
    }

    /// Change keychain password
    pub fn change_password<NPSW>(&self, get_new_password: NPSW) -> Result<(), Error>
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

    /// Clear cache
    pub async fn clear_cache(&self) -> Result<(), Error> {
        self.client.stop().await?;
        self.client
            .handle_notifications(|notification: RelayPoolNotification| async move {
                if let RelayPoolNotification::Stop = notification {
                    self.db.wipe()?;
                    self.client.clear_already_seen_events().await;
                    self.client.start().await;
                    self.sync();
                }
                Ok(false)
            })
            .await?;
        Ok(())
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
        let url = Url::parse(&url.into())?;
        self.db.insert_relay(&url, proxy)?;
        self.db.enable_relay(&url)?;
        self.client.add_relay(url.as_str(), proxy).await?;

        let relay = self.client.relay(&url).await?;
        let last_sync: Timestamp = match self.db.get_last_relay_sync(&url) {
            Ok(ts) => ts,
            Err(_) => Timestamp::from(0),
        };
        let filters = self.sync_filters(last_sync);
        relay.subscribe(filters, None).await?;
        relay.connect(false).await;
        Ok(())
    }

    /// Add multiple relays
    pub async fn add_relays<S>(&self, relays: Vec<(S, Option<SocketAddr>)>) -> Result<(), Error>
    where
        S: Into<String>,
    {
        for (url, proxy) in relays.into_iter() {
            self.add_relay(url, proxy).await?;
        }
        Ok(())
    }

    /// Get default relays for current [`Network`]
    pub fn default_relays(&self) -> Vec<String> {
        match self.network {
            Network::Bitcoin => vec![
                "wss://relay.house".into(),
                "wss://relay.snort.social".into(),
                "wss://relay.nostr.bg".into(),
                "wss://relay.nostr.ch".into(),
                "wss://relay.nostr.info".into(),
                "wss://nostr.rocks".into(),
                "wss://relay.damus.io".into(),
                "wss://nostr.bitcoiner.social".into(),
            ],
            _ => vec![
                "wss://test.relay.report".into(),
                "wss://nos.lol".into(),
                "wss://relay.nostrich.de".into(),
                "wss://nostr.mom".into(),
            ],
        }
    }

    async fn load_nostr_connect_relays(&self) -> Result<(), Error> {
        let relays = self.db.get_nostr_connect_sessions_relays()?;
        let relays = relays.into_iter().map(|r| (r, None)).collect();
        self.client.add_relays(relays).await?;
        Ok(())
    }

    /// Restore relays
    async fn restore_relays(&self) -> Result<(), Error> {
        let relays = self.db.get_relays(true)?;
        for (url, proxy) in relays.into_iter() {
            self.client.add_relay(url, proxy).await?;
        }

        if self.client.relays().await.is_empty() {
            let relays: Vec<(String, Option<SocketAddr>)> = self
                .default_relays()
                .into_iter()
                .map(|r| (r, None))
                .collect();
            self.add_relays(relays).await?;
        }

        // Restore Nostr Connect Session relays
        self.load_nostr_connect_relays().await?;

        // TODO: rebroadcast only once per day
        /* if let Err(e) = self.rebroadcast_all_events().await {
            log::error!("Impossible to rebroadcast stored events: {e}");
        } */

        Ok(())
    }

    pub async fn remove_relay<S>(&self, url: S) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let url = Url::parse(&url.into())?;
        self.db.delete_relay(&url)?;
        Ok(self.client.remove_relay(url).await?)
    }

    pub async fn connect_relay<S>(&self, url: S) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let url = Url::parse(&url.into())?;
        self.db.enable_relay(&url)?;
        self.client.connect_relay(url).await?;
        Ok(())
    }

    pub async fn disconnect_relay<S>(&self, url: S) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let url = Url::parse(&url.into())?;
        self.db.disable_relay(&url)?;
        self.client.disconnect_relay(url).await?;
        Ok(())
    }

    pub async fn relays(&self) -> BTreeMap<Url, Relay> {
        self.client.relays().await.into_iter().collect()
    }

    pub async fn shutdown(self) -> Result<(), Error> {
        Ok(self.client.shutdown().await?)
    }

    async fn send_event(&self, event: Event, wait: Option<Duration>) -> Result<EventId, Error> {
        self.db.save_event(&event)?;
        let event_id = event.id;
        let msg = ClientMessage::new_event(event);
        self.client.send_msg_with_custom_wait(msg, wait).await?;
        Ok(event_id)
    }

    /* async fn send_event_to<S>(
        &self,
        url: S,
        event: Event,
        wait: Option<Duration>,
    ) -> Result<EventId, Error>
    where
        S: Into<String>,
    {
        self.db.save_event(&event)?;
        let event_id = event.id;
        let msg = ClientMessage::new_event(event);
        self.client
            .send_msg_to_with_custom_wait(url, msg, wait)
            .await?;
        Ok(event_id)
    } */

    /// Get config
    pub fn config(&self) -> Config {
        self.config.clone()
    }

    pub fn set_electrum_endpoint<S>(&self, endpoint: S) -> Result<(), Error>
    where
        S: Into<String>,
    {
        // Set electrum endpoint
        self.config.set_electrum_endpoint(Some(endpoint));
        // Save config file
        self.config.save()?;
        Ok(())
    }

    pub fn electrum_endpoint(&self) -> Result<String, Error> {
        Ok(self.config.electrum_endpoint()?)
    }

    pub fn block_height(&self) -> u32 {
        self.db.block_height()
    }

    pub async fn set_metadata(&self, metadata: Metadata) -> Result<(), Error> {
        let keys = self.keys();
        let event = EventBuilder::set_metadata(metadata.clone()).to_event(&keys)?;
        self.send_event(event, Some(SEND_TIMEOUT)).await?;
        self.db.set_metadata(keys.public_key(), metadata)?;
        Ok(())
    }

    pub fn get_profile(&self) -> Result<Metadata, Error> {
        Ok(self.db.get_metadata(self.keys().public_key())?)
    }

    pub fn get_contacts(&self) -> Result<BTreeMap<XOnlyPublicKey, Metadata>, Error> {
        Ok(self.db.get_contacts_with_metadata()?)
    }

    pub async fn add_contact(&self, public_key: XOnlyPublicKey) -> Result<(), Error> {
        if public_key != self.keys().public_key() {
            let mut contacts: Vec<Contact> = self
                .db
                .get_contacts_public_keys()?
                .into_iter()
                .map(|p| Contact::new::<String>(p, None, None))
                .collect();
            contacts.push(Contact::new::<String>(public_key, None, None));
            let event = EventBuilder::set_contact_list(contacts).to_event(&self.keys())?;
            self.send_event(event, Some(SEND_TIMEOUT)).await?;
            self.db.save_contact(public_key)?;
        }

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
        let event = EventBuilder::set_contact_list(contacts).to_event(&self.keys())?;
        self.send_event(event, Some(SEND_TIMEOUT)).await?;
        self.db.delete_contact(public_key)?;
        Ok(())
    }

    pub fn get_policy_by_id(&self, policy_id: EventId) -> Result<GetPolicy, Error> {
        Ok(self.db.get_policy(policy_id)?)
    }

    pub fn get_proposal_by_id(&self, proposal_id: EventId) -> Result<GetProposal, Error> {
        Ok(self.db.get_proposal(proposal_id)?)
    }

    pub fn get_completed_proposal_by_id(
        &self,
        completed_proposal_id: EventId,
    ) -> Result<GetCompletedProposal, Error> {
        Ok(self.db.get_completed_proposal(completed_proposal_id)?)
    }

    pub fn get_signer_by_id(&self, signer_id: EventId) -> Result<Signer, Error> {
        Ok(self.db.get_signer_by_id(signer_id)?)
    }

    pub async fn delete_policy_by_id(&self, policy_id: EventId) -> Result<(), Error> {
        let Event { pubkey, .. } = self.db.get_event_by_id(policy_id)?;

        // Get nostr pubkeys and shared keys
        let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id)?;
        let shared_keys: Keys = self.db.get_shared_key(policy_id)?;

        if pubkey == shared_keys.public_key() {
            // Get all events linked to the policy
            let event_ids = self.db.get_event_ids_linked_to_policy(policy_id)?;

            let mut tags: Vec<Tag> = nostr_pubkeys
                .into_iter()
                .map(|p| Tag::PubKey(p, None))
                .collect();
            tags.push(Tag::Event(policy_id, None, None));
            event_ids
                .into_iter()
                .for_each(|id| tags.push(Tag::Event(id, None, None)));

            let event = EventBuilder::new(Kind::EventDeletion, "", &tags).to_event(&shared_keys)?;
            self.send_event(event, Some(SEND_TIMEOUT)).await?;

            self.db.delete_policy(policy_id)?;

            Ok(())
        } else {
            Err(Error::TryingToDeleteNotOwnedEvent)
        }
    }

    pub async fn delete_proposal_by_id(&self, proposal_id: EventId) -> Result<(), Error> {
        // Get the proposal
        let proposal_event = self.db.get_event_by_id(proposal_id)?;
        if proposal_event.kind != PROPOSAL_KIND {
            return Err(Error::ProposalNotFound);
        }

        let policy_id =
            util::extract_first_event_id(&proposal_event).ok_or(Error::PolicyNotFound)?;

        // Get shared key
        let shared_keys = self.db.get_shared_key(policy_id)?;

        if proposal_event.pubkey == shared_keys.public_key() {
            // Extract `p` tags from proposal event to notify users about proposal deletion
            let mut tags: Vec<Tag> = util::extract_tags_by_kind(&proposal_event, TagKind::P)
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
            self.send_event(event, Some(SEND_TIMEOUT)).await?;

            self.db.delete_proposal(proposal_id)?;

            Ok(())
        } else {
            Err(Error::TryingToDeleteNotOwnedEvent)
        }
    }

    pub async fn delete_completed_proposal_by_id(
        &self,
        completed_proposal_id: EventId,
    ) -> Result<(), Error> {
        // Get the completed proposal
        let proposal_event = self.db.get_event_by_id(completed_proposal_id)?;
        if proposal_event.kind != COMPLETED_PROPOSAL_KIND {
            return Err(Error::ProposalNotFound);
        }

        let policy_id = util::extract_tags_by_kind(&proposal_event, TagKind::E)
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

        if proposal_event.pubkey == shared_keys.public_key() {
            // Extract `p` tags from proposal event to notify users about proposal deletion
            let mut tags: Vec<Tag> = util::extract_tags_by_kind(&proposal_event, TagKind::P)
                .into_iter()
                .cloned()
                .collect();

            tags.push(Tag::Event(completed_proposal_id, None, None));

            let event = EventBuilder::new(Kind::EventDeletion, "", &tags).to_event(&shared_keys)?;
            self.send_event(event, Some(SEND_TIMEOUT)).await?;

            self.db.delete_completed_proposal(completed_proposal_id)?;

            Ok(())
        } else {
            Err(Error::TryingToDeleteNotOwnedEvent)
        }
    }

    pub async fn delete_signer_by_id(&self, signer_id: EventId) -> Result<(), Error> {
        let keys = self.client.keys();

        let my_shared_signers = self.db.get_my_shared_signers_by_signer_id(signer_id)?;
        let mut tags: Vec<Tag> = Vec::new();

        tags.push(Tag::Event(signer_id, None, None));

        for (shared_signer_id, public_key) in my_shared_signers.into_iter() {
            tags.push(Tag::PubKey(public_key, None));
            tags.push(Tag::Event(shared_signer_id, None, None));
        }

        let event = EventBuilder::new(Kind::EventDeletion, "", &tags).to_event(&keys)?;
        self.send_event(event, Some(SEND_TIMEOUT)).await?;

        self.db.delete_signer(signer_id)?;

        Ok(())
    }

    pub fn get_policies(&self) -> Result<Vec<GetPolicy>, Error> {
        Ok(self.db.get_policies()?)
    }

    pub fn get_detailed_policies(
        &self,
    ) -> Result<BTreeMap<EventId, GetDetailedPolicyResult>, Error> {
        Ok(self.db.get_detailed_policies()?)
    }

    pub fn get_proposals(&self) -> Result<Vec<GetProposal>, Error> {
        Ok(self.db.get_proposals()?)
    }

    pub fn get_proposals_by_policy_id(
        &self,
        policy_id: EventId,
    ) -> Result<Vec<GetProposal>, Error> {
        Ok(self.db.get_proposals_by_policy_id(policy_id)?)
    }

    pub fn get_approvals_by_proposal_id(
        &self,
        proposal_id: EventId,
    ) -> Result<BTreeMap<EventId, GetApprovedProposalResult>, Error> {
        Ok(self.db.get_approvals_by_proposal_id(proposal_id)?)
    }

    pub fn get_completed_proposals(&self) -> Result<Vec<GetCompletedProposal>, Error> {
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

    pub async fn save_policy<S>(
        &self,
        name: S,
        description: S,
        descriptor: S,
        nostr_pubkeys: Vec<XOnlyPublicKey>,
    ) -> Result<EventId, Error>
    where
        S: Into<String>,
    {
        let keys = self.client.keys();
        let descriptor = descriptor.into();

        if nostr_pubkeys.len() < 2 {
            return Err(Error::NotEnoughPublicKeys);
        }

        // Generate a shared key
        let shared_key = Keys::generate();
        let policy = Policy::from_desc_or_policy(name, description, descriptor, self.network)?;

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
            let event_id: EventId = self.send_event(event, None).await?;
            log::info!("Published shared key for {pubkey} at event {event_id}");
        }

        // Publish the event
        self.send_event(policy_event, Some(SEND_TIMEOUT)).await?;

        // Cache policy
        self.db.save_shared_key(policy_id, shared_key)?;
        self.db.save_policy(policy_id, policy, nostr_pubkeys)?;

        Ok(policy_id)
    }

    /// Make a spending proposal
    #[allow(clippy::too_many_arguments)]
    pub async fn spend<S>(
        &self,
        policy_id: EventId,
        address: Address,
        amount: Amount,
        description: S,
        fee_rate: FeeRate,
        utxos: Option<Vec<OutPoint>>,
        policy_path: Option<BTreeMap<String, Vec<usize>>>,
    ) -> Result<GetProposal, Error>
    where
        S: Into<String>,
    {
        // Get policy and shared keys
        let GetPolicy { policy, .. } = self.get_policy_by_id(policy_id)?;
        let shared_keys: Keys = self.db.get_shared_key(policy_id)?;

        let description: &str = &description.into();

        // Check and calculate fee rate
        if !fee_rate.is_valid() {
            return Err(Error::InvalidFeeRate);
        }

        let fee_rate = match fee_rate {
            FeeRate::Priority(priority) => {
                let endpoint: String = self.electrum_endpoint()?;
                let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&endpoint)?);
                blockchain.estimate_fee(priority.target_blocks() as usize)?
            }
            FeeRate::Rate(rate) => bdk::FeeRate::from_sat_per_vb(rate),
        };

        // Build spending proposal
        let wallet: Wallet<SqliteDatabase> =
            self.wallet(policy_id, &policy.descriptor.to_string())?;
        let proposal = policy.spend(
            wallet,
            address,
            amount,
            description,
            fee_rate,
            utxos,
            policy_path,
        )?;

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
            let proposal_id = self.send_event(event, Some(SEND_TIMEOUT)).await?;

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

            Ok(GetProposal {
                proposal_id,
                policy_id,
                proposal,
            })
        } else {
            Err(Error::UnexpectedProposal)
        }
    }

    /// Spend to another [`Policy`]
    pub async fn self_transfer(
        &self,
        from_policy_id: EventId,
        to_policy_id: EventId,
        amount: Amount,
        fee_rate: FeeRate,
        utxos: Option<Vec<OutPoint>>,
        policy_path: Option<BTreeMap<String, Vec<usize>>>,
    ) -> Result<GetProposal, Error> {
        let address = self.get_last_unused_address(to_policy_id)?.address;
        let description: String = format!(
            "Self transfer from policy #{} to #{}",
            util::cut_event_id(from_policy_id),
            util::cut_event_id(to_policy_id)
        );
        self.spend(
            from_policy_id,
            address,
            amount,
            description,
            fee_rate,
            utxos,
            policy_path,
        )
        .await
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
        let GetProposal {
            policy_id,
            proposal,
            ..
        } = self.get_proposal_by_id(proposal_id)?;
        let GetPolicy { policy, .. } = self.get_policy_by_id(policy_id)?;

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
        let event_id = self.send_event(event, Some(SEND_TIMEOUT)).await?;

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
        let GetProposal {
            policy_id,
            proposal,
            ..
        } = self.get_proposal_by_id(proposal_id)?;

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
        let event_id = self.send_event(event, Some(SEND_TIMEOUT)).await?;

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
        let GetProposal {
            policy_id,
            proposal,
            ..
        } = self.get_proposal_by_id(proposal_id)?;

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
        let event_id = self.send_event(event, Some(SEND_TIMEOUT)).await?;

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

    pub async fn revoke_approval(&self, approval_id: EventId) -> Result<(), Error> {
        let Event { pubkey, .. } = self.db.get_event_by_id(approval_id)?;
        let keys = self.keys();
        if pubkey == keys.public_key() {
            let policy_id = self.db.get_policy_id_by_approval_id(approval_id)?;

            // Get nostr pubkeys linked to policy
            let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id)?;

            let mut tags: Vec<Tag> = nostr_pubkeys
                .into_iter()
                .map(|p| Tag::PubKey(p, None))
                .collect();
            tags.push(Tag::Event(approval_id, None, None));

            let event = EventBuilder::new(Kind::EventDeletion, "", &tags).to_event(&keys)?;
            self.send_event(event, Some(SEND_TIMEOUT)).await?;

            self.db.delete_approval(approval_id)?;

            Ok(())
        } else {
            Err(Error::TryingToDeleteNotOwnedEvent)
        }
    }

    pub async fn finalize(&self, proposal_id: EventId) -> Result<CompletedProposal, Error> {
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
            let endpoint = self.config.electrum_endpoint()?;
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
        let event_id = self.send_event(event, Some(SEND_TIMEOUT)).await?;

        // Delete the proposal
        if let Err(e) = self.delete_proposal_by_id(proposal_id).await {
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
        let GetPolicy { policy, .. } = self.get_policy_by_id(policy_id)?;
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
        let proposal_id = self.send_event(event, Some(SEND_TIMEOUT)).await?;

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

    pub fn verify_proof_by_id(&self, completed_proposal_id: EventId) -> Result<u64, Error> {
        let GetCompletedProposal {
            proposal,
            policy_id,
            ..
        } = self.get_completed_proposal_by_id(completed_proposal_id)?;
        if let CompletedProposal::ProofOfReserve {
            message,
            descriptor,
            psbt,
            ..
        } = proposal
        {
            let wallet = self.wallet(policy_id, descriptor.to_string())?;
            Ok(wallet.verify_proof(&psbt, message, None)?)
        } else {
            Err(Error::UnexpectedProposal)
        }
    }

    pub async fn save_signer(&self, signer: Signer) -> Result<EventId, Error> {
        let keys = self.client.keys();

        if self.db.signer_descriptor_exists(signer.descriptor())? {
            return Err(Error::SignerDescriptorAlreadyExists);
        }

        // Compose the event
        let content: String = signer.encrypt_with_keys(&keys)?;

        // Compose signer event
        let event = EventBuilder::new(SIGNERS_KIND, content, &[]).to_event(&keys)?;

        // Publish the event
        let signer_id = self.send_event(event, Some(SEND_TIMEOUT)).await?;

        // Save signer in db
        self.db.save_signer(signer_id, signer)?;

        Ok(signer_id)
    }

    pub fn coinstr_signer_exists(&self) -> Result<bool, Error> {
        let signer = coinstr_signer(self.keechain.keychain.seed(), self.network)?;
        Ok(self.db.signer_descriptor_exists(signer.descriptor())?)
    }

    pub async fn save_coinstr_signer(&self) -> Result<EventId, Error> {
        let signer = coinstr_signer(self.keechain.keychain.seed(), self.network)?;
        self.save_signer(signer).await
    }

    /// Get all own signers and contacts shared signers
    pub fn get_all_signers(&self) -> Result<GetAllSigners, Error> {
        Ok(GetAllSigners {
            my: self.get_signers()?,
            contacts: self.get_shared_signers()?,
        })
    }

    pub fn get_signers(&self) -> Result<BTreeMap<EventId, Signer>, Error> {
        Ok(self.db.get_signers()?)
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
        Err(Error::SignerNotFound)
    }

    pub fn get_balance(&self, policy_id: EventId) -> Option<Balance> {
        self.db.get_balance(policy_id)
    }

    pub fn get_txs(&self, policy_id: EventId) -> Option<Vec<TransactionDetails>> {
        self.db.get_txs(policy_id)
    }

    pub fn get_txs_with_descriptions(&self, policy_id: EventId) -> Option<Transactions> {
        self.db.get_txs_with_descriptions(policy_id)
    }

    pub fn get_address(
        &self,
        policy_id: EventId,
        index: AddressIndex,
    ) -> Result<GetAddress, Error> {
        Ok(self.db.get_address(policy_id, index)?)
    }

    pub fn get_last_unused_address(&self, policy_id: EventId) -> Result<GetAddress, Error> {
        self.get_address(policy_id, AddressIndex::LastUnused)
    }

    /// Get wallet UTXOs
    pub fn get_utxos(&self, policy_id: EventId) -> Result<Vec<GetUtxo>, Error> {
        Ok(self.db.get_utxos(policy_id)?)
    }

    pub fn get_total_balance(&self) -> Result<Balance, Error> {
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
        // TODO: save last rebroadcast timestamp
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
            let event_id: EventId = self.send_event(event, None).await?;
            log::info!("Published shared key for {pubkey} at event {event_id}");
        }
        Ok(())
    }

    pub fn export_policy_backup(&self, policy_id: EventId) -> Result<PolicyBackup, Error> {
        let GetPolicy { policy, .. } = self.db.get_policy(policy_id)?;
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
        signer_id: EventId,
        public_key: XOnlyPublicKey,
    ) -> Result<EventId, Error> {
        if !self
            .db
            .my_shared_signer_already_shared(signer_id, public_key)?
        {
            let keys: Keys = self.client.keys();
            let signer: Signer = self.get_signer_by_id(signer_id)?;
            let shared_signer: SharedSigner = signer.to_shared_signer();
            let content: String =
                nip04::encrypt(&keys.secret_key()?, &public_key, shared_signer.as_json())?;
            let tags = &[
                Tag::Event(signer_id, None, None),
                Tag::PubKey(public_key, None),
            ];
            let event: Event =
                EventBuilder::new(SHARED_SIGNERS_KIND, content, tags).to_event(&keys)?;
            let event_id = self.send_event(event, Some(SEND_TIMEOUT)).await?;
            self.db
                .save_my_shared_signer(signer_id, event_id, public_key)?;
            Ok(event_id)
        } else {
            Err(Error::SignerAlreadyShared)
        }
    }

    pub async fn share_signer_to_multiple_public_keys(
        &self,
        signer_id: EventId,
        public_keys: Vec<XOnlyPublicKey>,
    ) -> Result<(), Error> {
        if public_keys.is_empty() {
            return Err(Error::NotEnoughPublicKeys);
        }

        let keys: Keys = self.client.keys();
        let signer: Signer = self.get_signer_by_id(signer_id)?;
        let shared_signer: SharedSigner = signer.to_shared_signer();

        for public_key in public_keys.into_iter() {
            if self
                .db
                .my_shared_signer_already_shared(signer_id, public_key)?
            {
                log::warn!("Signer {signer_id} already shared with {public_key}");
            } else {
                let content: String =
                    nip04::encrypt(&keys.secret_key()?, &public_key, shared_signer.as_json())?;
                let tags = &[
                    Tag::Event(signer_id, None, None),
                    Tag::PubKey(public_key, None),
                ];
                let event: Event =
                    EventBuilder::new(SHARED_SIGNERS_KIND, content, tags).to_event(&keys)?;
                let event_id = self.send_event(event, None).await?;
                self.db
                    .save_my_shared_signer(signer_id, event_id, public_key)?;
            }
        }

        Ok(())
    }

    pub async fn revoke_all_shared_signers(&self) -> Result<(), Error> {
        let keys = self.client.keys();
        for (shared_signer_id, public_key) in self.db.get_my_shared_signers()?.into_iter() {
            let tags = &[
                Tag::PubKey(public_key, None),
                Tag::Event(shared_signer_id, None, None),
            ];
            let event = EventBuilder::new(Kind::EventDeletion, "", tags).to_event(&keys)?;
            self.send_event(event, Some(SEND_TIMEOUT)).await?;
            self.db.delete_shared_signer(shared_signer_id)?;
        }
        Ok(())
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
        self.send_event(event, Some(SEND_TIMEOUT)).await?;
        self.db.delete_shared_signer(shared_signer_id)?;
        Ok(())
    }

    pub fn get_my_shared_signers_by_signer_id(
        &self,
        signer_id: EventId,
    ) -> Result<BTreeMap<EventId, XOnlyPublicKey>, Error> {
        Ok(self.db.get_my_shared_signers_by_signer_id(signer_id)?)
    }

    pub fn get_shared_signers(&self) -> Result<BTreeMap<EventId, GetSharedSignerResult>, Error> {
        Ok(self.db.get_shared_signers()?)
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

    pub fn mark_notification_as_seen_by_id(&self, event_id: EventId) -> Result<(), Error> {
        Ok(self.db.mark_notification_as_seen_by_id(event_id)?)
    }

    pub fn mark_notification_as_seen(&self, notification: Notification) -> Result<(), Error> {
        Ok(self.db.mark_notification_as_seen(notification)?)
    }

    pub fn delete_all_notifications(&self) -> Result<(), Error> {
        Ok(self.db.delete_all_notifications()?)
    }

    pub async fn new_nostr_connect_session(&self, uri: NostrConnectURI) -> Result<(), Error> {
        let relay_url: Url = uri.relay_url.clone();
        self.client.add_relay(relay_url.as_str(), None).await?;

        let relay = self.client.relay(&relay_url).await?;
        relay.connect(true).await;

        let last_sync: Timestamp = match self.db.get_last_relay_sync(&relay_url) {
            Ok(ts) => ts,
            Err(e) => {
                log::error!("Impossible to get last relay sync: {e}");
                Timestamp::from(0)
            }
        };
        let filters = self.sync_filters(last_sync);
        relay.subscribe(filters, None).await?;

        // Send connect ACK
        let keys = self.client.keys();
        let msg = NIP46Message::request(NIP46Request::Connect(keys.public_key()));
        let nip46_event =
            EventBuilder::nostr_connect(&keys, uri.public_key, msg)?.to_event(&keys)?;
        self.client
            .send_event_to_with_custom_wait(relay_url, nip46_event, Some(CONNECT_SEND_TIMEOUT))
            .await?;

        self.db.save_nostr_connect_uri(uri)?;

        Ok(())
    }

    pub fn get_nostr_connect_sessions(&self) -> Result<Vec<(NostrConnectURI, Timestamp)>, Error> {
        Ok(self.db.get_nostr_connect_sessions()?)
    }

    async fn _disconnect_nostr_connect_session(
        &self,
        app_public_key: XOnlyPublicKey,
        wait: Option<Duration>,
    ) -> Result<(), Error> {
        let uri = self.db.get_nostr_connect_session(app_public_key)?;
        let keys = self.client.keys();
        let msg = NIP46Message::request(NIP46Request::Disconnect);
        let nip46_event =
            EventBuilder::nostr_connect(&keys, uri.public_key, msg)?.to_event(&keys)?;
        self.client
            .send_event_to_with_custom_wait(uri.relay_url, nip46_event, wait)
            .await?;
        self.db.delete_nostr_connect_session(app_public_key)?;
        Ok(())
    }

    pub async fn disconnect_nostr_connect_session(
        &self,
        app_public_key: XOnlyPublicKey,
    ) -> Result<(), Error> {
        self._disconnect_nostr_connect_session(app_public_key, Some(CONNECT_SEND_TIMEOUT))
            .await
    }

    pub fn get_nostr_connect_requests(
        &self,
        approved: bool,
    ) -> Result<Vec<(EventId, NostrConnectRequest)>, Error> {
        Ok(self.db.get_nostr_connect_requests(approved)?)
    }

    pub async fn approve_nostr_connect_request(&self, event_id: EventId) -> Result<(), Error> {
        let NostrConnectRequest {
            app_public_key,
            message,
            approved,
            ..
        } = self.db.get_nostr_connect_request(event_id)?;
        if !approved {
            let uri = self.db.get_nostr_connect_session(app_public_key)?;
            let keys = self.client.keys();
            let msg = message
                .generate_response(&keys)?
                .ok_or(Error::CantGenerateNostrConnectResponse)?;
            let nip46_event =
                EventBuilder::nostr_connect(&keys, uri.public_key, msg)?.to_event(&keys)?;
            self.client
                .send_event_to_with_custom_wait(
                    uri.relay_url,
                    nip46_event,
                    Some(CONNECT_SEND_TIMEOUT),
                )
                .await?;
            self.db.set_nostr_connect_request_as_approved(event_id)?;
            Ok(())
        } else {
            Err(Error::NostrConnectRequestAlreadyApproved)
        }
    }

    pub fn auto_approve_nostr_connect_requests(
        &self,
        app_public_key: XOnlyPublicKey,
        duration: Duration,
    ) {
        let until: Timestamp = Timestamp::now() + duration;
        self.db
            .set_nostr_connect_auto_approve(app_public_key, until);
    }

    pub fn revoke_nostr_connect_auto_approve(&self, app_public_key: XOnlyPublicKey) {
        self.db.revoke_nostr_connect_auto_approve(app_public_key);
    }

    pub fn get_nostr_connect_pre_authorizations(&self) -> BTreeMap<XOnlyPublicKey, Timestamp> {
        self.db.get_nostr_connect_pre_authorizations()
    }

    pub fn delete_nostr_connect_request(&self, event_id: EventId) -> Result<(), Error> {
        Ok(self.db.delete_nostr_connect_request(event_id)?)
    }
}
