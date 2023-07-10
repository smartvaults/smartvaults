// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;

use coinstr_sdk::core::bdk::blockchain::ElectrumBlockchain;
use coinstr_sdk::core::bdk::database::{BatchDatabase, MemoryDatabase};
use coinstr_sdk::core::bdk::descriptor::policy::{PkOrF, SatisfiableItem};
use coinstr_sdk::core::bdk::electrum_client::Client as ElectrumClient;
use coinstr_sdk::core::bdk::wallet::AddressIndex;
use coinstr_sdk::core::bdk::{KeychainKind, SyncOptions, Wallet};
use coinstr_sdk::core::bips::bip32::Bip32;
use coinstr_sdk::core::bitcoin::util::bip32::ExtendedPubKey;
use coinstr_sdk::core::bitcoin::Network;
use coinstr_sdk::core::policy::Policy;
use coinstr_sdk::core::proposal::{CompletedProposal, Proposal};
use coinstr_sdk::core::signer::Signer;
use coinstr_sdk::core::types::Purpose;
use coinstr_sdk::core::{Keychain, Result};
use coinstr_sdk::db::model::{GetPolicyResult, NostrConnectRequest};
use coinstr_sdk::nostr::prelude::{FromMnemonic, NostrConnectURI, ToBech32, XOnlyPublicKey};
use coinstr_sdk::nostr::{EventId, Keys, Metadata, Relay, Timestamp, Url, SECP256K1};
use coinstr_sdk::util::{self, format};
use owo_colors::colors::css::Lime;
use owo_colors::colors::xterm::{BlazeOrange, BrightElectricViolet, Pistachio};
use owo_colors::colors::{BrightCyan, Magenta};
use owo_colors::OwoColorize;
use prettytable::{row, Table};
use termtree::Tree;

pub fn print_secrets(keychain: Keychain, network: Network) -> Result<()> {
    let mnemonic = keychain.seed.mnemonic();
    let passphrase = keychain.seed.passphrase();

    println!();

    println!("Mnemonic: {}", mnemonic);
    if let Some(passphrase) = passphrase {
        println!("Passphrase: {}", passphrase);
    }

    let keys = Keys::from_mnemonic(
        keychain.seed.mnemonic().to_string(),
        keychain.seed.passphrase(),
    )?;

    println!("\nNostr");
    println!(" Bech32 Keys");
    println!("  Public   : {} ", keys.public_key().to_bech32()?);
    println!("  Private  : {} ", keys.secret_key()?.to_bech32()?);
    println!(" Hex Keys");
    println!("  Public   : {} ", keys.public_key());
    println!("  Private  : {} ", keys.secret_key()?.display_secret());

    let root_key = keychain.seed.to_bip32_root_key(network)?;
    let descriptors = keychain.descriptors(network, None)?;
    let external = descriptors.get_by_purpose(Purpose::TR, false).unwrap();
    let internal = descriptors.get_by_purpose(Purpose::TR, true).unwrap();
    let wallet = Wallet::new(
        external.clone(),
        Some(internal.clone()),
        network,
        MemoryDatabase::new(),
    )
    .unwrap();

    println!("\nBitcoin");
    println!("  Root Private Key: {root_key}");
    println!(
        "  Extended Pub Key: {}",
        ExtendedPubKey::from_priv(SECP256K1, &root_key)
    );
    println!("  Output Descriptor: {external}");
    println!("  Change Descriptor: {internal}");
    println!(
        "  Ext Address 1: {}",
        wallet.get_address(AddressIndex::New).unwrap()
    );
    println!(
        "  Ext Address 2: {}",
        wallet.get_address(AddressIndex::New).unwrap()
    );
    println!(
        "  Change Address: {}",
        wallet.get_internal_address(AddressIndex::New).unwrap()
    );

    Ok(())
}

pub fn print_contacts(contacts: BTreeMap<XOnlyPublicKey, Metadata>) {
    let mut table = Table::new();

    table.set_titles(row![
        "#",
        "Public key",
        "Username",
        "Display name",
        "NIP-05",
    ]);

    for (index, (public_key, metadata)) in contacts.into_iter().enumerate() {
        table.add_row(row![
            index + 1,
            public_key,
            metadata.name.unwrap_or_default(),
            metadata.display_name.unwrap_or_default(),
            metadata.nip05.unwrap_or_default()
        ]);
    }

    table.printstd();
}

