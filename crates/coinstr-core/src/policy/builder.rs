// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use bdk::bitcoin::XOnlyPublicKey;
use bdk::miniscript::policy::concrete::Policy;

pub fn n_of_m_multisig(required_sig: usize, keys: Vec<XOnlyPublicKey>) -> String {
    let keys: Vec<Policy<XOnlyPublicKey>> = keys.into_iter().map(Policy::Key).collect();
    Policy::Threshold(required_sig, keys).to_string()
}
