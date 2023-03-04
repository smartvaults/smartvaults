#![allow(unused, dead_code)]

use std::fmt;
use std::str::FromStr;

use bdk::blockchain::{Blockchain, ElectrumBlockchain};
use bdk::electrum_client::Client;
use bdk::miniscript::descriptor::Descriptor;
use bdk::miniscript::policy::Concrete;
use bdk::{
	database::MemoryDatabase,
	descriptor::{
		policy::{Policy, *},
		IntoWalletDescriptor,
	},
	wallet::{SyncOptions, Wallet},
	KeychainKind,
};
use nostr_sdk::prelude::*;
use num_format::{Locale, ToFormattedString};
use owo_colors::{
	colors::{
		css::Lime,
		xterm,
		xterm::{DarkTundora, MineShaft, Pistachio, ScorpionGray, UserBrightWhite},
		BrightCyan,
	},
	OwoColorize,
};
use termtree::Tree;

use crate::user::User;
use crate::{DEFAULT_BITCOIN_ENDPOINT, DEFAULT_TESTNET_ENDPOINT};
pub struct CoinstrPolicy {
	pub name: String,
	pub description: String,
	pub wallet: Wallet<MemoryDatabase>,
	// pub descriptor: Descriptor<String>,
	pub policy: Policy,
}

impl CoinstrPolicy {
	pub fn from_descriptor(
		name: String,
		description: String,
		descriptor: String,
	) -> Result<CoinstrPolicy> {
		let (wallet_desc, _keymap) =
			descriptor.into_wallet_descriptor(SECP256K1, Network::Testnet)?;
		let database = MemoryDatabase::new();

		// Create a new wallet from this descriptor
		let wallet = Wallet::new(&format!("{}", wallet_desc), None, Network::Testnet, database)?;

		// BDK also has it's own `Policy` structure to represent the spending condition in a more
		// human readable json format.
		let spending_policy = wallet.policies(KeychainKind::External)?;

		Ok(CoinstrPolicy {
			name,
			description,
			// descriptor: "".to_string(), //wallet_desc.into_wallet_descriptor(),
			policy: spending_policy.unwrap(),
			wallet,
		})
	}

	pub fn from_policy_str(
		name: String,
		description: String,
		policy_str: String,
	) -> Result<CoinstrPolicy> {
		// Parse the string as a [`Concrete`] type miniscript policy.
		let policy = Concrete::<String>::from_str(policy_str.as_str())?;

		// Create a `wsh` type descriptor from the policy.
		// `policy.compile()` returns the resulting miniscript from the policy.
		let descriptor = Descriptor::new_wsh(policy.compile()?)?;
		let database = MemoryDatabase::new();

		// Create a new wallet from this descriptor
		let wallet = Wallet::new(&format!("{}", descriptor), None, Network::Testnet, database)?;

		// BDK also has it's own `Policy` structure to represent the spending condition in a more
		// human readable json format.
		let spending_policy = wallet.policies(KeychainKind::External)?;

		Ok(CoinstrPolicy {
			name,
			description,
			// descriptor:	descriptor,
			policy: spending_policy.unwrap(),
			wallet,
		})
	}

	pub fn new_one_of_two_taptree(
		name: String,
		description: String,
		signer: &User,
		other_signer: &User,
	) -> Result<CoinstrPolicy> {
		let signer_wif = signer.bitcoin_user.private_key.to_wif();
		let other_signer_pub =
			other_signer.bitcoin_user.private_key.public_key(SECP256K1).to_string();

		let policy_str = format!("or(pk({}),pk({}))", signer_wif, other_signer_pub);
		// println!("Policy string	<new_one_of_two_taptree>	: {}", &policy_str);

		let pol: Concrete<String> = Concrete::from_str(&policy_str).unwrap();
		// In case we can't find an internal key for the given policy, we set the internal key to
		// a random pubkey as specified by BIP341 (which are *unspendable* by any party :p)
		let desc = pol.compile_tr(Some("UNSPENDABLE_KEY".to_string())).unwrap();
		// println!("Descriptor    : {}", desc.to_string());

		let database = MemoryDatabase::new();
		let wallet = Wallet::new(&format!("{}", desc), None, Network::Testnet, database).unwrap();

		let spending_policy = wallet.policies(KeychainKind::External)?;

		Ok(CoinstrPolicy {
			name,
			description,
			// descriptor:	desc,
			policy: spending_policy.unwrap(),
			wallet,
		})

		// Self::from_descriptor(name, description, desc.to_string())
	}

