
#![allow(unused, dead_code)]
use crate::user::User;
use anyhow::Result;
use bdk::{
	bitcoin::Network,
	blockchain::EsploraBlockchain,
	database::MemoryDatabase,
	descriptor::{
		policy::{Policy, *},
		IntoWalletDescriptor,
	},
	wallet::{SyncOptions, Wallet},
	KeychainKind,
};
use miniscript::{descriptor::Descriptor, policy::Concrete};
use nostr::prelude::Secp256k1;
use std::{fmt, str::FromStr};
use termtree::Tree;

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
		let secp = bitcoin::secp256k1::Secp256k1::new();

		let (wallet_desc, _keymap) = descriptor.into_wallet_descriptor(&secp, Network::Testnet)?;
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

	pub fn new_one_of_two(
		name: String,
		description: String,
		signer: &User,
		other_signer: &User,
	) -> Result<CoinstrPolicy> {
		let secp = Secp256k1::new();
		let signer_wif = signer.bitcoin_user.private_key.to_wif();
		let other_signer_pub = other_signer.bitcoin_user.private_key.public_key(&secp).to_string();

		let policy_str = format!("thresh(1,pk({}),pk({}))", signer_wif, other_signer_pub);

		Self::from_policy_str(name, description, policy_str)
	}

	pub fn get_balance(
		&self,
		bitcoin_network: &Network,
		mut bitcoin_endpoint: Option<&str>,
	) -> bdk::Balance {
		const DEFAULT_TESTNET_ENDPOINT: &str = "https://blockstream.info/testnet/api";
		const DEFAULT_BITCOIN_ENDPOINT: &str = "https://blockstream.info/api";
		if bitcoin_endpoint.is_none() {
			if bitcoin_network == &bitcoin::network::constants::Network::Testnet {
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

fn display_key(key: &PkOrF) -> String {
	// TODO: Use aliases
	match key {
		PkOrF::Pubkey(pk) => format!("<pk:{}>", pk.to_string()),
		PkOrF::XOnlyPubkey(pk) => format!("<xonly-pk:{}>", pk.to_string()),
		PkOrF::Fingerprint(f) => User::from_fingerprint(f),
	}
}

fn add_node(item: &SatisfiableItem) -> Tree<String> {
	let mut si_tree: Tree<String> = Tree::new(format!("id -> {}", item.id()));

	match &item {
		SatisfiableItem::EcdsaSignature(key) => {
			si_tree.push(format!("✍️ ECDSA Sig of {}", display_key(key)));
		},
		SatisfiableItem::SchnorrSignature(key) => {
			si_tree.push(format!("🔑 Schnorr Sig of {}", display_key(key)));
		},
		SatisfiableItem::Sha256Preimage { hash } => {
			si_tree.push(format!("SHA256 Preimage of {}", hash.to_string()));
		},
		SatisfiableItem::Hash256Preimage { hash } => {
			si_tree.push(format!("Double-SHA256 Preimage of {}", hash.to_string()));
		},
		SatisfiableItem::Ripemd160Preimage { hash } => {
			si_tree.push(format!("RIPEMD160 Preimage of {}", hash.to_string()));
		},
		SatisfiableItem::Hash160Preimage { hash } => {
			si_tree.push(format!("Double-RIPEMD160 Preimage of {}", hash.to_string()));
		},
		SatisfiableItem::AbsoluteTimelock { value } => {
			si_tree.push(format!("⏰ Absolute Timelock of {}", value.to_string()));
		},
		SatisfiableItem::RelativeTimelock { value } => {
			si_tree.push(format!("⏳ Relative Timelock of {}", value.to_string()));
		},
		SatisfiableItem::Multisig { keys, threshold } => {
			// si_tree.push(format!("🎚️ {} of {} MultiSig:", threshold, keys.len()));
			let mut child_tree: Tree<String> =
				Tree::new(format!("🎚️ {} of {} MultiSig:", threshold, keys.len()));

			keys.iter().for_each(|x| {
				child_tree.push(display_key(x));
			});
			si_tree.push(child_tree);
		},
		SatisfiableItem::Thresh { items, threshold } => {
			let mut child_tree: Tree<String> =
				Tree::new(format!("🎚️ Threshold Condition    : {} of {} ", threshold, items.len()));

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
		writeln!(f, "\nCoinstr Policy")?;
		writeln!(f, "Name           : {}", &self.name)?;
		writeln!(f, "Description    : {}", &self.description)?;

		// TODO: fix because fails on None
		// writeln!(f, "Descriptor     : {}", &self.descriptor.as_ref().unwrap())?;
		writeln!(f)?;

		let mut tree: Tree<String> = Tree::new(self.name.clone());
		tree.push(add_node(&self.policy.item));
		writeln!(f, "{}", tree)?;

		let balance = self.get_balance(&bitcoin::network::constants::Network::Testnet, None);
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
	// use crate::user::User;
	use bdk::wallet::AddressIndex::New;

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
	fn build_with_descriptor() {
		let policy = CoinstrPolicy::from_descriptor(
			"💸 My testing policy".to_string(),
			"A policy with an ECDSA sig and threshold with Relative Timelock".to_string(),
            "wsh(multi(2,tpubD6NzVbkrYhZ4XHndKkuB8FifXm8r5FQHwrN6oZuWCz13qb93rtgKvD4PQsqC4HP4yhV3tA2fqr2RbY5mNXfM7RxXUoeABoDtsFUq2zJq6YK/1/*,tpubD6NzVbkrYhZ4XHndKkuB8FifXm8r5FQHwrN6oZuWCz13qb93rtgKvD4PQsqC4HP4yhV3tA2fqr2RbY5mNXfM7RxXUoeABoDtsFUq2zJq6YK/1/*))#7ke34793".to_string()
		);
		println!("{}", &policy.as_ref().unwrap());

		let receiving_address = &policy.unwrap().wallet.get_address(New).unwrap();
		println!("{}", receiving_address);
	}

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
}
