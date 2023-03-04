use crate::{user::User, DEFAULT_RELAY};
use keechain_core::crypto::aes::decrypt;
use nostr_sdk::{client::blocking::Client, prelude::*};

fn subscribe(subscriber: &User, publisher: &User) -> Result<()> {
	let subscriber_keys = Keys::new(subscriber.nostr_user.keys.secret_key().unwrap());
	let client = Client::new(&subscriber_keys);

	let subscription = Filter::new().pubkey(publisher.nostr_user.pub_key_hex());

	client.subscribe(vec![subscription]);

	client.handle_notifications(|notification| {
		if let RelayPoolNotification::Event(_url, event) = notification {
			if event.kind == Kind::EncryptedDirectMessage {
				if let Ok(msg) = decrypt(
					subscriber_keys.secret_key().unwrap().as_ref(),
					event.content.as_bytes(),
				) {
					println!("New DM: {:?}", msg);
				} else {
					eprintln!("Impossible to decrypt direct message");
				}
			} else {
				println!("{:#?}", event);
			}
		}

		Ok(())
	})?;

	Ok(())
}

/// The `subscribe` command
#[derive(Debug, Clone, clap::Parser)]
#[command(name = "subscribe", about = "Subscribe to nostr events")]
pub struct SubscribeCmd {
	/// The relay to request subscription from
	#[arg(short, long, default_value_t = String::from(DEFAULT_RELAY))]
	relay: String,

	/// user to subscribe from
	#[arg(short, long)]
	publisher: String,

	/// user to subscribe from
	#[arg(short, long)]
	subscriber: String,
}

impl SubscribeCmd {
	/// Run the command
	pub fn run(&self, _nostr_relay: &str) -> Result<()> {
		let subscriber = User::get(&self.subscriber).expect("user not found");
		let publisher = User::get(&self.publisher).expect("User not found");

		subscribe(&subscriber, &publisher).expect("Unable to subscribe");
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	// use super::*;

	#[test]
	fn subscribe_alice_to_foobar() {
		// how to test subscribe? must use async?
		// subscribe(&User::alice().unwrap(), &User::bob().unwrap())
		// 	.expect("Unable to publish from test");
	}
}
