// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_utility::{thread, time};
use bdk_electrum::electrum_client::{
    Client as ElectrumClient, Config as ElectrumConfig, Socks5Config,
};
use bdk_electrum::ElectrumExt;
use bdk_file_store::{IterError, Store as FileStore};
use coinstr_core::bdk::chain::keychain::LocalUpdate;
use coinstr_core::bdk::chain::local_chain::UpdateNotConnectedError;
use coinstr_core::bdk::chain::ConfirmationTimeAnchor;
use coinstr_core::bdk::chain::{ConfirmationTime, PersistBackend};
use coinstr_core::bdk::wallet::{AddressIndex, AddressInfo, Balance, ChangeSet};
use coinstr_core::bdk::{FeeRate, KeychainKind, TransactionDetails, Wallet};
use coinstr_core::bitcoin::{Address, Network, OutPoint, Script, Txid};
use coinstr_core::{Amount, Policy, Proposal};
use nostr_sdk::{EventId, Timestamp};
use parking_lot::Mutex;
use thiserror::Error;
use tokio::sync::broadcast;
use tokio::sync::mpsc::{self, Sender};

use crate::client::Message;
use crate::constants::{BDK_DB_MAGIC, WALLET_SYNC_INTERVAL};
use crate::db::model::{GetAddress, GetPolicy, GetTransaction, GetUtxo};
use crate::db::Store;
use crate::{Label, LabelData};

