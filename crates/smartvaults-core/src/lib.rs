// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

pub use bdk::miniscript;
use keechain_core::secp256k1::{rand, All, Secp256k1};
pub use keechain_core::*;
use once_cell::sync::Lazy;

pub mod constants;
pub mod policy;
pub mod proposal;
#[cfg(feature = "reserves")]
pub mod reserves;
pub mod signer;
pub mod types;
pub mod util;

pub use self::policy::{
    AbsoluteLockTime, DecayingTime, Locktime, Policy, PolicyTemplate, PolicyTemplateType,
    RecoveryTemplate, Sequence,
};
pub use self::proposal::{ApprovedProposal, CompletedProposal, Proposal};
pub use self::signer::{SharedSigner, Signer, SignerType};
pub use self::types::{Amount, FeeRate, Priority};

pub static SECP256K1: Lazy<Secp256k1<All>> = Lazy::new(|| {
    let mut ctx = Secp256k1::new();
    let mut rng = rand::thread_rng();
    ctx.randomize(&mut rng);
    ctx
});

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use keechain_core::bdk::chain::{BlockId, ConfirmationTime};
    use keechain_core::bdk::wallet::AddressIndex;
    use keechain_core::bdk::{FeeRate, Wallet};
    use keechain_core::bips::bip39::Mnemonic;
    use keechain_core::bitcoin::absolute::Height;
    use keechain_core::bitcoin::hashes::Hash;
    use keechain_core::bitcoin::{absolute, Address, BlockHash, Network, Transaction, TxOut};
    use keechain_core::miniscript::DescriptorPublicKey;
    use keechain_core::types::descriptors::ToDescriptor;
    use keechain_core::types::{Purpose, Seed};
    use keechain_core::Result;

    use crate::constants::SMARTVAULTS_ACCOUNT_INDEX;
    use crate::proposal::ProposalType;

    use super::*;

    const NETWORK: Network = Network::Testnet;
    const MNEMONIC_A: &str =
        "possible suffer flavor boring essay zoo collect stairs day cabbage wasp tackle";
    const MNEMONIC_B: &str =
        "panther tree neglect narrow drip act visit position pass assault tennis long";

    pub fn get_funded_wallet(descriptor: &str) -> Result<Wallet> {
        let mut wallet = Wallet::new_no_persist(descriptor, None, NETWORK)?;

        wallet
            .insert_checkpoint(BlockId {
                height: 2_520_450,
                hash: BlockHash::all_zeros(),
            })
            .unwrap();

        let address = wallet.get_address(AddressIndex::New).address;
        wallet
            .insert_tx(
                Transaction {
                    version: 1,
                    lock_time: absolute::LockTime::Blocks(
                        Height::from_consensus(2_477_486).unwrap(),
                    ),
                    input: vec![],
                    output: vec![TxOut {
                        value: 371,
                        script_pubkey: address.script_pubkey(),
                    }],
                },
                ConfirmationTime::Confirmed {
                    height: 2_477_484,
                    time: 1694533020,
                },
            )
            .unwrap();

        let address = wallet.get_address(AddressIndex::New).address;
        wallet
            .insert_tx(
                Transaction {
                    version: 1,
                    lock_time: absolute::LockTime::Blocks(
                        Height::from_consensus(2_505_450).unwrap(),
                    ),
                    input: vec![],
                    output: vec![TxOut {
                        value: 3500,
                        script_pubkey: address.script_pubkey(),
                    }],
                },
                ConfirmationTime::Confirmed {
                    height: 2_505_449,
                    time: 1695832380,
                },
            )
            .unwrap();

        let address = wallet.get_address(AddressIndex::New).address;
        wallet
            .insert_tx(
                Transaction {
                    version: 1,
                    lock_time: absolute::LockTime::Blocks(
                        Height::from_consensus(2_520_272).unwrap(),
                    ),
                    input: vec![],
                    output: vec![TxOut {
                        value: 4149,
                        script_pubkey: address.script_pubkey(),
                    }],
                },
                ConfirmationTime::Confirmed {
                    height: 2_520_271,
                    time: 1696282140,
                },
            )
            .unwrap();

        Ok(wallet)
    }

    #[test]
    fn test_policy_spend() -> Result<()> {
        // User A
        let mnemonic_a: Mnemonic = Mnemonic::from_str(MNEMONIC_A)?;
        let seed_a: Seed = Seed::from_mnemonic(mnemonic_a);
        let desc_a: DescriptorPublicKey =
            seed_a.to_descriptor(Purpose::TR, Some(7291640), false, NETWORK, &SECP256K1)?;

        // User B
        let mnemonic_b: Mnemonic = Mnemonic::from_str(MNEMONIC_B)?;
        let seed_b: Seed = Seed::from_mnemonic(mnemonic_b);
        let desc_b: DescriptorPublicKey =
            seed_b.to_descriptor(Purpose::TR, Some(7291640), false, NETWORK, &SECP256K1)?;

        let template = PolicyTemplate::multisig(2, vec![desc_a, desc_b]);
        let policy: Policy = Policy::from_template("Name", "Description", template, NETWORK)?;
        let descriptor: String = policy.descriptor.to_string();

        let mut wallet = get_funded_wallet(&descriptor).unwrap();
        let proposal: Proposal = policy.spend(
            &mut wallet,
            Address::from_str("mohjSavDdQYHRYXcS3uS6ttaHP8amyvX78")?,
            Amount::Custom(1120),
            "Testing",
            FeeRate::from_sat_per_vb(1.0),
            None,
            None,
            None,
        )?;

        let approved_a: ApprovedProposal = proposal.approve(&seed_a, Vec::new(), NETWORK)?;
        let approved_b: ApprovedProposal = proposal.approve(&seed_b, Vec::new(), NETWORK)?;

        let completed_proposal: CompletedProposal =
            proposal.finalize(vec![approved_a, approved_b], NETWORK)?;

        assert_eq!(completed_proposal.get_type(), ProposalType::Spending);

        Ok(())
    }

    #[test]
    #[cfg(feature = "reserves")]
    fn test_proof_of_reserve() -> Result<()> {
        // User A
        let mnemonic_a: Mnemonic = Mnemonic::from_str(MNEMONIC_A)?;
        let seed_a: Seed = Seed::from_mnemonic(mnemonic_a);
        let desc_a: DescriptorPublicKey =
            seed_a.to_descriptor(Purpose::TR, Some(7291640), false, NETWORK, &SECP256K1)?;

        // User B
        let mnemonic_b: Mnemonic = Mnemonic::from_str(MNEMONIC_B)?;
        let seed_b: Seed = Seed::from_mnemonic(mnemonic_b);
        let desc_b: DescriptorPublicKey =
            seed_b.to_descriptor(Purpose::TR, Some(7291640), false, NETWORK, &SECP256K1)?;

        let template = PolicyTemplate::multisig(2, vec![desc_a, desc_b]);
        let policy: Policy = Policy::from_template("Name", "Description", template, NETWORK)?;
        let descriptor: String = policy.descriptor.to_string();

        let mut wallet = get_funded_wallet(&descriptor).unwrap();
        let proposal: Proposal =
            policy.proof_of_reserve(&mut wallet, "Testing proof of reserve")?;

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
        let desc_a: DescriptorPublicKey = seed_a.to_descriptor(
            Purpose::TR,
            Some(SMARTVAULTS_ACCOUNT_INDEX),
            false,
            NETWORK,
            &SECP256K1,
        )?;

        // User B
        let mnemonic_b: Mnemonic = Mnemonic::from_str(MNEMONIC_B)?;
        let seed_b: Seed = Seed::from_mnemonic(mnemonic_b);
        let desc_b: DescriptorPublicKey = seed_b.to_descriptor(
            Purpose::TR,
            Some(SMARTVAULTS_ACCOUNT_INDEX),
            false,
            NETWORK,
            &SECP256K1,
        )?;

        let template = PolicyTemplate::multisig(1, vec![desc_a, desc_b]);
        let policy: Policy = Policy::from_template("Name", "Description", template, NETWORK)?;
        let descriptor: String = policy.descriptor.to_string();

        let mut wallet = get_funded_wallet(&descriptor).unwrap();
        let proposal: Proposal = policy.spend(
            &mut wallet,
            Address::from_str("mohjSavDdQYHRYXcS3uS6ttaHP8amyvX78")?,
            Amount::Custom(1120),
            "Testing",
            FeeRate::from_sat_per_vb(1.0),
            None,
            None,
            None,
        )?;

        let approved_a: ApprovedProposal = proposal.approve(&seed_a, Vec::new(), NETWORK)?;

        let completed_proposal: CompletedProposal = proposal.finalize(vec![approved_a], NETWORK)?;

        assert_eq!(completed_proposal.get_type(), ProposalType::Spending);

        Ok(())
    }

    #[test]
    fn test_1_of_3_multisig() {
        let network = Network::Testnet;

        let mnemonic = Mnemonic::from_str(
            "message scissors typical gravity patrol lunch about bacon person focus cry uncover",
        )
        .unwrap();
        let seed = Seed::from_mnemonic(mnemonic);

        let descriptor = "tr([5cb492a5/86'/1'/784923']tpubDD56LAR1MR7X5EeZYMpvivk2Lh3HMo4vdDNQ8jAv4oBjLPEddQwxaxNypvrHbMk2qTxAj44YLzqHrzwy5LDNmVyYZBesm6aShhmhYrA8veT/0/*,{pk([76fdbca2/86'/1'/784923']tpubDCDepsNyAPWySAgXx1Por6sHpSWzxsTB9XJp5erEN7NumgdZMhhmycJGMQ1cHZwx66KyZr6psjttDDQ7mV4uJGV2DvB9Mri1nTVmpquvTDR/0/*),pk([3b8ae29b/86'/1'/784923']tpubDDpkQsJQTpHi2bH5Cg7L1pThUxeEStcn9ZsQ53XHkW8Fs81h71XobqpwYf2Jb8ECmW1mUUJxQhZstmwFUg5wQ6EVzH5HmF3cpHcyxjvF1Ep/0/*)})#yxpuntg3";
        let policy = Policy::from_descriptor("", "", descriptor, network).unwrap();

        let mut wallet = get_funded_wallet(&descriptor).unwrap();
        let proposal: Proposal = policy
            .spend(
                &mut wallet,
                Address::from_str("mohjSavDdQYHRYXcS3uS6ttaHP8amyvX78").unwrap(),
                Amount::Custom(2000),
                "Testing",
                FeeRate::from_sat_per_vb(1.0),
                None,
                None,
                None,
            )
            .unwrap();

        let approved_a: ApprovedProposal = proposal.approve(&seed, Vec::new(), network).unwrap();

        proposal.finalize(vec![approved_a], network).unwrap();
    }
}
