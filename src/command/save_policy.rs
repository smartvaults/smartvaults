use nostr_sdk::prelude::*;

use crate::{policy::CoinstrPolicy, util::create_client};

#[derive(Debug, Clone, clap::Parser)]
#[command(name = "save-policy", about = "Save policy")]
pub struct SavePolicyCmd {
	/// Secret Key
	#[arg(required = true)]
	secret_key: String,

	/// Name
	#[arg(required = true)]
	name: String,

	/// Description
	#[arg(required = true)]
	description: String,

	/// Descriptor
	#[arg(required = true)]
	descriptor: String,
}

impl SavePolicyCmd {
	/// Run the command
	pub fn run(&self, nostr_relay: String) -> Result<()> {
		let relays = vec![nostr_relay];

		let keys = Keys::from_sk_str(&self.secret_key)?;
		let client = create_client(&keys, relays, 0).expect("cannot create client");

		let policy =
			match CoinstrPolicy::from_descriptor(&self.name, &self.description, &self.descriptor) {
				Ok(policy) => policy,
				Err(_) => match CoinstrPolicy::from_policy_str(
					&self.name,
					&self.description,
					&self.descriptor,
				) {
					Ok(policy) => policy,
					Err(e) => return Err(e),
				},
			};

		let content =
			nips::nip04::encrypt(&keys.secret_key()?, &keys.public_key(), policy.as_json())?;
		let event = EventBuilder::new(Kind::Custom(9289), content, &[]).to_event(&keys)?;
		let event_id = client.send_event(event)?;

		println!("Saved policy at event {}", event_id.to_bech32()?);

		Ok(())
	}
}
