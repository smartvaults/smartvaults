use bdk::blockchain::EsploraBlockchain;
use bdk::database::MemoryDatabase;
use bdk::keys::bip39::{Language::English, Mnemonic};
use bdk::keys::{DerivableKey, DescriptorKey};
use bdk::wallet::SyncOptions;
use bdk::wallet::Wallet;
use bitcoin::network::address;
use bitcoin::util::bip32::{DerivationPath, ExtendedPrivKey};
use bitcoin::Network;
use miniscript::{ScriptContext, Descriptor, DescriptorPublicKey};
use nostr::nips::nip19::ToBech32;
use nostr::{
    prelude::{FromMnemonic, PublicKey, Secp256k1, SecretKey},
    Keys, Result,
};
// use secp256k1::Secp256k1;
use std::collections::HashMap;
use bdk::keys::ExtendedKey;
use std::error::Error;
use std::fmt;
use std::vec;
use bdk::wallet::AddressInfo;

use std::str::FromStr;

enum KeyType {
    Nostr,
    Bech32,
}
pub struct User {
    mnemonic: Mnemonic,
    passphrase: String,

    nostr_secret_hex: nostr::prelude::SecretKey,
    nostr_secret_bech32: String,

    nostr_x_only_public_key: nostr::prelude::XOnlyPublicKey,
    nostr_public_hex: nostr::prelude::PublicKey,
    nostr_public_bech32: String,

    output_descriptor: Descriptor<DescriptorPublicKey>,
    addresses: Vec<AddressInfo>,

}

impl User
  {

    pub fn new(mnemonic: &String, passphrase: &String) -> Result<User, Box<dyn Error>> {
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
        let root_key = ExtendedPrivKey::new_master(Network::Testnet, &seed)?;
        let xpub = root_key.into_extended_key()?.into_descriptor_key(None, path).unwrap();
        let (desc, _, _) = bdk::descriptor!(tr(xpub)).unwrap();
        
        let db = bdk::database::memory::MemoryDatabase::new();
        let wallet = Wallet::new(desc.clone(), None, Network::Testnet, db);
        let address = wallet
            .as_ref()
            .unwrap()
            .get_address(bdk::wallet::AddressIndex::New)
            .unwrap();
        
        let mut addresses = vec![address];
        // println!("  First Address       : {} ", address.to_string());

        let address = wallet
            .unwrap()
            .get_address(bdk::wallet::AddressIndex::New)
            .unwrap();
        addresses.push(address);
        // println!("  Second Address      : {} ", address.to_string());

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
        })
    }

    pub fn alice() -> Result<User, Box<dyn Error>> {
        User::new(
            &"carry surface crater rude auction ritual banana elder shuffle much wonder decrease"
                .to_string(),
            &"oy+hB/qeJ1AasCCR".to_string(),
        )
    }

    // pub fn bob() -> User {
    //     User {
    //         mnemonic:Mnemonic::parse_in_normalized(English, "market museum car noodle cream pool enhance please level price slide process").unwrap(),
    //         passphrase: "B3Q0YHYYHmF798Jg".to_string(),
    //     }
    // }

    // pub fn charlie() -> User {
    //     User {
    //         mnemonic: Mnemonic::parse_in_normalized(English, "cry modify gallery home desert tongue immune address bunker bean tone giggle").unwrap(),
    //         passphrase: "nTVuKiINc5TKMjfV".to_string(),
    //     }
    // }

    // pub fn david() -> User {
    //     User {
    //         mnemonic: Mnemonic::parse_in_normalized(English, "alone hospital depth worth vapor lazy burst skill apart accuse maze evidence").unwrap(),
    //         passphrase: "f5upOqUyG0iPY4n+".to_string(),
    //     }
    // }

    // pub fn erika() -> User {
    //     User {
    //         mnemonic: Mnemonic::parse_in_normalized(English, "confirm rifle kit warrior aware clump shallow eternal real shift puzzle wife").unwrap(),
    //         passphrase: "JBtdXy+2ut2fxplW".to_string(),
    //     }
    // }

    // pub fn print_nostr_keys(&self) -> Result<()> {
    //     // let keys = Keys::from_mnemonic(&self.mnemonic, Some(&self.passphrase)).unwrap();

    //
    //     Ok(())
    // }

    pub fn known_users() -> Vec<User> {
        vec![
            User::alice().unwrap(),
            // User::bob(),
            // User::charlie(),
            // User::david(),
            // User::erika(),
        ]
    }

    pub fn get_balance(&self,
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
        // let mut output = String::from("");

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
        writeln!(f, "  Output Descriptor   : {}", &self.output_descriptor.to_string())?;

        for address in &self.addresses {
            writeln!(f, "  Address             : {}", address.to_string())?;
        }

        let balance = self.get_balance(&"https://blockstream.info/testnet/api".to_string(), Network::Testnet);
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
}
