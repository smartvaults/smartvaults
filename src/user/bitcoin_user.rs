use anyhow::Result;
use bdk::{
	bitcoin::Network,
	blockchain::EsploraBlockchain,
	database::MemoryDatabase,
	descriptor,
	descriptor::IntoWalletDescriptor,
	keys::{
		bip39::{Language::English, Mnemonic},
		DescriptorKey, IntoDescriptorKey,
	},
	wallet::{AddressIndex::New, SyncOptions, Wallet},
	KeychainKind,
};
use bitcoin::util::bip32::{DerivationPath, ExtendedPrivKey, ExtendedPubKey, Fingerprint};
use miniscript::{Descriptor, DescriptorPublicKey, ScriptContext};
use nostr::prelude::Secp256k1;
use nostr_sdk::prelude::*;
use std::{fmt, str::FromStr};

const BIP86_DERIVATION_PATH: &str = "m/86'/0'/0'/0";
const BIP86_DERIVATION_INTERNAL_PATH: &str = "m/86'/0'/0'/1";

pub struct BitcoinUser {
	pub bitcoin_network: bitcoin::Network,
	pub root_priv: Option<ExtendedPrivKey>,
	pub wallet: Wallet<MemoryDatabase>,
	pub private_key: bitcoin::util::key::PrivateKey,
}

impl BitcoinUser {
	pub fn new(
		mnemonic: String,
		passphrase: Option<String>,
		bitcoin_network: &Network,
	) -> Result<BitcoinUser> {
		let secp = Secp256k1::new();
		let parsed_mnemonic = Mnemonic::parse_in_normalized(English, &mnemonic).unwrap();

		let seed =
			parsed_mnemonic.to_seed_normalized(&passphrase.clone().unwrap_or("".to_string()));
		let root_priv = ExtendedPrivKey::new_master(*bitcoin_network, &seed)?;

		let private_key = bitcoin::util::key::PrivateKey::new(
			SecretKey::from_slice(&seed[0..32]).unwrap(),
			*bitcoin_network,
		);

		// generate external and internal descriptor from mnemonic
		let (external_descriptor, _ext_keymap) = descriptor!(tr((
			seed.clone(),
			DerivationPath::from_str(BIP86_DERIVATION_PATH).unwrap()
		)))?
		.into_wallet_descriptor(&secp, *bitcoin_network)?;
		let (internal_descriptor, _int_keymap) = descriptor!(tr((
			seed,
			DerivationPath::from_str(BIP86_DERIVATION_INTERNAL_PATH).unwrap()
		)))?
		.into_wallet_descriptor(&secp, *bitcoin_network)?;

		let db = bdk::database::memory::MemoryDatabase::new();

		// not sure that we need to save the wallet, but doing it for now
		let wallet = Wallet::new(
			external_descriptor.clone(),
			Some(internal_descriptor.clone()),
			*bitcoin_network,
			db,
		);

		Ok(BitcoinUser {
			root_priv: Some(root_priv),
			private_key,
			bitcoin_network: *bitcoin_network,
			wallet: wallet.unwrap(),
		})
	}

	pub fn setup_keys<Ctx: ScriptContext>(
		&self,
	) -> (DescriptorKey<Ctx>, DescriptorKey<Ctx>, Fingerprint) {
		let secp = Secp256k1::new();
		let path = DerivationPath::from_str(BIP86_DERIVATION_PATH).unwrap();
		let tprv = self.root_priv.unwrap().derive_priv(&secp, &path).unwrap();
		let tpub = ExtendedPubKey::from_priv(&secp, &tprv);
		let fingerprint = tprv.fingerprint(&secp);
		let prvkey = (tprv, path.clone()).into_descriptor_key().unwrap();
		let pubkey = (tpub, path).into_descriptor_key().unwrap();

		(prvkey, pubkey, fingerprint)
	}

	pub fn get_descriptor(&self) -> Descriptor<DescriptorPublicKey> {
		let secp = Secp256k1::new();
		let desc = self
			.wallet
			.public_descriptor(KeychainKind::External)
			.unwrap()
			.unwrap()
			.into_wallet_descriptor(&secp, self.bitcoin_network)
			.unwrap()
			.0;
		desc
	}

	pub fn get_change_descriptor(&self) -> Descriptor<DescriptorPublicKey> {
		let secp = Secp256k1::new();
		let desc = self
			.wallet
			.public_descriptor(KeychainKind::Internal)
			.unwrap()
			.unwrap()
			.into_wallet_descriptor(&secp, self.bitcoin_network)
			.unwrap()
			.0;
		desc
	}

	pub fn get_balance(&self, mut bitcoin_endpoint: Option<&str>) -> bdk::Balance {
		const DEFAULT_TESTNET_ENDPOINT: &str = "https://blockstream.info/testnet/api";
		const DEFAULT_BITCOIN_ENDPOINT: &str = "https://blockstream.info/api";
		if bitcoin_endpoint.is_none() {
			if &self.bitcoin_network == &bitcoin::network::constants::Network::Testnet {
				bitcoin_endpoint = Some(DEFAULT_TESTNET_ENDPOINT);
			} else {
				bitcoin_endpoint = Some(DEFAULT_BITCOIN_ENDPOINT);
			}
		}

		let esplora = EsploraBlockchain::new(&bitcoin_endpoint.unwrap(), 20);

		self.wallet.sync(&esplora, SyncOptions::default()).unwrap();

		return self.wallet.get_balance().unwrap()
	}
}

impl fmt::Display for BitcoinUser {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let secp = Secp256k1::new();

