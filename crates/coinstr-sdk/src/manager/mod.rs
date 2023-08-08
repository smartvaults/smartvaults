// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::Add;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_utility::{thread, time};
use bdk::bitcoin::{Address, Network, OutPoint, Script, Txid};
use bdk::blockchain::{ElectrumBlockchain, GetHeight};
use bdk::database::SqliteDatabase;
use bdk::electrum_client::{Client as ElectrumClient, Config as ElectrumConfig, Socks5Config};
use bdk::wallet::AddressIndex;
use bdk::{Balance, LocalUtxo, SyncOptions, TransactionDetails, Wallet};
use coinstr_core::Policy;
use nostr_sdk::{EventId, Timestamp};
use parking_lot::Mutex;
use thiserror::Error;
use tokio::sync::broadcast;
use tokio::sync::mpsc::{self, Sender};

use crate::client::Message;
use crate::constants::WALLET_SYNC_INTERVAL;
use crate::db::model::{GetAddress, GetTransaction, GetUtxo};
use crate::db::Store;
use crate::{Label, LabelData};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Bdk(#[from] bdk::Error),
    #[error(transparent)]
    Electrum(#[from] bdk::electrum_client::Error),
    #[error(transparent)]
    Address(#[from] coinstr_core::bitcoin::util::address::Error),
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
        sender: broadcast::Sender<Message>,
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
}

#[derive(Debug, Clone)]
pub struct PolicyHandle {
    sender: Sender<Command>,
    receiver: broadcast::Sender<Response>,
}

impl PolicyHandle {
    pub fn receiver(&self) -> broadcast::Receiver<Response> {
        self.receiver.subscribe()
    }
}

#[derive(Debug, Clone)]
pub struct Manager {
    policies: Arc<Mutex<HashMap<EventId, PolicyHandle>>>,
    db: Store,
}

impl Manager {
    pub fn new(db: Store) -> Self {
        Self {
            policies: Arc::new(Mutex::new(HashMap::new())),
            db,
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

    fn sync_wallet<S>(
        &self,
        policy_id: EventId,
        wallet: &Wallet<SqliteDatabase>,
        endpoint: S,
        proxy: Option<SocketAddr>,
    ) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let endpoint = endpoint.into();

        tracing::info!("Initializing electrum client: endpoint={endpoint}, proxy={proxy:?}");
        let proxy: Option<Socks5Config> = proxy.map(Socks5Config::new);
        let config = ElectrumConfig::builder().socks5(proxy)?.build();
        let blockchain = ElectrumBlockchain::from(ElectrumClient::from_config(&endpoint, config)?);

        if !self.db.block_height.is_synced() {
            match blockchain.get_height() {
                Ok(height) => {
                    self.db.block_height.set_block_height(height);
                    self.db.block_height.just_synced();
                }
                Err(e) => tracing::error!("Impossible to sync block height: {e}"),
            }
        }

        wallet.sync(&blockchain, SyncOptions::default())?;

        self.db
            .update_last_sync(policy_id, Some(Timestamp::now()))?;
        Ok(())
    }

    fn _get_address(
        &self,
        policy_id: EventId,
        index: AddressIndex,
        wallet: &Wallet<SqliteDatabase>,
    ) -> Result<GetAddress, Error> {
        let address = wallet.get_address(index)?;

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

    fn _get_addresses(
        &self,
        policy_id: EventId,
        wallet: &Wallet<SqliteDatabase>,
    ) -> Result<Vec<GetAddress>, Error> {
        let last_unused = wallet.get_address(AddressIndex::LastUnused)?;
        let script_labels: HashMap<Script, Label> = self.db.get_addresses_labels(policy_id)?;

        let mut addresses: Vec<GetAddress> = Vec::new();

        for index in 0.. {
            let addr = wallet.get_address(AddressIndex::Peek(index))?;
            addresses.push(GetAddress {
                address: addr.address.clone(),
                label: script_labels
                    .get(&addr.address.script_pubkey())
                    .map(|l| l.text()),
            });
            if addr == last_unused {
                for i in index + 1..index + 20 {
                    let addr = wallet.get_address(AddressIndex::Peek(i))?;
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

    fn _get_addresses_balances(
        &self,
        wallet: &Wallet<SqliteDatabase>,
    ) -> Result<HashMap<Script, u64>, Error> {
        // Get UTXOs
        let utxos: Vec<LocalUtxo> = wallet.list_unspent()?;

        let mut map: HashMap<Script, u64> = HashMap::new();

        for utxo in utxos.into_iter() {
            map.entry(utxo.txout.script_pubkey)
                .and_modify(|amount| *amount += utxo.txout.value)
                .or_insert(utxo.txout.value);
        }

        Ok(map)
    }

    fn _get_txs(
        &self,
        policy_id: EventId,
        wallet: &Wallet<SqliteDatabase>,
        sort: bool,
    ) -> Result<Vec<GetTransaction>, Error> {
        let txs: Vec<TransactionDetails> = wallet.list_transactions(true)?;

        let descriptions: HashMap<Txid, String> = self.db.get_txs_descriptions(policy_id)?;
        let script_labels: HashMap<Script, Label> = self.db.get_addresses_labels(policy_id)?;

        let mut list: Vec<GetTransaction> = Vec::new();

        for tx in txs.into_iter() {
            let label: Option<String> = if tx.received > tx.sent {
                let mut label = None;
                if let Some(transaction) = tx.transaction.as_ref() {
                    for txout in transaction.output.iter() {
                        if wallet.is_mine(&txout.script_pubkey)? {
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

        if sort {
            list.sort_by(|a, b| {
                b.confirmation_time
                    .as_ref()
                    .map(|t| t.height)
                    .unwrap_or(u32::MAX)
                    .cmp(
                        &a.confirmation_time
                            .as_ref()
                            .map(|t| t.height)
                            .unwrap_or(u32::MAX),
                    )
            });
        }

        Ok(list)
    }

    fn _get_tx(
        &self,
        policy_id: EventId,
        txid: Txid,
        wallet: &Wallet<SqliteDatabase>,
    ) -> Result<GetTransaction, Error> {
        let tx: TransactionDetails = wallet.get_tx(&txid, true)?.ok_or(Error::NotFound)?;

        let label: Option<String> = if tx.received > tx.sent {
            let mut label = None;
            for txout in tx
                .transaction
                .as_ref()
                .ok_or(Error::NotFound)?
                .output
                .iter()
            {
                if wallet.is_mine(&txout.script_pubkey)? {
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

    fn _get_utxos(
        &self,
        policy_id: EventId,
        wallet: &Wallet<SqliteDatabase>,
    ) -> Result<Vec<GetUtxo>, Error> {
        let utxos: Vec<LocalUtxo> = wallet.list_unspent()?;

        // Get labels
        let script_labels: HashMap<Script, Label> = self.db.get_addresses_labels(policy_id)?;
        let utxo_labels: HashMap<OutPoint, Label> = self.db.get_utxos_labels(policy_id)?;

        // Compose output
        Ok(utxos
            .into_iter()
            .map(|utxo| GetUtxo {
                label: utxo_labels
                    .get(&utxo.outpoint)
                    .or_else(|| script_labels.get(&utxo.txout.script_pubkey))
                    .map(|l| l.text()),
                utxo,
            })
            .collect())
    }

    pub fn load_policy(
        &self,
        policy_id: EventId,
        policy: Policy,
        db: SqliteDatabase,
        network: Network,
    ) -> Result<(), Error> {
        let wallet = Wallet::new(&policy.descriptor.to_string(), None, network, db)?;

        let mut policies = self.policies.lock();
        if policies.contains_key(&policy_id) {
            return Err(Error::AlreadyLoaded(policy_id));
        }

        let (sender, mut receiver) = mpsc::channel::<Command>(4096);
        let (tx, mut rx) = broadcast::channel::<Response>(4096);

        // Keep channel opened
        thread::spawn(async move { while rx.recv().await.is_ok() {} });

        let tx_sender = tx.clone();
        let this = self.clone();
        thread::spawn(async move {
            while let Some(cmd) = receiver.recv().await {
                match cmd {
                    Command::Sync {
                        endpoint,
                        proxy,
                        sender,
                    } => match this.db.get_last_sync(policy_id) {
                        Ok(last_sync) => {
                            let last_sync: Timestamp =
                                last_sync.unwrap_or_else(|| Timestamp::from(0));
                            if last_sync.add(WALLET_SYNC_INTERVAL) <= Timestamp::now() {
                                tracing::info!("Syncing policy {policy_id}");
                                let now = Instant::now();
                                match this.sync_wallet(policy_id, &wallet, endpoint, proxy) {
                                    Ok(_) => {
                                        let _ =
                                            sender.send(Message::WalletSyncCompleted(policy_id));
                                        tracing::info!(
                                            "Policy {policy_id} synced in {} ms",
                                            now.elapsed().as_millis()
                                        );
                                    }
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
                    Command::GetBalance => match wallet.get_balance() {
                        Ok(balance) => this.send(&tx_sender, Response::Balance(balance)),
                        Err(e) => tracing::error!("Impossible to get balance: {e}"),
                    },
                    Command::GetAddresses => match this._get_addresses(policy_id, &wallet) {
                        Ok(addresses) => this.send(&tx_sender, Response::Addresses(addresses)),
                        Err(e) => tracing::error!("Impossible to get addresses: {e}"),
                    },
                    Command::GetAddressesBalances => match this._get_addresses_balances(&wallet) {
                        Ok(balances) => {
                            this.send(&tx_sender, Response::AddressesBalances(balances))
                        }
                        Err(e) => tracing::error!("Impossible to get addresses balances: {e}"),
                    },
                    Command::GetAddress(index) => {
                        match this._get_address(policy_id, index, &wallet) {
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

    pub fn sync_all<S>(
        &self,
        endpoint: S,
        proxy: Option<SocketAddr>,
        sender: broadcast::Sender<Message>,
    ) -> Result<(), Error>
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
                    sender: sender.clone(),
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
}
