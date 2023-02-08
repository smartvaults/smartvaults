use bdk::blockchain::EsploraBlockchain;
use bdk::database::MemoryDatabase;
use bdk::wallet::SyncOptions;
use bdk::wallet::Wallet;
use nostr_sdk::client::blocking::Client;
use nostr_sdk::prelude::*;

pub fn get_balance(
    descriptor: &String,
    bitcoin_endpoint: &String,
    bitcoin_network: bitcoin::Network,
) -> bdk::Balance {
    let esplora = EsploraBlockchain::new(&bitcoin_endpoint, 20);

    let wallet = Wallet::new(
        &descriptor.to_string(),
        None,
        bitcoin_network,
        MemoryDatabase::default(),
    )
    .unwrap();

    wallet.sync(&esplora, SyncOptions::default()).unwrap();

    return wallet.get_balance().unwrap();
}

pub fn create_client(keys: &Keys, relays: Vec<String>, difficulty: u8) -> Result<Client> {
    let opts = Options::new().wait_for_send(true).difficulty(difficulty);
    let client = Client::new_with_opts(keys, opts);
    let relays = relays.iter().map(|url| (url, None)).collect();
    client.add_relays(relays)?;
    client.connect();
    Ok(client)
}
