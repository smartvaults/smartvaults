
use nostr_rust::{
    events::extract_events_ws, nostr_client::Client, req::ReqFilter, Message,
};
use std::{
    sync::{Arc, Mutex},
    thread,
};

fn handle_message(message: &Message) -> Result<(), String> {
    let events = extract_events_ws(message);
    println!("{}", serde_json::to_string_pretty(&events).unwrap());

    Ok(())
}

use crate::users::{alice_keys, bob_keys, charlie_keys};

fn subscribe(nostr_client: Arc<Mutex<Client>>) {
    // Run a new thread to handle messages
    let _subscription_id = nostr_client
        .lock()
        .unwrap()
        .subscribe(vec![ReqFilter {
            ids: None,
            authors: Some(vec![
                alice_keys().1,
                bob_keys().1,
                charlie_keys().1, //, elephant_keys().1
            ]),
            kinds: None,
            e: None,
            p: None,
            since: Some(1673908031),
            until: None,
            limit: Some(10),
        }])
        .unwrap();

    let nostr_clone = nostr_client.clone();
    let handle_thread = thread::spawn(move || {
        println!("Listening...");

        loop {
            let events = nostr_clone.lock().unwrap().next_data().unwrap();

            for (_relay_url, message) in events.iter() {
                handle_message(message).unwrap();
            }
        }
    });

    handle_thread.join().unwrap();
}

/// The `subscribe` command
#[derive(Debug, Clone, clap::Parser)]
#[command(name = "subscribe", about = "Subscribe to nostr events")]
pub struct SubscribeCmd {
    /// The relay to request subscription from
    #[arg(short, long, default_value_t = String::from("ws://127.0.0.1:8081"))]
    relay: String,
}

use clap::Error;
impl SubscribeCmd {
    /// Run the command
    pub fn run(&self) -> Result<(), Error> {
        let nostr_client = Arc::new(Mutex::new(Client::new(vec![&self.relay]).unwrap()));

        subscribe(nostr_client);
        Ok(())
    }
}
