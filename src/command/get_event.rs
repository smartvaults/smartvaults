use nostr_sdk::prelude::*;

use crate::util::create_client;

/// Get a list of events
#[derive(Debug, Clone, clap::Parser)]
#[command(name = "event", about = "Get an event")]
pub struct GetEventCmd {
	/// Id
	#[arg(required = true)]
	id: String,
}

impl GetEventCmd {
	/// Run the command
	pub fn run(&self, nostr_relay: String) -> Result<()> {
		let relays = vec![nostr_relay];
		let client = create_client(&Keys::generate(), relays, 0).expect("cannot create client");

		let events: Vec<Event> = client
			.get_events_of(vec![Filter::new().id(&self.id)], None)
			.expect("cannot get event");

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

	use super::*;
	use crate::constants::DEFAULT_RELAY;

	#[test]
	fn subscribe_alice_to_foobar() {
		let get_event_cmd = GetEventCmd {
			id: "d3a421ae9cde2a530429867db0923fcfd5812dde84bb789169cd99b1d53d236a".to_string(),
		};
		get_event_cmd.run(DEFAULT_RELAY.to_string()).expect("Cannot get events");
	}
}
