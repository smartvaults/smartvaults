// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::str::FromStr;
use std::collections::{BTreeMap, HashMap, HashSet};

use bdk::chain::{ConfirmationTime, PersistBackend};
use bdk::descriptor::policy::{BuildSatisfaction, PkOrF, SatisfiableItem};
use bdk::descriptor::{ExtractPolicy, IntoWalletDescriptor, Policy as SpendingPolicy};
use bdk::signer::SignersContainer;
use bdk::wallet::tx_builder::AddUtxoError;
use bdk::wallet::ChangeSet;
use bdk::{FeeRate, KeychainKind, LocalOutput, Wallet};
use keechain_core::bitcoin::absolute::{self, Height, Time};
use keechain_core::bitcoin::address::NetworkUnchecked;
use keechain_core::bitcoin::bip32::Fingerprint;
#[cfg(feature = "reserves")]
use keechain_core::bitcoin::psbt::PartiallySignedTransaction;
use keechain_core::bitcoin::{Address, Network, OutPoint};
use keechain_core::miniscript::descriptor::DescriptorType;
use keechain_core::miniscript::policy::Concrete;
use keechain_core::miniscript::Descriptor;
use keechain_core::secp256k1::XOnlyPublicKey;
use keechain_core::util::time;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub mod template;

