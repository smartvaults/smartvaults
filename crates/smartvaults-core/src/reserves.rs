// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use keechain_core::bdk::chain::{ConfirmationTime, PersistBackend};
use keechain_core::bdk::wallet::tx_builder::{AddForeignUtxoError, TxOrdering};
use keechain_core::bdk::wallet::{ChangeSet, Wallet};
use keechain_core::bitcoin::address::Payload;
use keechain_core::bitcoin::blockdata::opcodes;
use keechain_core::bitcoin::blockdata::script::{Builder, Script};
use keechain_core::bitcoin::blockdata::transaction::{OutPoint, TxIn, TxOut};
use keechain_core::bitcoin::consensus::encode::serialize;
use keechain_core::bitcoin::hash_types::{PubkeyHash, Txid};
use keechain_core::bitcoin::hashes::{hash160, sha256d, Hash};
use keechain_core::bitcoin::psbt::{Input, PartiallySignedTransaction};
use keechain_core::bitcoin::sighash::EcdsaSighashType;
use keechain_core::bitcoin::{Address, Network, Sequence};

/// Proof error
#[derive(Debug, thiserror::Error)]
pub enum ProofError {
    /// Less than two inputs
    #[error("wrong number of inputs")]
    WrongNumberOfInputs,
    /// Must have exactly 1 output
    #[error("wrong number of outputs")]
    WrongNumberOfOutputs,
    /// Challenge input does not match
    #[error("challenge input does not match")]
    ChallengeInputMismatch,
    /// Found an input other than the challenge which is not spendable. Holds the position of the input.
    #[error("found an input other than the challenge which is not spendable at position {0}")]
    NonSpendableInput(usize),
    /// Found an input that has no signature at position
    #[error("found an input that has no signature at position {0}")]
    NotSignedInput(usize),
    /// Found an input with an unsupported SIGHASH type at position
    #[error("unsupported sighash type at position {0}")]
    UnsupportedSighashType(usize),
    /// Found an input that is neither witness nor legacy at position
    #[error("found an input that is neither witness nor legacy at position {0}")]
    NeitherWitnessNorLegacy(usize),
    /// Signature validation failed
    #[error("signature validation failed: {1}")]
    SignatureValidation(usize, String),
    /// The output is not valid
    #[error("invalid output")]
    InvalidOutput,
    /// Input and output values are not equal, implying a miner fee
    #[error("input and output values are not equal")]
    InAndOutValueNotEqual,
    /// No matching outpoint found
    #[error("no matching outpoint found: {0}")]
    OutpointNotFound(usize),
    /// Failed to retrieve the block height of a Tx or UTXO
    #[error("missing confirmation info")]
    MissingConfirmationInfo,
    /// BDK Error
    #[error(transparent)]
    BdkAddForeignUtxo(#[from] AddForeignUtxoError),
    #[error("{0}")]
    BdkCreateTx(String),
}

/// The API for proof of reserves
pub trait ProofOfReserves {
    /// Create a proof for all spendable UTXOs in a wallet
    fn create_proof<S>(&mut self, message: S) -> Result<PartiallySignedTransaction, ProofError>
    where
        S: Into<String>;

