use bdk::blockchain::ElectrumBlockchain;
use bdk::database::MemoryDatabase;
use bdk::electrum_client::Client as ElectrumClient;
use bdk::miniscript::{Descriptor, DescriptorPublicKey};
use bdk::wallet::{SyncOptions, Wallet};
use nostr_sdk::client::blocking::Client;
use nostr_sdk::prelude::*;

pub fn get_balance(
	descriptor: Descriptor<DescriptorPublicKey>,
	bitcoin_endpoint: String,
	bitcoin_network: Network,
) -> Result<bdk::Balance> {
	let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&bitcoin_endpoint)?);
	let wallet = Wallet::new(descriptor, None, bitcoin_network, MemoryDatabase::default())?;
	wallet.sync(&blockchain, SyncOptions::default())?;
	Ok(wallet.get_balance()?)
}

pub fn create_client(keys: &Keys, relays: Vec<String>, difficulty: u8) -> Result<Client> {
	let opts = Options::new().wait_for_send(true).difficulty(difficulty);
	let client = Client::new_with_opts(keys, opts);
	let relays = relays.iter().map(|url| (url, None)).collect();
	client.add_relays(relays)?;
	client.connect();
	Ok(client)
}
