// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::BTreeMap;
use std::ops::Deref;
use std::str::FromStr;

use smartvaults_core::bips::bip48::ScriptType;
use smartvaults_core::bitcoin::bip32::Fingerprint;
use smartvaults_core::{ColdcardGenericJson, CoreSigner, Purpose, SignerType};
use wasm_bindgen::prelude::*;

use crate::descriptor::JsDescriptorPublicKey;
use crate::error::{into_err, Result};
use crate::network::JsNetwork;

#[wasm_bindgen(js_name = Purpose)]
pub enum JsPurpose {
    /// BIP44 - P2PKH
    ///
    /// <https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki>
    BIP44,
    /// BIP48 - P2SHWSH
    ///
    /// <https://github.com/bitcoin/bips/blob/master/bip-0048.mediawiki>
    BIP48_1,
    /// BIP48 - P2WSH
    ///
    /// <https://github.com/bitcoin/bips/blob/master/bip-0048.mediawiki>
    BIP48_2,
    /// BIP48 - P2TR
    ///
    /// <https://github.com/bitcoin/bips/blob/master/bip-0048.mediawiki>
    BIP48_3,
    /// BIP49 - P2SH-WPKH
    ///
    /// <https://github.com/bitcoin/bips/blob/master/bip-0049.mediawiki>
    BIP49,
    /// BIP84 - P2WPKH
    ///
    /// <https://github.com/bitcoin/bips/blob/master/bip-0084.mediawiki>
    BIP84,
    /// BIP86 - P2TR
    ///
    /// <https://github.com/bitcoin/bips/blob/master/bip-0086.mediawiki>
    BIP86,
}

impl From<JsPurpose> for Purpose {
    fn from(value: JsPurpose) -> Self {
        match value {
            JsPurpose::BIP44 => Self::BIP44,
            JsPurpose::BIP48_1 => Self::BIP48 {
                script: ScriptType::P2SHWSH,
            },
            JsPurpose::BIP48_2 => Self::BIP48 {
                script: ScriptType::P2WSH,
            },
            JsPurpose::BIP48_3 => Self::BIP48 {
                script: ScriptType::P2TR,
            },
            JsPurpose::BIP49 => Self::BIP49,
            JsPurpose::BIP84 => Self::BIP84,
            JsPurpose::BIP86 => Self::BIP86,
        }
    }
}

#[wasm_bindgen(js_name = SignerType)]
pub enum JsSignerType {
    /// Seed
    Seed,
    /// Signing Device (aka Hardware Wallet) that can be used
    /// with USB, Bluetooth or other that provides a direct connection with the wallet.
    Hardware,
    /// Signing Device that can be used without ever being connected
    /// to online devices, via microSD or camera.
    AirGap,
    /// Unknown signer type
    Unknown,
}

impl From<SignerType> for JsSignerType {
    fn from(value: SignerType) -> Self {
        match value {
            SignerType::Seed => Self::Seed,
            SignerType::Hardware => Self::Hardware,
            SignerType::AirGap => Self::AirGap,
            SignerType::Unknown => Self::Unknown,
        }
    }
}

impl From<JsSignerType> for SignerType {
    fn from(value: JsSignerType) -> Self {
        match value {
            JsSignerType::Seed => Self::Seed,
            JsSignerType::Hardware => Self::Hardware,
            JsSignerType::AirGap => Self::AirGap,
            JsSignerType::Unknown => Self::Unknown,
        }
    }
}

#[wasm_bindgen(js_name = ColdcardGenericJson)]
pub struct JsColdcardGenericJson {
    inner: ColdcardGenericJson,
}

impl Deref for JsColdcardGenericJson {
    type Target = ColdcardGenericJson;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[wasm_bindgen(js_class = ColdcardGenericJson)]
impl JsColdcardGenericJson {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<JsColdcardGenericJson> {
        Ok(Self {
            inner: ColdcardGenericJson::from_json(json).map_err(into_err)?,
        })
    }
}

#[wasm_bindgen(js_name = CoreSigner)]
pub struct JsCoreSigner {
    inner: CoreSigner,
}

impl Deref for JsCoreSigner {
    type Target = CoreSigner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<CoreSigner> for JsCoreSigner {
    fn from(inner: CoreSigner) -> Self {
        Self { inner }
    }
}

impl From<JsCoreSigner> for CoreSigner {
    fn from(value: JsCoreSigner) -> Self {
        value.inner
    }
}

#[wasm_bindgen(js_class = CoreSigner)]
impl JsCoreSigner {
    /// Create new **empty** signer (without descriptors)
    ///
    /// Add descriptors with `addDescriptor` method
    pub fn empty(
        fingerprint: &str,
        signer_type: JsSignerType,
        network: JsNetwork,
    ) -> Result<JsCoreSigner> {
        let fingerprint = Fingerprint::from_str(fingerprint).map_err(into_err)?;
        Ok(Self {
            inner: CoreSigner::new(fingerprint, BTreeMap::new(), signer_type.into(), network.into())
                .map_err(into_err)?,
        })
    }

    /// Compose `CoreSigner` from Coldcard generic JSON (`coldcard-export.json`)
    #[wasm_bindgen(js_name = fromColdcard)]
    pub fn from_coldcard(
        coldcard: &JsColdcardGenericJson,
        network: JsNetwork,
    ) -> Result<JsCoreSigner> {
        Ok(Self {
            inner: CoreSigner::from_coldcard(coldcard.deref(), network.into()).map_err(into_err)?,
        })
    }

    #[wasm_bindgen(js_name = signerType)]
    pub fn signer_type(&self) -> JsSignerType {
        self.inner.r#type().into()
    }

    pub fn network(&self) -> JsNetwork {
        self.inner.network().into()
    }

    #[wasm_bindgen(js_name = addDescriptor)]
    pub fn add_descriptor(
        &mut self,
        purpose: JsPurpose,
        descriptor: &JsDescriptorPublicKey,
    ) -> Result<()> {
        self.inner
            .add_descriptor(purpose.into(), descriptor.deref().clone())
            .map_err(into_err)
    }
}