		writeln!(f, "\nBitcoin User Configuration")?;
		writeln!(f, "  Saved Data")?;
		writeln!(f, "    Root Private Key	: {}", &self.root_priv.unwrap().to_string())?;
		writeln!(f, "    Private Key		: {}", &self.private_key.to_string())?;
		// writeln!(f, "    Wallet Object		: {:?}", &self.wallet)?;
		writeln!(f)?;
		writeln!(f, "  Derived Data")?;
		writeln!(
			f,
			"    Extended Pub Key	: {}",
			ExtendedPubKey::from_priv(&secp, &self.root_priv.unwrap())
		)?;
		writeln!(f, "    Output Descriptor	: {}", &self.get_descriptor().to_string())?;
		writeln!(f, "    Change Descriptor	: {}", &self.get_change_descriptor().to_string())?;
		writeln!(f, "    Ext Address 1	: {}", &self.wallet.get_address(New).unwrap())?;
		writeln!(f, "    Ext Address 2	: {}", &self.wallet.get_address(New).unwrap())?;
		writeln!(f, "    Change Address	: {}", &self.wallet.get_internal_address(New).unwrap())?;

		let balance = self.get_balance(None);
		writeln!(f, "\nBitcoin Balances")?;
		writeln!(f, "  Immature            	: {} ", balance.immature)?;
		writeln!(f, "  Trusted Pending     	: {} ", balance.trusted_pending)?;
		writeln!(f, "  Untrusted Pending   	: {} ", balance.untrusted_pending)?;
		writeln!(f, "  Confirmed           	: {} ", balance.confirmed)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {

	use super::*;
	use crate::user::constants::user_constants;

	#[test]
	fn test_alice_keys() {
		let secp = Secp256k1::new();
		let user_constants = user_constants();
		let alice_constants = user_constants.get(&String::from("Alice")).unwrap();
		let alice_bitcoin = BitcoinUser::new(
			alice_constants.mnemonic.to_string(),
			Some(alice_constants.passphrase.to_string()),
			&bitcoin::network::constants::Network::Testnet,
		)
		.unwrap();

		assert_eq!(format!("{}", alice_bitcoin.root_priv.unwrap()), "tprv8ZgxMBicQKsPeFd9cajKjGekZW5wDXq2e1vpKToJvZMqjyNkMqmr7exPFUbJ92YxSkqL4w19HpuzYkVYvc4n4pvySBmJfsawS7Seb8FzuNJ".to_string());
		assert_eq!(format!("{}", ExtendedPubKey::from_priv(&secp, &alice_bitcoin.root_priv.unwrap())),
		"tpubD6NzVbkrYhZ4XiewWEPv8gJs8XbsNs1wDKXbbyqcLqAEaTdWzEbSJ9aFRamjrj3RQKyZ2Q848BkMxyt6J6e36Y14ga6Et7suFXk3RKFqEaA"
		.to_string());
		assert_eq!(format!("{}", alice_bitcoin.get_descriptor()), "tr([9b5d4149/86'/0'/0']tpubDDfNLjZpqGcbyjiSzxxbvTRqvySNkCQKKDJHXkJPZCKQPVsVX9fcuvkd65MU3oyRmqgzpzvuEUxe6zstCCDP2ogHn5ModwnrxP4cdWLFdc3/0/*)#2azlv5fk".to_string());

		println!("Alice {} ", alice_bitcoin);
	}

	#[test]
	fn test_key_derivation_for_single_key_p2tr_outputs() -> Result<()> {
		let secp = Secp256k1::new();

		// Test data from:
		// https://github.com/bitcoin/bips/blob/master/bip-0086.mediawiki#test-vectors
		const MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
		const EXPECTED_ROOT_PRIV: &str = "xprv9s21ZrQH143K3GJpoapnV8SFfukcVBSfeCficPSGfubmSFDxo1kuHnLisriDvSnRRuL2Qrg5ggqHKNVpxR86QEC8w35uxmGoggxtQTPvfUu";
		const EXPECTED_ROOT_PUB: &str = "xpub661MyMwAqRbcFkPHucMnrGNzDwb6teAX1RbKQmqtEF8kK3Z7LZ59qafCjB9eCRLiTVG3uxBxgKvRgbubRhqSKXnGGb1aoaqLrpMBDrVxga8";

		let bip86_user = BitcoinUser::new(
			MNEMONIC.to_string(),
			None,
			&bitcoin::network::constants::Network::Bitcoin,
		)?;

		assert_eq!(format!("{}", bip86_user.root_priv.unwrap()), EXPECTED_ROOT_PRIV.to_string());
		assert_eq!(
			format!("{}", ExtendedPubKey::from_priv(&secp, &bip86_user.root_priv.unwrap())),
			EXPECTED_ROOT_PUB.to_string()
		);

		// check that first 3 addresses match
		assert_eq!(
			"bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr",
			format!("{}", bip86_user.wallet.get_address(bdk::wallet::AddressIndex::New)?)
		);

		assert_eq!(
			"bc1p4qhjn9zdvkux4e44uhx8tc55attvtyu358kutcqkudyccelu0was9fqzwh",
			format!("{}", bip86_user.wallet.get_address(bdk::wallet::AddressIndex::New)?)
		);

		assert_eq!(
			"bc1p3qkhfews2uk44qtvauqyr2ttdsw7svhkl9nkm9s9c3x4ax5h60wqwruhk7",
			format!("{}", bip86_user.wallet.get_internal_address(bdk::wallet::AddressIndex::New)?)
		);

		println!("bip86 user {}", bip86_user);
		Ok(())
	}
}
