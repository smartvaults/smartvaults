// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::fs::File;
use std::io::Error;
use std::io::Write;
use std::path::Path;

use bdk::bitcoin::XOnlyPublicKey;
use bdk::miniscript::Descriptor;
use coinstr_core::util::Serde;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyBackup {
    descriptor: Descriptor<String>,
    public_keys: Vec<XOnlyPublicKey>,
}

impl Serde for PolicyBackup {}

impl PolicyBackup {
    pub fn new(descriptor: Descriptor<String>, public_keys: Vec<XOnlyPublicKey>) -> Self {
        Self {
            descriptor,
            public_keys,
        }
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