	pub fn new_one_of_two(
		name: String,
		description: String,
		signer: &User,
		other_signer: &User,
	) -> Result<CoinstrPolicy> {
		let signer_wif = signer.bitcoin_user.private_key.to_wif();
		let other_signer_pub =
			other_signer.bitcoin_user.private_key.public_key(SECP256K1).to_string();

		let policy_str = format!("thresh(1,pk({}),pk({}))", signer_wif, other_signer_pub);

		Self::from_policy_str(name, description, policy_str)
	}

	pub fn get_balance(
		&self,
		bitcoin_network: Network,
		bitcoin_endpoint: Option<&str>,
	) -> Result<bdk::Balance> {
		let endpoint = match bitcoin_endpoint {
			Some(e) => e,
			None => {
				if bitcoin_network == Network::Testnet {
					DEFAULT_TESTNET_ENDPOINT
				} else {
					DEFAULT_BITCOIN_ENDPOINT
				}
			},
		};
		let blockchain = ElectrumBlockchain::from(Client::new(endpoint)?);
		self.wallet.sync(&blockchain, SyncOptions::default())?;
		Ok(self.wallet.get_balance()?)
	}
}

fn display_key(key: &PkOrF) -> String {
	// TODO: Use aliases
	match key {
		PkOrF::Pubkey(pk) => format!("<pk:{}>", pk.to_string().fg::<MineShaft>()),
		PkOrF::XOnlyPubkey(pk) => format!("<xonly-pk:{pk}>"),
		PkOrF::Fingerprint(f) => User::from_fingerprint(f),
	}
}

fn add_node(item: &SatisfiableItem) -> Tree<String> {
	let mut si_tree: Tree<String> =
		Tree::new(format!("{}{}", "id -> ".fg::<MineShaft>(), item.id().fg::<MineShaft>()));

	match &item {
		SatisfiableItem::EcdsaSignature(key) => {
			si_tree.push(format!("🗝️ {} {}", "ECDSA Sig of ".fg::<Pistachio>(), display_key(key)));
		},
		SatisfiableItem::SchnorrSignature(key) => {
			si_tree.push(format!(
				"🔑 {} {}",
				"Schnorr Sig of ".fg::<Pistachio>(),
				display_key(key)
			));
		},
		SatisfiableItem::Sha256Preimage { hash } => {
			si_tree.push(format!("SHA256 Preimage of {hash}"));
		},
		SatisfiableItem::Hash256Preimage { hash } => {
			si_tree.push(format!("Double-SHA256 Preimage of {hash}"));
		},
		SatisfiableItem::Ripemd160Preimage { hash } => {
			si_tree.push(format!("RIPEMD160 Preimage of {hash}"));
		},
		SatisfiableItem::Hash160Preimage { hash } => {
			si_tree.push(format!("Double-RIPEMD160 Preimage of {hash}"));
		},
		SatisfiableItem::AbsoluteTimelock { value } => {
			si_tree.push(format!("⏰ {} {value}", "Absolute Timelock of ".fg::<Lime>()));
		},
		SatisfiableItem::RelativeTimelock { value } => {
			si_tree.push(format!("⏳ {} {value}", "Relative Timelock of".fg::<Lime>(),));
		},
		SatisfiableItem::Multisig { keys, threshold } => {
			// si_tree.push(format!("🎚️ {} of {} MultiSig:", threshold, keys.len()));
			let mut child_tree: Tree<String> = Tree::new(format!(
				"🤝 {}{} of {}",
				"MultiSig  :  ".fg::<BrightCyan>(),
				threshold,
				keys.len()
			));

			keys.iter().for_each(|x| {
				child_tree.push(display_key(x));
			});
			si_tree.push(child_tree);
		},
		SatisfiableItem::Thresh { items, threshold } => {
			let mut child_tree: Tree<String> = Tree::new(format!(
				"👑{}{} of {} ",
				" Threshold Condition   : ".fg::<BrightCyan>(),
				threshold,
				items.len()
			));

			items.iter().for_each(|x| {
				child_tree.push(add_node(&x.item));
			});
			si_tree.push(child_tree);
		},
	}
	si_tree
}

