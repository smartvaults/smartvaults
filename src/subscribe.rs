use nostr_sdk::prelude::*;
use std::{
    sync::{Arc, Mutex},
    thread,
};

use crate::users::User;

// fn handle_message(message: &Message) -> Result<(), String> {
//     let events = extract_events_ws(message);
//     println!("{}", serde_json::to_string_pretty(&events).unwrap());

//     Ok(())
// }

async fn subscribe()-> Result<()> {

    // let my_keys = Keys::new(user.nostr_secret_hex);
    let user = User::alice().unwrap();
    let my_keys = Keys::new(user.nostr_secret_hex);
    let client = Client::new(&my_keys);

    let subscription = SubscriptionFilter::new()
        .pubkey(user.nostr_x_only_public_key)
        .since(Timestamp::now());

    client.subscribe(vec![subscription]).await?;

    loop {
        let mut notifications = client.notifications();
        while let Ok(notification) = notifications.recv().await {
            if let RelayPoolNotification::Event(_url, event) = notification {
                if event.kind == Kind::EncryptedDirectMessage {
                    if let Ok(msg) = decrypt(&my_keys.secret_key()?, &event.pubkey, &event.content)
                    {
                        println!("New DM: {}", msg);
                    } else {
                        log::error!("Impossible to decrypt direct message");
                    }
                } else {
                    println!("{:?}", event);
                }
            }
        }
    }

    // // Run a new thread to handle messages
    // let _subscription_id = nostr_client
    //     .lock()
    //     .unwrap()
    //     .subscribe(vec![ReqFilter {
    //         ids: None,
    //         authors: Some(vec![
    //             alice_keys().1,
    //             bob_keys().1,
    //             charlie_keys().1, //, elephant_keys().1
    //         ]),
    //         kinds: None,
    //         e: None,
    //         p: None,
    //         since: Some(1673969339),
    //         until: None,
    //         limit: Some(10),
    //     }])
    //     .unwrap();

    // let nostr_clone = nostr_client.clone();
    // let handle_thread = thread::spawn(move || {
    //     println!("Listening...");

    //     loop {
    //         let events = nostr_clone.lock().unwrap().next_data().unwrap();

    //         for (_relay_url, message) in events.iter() {
    //             handle_message(message).unwrap();
    //         }
    //     }
    // });

    // handle_thread.join().unwrap();
}

/// The `subscribe` command
#[derive(Debug, Clone, clap::Parser)]
#[command(name = "subscribe", about = "Subscribe to nostr events")]
pub struct SubscribeCmd {
    /// The relay to request subscription from
    #[arg(short, long, default_value_t = String::from("wss://relay.damus.io"))]
    relay: String,
}

use clap::Error;
impl SubscribeCmd {
    /// Run the command
    pub fn run(&self, nostr_relay: &String) -> Result<(), Error> {
        // let nostr_client = Arc::new(Mutex::new(Client::new(vec![&nostr_relay]).unwrap()));

        subscribe();
        Ok(())
    }
}