const STOP_GAP: usize = 50;
const BATCH_SIZE: usize = 5;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Bdk(#[from] coinstr_core::bdk::Error),
    #[error(transparent)]
    BdkNew(#[from] coinstr_core::bdk::wallet::NewError<IterError>),
    #[error(transparent)]
    BdkFileStore(#[from] bdk_file_store::FileError<'static>),
    #[error(transparent)]
    Electrum(#[from] bdk_electrum::electrum_client::Error),
    #[error(transparent)]
    UpdateNotConnected(#[from] UpdateNotConnectedError),
    #[error(transparent)]
    Address(#[from] coinstr_core::bitcoin::util::address::Error),
    #[error(transparent)]
    Policy(#[from] coinstr_core::policy::Error),
    #[error(transparent)]
    Store(#[from] crate::db::Error),
    #[error(transparent)]
    Label(#[from] crate::types::label::Error),
    #[error("policy {0} already loaded")]
    AlreadyLoaded(EventId),
    #[error("policy {0} not loaded")]
    NotLoaded(EventId),
    #[error("not found")]
    NotFound,
    #[error("timeout")]
    Timeout,
    #[error("send error")]
    SendError,
}

pub enum Command {
    Sync {
        endpoint: String,
        proxy: Option<SocketAddr>,
    },
    GetBalance,
    GetAddress(AddressIndex),
    GetAddresses,
    GetAddressesBalances,
    GetTxs {
        sort: bool,
    },
    GetTx(Txid),
    GetUtxos,
    Spend {
        address: Address,
        amount: Amount,
        description: String,
        fee_rate: FeeRate,
        utxos: Option<Vec<OutPoint>>,
        policy_path: Option<BTreeMap<String, Vec<usize>>>,
    },
    ApplyUpdate(LocalUpdate<KeychainKind, ConfirmationTimeAnchor>),
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum Response {
    Balance(Balance),
    Address(GetAddress),
    Addresses(Vec<GetAddress>),
    AddressesBalances(HashMap<Script, u64>),
    Txs(Vec<GetTransaction>),
    Tx(GetTransaction),
    Utxos(Vec<GetUtxo>),
    Proposal(Proposal),
}

#[derive(Debug, Clone)]
pub struct PolicyHandle {
    sender: Sender<Command>,
    receiver: broadcast::Sender<Response>,
    syncing: Arc<AtomicBool>,
}

impl PolicyHandle {
    pub fn receiver(&self) -> broadcast::Receiver<Response> {
        self.receiver.subscribe()
    }

    pub fn is_syncing(&self) -> bool {
        self.syncing.load(Ordering::SeqCst)
    }

    pub fn set_syncing(&self, syncing: bool) {
        let _ = self
            .syncing
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |_| Some(syncing));
    }
}

#[derive(Debug, Clone)]
pub struct Manager {
    policies: Arc<Mutex<HashMap<EventId, PolicyHandle>>>,
    db: Store,
    timechain_path: PathBuf,
    sync_channel: broadcast::Sender<Message>,
}

impl Manager {
    pub fn new<P>(db: Store, timechain_path: P, sync_channel: broadcast::Sender<Message>) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            policies: Arc::new(Mutex::new(HashMap::new())),
            db,
            timechain_path: timechain_path.as_ref().to_path_buf(),
            sync_channel,
        }
    }

    fn policies(&self) -> HashMap<EventId, PolicyHandle> {
        let policies = self.policies.lock();
        policies.clone()
    }

    fn ids(&self) -> Vec<EventId> {
        let policies = self.policies.lock();
        policies.keys().copied().collect()
    }

    fn send(&self, sender: &broadcast::Sender<Response>, response: Response) {
        if let Err(e) = sender.send(response) {
            tracing::error!("Impossible to send reponse: {e}")
        }
    }

    fn sync_wallet<D, S>(
        &self,
        policy_id: EventId,
        wallet: &mut Wallet<D>,
        endpoint: S,
        proxy: Option<SocketAddr>,
    ) -> Result<(), Error>
    where
        D: PersistBackend<ChangeSet> + 'static,
        S: Into<String>,
    {
        let handle: PolicyHandle = self.get_policy_handle(policy_id)?;

        if !handle.is_syncing() {
            tracing::info!("Syncing policy {policy_id}");

            handle.set_syncing(true);

            let endpoint: String = endpoint.into();
            let local_chain = wallet.checkpoints().clone();
            let keychain_spks = wallet.spks_of_all_keychains();
            let graph = wallet.as_ref().clone();

            fn sync(
                handle: PolicyHandle,
                endpoint: String,
                proxy: Option<SocketAddr>,
                local_chain: BTreeMap<u32, coinstr_core::bitcoin::BlockHash>,
                keychain_spks: BTreeMap<KeychainKind, impl Iterator<Item = (u32, Script)> + Clone>,
                graph: bdk_electrum::bdk_chain::TxGraph<ConfirmationTimeAnchor>,
            ) -> Result<(), Error> {
                tracing::info!(
                    "Initializing electrum client: endpoint={endpoint}, proxy={proxy:?}"
                );
                let proxy: Option<Socks5Config> = proxy.map(Socks5Config::new);
                let config = ElectrumConfig::builder().socks5(proxy)?.build();
                let client = ElectrumClient::from_config(&endpoint, config)?;

                let electrum_update = client.scan(
                    &local_chain,
                    keychain_spks,
                    None,
                    None,
                    STOP_GAP,
                    BATCH_SIZE,
                )?;
                let missing = electrum_update.missing_full_txs(&graph);
                let update =
                    electrum_update.finalize_as_confirmation_time(&client, None, missing)?;

                handle.set_syncing(false);

                handle
                    .sender
                    .try_send(Command::ApplyUpdate(update))
                    .map_err(|_| Error::SendError)?;
                Ok(())
            }

            thread::spawn(async move {
                match sync(
                    handle.clone(),
                    endpoint,
                    proxy,
                    local_chain,
                    keychain_spks,
                    graph,
                ) {
                    Ok(_) => tracing::debug!("Policy {policy_id} received timechain update data"),
                    Err(e) => {
                        handle.set_syncing(false);
                        tracing::error!("Impossible to receive timechain update data for policy {policy_id}: {e}")
                    }
                }
            });
        } else {
            tracing::warn!("Policy {policy_id} is already syncing");
        }
        Ok(())
    }

    fn apply_update<D>(
        &self,
        policy_id: EventId,
        update: LocalUpdate<KeychainKind, ConfirmationTimeAnchor>,
        wallet: &mut Wallet<D>,
    ) -> Result<(), Error>
    where
        D: PersistBackend<ChangeSet>,
        Error: From<<D as PersistBackend<ChangeSet>>::WriteError>,
    {
        wallet.apply_update(update)?;
        wallet.commit()?;
        self.db
            .update_last_sync(policy_id, Some(Timestamp::now()))?;
        Ok(())
    }

    fn _get_address<D>(
        &self,
        policy_id: EventId,
        index: AddressIndex,
        wallet: &mut Wallet<D>,
    ) -> Result<GetAddress, Error>
    where
        D: PersistBackend<ChangeSet>,
    {
        let address: AddressInfo = wallet.get_address(index);

        let shared_key = self.db.get_shared_key(policy_id)?;
        let identifier: String =
            LabelData::Address(address.address.clone()).generate_identifier(&shared_key)?;
        let label = self
            .db
            .get_label_by_identifier(identifier)
            .ok()
            .map(|l| l.text());
        Ok(GetAddress {
            address: address.address,
            label,
        })
    }

    fn _get_addresses<D>(
        &self,
        policy_id: EventId,
        wallet: &mut Wallet<D>,
    ) -> Result<Vec<GetAddress>, Error>
    where
        D: PersistBackend<ChangeSet>,
    {
        let last_unused = wallet.get_address(AddressIndex::LastUnused);
        let script_labels: HashMap<Script, Label> = self.db.get_addresses_labels(policy_id)?;

        let mut addresses: Vec<GetAddress> = Vec::new();

        for index in 0.. {
            let addr = wallet.get_address(AddressIndex::Peek(index));
            addresses.push(GetAddress {
                address: addr.address.clone(),
                label: script_labels
                    .get(&addr.address.script_pubkey())
                    .map(|l| l.text()),
            });
            if addr == last_unused {
                for i in index + 1..index + 20 {
                    let addr = wallet.get_address(AddressIndex::Peek(i));
                    addresses.push(GetAddress {
                        address: addr.address.clone(),
                        label: script_labels
                            .get(&addr.address.script_pubkey())
                            .map(|l| l.text()),
                    });
                }
                break;
            }
        }

        Ok(addresses)
    }

    fn _get_addresses_balances<D>(&self, wallet: &Wallet<D>) -> HashMap<Script, u64>
    where
        D: PersistBackend<ChangeSet>,
    {
        let mut map: HashMap<Script, u64> = HashMap::new();

        for utxo in wallet.list_unspent() {
            map.entry(utxo.txout.script_pubkey)
                .and_modify(|amount| *amount += utxo.txout.value)
                .or_insert(utxo.txout.value);
        }

        map
    }

    fn _get_txs<D>(
        &self,
        policy_id: EventId,
        wallet: &Wallet<D>,
        sort: bool,
    ) -> Result<Vec<GetTransaction>, Error>
    where
        D: PersistBackend<ChangeSet>,
    {
        let mut txs: Vec<TransactionDetails> = wallet
            .transactions()
            .filter_map(|t| wallet.get_tx(t.node.txid, true))
            .collect();

        if sort {
            txs.sort_by(|a, b| {
                let a = match a.confirmation_time {
                    ConfirmationTime::Confirmed { height, .. } => height,
                    ConfirmationTime::Unconfirmed { .. } => u32::MAX,
                };

                let b = match b.confirmation_time {
                    ConfirmationTime::Confirmed { height, .. } => height,
                    ConfirmationTime::Unconfirmed { .. } => u32::MAX,
                };

                b.cmp(&a)
            });
        }

        let descriptions: HashMap<Txid, String> = self.db.get_txs_descriptions(policy_id)?;
        let script_labels: HashMap<Script, Label> = self.db.get_addresses_labels(policy_id)?;

        let mut list: Vec<GetTransaction> = Vec::new();

        for tx in txs.into_iter() {
            let label: Option<String> = if tx.received > tx.sent {
                let mut label = None;
                if let Some(transaction) = tx.transaction.as_ref() {
                    for txout in transaction.output.iter() {
                        if wallet.is_mine(&txout.script_pubkey) {
                            label = script_labels.get(&txout.script_pubkey).map(|l| l.text());
                            break;
                        }
                    }
                }
                label
            } else {
                // TODO: try to get UTXO label?
                descriptions.get(&tx.txid).cloned()
            };

            list.push(GetTransaction {
                policy_id,
                label,
                tx,
            })
        }

        Ok(list)
    }

    fn _get_tx<D>(
        &self,
        policy_id: EventId,
        txid: Txid,
        wallet: &Wallet<D>,
    ) -> Result<GetTransaction, Error>
    where
        D: PersistBackend<ChangeSet>,
    {
        let tx: TransactionDetails = wallet.get_tx(txid, true).ok_or(Error::NotFound)?;

        let label: Option<String> = if tx.received > tx.sent {
            let mut label = None;
            for txout in tx
                .transaction
                .as_ref()
                .ok_or(Error::NotFound)?
                .output
                .iter()
            {
                if wallet.is_mine(&txout.script_pubkey) {
                    let shared_key = self.db.get_shared_key(policy_id)?;
                    let identifier: String = LabelData::Address(Address::from_script(
                        &txout.script_pubkey,
                        wallet.network(),
                    )?)
                    .generate_identifier(&shared_key)?;
                    label = self
                        .db
                        .get_label_by_identifier(identifier)
                        .ok()
                        .map(|l| l.text());
                    break;
                }
            }
            label
        } else {
            // TODO: try to get UTXO label?
            self.db.get_description_by_txid(policy_id, txid)?
        };

        Ok(GetTransaction {
            policy_id,
            tx,
            label,
        })
    }

    fn _get_utxos<D>(&self, policy_id: EventId, wallet: &Wallet<D>) -> Result<Vec<GetUtxo>, Error>
    where
        D: PersistBackend<ChangeSet>,
    {
        // Get labels
        let script_labels: HashMap<Script, Label> = self.db.get_addresses_labels(policy_id)?;
        let utxo_labels: HashMap<OutPoint, Label> = self.db.get_utxos_labels(policy_id)?;

        // Compose output
        Ok(wallet
            .list_unspent()
            .map(|utxo| GetUtxo {
                label: utxo_labels
                    .get(&utxo.outpoint)
                    .or_else(|| script_labels.get(&utxo.txout.script_pubkey))
                    .map(|l| l.text()),
                utxo,
            })
            .collect())
    }

    fn _spend<D>(
        &self,
        policy_id: EventId,
        address: Address,
        amount: Amount,
        description: String,
        fee_rate: FeeRate,
        utxos: Option<Vec<OutPoint>>,
        policy_path: Option<BTreeMap<String, Vec<usize>>>,
        wallet: &mut Wallet<D>,
    ) -> Result<Proposal, Error>
    where
        D: PersistBackend<ChangeSet>,
    {
        let GetPolicy { policy, .. } = self.db.get_policy(policy_id)?;
        let proposal = policy.spend(
            wallet,
            address,
            amount,
            description,
            fee_rate,
            utxos,
            policy_path,
        )?;
        Ok(proposal)
    }

    pub fn load_policy(
        &self,
        policy_id: EventId,
        policy: Policy,
        network: Network,
    ) -> Result<(), Error> {
        let mut policies = self.policies.lock();
        if policies.contains_key(&policy_id) {
            return Err(Error::AlreadyLoaded(policy_id));
        }

        // Init wallet
        let db: FileStore<ChangeSet> = FileStore::new_from_path(
            BDK_DB_MAGIC.as_bytes(),
            self.timechain_path.join(policy_id.to_hex()),
        )?;
        let mut wallet = Wallet::new(&policy.descriptor.to_string(), None, db, network)?;

        let (sender, mut receiver) = mpsc::channel::<Command>(4096);
        let (tx, mut rx) = broadcast::channel::<Response>(4096);

        // Keep channel opened
        thread::spawn(async move { while rx.recv().await.is_ok() {} });

        let tx_sender = tx.clone();
        let this = self.clone();
        thread::spawn(async move {
            while let Some(cmd) = receiver.recv().await {
                match cmd {
                    Command::Sync { endpoint, proxy } => match this.db.get_last_sync(policy_id) {
                        Ok(last_sync) => {
                            let last_sync: Timestamp =
                                last_sync.unwrap_or_else(|| Timestamp::from(0));
                            if last_sync.add(WALLET_SYNC_INTERVAL) <= Timestamp::now() {
                                match this.sync_wallet(policy_id, &mut wallet, endpoint, proxy) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        tracing::error!(
                                            "Impossible to sync policy {policy_id}: {e}"
                                        )
                                    }
                                }
                            }
                        }
                        Err(e) => tracing::error!("Impossible to get last policy sync: {e}"),
                    },
                    Command::ApplyUpdate(update) => {
                        match this.apply_update(policy_id, update, &mut wallet) {
                            Ok(_) => {
                                tracing::info!("Policy {policy_id} synced");
                                let _ = this
                                    .sync_channel
                                    .send(Message::WalletSyncCompleted(policy_id));
                            }
                            Err(e) => tracing::error!(
                                "Impossible to apply wallet update for policy {policy_id}: {e}"
                            ),
                        }
                    }
                    Command::GetBalance => {
                        let balance = wallet.get_balance();
                        this.send(&tx_sender, Response::Balance(balance));
                    }
                    Command::GetAddresses => match this._get_addresses(policy_id, &mut wallet) {
                        Ok(addresses) => this.send(&tx_sender, Response::Addresses(addresses)),
                        Err(e) => tracing::error!("Impossible to get addresses: {e}"),
                    },
                    Command::GetAddressesBalances => {
                        let balances = this._get_addresses_balances(&wallet);
                        this.send(&tx_sender, Response::AddressesBalances(balances));
                    }
                    Command::GetAddress(index) => {
                        match this._get_address(policy_id, index, &mut wallet) {
                            Ok(address) => this.send(&tx_sender, Response::Address(address)),
                            Err(e) => tracing::error!("Impossible to get address: {e}"),
                        }
                    }
                    Command::GetTxs { sort } => match this._get_txs(policy_id, &wallet, sort) {
                        Ok(txs) => this.send(&tx_sender, Response::Txs(txs)),
                        Err(e) => tracing::error!("Impossible to get txs: {e}"),
                    },
                    Command::GetTx(txid) => match this._get_tx(policy_id, txid, &wallet) {
                        Ok(tx) => this.send(&tx_sender, Response::Tx(tx)),
                        Err(e) => tracing::error!("Impossible to get tx {txid}: {e}"),
                    },
                    Command::GetUtxos => match this._get_utxos(policy_id, &wallet) {
                        Ok(utxos) => this.send(&tx_sender, Response::Utxos(utxos)),
                        Err(e) => tracing::error!("Impossible to get utxos: {e}"),
                    },
                    Command::Spend {
                        address,
                        amount,
                        description,
                        fee_rate,
                        utxos,
                        policy_path,
                    } => match this._spend(
                        policy_id,
                        address,
                        amount,
                        description,
                        fee_rate,
                        utxos,
                        policy_path,
                        &mut wallet,
                    ) {
                        Ok(proposal) => this.send(&tx_sender, Response::Proposal(proposal)),
                        Err(e) => tracing::error!("Impossible to create proposal: {e}"),
                    },
                    Command::Shutdown => break,
                }
            }

            tracing::debug!("Exited from {policy_id} wallet loop");
        });

        policies.insert(
            policy_id,
            PolicyHandle {
                sender,
                receiver: tx,
                syncing: Arc::new(AtomicBool::new(false)),
            },
        );
        drop(policies);

        tracing::info!("Loaded policy {policy_id}");

        Ok(())
    }

    pub async fn unload_policy(&self, policy_id: EventId) -> Result<(), Error> {
        let mut policies = self.policies.lock();
        let handle = policies
            .get(&policy_id)
            .ok_or(Error::NotLoaded(policy_id))?;
        let handle = handle.clone();
        policies.remove(&policy_id);
        drop(policies);
        thread::spawn(async move {
            thread::sleep(Duration::from_secs(1)).await;
            let _ = handle.sender.send(Command::Shutdown).await;
        });
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), Error> {
        for policy_id in self.ids().into_iter() {
            self.unload_policy(policy_id).await?;
        }
        Ok(())
    }

    fn get_policy_handle(&self, policy_id: EventId) -> Result<PolicyHandle, Error> {
        let policies = self.policies.lock();
        let handle = policies
            .get(&policy_id)
            .ok_or(Error::NotLoaded(policy_id))?;
        Ok(handle.clone())
    }

    /* pub async fn sync<S>(
        &self,
        policy_id: EventId,
        endpoint: S,
        proxy: Option<SocketAddr>,
        sender: broadcast::Sender<Option<Message>>,
    ) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let endpoint = endpoint.into();
        let handle = self.get_policy_handle(policy_id)?;
        handle
            .sender
            .send_timeout(
                Command::Sync {
                    endpoint: endpoint.clone(),
                    proxy,
                    sender: sender.clone(),
                },
                Duration::from_secs(10),
            )
            .await
            ?;
        Ok(())
    } */

    pub fn sync_all<S>(&self, endpoint: S, proxy: Option<SocketAddr>) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let endpoint = endpoint.into();
        for handle in self.policies().into_values() {
            handle
                .sender
                .try_send(Command::Sync {
                    endpoint: endpoint.clone(),
                    proxy,
                })
                .map_err(|_| Error::SendError)?;
        }
        Ok(())
    }

    pub async fn get_balance(
        &self,
        policy_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<Balance, Error> {
        let handle = self.get_policy_handle(policy_id)?;
        handle
            .sender
            .try_send(Command::GetBalance)
            .map_err(|_| Error::SendError)?;
        let mut notifications = handle.receiver();
        time::timeout(timeout, async move {
            while let Ok(res) = notifications.recv().await {
                if let Response::Balance(balance) = res {
                    return Ok(balance);
                }
            }
            Err(Error::NotFound)
        })
        .await
        .ok_or(Error::Timeout)?
    }

    pub async fn get_address(
        &self,
        policy_id: EventId,
        index: AddressIndex,
        timeout: Option<Duration>,
    ) -> Result<GetAddress, Error> {
        let handle = self.get_policy_handle(policy_id)?;
        handle
            .sender
            .try_send(Command::GetAddress(index))
            .map_err(|_| Error::SendError)?;
        let mut notifications = handle.receiver();
        time::timeout(timeout, async move {
            while let Ok(res) = notifications.recv().await {
                if let Response::Address(addr) = res {
                    return Ok(addr);
                }
            }
            Err(Error::NotFound)
        })
        .await
        .ok_or(Error::Timeout)?
    }

    pub async fn get_addresses(
        &self,
        policy_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<Vec<GetAddress>, Error> {
        let handle = self.get_policy_handle(policy_id)?;
        handle
            .sender
            .try_send(Command::GetAddresses)
            .map_err(|_| Error::SendError)?;
        let mut notifications = handle.receiver();
        time::timeout(timeout, async move {
            while let Ok(res) = notifications.recv().await {
                if let Response::Addresses(list) = res {
                    return Ok(list);
                }
            }
            Err(Error::NotFound)
        })
        .await
        .ok_or(Error::Timeout)?
    }

    pub async fn get_addresses_balances(
        &self,
        policy_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<HashMap<Script, u64>, Error> {
        let handle = self.get_policy_handle(policy_id)?;
        handle
            .sender
            .try_send(Command::GetAddressesBalances)
            .map_err(|_| Error::SendError)?;
        let mut notifications = handle.receiver();
        time::timeout(timeout, async move {
            while let Ok(res) = notifications.recv().await {
                if let Response::AddressesBalances(balances) = res {
                    return Ok(balances);
                }
            }
            Err(Error::NotFound)
        })
        .await
        .ok_or(Error::Timeout)?
    }

    pub async fn get_txs(
        &self,
        policy_id: EventId,
        sort: bool,
        timeout: Option<Duration>,
    ) -> Result<Vec<GetTransaction>, Error> {
        let handle = self.get_policy_handle(policy_id)?;
        handle
            .sender
            .try_send(Command::GetTxs { sort })
            .map_err(|_| Error::SendError)?;
        let mut notifications = handle.receiver();
        time::timeout(timeout, async move {
            while let Ok(res) = notifications.recv().await {
                if let Response::Txs(txs) = res {
                    return Ok(txs);
                }
            }
            Err(Error::NotFound)
        })
        .await
        .ok_or(Error::Timeout)?
    }

    pub async fn get_tx(
        &self,
        policy_id: EventId,
        txid: Txid,
        timeout: Option<Duration>,
    ) -> Result<GetTransaction, Error> {
        let handle = self.get_policy_handle(policy_id)?;
        handle
            .sender
            .try_send(Command::GetTx(txid))
            .map_err(|_| Error::SendError)?;
        let mut notifications = handle.receiver();
        time::timeout(timeout, async move {
            while let Ok(res) = notifications.recv().await {
                if let Response::Tx(tx) = res {
                    return Ok(tx);
                }
            }
            Err(Error::NotFound)
        })
        .await
        .ok_or(Error::Timeout)?
    }

    pub async fn get_utxos(
        &self,
        policy_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<Vec<GetUtxo>, Error> {
        let handle = self.get_policy_handle(policy_id)?;
        handle
            .sender
            .try_send(Command::GetUtxos)
            .map_err(|_| Error::SendError)?;
        let mut notifications = handle.receiver();
        time::timeout(timeout, async move {
            while let Ok(res) = notifications.recv().await {
                if let Response::Utxos(utxos) = res {
                    return Ok(utxos);
                }
            }
            Err(Error::NotFound)
        })
        .await
        .ok_or(Error::Timeout)?
    }

    pub async fn get_all_txs(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Vec<GetTransaction>, Error> {
        let mut txs = Vec::new();
        let mut already_seen = Vec::new();
        for GetPolicy {
            policy_id, policy, ..
        } in self.db.get_policies()?.into_iter()
        {
            if !already_seen.contains(&policy.descriptor) {
                for tx in self
                    .get_txs(policy_id, false, timeout)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                {
                    txs.push(tx)
                }
                already_seen.push(policy.descriptor);
            }
        }

        txs.sort_by(|a, b| {
            let a = match a.confirmation_time {
                ConfirmationTime::Confirmed { height, .. } => height,
                ConfirmationTime::Unconfirmed { .. } => u32::MAX,
            };

            let b = match b.confirmation_time {
                ConfirmationTime::Confirmed { height, .. } => height,
                ConfirmationTime::Unconfirmed { .. } => u32::MAX,
            };

            b.cmp(&a)
        });

        Ok(txs)
    }

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
        timeout: Option<Duration>,
    ) -> Result<Proposal, Error>
    where
        S: Into<String>,
    {
        let handle = self.get_policy_handle(policy_id)?;
        handle
            .sender
            .try_send(Command::Spend {
                address,
                amount,
                description: description.into(),
                fee_rate,
                utxos,
                policy_path,
            })
            .map_err(|_| Error::SendError)?;
        let mut notifications = handle.receiver();
        time::timeout(timeout, async move {
            while let Ok(res) = notifications.recv().await {
                if let Response::Proposal(proposal) = res {
                    return Ok(proposal);
                }
            }
            Err(Error::NotFound)
        })
        .await
        .ok_or(Error::Timeout)?
    }
}
