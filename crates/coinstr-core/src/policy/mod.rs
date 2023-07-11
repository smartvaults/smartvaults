// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;
use std::str::FromStr;

use bdk::bitcoin::psbt::PartiallySignedTransaction;
use bdk::bitcoin::{Address, Network, XOnlyPublicKey};
use bdk::database::{BatchDatabase, MemoryDatabase};
use bdk::descriptor::policy::SatisfiableItem;
use bdk::miniscript::descriptor::DescriptorType;
use bdk::miniscript::policy::Concrete;
use bdk::miniscript::Descriptor;
use bdk::{FeeRate, KeychainKind, Wallet};
use keechain_core::types::psbt::{self, Psbt};
use serde::{Deserialize, Serialize};

pub mod builder;

use crate::proposal::Proposal;
use crate::reserves::ProofOfReserves;
use crate::util::{Encryption, Serde, Unspendable};
use crate::Amount;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Bdk(#[from] bdk::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Miniscript(#[from] bdk::miniscript::Error),
    #[error(transparent)]
    Psbt(#[from] psbt::Error),
    #[error(transparent)]
    ProofOfReserves(#[from] crate::reserves::ProofError),
    #[error(transparent)]
    Policy(#[from] bdk::miniscript::policy::compiler::CompilerError),
    #[error("{0}, {1}")]
    DescOrPolicy(Box<Self>, Box<Self>),
    #[error("must be a taproot descriptor")]
    NotTaprootDescriptor,
    #[error("wallet spending policy not found")]
    WalletSpendingPolicyNotFound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    pub fn from_policy<S>(name: S, description: S, policy: S) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let policy = Concrete::<String>::from_str(&policy.into())?;
        let unspendable_pk = XOnlyPublicKey::unspendable();
        let descriptor = policy.compile_tr(Some(unspendable_pk.to_string()))?;
        Self::new(name, description, descriptor)
    }

    pub fn from_desc_or_policy<N, D, P>(
        name: N,
        description: D,
        desc_or_policy: P,
    ) -> Result<Self, Error>
    where
        N: Into<String>,
        D: Into<String>,
        P: Into<String>,
    {
        let name = &name.into();
        let description = &description.into();
        let desc_or_policy = &desc_or_policy.into();
        match Self::from_descriptor(name, description, desc_or_policy) {
            Ok(policy) => Ok(policy),
            Err(desc_e) => match Self::from_policy(name, description, desc_or_policy) {
                Ok(policy) => Ok(policy),
                Err(policy_e) => Err(Error::DescOrPolicy(Box::new(desc_e), Box::new(policy_e))),
            },
        }
    }

    pub fn satisfiable_item(&self, network: Network) -> Result<SatisfiableItem, Error> {
        let db = MemoryDatabase::new();
        let wallet = Wallet::new(&self.descriptor.to_string(), None, network, db)?;
        let wallet_policy = wallet
            .policies(KeychainKind::External)?
            .ok_or(Error::WalletSpendingPolicyNotFound)?;
        Ok(wallet_policy.item)
    }

    pub fn spend<D, S>(
        &self,
        wallet: Wallet<D>,
        address: Address,
        amount: Amount,
        description: S,
        fee_rate: FeeRate,
    ) -> Result<Proposal, Error>
    where
        D: BatchDatabase,
        S: Into<String>,
    {
        // Get policies and specify which ones to use
        let wallet_policy = wallet
            .policies(KeychainKind::External)?
            .ok_or(Error::WalletSpendingPolicyNotFound)?;
        let mut path = BTreeMap::new();
        path.insert(wallet_policy.id, vec![1]);

        // Build the PSBT
        let (psbt, details) = {
            let mut builder = wallet.build_tx();
            builder
                .policy_path(path, KeychainKind::External)
                .fee_rate(fee_rate)
                .enable_rbf();
            match amount {
                Amount::Max => builder.drain_wallet().drain_to(address.script_pubkey()),
                Amount::Custom(amount) => builder.add_recipient(address.script_pubkey(), amount),
            };
            builder.finish()?
        };

        let amount: u64 = match amount {
            Amount::Max => {
                let fee: u64 = psbt.fee()?;
                details
                    .sent
                    .saturating_sub(details.received)
                    .saturating_sub(fee)
            }
            Amount::Custom(amount) => amount,
        };

        Ok(Proposal::spending(
            self.descriptor.clone(),
            address,
            amount,
            description,
            psbt,
        ))
    }

    pub fn proof_of_reserve<D, S>(&self, wallet: Wallet<D>, message: S) -> Result<Proposal, Error>
    where
        D: BatchDatabase,
        S: Into<String>,
    {
        let message: &str = &message.into();

        // Get policies and specify which ones to use
        let wallet_policy = wallet
            .policies(KeychainKind::External)?
            .ok_or(Error::WalletSpendingPolicyNotFound)?;
        let mut path = BTreeMap::new();
        path.insert(wallet_policy.id, vec![1]);

        let psbt: PartiallySignedTransaction = wallet.create_proof(message)?;

        Ok(Proposal::proof_of_reserve(
            self.descriptor.clone(),
            message,
            psbt,
        ))
    }
}

impl Serde for Policy {}
impl Encryption for Policy {}
