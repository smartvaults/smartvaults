use serde::{Deserialize, Serialize};
use smartvaults_core::miniscript::Descriptor;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Vault {
    pub name: String,
    pub description: String,
    pub descriptor: Descriptor<String>,
}
