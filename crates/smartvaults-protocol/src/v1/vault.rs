// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::ops::Deref;

use serde::de::Error as DeserializerError;
use serde::{Deserialize, Deserializer, Serialize};
use smartvaults_core::bitcoin::Network;
use smartvaults_core::miniscript::Descriptor;
use smartvaults_core::util::search_network_for_descriptor;
use smartvaults_core::{policy, Policy, PolicyTemplate};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Vault {
    pub name: String,
    pub description: String,
    policy: Policy,
}

impl Deref for Vault {
    type Target = Policy;

    fn deref(&self) -> &Self::Target {
        &self.policy
    }
}

impl Vault {
    pub fn new<N, D, P>(
        name: N,
        description: D,
        descriptor: P,
        network: Network,
    ) -> Result<Self, policy::Error>
    where
        N: Into<String>,
        D: Into<String>,
        P: AsRef<str>,
    {
        let policy: Policy = Policy::from_desc_or_miniscript(descriptor, network)?;
        Ok(Self {
            name: name.into(),
            description: description.into(),
            policy,
        })
    }

    pub fn from_template<S>(
        name: S,
        description: S,
        template: PolicyTemplate,
        network: Network,
    ) -> Result<Self, policy::Error>
    where
        S: Into<String>,
    {
        let policy: Policy = Policy::from_template(template, network)?;
        Ok(Self {
            name: name.into(),
            description: description.into(),
            policy,
        })
    }

    pub fn policy(&self) -> Policy {
        self.policy.clone()
    }
}

#[derive(Serialize, Deserialize)]
struct VaultIntermediate {
    name: String,
    description: String,
    descriptor: Descriptor<String>,
}

impl Serialize for Vault {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let intermediate = VaultIntermediate {
            name: self.name.clone(),
            description: self.description.clone(),
            descriptor: self.policy.descriptor(),
        };
        intermediate.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Vault {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let intermediate: VaultIntermediate = VaultIntermediate::deserialize(deserializer)?;
        let network: Network = search_network_for_descriptor(&intermediate.descriptor)
            .ok_or(DeserializerError::custom("Network not found"))?;
        Ok(Self {
            name: intermediate.name,
            description: intermediate.description,
            policy: Policy::new(intermediate.descriptor.clone(), network)
                .map_err(DeserializerError::custom)?,
        })
    }
}

#[cfg(bench)]
mod benches {
    use core::str::FromStr;

    use nostr::secp256k1::SecretKey;
    use nostr::Keys;
    use test::{black_box, Bencher};

    use super::*;
    use crate::v1::Encryption;

    const NETWORK: Network = Network::Testnet;
    const SECRET_KEY: &str = "6b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b718e";

    #[bench]
    pub fn encrypt_vault(bh: &mut Bencher) {
        let desc = "tr(c0e6675756101c53287237945c4ed0fbb780b20c5ca6e36b4178ac89075f629c,multi_a(2,[7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*,[4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*))#ccsgt5j5";
        let vault = Vault::new("", "", desc, NETWORK).unwrap();

        let secret_key = SecretKey::from_str(SECRET_KEY).unwrap();
        let keys = Keys::new(secret_key);

        bh.iter(|| {
            black_box(vault.encrypt_with_keys(&keys)).unwrap();
        });
    }

    #[bench]
    pub fn decrypt_vault(bh: &mut Bencher) {
        let encrypted_vault = "c+7q23PwxEQfxfIf09qHkZjbWPjiJZVMGV6ZBLl7v/Qy57qQJvyrR2FTKYrtIDfrLEeGr1dnsXWdrocxn9f9KK49TzNNHnpQkxRauvn125itkBe9TqHnJDfqJkz0AD2G9/JF2NEqPy/feTgk6F1eRueThWJOo612RCt37P4c+XMNBJBH4ohh/sHuSJ0XnG5irQZJkYXNUxNeRcghQFlnFvgyNjGUcti8aysjel0twqjDhYbUXMuZbZ3dUT2soiUcgHyKG8KZLiIvHo3kY27cVGoUymDX5pGTm+HtGSyOwso1cgFPWnr8xO/2BZRd7x7gvIhYXrPIhZuEobmVM5CXPaySUKhIsOs0Pc/Ely9mFtrxS+UXySNgtZfSzQAj3cSBUWquW6BEWT7++YEW/t4AhFmNiCC4he7E9zWaspvTSqINxIVzi0KyF3JP9fhAqVtVWzUcxxyb+NkbuFW+4AT8zz1FBi9RwOURB4RANvVwPjs2vzJOdVqBIkn1FP6pgkvEJe3lawYD2qYwoi3TKJ8ksGzqIIsXYLB/Tlj8g/UJt5c=?iv=uB8kUyMw11W7yN8opa/sIw==";

        let secret_key = SecretKey::from_str(SECRET_KEY).unwrap();
        let keys = Keys::new(secret_key);

        bh.iter(|| {
            black_box(Vault::decrypt_with_keys(&keys, encrypted_vault)).unwrap();
        });
    }
}
