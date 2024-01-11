// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use nostr::Metadata;
use serde::{Deserialize, Serialize};

pub const JURISDICTION: &str = "jurisdiction";
pub const X: &str = "x";
pub const FACEBOOK: &str = "facebook";
pub const LINKEDIN: &str = "linkedin";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyAgentMetadata {
    pub jurisdiction: Option<String>,
    pub x: Option<String>,
    pub facebook: Option<String>,
    pub linkedin: Option<String>,
}

impl KeyAgentMetadata {
    pub fn into_metadata(self) -> Metadata {
        let mut metadata = Metadata::new();

        if let Some(jurisdiction) = self.jurisdiction {
            metadata = metadata.custom_field(JURISDICTION, jurisdiction);
        }

        if let Some(x) = self.x {
            metadata = metadata.custom_field(X, x);
        }

        if let Some(facebook) = self.facebook {
            metadata = metadata.custom_field(FACEBOOK, facebook);
        }

        if let Some(linkedin) = self.linkedin {
            metadata = metadata.custom_field(LINKEDIN, linkedin);
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
        key_agent_metadata.jurisdiction = Some(String::from("British Virgin Islands (BVI)"));

        let metadata: Metadata = key_agent_metadata.into_metadata();
        let metadata = metadata.name("keyagent").display_name("Key Agent");
        assert_eq!(metadata.as_json(), String::from("{\"name\":\"keyagent\",\"display_name\":\"Key Agent\",\"jurisdiction\":\"British Virgin Islands (BVI)\"}"))
    }
}
