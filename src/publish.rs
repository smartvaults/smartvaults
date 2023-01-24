use nostr_rust::events::{Event, EventPrepare};
use nostr_rust::nips::nip1::NIP1Error;
use nostr_rust::utils::get_timestamp;
use nostr_rust::{nostr_client::Client, Identity};
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};
use crate::users::known_users;
use clap::Error;

fn publish(
    nostr_client: Arc<Mutex<Client>>,
    identity: &Identity,
    content: &str,
    tags: &[Vec<String>],
    difficulty_target: u16,
) -> Result<Event, NIP1Error> {
    let event = EventPrepare {
        pub_key: identity.public_key_str.clone(),
        created_at: get_timestamp(),
        kind: 21,
        tags: tags.to_vec(),
        content: content.to_string(),
    }
    .to_event(identity, difficulty_target);

    nostr_client.lock().unwrap().publish_event(&event)?;
    Ok(event)
}

/// The `publish` command
#[derive(Debug, Clone, clap::Parser)]
#[command(name = "publish", about = "Publish a nostr events")]
pub struct PublishCmd {
   
    /// Content to post within an event
    #[arg(short, long)]
    content: String,

    /// Content to post within an event
    #[arg(short, long)]
    user: String,
}


impl PublishCmd {
    /// Run the command
    pub fn run(&self, nostr_relay: &String) -> Result<(), Error> {
        let binding = known_users();
        let user_key = binding.get(&self.user.to_ascii_uppercase()).unwrap();

        let poster_identity = Identity::from_str(&user_key).unwrap();

        let nostr_client = Arc::new(Mutex::new(Client::new(vec![nostr_relay]).unwrap()));

        publish(nostr_client, &poster_identity, &self.content, &[], 0).unwrap();
        Ok(())
    }
}
