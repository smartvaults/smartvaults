use nostr_sdk::client::blocking::Client;
use nostr_sdk::prelude::*;

use crate::users::User;

fn subscribe(subscriber: &User, publisher: &User)-> Result<()> {

    let subscriber_keys = Keys::new(subscriber.nostr_secret_hex);
    let client = Client::new(&subscriber_keys);

    let subscription = SubscriptionFilter::new()
        .pubkey(publisher.nostr_x_only_public_key);
        // .since(Timestamp::now());

    client.subscribe(vec![subscription]);

    client.handle_notifications(|notification| {
        if let RelayPoolNotification::Event(_url, event) = notification {
            if event.kind == Kind::EncryptedDirectMessage {
                if let Ok(msg) = decrypt(
                    &subscriber_keys.secret_key().unwrap(),
                    &event.pubkey,
                    &event.content,
                ) {
                    println!("New DM: {}", msg);
                } else {
                    log::error!("Impossible to decrypt direct message");
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
    #[arg(short, long, default_value_t = String::from("ws://127.0.0.1:8081"))]
    relay: String,

    /// user to subscribe from
    #[arg(short, long)]
    publisher: String,

    /// user to subscribe from
    #[arg(short, long)]
    subscriber: String,
}

use clap::Error;
impl SubscribeCmd {
    /// Run the command
    pub fn run(&self, _nostr_relay: &String) -> Result<(), Error> {

        let subscriber = User::get(&self.subscriber).expect("user not found");
        let publisher = User::get(&self.publisher).expect("User not found");

        subscribe(&subscriber, &publisher).expect("Unable to subscribe");
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn subscribe_alice_to_foobar() {
        subscribe(&User::alice().unwrap(), &User::bob().unwrap()).expect("Unable to publish from test");
    }
}
