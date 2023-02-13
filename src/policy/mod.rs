use bdk::bitcoin::Network;
use miniscript::{descriptor::Descriptor, DescriptorPublicKey};
use nostr::prelude::Secp256k1;
use std::sync::Arc;
use miniscript::policy::Concrete;
// use miniscript::Descriptor;

use crate::user::User;
use anyhow::Result;
use bdk::{
	descriptor::{
		policy::{Policy, *},
		ExtendedDescriptor, ExtractPolicy, IntoWalletDescriptor,
	},
	wallet::signer::SignersContainer,
};
use std::fmt;
use termtree::Tree;
use std::str::FromStr;
pub struct CoinstrPolicy {
	pub name: String,
	pub description: String,
	pub descriptor: Descriptor<DescriptorPublicKey>,
	pub policy: Policy,
}

impl CoinstrPolicy {
	pub fn from_descriptor(
		name: String,
		description: String,
		descriptor: String,
	) -> Result<CoinstrPolicy> {
		let secp = Secp256k1::new();

		// let signer_keys = signer.bitcoin_user.setup_keys();
		// let other_signer_keys = other_signer.bitcoin_user.setup_keys();

		let (extended_desc, key_map) =
			ExtendedDescriptor::parse_descriptor(&secp, descriptor.as_str())?;

		// let descriptor = bdk::descriptor!(&descriptor).unwrap();
		// let (wallet_desc, keymap) =
		// 	descriptor.into_wallet_descriptor(&secp, Network::Testnet).unwrap();
		let signers = Arc::new(SignersContainer::build(key_map, &extended_desc, &secp));

		let policy = extended_desc
			.extract_policy(&signers, BuildSatisfaction::None, &secp)
			.unwrap()
			.unwrap();

		Ok(CoinstrPolicy { name, description, descriptor: extended_desc, policy })
	}

	pub fn new_dumb_multisig(
		name: String,
		description: String,
		signer: &User,
		other_signer: &User,
	) -> Result<CoinstrPolicy> {
		let secp = Secp256k1::new();

		let signer_keys = signer.bitcoin_user.setup_keys();
		let other_signer_keys = other_signer.bitcoin_user.setup_keys();

		let descriptor = bdk::descriptor!(tr(other_signer_keys.1, pk(signer_keys.1))).unwrap();

		let (wallet_desc, keymap) =
			descriptor.into_wallet_descriptor(&secp, Network::Testnet).unwrap();
		let signers_container = Arc::new(SignersContainer::build(keymap, &wallet_desc, &secp));

		let policy = wallet_desc
			.extract_policy(&signers_container, BuildSatisfaction::None, &secp)
			.unwrap()
			.unwrap();

		Ok(CoinstrPolicy { name, description, descriptor: wallet_desc, policy })
	}
}

fn display_key(key: &PkOrF) -> String {
	// TODO: Use aliases
	match key {
		PkOrF::Pubkey(pk) => format!("<pk:{}>", pk.to_string()),
		PkOrF::XOnlyPubkey(pk) => format!("<xonly-pk:{}>", pk.to_string()),
		PkOrF::Fingerprint(f) => format!("<fingerprint:{}>", f.to_string()),
	}
}

fn add_node(item: &SatisfiableItem) -> Tree<String> {
	let mut si_tree: Tree<String> = Tree::new(format!("🆔 {}", item.id()));

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
			si_tree.push(format!("🎚️ {} of {} MultiSig:", threshold, keys.len()));
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
		writeln!(f, "Name	        : {}", &self.name)?;
		writeln!(f, "Description	: {}", &self.description)?;

		let mut tree: Tree<String> = Tree::new(self.name.clone());
		tree.push(add_node(&self.policy.item));
		writeln!(f, "{}", tree)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {

	use super::*;
	use crate::user::User;

	#[test]
	fn build_multisig_policy() {
		let alice = User::get(&"alice".to_string()).unwrap();
		let bob = User::get(&"bob".to_string()).unwrap();

		let policy = CoinstrPolicy::new_dumb_multisig(
			"💸 My testing policy".to_string(),
			"A policy for testing Alice and Bob multisig".to_string(),
			&alice,
			&bob,
		);
		println!("{}", &policy.unwrap());
	}

	#[test]
	fn build_with_descriptor() {
		let policy = CoinstrPolicy::from_descriptor(
			"💸 My testing policy".to_string(),
			"A policy for testing Alice and Bob multisig".to_string(),
            "wsh(and_v(v:pk(cV3oCth6zxZ1UVsHLnGothsWNsaoxRhC6aeNi5VbSdFpwUkgkEci),or_d(pk(cVMTy7uebJgvFaSBwcgvwk8qn8xSLc97dKow4MBetjrrahZoimm2),older(12960))))".to_string()
		);
		println!("{}", &policy.unwrap());
	}

	#[test]
	fn build_with_complex_descriptor() {
		let policy_str = "or(10@thresh(4,pk(029ffbe722b147f3035c87cb1c60b9a5947dd49c774cc31e94773478711a929ac0),pk(025f05815e3a1a8a83bfbb03ce016c9a2ee31066b98f567f6227df1d76ec4bd143),pk(025625f41e4a065efc06d5019cbbd56fe8c07595af1231e7cbc03fafb87ebb71ec),pk(02a27c8b850a00f67da3499b60562673dcf5fdfb82b7e17652a7ac54416812aefd),pk(03e618ec5f384d6e19ca9ebdb8e2119e5bef978285076828ce054e55c4daf473e2)),1@and(older(4209713),thresh(2,pk(03deae92101c790b12653231439f27b8897264125ecb2f46f48278603102573165),pk(033841045a531e1adf9910a6ec279589a90b3b8a904ee64ffd692bd08a8996c1aa),pk(02aebf2d10b040eb936a6f02f44ee82f8b34f5c1ccb20ff3949c2b28206b7c1068))))";

		// Parse the string as a [`Concrete`] type miniscript policy.
		let policy = Concrete::<String>::from_str(policy_str).unwrap();

		// Create a `wsh` type descriptor from the policy.
		// `policy.compile()` returns the resulting miniscript from the policy.
		let descriptor = Descriptor::new_wsh(policy.compile().unwrap()).unwrap();

		let policy = CoinstrPolicy::from_descriptor(
			"💸 My testing policy".to_string(),
			"A policy for testing Alice and Bob multisig".to_string(),
			"or(10@thresh(4,pk(029ffbe722b147f3035c87cb1c60b9a5947dd49c774cc31e94773478711a929ac0),pk(025f05815e3a1a8a83bfbb03ce016c9a2ee31066b98f567f6227df1d76ec4bd143),pk(025625f41e4a065efc06d5019cbbd56fe8c07595af1231e7cbc03fafb87ebb71ec),pk(02a27c8b850a00f67da3499b60562673dcf5fdfb82b7e17652a7ac54416812aefd),pk(03e618ec5f384d6e19ca9ebdb8e2119e5bef978285076828ce054e55c4daf473e2)),1@and(older(4209713),thresh(2,pk(03deae92101c790b12653231439f27b8897264125ecb2f46f48278603102573165),pk(033841045a531e1adf9910a6ec279589a90b3b8a904ee64ffd692bd08a8996c1aa),pk(02aebf2d10b040eb936a6f02f44ee82f8b34f5c1ccb20ff3949c2b28206b7c1068))))".to_string(),
		);
		println!("{}", &policy.unwrap());
	}
}
