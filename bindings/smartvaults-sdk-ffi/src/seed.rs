// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use smartvaults_sdk::core::types::seed;

pub struct Seed {
    inner: seed::Seed,
}

impl From<seed::Seed> for Seed {
    fn from(inner: seed::Seed) -> Self {
        Self { inner }
    }
}

impl Seed {
    pub fn mnemonic(&self) -> String {
        self.inner.mnemonic().to_string()
    }

    pub fn passphrase(&self) -> Option<String> {
        self.inner.passphrase()
    }
}
