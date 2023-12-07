// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use smartvaults_sdk::core::types::{self, seed};
use uniffi::{Enum, Object};

#[derive(Enum)]
pub enum WordCount {
    W12,
    W18,
    W24,
}

impl From<WordCount> for types::WordCount {
    fn from(value: WordCount) -> Self {
        match value {
            WordCount::W12 => Self::W12,
            WordCount::W18 => Self::W18,
            WordCount::W24 => Self::W24,
        }
    }
}

#[derive(Object)]
pub struct Seed {
    inner: seed::Seed,
}

impl From<seed::Seed> for Seed {
    fn from(inner: seed::Seed) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl Seed {
    pub fn mnemonic(&self) -> String {
        self.inner.mnemonic().to_string()
    }

    pub fn passphrase(&self) -> Option<String> {
        self.inner.passphrase()
    }
}
