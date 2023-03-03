mod bitcoin_user;
pub mod constants;
mod nostr_user;
// mod hashed_user;
use anyhow::Result;
use bdk::bitcoin::Network;
use std::fmt;
use bitcoin::util::bip32::Fingerprint;
use crate::user::{bitcoin_user::BitcoinUser, nostr_user::NostrUser};

pub struct User {
	pub name: Option<String>,
	pub nostr_user: NostrUser,
	pub bitcoin_user: BitcoinUser,
	mnemonic: String,
	passphrase: Option<String>,
}

impl User {
	pub fn new(
		mnemonic: String,
		passphrase: Option<String>,
		name: Option<String>,
		bitcoin_network: &Network,
	) -> Result<User> {
		Ok(User {
			name,
			mnemonic: mnemonic.clone(),
			passphrase: passphrase.clone(),
			nostr_user: NostrUser::new(mnemonic.clone(), passphrase.clone()).unwrap(),
			bitcoin_user: BitcoinUser::new(mnemonic.clone(), passphrase.clone(), bitcoin_network)
				.unwrap(),
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
		]
	}

    pub fn from_fingerprint(f: &Fingerprint) -> String {
    
        let known_users = Self::known_users();
        let maybe_user = known_users.iter().find(|u| &u.bitcoin_user.setup_keys::<miniscript::Tap>().2 == f);

        // WTF- gotta be a better way to do this
        if maybe_user.is_some() {
            let user: &User = maybe_user.unwrap();
            let another_user = user.clone();
            let user_name = another_user.name.as_ref().unwrap();

            return format!("<known-user:{} from fingerprint {}>", user_name, f.to_string());
        }
        format!("<fingerprint:{}>", f.to_string())
    }

	pub fn alice() -> Result<User> {
		User::new(
			"carry surface crater rude auction ritual banana elder shuffle much wonder decrease"
				.to_string(),
			Some("oy+hB/qeJ1AasCCR".to_string()),
			Some("Alice".to_string()),
			&Network::Testnet,
		)
	}

	pub fn bob() -> Result<User> {
		User::new(
			"market museum car noodle cream pool enhance please level price slide process"
				.to_string(),
			Some("B3Q0YHYYHmF798Jg".to_string()),
			Some("Bob".to_string()),
			&Network::Testnet,
		)
	}

	pub fn charlie() -> Result<User> {
		User::new(
			"cry modify gallery home desert tongue immune address bunker bean tone giggle"
				.to_string(),
			Some("nTVuKiINc5TKMjfV".to_string()),
			Some("Charlie".to_string()),
			&Network::Testnet,
		)
	}

	pub fn david() -> Result<User> {
		User::new(
			"alone hospital depth worth vapor lazy burst skill apart accuse maze evidence"
				.to_string(),
			Some("f5upOqUyG0iPY4n+".to_string()),
			Some("David".to_string()),
			&Network::Testnet,
		)
	}

	pub fn erika() -> Result<User> {
		User::new(
			"confirm rifle kit warrior aware clump shallow eternal real shift puzzle wife"
				.to_string(),
			Some("JBtdXy+2ut2fxplW".to_string()),
			Some("Erika".to_string()),
			&Network::Testnet,
		)
	}

	#[allow(dead_code)]
	pub fn get(name: &String) -> Result<User> {
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
}

impl fmt::Display for User {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if self.name.is_some() {
			writeln!(f, "Name       : {}", &self.name.as_ref().unwrap())?;
		}
		writeln!(f, "\nMnemonic   : {:?} ", &self.mnemonic.to_string())?;
		writeln!(f, "Passphrase : \"{}\" ", &self.passphrase.clone().unwrap_or("".to_string()))?;

		writeln!(f, "{}", &self.nostr_user)?;
		writeln!(f, "{}", &self.bitcoin_user)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {

	use super::*;
	use crate::user::constants::user_constants;

	use bdk::{
		descriptor::{policy::*, ExtractPolicy, IntoWalletDescriptor},
		keys::{DescriptorKey, IntoDescriptorKey},
		wallet::{signer::SignersContainer},
	};
	use bitcoin::util::{bip32, bip32::Fingerprint};
	use miniscript::ScriptContext;
	use nostr::prelude::{All, Secp256k1};
	use std::{str::FromStr, sync::Arc};
    use assert_matches::assert_matches;

	#[test]
	fn test_alice() {
		let user_constants = user_constants();
		let alice_constants = user_constants.get(&String::from("Alice")).unwrap();
		let alice_user = User::new(
			alice_constants.mnemonic.to_string(),
			Some(alice_constants.passphrase.to_string()),
			Some("Alice".to_string()),
			&bitcoin::network::constants::Network::Testnet,
		)
		.unwrap();
		println!("{}", alice_user);
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
		secp: &Secp256k1<All>,
	) -> (DescriptorKey<Ctx>, DescriptorKey<Ctx>, Fingerprint) {
		let path = bip32::DerivationPath::from_str(path).unwrap();
		let tprv = tprv.derive_priv(secp, &path).unwrap();
		let tpub = bip32::ExtendedPubKey::from_priv(secp, &tprv);
		let fingerprint = tprv.fingerprint(secp);
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
		let secp = Secp256k1::new();

		let (alice_prv, _, alice_fing) =
			setup_keys(&alice.bitcoin_user.root_priv.unwrap(), ALICE_BOB_PATH, &secp);
		let (_, bob_pub, bob_fing) =
			setup_keys(&bob.bitcoin_user.root_priv.unwrap(), ALICE_BOB_PATH, &secp);

		let desc = bdk::descriptor!(tr(bob_pub, pk(alice_prv))).unwrap();
		let (wallet_desc, keymap) = desc.into_wallet_descriptor(&secp, Network::Testnet).unwrap();
		let signers_container = Arc::new(SignersContainer::build(keymap, &wallet_desc, &secp));
        println!("Script descriptor : {:#?} ", &wallet_desc.to_string());

		let policy = wallet_desc
			.extract_policy(&signers_container, BuildSatisfaction::None, &secp)
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
	// 	let secp = Secp256k1::new();

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
