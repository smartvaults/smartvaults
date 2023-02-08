use bdk::blockchain::EsploraBlockchain;
use bdk::database::MemoryDatabase;
use bdk::keys::bip39::{Language::English, Mnemonic};
use bdk::keys::DerivableKey;
use bdk::wallet::AddressInfo;
use bdk::wallet::SyncOptions;
use bdk::wallet::Wallet;
use bitcoin::util::bip32::{DerivationPath, ExtendedPrivKey};
use bitcoin::Network;
use miniscript::{Descriptor, DescriptorPublicKey};
use nostr::nips::nip19::ToBech32;
use nostr::{
    prelude::{FromMnemonic, PublicKey, Secp256k1},
    Keys, //Result,
};
use std::error::Error;
use std::fmt;
use std::vec;
use nostr_sdk::prelude::*;

use std::str::FromStr;

pub struct User {
    pub name: Option<String>,
    mnemonic: Mnemonic,
    passphrase: String,
    pub nostr_secret_hex: nostr::prelude::SecretKey,
    pub nostr_secret_bech32: String,
    pub nostr_x_only_public_key: nostr::prelude::XOnlyPublicKey,
    nostr_public_hex: nostr::prelude::PublicKey,
    nostr_public_bech32: String,
    output_descriptor: Descriptor<DescriptorPublicKey>,
    addresses: Vec<AddressInfo>,
}

impl User {
    // pub fn from_xpub(x_only_public_key: &String) {
    //     Ok(User {
           
    //         nostr_x_only_public_key: XOnlyPublicKey::from_str(x_only_public_key.as_str()).expect("Invalid public key")
           
    //     });
    // }

    pub fn new(
        mnemonic: &String,
        passphrase: &String,
        name: Option<String>,
        bitcoin_network: &Network,
    ) -> Result<User, Box<dyn Error>> {
        let keys = Keys::from_mnemonic(mnemonic.clone(), Some(passphrase.to_string())).unwrap();

        // begin nostr key work
        let seeded_secret_key = keys.secret_key().unwrap();
        let secp = Secp256k1::new();
        let x_public_key = &seeded_secret_key.x_only_public_key(&secp);
        let hex_public_key = PublicKey::from_secret_key(&secp, &seeded_secret_key);
        // end nostr key work

        let parsed_mnemonic = Mnemonic::parse_in_normalized(English, mnemonic).unwrap();
        let path = DerivationPath::from_str("m/44'/0'/0'/0").unwrap();
        let seed = parsed_mnemonic.to_seed_normalized(&passphrase);
        let root_key = ExtendedPrivKey::new_master(*bitcoin_network, &seed)?;
        let xpub = root_key
            .into_extended_key()?
            .into_descriptor_key(None, path)
            .unwrap();
        let (desc, _, _) = bdk::descriptor!(tr(xpub)).unwrap();

        let db = bdk::database::memory::MemoryDatabase::new();
        let wallet = Wallet::new(desc.clone(), None, Network::Testnet, db);
        let address = wallet
            .as_ref()
            .unwrap()
            .get_address(bdk::wallet::AddressIndex::New)
            .unwrap();

        let mut addresses = vec![address];

        let address = wallet
            .unwrap()
            .get_address(bdk::wallet::AddressIndex::New)
            .unwrap();
        addresses.push(address);

        Ok(User {
            nostr_secret_bech32: seeded_secret_key.to_bech32().unwrap().to_string(),
            nostr_secret_hex: seeded_secret_key,
            nostr_public_hex: hex_public_key,
            nostr_public_bech32: x_public_key.0.to_bech32().unwrap().to_string(),
            nostr_x_only_public_key: x_public_key.0,
            mnemonic: parsed_mnemonic,
            passphrase: passphrase.clone(),
            output_descriptor: desc,
            addresses,
            name,
        })
    }

    pub fn alice() -> Result<User, Box<dyn Error>> {
        User::new(
            &"carry surface crater rude auction ritual banana elder shuffle much wonder decrease"
                .to_string(),
            &"oy+hB/qeJ1AasCCR".to_string(),
            Some("Alice".to_string()),
            &Network::Testnet,
        )
    }

    pub fn bob() -> Result<User, Box<dyn Error>> {
        User::new(
            &"market museum car noodle cream pool enhance please level price slide process"
                .to_string(),
            &"B3Q0YHYYHmF798Jg".to_string(),
            Some("Bob".to_string()),
            &Network::Testnet,
        )
    }

