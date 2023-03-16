#[cfg(test)]
mod tests {

	use crate::command::save_policy::SavePolicyCmd;
    use crate::policy::CoinstrPolicy;
	use crate::user::User;
    use crate::util;
	use bdk::database::MemoryDatabase;
	use bdk::miniscript::policy::Concrete;
	use bdk::wallet::AddressIndex::New;
	use bdk::wallet::Wallet;
	use nostr_sdk::prelude::*;
	use std::str::FromStr;
    use bdk::descriptor::IntoWalletDescriptor;

	#[test]
    #[rustfmt::skip]
	fn test_waterwell_3_of_5() {
		let sarah = User::get(&"sarah".to_string()).unwrap();
		let john = User::get(&"john".to_string()).unwrap();
		let maria = User::get(&"maria".to_string()).unwrap();
		let trey = User::get(&"trey".to_string()).unwrap();
		let lee = User::get(&"lee".to_string()).unwrap();

		let policy_str = format!("thresh(3,pk({}),pk({}),pk({}),pk({}),pk({}))", 
            sarah.nostr_user.keys.secret_key().unwrap().public_key(SECP256K1).to_string(), 
            john.nostr_user.keys.secret_key().unwrap().public_key(SECP256K1).to_string(),
            maria.nostr_user.keys.secret_key().unwrap().public_key(SECP256K1).to_string(), 
            trey.nostr_user.keys.secret_key().unwrap().public_key(SECP256K1).to_string(),
            lee.nostr_user.keys.secret_key().unwrap().public_key(SECP256K1).to_string(), 
        );
		println!("Policy string	<test_waterwell_3_of_5>	: {}", &policy_str);

		let pol: Concrete<String> = Concrete::from_str(&policy_str).unwrap();
		// In case we can't find an internal key for the given policy, we set the internal key to
		// a random pubkey as specified by BIP341 (which are *unspendable* by any party :p)
		let desc = pol.compile_tr(Some("UNSPENDABLE_KEY".to_string())).unwrap();
		println!("Descriptor    : {}", desc.to_string());

        let policy = CoinstrPolicy::from_policy_str(
			"💸 Waterwell Crowdfund Vault".to_string(),
			"3 of 5 policy for initial Waterwell fundraising".to_string(),
			pol.to_string()
		).unwrap();

		println!("{policy}");

        let database = MemoryDatabase::new();
		let wallet =
			Wallet::new(&policy.descriptor.to_string(), None, Network::Testnet, database).unwrap();

        let receiving_address = wallet.get_address(New).unwrap();
		println!("{}", receiving_address); 

		let relays = vec!["wss://relay.rip".to_string()];
		let client = util::create_client(&lee.nostr_user.keys, relays, 0).expect("cannot create client");

        let content =
			nips::nip04::encrypt(&lee.nostr_user.keys.secret_key().unwrap(), &lee.nostr_user.keys.public_key(), policy.as_json()).unwrap();
		let event = EventBuilder::new(Kind::Custom(9289), content, &[]).to_event(&lee.nostr_user.keys).unwrap();
		let event_id = client.send_event(event).unwrap();

		println!("Saved policy at event {}", event_id.to_bech32().unwrap());

        }
	}

