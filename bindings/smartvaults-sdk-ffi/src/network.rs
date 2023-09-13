// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use smartvaults_sdk::core::bitcoin::Network as NetworkSdk;

pub enum Network {
    Bitcoin,
    Testnet,
    Signet,
    Regtest,
}

impl From<NetworkSdk> for Network {
    fn from(value: NetworkSdk) -> Self {
        match value {
            NetworkSdk::Bitcoin => Self::Bitcoin,
            NetworkSdk::Testnet => Self::Testnet,
            NetworkSdk::Signet => Self::Signet,
            _ => Self::Regtest,
        }
    }
}

impl From<Network> for NetworkSdk {
    fn from(value: Network) -> Self {
        match value {
            Network::Bitcoin => Self::Bitcoin,
            Network::Testnet => Self::Testnet,
            Network::Signet => Self::Signet,
            Network::Regtest => Self::Regtest,
        }
    }
}
