use nostr_sdk::client::blocking::Client;
use nostr_sdk::prelude::*;

use crate::users::User;
use clap::Error;

fn publish(
    user: &User,
    content: &str,
    _tags: &[Vec<String>]) -> Result<()> {
    
    let my_keys = Keys::new(user.nostr_secret_hex);
    let client = Client::new(&my_keys);

    client.add_relay("ws://127.0.0.1:8081", None)?;
    client.connect();

    // TODO: support for tags
    client.publish_text_note(content, &[])?;

    Ok(())
 }

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
    pub fn run(&self, _nostr_relay: &String) -> Result<(), Error> {

        let publisher = User::get(&self.user);
        
        publish(&publisher, &self.content, &[]).expect("Unable to publish note");
        Ok(())
    }
}


#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn publish_foobar() {
        publish(&User::bob().unwrap(), "foobar", &[]).expect("Unable to publish from test");
    }
}
