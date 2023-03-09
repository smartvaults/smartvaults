use std::fmt;

use keechain_core::{
	bitcoin::{util::bip32::Fingerprint, Network},
	types::Seed,
};
use nostr_sdk::Result;

mod bitcoin_user;
pub mod constants;
mod nostr_user;

use crate::user::{bitcoin_user::BitcoinUser, nostr_user::NostrUser};

pub struct User {
	pub name: Option<String>,
	pub nostr_user: NostrUser,
	pub bitcoin_user: BitcoinUser,
	seed: Seed,
}

impl User {
	pub fn new(seed: Seed, name: Option<String>, bitcoin_network: Network) -> Result<Self> {
		Ok(Self {
			name,
			nostr_user: NostrUser::new(seed.clone()).unwrap(),
			bitcoin_user: BitcoinUser::new(seed.clone(), bitcoin_network).unwrap(),
			seed,
		})
	}

	#[allow(dead_code)]
	pub fn known_users() -> Vec<User> {
		vec![
			User::alice().unwrap(),
			User::bob().unwrap(),
			User::charlie().unwrap(),
			User::david().unwrap(),
			User::erika().unwrap(),
			constants::get_known_user(constants::SARAH).unwrap(),
			constants::get_known_user(constants::JOHN).unwrap(),
			constants::get_known_user(constants::MARIA).unwrap(),
			constants::get_known_user(constants::LEE).unwrap(),
			constants::get_known_user(constants::RACHEL).unwrap(),
			constants::get_known_user(constants::JAMES).unwrap(),
			constants::get_known_user(constants::KAREN).unwrap(),
			constants::get_known_user(constants::MARK).unwrap(),
			constants::get_known_user(constants::AMANDA).unwrap(),
		]
	}

	pub fn from_fingerprint(f: &Fingerprint) -> String {
		let known_users = Self::known_users();
		let maybe_user = known_users
			.iter()
			.find(|u| &u.bitcoin_user.setup_keys::<bdk::miniscript::Tap>().2 == f);

		if let Some(user) = maybe_user {
			let user_name = user.name.as_ref().unwrap();
			return format!("<known-user:{user_name} from fingerprint {f}>");
		}
		format!("<fingerprint:{f}>")
	}

	pub fn alice() -> Result<User> {
		constants::get_known_user(constants::ALICE)

		// let mnemonic = Mnemonic::from_str(constants::ALICE.1)?;
		// let seed = Seed::new(mnemonic, Some(constants::ALICE.2));
		// User::new(seed, Some(constants::ALICE.0.to_string()), Network::Testnet)
	}

	pub fn bob() -> Result<User> {
		constants::get_known_user(constants::BOB)
	}

	pub fn charlie() -> Result<User> {
		constants::get_known_user(constants::CHARLIE)
	}

	pub fn david() -> Result<User> {
		constants::get_known_user(constants::DAVID)
	}

	pub fn erika() -> Result<User> {
		constants::get_known_user(constants::ERIKA)
	
	}

	#[allow(dead_code)]
	pub fn get(name: &str) -> Result<User> {
		match name {
			"alice" => User::alice(),
			"bob" => User::bob(),
			"charlie" => User::charlie(),
			"david" => User::david(),
			"erika" => User::erika(),
			"sarah" => constants::get_known_user(constants::SARAH),
			"john" => constants::get_known_user(constants::JOHN),
			"maria" => constants::get_known_user(constants::MARIA),
			"lee" => constants::get_known_user(constants::LEE),
			"rachel" => constants::get_known_user(constants::RACHEL),
			"james" => constants::get_known_user(constants::JAMES),
			"karen" => constants::get_known_user(constants::KAREN),
			"mark" => constants::get_known_user(constants::MARK),
			"amanda" => constants::get_known_user(constants::AMANDA),
			_ => User::alice(), // todo: should raise an error if not found rather than return alice
		}
	}
}

impl fmt::Display for User {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if let Some(name) = self.name.as_ref() {
			writeln!(f, "Name       : {name}")?;
		}
		writeln!(f, "\nMnemonic   : {} ", &self.seed.mnemonic())?;
		writeln!(f, "Passphrase : \"{}\" ", &self.seed.passphrase().unwrap_or_default())?;