use self::template::PolicyTemplateResult;
pub use self::template::{
    AbsoluteLockTime, DecayingTime, Locktime, PolicyTemplate, PolicyTemplateType, RecoveryTemplate,
    Sequence,
};
use crate::proposal::Proposal;
#[cfg(feature = "reserves")]
use crate::reserves::ProofOfReserves;
use crate::util::{search_network_for_descriptor, Unspendable};
use crate::{Amount, Signer, SECP256K1};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    BdkAddUtxo(#[from] AddUtxoError),
    #[error("{0}")]
    BdkCreateTx(String),
    #[error(transparent)]
    BdkDescriptor(#[from] bdk::descriptor::DescriptorError),
    #[error(transparent)]
    Miniscript(#[from] keechain_core::miniscript::Error),
    #[error(transparent)]
    AbsoluteTimelock(#[from] absolute::Error),
    #[error(transparent)]
    Psbt(#[from] keechain_core::bitcoin::psbt::Error),
    #[cfg(feature = "reserves")]
    #[error(transparent)]
    ProofOfReserves(#[from] crate::reserves::ProofError),
    #[error(transparent)]
    Signer(#[from] crate::signer::Error),
    #[error(transparent)]
    Policy(#[from] keechain_core::miniscript::policy::compiler::CompilerError),
    #[error(transparent)]
    Template(#[from] template::Error),
    #[error("{0}, {1}")]
    DescOrPolicy(Box<Self>, Box<Self>),
    #[error("must be a taproot descriptor")]
    NotTaprootDescriptor,
    #[error("spending policy not found")]
    SpendingPolicyNotFound,
    #[error("no utxos selected")]
    NoUtxosSelected,
    #[error("No UTXOs available: {0}")]
    NoUtxosAvailable(String),
    #[error("Absolute timelock not satisfied")]
    AbsoluteTimelockNotSatisfied,
    #[error("Relative timelock not satisfied")]
    RelativeTimelockNotSatisfied,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SelectableCondition {
    pub path: String,
    pub thresh: usize,
    pub sub_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyPathSelector {
    Complete {
        path: BTreeMap<String, Vec<usize>>,
    },
    Partial {
        selected_path: BTreeMap<String, Vec<usize>>,
        missing_to_select: BTreeMap<String, Vec<String>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyPath {
    Single(PolicyPathSelector),
    Multiple(HashMap<Signer, Option<PolicyPathSelector>>),
    None,
}

#[derive(Serialize, Deserialize)]
struct PolicyItermediate {
    name: String,
    description: String,
    descriptor: Descriptor<String>,
}

impl Serialize for Policy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let intermediate = PolicyItermediate {
            name: self.name.clone(),
            description: self.description.clone(),
            descriptor: self.descriptor.clone(),
        };
        intermediate.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Policy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let PolicyItermediate {
            name,
            description,
            descriptor,
        } = PolicyItermediate::deserialize(deserializer)?;
        let network: Network = search_network_for_descriptor(&descriptor)
            .ok_or(serde::de::Error::custom("Network not found"))?;
        Self::new(name, description, descriptor, network).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone)]
pub struct Policy {
    name: String,
    description: String,
    /// Descriptor
    descriptor: Descriptor<String>,
    /// Spending policy
    spending_policy: Option<SpendingPolicy>,
    /// network
    network: Network,
}

impl PartialEq for Policy {
    fn eq(&self, other: &Self) -> bool {
        self.descriptor == other.descriptor && self.network == other.network
    }
}

impl Eq for Policy {}

impl PartialOrd for Policy {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Policy {
    fn cmp(&self, other: &Self) -> Ordering {
        self.descriptor.cmp(&other.descriptor)
    }
}

impl Hash for Policy {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.descriptor.hash(state);
    }
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
            // Check if descriptor match network
            let desc: String = descriptor.to_string();
            let (descriptor_public_key, keymap) =
                desc.into_wallet_descriptor(&SECP256K1, network)?;

            // Get spending policy
            let signer = SignersContainer::build(keymap, &descriptor_public_key, &SECP256K1);
            let spending_policy: Option<SpendingPolicy> = descriptor_public_key.extract_policy(
                &signer,
                BuildSatisfaction::None,
                &SECP256K1,
            )?;

            // Compose policy
            Ok(Self {
                name: name.into(),
                description: description.into(),
                descriptor,
                spending_policy,
                network,
            })
        } else {
            Err(Error::NotTaprootDescriptor)
        }
    }

    pub fn from_descriptor<S, D>(
        name: S,
        description: S,
        descriptor: D,
        network: Network,
    ) -> Result<Self, Error>
    where
        S: Into<String>,
        D: AsRef<str>,
    {
        let descriptor: Descriptor<String> = Descriptor::from_str(descriptor.as_ref())?;
        Self::new(name, description, descriptor, network)
    }

    pub fn from_policy<S, P>(
        name: S,
        description: S,
        policy: P,
        network: Network,
    ) -> Result<Self, Error>
    where
        S: Into<String>,
        P: AsRef<str>,
    {
        let policy: Concrete<String> = Concrete::<String>::from_str(policy.as_ref())?;
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
        N: AsRef<str>,
        D: AsRef<str>,
        P: AsRef<str>,
    {
        let name: &str = name.as_ref();
        let description: &str = description.as_ref();
        let desc_or_policy: &str = desc_or_policy.as_ref();
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
        match template.build()? {
            PolicyTemplateResult::Singlesig(key) => {
                let descriptor = Descriptor::new_tr(key, None)?;
                Self::from_descriptor(name, description, descriptor.to_string(), network)
            }
            PolicyTemplateResult::Policy(policy) => {
                Self::from_policy(name.into(), description.into(), policy.to_string(), network)
            }
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn description(&self) -> String {
        self.description.clone()
    }

    pub fn descriptor(&self) -> Descriptor<String> {
        self.descriptor.clone()
    }

    pub fn as_descriptor(&self) -> &Descriptor<String> {
        &self.descriptor
    }

    pub fn network(&self) -> Network {
        self.network
    }

    /// Check if [`Policy`] has an `absolute` or `relative` timelock
    #[inline]
    pub fn has_timelock(&self) -> bool {
        self.has_absolute_timelock() || self.has_relative_timelock()
    }

    /// Check if [`Policy`] has a `absolute` timelock
    #[inline]
    pub fn has_absolute_timelock(&self) -> bool {
        let descriptor = self.descriptor.to_string();
        descriptor.contains("after")
    }

    /// Check if [`Policy`] has a `relative` timelock
    #[inline]
    pub fn has_relative_timelock(&self) -> bool {
        let descriptor = self.descriptor.to_string();
        descriptor.contains("older")
    }

    pub fn spending_policy(&self) -> Result<&SpendingPolicy, Error> {
        self.spending_policy
            .as_ref()
            .ok_or(Error::SpendingPolicyNotFound)
    }

    /// Get [`SatisfiableItem`]
    pub fn satisfiable_item(&self) -> Result<&SatisfiableItem, Error> {
        let policy = self.spending_policy()?;
        Ok(&policy.item)
    }

    /// Get list of [SelectableCondition]
    ///
    /// Return `None` if the [Policy] not contains a timelock
    pub fn selectable_conditions(&self) -> Result<Option<Vec<SelectableCondition>>, Error> {
        if self.has_timelock() {
            fn selectable_conditions(
                item: &SatisfiableItem,
                prev_id: String,
                result: &mut Vec<SelectableCondition>,
            ) {
                if let SatisfiableItem::Thresh { items, threshold } = item {
                    if *threshold < items.len() {
                        result.push(SelectableCondition {
                            path: prev_id,
                            thresh: *threshold,
                            sub_paths: items.iter().map(|i| i.id.clone()).collect(),
                        });
                    }

                    for x in items.iter() {
                        selectable_conditions(&x.item, x.id.clone(), result);
                    }
                }
            }

            let item: &SatisfiableItem = self.satisfiable_item()?;
            let mut result = Vec::new();
            selectable_conditions(item, item.id(), &mut result);
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Get [SatisfiableItem] by policy path
    fn satisfiable_item_by_path<S>(&self, path: S) -> Result<Option<SatisfiableItem>, Error>
    where
        S: Into<String>,
    {
        fn check(
            item: &SatisfiableItem,
            prev_item: Option<SatisfiableItem>,
            path: &String,
        ) -> Option<SatisfiableItem> {
            match item {
                SatisfiableItem::SchnorrSignature(..) | SatisfiableItem::EcdsaSignature(..) => {
                    if &item.id() == path {
                        return prev_item;
                    }
                }
                SatisfiableItem::Thresh { items, .. } => {
                    if &item.id() == path {
                        return prev_item;
                    }

                    for x in items.iter() {
                        if let Some(i) = check(&x.item, Some(x.item.clone()), path) {
                            return Some(i);
                        }
                    }
                }
                _ => (),
            }

            None
        }

        let item: &SatisfiableItem = self.satisfiable_item()?;
        let path: String = path.into();
        Ok(check(item, None, &path))
    }

    /// Check if a [Fingerprint] is involved in the [Policy]
    pub fn is_fingerprint_involved(&self, fingerprint: &Fingerprint) -> Result<bool, Error> {
        let item: &SatisfiableItem = self.satisfiable_item()?;
        Ok(satisfiable_item_contains_fingerprint(item, fingerprint))
    }

    /// Search used signers in this [`Policy`]
    pub fn search_used_signers<I>(&self, my_signers: I) -> impl Iterator<Item = Signer>
    where
        I: Iterator<Item = Signer>,
    {
        let descriptor: String = self.descriptor.to_string();
        my_signers.into_iter().filter_map(move |signer| {
            let signer_descriptor: String = signer.descriptor_public_key().ok()?.to_string();
            if descriptor.contains(&signer_descriptor) {
                Some(signer)
            } else {
                None
            }
        })
    }

    /// Search and map the selectable conditions for the passed [Signer]
    fn map_selectable_conditions_for_signer(
        &self,
        selectable_conditions: &[SelectableCondition],
        signer: &Signer,
    ) -> Result<BTreeMap<String, (usize, Vec<usize>)>, Error> {
        let mut map = BTreeMap::new();
        for SelectableCondition {
            path,
            thresh,
            sub_paths,
        } in selectable_conditions.iter()
        {
            // Iterate all the sub-paths of the selectable condition
            for (index, sub_path) in sub_paths.iter().enumerate() {
                // Try to get the `SatisfiableItem` for the sub-path
                if let Some(item) = self.satisfiable_item_by_path(sub_path)? {
                    // Check if the `SatisfiableItem` contains the signer `fingerprint`
                    if satisfiable_item_contains_fingerprint(&item, &signer.fingerprint()) {
                        map.insert(path.clone(), (*thresh, vec![index]));
                    }
                }
            }
        }
        Ok(map)
    }

    /// Automatically select the `policy path` to use for the passed [Signer].
    pub fn get_policy_path_from_signer(
        &self,
        signer: &Signer,
    ) -> Result<Option<PolicyPathSelector>, Error> {
        // Get selectable conditions
        let selectable_conditions = self.selectable_conditions()?;

        match selectable_conditions {
            Some(selectable_conditions) => {
                // Map the selectable conditions
                let map =
                    self.map_selectable_conditions_for_signer(&selectable_conditions, signer)?;

                // Check status of the map
                if map.is_empty() {
                    Ok(None)
                } else if selectable_conditions.len() == map.len() {
                    // Search paths that not satisfy the thresh
                    let not_matching_thresh: HashMap<&String, &Vec<usize>> = map
                        .iter()
                        .filter(|(_, (thresh, sp))| thresh != &sp.len())
                        .map(|(p, (_, sp))| (p, sp))
                        .collect();

                    if not_matching_thresh.is_empty() {
                        // Policy path selector complete
                        Ok(Some(PolicyPathSelector::Complete {
                            path: map.into_iter().map(|(p, (_, sp))| (p, sp)).collect(),
                        }))
                    } else {
                        // Search missing paths to satisfy thresh
                        let missing_to_select = selectable_conditions
                            .into_iter()
                            .filter_map(
                                |SelectableCondition {
                                     path,
                                     mut sub_paths,
                                     ..
                                 }| {
                                    let idxs: &&Vec<usize> = not_matching_thresh.get(&path)?;
                                    for idx in idxs.iter() {
                                        sub_paths.remove(*idx);
                                    }
                                    Some((path, sub_paths))
                                },
                            )
                            .collect();
                        Ok(Some(PolicyPathSelector::Partial {
                            missing_to_select,
                            selected_path: map.into_iter().map(|(p, (_, sp))| (p, sp)).collect(),
                        }))
                    }
                } else {
                    // Compose internal key path
                    let mut internal_key_path: BTreeMap<String, Vec<usize>> = BTreeMap::new();
                    if let Some(SelectableCondition {
                        path: first_path, ..
                    }) = selectable_conditions.first().cloned()
                    {
                        internal_key_path.insert(first_path, vec![0]);
                    }

                    // Compose selected path
                    let selected_path: BTreeMap<String, Vec<usize>> =
                        map.into_iter().map(|(p, (_, sp))| (p, sp)).collect();

                    // Check if internal path match the current selected policy path
                    if internal_key_path == selected_path {
                        Ok(Some(PolicyPathSelector::Complete {
                            path: selected_path,
                        }))
                    } else {
                        let missing_to_select = selectable_conditions
                            .into_iter()
                            .filter_map(
                                |SelectableCondition {
                                     path,
                                     thresh,
                                     mut sub_paths,
                                 }| {
                                    // Check if path already exists in selected_path
                                    if let Some(idxs) = selected_path.get(&path) {
                                        // Check it thresh satisfied with the already selected path
                                        if idxs.len() < thresh {
                                            // If not, remove the already selected sub paths
                                            for idx in idxs.iter() {
                                                sub_paths.remove(*idx);
                                            }
                                        } else {
                                            // Thresh satisfied, skip selectable condition
                                            return None;
                                        }
                                    }

                                    Some((path, sub_paths))
                                },
                            )
                            .collect();
                        Ok(Some(PolicyPathSelector::Partial {
                            missing_to_select,
                            selected_path,
                        }))
                    }
                }
            }
            None => Ok(None),
        }
    }

    pub fn get_policy_paths_from_signers<I>(&self, my_signers: I) -> Result<PolicyPath, Error>
    where
        I: Iterator<Item = Signer>,
    {
        #[allow(clippy::mutable_key_type)]
        let map: HashMap<Signer, Option<PolicyPathSelector>> = self
            .search_used_signers(my_signers)
            .filter_map(|signer| {
                let pp: Option<PolicyPathSelector> =
                    self.get_policy_path_from_signer(&signer).ok()?;
                Some((signer, pp))
            })
            .collect();

        if map.is_empty() {
            Ok(PolicyPath::None)
        } else {
            let map_len: usize = map.len();
            if map_len == 1 {
                match map.into_values().next() {
                    Some(Some(pp)) => Ok(PolicyPath::Single(pp)),
                    _ => Ok(PolicyPath::None),
                }
            } else {
                let map_by_pps: HashSet<Option<PolicyPathSelector>> =
                    map.values().cloned().collect();
                if map_by_pps.len() == 1 {
                    match map_by_pps.into_iter().next() {
                        Some(Some(pp)) => Ok(PolicyPath::Single(pp)),
                        _ => Ok(PolicyPath::None),
                    }
                } else {
                    Ok(PolicyPath::Multiple(map))
                }
            }
        }
    }

    /// Check if [`Policy`] match any [`PolicyTemplateType`]
    pub fn template_match(&self) -> Result<Option<PolicyTemplateType>, Error> {
        match self.satisfiable_item()? {
            SatisfiableItem::SchnorrSignature(..) => {
                return Ok(Some(PolicyTemplateType::Singlesig))
            }
            SatisfiableItem::Thresh { items, threshold } => {
                if *threshold == 1 && items.len() == 2 {
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
                                            if let SatisfiableItem::RelativeTimelock { .. }
                                            | SatisfiableItem::AbsoluteTimelock { .. } =
                                                items[1].item
                                            {
                                                return Ok(Some(PolicyTemplateType::Hold));
                                            }
                                        }
                                        // Recovery templates
                                        SatisfiableItem::Multisig { .. } => {
                                            // Social Recovery / Inheritance
                                            if let SatisfiableItem::RelativeTimelock { .. }
                                            | SatisfiableItem::AbsoluteTimelock { .. } =
                                                items[1].item
                                            {
                                                return Ok(Some(PolicyTemplateType::Recovery));
                                            }
                                        }
                                        _ => (),
                                    }
                                }

                                // Decaying template
                                if threshold < &items.len() {
                                    let keys_count: usize = items
                                        .iter()
                                        .filter(|i| {
                                            matches!(i.item, SatisfiableItem::SchnorrSignature(_))
                                        })
                                        .count();
                                    let absolute_timelock_count: usize = items
                                        .iter()
                                        .filter(|i| {
                                            matches!(
                                                i.item,
                                                SatisfiableItem::AbsoluteTimelock { .. }
                                            )
                                        })
                                        .count();
                                    let relative_timelock_count: usize = items
                                        .iter()
                                        .filter(|i| {
                                            matches!(
                                                i.item,
                                                SatisfiableItem::RelativeTimelock { .. }
                                            )
                                        })
                                        .count();

                                    if threshold <= &keys_count
                                        && absolute_timelock_count + relative_timelock_count
                                            == items.len() - keys_count
                                    {
                                        return Ok(Some(PolicyTemplateType::Decaying));
                                    }
                                }
                            }
                            _ => (),
                        }
                    }
                }
            }
            _ => {}
        };

        Ok(None)
    }

    /// Estimate TX vsize
    ///
    /// Useful to estimate TX fees
    pub fn estimate_tx_vsize<D>(
        &self,
        wallet: &mut Wallet<D>,
        address: Address<NetworkUnchecked>,
        amount: Amount,
        utxos: Option<Vec<OutPoint>>,
        frozen_utxos: Option<Vec<OutPoint>>,
        policy_path: Option<BTreeMap<String, Vec<usize>>>,
    ) -> Option<usize>
    where
        D: PersistBackend<ChangeSet>,
    {
        let proposal = self
            .spend(
                wallet,
                address,
                amount,
                "",
                FeeRate::default_min_relay_fee(),
                utxos,
                frozen_utxos,
                policy_path,
            )
            .ok()?;
        let psbt = proposal.psbt();
        Some(psbt.unsigned_tx.vsize())
    }

    pub fn spend<D, S>(
        &self,
        wallet: &mut Wallet<D>,
        address: Address<NetworkUnchecked>,
        amount: Amount,
        description: S,
        fee_rate: FeeRate,
        utxos: Option<Vec<OutPoint>>,
        frozen_utxos: Option<Vec<OutPoint>>,
        policy_path: Option<BTreeMap<String, Vec<usize>>>,
    ) -> Result<Proposal, Error>
    where
        D: PersistBackend<ChangeSet>,
        S: Into<String>,
    {
        let wallet_utxos: HashMap<OutPoint, LocalOutput> = wallet
            .list_unspent()
            .map(|utxo| (utxo.outpoint, utxo))
            .collect();

        // Check available UTXOs
        if wallet_utxos.is_empty() {
            return Err(Error::NoUtxosAvailable(String::from(
                "wallet not contains any UTXO",
            )));
        }

        let checkpoint = wallet.latest_checkpoint();

        let current_height: u32 = checkpoint.height();
        let timestamp: u64 = time::timestamp();

        if let Some(frozen_utxos) = &frozen_utxos {
            if wallet
                .list_unspent()
                .filter(|utxo| !frozen_utxos.contains(&utxo.outpoint))
                .count()
                == 0
            {
                return Err(Error::NoUtxosAvailable(String::from(
                    "frozen by other proposals",
                )));
            }
        }

        // Build the PSBT
        let psbt = {
            let mut builder = wallet.build_tx();

            if let Some(frozen_utxos) = frozen_utxos {
                for unspendable in frozen_utxos.into_iter() {
                    builder.add_unspendable(unspendable);
                }
            }

            if let Some(utxos) = utxos {
                if utxos.is_empty() {
                    return Err(Error::NoUtxosSelected);
                }
                builder.manually_selected_only();
                builder.add_utxos(&utxos)?;
            }

            if let Some(path) = policy_path.clone() {
                builder.policy_path(path, KeychainKind::External);
            }

            // TODO: add custom coin selection alorithm (to exclude UTXOs with timelock enabled)
            builder
                .fee_rate(fee_rate)
                .enable_rbf()
                .current_height(current_height);
            match amount {
                Amount::Max => builder
                    .drain_wallet()
                    .drain_to(address.payload.script_pubkey()),
                Amount::Custom(amount) => {
                    builder.add_recipient(address.payload.script_pubkey(), amount)
                }
            };
            builder
                .finish()
                .map_err(|e| Error::BdkCreateTx(format!("{e:?}")))?
        };

        if self.has_timelock() {
            // Check if absolute timelock is satisfied
            if !psbt.unsigned_tx.is_absolute_timelock_satisfied(
                Height::from_consensus(current_height)?,
                Time::from_consensus(timestamp as u32)?,
            ) {
                return Err(Error::AbsoluteTimelockNotSatisfied);
            }

            for txin in psbt.unsigned_tx.input.iter() {
                let sequence: Sequence = txin.sequence;

                // Check if relative timelock is satisfied
                if sequence.is_height_locked() || sequence.is_time_locked() {
                    if let Some(utxo) = wallet_utxos.get(&txin.previous_output) {
                        match utxo.confirmation_time {
                            ConfirmationTime::Confirmed { height, .. } => {
                                if current_height.saturating_sub(height) < sequence.0 {
                                    return Err(Error::RelativeTimelockNotSatisfied);
                                }
                            }
                            ConfirmationTime::Unconfirmed { .. } => {
                                return Err(Error::RelativeTimelockNotSatisfied);
                            }
                        }
                    }
                }
            }
        }

        let amount: u64 = match amount {
            Amount::Max => {
                let fee: u64 = psbt.fee()?.to_sat();
                let (sent, received) = wallet.sent_and_received(&psbt.unsigned_tx);
                sent.saturating_sub(received).saturating_sub(fee)
            }
            Amount::Custom(amount) => amount,
        };

        Ok(Proposal::spending(
            self.descriptor.clone(),
            address,
            amount,
            description,
            psbt,
            policy_path,
        ))
    }

    #[cfg(feature = "reserves")]
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
            .ok_or(Error::SpendingPolicyNotFound)?;
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

/// Check if [SatisfiableItem] contains [Fingerprint]
fn satisfiable_item_contains_fingerprint(
    item: &SatisfiableItem,
    fingerprint: &Fingerprint,
) -> bool {
    match item {
        SatisfiableItem::EcdsaSignature(key) => {
            if let PkOrF::Fingerprint(f) = key {
                f == fingerprint
            } else {
                false
            }
        }
        SatisfiableItem::SchnorrSignature(key) => {
            if let PkOrF::Fingerprint(f) = key {
                f == fingerprint
            } else {
                false
            }
        }
        SatisfiableItem::Sha256Preimage { .. } => false,
        SatisfiableItem::Hash256Preimage { .. } => false,
        SatisfiableItem::Ripemd160Preimage { .. } => false,
        SatisfiableItem::Hash160Preimage { .. } => false,
        SatisfiableItem::AbsoluteTimelock { .. } => false,
        SatisfiableItem::RelativeTimelock { .. } => false,
        SatisfiableItem::Multisig { keys, .. } => {
            for key in keys.iter() {
                if let PkOrF::Fingerprint(f) = key {
                    if f == fingerprint {
                        return true;
                    }
                }
            }
            false
        }
        SatisfiableItem::Thresh { items, .. } => {
            for x in items.iter() {
                if satisfiable_item_contains_fingerprint(&x.item, fingerprint) {
                    return true;
                }
            }

            false
        }
    }
}

#[cfg(test)]
mod tests {
    use bdk::keys::DescriptorPublicKey;
    use keechain_core::bips::bip39::Mnemonic;
    use keechain_core::Seed;

    use super::*;
    use crate::signer::smartvaults_signer;

    const NETWORK: Network = Network::Testnet;

    const COMPLEX_DESCRIPTOR: &str = "tr([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*,and_v(v:pk([f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*),andor(pk([f57a6b99/86'/1'/784923']tpubDC45v32EZGP2U4qVTKayC3kkdKmFAFDxxA7wnCCVgUuPXRFNms1W1LZq2LiCUBk5XmNvTZcEtbexZUMtY4ubZGS74kQftEGibUxUpybMan7/0/*),older(52000),multi_a(2,[4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*,[8cab67b4/86'/1'/784923']tpubDC6N2TsKj5zdHzqU17wnQMHsD1BdLVue3bkk2a2BHnVHoTvhX2JdKGgnMwRiMRVVs3art21SusorgGxXoZN54JhXNQ7KoJsHLTR6Kvtu7Ej/0/*))))#auurkhk6";
    const COMPLEX_DESCRIPTOR_WITH_TIMELOCK: &str = "tr(af2486c537bbb20285bb29e0dd7c05a875b684aee7d4a2501c5c1aea63eaff1c,thresh(2,or_b(pk([bc2776d1/86'/1'/0']tpubDC4TeTzs8NdabBTsyKfm2agwwmeq1LmdPhqv7Zt52VjvVNPDz7Mex8F5hsZxctzY5QQAr2jRH7Fq4xfijcngzKxmB73DapuTvjbcwH6Mm8K/0/*),s:pk([165200fa/86'/1'/0']tpubDDMDcGB9jV7K5vj64NhwWwDC6rrjTF9H1qtzbgK9Daw8S9aF7ueoqtGhwmWoG8ugdkufaiux21EmZU7ymim1cTZWvuy8gPNbxCVDCR7ponD/0/*)),s:pk([d9cf55da/86'/1'/784923']tpubDDezFokYJHuh5HSidMM728ntSNzNYFGCn2Ei9dNyF2jDbeoGFL2vdu9tCKcULD9bY9aJrfzLX4f5D3BBqKFt6LZW24PacakDUV7zPB4MBwS/0/*),ajc:and_v(v:after(1709133311),and_v(v:pk([bfa46e8e/86'/1'/784923']tpubDCLfFtgA8wiEj9c5zLrmGzD7qghbpo9uEvPH4vqAogsS43oaHbqoTHbjuUqsvnwVouzUmdsuMSRPCQu36eRfvq7mda6zob76QCcwPZeFCEQ/0/*),pk_k([bd5efadb/86'/1'/784923']tpubDDFdQjA7WGJaD5DcuZL2rKzcYNpA6p3E8TpoV2isBSfvrUBf2XhBxm7qxxAURFK5tBA5i4YEJG1gLZiaXt9P96vVRdYGgGjvHyk5BfCG9cV/0/*)))))#qh92myc8";

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
        let policy = Policy::from_descriptor("", "", COMPLEX_DESCRIPTOR, NETWORK).unwrap();
        let conditions = policy.selectable_conditions().unwrap();
        let mut c = Vec::new();
        c.push(SelectableCondition {
            path: String::from("y46gds64"),
            thresh: 1,
            sub_paths: vec![String::from("lcjxl004"), String::from("8sld2cgj")],
        });
        c.push(SelectableCondition {
            path: String::from("fx0z8u06"),
            thresh: 1,
            sub_paths: vec![String::from("0e36xhlc"), String::from("m4n7s285")],
        });
        assert_eq!(conditions, Some(c));

        let policy: &str = "thresh(2,pk([87131a00/86'/1'/784923']tpubDDEaK5JwGiGDTRkML9YKh8AF4rHPhkpnXzVjVMDBtzayJpnsWKeiFPxtiyYeGHQj8pnjsei7N98winwZ3ivGoVVKArZVMsEYGig73XVqbSX/0/*),pk([e157a520/86'/1'/784923']tpubDCCYFYCyDkxo1xAzDpoFNdtGcjD5BPLZbEJswjJmwqp67Weqd2C7fg6Jy1SBjgn3wYnKyUtoYKXG4VdQczjqb6FJnqHe3NmFdgy8vNBSty4/0/*))";
        let policy = Policy::from_policy("", "", policy, NETWORK).unwrap();
        let conditions = policy.selectable_conditions().unwrap();
        assert!(conditions.is_none());

        let policy =
            Policy::from_descriptor("", "", COMPLEX_DESCRIPTOR_WITH_TIMELOCK, NETWORK).unwrap();
        let conditions = policy.selectable_conditions().unwrap();
        let mut c = Vec::new();
        c.push(SelectableCondition {
            path: String::from("lahrrd60"),
            thresh: 1,
            sub_paths: vec![String::from("qu0ehqvt"), String::from("w3x74z9c")],
        });
        c.push(SelectableCondition {
            path: String::from("w3x74z9c"),
            thresh: 2,
            sub_paths: vec![
                String::from("fnd3dju4"),
                String::from("7lkk2qnw"),
                String::from("8exk8zzy"),
            ],
        });
        c.push(SelectableCondition {
            path: String::from("fnd3dju4"),
            thresh: 1,
            sub_paths: vec![String::from("289a0cgn"), String::from("urk6p6zp")],
        });
        assert_eq!(conditions, Some(c));
    }

    #[test]
    fn test_get_policy_path_from_signer() {
        // Common policy
        let policy = Policy::from_descriptor("", "", COMPLEX_DESCRIPTOR, NETWORK).unwrap();

        // Internal key path
        let mnemonic = Mnemonic::from_str(
            "possible suffer flavor boring essay zoo collect stairs day cabbage wasp tackle",
        )
        .unwrap();
        let seed = Seed::from_mnemonic(mnemonic);
        let signer = smartvaults_signer(seed, NETWORK).unwrap();
        let policy_path: Option<PolicyPathSelector> =
            policy.get_policy_path_from_signer(&signer).unwrap();
        let mut path: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        path.insert(String::from("y46gds64"), vec![0]);
        assert_eq!(policy_path, Some(PolicyPathSelector::Complete { path }));

        // Path
        let mnemonic = Mnemonic::from_str(
            "vicious climb harsh insane yard aspect frequent already tackle fetch ask throw",
        )
        .unwrap();
        let seed = Seed::from_mnemonic(mnemonic);
        let signer = smartvaults_signer(seed, NETWORK).unwrap();

        let policy_path: Option<PolicyPathSelector> =
            policy.get_policy_path_from_signer(&signer).unwrap();

        // Result
        let mut path: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        path.insert(String::from("fx0z8u06"), vec![0]);
        path.insert(String::from("y46gds64"), vec![1]);
        assert_eq!(policy_path, Some(PolicyPathSelector::Complete { path }));

        // Another path
        let mnemonic = Mnemonic::from_str(
            "involve camp enter man minimum milk minimum news hockey divert window mind",
        )
        .unwrap();
        let seed = Seed::from_mnemonic(mnemonic);
        let signer = smartvaults_signer(seed, NETWORK).unwrap();

        let policy_path: Option<PolicyPathSelector> =
            policy.get_policy_path_from_signer(&signer).unwrap();

        // Result
        let mut selected_path: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        selected_path.insert(String::from("y46gds64"), vec![1]);
        let mut missing_to_select: BTreeMap<String, Vec<String>> = BTreeMap::new();
        missing_to_select.insert(
            String::from("fx0z8u06"),
            vec![String::from("0e36xhlc"), String::from("m4n7s285")],
        );
        assert_eq!(
            policy_path,
            Some(PolicyPathSelector::Partial {
                selected_path,
                missing_to_select
            })
        );

        // Decaying 2 of 4 (absolute)
        let desc = "tr(56f05264c005e2a2f6e261996ed2cd904dfafbc6d75cc07a5a76d46df56e6ff9,thresh(2,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),s:pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*),s:pk([f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*),snl:after(1698947462)))#2263dhaf";
        let policy = Policy::from_descriptor("", "", desc, NETWORK).unwrap();
        let mnemonic = Mnemonic::from_str(
            "possible suffer flavor boring essay zoo collect stairs day cabbage wasp tackle",
        )
        .unwrap();
        let seed = Seed::from_mnemonic(mnemonic);
        let signer = smartvaults_signer(seed, NETWORK).unwrap();
        let policy_path: Option<PolicyPathSelector> =
            policy.get_policy_path_from_signer(&signer).unwrap();
        let mut selected_path: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        selected_path.insert(String::from("5p29qyl4"), vec![0]);
        selected_path.insert(String::from("n4vvscy5"), vec![1]);
        let mut missing_to_select: BTreeMap<String, Vec<String>> = BTreeMap::new();
        missing_to_select.insert(
            String::from("5p29qyl4"),
            vec![
                String::from("8w0qa9ut"),
                String::from("ayztcvww"),
                String::from("ysdtrx7w"),
            ],
        );
        assert_eq!(
            policy_path,
            Some(PolicyPathSelector::Partial {
                selected_path,
                missing_to_select
            })
        );
    }

    #[test]
    fn test_get_policy_path_from_signers() {
        // Signer 1
        let fing1 = Fingerprint::from_str("165200fa").unwrap();
        let desc1 = Descriptor::from_str("tr([165200fa/86'/1'/0']tpubDDMDcGB9jV7K5vj64NhwWwDC6rrjTF9H1qtzbgK9Daw8S9aF7ueoqtGhwmWoG8ugdkufaiux21EmZU7ymim1cTZWvuy8gPNbxCVDCR7ponD/0/*)").unwrap();
        let signer1 = Signer::airgap("", None, fing1, desc1, NETWORK).unwrap();

        // Signer 2
        let fing2 = Fingerprint::from_str("bd5efadb").unwrap();
        let desc2 = Descriptor::from_str("tr([bd5efadb/86'/1'/784923']tpubDDFdQjA7WGJaD5DcuZL2rKzcYNpA6p3E8TpoV2isBSfvrUBf2XhBxm7qxxAURFK5tBA5i4YEJG1gLZiaXt9P96vVRdYGgGjvHyk5BfCG9cV/0/*)").unwrap();
        let signer2 = Signer::airgap("", None, fing2, desc2, NETWORK).unwrap();

        // Policy
        let policy =
            Policy::from_descriptor("", "", COMPLEX_DESCRIPTOR_WITH_TIMELOCK, NETWORK).unwrap();

        // Path
        let path = policy
            .get_policy_paths_from_signers([signer1.clone(), signer2.clone()].into_iter())
            .unwrap();

        if let PolicyPath::Multiple(map) = path {
            let pp1 = map.get(&signer1).cloned().unwrap();
            let mut selected_path: BTreeMap<String, Vec<usize>> = BTreeMap::new();
            selected_path.insert(String::from("fnd3dju4"), vec![1]);
            selected_path.insert(String::from("lahrrd60"), vec![1]);
            selected_path.insert(String::from("w3x74z9c"), vec![0]);
            let mut missing_to_select: BTreeMap<String, Vec<String>> = BTreeMap::new();
            missing_to_select.insert(
                String::from("w3x74z9c"),
                vec![String::from("7lkk2qnw"), String::from("8exk8zzy")],
            );
            assert_eq!(
                pp1,
                Some(PolicyPathSelector::Partial {
                    selected_path,
                    missing_to_select
                })
            );

            // --------------------

            let pp2 = map.get(&signer2).cloned().unwrap();
            let mut selected_path: BTreeMap<String, Vec<usize>> = BTreeMap::new();
            selected_path.insert(String::from("lahrrd60"), vec![1]);
            selected_path.insert(String::from("w3x74z9c"), vec![2]);
            let mut missing_to_select: BTreeMap<String, Vec<String>> = BTreeMap::new();
            missing_to_select.insert(
                String::from("fnd3dju4"),
                vec![String::from("289a0cgn"), String::from("urk6p6zp")],
            );
            missing_to_select.insert(
                String::from("w3x74z9c"),
                vec![String::from("fnd3dju4"), String::from("7lkk2qnw")],
            );
            assert_eq!(
                pp2,
                Some(PolicyPathSelector::Partial {
                    selected_path,
                    missing_to_select
                })
            );
        } else {
            panic!("Unexpected policy path");
        }
    }

    #[test]
    fn test_is_fingerprint_involved() {
        let policy = Policy::from_descriptor("", "", COMPLEX_DESCRIPTOR, NETWORK).unwrap();

        let fingerprint = Fingerprint::from_str("7356e457").unwrap();
        assert!(policy.is_fingerprint_involved(&fingerprint).unwrap());

        let fingerprint = Fingerprint::from_str("f3ab64d8").unwrap();
        assert!(policy.is_fingerprint_involved(&fingerprint).unwrap());

        let fingerprint = Fingerprint::from_str("f57a6b99").unwrap();
        assert!(policy.is_fingerprint_involved(&fingerprint).unwrap());

        let fingerprint = Fingerprint::from_str("4eb5d5a1").unwrap();
        assert!(policy.is_fingerprint_involved(&fingerprint).unwrap());

        let fingerprint = Fingerprint::from_str("8cab67b4").unwrap();
        assert!(policy.is_fingerprint_involved(&fingerprint).unwrap());

        // NOT involved
        let fingerprint = Fingerprint::from_str("7c997e72").unwrap();
        assert!(!policy.is_fingerprint_involved(&fingerprint).unwrap());
    }

    #[test]
    fn test_policy_template_match() {
        let singlesig = DescriptorPublicKey::from_str("[7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*").unwrap();
        let singlesig = PolicyTemplate::singlesig(singlesig);
        let policy = Policy::from_template("Singlesig", "", singlesig, NETWORK).unwrap();
        assert_eq!(
            policy.template_match().unwrap(),
            Some(PolicyTemplateType::Singlesig)
        );

        let multisig_1_of_2 = "thresh(1,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*))";
        let policy = Policy::from_policy("Multisig 1 of 2", "", multisig_1_of_2, NETWORK).unwrap();
        assert_eq!(
            policy.template_match().unwrap(),
            Some(PolicyTemplateType::Multisig)
        );

        let multisig_2_of_2 = "thresh(2,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*))";
        let policy = Policy::from_policy("Multisig 2 of 2", "", multisig_2_of_2, NETWORK).unwrap();
        assert_eq!(
            policy.template_match().unwrap(),
            Some(PolicyTemplateType::Multisig)
        );

        let social_recovery = "or(1@pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),1@and(thresh(2,pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*),pk([f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*)),older(6)))";
        let policy = Policy::from_policy("Social Recovery", "", social_recovery, NETWORK).unwrap();
        assert_eq!(
            policy.template_match().unwrap(),
            Some(PolicyTemplateType::Recovery)
        );

        let inheritance = "or(1@pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),1@and(thresh(2,pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*),pk([f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*)),after(700000)))";
        let policy = Policy::from_policy("Inheritance", "", inheritance, NETWORK).unwrap();
        assert_eq!(
            policy.template_match().unwrap(),
            Some(PolicyTemplateType::Recovery)
        );

        // Hold (older)
        let hold = "and(pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),older(144))";
        let policy = Policy::from_policy("Hold", "", hold, NETWORK).unwrap();
        assert_eq!(
            policy.template_match().unwrap(),
            Some(PolicyTemplateType::Hold)
        );

        // Hold (after)
        let hold = "and(pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),after(840000))";
        let policy = Policy::from_policy("Hold", "", hold, NETWORK).unwrap();
        assert_eq!(
            policy.template_match().unwrap(),
            Some(PolicyTemplateType::Hold)
        );

        // Decaying 3 of 3 (older)
        let decaying = "thresh(3,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*),pk([f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*),older(2))";
        let policy = Policy::from_policy("Decaying", "", decaying, NETWORK).unwrap();
        assert_eq!(
            policy.template_match().unwrap(),
            Some(PolicyTemplateType::Decaying)
        );

        // Decaying 2 of 3 (older)
        let decaying = "thresh(2,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*),pk([f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*),older(2))";
        let policy = Policy::from_policy("Decaying", "", decaying, NETWORK).unwrap();
        assert_eq!(
            policy.template_match().unwrap(),
            Some(PolicyTemplateType::Decaying)
        );

        // Decaying (after)
        let decaying = "thresh(3,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*),pk([f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*),after(840000),after(940000))";
        let policy = Policy::from_policy("Decaying", "", decaying, NETWORK).unwrap();
        assert_eq!(
            policy.template_match().unwrap(),
            Some(PolicyTemplateType::Decaying)
        );
    }
}

#[cfg(bench)]
mod benches {
    use test::{black_box, Bencher};

    use super::*;

    const NETWORK: Network = Network::Testnet;

    #[bench]
    pub fn bench_policy_from_descriptor(bh: &mut Bencher) {
        let desc = "tr(56f05264c005e2a2f6e261996ed2cd904dfafbc6d75cc07a5a76d46df56e6ff9,thresh(2,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),s:pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*),s:pk([f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*),snl:after(1698947462)))#2263dhaf";
        bh.iter(|| {
            black_box(Policy::from_descriptor("", "", desc, NETWORK)).unwrap();
        });
    }

    #[bench]
    pub fn bench_policy_from_miniscript(bh: &mut Bencher) {
        let desc = "thresh(3,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*),pk([f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*),after(840000),after(940000))";
        bh.iter(|| {
            black_box(Policy::from_policy("", "", desc, NETWORK)).unwrap();
        });
    }

    #[bench]
    pub fn bench_decaying_template_match(bh: &mut Bencher) {
        let desc = "thresh(3,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*),pk([f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*),after(840000),after(940000))";
        let policy = Policy::from_policy("", "", desc, NETWORK).unwrap();
        bh.iter(|| {
            black_box(policy.template_match()).unwrap();
        });
    }

    #[bench]
    pub fn bench_social_recovery_template_match(bh: &mut Bencher) {
        let desc = "or(1@pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*),1@and(thresh(2,pk([4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*),pk([f3ab64d8/86'/1'/784923']tpubDCh4uyVDVretfgTNkazUarV9ESTh7DJy8yvMSuWn5PQFbTDEsJwHGSBvTrNF92kw3x5ZLFXw91gN5LYtuSCbr1Vo6mzQmD49sF2vGpReZp2/0/*)),older(6)))";
        let policy = Policy::from_policy("", "", desc, NETWORK).unwrap();
        bh.iter(|| {
            black_box(policy.template_match()).unwrap();
        });
    }
}