    pub fn charlie() -> Result<User, Box<dyn Error>> {
        User::new(
            &"cry modify gallery home desert tongue immune address bunker bean tone giggle"
                .to_string(),
            &"nTVuKiINc5TKMjfV".to_string(),
            Some("Charlie".to_string()),
            &Network::Testnet,
        )
    }

    pub fn david() -> Result<User, Box<dyn Error>> {
        User::new(
            &"alone hospital depth worth vapor lazy burst skill apart accuse maze evidence"
                .to_string(),
            &"f5upOqUyG0iPY4n+".to_string(),
            Some("David".to_string()),
            &Network::Testnet,
        )
    }

    pub fn erika() -> Result<User, Box<dyn Error>> {
        User::new(
            &"confirm rifle kit warrior aware clump shallow eternal real shift puzzle wife"
                .to_string(),
            &"JBtdXy+2ut2fxplW".to_string(),
            Some("Erika".to_string()),
            &Network::Testnet,
        )
    }

    #[allow(dead_code)]
    pub fn known_users() -> Vec<User> {
        vec![
            User::alice().unwrap(),
            User::bob().unwrap(),
            User::charlie().unwrap(),
            User::david().unwrap(),
            User::erika().unwrap(),
        ]
    }

    pub fn get(name: &String) -> Result<User, Box<dyn Error>> {
        // type Err = UserNotFoundError;
        match name.as_str() {
            "alice" => User::alice(),
            "bob" => User::bob(),
            "charlie" => User::charlie(),
            "david" => User::david(),
            _ => User::erika(),            
            // _ => return Err(UserNotFoundError),
        }
    }

    pub fn get_balance(
        &self,
        bitcoin_endpoint: &String,
        bitcoin_network: bitcoin::Network,
    ) -> bdk::Balance {
        let esplora = EsploraBlockchain::new(&bitcoin_endpoint, 20);

        let wallet = Wallet::new(
            &self.output_descriptor.to_string(),
            None,
            bitcoin_network,
            MemoryDatabase::default(),
        )
        .unwrap();

        wallet.sync(&esplora, SyncOptions::default()).unwrap();

        return wallet.get_balance().unwrap();
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        if self.name.is_some() {
            writeln!(f, "Name       : {}", &self.name.as_ref().unwrap())?;
        }
        writeln!(f, "\nMnemonic   : {:?} ", &self.mnemonic.to_string())?;
        writeln!(f, "Passphrase : \"{}\" ", &self.passphrase)?;

        writeln!(f, "\nNostr Configuration")?;
        writeln!(
            f,
            "  Secret Key (HEX)    : {} ",
            &self.nostr_secret_hex.display_secret().to_string()
        )?;
        writeln!(f, "  Secret Key (bech32) : {} ", &self.nostr_secret_bech32)?;

        writeln!(
            f,
            "  Public Key (HEX)    : {} ",
            &self.nostr_public_hex.to_string()
        )?;
        writeln!(
            f,
            "  X Only Public Key   : {} ",
            &self.nostr_x_only_public_key.to_string()
        )?;
        writeln!(f, "  Public Key (bech32) : {} ", &self.nostr_public_bech32)?;

        writeln!(f, "\nBitcoin Configuration")?;
        writeln!(
            f,
            "  Output Descriptor   : {}",
            &self.output_descriptor.to_string()
        )?;

        for address in &self.addresses {
            writeln!(f, "  Address             : {}", address.to_string())?;
        }

        let balance = self.get_balance(
            &"https://blockstream.info/testnet/api".to_string(),
            Network::Testnet,
        );
        writeln!(f, "\nBitcoin Balances")?;
        writeln!(f, "  Immature            : {} ", balance.immature)?;
        writeln!(f, "  Trusted Pending     : {} ", balance.trusted_pending)?;
        writeln!(f, "  Untrusted Pending   : {} ", balance.untrusted_pending)?;
        writeln!(f, "  Confirmed           : {} ", balance.confirmed)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn dump_alice() {
        println!("Alice:\n{}\n", &User::alice().unwrap());
    }

    #[test]
    fn dump_known_users() {
        for user in User::known_users() {
            println!("{}", user);
        }
    }
}
