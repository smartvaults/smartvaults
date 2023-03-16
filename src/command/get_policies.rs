use std::time::Duration;

use nostr_sdk::prelude::*;

use crate::constants::POLICY_KIND;
use crate::user::User;
use crate::{policy::CoinstrPolicy, util::create_client};

#[derive(Debug, Clone, clap::Parser)]
#[command(name = "policies", about = "Get policies list from nostr")]
pub struct GetPoliciesCmd {
	// User name
	#[arg(required = true)]
	user: String,
}

impl GetPoliciesCmd {
	/// Run the command
	pub fn run(&self, nostr_relay: String) -> Result<()> {
		let relays = vec![nostr_relay];
		let user = User::get(&self.user)?;
		let keys = user.nostr_user.keys;
		let client = create_client(&keys, relays, 0).expect("cannot create client");

		let timeout = Some(Duration::from_secs(300));
		let filter = Filter::new().author(keys.public_key()).kind(POLICY_KIND);
		let events: Vec<Event> = client.get_events_of(vec![filter], timeout)?;

		for event in events.into_iter() {
			let content =
				nips::nip04::decrypt(&keys.secret_key()?, &keys.public_key(), &event.content)?;

			let policy = CoinstrPolicy::from_json(&content)?;
			println!("Policy:");
			println!("- ID: {}", &event.id);
			println!("- Name: {}", &policy.name);
			println!("- Description: {}", &policy.description);
			println!("- Descriptor: {}", policy.descriptor);
			println!();

			println!("{}", policy);
		}

		Ok(())
	}
}
