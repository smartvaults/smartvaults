use std::{fmt, str::FromStr};

use bdk::{
	blockchain::ElectrumBlockchain,
	database::MemoryDatabase,
	descriptor::IntoWalletDescriptor,
	electrum_client::Client,
	keys::{DescriptorKey, IntoDescriptorKey},
	miniscript::{Descriptor, DescriptorPublicKey, ScriptContext},
	wallet::{AddressIndex, SyncOptions, Wallet},
	KeychainKind,
};
use keechain_core::{
	bitcoin::util::{
		bip32::{DerivationPath, ExtendedPrivKey, ExtendedPubKey, Fingerprint},
		key::PrivateKey,
	},
	types::{Descriptors, Purpose, Seed},
	util::bip::bip32::Bip32RootKey,
};
use nostr_sdk::prelude::*;

use crate::{DEFAULT_BITCOIN_ENDPOINT, DEFAULT_TESTNET_ENDPOINT};

const BIP86_DERIVATION_PATH: &str = "m/86'/0'/0'/0";

pub struct BitcoinUser {
	pub bitcoin_network: Network,
	pub root_priv: Option<ExtendedPrivKey>,
	pub wallet: Wallet<MemoryDatabase>,
	pub private_key: PrivateKey,
}

impl BitcoinUser {
	pub fn new(seed: Seed, network: Network) -> Result<Self> {
		let seed_bytes = seed.to_bytes();
		let private_key = PrivateKey::new(SecretKey::from_slice(&seed_bytes[0..32])?, network);
		let root_priv = Some(seed.to_bip32_root_key(network)?);

		let descriptors = Descriptors::new(seed, network, None)?;
		let external = descriptors.get_by_purpose(Purpose::TR, false).unwrap();
		let internal = descriptors.get_by_purpose(Purpose::TR, true).unwrap();

		let db = MemoryDatabase::new();

		// not sure that we need to save the wallet, but doing it for now
		let wallet = Wallet::new(external, Some(internal), network, db)?;

		Ok(Self { root_priv, private_key, bitcoin_network: network, wallet })
	}

	pub fn setup_keys<Ctx: ScriptContext>(
		&self,
	) -> (DescriptorKey<Ctx>, DescriptorKey<Ctx>, Fingerprint) {
		let path = DerivationPath::from_str(BIP86_DERIVATION_PATH).unwrap();
		let tprv = self.root_priv.unwrap().derive_priv(SECP256K1, &path).unwrap();
		let tpub = ExtendedPubKey::from_priv(SECP256K1, &tprv);
		let fingerprint = tprv.fingerprint(SECP256K1);
		let prvkey = (tprv, path.clone()).into_descriptor_key().unwrap();
		let pubkey = (tpub, path).into_descriptor_key().unwrap();

		(prvkey, pubkey, fingerprint)
	}

	pub fn get_descriptor(&self) -> Descriptor<DescriptorPublicKey> {
		self.wallet
			.public_descriptor(KeychainKind::External)
			.unwrap()
			.unwrap()
			.into_wallet_descriptor(SECP256K1, self.bitcoin_network)
			.unwrap()
			.0
	}

	pub fn get_change_descriptor(&self) -> Descriptor<DescriptorPublicKey> {
		self.wallet
			.public_descriptor(KeychainKind::Internal)
			.unwrap()
			.unwrap()
			.into_wallet_descriptor(SECP256K1, self.bitcoin_network)
			.unwrap()
			.0
	}

	pub fn get_balance(&self, bitcoin_endpoint: Option<&str>) -> Result<bdk::Balance> {
		let endpoint = match bitcoin_endpoint {
			Some(e) => e,
			None =>
				if self.bitcoin_network == Network::Testnet {
					DEFAULT_TESTNET_ENDPOINT
				} else {
					DEFAULT_BITCOIN_ENDPOINT
				},
		};
		let blockchain = ElectrumBlockchain::from(Client::new(endpoint)?);
		self.wallet.sync(&blockchain, SyncOptions::default())?;
		Ok(self.wallet.get_balance()?)
	}
}

