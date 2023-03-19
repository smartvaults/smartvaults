#[macro_use]
extern crate serde;
pub extern crate bdk;
pub extern crate nostr_sdk;

use std::path::Path;

use bdk::database::MemoryDatabase;
use bdk::Wallet;
use keechain_core::bip39::Mnemonic;
use keechain_core::bitcoin::Network;
use keechain_core::types::WordCount;
pub use keechain_core::Result;
pub use keechain_core::*;
use nostr_sdk::blocking::Client;
use nostr_sdk::Options;

pub mod constants;
pub mod policy;
pub mod proposal;
pub mod util;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Keechain(#[from] keechain_core::types::keychain::Error),
    #[error(transparent)]
    Bdk(#[from] bdk::Error),
    #[error(transparent)]
    Nostr(#[from] nostr_sdk::client::Error),
    #[error(transparent)]
    Nip06(#[from] nostr_sdk::nips::nip06::Error),
    #[error("{0}")]
    Generic(String),
}

/// Coinstr Keystore
pub struct Coinstr {
    network: Network,
    keechain: KeeChain,
}

impl Coinstr {
    pub fn open<P, PSW>(path: P, get_password: PSW, network: Network) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        PSW: FnOnce() -> Result<String>,
    {
        let mut keechain: KeeChain = KeeChain::open(path, get_password)?;
        let passphrase: Option<String> = keechain.keychain.get_passphrase(0);
        keechain.keychain.apply_passphrase(passphrase);

        Ok(Self { network, keechain })
    }

    pub fn generate<P, PSW, PASSP>(
        path: P,
        get_password: PSW,
        word_count: WordCount,
        get_passphrase: PASSP,
        network: Network,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        PSW: FnOnce() -> Result<String>,
        PASSP: FnOnce() -> Result<Option<String>>,
    {
        let mut keechain: KeeChain =
            KeeChain::generate(path, get_password, word_count, || Ok(None))?;
        let passphrase: Option<String> =
            get_passphrase().map_err(|e| Error::Generic(e.to_string()))?;
        if let Some(passphrase) = passphrase {
            keechain.keychain.add_passphrase(&passphrase);
            keechain.save()?;
            keechain.keychain.apply_passphrase(Some(passphrase));
        }

        Ok(Self { network, keechain })
    }

    pub fn restore<P, PSW, M, PASSP>(
        path: P,
        get_password: PSW,
        get_mnemonic: M,
        get_passphrase: PASSP,
        network: Network,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        PSW: FnOnce() -> Result<String>,
        M: FnOnce() -> Result<Mnemonic>,
        PASSP: FnOnce() -> Result<Option<String>>,
    {
        let mut keechain: KeeChain = KeeChain::restore(path, get_password, get_mnemonic)?;
        let passphrase: Option<String> =
            get_passphrase().map_err(|e| Error::Generic(e.to_string()))?;
        if let Some(passphrase) = passphrase {
            keechain.keychain.add_passphrase(&passphrase);
            keechain.save()?;
            keechain.keychain.apply_passphrase(Some(passphrase));
        }

        Ok(Self { network, keechain })
    }

    pub fn save(&self) -> Result<(), Error> {
        Ok(self.keechain.save()?)
    }

    pub fn check_password<S>(&self, password: S) -> bool
    where
        S: Into<String>,
    {
        self.keechain.check_password(password)
    }

    pub fn rename<P>(&mut self, path: P) -> Result<(), Error>
    where
        P: AsRef<Path>,
    {
        Ok(self.keechain.rename(path)?)
    }

    pub fn change_password<NPSW>(&mut self, get_new_password: NPSW) -> Result<(), Error>
    where
        NPSW: FnOnce() -> Result<String>,
    {
        Ok(self.keechain.change_password(get_new_password)?)
    }

    pub fn wipe(&self) -> Result<(), Error> {
        Ok(self.keechain.wipe()?)
    }

    pub fn keychain(&self) -> Keychain {
        self.keechain.keychain.clone()
    }

    pub fn network(&self) -> Network {
        self.network
    }

    pub fn wallet<S>(&self, descriptor: S) -> Result<Wallet<MemoryDatabase>, Error>
    where
        S: Into<String>,
    {
        let db = MemoryDatabase::new();
        Ok(Wallet::new(&descriptor.into(), None, self.network, db)?)
    }

    pub fn nostr_client(&self, relays: Vec<String>) -> Result<Client, Error> {
        let opts = Options::new().wait_for_send(true);
        let keys = self.keechain.keychain.nostr_keys()?;
        let client = Client::new_with_opts(&keys, opts);
        let relays = relays.iter().map(|url| (url, None)).collect();
        client.add_relays(relays)?;
        client.connect();
        Ok(client)
    }
}
