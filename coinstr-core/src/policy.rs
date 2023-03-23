use std::str::FromStr;

use bdk::bitcoin::XOnlyPublicKey;
use bdk::miniscript::descriptor::DescriptorType;
use bdk::miniscript::policy::Concrete;
use bdk::miniscript::Descriptor;

use crate::util::Unspendable;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Miniscript(#[from] bdk::miniscript::Error),
    #[error(transparent)]
    Policy(#[from] bdk::miniscript::policy::compiler::CompilerError),
    #[error("{0}, {1}")]
    DescOrPolicy(Box<Self>, Box<Self>),
    #[error("must be a taproot descriptor")]
    NotTaprootDescriptor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub name: String,
    pub description: String,
    pub descriptor: Descriptor<String>,
}

impl Policy {
    pub fn new<S>(name: S, description: S, descriptor: Descriptor<String>) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        if let DescriptorType::Tr = descriptor.desc_type() {
            Ok(Self {
                name: name.into(),
                description: description.into(),
                descriptor,
            })
        } else {
            Err(Error::NotTaprootDescriptor)
        }
    }

    pub fn from_descriptor<S>(name: S, description: S, descriptor: S) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let descriptor = Descriptor::from_str(&descriptor.into())?;
        Self::new(name, description, descriptor)
    }

    pub fn from_miniscript_policy<S>(name: S, description: S, policy: S) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let policy = Concrete::<String>::from_str(&policy.into())?;
        let unspendable_pk = XOnlyPublicKey::unspendable();
        let descriptor = policy.compile_tr(Some(unspendable_pk.to_string()))?;
        Self::new(name, description, descriptor)
    }

    pub fn from_desc_or_policy<S>(name: S, description: S, desc_or_policy: S) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let name = &name.into();
        let description = &description.into();
        let desc_or_policy = &desc_or_policy.into();
        match Self::from_descriptor(name, description, desc_or_policy) {
            Ok(policy) => Ok(policy),
            Err(desc_e) => match Self::from_miniscript_policy(name, description, desc_or_policy) {
                Ok(policy) => Ok(policy),
                Err(policy_e) => Err(Error::DescOrPolicy(Box::new(desc_e), Box::new(policy_e))),
            },
        }
    }

    /// Deserialize from `JSON` string
    pub fn from_json<S>(json: S) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        Ok(serde_json::from_str(&json.into())?)
    }

    /// Serialize to `JSON` string
    pub fn as_json(&self) -> String {
        serde_json::json!(self).to_string()
    }
}
