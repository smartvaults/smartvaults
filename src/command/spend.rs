use bdk::blockchain::ElectrumBlockchain;
use bdk::database::MemoryDatabase;
use bdk::electrum_client::Client as ElectrumClient;
use bdk::{SyncOptions, Wallet};
use keechain_core::bitcoin::util::address::Address;
use nostr_sdk::prelude::*;

use crate::constants::SPENDING_PROPOSAL_KIND;
use crate::policy::CoinstrPolicy;
use crate::proposal::SpendingProposal;
use crate::user::User;
use crate::util::create_client;

#[derive(Debug, Clone, clap::Parser)]
#[command(name = "spend", about = "Proposing a send on a policy")]
pub struct SpendCmd {
	// User name
	#[arg(required = true)]
	user: String,

	/// Policy id
	#[arg(required = true)]
	policy_id: EventId,

	/// Memo
	#[arg(required = true)]
	memo: String,

	/// To address
	#[arg(required = true)]
	to_address: Address,

	/// Amount in sats
	#[arg(required = true)]
	amount: u64,
}

impl SpendCmd {
	/// Run the command
	pub fn run(
		&self,
		nostr_relay: String,
		bitcoin_endpoint: String,
		bitcoin_network: Network,
	) -> Result<()> {
		let relays = vec![nostr_relay];
		let user = User::get(&self.user)?;
		let keys = user.nostr_user.keys;
		let client = create_client(&keys, relays, 0).expect("cannot create client");

		let filter = Filter::new().id(self.policy_id);
		let events: Vec<Event> = client.get_events_of(vec![filter], None)?;
		let event = events.first().expect("Policy not found");
		let content =
			nips::nip04::decrypt(&keys.secret_key()?, &keys.public_key(), &event.content)?;
		let policy = CoinstrPolicy::from_json(&content)?;

		let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&bitcoin_endpoint)?);
		let wallet = Wallet::new(
			&policy.descriptor.to_string(),
			None,
			bitcoin_network,
			MemoryDatabase::default(),
		)?;
		wallet.sync(&blockchain, SyncOptions::default())?;

		let (psbt, _details) = {
			let mut builder = wallet.build_tx();
			builder.add_recipient(self.to_address.script_pubkey(), self.amount).enable_rbf();
			builder.finish()?
		};

		let proposal =
			SpendingProposal::new(&self.memo, self.to_address.clone(), self.amount, psbt);

		let content =
			nips::nip04::encrypt(&keys.secret_key()?, &keys.public_key(), proposal.as_json())?;
		let event = EventBuilder::new(
			SPENDING_PROPOSAL_KIND,
			content,
			&[Tag::Event(self.policy_id, None, None)],
		)
		.to_event(&keys)?;
		let proposal_id = client.send_event(event)?;

		println!("Spending proposal sent: {proposal_id}");

		Ok(())
	}
}