impl fmt::Display for CoinstrPolicy {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "{}", "\nCoinstr Policy".fg::<UserBrightWhite>().underline())?;
		writeln!(f, "Name           : {}", &self.name)?;
		writeln!(f, "Description    : {}", &self.description)?;

		// TODO: fix because fails on None
		// writeln!(f, "Descriptor     : {}", &self.descriptor.as_ref().unwrap())?;
		writeln!(f)?;

		let mut tree: Tree<String> = Tree::new(self.name.clone());
		tree.push(add_node(&self.policy.item));
		writeln!(f, "{}", tree)?;

		let balance = self.get_balance(Network::Testnet, None).unwrap();
		writeln!(f, "{}", "\nBitcoin Balances (sats)".fg::<UserBrightWhite>().underline())?;
		writeln!(f, "  Immature            	: {} ", balance.immature)?;
		writeln!(f, "  Trusted Pending     	: {} ", balance.trusted_pending)?;
		writeln!(f, "  Untrusted Pending   	: {} ", balance.untrusted_pending)?;
		writeln!(
			f,
			"  Confirmed           	: {} ",
			balance.confirmed.to_formatted_string(&Locale::en)
		)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {

	use std::hash::Hasher;

	use super::*;
	// use crate::user::User;
	use bdk::wallet::{tx_builder::TxOrdering, AddressIndex::New};

	#[test]
	fn build_multisig_policy() {
		let alice = User::get(&"alice".to_string()).unwrap();
		let bob = User::get(&"bob".to_string()).unwrap();

		let policy = CoinstrPolicy::new_one_of_two(
			"💸 My testing policy".to_string(),
			"A policy for testing Alice and Bob multisig".to_string(),
			&alice,
			&bob,
		);
		println!("{}", &policy.as_ref().unwrap());

		let receiving_address = &policy.unwrap().wallet.get_address(New).unwrap();
		println!("{}", receiving_address);
	}

	#[test]
    #[rustfmt::skip]
	fn test_with_taptree() {
		let alice = User::get(&"alice".to_string()).unwrap();
		let bob = User::get(&"bob".to_string()).unwrap();

		let policy = CoinstrPolicy::new_one_of_two_taptree(
			"💸 My TapTree policy".to_string(),
			"A 1 of 2 Taptree policy".to_string(),
			&alice,
			&bob,
		);
		println!("{}", &policy.as_ref().unwrap());

        let receiving_address = &policy.unwrap().wallet.get_address(New).unwrap();
		println!("{}", receiving_address);
	}

	use bdk::SignOptions;
	#[test]
    #[rustfmt::skip]
	fn test_tx_builder_on_policy() {
		let alice = User::get(&"alice".to_string()).unwrap();
		let bob = User::get(&"bob".to_string()).unwrap();

		let secp = Secp256k1::new();

		let alice_address = alice.bitcoin_user.wallet.get_address(New).unwrap();
		println!("Alice address	: {}", alice_address);

		let mut policy = CoinstrPolicy::new_one_of_two_taptree(
			"💸 My TapTree policy".to_string(),
			"A 1 of 2 Taptree policy".to_string(),
			&alice,
			&bob,
		);
		println!("{}", &policy.as_ref().unwrap());

		println!("Syncing policy wallet.");
		let blockchain = ElectrumBlockchain::from(Client::new(DEFAULT_TESTNET_ENDPOINT).unwrap());
		policy.as_ref().unwrap().wallet.sync(&blockchain, SyncOptions::default()).unwrap();

		let balance = policy.as_ref().unwrap().wallet.get_balance().unwrap();
		println!("Wallet balances in SATs: {}", balance);

		const TEST_NUM_SATS: u64 = 500;
		if balance.confirmed < TEST_NUM_SATS {
			let receiving_address = &policy.unwrap().wallet.get_address(New).unwrap();
			println!("Refill this testnet wallet from the faucet: 	https://bitcoinfaucet.uo1.net/?to={receiving_address}");
			return;
		}

		let (mut psbt, tx_details) = {
			let mut builder = policy.as_ref().unwrap().wallet.build_tx();
			builder.add_recipient(alice_address.script_pubkey(), 500);
			builder.finish().unwrap()
		};

		println!("\nNumber of signers in policy wallet   {}", policy.as_ref().unwrap().wallet.get_signers(bdk::KeychainKind::External).signers().len());
		println!("\nUnsigned PSBT: \n{}", psbt);

		let finalized = policy.as_ref().unwrap().wallet.sign(&mut psbt, SignOptions::default()).unwrap();
		println!("\nSigned the PSBT: \n{}\n", psbt);

		assert!(finalized, "The PSBT was not finalized!");
        println!("The PSBT has been signed and finalized.");

		let raw_transaction = psbt.extract_tx();
		let txid = raw_transaction.txid();
	
		blockchain.broadcast(&raw_transaction);
		println!("Transaction broadcast! TXID: {txid}.\nExplorer URL: https://mempool.space/testnet/tx/{txid}", txid = txid);
	}

	// @todo: FIX ME - miniscript fails with duplicated pubkeys in the descriptor
	// #[test]
	// #[rustfmt::skip]
	// fn build_with_descriptor() {
	// 	let policy = CoinstrPolicy::from_descriptor(
	// 		"💸 My testing policy".to_string(),
	// 		"A policy with an ECDSA sig and threshold with Relative Timelock".to_string(),
	//         "wsh(multi(2,tpubD6NzVbkrYhZ4XHndKkuB8FifXm8r5FQHwrN6oZuWCz13qb93rtgKvD4PQsqC4HP4yhV3tA2fqr2RbY5mNXfM7RxXUoeABoDtsFUq2zJq6YK/1/*,tpubD6NzVbkrYhZ4XHndKkuB8FifXm8r5FQHwrN6oZuWCz13qb93rtgKvD4PQsqC4HP4yhV3tA2fqr2RbY5mNXfM7RxXUoeABoDtsFUq2zJq6YK/1/*))#7ke34793".to_string()
	// 	);
	// 	println!("{}", &policy.as_ref().unwrap());

	// 	let receiving_address = &policy.unwrap().wallet.get_address(New).unwrap();
	// 	println!("{}", receiving_address);
	// }

	#[test]
    #[rustfmt::skip]
	fn build_with_complex_policy_str() {
		let policy = CoinstrPolicy::from_policy_str(
			"💸 Complex policy".to_string(),
			"Nested thresholds and multisig with relative timelock".to_string(),
			"or(10@thresh(4,pk(029ffbe722b147f3035c87cb1c60b9a5947dd49c774cc31e94773478711a929ac0),pk(025f05815e3a1a8a83bfbb03ce016c9a2ee31066b98f567f6227df1d76ec4bd143),pk(025625f41e4a065efc06d5019cbbd56fe8c07595af1231e7cbc03fafb87ebb71ec),pk(02a27c8b850a00f67da3499b60562673dcf5fdfb82b7e17652a7ac54416812aefd),pk(03e618ec5f384d6e19ca9ebdb8e2119e5bef978285076828ce054e55c4daf473e2)),1@and(older(4209713),thresh(2,pk(03deae92101c790b12653231439f27b8897264125ecb2f46f48278603102573165),pk(033841045a531e1adf9910a6ec279589a90b3b8a904ee64ffd692bd08a8996c1aa),pk(02aebf2d10b040eb936a6f02f44ee82f8b34f5c1ccb20ff3949c2b28206b7c1068))))".to_string(),
		);
		println!("{}", &policy.as_ref().unwrap());

        let receiving_address = &policy.unwrap().wallet.get_address(New).unwrap();
		println!("{}", receiving_address);
	}

	use bdk::miniscript::{descriptor::Tr, Miniscript, Tap};

	#[test]
    #[rustfmt::skip]
	fn test_taptree_from_miniscript_tests() {

		let alice = User::get(&"alice".to_string()).unwrap();
		let bob = User::get(&"bob".to_string()).unwrap();
		let secp = Secp256k1::new();

		let alice_pub = alice.bitcoin_user.private_key.public_key(&secp).to_string();
		let bob_pub = bob.bitcoin_user.private_key.public_key(&secp).to_string();

		let pol_str = format!("or(pk({}),pk({}))", alice_pub, bob_pub);

		let policy = CoinstrPolicy::from_policy_str(
			"💸 Taproot Policy".to_string(),
			"1 of 2 taproot policy".to_string(),
			pol_str,
		);
		println!("{}", &policy.as_ref().unwrap());

        let receiving_address = &policy.unwrap().wallet.get_address(New).unwrap();
		println!("{}", receiving_address);
	}

	// or(pk(cPuK7a4dmU1eF5ZkiF22ABBWWxeaQyXND2oanNc58VMb2ZzJsee5),
	// and(pk(028bcac3f94577994ce9e2663441d183b765a6584f4b608a54d483e14b485611df),after(432)))

	#[test]
    #[rustfmt::skip]
	fn test_taptree_from_elephant_example() {

		let alice = User::get(&"alice".to_string()).unwrap();
		let bob = User::get(&"bob".to_string()).unwrap();
		let secp = Secp256k1::new();

		let alice_wif = alice.bitcoin_user.private_key.to_wif().to_string();
		let bob_pub = bob.bitcoin_user.private_key.public_key(&secp).to_string();

		let pol_str = format!("or(pk({}),and(pk({}),after(6)))", alice_wif, bob_pub);

		let policy = CoinstrPolicy::from_policy_str(
			"💸 Policy with two signers and a weight".to_string(),
			"1 signature or 2nd signature plus 6 block wait".to_string(),
			pol_str,
		);
		println!("{}", &policy.as_ref().unwrap());

        let receiving_address = &policy.unwrap().wallet.get_address(New).unwrap();
		println!("{}", receiving_address);
	}

	// FAILING - need to update the Liana policy string; miniscript fails on repeated pubkeys
	// #[test]
	// #[rustfmt::skip]
	// fn build_with_liana_descriptor() {
	// 	let policy = CoinstrPolicy::from_descriptor(
	// 		"💸 Policy from Liana".to_string(),
	// 		"2 of 2 with a time lock from Liana".to_string(),
	// 		"wsh(or_d(multi(2,[edbae63f/48'/1'/0'/2']tpubDFPMc78w6HNq3sQHucnvaXFvV4bog3PY9Z6BnvLEW2zgw1mx1Hjtgok9ZJAg4CkyzHh9GzhFZ1HEEUXPfL2G8sxh5MSgX1KZf4mgWyyzrn7/1/*,[edbae63f/48'/1'/1'/2']tpubDEm8zCbdTzY3sgMKs4aWHft5f3rL4XuiqKEpeWKo3MEm3nzj5vyxeFMPK2cK4nZM8wK9quscmXyKnSmZh7YWP5aYGSNuiyQ4YczrNqNuBst/1/*),and_v(v:pkh([edbae63f/48'/1'/2'/2']tpubDEpvmURAxnX64rppaThzE99GAfiABkJP3MvoGoFFwexyyt18prYqVFJrDFZSFMdexUo6RhEwezrWQQMVzdi5EcAZVoxYyfhbrqM2VgTn5jV/1/*),older(6))))".to_string(),
	// 	);
	// 	println!("{}", &policy.as_ref().unwrap());

	//     let receiving_address = &policy.unwrap().wallet.get_address(New).unwrap();
	// 	println!("{}", receiving_address);
	// }
}
