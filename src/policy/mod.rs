use bdk::bitcoin::Network;
use miniscript::{descriptor::Descriptor, DescriptorPublicKey};
use nostr::prelude::Secp256k1;
use serde_json::json;
use std::{any::Any, str::FromStr, sync::Arc};

use anyhow::Result;
use bdk::{
	blockchain::EsploraBlockchain,
	database::MemoryDatabase,
	descriptor::{
		policy::{Policy, *},
		ExtractPolicy, IntoWalletDescriptor, ExtendedDescriptor,
	},
	keys::{
		bip39::{Language::English, Mnemonic},
		DerivableKey, IntoDescriptorKey,
	},
	wallet::{signer::SignersContainer, SyncOptions, Wallet},
	KeychainKind,
};
use std::fmt;
// use ptree::{print_tree, TreeBuilder};
use crate::user::User;
use termtree::Tree;

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

		let (extended_desc, key_map) = ExtendedDescriptor::parse_descriptor(&secp, descriptor.as_str())?;

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
			// let leaf = ;
			si_tree.push(format!("🔑 Schnorr Sig of {}", display_key(key)));
			// return Tree::new(format!("Schnorr Sig of {}", display_key(key)));
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

			// s = s+ &items_s;
			// // for item in items {
			// //     s = s+ &format!("Individual item: {} ", &item);
			// // }

			// },
		},
	}
	si_tree
}

// fn add_policy(p: &Policy) -> Tree<&'static str>  {

//     let mut tree = Tree::new(p.id.as_str());
//     // tree.push();
//     // print!("ID    : {}  - ", p.id);

//     tree.push(add_node(&p.item));
//     println!("Satisfaction  : {:?}", p.satisfaction);
//     // tb.end_child();
//     // tb
//     // println!("Contribution  : {:?}", p.contribution);
//     tree
// }

// pub struct BdkPolicy(Policy);

// impl fmt::Display for BdkPolicy {
// 	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
// 		// writeln!(f, "bdk policy : {:?}", &self.0)?;

//         // writeln!(f, "WHOLE: {}", serde_json::to_string_pretty(&self.0).unwrap())?;

//         print_satisfiable_item(&self.0.item);
// 		Ok(())
// 	}
// }

fn print_coinstr_policy_tree(pol: &CoinstrPolicy) {
	println!("\nCoinstr Policy");
	println!("Name	        : {}", &pol.name);
	println!("Description	: {}", &pol.description);

	// writeln!(f, "  BDK Policy	: {}", &self.policy.to_string())?;

	let mut tree: Tree<String> = Tree::new(pol.name.to_string());

	tree.push(add_node(&pol.policy.item));
	println!("{}", tree);
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
		print_coinstr_policy_tree(&policy.unwrap());
	}

	#[test]
	fn build_with_descriptor() {
		let alice = User::get(&"alice".to_string()).unwrap();
		let bob = User::get(&"bob".to_string()).unwrap();

		let policy = CoinstrPolicy::from_descriptor(
			"💸 My testing policy".to_string(),
			"A policy for testing Alice and Bob multisig".to_string(),
            "wsh(and_v(v:pk(cV3oCth6zxZ1UVsHLnGothsWNsaoxRhC6aeNi5VbSdFpwUkgkEci),or_d(pk(cVMTy7uebJgvFaSBwcgvwk8qn8xSLc97dKow4MBetjrrahZoimm2),older(12960))))".to_string()
		);
		print_coinstr_policy_tree(&policy.unwrap());
	}
}
