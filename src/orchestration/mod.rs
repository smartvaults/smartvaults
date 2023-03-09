#[allow(dead_code)]
pub struct Maestro;

#[allow(dead_code)]
impl Maestro {
	pub fn what_is() {
		println!(
			"Maestro orchestrates signature requests, either as a bot or a relay. To be developed."
		);
	}
}

#[cfg(test)]
mod tests {
	use bdk::{
		bitcoin::Network,
		blockchain::ElectrumBlockchain,
		database::MemoryDatabase,
		electrum_client::Client,
		wallet::{AddressIndex::New, SyncOptions},
		Wallet,
	};

	use crate::{policy::CoinstrPolicy, user::User, DEFAULT_RELAY, DEFAULT_TESTNET_ENDPOINT};

	const NOSTR_RELAY: &str = "wss://relay.house";

	#[allow(unused)]
	#[test]
	fn test_tagging_bob_on_psbt() {
		let alice = User::get(&"alice".to_string()).unwrap();
		let bob = User::get(&"bob".to_string()).unwrap();

		let alice_address = alice.bitcoin_user.wallet.get_address(New).unwrap();
		println!("Alice address	: {}", alice_address);

		let policy = CoinstrPolicy::new_one_of_two_taptree(
			"💸 My TapTree policy".to_string(),
			"A 1 of 2 Taptree policy".to_string(),
			&alice,
			&bob,
		)
		.unwrap();
		println!("{policy}");

		println!("Syncing policy wallet.");
		let database = MemoryDatabase::new();
		let blockchain = ElectrumBlockchain::from(Client::new(DEFAULT_TESTNET_ENDPOINT).unwrap());
		let wallet =
			Wallet::new(&policy.descriptor.to_string(), None, Network::Testnet, database).unwrap();
		wallet.sync(&blockchain, SyncOptions::default()).unwrap();

		let balance = wallet.get_balance().unwrap();
		println!("Wallet balances in SATs: {}", balance);

		const TEST_NUM_SATS: u64 = 500;
		if balance.confirmed < TEST_NUM_SATS {
			let receiving_address = wallet.get_address(New).unwrap();
			println!("Refill this testnet wallet from the faucet: 	https://bitcoinfaucet.uo1.net/?to={receiving_address}");
			return;
		}

		let (mut psbt, tx_details) = {
			let mut builder = wallet.build_tx();
			builder.add_recipient(alice_address.script_pubkey(), TEST_NUM_SATS);
			builder.finish().unwrap()
		};

		let relays: Vec<String> = vec![NOSTR_RELAY.to_string()];
		let client = crate::util::create_client(&alice.nostr_user.keys, relays, 0)
			.expect("cannot create client");

		let bob_tag = nostr_sdk::prelude::Tag::PubKey(
			bob.nostr_user.pub_key_hex(),
			Some(DEFAULT_RELAY.to_string()),
		);

		client
			.publish_text_note(psbt.to_string(), &[bob_tag])
			.expect("cannot publish note");

		let receiving_address = wallet.get_address(New).unwrap();
		println!("Refill this testnet wallet from the faucet: 	https://bitcoinfaucet.uo1.net/?to={receiving_address}");
	}

	#[test]
	pub fn basic_payload_signature_send_and_reply() {
		let _alice = User::get(&"alice".to_string()).unwrap();
		let _bob = User::get(&"bob".to_string()).unwrap();
	}
}
