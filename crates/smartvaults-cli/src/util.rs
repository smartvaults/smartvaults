// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::{BTreeMap, BTreeSet, HashMap};

use owo_colors::colors::css::Lime;
use owo_colors::colors::xterm::{BlazeOrange, BrightElectricViolet, Pistachio};
use owo_colors::colors::{BrightCyan, Magenta};
use owo_colors::OwoColorize;
use prettytable::{row, Table};
use smartvaults_sdk::core::bdk::chain::ConfirmationTime;
use smartvaults_sdk::core::bdk::descriptor::policy::{PkOrF, SatisfiableItem};
use smartvaults_sdk::core::bips::bip32::Bip32;
use smartvaults_sdk::core::bitcoin::bip32::ExtendedPubKey;
use smartvaults_sdk::core::bitcoin::{Network, ScriptBuf};
use smartvaults_sdk::core::proposal::{CompletedProposal, Proposal};
use smartvaults_sdk::core::{Keychain, Purpose, Result, SECP256K1};
use smartvaults_sdk::nostr::prelude::{FromMnemonic, NostrConnectURI, ToBech32};
use smartvaults_sdk::nostr::{EventId, Keys, Profile, PublicKey, Relay, Timestamp, Url};
use smartvaults_sdk::types::{
    GetAddress, GetCompletedProposal, GetPolicy, GetProposal, GetSigner, GetSignerOffering,
    GetTransaction, GetUtxo, NostrConnectRequest,
};
use smartvaults_sdk::util::{self, format};
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
    let descriptors = keychain.descriptors(network, None, &SECP256K1)?;
    let external = descriptors.get_by_purpose(Purpose::BIP86, false).unwrap();
    let internal = descriptors.get_by_purpose(Purpose::BIP86, true).unwrap();

    println!("\nBitcoin");
    println!("  Root Private Key: {root_key}");
    println!(
        "  Extended Pub Key: {}",
        ExtendedPubKey::from_priv(&SECP256K1, &root_key)
    );
    println!("  Output Descriptor: {external}");
    println!("  Change Descriptor: {internal}");

    Ok(())
}

pub fn print_contacts(contacts: BTreeSet<Profile>) {
    let mut table = Table::new();

    table.set_titles(row![
        "#",
        "Public key",
        "Username",
        "Display name",
        "NIP-05",
    ]);

    for (index, user) in contacts.into_iter().enumerate() {
        let metadata = user.metadata();
        table.add_row(row![
            index + 1,
            user.public_key(),
            metadata.name.unwrap_or_default(),
            metadata.display_name.unwrap_or_default(),
            metadata.nip05.unwrap_or_default()
        ]);
    }

    table.printstd();
}

pub fn print_policy(
    vault: GetPolicy,
    policy_id: EventId,
    item: SatisfiableItem,
    address: GetAddress,
    txs: BTreeSet<GetTransaction>,
    utxos: Vec<GetUtxo>,
) {
    println!("{}", "\nPolicy".fg::<BlazeOrange>().underline());
    println!("- ID: {policy_id}");
    println!("- Name: {}", &vault.name);
    println!("- Description: {}", vault.description);

    let mut tree: Tree<String> = Tree::new("- Descriptor".to_string());
    tree.push(add_node(&item));
    println!("{tree}");

    println!("{}", "Balances".fg::<BlazeOrange>().underline());
    println!(
        "- Immature            	: {} sat",
        format::number(vault.balance.immature)
    );
    println!(
        "- Trusted pending     	: {} sat",
        format::number(vault.balance.trusted_pending)
    );
    println!(
        "- Untrusted pending   	: {} sat",
        format::number(vault.balance.untrusted_pending)
    );
    println!(
        "- Confirmed           	: {} sat",
        format::number(vault.balance.confirmed)
    );

    println!(
        "\n{}: {}\n",
        "Deposit address".fg::<BlazeOrange>().underline(),
        address.address.assume_checked()
    );

    if !txs.is_empty() {
        println!(
            "{}",
            "Latest 10 transactions".fg::<BlazeOrange>().underline()
        );
        print_txs(txs, 10);
    }

    println!();

    if !utxos.is_empty() {
        println!("{}", "Latest 10 UTXOs".fg::<BlazeOrange>().underline());
        print_utxos(utxos, 10);
    }
}

