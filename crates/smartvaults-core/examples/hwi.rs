// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use smartvaults_core::bitcoin::Network;
use smartvaults_core::{hwi, CoreSigner};

#[tokio::main]
async fn main() {
    let devices = hwi::list(Network::Testnet).await.unwrap();
    for device in devices.into_iter() {
        println!("Kind: {}", device.device_kind());
        let signer = CoreSigner::from_hwi(device, Network::Testnet)
            .await
            .unwrap();
        println!("{signer:?}");
    }
}