pub fn print_policy<D, S>(
    policy: Policy,
    policy_id: EventId,
    wallet: Wallet<D>,
    endpoint: S,
) -> Result<()>
where
    D: BatchDatabase,
    S: Into<String>,
{
    println!("{}", "\nPolicy".fg::<BlazeOrange>().underline());
    println!("- ID: {policy_id}");
    println!("- Name: {}", &policy.name);
    println!("- Description: {}", policy.description);

    let spending_policy = wallet.policies(KeychainKind::External)?.unwrap();

    let mut tree: Tree<String> = Tree::new("- Descriptor".to_string());
    tree.push(add_node(&spending_policy.item));
    println!("{tree}");

    let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&endpoint.into())?);
    wallet.sync(&blockchain, SyncOptions::default())?;

    let balance = wallet.get_balance()?;
    println!("{}", "Balances".fg::<BlazeOrange>().underline());
    println!(
        "- Immature            	: {} sat",
        format::number(balance.immature)
    );
    println!(
        "- Trusted pending     	: {} sat",
        format::number(balance.trusted_pending)
    );
    println!(
        "- Untrusted pending   	: {} sat",
        format::number(balance.untrusted_pending)
    );
    println!(
        "- Confirmed           	: {} sat",
        format::number(balance.confirmed)
    );

    println!(
        "\n{}: {}\n",
        "Deposit address".fg::<BlazeOrange>().underline(),
        wallet.get_address(AddressIndex::New)?
    );

    let mut txs = wallet.list_transactions(false)?;
    txs.sort_by(|a, b| {
        b.confirmation_time
            .as_ref()
            .map(|t| t.height)
            .cmp(&a.confirmation_time.as_ref().map(|t| t.height))
    });
    if !txs.is_empty() {
        println!("{}", "Latest transactions".fg::<BlazeOrange>().underline());
        for tx in txs.into_iter().take(10) {
            println!("{tx:?}")
        }
    }

    Ok(())
}

fn display_key(key: &PkOrF) -> String {
    match key {
        PkOrF::Pubkey(pk) => format!("<pk:{pk}>"),
        PkOrF::XOnlyPubkey(pk) => format!("<xonly-pk:{pk}>"),
        PkOrF::Fingerprint(f) => format!("<fingerprint:{f}>"),
    }
}

fn add_node(item: &SatisfiableItem) -> Tree<String> {
    let mut si_tree: Tree<String> = Tree::new(format!(
        "{}{}",
        "id -> ".fg::<Pistachio>(),
        item.id().fg::<Pistachio>()
    ));

    match &item {
        SatisfiableItem::EcdsaSignature(key) => {
            si_tree.push(format!(
                "🗝️ {} {}",
                "ECDSA Sig of ".fg::<BrightElectricViolet>(),
                display_key(key)
            ));
        }
        SatisfiableItem::SchnorrSignature(key) => {
            si_tree.push(format!(
                "🔑 {} {}",
                "Schnorr Sig of ".fg::<Pistachio>(),
                display_key(key)
            ));
        }
        SatisfiableItem::Sha256Preimage { hash } => {
            si_tree.push(format!("SHA256 Preimage of {hash}"));
        }
        SatisfiableItem::Hash256Preimage { hash } => {
            si_tree.push(format!("Double-SHA256 Preimage of {hash}"));
        }
        SatisfiableItem::Ripemd160Preimage { hash } => {
            si_tree.push(format!("RIPEMD160 Preimage of {hash}"));
        }
        SatisfiableItem::Hash160Preimage { hash } => {
            si_tree.push(format!("Double-RIPEMD160 Preimage of {hash}"));
        }
        SatisfiableItem::AbsoluteTimelock { value } => {
            si_tree.push(format!(
                "⏰ {} {value}",
                "Absolute Timelock of ".fg::<Lime>()
            ));
        }
        SatisfiableItem::RelativeTimelock { value } => {
            si_tree.push(format!(
                "⏳ {} {value}",
                "Relative Timelock of".fg::<Lime>(),
            ));
        }
        SatisfiableItem::Multisig { keys, threshold } => {
            // si_tree.push(format!("🎚️ {} of {} MultiSig:", threshold, keys.len()));
            let mut child_tree: Tree<String> = Tree::new(format!(
                "🤝 {}{} of {}",
                "MultiSig  :  ".fg::<BrightCyan>(),
                threshold,
                keys.len()
            ));

            keys.iter().for_each(|x| {
                child_tree.push(format!("🔑 {}", display_key(x).fg::<Magenta>()));
            });
            si_tree.push(child_tree);
        }
        SatisfiableItem::Thresh { items, threshold } => {
            let mut child_tree: Tree<String> = Tree::new(format!(
                "👑{}{} of {} ",
                " Threshold Condition   : ".fg::<BrightCyan>(),
                threshold,
                items.len()
            ));

            items.iter().for_each(|x| {
                child_tree.push(add_node(&x.item));
            });
            si_tree.push(child_tree);
        }
    }
    si_tree
}

pub fn print_policies(policies: BTreeMap<EventId, GetPolicyResult>) {
    let mut table = Table::new();

    table.set_titles(row!["#", "ID", "Name", "Description"]);

    for (index, (policy_id, GetPolicyResult { policy, .. })) in policies.into_iter().enumerate() {
        table.add_row(row![index + 1, policy_id, policy.name, policy.description]);
    }

    table.printstd();
}

