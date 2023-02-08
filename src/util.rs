use bdk::blockchain::EsploraBlockchain;
use bdk::database::MemoryDatabase;
use bdk::wallet::SyncOptions;
use bdk::wallet::Wallet;

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
