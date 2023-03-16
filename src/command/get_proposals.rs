use std::time::Duration;

use nostr_sdk::prelude::*;

use crate::constants::SPENDING_PROPOSAL_KIND;
use crate::proposal::SpendingProposal;
use crate::user::User;
use crate::util::create_client;

#[derive(Debug, Clone, clap::Parser)]
#[command(name = "proposals", about = "Get proposals list from nostr")]
pub struct GetProposalsCmd {
	// User name
	#[arg(required = true)]
	user: String,
}

impl GetProposalsCmd {
	/// Run the command
	pub fn run(&self, nostr_relay: String) -> Result<()> {
		let relays = vec![nostr_relay];
		let user = User::get(&self.user)?;
		let keys = user.nostr_user.keys;
		let client = create_client(&keys, relays, 0).expect("cannot create client");

		let timeout = Some(Duration::from_secs(300));
		let filter = Filter::new().author(keys.public_key()).kind(SPENDING_PROPOSAL_KIND);
		let events: Vec<Event> = client.get_events_of(vec![filter], timeout)?;

		for event in events.into_iter() {
			let content =
				nips::nip04::decrypt(&keys.secret_key()?, &keys.public_key(), &event.content)?;
			let proposal = SpendingProposal::from_json(&content)?;
			println!();
			println!("- Proposal id: {}", &event.id);
			println!("- Memo: {}", &proposal.memo);
			println!("- To address: {}", &proposal.to_address);
			println!("- Amount: {}", &proposal.amount);
			println!();
		}

		Ok(())
	}
}
