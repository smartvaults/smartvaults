// use nostr_sdk::client::blocking::Client;
use nostr_sdk::prelude::*;

use crate::user::User;
use crate::util::create_client;
use clap::Error;

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
    pub fn run(&self, nostr_relay: &String) -> Result<(), Error> {
        let user = User::get(&self.user).unwrap();
        let relays: Vec<String> = vec![nostr_relay.clone()];
        let client = create_client(&user.nostr_user.keys, relays, 0).expect("cannot create client");

        // TODO: support for tags
        client
            .publish_text_note(&self.content, &[])
            .expect("cannot publish note");

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn publish_foobar() {
        let publish_cmd = PublishCmd {
            user: "bob".to_string(),
            content: "foobar".to_string(),
        };
        publish_cmd
            .run(&"wss://nostr.hashed.systems".to_string())
            .expect("Unable to publish from test");
    }
}