    /// Make sure this is a proof, and not a spendable transaction.
    /// Make sure the proof is valid.
    /// Currently proofs can only be validated against the tip of the chain.
    /// If some of the UTXOs in the proof were spent in the meantime, the proof will fail.
    /// We can currently not validate whether it was valid at a certain block height.
    /// With the max_block_height parameter the caller can ensure that only UTXOs with sufficient confirmations are considered.
    /// If no max_block_height is provided, also UTXOs from transactions in the mempool are considered.
    /// Returns the spendable amount of the proof.
    fn verify_proof<S>(
        &self,
        psbt: &PartiallySignedTransaction,
        message: S,
        max_block_height: Option<u32>,
    ) -> Result<u64, ProofError>
    where
        S: Into<String>;
}

impl<D> ProofOfReserves for Wallet<D>
where
    D: PersistBackend<ChangeSet>,
{
    fn create_proof<S>(&mut self, message: S) -> Result<PartiallySignedTransaction, ProofError>
    where
        S: Into<String>,
    {
        let message: &str = &message.into();
        if message.is_empty() {
            return Err(ProofError::ChallengeInputMismatch);
        }

        let challenge_txin = challenge_txin(message);
        let challenge_psbt_inp = Input {
            witness_utxo: Some(TxOut {
                value: 0,
                script_pubkey: Builder::new().push_opcode(opcodes::OP_TRUE).into_script(),
            }),
            final_script_sig: Some(Script::builder().into_script()), /* "finalize" the input with an empty scriptSig */
            ..Default::default()
        };

        let pkh = PubkeyHash::from_raw_hash(hash160::Hash::hash(&[0]));
        let out_script_unspendable =
            Address::new(self.network(), Payload::PubkeyHash(pkh)).script_pubkey();

        let psbt = {
            let mut builder = self.build_tx();
            builder
                .drain_wallet()
                .add_foreign_utxo(challenge_txin.previous_output, challenge_psbt_inp, 42)?
                .fee_absolute(0)
                .only_witness_utxo()
                .current_height(0)
                .drain_to(out_script_unspendable)
                .ordering(TxOrdering::Untouched);

            builder
                .finish()
                .map_err(|e| ProofError::BdkCreateTx(format!("{e:?}")))?
        };

        Ok(psbt)
    }

    fn verify_proof<S>(
        &self,
        psbt: &PartiallySignedTransaction,
        message: S,
        max_block_height: Option<u32>,
    ) -> Result<u64, ProofError>
    where
        S: Into<String>,
    {
        // verify the proof UTXOs are still spendable
        let unspents = self
            .list_unspent()
            .map(|utxo| {
                if max_block_height.is_none() {
                    Ok((utxo, None))
                } else if let Some(tx_details) = self.get_tx(utxo.outpoint.txid) {
                    match tx_details.chain_position.cloned().into() {
                        ConfirmationTime::Confirmed { height, .. } => Ok((utxo, Some(height))),
                        ConfirmationTime::Unconfirmed { .. } => Ok((utxo, None)),
                    }
                } else {
                    Err(ProofError::MissingConfirmationInfo)
                }
            })
            .collect::<Result<Vec<_>, ProofError>>()?;

        let outpoints = unspents
            .iter()
            .filter(|(_utxo, block_height)| {
                block_height.unwrap_or(u32::MAX) <= max_block_height.unwrap_or(u32::MAX)
            })
            .map(|(utxo, _)| (utxo.outpoint, utxo.txout.clone()))
            .collect();

        verify_proof(psbt, message, outpoints, self.network())
    }
}

/// Make sure this is a proof, and not a spendable transaction.
/// Make sure the proof is valid.
/// Currently proofs can only be validated against the tip of the chain.
/// If some of the UTXOs in the proof were spent in the meantime, the proof will fail.
/// We can currently not validate whether it was valid at a certain block height.
/// Since the caller provides the outpoints, he is also responsible to make sure they have enough confirmations.
/// Returns the spendable amount of the proof.
fn verify_proof<S>(
    psbt: &PartiallySignedTransaction,
    message: S,
    outpoints: Vec<(OutPoint, TxOut)>,
    network: Network,
) -> Result<u64, ProofError>
where
    S: Into<String>,
{
    let tx = psbt.clone().extract_tx();

    if tx.output.len() != 1 {
        return Err(ProofError::WrongNumberOfOutputs);
    }
    if tx.input.len() <= 1 {
        return Err(ProofError::WrongNumberOfInputs);
    }

    // verify the challenge txin
    let challenge_txin = challenge_txin(message);
    if tx.input[0].previous_output != challenge_txin.previous_output {
        return Err(ProofError::ChallengeInputMismatch);
    }

    // verify the proof UTXOs are still spendable
    if let Some((i, _inp)) = tx
        .input
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_i, inp)| !outpoints.iter().any(|op| op.0 == inp.previous_output))
    {
        return Err(ProofError::NonSpendableInput(i));
    }

    // verify that the inputs are signed, except the challenge
    if let Some((i, _inp)) = psbt
        .inputs
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_i, inp)| inp.final_script_sig.is_none() && inp.final_script_witness.is_none())
    {
        return Err(ProofError::NotSignedInput(i));
    }

    // Verify the SIGHASH
    if let Some((i, _psbt_in)) = psbt.inputs.iter().enumerate().find(|(_i, psbt_in)| {
        psbt_in.sighash_type.is_some() && psbt_in.sighash_type != Some(EcdsaSighashType::All.into())
    }) {
        return Err(ProofError::UnsupportedSighashType(i));
    }

    let serialized_tx = serialize(&tx);
    // Verify the challenge input
    if let Some(utxo) = &psbt.inputs[0].witness_utxo {
        if let Err(err) = bitcoinconsensus::verify(
            utxo.script_pubkey.to_bytes().as_slice(),
            utxo.value,
            &serialized_tx,
            0,
        ) {
            return Err(ProofError::SignatureValidation(0, format!("{:?}", err)));
        }
    } else {
        return Err(ProofError::SignatureValidation(
            0,
            "witness_utxo not found for challenge input".to_string(),
        ));
    }
    // Verify other inputs against prevouts.
    if let Some((i, res)) = tx
        .input
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, tx_in)| {
            if let Some(op) = outpoints.iter().find(|op| op.0 == tx_in.previous_output) {
                (i, Ok(op.1.clone()))
            } else {
                (i, Err(ProofError::OutpointNotFound(i)))
            }
        })
        .map(|(i, res)| match res {
            Ok(txout) => (
                i,
                Ok(bitcoinconsensus::verify(
                    txout.script_pubkey.to_bytes().as_slice(),
                    txout.value,
                    &serialized_tx,
                    i,
                )),
            ),
            Err(err) => (i, Err(err)),
        })
        .find(|(_i, res)| res.is_err())
    {
        return Err(ProofError::SignatureValidation(
            i,
            format!("{:?}", res.err().unwrap()),
        ));
    }

    // calculate the spendable amount of the proof
    let sum = tx
        .input
        .iter()
        .map(|tx_in| {
            if let Some(op) = outpoints.iter().find(|op| op.0 == tx_in.previous_output) {
                op.1.value
            } else {
                0
            }
        })
        .sum();

    // verify the unspendable output
    let pkh = PubkeyHash::from_raw_hash(hash160::Hash::hash(&[0]));
    let out_script_unspendable = Address::new(network, Payload::PubkeyHash(pkh)).script_pubkey();
    if tx.output[0].script_pubkey != out_script_unspendable {
        return Err(ProofError::InvalidOutput);
    }

    // inflow and outflow being equal means no miner fee
    if tx.output[0].value != sum {
        return Err(ProofError::InAndOutValueNotEqual);
    }

    Ok(sum)
}

