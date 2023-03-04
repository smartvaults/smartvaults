use std::str::FromStr;

use nostr_sdk::prelude::*;

use crate::util::create_client;

/// Get a list of events
#[derive(Debug, Clone, clap::Parser)]
#[command(name = "events", about = "Get a list of events")]
pub struct GetEventsCmd {
	/// Ids
	#[arg(short, long, action = clap::ArgAction::Append)]
	ids: Option<Vec<String>>,
	/// Authors
	#[arg(short, long, action = clap::ArgAction::Append)]
	authors: Option<Vec<String>>,
	// #[arg(long, action = clap::ArgAction::Append)]
	// users: Option<Vec<String>>,
	/// Kinds
	#[arg(short, long, action = clap::ArgAction::Append)]
	kinds: Option<Vec<u64>>,
	/// p tag
	#[arg(short, long, action = clap::ArgAction::Append)]
	e: Option<Vec<String>>,
	/// p tag
	#[arg(short, long, action = clap::ArgAction::Append)]
	p: Option<Vec<String>>,
	/// Since
	#[arg(short, long, action = clap::ArgAction::Append)]
	since: Option<u64>,
	/// Until
	#[arg(short, long, action = clap::ArgAction::Append)]
	until: Option<u64>,
	/// Limit
	#[arg(short, long, action = clap::ArgAction::Append)]
	limit: Option<usize>,
}

impl GetEventsCmd {
	/// Run the command
	pub fn run(&self, nostr_relay: String) -> Result<()> {
		// let subscriber = User::get(&self.subscriber);
		// let publisher = User::get(&self.publisher);

		let relays = vec![nostr_relay];

		let client = create_client(&Keys::generate(), relays, 0).expect("cannot create client");

		let authors: Option<Vec<XOnlyPublicKey>> = self.authors.as_ref().map(|auths| {
			auths
				.iter()
				.map(|a| XOnlyPublicKey::from_str(a.as_str()).expect("Invalid public key"))
				.collect()
		});

		let kinds: Option<Vec<Kind>> =
			self.kinds.as_ref().map(|kinds| kinds.iter().map(|k| Kind::from(*k)).collect());

		let events: Option<Vec<EventId>> = self.e.as_ref().map(|events| {
			events
				.iter()
				.map(|e| EventId::from_hex(e.as_str()).expect("Invalid event id"))
				.collect()
		});

		let pubkeys: Option<Vec<XOnlyPublicKey>> = self.p.as_ref().map(|pubs| {
			pubs.iter()
				.map(|p| XOnlyPublicKey::from_str(p.as_str()).expect("Invalid public key"))
				.collect()
		});

		let events: Vec<Event> = client
			.get_events_of(
				vec![Filter {
					ids: self.ids.clone(),
					authors,
					kinds,
					events,
					pubkeys,
					hashtags: None,
					references: None,
					search: None,
					since: self.since.map(Timestamp::from),
					until: self.until.map(Timestamp::from),
					limit: self.limit,
				}],
				None,
			)
			.expect("cannot get events of");

		for (i, event) in events.iter().enumerate() {
			if let Ok(e) = serde_json::to_string_pretty(event) {
				println!("{i}: {e:#}")
			}
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {

	use crate::DEFAULT_RELAY;
	use crate::user::User;
	use super::*;

	#[test]
	fn subscribe_alice_to_foobar() {
		let get_events_cmd = GetEventsCmd {
			ids: None,
			authors: Some(vec![User::alice().unwrap().nostr_user.pub_key().unwrap()]),
			kinds: None,
			e: None, 
			p: None,
			since: None,
			until: None,
			limit: None,
		};
		get_events_cmd.run(DEFAULT_RELAY.to_string()).expect("Cannot get events");
		// subscribe(&User::alice().unwrap(), &User::bob().unwrap()).expect("Unable to publish from
		// test");
	}
}