pub fn print_txs(txs: BTreeSet<GetTransaction>, limit: usize) {
    let mut table = Table::new();

    table.set_titles(row![
        "#",
        "Txid",
        "Sent",
        "Received",
        "Total",
        "Label",
        "Date/Time"
    ]);

    for (index, GetTransaction { tx, label, .. }) in txs.into_iter().take(limit).enumerate() {
        let (total, positive): (u64, bool) = {
            let received: i64 = tx.received as i64;
            let sent: i64 = tx.sent as i64;
            let tot = received - sent;
            let positive = tot >= 0;
            (tot.unsigned_abs(), positive)
        };
        table.add_row(row![
            index + 1,
            tx.txid(),
            format!("{} sat", format::number(tx.sent)),
            format!("{} sat", format::number(tx.received)),
            format!(
                "{}{} sat",
                if positive { "+" } else { "-" },
                format::number(total)
            ),
            label.unwrap_or_else(|| String::from("-")),
            match tx.confirmation_time {
                ConfirmationTime::Confirmed { time, .. } =>
                    Timestamp::from(time).to_human_datetime(),
                ConfirmationTime::Unconfirmed { .. } => String::from("Pending"),
            }
        ]);
    }

    table.printstd();
}

pub fn print_utxos(utxos: Vec<GetUtxo>, limit: usize) {
    let mut table = Table::new();

    table.set_titles(row![
        "#",
        "UTXO",
        "Value",
        "Label",
        "Block Height",
        "Frozen"
    ]);

    for (
        index,
        GetUtxo {
            utxo,
            label,
            frozen,
        },
    ) in utxos.into_iter().take(limit).enumerate()
    {
        table.add_row(row![
            index + 1,
            utxo.outpoint.to_string(),
            format!("{} sat", format::number(utxo.txout.value)),
            label.unwrap_or_else(|| String::from("-")),
            match utxo.confirmation_time {
                ConfirmationTime::Confirmed { height, .. } => format::number(height as u64),
                ConfirmationTime::Unconfirmed { .. } => String::from("Pending"),
            },
            frozen
        ]);
    }

    table.printstd();
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

pub fn print_policies(policies: Vec<GetPolicy>) {
    let mut table = Table::new();

    table.set_titles(row!["#", "ID", "Name", "Description"]);

    for (
        index,
        GetPolicy {
            policy_id, vault, ..
        },
    ) in policies.into_iter().enumerate()
    {
        table.add_row(row![index + 1, policy_id, vault.name, vault.description]);
    }

    table.printstd();
}

pub fn print_proposal(proposal: GetProposal) {
    let GetProposal {
        proposal_id,
        policy_id,
        proposal,
        signed,
        ..
    } = proposal;
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
            println!("- To address: {}", to_address.assume_checked());
            println!("- Amount: {amount}");
            println!("- Signed: {signed}");
        }
        Proposal::KeyAgentPayment {
            signer_descriptor,
            amount,
            description,
            ..
        } => {
            println!("- Type: key-agent-payment");
            println!("- Description: {description}");
            println!("- Signer: {signer_descriptor}");
            println!("- Amount: {amount}");
            println!("- Signed: {signed}");
        }
        Proposal::ProofOfReserve { message, .. } => {
            println!("- Type: proof-of-reserve");
            println!("- Message: {message}");
        }
    }
    println!();
}

pub fn print_proposals(proposals: Vec<GetProposal>) {
    let mut table = Table::new();

    table.set_titles(row![
        "#",
        "ID",
        "Policy ID",
        "Type",
        "Desc/Msg",
        "Address/Signer",
        "Amount",
        "Signed",
    ]);

    for (
        index,
        GetProposal {
            proposal_id,
            policy_id,
            proposal,
            signed,
            ..
        },
    ) in proposals.into_iter().enumerate()
    {
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
                    to_address.assume_checked(),
                    format!("{} sat", format::number(amount)),
                    signed
                ]);
            }
            Proposal::KeyAgentPayment {
                signer_descriptor,
                amount,
                description,
                ..
            } => {
                table.add_row(row![
                    index + 1,
                    proposal_id,
                    util::cut_event_id(policy_id),
                    "key-agent-payment",
                    description,
                    signer_descriptor,
                    format!("{} sat", format::number(amount)),
                    signed
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
                    "-",
                    signed,
                ]);
            }
        }
    }

    table.printstd();
}

