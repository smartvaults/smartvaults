// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::fs::File;
use std::io::{Error, Read, Write};
use std::path::Path;

use nostr_sdk::PublicKey;
use serde::{Deserialize, Serialize};
use smartvaults_core::miniscript::Descriptor;
use smartvaults_protocol::v1::util::Serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyBackup {
    name: Option<String>,
    description: Option<String>,
    descriptor: Descriptor<String>,
    public_keys: Vec<PublicKey>,
}

impl Serde for PolicyBackup {}

impl PolicyBackup {
    pub fn new<S>(
        name: S,
        description: S,
        descriptor: Descriptor<String>,
        public_keys: Vec<PublicKey>,
    ) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: Some(name.into()),
            description: Some(description.into()),
            descriptor,
            public_keys,
        }
    }

    pub fn open<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let mut file = File::open(path)?;
        let mut json = String::new();
        file.read_to_string(&mut json)?;
        Ok(Self::from_json(json)?)
    }

    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    pub fn descriptor(&self) -> Descriptor<String> {
        self.descriptor.clone()
    }

    pub fn public_keys(&self) -> Vec<PublicKey> {
        self.public_keys.clone()
    }

    pub fn save<P>(&self, path: P) -> Result<(), Error>
    where
        P: AsRef<Path>,
    {
        let mut file = File::create(path)?;
        file.write_all(self.as_json().as_bytes())?;
        Ok(())
    }
}
