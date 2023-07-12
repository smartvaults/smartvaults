// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

#[cfg(feature = "hwi")]
pub use hwi;
pub use keechain_core::*;

pub mod constants;
pub mod policy;
pub mod proposal;
pub mod reserves;
pub mod signer;
pub mod types;
pub mod util;

pub use self::policy::Policy;
pub use self::proposal::{ApprovedProposal, CompletedProposal, Proposal};
pub use self::types::{Amount, FeeRate};

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use bdk::bitcoin::{Address, Network};
    use bdk::miniscript::DescriptorPublicKey;
    use bdk::wallet::get_funded_wallet;
    use bdk::FeeRate;
    use keechain_core::bips::bip39::Mnemonic;
    use keechain_core::types::descriptors::ToDescriptor;
    use keechain_core::types::{Purpose, Seed};
    use keechain_core::Result;

    use crate::constants::COINSTR_ACCOUNT_INDEX;
    use crate::proposal::ProposalType;

    use super::*;

    const NETWORK: Network = Network::Testnet;
    const MNEMONIC_A: &str =
        "possible suffer flavor boring essay zoo collect stairs day cabbage wasp tackle";
    const MNEMONIC_B: &str =
        "panther tree neglect narrow drip act visit position pass assault tennis long";

    #[test]
    fn test_policy_spend() -> Result<()> {
        // User A
        let mnemonic_a: Mnemonic = Mnemonic::from_str(MNEMONIC_A)?;
        let seed_a: Seed = Seed::from_mnemonic(mnemonic_a);
        let desc_a: DescriptorPublicKey =
            seed_a.to_descriptor(Purpose::TR, Some(7291640), false, NETWORK)?;

        // User B
        let mnemonic_b: Mnemonic = Mnemonic::from_str(MNEMONIC_B)?;
        let seed_b: Seed = Seed::from_mnemonic(mnemonic_b);
        let desc_b: DescriptorPublicKey =
            seed_b.to_descriptor(Purpose::TR, Some(7291640), false, NETWORK)?;

        let policy: String = policy::builder::n_of_m_ext_multisig(2, vec![desc_a, desc_b])?;
        let policy: Policy = Policy::from_policy("Name", "Description", &policy, NETWORK)?;
        let descriptor: String = policy.descriptor.to_string();

        let (wallet, ..) = get_funded_wallet(&descriptor);
        let proposal: Proposal = policy.spend(
            wallet,
            Address::from_str("mohjSavDdQYHRYXcS3uS6ttaHP8amyvX78")?,
            Amount::Custom(1120),
            "Testing",
            FeeRate::from_sat_per_vb(1.0),
        )?;

        let approved_a: ApprovedProposal = proposal.approve(&seed_a, Vec::new(), NETWORK)?;
        let approved_b: ApprovedProposal = proposal.approve(&seed_b, Vec::new(), NETWORK)?;

        let completed_proposal: CompletedProposal =
            proposal.finalize(vec![approved_a, approved_b], NETWORK)?;

        assert_eq!(completed_proposal.get_type(), ProposalType::Spending);

        Ok(())
    }

    #[test]
    fn test_proof_of_reserve() -> Result<()> {
        // User A
        let mnemonic_a: Mnemonic = Mnemonic::from_str(MNEMONIC_A)?;
        let seed_a: Seed = Seed::from_mnemonic(mnemonic_a);
        let desc_a: DescriptorPublicKey =
            seed_a.to_descriptor(Purpose::TR, Some(7291640), false, NETWORK)?;

        // User B
        let mnemonic_b: Mnemonic = Mnemonic::from_str(MNEMONIC_B)?;
        let seed_b: Seed = Seed::from_mnemonic(mnemonic_b);
        let desc_b: DescriptorPublicKey =
            seed_b.to_descriptor(Purpose::TR, Some(7291640), false, NETWORK)?;

        let policy: String = policy::builder::n_of_m_ext_multisig(2, vec![desc_a, desc_b])?;
        let policy: Policy = Policy::from_policy("Name", "Description", &policy, NETWORK)?;
        let descriptor: String = policy.descriptor.to_string();

        let (wallet, ..) = get_funded_wallet(&descriptor);
        let proposal: Proposal = policy.proof_of_reserve(wallet, "Testing proof of reserve")?;

        let approved_a: ApprovedProposal = proposal.approve(&seed_a, Vec::new(), NETWORK)?;
        let approved_b: ApprovedProposal = proposal.approve(&seed_b, Vec::new(), NETWORK)?;

        let completed_proposal: CompletedProposal =
            proposal.finalize(vec![approved_a, approved_b], NETWORK)?;

        assert_eq!(completed_proposal.get_type(), ProposalType::ProofOfReserve);

        Ok(())
    }

    #[test]
    fn test_policy_spend_1_of_2_multisig() -> Result<()> {
        // User A
        let mnemonic_a: Mnemonic = Mnemonic::from_str(MNEMONIC_A)?;
        let seed_a: Seed = Seed::from_mnemonic(mnemonic_a);
        let desc_a: DescriptorPublicKey =
            seed_a.to_descriptor(Purpose::TR, Some(COINSTR_ACCOUNT_INDEX), false, NETWORK)?;

        // User B
        let mnemonic_b: Mnemonic = Mnemonic::from_str(MNEMONIC_B)?;
        let seed_b: Seed = Seed::from_mnemonic(mnemonic_b);
        let desc_b: DescriptorPublicKey =
            seed_b.to_descriptor(Purpose::TR, Some(COINSTR_ACCOUNT_INDEX), false, NETWORK)?;

        let policy: String = policy::builder::n_of_m_ext_multisig(1, vec![desc_a, desc_b])?;
        let policy: Policy = Policy::from_policy("Name", "Description", &policy, NETWORK)?;
        let descriptor: String = policy.descriptor.to_string();

        let (wallet, ..) = get_funded_wallet(&descriptor);
        let proposal: Proposal = policy.spend(
            wallet,
            Address::from_str("mohjSavDdQYHRYXcS3uS6ttaHP8amyvX78")?,
            Amount::Custom(1120),
            "Testing",
            FeeRate::from_sat_per_vb(1.0),
        )?;

        let approved_a: ApprovedProposal = proposal.approve(&seed_a, Vec::new(), NETWORK)?;

        let completed_proposal: CompletedProposal = proposal.finalize(vec![approved_a], NETWORK)?;

        assert_eq!(completed_proposal.get_type(), ProposalType::Spending);

        Ok(())
    }
}
