// use nostr_rust::events::{Event, EventPrepare};
// use nostr_rust::nips::nip1::NIP1Error;
// use nostr_rust::utils::get_timestamp;
// use nostr_rust::{nostr_client::Client, Identity};
use nostr::{
    prelude::{FromMnemonic, SecretKey, PublicKey, Secp256k1, FromSkStr},
    Keys, //Result,
};
use nostr_sdk::prelude::*;
use nostr::{
    ClientMessage, EventBuilder, Kind, RelayMessage, Result, SubscriptionFilter,
    SubscriptionId,
};
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};
use crate::users::User;
use clap::Error;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};


fn publish(
    user: &User,
    content: &str,
    tags: &[Vec<String>]) -> Result<()> {
    
    // let keys = Keys::from(user.nostr_secret_hex)?;
    let my_keys = Keys::new(user.nostr_secret_hex);
    let client = Client::new(&my_keys);

    let proxy = Some(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 9050)));

    client.add_relay("wss://relay.damus.io", None);
    client.add_relay("wss://relay.nostr.info", proxy);
    client.add_relay(
        "ws://jgqaglhautb4k6e6i2g34jakxiemqp6z4wynlirltuukgkft2xuglmqd.onion",
        proxy,
    );

    client.connect();

    // TODO: support for tags
    client.publish_text_note(content, &[]);
    Ok(())
    // Handle notifications
    // loop {
    //     let mut notifications = client.notifications();
    //     while let Ok(notification) = notifications.recv() {
    //         println!("{:?}", notification);
    //     }
    // }
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
    pub fn run(&self, _nostr_relay: &String) -> Result<(), Error> {

        let publisher = User::get(&self.user);
        
        // let nostr_client = Arc::new(Mutex::new(Client::new(vec![nostr_relay]).unwrap()));

        publish(&publisher, &self.content, &[]);
        Ok(())
    }
}


#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn publish_foobar() {
        publish(&User::alice().unwrap(), "foobar", &[]);
    }

    #[test]
    fn dump_known_users() {
        for user in User::known_users() {
            println!("{}", user);
        }
    }
}