impl fmt::Display for BitcoinUser {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
			ExtendedPubKey::from_priv(SECP256K1, &self.root_priv.unwrap())
		)?;
		writeln!(f, "    Output Descriptor	: {}", &self.get_descriptor().to_string())?;
		writeln!(f, "    Change Descriptor	: {}", &self.get_change_descriptor().to_string())?;
		writeln!(f, "    Ext Address 1	: {}", &self.wallet.get_address(AddressIndex::New).unwrap())?;
		writeln!(f, "    Ext Address 2	: {}", &self.wallet.get_address(AddressIndex::New).unwrap())?;
		writeln!(
			f,
			"    Change Address	: {}",
			&self.wallet.get_internal_address(AddressIndex::New).unwrap()
		)?;

		let balance = self.get_balance(None).unwrap();
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
		let user_constants = user_constants();
		let alice_constants = user_constants.get(&String::from("Alice")).unwrap();
		let mnemonic = Mnemonic::from_str(alice_constants.mnemonic).unwrap();
		let seed = Seed::new(mnemonic, Some(alice_constants.passphrase));
		let alice_bitcoin = BitcoinUser::new(seed, Network::Testnet).unwrap();

		assert_eq!(alice_bitcoin.root_priv.unwrap().to_string(), "tprv8ZgxMBicQKsPeFd9cajKjGekZW5wDXq2e1vpKToJvZMqjyNkMqmr7exPFUbJ92YxSkqL4w19HpuzYkVYvc4n4pvySBmJfsawS7Seb8FzuNJ".to_string());
		assert_eq!(ExtendedPubKey::from_priv(SECP256K1, &alice_bitcoin.root_priv.unwrap()).to_string(),
		"tpubD6NzVbkrYhZ4XiewWEPv8gJs8XbsNs1wDKXbbyqcLqAEaTdWzEbSJ9aFRamjrj3RQKyZ2Q848BkMxyt6J6e36Y14ga6Et7suFXk3RKFqEaA"
		.to_string());
		assert_eq!(alice_bitcoin.get_descriptor().to_string(), "tr([9b5d4149/86'/1'/0']tpubDCcGwi31GzRdgLiDanko4op3HgiGPdByResjWRS51dWDzpfZAh2VRPPKUdKSYojy1JsxMmdGHJf1eUKbVcjZvEnPHEnefLxgWm1Q6BYXs28/0/*)#jsa95v8y".to_string());

		println!("Alice {} ", alice_bitcoin);
	}

	#[test]
	fn test_key_derivation_for_single_key_p2tr_outputs() -> Result<()> {
		// Test data from:
		// https://github.com/bitcoin/bips/blob/master/bip-0086.mediawiki#test-vectors
		const MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
		const EXPECTED_ROOT_PRIV: &str = "xprv9s21ZrQH143K3GJpoapnV8SFfukcVBSfeCficPSGfubmSFDxo1kuHnLisriDvSnRRuL2Qrg5ggqHKNVpxR86QEC8w35uxmGoggxtQTPvfUu";
		const EXPECTED_ROOT_PUB: &str = "xpub661MyMwAqRbcFkPHucMnrGNzDwb6teAX1RbKQmqtEF8kK3Z7LZ59qafCjB9eCRLiTVG3uxBxgKvRgbubRhqSKXnGGb1aoaqLrpMBDrVxga8";

		let mnemonic = Mnemonic::from_str(MNEMONIC).unwrap();
		let seed = Seed::new::<String>(mnemonic, None);
		let bip86_user = BitcoinUser::new(seed, Network::Bitcoin)?;

		assert_eq!(format!("{}", bip86_user.root_priv.unwrap()), EXPECTED_ROOT_PRIV.to_string());
		assert_eq!(
			format!("{}", ExtendedPubKey::from_priv(SECP256K1, &bip86_user.root_priv.unwrap())),
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

		Ok(())
	}
}