		writeln!(f, "{}", &self.nostr_user)?;
		writeln!(f, "{}", &self.bitcoin_user)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {

	use super::*;
	use crate::user::constants::user_constants;

	use assert_matches::assert_matches;
	use bdk::{
		descriptor::{policy::*, ExtractPolicy, IntoWalletDescriptor},
		keys::{DescriptorKey, IntoDescriptorKey},
		miniscript::ScriptContext,
		wallet::signer::SignersContainer,
	};
	use keechain_core::bitcoin::util::bip32::{self, Fingerprint};
	use keechain_core::bip39::Mnemonic;
	use nostr_sdk::{bitcoin::Network, SECP256K1};
	use std::{str::FromStr, sync::Arc};

	#[test]
	fn test_alice() {
		let user_constants = user_constants();
		let alice_constants = user_constants.get(&String::from("Alice")).unwrap();
		let mnemonic = Mnemonic::from_str(alice_constants.mnemonic).unwrap();
		let seed = Seed::new(mnemonic, Some(alice_constants.passphrase));
		let alice_user = User::new(seed, Some("Alice".to_string()), Network::Testnet).unwrap();
		println!("{}", alice_user);
	}

	#[test]
	fn test_sarah() {
		let sarah = User::get("sarah").unwrap();
		println!("{}", sarah);
	}

	#[test]
	fn dump_known_users() {
		for user in User::known_users() {
			println!("{}", user);
		}
	}

	const ALICE_BOB_PATH: &str = "m/0'";
	fn setup_keys<Ctx: ScriptContext>(
		tprv: &bip32::ExtendedPrivKey,
		path: &str,
	) -> (DescriptorKey<Ctx>, DescriptorKey<Ctx>, Fingerprint) {
		let path = bip32::DerivationPath::from_str(path).unwrap();
		let tprv = tprv.derive_priv(SECP256K1, &path).unwrap();
		let tpub = bip32::ExtendedPubKey::from_priv(SECP256K1, &tprv);
		let fingerprint = tprv.fingerprint(SECP256K1);
		let prvkey = (tprv, path.clone()).into_descriptor_key().unwrap();
		let pubkey = (tpub, path).into_descriptor_key().unwrap();

		(prvkey, pubkey, fingerprint)
	}

	fn display_key(key: &PkOrF) -> String {
		// TODO: Use aliases
		match key {
			PkOrF::Pubkey(pk) => format!("<pk:{}>", pk.to_string()),
			PkOrF::XOnlyPubkey(pk) => format!("<xonly-pk:{}>", pk.to_string()),
			PkOrF::Fingerprint(f) => format!("<fingerprint:{}>", f.to_string()),
		}
	}

	fn description(item: &SatisfiableItem) -> String {
		match &item {
			SatisfiableItem::EcdsaSignature(key) => format!("ECDSA Sig of {}", display_key(key)),
			SatisfiableItem::SchnorrSignature(key) => {
				format!("Schnorr Sig of {}", display_key(key))
			},
			SatisfiableItem::Sha256Preimage { hash } => {
				format!("SHA256 Preimage of {}", hash.to_string())
			},
			SatisfiableItem::Hash256Preimage { hash } => {
				format!("Double-SHA256 Preimage of {}", hash.to_string())
			},
			SatisfiableItem::Ripemd160Preimage { hash } => {
				format!("RIPEMD160 Preimage of {}", hash.to_string())
			},
			SatisfiableItem::Hash160Preimage { hash } => {
				format!("Double-RIPEMD160 Preimage of {}", hash.to_string())
			},
			SatisfiableItem::AbsoluteTimelock { value } => {
				format!("Absolute Timelock of {}", value.to_string())
			},
			SatisfiableItem::RelativeTimelock { value } => {
				format!("Relative Timelock of {}", value.to_string())
			},
			SatisfiableItem::Multisig { keys, threshold } => {
				format!("{} of {} MultiSig:", threshold, keys.len())
			},
			SatisfiableItem::Thresh { items, threshold } => {
				format!("{} of {} Threshold:", threshold, items.len())
			},
		}
	}

	// from: https://github.com/bitcoindevkit/bdk/blob/master/src/descriptor/policy.rs#L1749
	#[test]
	fn test_extract_tr_script_spend() {
		let alice = User::get(&"alice".to_string()).unwrap();
		let bob = User::get(&"bob".to_string()).unwrap();

		let (alice_prv, _, alice_fing) =
			setup_keys(&alice.bitcoin_user.root_priv.unwrap(), ALICE_BOB_PATH);
		let (_, bob_pub, bob_fing) =
			setup_keys(&bob.bitcoin_user.root_priv.unwrap(), ALICE_BOB_PATH);

		let desc = bdk::descriptor!(tr(bob_pub, pk(alice_prv))).unwrap();
		let (wallet_desc, keymap) =
			desc.into_wallet_descriptor(SECP256K1, Network::Testnet).unwrap();
		let signers_container = Arc::new(SignersContainer::build(keymap, &wallet_desc, SECP256K1));
		println!("Script descriptor : {:#?} ", &wallet_desc.to_string());

		let policy = wallet_desc
			.extract_policy(&signers_container, BuildSatisfaction::None, SECP256K1)
			.unwrap()
			.unwrap();

		println!("Policy    : {:?}", policy);
		assert_matches!(policy.item, SatisfiableItem::Thresh { ref items, threshold: 1 } if	items.len() == 2);
		assert_matches!(policy.contribution, Satisfaction::PartialComplete {n: 2, m: 1, items, .. } if items == vec![1]);

		let alice_sig = SatisfiableItem::SchnorrSignature(PkOrF::Fingerprint(alice_fing));
		let bob_sig = SatisfiableItem::SchnorrSignature(PkOrF::Fingerprint(bob_fing));
		println!("alice_sig     : {}", description(&alice_sig));
		println!("bob_sig       : {}", description(&bob_sig));

		println!("Description: {}", description(&policy.item));

		let thresh_items = match policy.item {
			SatisfiableItem::Thresh { items, .. } => items,
			_ => unreachable!(),
		};

		assert_eq!(thresh_items[0].item, bob_sig);
		assert_eq!(thresh_items[1].item, alice_sig);
	}

	// #[test]
	// fn test_multisig_2() {
	// 	let alice = User::get(&"alice".to_string()).unwrap();
	// 	let bob = User::get(&"bob".to_string()).unwrap();
	//

	// 	let (alice_prv, _, alice_fing) =
	// 		setup_keys(&alice.extended_private_key.unwrap(), ALICE_BOB_PATH, &secp);
	// 	let (_, bob_pub, bob_fing) =
	// 		setup_keys(&bob.extended_private_key.unwrap(), ALICE_BOB_PATH, &secp);

	// 	let desc = descriptor!(tr(bob_pub, pk(alice_prv))).unwrap();
	// 	let (wallet_desc, keymap) = desc.into_wallet_descriptor(&secp, Network::Testnet).unwrap();
	// 	let signers_container = Arc::new(SignersContainer::build(keymap, &wallet_desc, &secp));

	// 	let policy = wallet_desc
	// 		.extract_policy(&signers_container, BuildSatisfaction::None, &secp)
	// 		.unwrap()
	// 		.unwrap();

	// 	println!("Policy    : {:?}", policy);
	// 	// assert_matches!(policy.item, SatisfiableItem::Thresh { ref items, threshold: 1 } if
	// 	// items.len() == 2); assert_matches!(policy.contribution, Satisfaction::PartialComplete {
	// 	// n: 2, m: 1, items, .. } if items == vec![1]);

	// 	let alice_sig = SatisfiableItem::SchnorrSignature(PkOrF::Fingerprint(alice_fing));
	// 	let bob_sig = SatisfiableItem::SchnorrSignature(PkOrF::Fingerprint(bob_fing));
	// 	println!("alice_sig     : {}", description(&alice_sig));
	// 	println!("bob_sig       : {}", description(&bob_sig));

	// 	println!("Description: {}", description(&policy.item));

	// 	let thresh_items = match policy.item {
	// 		SatisfiableItem::Thresh { items, .. } => items,
	// 		_ => unreachable!(),
	// 	};

	// 	assert_eq!(thresh_items[0].item, bob_sig);
	// 	assert_eq!(thresh_items[1].item, alice_sig);
	// }
}
