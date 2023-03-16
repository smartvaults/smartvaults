use keechain_core::types::Psbt;
use nostr_sdk::prelude::*;

use crate::constants::SPENDING_PROPOSAL_APPROVED_KIND;
use crate::proposal::SpendingProposal;
use crate::user::User;
use crate::util::create_client;

#[derive(Debug, Clone, clap::Parser)]
#[command(name = "spend", about = "Proposing a send on a policy")]
pub struct ApproveCmd {
	/// User name
	#[arg(required = true)]
	user_name: String,

	/// Proposal id
	#[arg(required = true)]
	proposal_id: EventId,
}

impl ApproveCmd {
	/// Run the command
	pub fn run(&self, nostr_relay: String, bitcoin_network: Network) -> Result<()> {
		let user = User::get(&self.user_name)?;

		let relays = vec![nostr_relay];

		let keys = user.nostr_user.keys;
		let client = create_client(&keys, relays, 0).expect("cannot create client");

		let filter = Filter::new().id(self.proposal_id);
		let events: Vec<Event> = client.get_events_of(vec![filter], None)?;
		let event = events.first().expect("Proposal not found");
		let content =
			nips::nip04::decrypt(&keys.secret_key()?, &keys.public_key(), &event.content)?;
		let proposal = SpendingProposal::from_json(content)?;

		let mut psbt = Psbt::new(proposal.psbt, bitcoin_network);

		if psbt.sign(&user.seed)? {
			let content =
				nips::nip04::encrypt(&keys.secret_key()?, &keys.public_key(), psbt.as_base64())?;
			let event = EventBuilder::new(
				SPENDING_PROPOSAL_APPROVED_KIND,
				content,
				&[Tag::Event(self.proposal_id, None, None)],
			)
			.to_event(&keys)?;
			let event_id = client.send_event(event)?;
			println!("Spending proposal approved: {event_id}");
		} else {
			eprintln!("PSBT not signed!");
		}

		Ok(())
	}
}
