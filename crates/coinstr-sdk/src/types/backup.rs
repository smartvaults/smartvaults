// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::fs::File;
use std::io::Error;
use std::io::Read;
use std::io::Write;
use std::path::Path;

use bdk::bitcoin::XOnlyPublicKey;
use bdk::miniscript::Descriptor;
use coinstr_core::util::Serde;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyBackup {
    name: Option<String>,
    description: Option<String>,
    descriptor: Descriptor<String>,
    public_keys: Vec<XOnlyPublicKey>,
}

impl Serde for PolicyBackup {}

impl PolicyBackup {
    pub fn new<S>(
        name: S,
        description: S,
        descriptor: Descriptor<String>,
        public_keys: Vec<XOnlyPublicKey>,
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

    pub fn public_keys(&self) -> Vec<XOnlyPublicKey> {
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
