// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;
use std::str::FromStr;

use bdk::chain::PersistBackend;
use bdk::descriptor::policy::SatisfiableItem;
use bdk::descriptor::Policy as SpendingPolicy;
use bdk::wallet::ChangeSet;
use bdk::{FeeRate, KeychainKind, Wallet};
use keechain_core::bitcoin::address::NetworkUnchecked;
use keechain_core::bitcoin::psbt::PartiallySignedTransaction;
use keechain_core::bitcoin::{Address, Network, OutPoint};
use keechain_core::miniscript::descriptor::DescriptorType;
use keechain_core::miniscript::policy::Concrete;
use keechain_core::miniscript::{Descriptor, DescriptorPublicKey};
use keechain_core::secp256k1::XOnlyPublicKey;
use serde::{Deserialize, Serialize};

pub mod template;

pub use self::template::{PolicyTemplate, PolicyTemplateType, RecoveryTemplate};
use crate::proposal::Proposal;
use crate::reserves::ProofOfReserves;
use crate::util::serde::SerdeSer;
use crate::util::{Encryption, Serde, Unspendable};
use crate::{Amount, SECP256K1};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Bdk(#[from] bdk::Error),
    #[error(transparent)]
    BdkDescriptor(#[from] bdk::descriptor::DescriptorError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Miniscript(#[from] keechain_core::miniscript::Error),
    #[error(transparent)]
    Psbt(#[from] keechain_core::bitcoin::psbt::Error),
    #[error(transparent)]
    ProofOfReserves(#[from] crate::reserves::ProofError),
    #[error(transparent)]
    Policy(#[from] keechain_core::miniscript::policy::compiler::CompilerError),
    #[error(transparent)]
    Template(#[from] template::Error),
    #[error("{0}, {1}")]
    DescOrPolicy(Box<Self>, Box<Self>),
    #[error("must be a taproot descriptor")]
    NotTaprootDescriptor,
    #[error("wallet spending policy not found")]
    WalletSpendingPolicyNotFound,
}

impl SerdeSer for SpendingPolicy {}
impl SerdeSer for SatisfiableItem {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    pub name: String,
    pub description: String,
    pub descriptor: Descriptor<String>,
}

impl Policy {
    pub fn new<S>(
        name: S,
        description: S,
        descriptor: Descriptor<String>,
        network: Network,
    ) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        if let DescriptorType::Tr = descriptor.desc_type() {
            Wallet::new_no_persist(&descriptor.to_string(), None, network)?;
            Ok(Self {
                name: name.into(),
                description: description.into(),
                descriptor,
            })
        } else {
            Err(Error::NotTaprootDescriptor)
        }
    }

    pub fn from_descriptor<S>(
        name: S,
        description: S,
        descriptor: S,
        network: Network,
    ) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let descriptor: Descriptor<String> = Descriptor::from_str(&descriptor.into())?;
        Self::new(name, description, descriptor, network)
    }

    pub fn from_policy<S>(
        name: S,
        description: S,
        policy: S,
        network: Network,
    ) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let policy: Concrete<String> = Concrete::<String>::from_str(&policy.into())?;
        let unspendable_pk: XOnlyPublicKey = XOnlyPublicKey::unspendable(&SECP256K1);
        let descriptor: Descriptor<String> = policy.compile_tr(Some(unspendable_pk.to_string()))?;
        Self::new(name, description, descriptor, network)
    }

    pub fn from_desc_or_policy<N, D, P>(
        name: N,
        description: D,
        desc_or_policy: P,
        network: Network,
    ) -> Result<Self, Error>
    where
        N: Into<String>,
        D: Into<String>,
        P: Into<String>,
    {
        let name: &str = &name.into();
        let description: &str = &description.into();
        let desc_or_policy: &str = &desc_or_policy.into();
        match Self::from_descriptor(name, description, desc_or_policy, network) {
            Ok(policy) => Ok(policy),
            Err(desc_e) => match Self::from_policy(name, description, desc_or_policy, network) {
                Ok(policy) => Ok(policy),
                Err(policy_e) => Err(Error::DescOrPolicy(Box::new(desc_e), Box::new(policy_e))),
            },
        }
    }

    pub fn from_template<S>(
        name: S,
        description: S,
        template: PolicyTemplate,
        network: Network,
    ) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let policy: Concrete<DescriptorPublicKey> = template.build()?;
        Self::from_policy(name.into(), description.into(), policy.to_string(), network)
    }

    pub fn spending_policy(&self, network: Network) -> Result<SpendingPolicy, Error> {
        let wallet = Wallet::new_no_persist(&self.descriptor.to_string(), None, network)?;
        wallet
            .policies(KeychainKind::External)?
            .ok_or(Error::WalletSpendingPolicyNotFound)
    }

    pub fn satisfiable_item(&self, network: Network) -> Result<SatisfiableItem, Error> {
        let policy = self.spending_policy(network)?;
        Ok(policy.item)
    }

    pub fn selectable_conditions(
        &self,
        network: Network,
    ) -> Result<Vec<(String, Vec<String>)>, Error> {
        fn selectable_conditions(
            item: &SatisfiableItem,
            prev_id: String,
            result: &mut Vec<(String, Vec<String>)>,
        ) {
            if let SatisfiableItem::Thresh { items, threshold } = item {
                if *threshold < items.len() {
                    result.push((prev_id, items.iter().map(|i| i.id.clone()).collect()));
                }

                for x in items.iter() {
                    selectable_conditions(&x.item, x.id.clone(), result);
                }
            }
        }

        let item = self.satisfiable_item(network)?;
        let mut result = Vec::new();
        selectable_conditions(&item, item.id(), &mut result);
        Ok(result)
    }

    /// Check if [`Policy`] match any [`PolicyTemplateType`]
    pub fn template_match(&self, network: Network) -> Result<Option<PolicyTemplateType>, Error> {
        if let SatisfiableItem::Thresh { items, threshold } = self.satisfiable_item(network)? {
            if threshold == 1 && items.len() == 2 {
                if let SatisfiableItem::SchnorrSignature(..) = items[0].item {
                    match &items[1].item {
                        // Multisig 1 of 2 or N of M templates
                        SatisfiableItem::SchnorrSignature(..)
                        | SatisfiableItem::Multisig { .. } => {
                            return Ok(Some(PolicyTemplateType::Multisig))
                        }
                        SatisfiableItem::Thresh { items, threshold } => {
                            if *threshold == 2 && items.len() == 2 {
                                match items[0].item {
                                    // Hold template
                                    SatisfiableItem::SchnorrSignature(..) => {
                                        if let SatisfiableItem::AbsoluteTimelock { .. } =
                                            items[1].item
                                        {
                                            return Ok(Some(PolicyTemplateType::Hold));
                                        }
                                    }
                                    // Recovery templates
                                    SatisfiableItem::Multisig { .. } => match items[1].item {
                                        // Social Recovery
                                        SatisfiableItem::AbsoluteTimelock { .. } => {
                                            return Ok(Some(PolicyTemplateType::SocialRecovery))
                                        }
                                        // Inheritance
                                        SatisfiableItem::RelativeTimelock { .. } => {
                                            return Ok(Some(PolicyTemplateType::Inheritance))
                                        }
                                        _ => (),
                                    },
                                    _ => (),
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
        }

        Ok(None)
    }

    /// Check if [`Policy`] has a timelock
    pub fn has_timelock(&self) -> bool {
        let descriptor = self.descriptor.to_string();
        descriptor.contains("after") || descriptor.contains("older")
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spend<D, S>(
        &self,
        wallet: &mut Wallet<D>,
        address: Address<NetworkUnchecked>,
        amount: Amount,
        description: S,
        fee_rate: FeeRate,
        utxos: Option<Vec<OutPoint>>,
        policy_path: Option<BTreeMap<String, Vec<usize>>>,
    ) -> Result<Proposal, Error>
    where
        D: PersistBackend<ChangeSet>,
        S: Into<String>,
    {
        // Build the PSBT
        let (psbt, details) = {
            let mut builder = wallet.build_tx();

            if let Some(path) = policy_path {
                builder.policy_path(path, KeychainKind::External);
            }

            if let Some(utxos) = utxos {
                builder.manually_selected_only();
                builder.add_utxos(&utxos)?;
            }

            builder.fee_rate(fee_rate).enable_rbf();
            match amount {
                Amount::Max => builder
                    .drain_wallet()
                    .drain_to(address.payload.script_pubkey()),
                Amount::Custom(amount) => {
                    builder.add_recipient(address.payload.script_pubkey(), amount)
                }
            };
            builder.finish()?
        };

        let amount: u64 = match amount {
            Amount::Max => {
                let fee: u64 = psbt.fee()?.to_sat();
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

    pub fn proof_of_reserve<D, S>(
        &self,
        wallet: &mut Wallet<D>,
        message: S,
    ) -> Result<Proposal, Error>
    where
        D: PersistBackend<ChangeSet>,
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

#[cfg(test)]
mod test {
    use super::*;

    const NETWORK: Network = Network::Testnet;

    #[test]
    fn test_policy() {
        let policy = "thresh(2,pk([87131a00/86'/1'/784923']tpubDDEaK5JwGiGDTRkML9YKh8AF4rHPhkpnXzVjVMDBtzayJpnsWKeiFPxtiyYeGHQj8pnjsei7N98winwZ3ivGoVVKArZVMsEYGig73XVqbSX/0/*),pk([e157a520/86'/1'/784923']tpubDCCYFYCyDkxo1xAzDpoFNdtGcjD5BPLZbEJswjJmwqp67Weqd2C7fg6Jy1SBjgn3wYnKyUtoYKXG4VdQczjqb6FJnqHe3NmFdgy8vNBSty4/0/*))";
        assert!(Policy::from_policy("", "", policy, NETWORK).is_ok());
    }

    #[test]
    fn test_wrong_policy() {
        let policy = "thresh(2,pk([7c997e72/86'/0'/784923']xpub6DGQCZUmD4kdGDj8ttgba5Jc6pUSkFWaMwB1jedmzer1BtKDdef18k3cWwC9k7HfJGci7Q9S5KTRD9bBn4JZm3xPcDvidkSXvZ6pg4now57/0/),pk([87131a00/86'/1'/784923']tpubDDEaK5JwGiGDTRkML9YKh8AF4rHPhkpnXzVjVMDBtzayJpnsWKeiFPxtiyYeGHQj8pnjsei7N98winwZ3ivGoVVKArZVMsEYGig73XVqbSX/0/),pk([e157a520/86'/1'/784923']tpubDCCYFYCyDkxo1xAzDpoFNdtGcjD5BPLZbEJswjJmwqp67Weqd2C7fg6Jy1SBjgn3wYnKyUtoYKXG4VdQczjqb6FJnqHe3NmFdgy8vNBSty4/0/))";
        assert!(Policy::from_policy("", "", policy, NETWORK).is_err());
    }

    #[test]
    fn test_descriptor() {
        let descriptor = "tr([9bf4354b/86'/1'/784923']tpubDCT8uwnkZj7woaY71Xr5hU7Wvjr7B1BXJEpwMzzDLd1H6HLnKTiaLPtt6ZfEizDMwdQ8PT8JCmKbB4ESVXTkCzv51oxhJhX5FLBvkeN9nJ3/0/*,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*))#rs0udsfg";
        assert!(Policy::from_descriptor("", "", descriptor, NETWORK).is_ok())
    }

    #[test]
    fn test_wrong_descriptor() {
        let descriptor = "tr(939742dc67dd3c5b5c9201df54ee8a92b053b2613770c8c26f2156cfd9514a0b,multi_a(2,[7c997e72/86'/0'/784923']xpub6DGQCZUmD4kdGDj8ttgba5Jc6pUSkFWaMwB1jedmzer1BtKDdef18k3cWwC9k7HfJGci7Q9S5KTRD9bBn4JZm3xPcDvidkSXvZ6pg4now57/0/,[87131a00/86'/1'/784923']tpubDDEaK5JwGiGDTRkML9YKh8AF4rHPhkpnXzVjVMDBtzayJpnsWKeiFPxtiyYeGHQj8pnjsei7N98winwZ3ivGoVVKArZVMsEYGig73XVqbSX/0/,[e157a520/86'/1'/784923']tpubDCCYFYCyDkxo1xAzDpoFNdtGcjD5BPLZbEJswjJmwqp67Weqd2C7fg6Jy1SBjgn3wYnKyUtoYKXG4VdQczjqb6FJnqHe3NmFdgy8vNBSty4/0/))#kdvl4ku3";
        assert!(Policy::from_descriptor("", "", descriptor, NETWORK).is_err())
    }

    #[test]
    fn test_descriptor_with_wrong_network() {
        let descriptor = "tr([9bf4354b/86'/1'/784923']tpubDCT8uwnkZj7woaY71Xr5hU7Wvjr7B1BXJEpwMzzDLd1H6HLnKTiaLPtt6ZfEizDMwdQ8PT8JCmKbB4ESVXTkCzv51oxhJhX5FLBvkeN9nJ3/0/*,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*))#rs0udsfg";
        assert!(Policy::from_descriptor("", "", descriptor, Network::Bitcoin).is_err())
    }

    #[test]
    fn selectable_conditions() {
        let desc: &str = "tr([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*,and_v(v:pk([f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*),andor(pk([f57a6b99/86'/1'/784923']tpubDC45v32EZGP2U4qVTKayC3kkdKmFAFDxxA7wnCCVgUuPXRFNms1W1LZq2LiCUBk5XmNvTZcEtbexZUMtY4ubZGS74kQftEGibUxUpybMan7/0/*),older(52000),multi_a(2,[4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*,[8cab67b4/86'/1'/784923']tpubDC6N2TsKj5zdHzqU17wnQMHsD1BdLVue3bkk2a2BHnVHoTvhX2JdKGgnMwRiMRVVs3art21SusorgGxXoZN54JhXNQ7KoJsHLTR6Kvtu7Ej/0/*))))#auurkhk6";
        let policy = Policy::from_descriptor("", "", desc, Network::Testnet).unwrap();
        let conditions = policy.selectable_conditions(Network::Testnet).unwrap();
        let mut c = Vec::new();
        c.push((
            String::from("y46gds64"),
            vec![String::from("lcjxl004"), String::from("8sld2cgj")],
        ));
        c.push((
            String::from("fx0z8u06"),
            vec![String::from("0e36xhlc"), String::from("m4n7s285")],
        ));

        assert_eq!(conditions, c)
    }

    #[test]
    fn test_policy_template_match() {
        let multisig_1_of_2 = "thresh(1,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*))";
        let policy = Policy::from_policy("Multisig 1 of 2", "", multisig_1_of_2, NETWORK).unwrap();
        assert_eq!(
            policy.template_match(NETWORK).unwrap(),
            Some(PolicyTemplateType::Multisig)
        );

        let multisig_2_of_2 = "thresh(2,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*))";
        let policy = Policy::from_policy("Multisig 2 of 2", "", multisig_2_of_2, NETWORK).unwrap();
        assert_eq!(
            policy.template_match(NETWORK).unwrap(),
            Some(PolicyTemplateType::Multisig)
        );

        let social_recovery = "or(1@pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),1@and(thresh(2,pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*),pk([f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*)),after(10000)))";
        let policy = Policy::from_policy("Social Recovery", "", social_recovery, NETWORK).unwrap();
        assert_eq!(
            policy.template_match(NETWORK).unwrap(),
            Some(PolicyTemplateType::SocialRecovery)
        );

        let inheritance = "or(1@pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),1@and(thresh(2,pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*),pk([f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*)),older(6)))";
        let policy = Policy::from_policy("Inheritance", "", inheritance, NETWORK).unwrap();
        assert_eq!(
            policy.template_match(NETWORK).unwrap(),
            Some(PolicyTemplateType::Inheritance)
        );

        let hold = "and(pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),after(10000))";
        let policy = Policy::from_policy("Hold", "", hold, NETWORK).unwrap();
        assert_eq!(
            policy.template_match(NETWORK).unwrap(),
            Some(PolicyTemplateType::Hold)
        );
    }
}
