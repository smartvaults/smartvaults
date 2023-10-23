// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use nostr::Metadata;
use serde::{Deserialize, Serialize};

pub const JURISDICTION: &str = "jurisdiction";
pub const X: &str = "x";
pub const FACEBOOK: &str = "facebook";
pub const LINKEDIN: &str = "linkedin";
pub const SMARTVAULTS_NIP05: &str = "smartvaults_nip05";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyAgentMetadata {
    pub jurisdiction: Option<String>,
    pub x: Option<String>,
    pub facebook: Option<String>,
    pub linkedin: Option<String>,
    pub smartvaults_nip05: Option<String>,
}

impl KeyAgentMetadata {
    pub fn into_metadata(self) -> Metadata {
        let mut metadata = Metadata::new();

        if let Some(jurisdiction) = self.jurisdiction {
            metadata = metadata.custom_field(JURISDICTION, &jurisdiction);
        }

        if let Some(x) = self.x {
            metadata = metadata.custom_field(X, &x);
        }

        if let Some(facebook) = self.facebook {
            metadata = metadata.custom_field(FACEBOOK, &facebook);
        }

        if let Some(linkedin) = self.linkedin {
            metadata = metadata.custom_field(LINKEDIN, &linkedin);
        }

        if let Some(smartvaults_nip05) = self.smartvaults_nip05 {
            metadata = metadata.custom_field(SMARTVAULTS_NIP05, &smartvaults_nip05);
        }

        metadata
    }
}

#[cfg(test)]
mod tests {
    use nostr::JsonUtil;

    use super::*;

    #[test]
    fn test_key_agent_metadata() {
        let mut key_agent_metadata: KeyAgentMetadata = KeyAgentMetadata::default();
        key_agent_metadata.smartvaults_nip05 = Some(String::from("agent@smartvaults.app"));

        let metadata: Metadata = key_agent_metadata.into_metadata();
        let metadata = metadata.name("keyagent").display_name("Key Agent");
        assert_eq!(metadata.as_json(), String::from("{\"display_name\":\"Key Agent\",\"name\":\"keyagent\",\"smartvaults_nip05\":\"agent@smartvaults.app\"}"))
    }
}
