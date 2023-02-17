

pub struct Maestro {

}

#[cfg(test)]
mod tests {

    const NOSTR_RELAY: &str = "wss://nostr.hashed.systems";

    use crate::user::User;
    use crate::policy::CoinstrPolicy;
    
    use nostr::prelude::Secp256k1;
    use bdk::{
        bitcoin::Network,
        blockchain::EsploraBlockchain,
        database::MemoryDatabase,
        descriptor::{
            policy::{Policy, *},
            IntoWalletDescriptor,
        },
        wallet::{SyncOptions, Wallet, AddressIndex::New},
        KeychainKind,
    };

    #[test]
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
		let esplora = EsploraBlockchain::new("https://blockstream.info/testnet/api", 20);
		policy.as_ref().unwrap().wallet.sync(&esplora, SyncOptions::default()).unwrap();

		let balance = policy.as_ref().unwrap().wallet.get_balance().unwrap();
		println!("Wallet balances in SATs: {}", balance);

		let (mut psbt, tx_details) = {
			let mut builder = policy.as_ref().unwrap().wallet.build_tx();
			builder.add_recipient(alice_address.script_pubkey(), 500);
			builder.finish().unwrap()
		};

		println!("\nNumber of signers in policy wallet   {}", policy.as_ref().unwrap().wallet.get_signers(bdk::KeychainKind::External).signers().len());
		println!("\nUnsigned PSBT: \n{}", psbt);

        let relays: Vec<String> = vec![NOSTR_RELAY.to_string()];
        let client = crate::util::create_client(&alice.nostr_user.keys, relays, 0).expect("cannot create client");

        // TODO: support for tags
        let bob_tag = nostr_sdk::prelude::Tag::PubKey(bob.nostr_user.pub_key_hex(), Some("New spending request; memo: this is for the final milestone deliverable, thx".to_string()));
        client
            .publish_text_note(psbt.to_string(), &[bob_tag])
            .expect("cannot publish note");

		// let finalized = policy.as_ref().unwrap().wallet.sign(&mut psbt, SignOptions::default()).unwrap();
		// println!("\nSigned the PSBT: \n{}\n", psbt);

		// assert!(finalized, "The PSBT was not finalized!");
        // println!("The PSBT has been signed and finalized.");

		// let raw_transaction = psbt.extract_tx();
		// let txid = raw_transaction.txid();
	
		println!("Not sending unless below is uncommented");
		// esplora.broadcast(&raw_transaction);
		// println!("Transaction broadcast! TXID: {txid}.\nExplorer URL: https://mempool.space/testnet/tx/{txid}", txid = txid);

		let receiving_address = &policy.unwrap().wallet.get_address(New).unwrap();
		println!("Refill this testnet wallet from the faucet: 	https://bitcoinfaucet.uo1.net/?to={receiving_address}");
	}
}