pub fn print_proposal(proposal_id: EventId, proposal: Proposal, policy_id: EventId) {
    println!();
    println!("- Proposal id: {proposal_id}");
    println!("- Policy id: {policy_id}");
    match proposal {
        Proposal::Spending {
            to_address,
            amount,
            description,
            ..
        } => {
            println!("- Type: spending");
            println!("- Description: {description}");
            println!("- To address: {to_address}");
            println!("- Amount: {amount}");
        }
        Proposal::ProofOfReserve { message, .. } => {
            println!("- Type: proof-of-reserve");
            println!("- Message: {message}");
        }
    }
    println!();
}

pub fn print_proposals(proposals: BTreeMap<EventId, (EventId, Proposal)>) {
    let mut table = Table::new();

    table.set_titles(row![
        "#",
        "ID",
        "Policy ID",
        "Type",
        "Desc/Msg",
        "Address",
        "Amount"
    ]);

    for (index, (proposal_id, (policy_id, proposal))) in proposals.into_iter().enumerate() {
        match proposal {
            Proposal::Spending {
                to_address,
                amount,
                description,
                ..
            } => {
                table.add_row(row![
                    index + 1,
                    proposal_id,
                    util::cut_event_id(policy_id),
                    "spending",
                    description,
                    to_address,
                    format!("{} sat", format::number(amount))
                ]);
            }
            Proposal::ProofOfReserve { message, .. } => {
                table.add_row(row![
                    index + 1,
                    proposal_id,
                    util::cut_event_id(policy_id),
                    "proof-of-reserve",
                    message,
                    "-",
                    "-"
                ]);
            }
        }
    }

    table.printstd();
}

pub fn print_completed_proposals(proposals: BTreeMap<EventId, (EventId, CompletedProposal)>) {
    let mut table = Table::new();

    table.set_titles(row!["#", "ID", "Policy ID", "Type", "Txid", "Description"]);

    for (index, (proposal_id, (policy_id, proposal))) in proposals.into_iter().enumerate() {
        match proposal {
            CompletedProposal::Spending {
                tx, description, ..
            } => {
                table.add_row(row![
                    index + 1,
                    proposal_id,
                    util::cut_event_id(policy_id),
                    "spending",
                    tx.txid(),
                    description,
                ]);
            }
            CompletedProposal::ProofOfReserve { message, .. } => {
                table.add_row(row![
                    index + 1,
                    proposal_id,
                    util::cut_event_id(policy_id),
                    "proof-of-reserve",
                    "-",
                    message,
                ]);
            }
        }
    }

    table.printstd();
}

pub fn print_signers(signers: BTreeMap<EventId, Signer>) {
    let mut table = Table::new();

    table.set_titles(row!["#", "ID", "Name", "Fingerprint", "Type",]);

    for (index, (signer_id, signer)) in signers.into_iter().enumerate() {
        table.add_row(row![
            index + 1,
            signer_id,
            signer.name(),
            signer.fingerprint(),
            signer.signer_type(),
        ]);
    }

    table.printstd();
}

pub async fn print_relays(relays: BTreeMap<Url, Relay>) {
    let mut table = Table::new();

    table.set_titles(row![
        "#",
        "Url",
        "Status",
        "Attemps",
        "Success",
        "Connected at"
    ]);

    for (index, (url, relay)) in relays.into_iter().enumerate() {
        let stats = relay.stats();
        table.add_row(row![
            index + 1,
            url,
            relay.status().await,
            stats.attempts(),
            stats.success(),
            if stats.connected_at() == Timestamp::from(0) {
                String::from("-")
            } else {
                stats.connected_at().to_human_datetime()
            }
        ]);
    }

    table.printstd();
}

pub fn print_sessions(sessions: Vec<(NostrConnectURI, Timestamp)>) {
    let mut table = Table::new();

    table.set_titles(row![
        "#",
        "App Name",
        "App Public Key",
        "Relay Url",
        "Connected at"
    ]);

    for (index, (uri, timestamp)) in sessions.into_iter().enumerate() {
        table.add_row(row![
            index + 1,
            uri.metadata.name,
            uri.public_key,
            uri.relay_url,
            timestamp.to_human_datetime(),
        ]);
    }

    table.printstd();
}

pub fn print_requests(requests: BTreeMap<EventId, NostrConnectRequest>) -> Result<()> {
    let mut table = Table::new();

    table.set_titles(row![
        "#",
        "Event ID",
        "App Public Key",
        "Method",
        "Requested at",
    ]);

    for (index, (event_id, req)) in requests.into_iter().enumerate() {
        table.add_row(row![
            index + 1,
            event_id,
            util::cut_public_key(req.app_public_key),
            req.message.to_request()?.method(),
            req.timestamp.to_human_datetime(),
        ]);
    }

    table.printstd();

    Ok(())
}

pub fn print_authorizations(authorizations: BTreeMap<XOnlyPublicKey, Timestamp>) {
    let mut table = Table::new();

    table.set_titles(row!["#", "App Public Key", "Authorized until",]);

    for (index, (app_public_key, until)) in authorizations.into_iter().enumerate() {
        table.add_row(row![index + 1, app_public_key, until.to_human_datetime(),]);
    }

    table.printstd();
}