/// Construct a challenge input with the message
fn challenge_txin<S>(message: S) -> TxIn
where
    S: Into<String>,
{
    let message = "Proof-of-Reserves: ".to_string() + &message.into();
    let message = sha256d::Hash::hash(message.as_bytes());
    TxIn {
        previous_output: OutPoint::new(Txid::from_raw_hash(message), 0),
        sequence: Sequence(0xFFFFFFFF),
        ..Default::default()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::get_funded_wallet;

    // const PSBT_BASE64: &str = "cHNidP8BAH4BAAAAAmw1RvG4UzfnSafpx62EPTyha6VslP0Er7n3TxjEpeBeAAAAAAD/////FcB9C8LQwqAoYxGcM/YLhUt3XZIQUmFAlaJlBjVmFO8AAAAAAP////8BUMMAAAAAAAAZdqkUn3/QltN+0sDj9/DPySS+70/862iIrAAAAAAAAQEKAAAAAAAAAAABUQEHAAABAR9QwwAAAAAAABYAFOzlJlcQU9qGRUyeBmd56vnRUC5qIgYDKwVYB4vsOGlKhJM9ZZMD4lddrn6RaFkRRUEVv9ZEh+ME7OUmVwAA";
    const MESSAGE: &str = "This belongs to me.";
    const DESCRIPTOR: &str = "wpkh(cVpPVruEDdmutPzisEsYvtST1usBR3ntr8pXSyt6D2YYqXRyPcFW)";

    // #[test]
    // fn test_proof() {
    // let mut wallet = get_funded_wallet(DESCRIPTOR).unwrap();
    // let psbt = wallet.create_proof(MESSAGE).unwrap();
    // assert_eq!(psbt.to_string(), PSBT_BASE64);
    // }

    #[test]
    #[should_panic(
        expected = "Miniscript(Unexpected(\"unexpected «Key too short (<66 char), doesn't match any format»\"))"
    )]
    fn invalid_descriptor() {
        let descriptor = "wpkh(cVpPVqXRyPcFW)";
        let mut wallet = get_funded_wallet(descriptor).unwrap();
        let _psbt = wallet.create_proof(MESSAGE).unwrap();
    }

    #[test]
    #[should_panic(expected = "ChallengeInputMismatch")]
    fn empty_message() {
        let mut wallet = get_funded_wallet(DESCRIPTOR).unwrap();

        let message = "";
        let _psbt = wallet.create_proof(message).unwrap();
    }

    fn get_signed_proof() -> PartiallySignedTransaction {
        let mut wallet = get_funded_wallet(DESCRIPTOR).unwrap();
        let psbt = wallet.create_proof(MESSAGE).unwrap();
        psbt
    }

    // #[test]
    // fn verify_internal() {
    // let wallet = get_funded_wallet(DESCRIPTOR).unwrap();
    // let psbt = get_signed_proof();
    // let spendable = wallet.verify_proof(&psbt, MESSAGE, None).unwrap();
    // assert_eq!(spendable, 50_000);
    // }

    #[test]
    #[should_panic(expected = "NonSpendableInput")]
    fn verify_internal_90() {
        let wallet = get_funded_wallet(DESCRIPTOR).unwrap();

        let psbt = get_signed_proof();
        let spendable = wallet.verify_proof(&psbt, MESSAGE, Some(90)).unwrap();
        assert_eq!(spendable, 50_000);
    }

    // #[test]
    // fn verify_internal_100() {
    // let wallet = get_funded_wallet(DESCRIPTOR).unwrap();
    //
    // let psbt = get_signed_proof();
    // let spendable = wallet.verify_proof(&psbt, MESSAGE, Some(100)).unwrap();
    // assert_eq!(spendable, 50_000);
    // }

    // #[test]
    // fn verify_external() {
    // let wallet = get_funded_wallet(DESCRIPTOR).unwrap();
    //
    // let psbt = get_signed_proof();
    // let unspents = wallet.list_unspent();
    // let outpoints = unspents
    // .map(|utxo| (utxo.outpoint, utxo.txout.clone()))
    // .collect();
    // let spendable = verify_proof(&psbt, MESSAGE, outpoints, Network::Testnet).unwrap();
    //
    // assert_eq!(spendable, 50_000);
    // }

    #[test]
    #[should_panic(expected = "ChallengeInputMismatch")]
    fn wrong_message() {
        let wallet = get_funded_wallet(DESCRIPTOR).unwrap();
        let message = "Wrong message!";
        let psbt = get_signed_proof();
        wallet.verify_proof(&psbt, message, None).unwrap();
    }

    #[test]
    #[should_panic(expected = "WrongNumberOfInputs")]
    fn too_few_inputs() {
        let wallet = get_funded_wallet(DESCRIPTOR).unwrap();

        let mut psbt = get_signed_proof();
        psbt.unsigned_tx.input.truncate(1);
        psbt.inputs.truncate(1);

        wallet.verify_proof(&psbt, MESSAGE, None).unwrap();
    }

    #[test]
    #[should_panic(expected = "WrongNumberOfOutputs")]
    fn no_output() {
        let wallet = get_funded_wallet(DESCRIPTOR).unwrap();

        let mut psbt = get_signed_proof();
        psbt.unsigned_tx.output.clear();
        psbt.inputs.clear();

        wallet.verify_proof(&psbt, MESSAGE, None).unwrap();
    }

    #[test]
    #[should_panic(expected = "NotSignedInput")]
    fn missing_signature() {
        let wallet = get_funded_wallet(DESCRIPTOR).unwrap();

        let mut psbt = get_signed_proof();
        psbt.inputs[1].final_script_sig = None;
        psbt.inputs[1].final_script_witness = None;

        wallet.verify_proof(&psbt, MESSAGE, None).unwrap();
    }

    // #[test]
    // #[should_panic(expected = "UnsupportedSighashType(1)")]
    // fn wrong_sighash_type() {
    // let wallet = get_funded_wallet(DESCRIPTOR).unwrap();
    //
    // let mut psbt = get_signed_proof();
    // psbt.inputs[1].sighash_type = Some(EcdsaSighashType::SinglePlusAnyoneCanPay.into());
    //
    // wallet.verify_proof(&psbt, MESSAGE, None).unwrap();
    // }

    // #[test]
    // #[should_panic(expected = "InvalidOutput")]
    // fn invalid_output() {
    // let wallet = get_funded_wallet(DESCRIPTOR).unwrap();
    //
    // let mut psbt = get_signed_proof();
    //
    // let pkh = PubkeyHash::from_hash(hash160::Hash::hash(&[0, 1, 2, 3]));
    // let out_script_unspendable = Address {
    // payload: Payload::PubkeyHash(pkh),
    // network: Network::Testnet,
    // }
    // .script_pubkey();
    // psbt.unsigned_tx.output[0].script_pubkey = out_script_unspendable;
    //
    // wallet.verify_proof(&psbt, MESSAGE, None).unwrap();
    // }

    // #[test]
    // #[should_panic(expected = "InAndOutValueNotEqual")]
    // fn sum_mismatch() {
    // let wallet = get_funded_wallet(DESCRIPTOR).unwrap();
    //
    // let mut psbt = get_signed_proof();
    // psbt.unsigned_tx.output[0].value = 123;
    //
    // wallet.verify_proof(&psbt, MESSAGE, None).unwrap();
    // }
}