pub fn print_completed_proposals(proposals: Vec<GetCompletedProposal>) {
    let mut table = Table::new();

    table.set_titles(row!["#", "ID", "Policy ID", "Type", "Txid", "Description"]);

    for (
        index,
        GetCompletedProposal {
            policy_id,
            completed_proposal_id,
            proposal,
            ..
        },
    ) in proposals.into_iter().enumerate()
    {
        match proposal {
            CompletedProposal::Spending {
                tx, description, ..
            } => {
                table.add_row(row![
                    index + 1,
                    completed_proposal_id,
                    util::cut_event_id(policy_id),
                    "spending",
                    tx.txid(),
                    description,
                ]);
            }
            CompletedProposal::KeyAgentPayment {
                tx, description, ..
            } => {
                table.add_row(row![
                    index + 1,
                    completed_proposal_id,
                    util::cut_event_id(policy_id),
                    "key-agent-payment",
                    tx.txid(),
                    description,
                ]);
            }
            CompletedProposal::ProofOfReserve { message, .. } => {
                table.add_row(row![
                    index + 1,
                    completed_proposal_id,
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

pub fn print_signers(signers: Vec<GetSigner>) {
    let mut table = Table::new();

    table.set_titles(row!["#", "ID", "Name", "Fingerprint", "Type",]);

    for (index, GetSigner { signer_id, signer }) in signers.into_iter().enumerate() {
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
        "Sent (bytes)",
        "Received (bytes)",
        "Queue",
        "Latency",
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
            format::big_number(stats.bytes_sent() as u64),
            format::big_number(stats.bytes_received() as u64),
            relay.queue(),
            match stats.latency().await {
                Some(latency) => format!("{} ms", latency.as_millis()),
                None => String::from("-"),
            },
            if stats.connected_at() == Timestamp::from(0) {
                String::from("-")
            } else {
                stats.connected_at().to_human_datetime()
            }
        ]);
    }

    table.printstd();
}

pub fn print_addresses(addresses: Vec<GetAddress>, balances: HashMap<ScriptBuf, u64>) {
    let mut table = Table::new();

    table.set_titles(row!["#", "Address", "Label", "Balance"]);

    for (index, GetAddress { address, label }) in addresses.into_iter().enumerate() {
        table.add_row(row![
            index + 1,
            address.clone().assume_checked().to_string(),
            label.unwrap_or_else(|| String::from("-")),
            format!(
                "{} sat",
                format::number(
                    balances
                        .get(&address.payload.script_pubkey())
                        .copied()
                        .unwrap_or_default()
                )
            )
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

pub fn print_requests(requests: Vec<NostrConnectRequest>) -> Result<()> {
    let mut table = Table::new();

    table.set_titles(row![
        "#",
        "Event ID",
        "App Public Key",
        "Method",
        "Requested at",
    ]);

    for (index, req) in requests.into_iter().enumerate() {
        table.add_row(row![
            index + 1,
            req.event_id,
            util::cut_public_key(req.app_public_key),
            req.message.to_request()?.method(),
            req.timestamp.to_human_datetime(),
        ]);
    }

    table.printstd();

    Ok(())
}

pub fn print_authorizations(authorizations: BTreeMap<PublicKey, Timestamp>) {
    let mut table = Table::new();

    table.set_titles(row!["#", "App Public Key", "Authorized until",]);

    for (index, (app_public_key, until)) in authorizations.into_iter().enumerate() {
        table.add_row(row![index + 1, app_public_key, until.to_human_datetime(),]);
    }

    table.printstd();
}

pub fn print_key_agents_signer_offersing<I>(offerings: I)
where
    I: IntoIterator<Item = GetSignerOffering>,
{
    let mut table = Table::new();

    table.set_titles(row![
        "#",
        "Name",
        "Fingerprint",
        "Temperature",
        "Response time",
        "Device type",
        "Cost per signature",
        "Yearly cost (BSP)",
        "Yearly cost"
    ]);

    for (
        index,
        GetSignerOffering {
            signer, offering, ..
        },
    ) in offerings.into_iter().enumerate()
    {
        table.add_row(row![
            index + 1,
            signer.name(),
            signer.fingerprint(),
            offering.temperature,
            offering
                .response_time
                .map(|p| format!("{p} min"))
                .unwrap_or_default(),
            offering.device_type,
            offering
                .cost_per_signature
                .map(|p| p.to_string())
                .unwrap_or_default(),
            offering
                .yearly_cost_basis_points
                .map(|p| p.to_string())
                .unwrap_or_default(),
            offering
                .yearly_cost
                .map(|p| p.to_string())
                .unwrap_or_default(),
        ]);
    }

    table.printstd();
}
