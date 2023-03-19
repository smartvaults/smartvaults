use std::collections::HashMap;

use coinstr_core::bdk::blockchain::ElectrumBlockchain;
use coinstr_core::bdk::database::MemoryDatabase;
use coinstr_core::bdk::descriptor::policy::{PkOrF, SatisfiableItem};
use coinstr_core::bdk::electrum_client::Client as ElectrumClient;
use coinstr_core::bdk::wallet::AddressIndex;
use coinstr_core::bdk::{KeychainKind, SyncOptions, Wallet};
use coinstr_core::bitcoin::util::bip32::ExtendedPubKey;
use coinstr_core::bitcoin::Network;
use coinstr_core::nostr_sdk::prelude::{ToBech32, XOnlyPublicKey};
use coinstr_core::nostr_sdk::{EventId, Metadata, SECP256K1};
use coinstr_core::policy::Policy;
use coinstr_core::proposal::SpendingProposal;
use coinstr_core::types::Purpose;
use coinstr_core::util::bip::bip32::Bip32RootKey;
use coinstr_core::{Keychain, Result};
use owo_colors::colors::css::Lime;
use owo_colors::colors::xterm::{BrightElectricViolet, Pistachio, UserBrightWhite};
use owo_colors::colors::{BrightCyan, Magenta};
use owo_colors::OwoColorize;
use prettytable::{row, Table};
use termtree::Tree;

mod format;

pub fn print_secrets(keychain: Keychain, network: Network) -> Result<()> {
    let mnemonic = keychain.seed.mnemonic();
    let passphrase = keychain.seed.passphrase();

    println!();

    println!("Mnemonic: {}", mnemonic);
    if let Some(passphrase) = passphrase {
        println!("Passphrase: {}", passphrase);
    }

    let keys = keychain.nostr_keys()?;

    println!("\nNostr");
    println!(" Bech32 Keys");
    println!("  Public   : {} ", keys.public_key().to_bech32()?);
    println!("  Private  : {} ", keys.secret_key()?.to_bech32()?);
    println!(" Hex Keys");
    println!("  Public   : {} ", keys.public_key());
    println!("  Private  : {} ", keys.secret_key()?.display_secret());
    println!(
        "  Normalized Public   : {} ",
        keys.secret_key()?.public_key(SECP256K1)
    );

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

pub fn print_contacts(contacts: HashMap<XOnlyPublicKey, Metadata>) {
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

pub fn print_policy<S>(
    policy: Policy,
    policy_id: EventId,
    wallet: Wallet<MemoryDatabase>,
    endpoint: S,
) -> Result<()>
where
    S: Into<String>,
{
    println!("{}", "\nPolicy".fg::<UserBrightWhite>().underline());
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
    println!("{}", "Balances".fg::<UserBrightWhite>().underline());
    println!(
        "- Immature            	: {} sats",
        format::number(balance.immature)
    );
    println!(
        "- Trusted pending     	: {} sats",
        format::number(balance.trusted_pending)
    );
    println!(
        "- Untrusted pending   	: {} sats",
        format::number(balance.untrusted_pending)
    );
    println!(
        "- Confirmed           	: {} sats",
        format::number(balance.confirmed)
    );

    println!(
        "\n{}: {}\n",
        "Deposit address".fg::<UserBrightWhite>().underline(),
        wallet.get_address(AddressIndex::New)?
    );

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

pub fn print_policies(policies: Vec<(EventId, Policy)>) {
    let mut table = Table::new();

    table.set_titles(row!["#", "ID", "Name", "Description"]);

    for (index, (policy_id, policy)) in policies.into_iter().enumerate() {
        table.add_row(row![index + 1, policy_id, policy.name, policy.description]);
    }

    table.printstd();
}

pub fn print_proposals(proposals: Vec<(EventId, SpendingProposal, EventId)>) {
    let mut table = Table::new();

    table.set_titles(row!["#", "ID", "Policy ID", "Memo", "Address", "Amount"]);

    for (index, (proposal_id, proposal, policy_id)) in proposals.into_iter().enumerate() {
        table.add_row(row![
            index + 1,
            proposal_id,
            policy_id.to_hex()[..9],
            proposal.memo,
            proposal.to_address,
            format!("{} sats", format::number(proposal.amount))
        ]);
    }

    table.printstd();
}
