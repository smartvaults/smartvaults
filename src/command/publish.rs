// use nostr_sdk::client::blocking::Client;
use nostr_sdk::prelude::*;

use crate::{user::User, util::create_client};

/// The `publish` command
#[derive(Debug, Clone, clap::Parser)]
#[command(name = "publish", about = "Publish a nostr events")]
pub struct PublishCmd {
	/// Content to post within an event
	#[arg(short, long)]
	content: String,

	/// user to publish from
	#[arg(short, long)]
	user: String,
}

impl PublishCmd {
	/// Run the command
	pub fn run(&self, nostr_relay: String) -> Result<()> {
		let user = User::get(&self.user).unwrap();
		let relays: Vec<String> = vec![nostr_relay];
		let client = create_client(&user.nostr_user.keys, relays, 0).expect("cannot create client");

		// TODO: support for tags
		client.publish_text_note(&self.content, &[]).expect("cannot publish note");

		Ok(())
	}
}

#[cfg(test)]
mod tests {

	use crate::DEFAULT_RELAY;

	use super::*;

	#[test]
	fn publish_foobar() {
		let publish_cmd = PublishCmd { user: "bob".to_string(), content: "foobar".to_string() };
		publish_cmd.run(DEFAULT_RELAY.to_string()).expect("Unable to publish from test");
	}
}
