// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use async_hwi::bitbox::api::runtime;
use async_hwi::bitbox::{BitBox02, PairingBitbox02WithLocalCache};
use async_hwi::ledger::{HidApi, Ledger, LedgerSimulator, TransportHID};
use async_hwi::specter::{Specter, SpecterSimulator};
pub use async_hwi::HWI;
use keechain_core::bitcoin::Network;
use thiserror::Error;

pub type BoxedHWI = Box<dyn HWI + Send>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    HWI(#[from] async_hwi::Error),
    #[error(transparent)]
    HidApi(Box<dyn std::error::Error>),
}

/// Get list of Hardware Wallets connected
pub async fn list(network: Network) -> Result<Vec<BoxedHWI>, Error> {
    let mut hws = Vec::new();

    if let Ok(device) = SpecterSimulator::try_connect().await {
        hws.push(device.into());
    }

    if let Ok(devices) = Specter::enumerate().await {
        for device in devices {
            hws.push(device.into());
        }
    }

    if let Ok(device) = LedgerSimulator::try_connect().await {
        hws.push(device.into());
    }

    let api: HidApi = HidApi::new().map_err(|e| Error::HidApi(Box::new(e)))?;

    for device_info in api.device_list() {
        if async_hwi::bitbox::is_bitbox02(device_info) {
            if let Ok(device) = device_info.open_device(&api) {
                if let Ok(device) =
                    PairingBitbox02WithLocalCache::<runtime::TokioRuntime>::connect(device, None)
                        .await
                {
                    if let Ok((device, _)) = device.wait_confirm().await {
                        let bb02 = BitBox02::from(device).with_network(network);
                        hws.push(bb02.into());
                    }
                }
            }
        }
    }

    for detected in Ledger::<TransportHID>::enumerate(&api) {
        if let Ok(device) = Ledger::<TransportHID>::connect(&api, detected) {
            hws.push(device.into());
        }
    }

    Ok(hws)
}
