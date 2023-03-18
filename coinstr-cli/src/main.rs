use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use clap::Parser;
use cli::GetCommand;
use coinstr_core::bdk::blockchain::ElectrumBlockchain;
use coinstr_core::bdk::electrum_client::Client as ElectrumClient;
use coinstr_core::bdk::SyncOptions;
use coinstr_core::bip39::Mnemonic;
use coinstr_core::bitcoin::Network;
use coinstr_core::constants::{POLICY_KIND, SHARED_GLOBAL_KEY_KIND, SPENDING_PROPOSAL_KIND};
use coinstr_core::nostr_sdk::blocking::Client;
use coinstr_core::nostr_sdk::secp256k1::SecretKey;
use coinstr_core::nostr_sdk::{nips, EventBuilder, EventId, Filter, Keys, Tag};
use coinstr_core::policy::Policy;
use coinstr_core::proposal::SpendingProposal;
use coinstr_core::util::dir;
use coinstr_core::{Coinstr, Keychain, Result};

mod cli;
mod util;

use self::cli::{io, Cli, Command, SettingCommand};

const DEFAULT_RELAY: &str = "wss://relay.rip";

fn main() -> Result<()> {
    env_logger::init();

    let args = Cli::parse();
    let network: Network = args.network.into();
    let keychains: PathBuf = Path::new("./keychains").to_path_buf();

    let bitcoin_endpoint: &str = match network {
        Network::Bitcoin => "ssl://blockstream.info:700",
        Network::Testnet => "ssl://blockstream.info:993",
        _ => panic!("Endpoints nnot availabe for this network"),
    };

    // Create path
    std::fs::create_dir_all(keychains.as_path())?;

    match args.command {
        Command::Generate { name, word_count } => {
            let path: PathBuf = dir::get_keychain_file(keychains, name)?;
            let coinstr = Coinstr::generate(
                path,
                io::get_password_with_confirmation,
                word_count.into(),
                || {
                    if io::ask("Do you want to use a passphrase?")? {
                        Ok(Some(io::get_input("Passphrase")?))
                    } else {
                        Ok(None)
                    }
                },
                network,
            )?;
            let keychain: Keychain = coinstr.keychain();

            println!("\n!!! WRITE DOWN YOUT SEED PHRASE !!!");
            println!("\n################################################################\n");
            println!("{}", keychain.seed.mnemonic());
            println!("\n################################################################\n");

            Ok(())
        }
        Command::Restore { name } => {
            let path = dir::get_keychain_file(keychains, name)?;
            Coinstr::restore(
                path,
                io::get_password_with_confirmation,
                || Ok(Mnemonic::from_str(&io::get_input("Mnemonic")?)?),
                || {
                    if io::ask("Do you want to use a passphrase?")? {
                        Ok(Some(io::get_input("Passphrase")?))
                    } else {
                        Ok(None)
                    }
                },
                network,
            )?;
            Ok(())
        }
        Command::List => {
            let names = dir::get_keychains_list(keychains)?;
            for (index, name) in names.iter().enumerate() {
                println!("{}. {name}", index + 1);
            }
            Ok(())
        }
        Command::Inspect { name } => {
            let path = dir::get_keychain_file(keychains, name)?;
            let coinstr = Coinstr::open(path, io::get_password, network)?;
            let keychain = coinstr.keychain();
            util::print_secrets(keychain, network)
        }
        Command::SavePolicy {
            name,
            policy_name,
            policy_description,
            policy_descriptor,
        } => {
            let path = dir::get_keychain_file(keychains, name)?;
            let coinstr = Coinstr::open(path, io::get_password, network)?;
            let client = coinstr.nostr_client(vec![DEFAULT_RELAY.to_string()])?;
            let keys = coinstr.keychain().nostr_keys()?;

            let extracted_pubkeys = coinstr_core::util::extract_public_keys(&policy_descriptor)?;

            // Generate a shared secret key and encrypt the policy with it
            let global_key = Keys::generate();
            let policy =
                Policy::from_desc_or_policy(policy_name, policy_description, policy_descriptor)?;
            let content = nips::nip04::encrypt(
                &global_key.secret_key()?,
                &global_key.public_key(),
                policy.as_json(),
            )?;
            let tags: Vec<Tag> = extracted_pubkeys
                .iter()
                .map(|p| Tag::PubKey(*p, None))
                .collect();
            let policy_event = EventBuilder::new(POLICY_KIND, content, &tags).to_event(&keys)?;
            let policy_id = client.send_event(policy_event)?;

            // Publish the global shared key
            for pubkey in extracted_pubkeys.into_iter() {
                let encrypted_global_key = nips::nip04::encrypt(
                    &keys.secret_key()?,
                    &pubkey,
                    global_key.secret_key()?.display_secret().to_string(),
                )?;
                let event = EventBuilder::new(
                    SHARED_GLOBAL_KEY_KIND,
                    encrypted_global_key,
                    &[Tag::Event(policy_id, None, None), Tag::PubKey(pubkey, None)],
                )
                .to_event(&keys)?;
                let event_id = client.send_event(event)?;
                println!("Published global shared key for {pubkey} at event {event_id}");
            }

            println!("Policy saved: {policy_id}");
            Ok(())
        }
        Command::Spend {
            name,
            policy_id,
            memo,
            to_address,
            amount,
        } => {
            let path = dir::get_keychain_file(keychains, name)?;
            let coinstr = Coinstr::open(path, io::get_password, network)?;
            let client = coinstr.nostr_client(vec![DEFAULT_RELAY.to_string()])?;
            let keys = client.keys();

            // Get policy
            let timeout = Some(Duration::from_secs(300));
            let (policy, global_keys) = get_policy_by_id(&client, policy_id, timeout)?;

            // Sync balance
            let blockchain = ElectrumBlockchain::from(ElectrumClient::new(bitcoin_endpoint)?);
            let wallet = coinstr.wallet(policy.descriptor.to_string())?;
            wallet.sync(&blockchain, SyncOptions::default())?;

            // Compose PSBT
            let (psbt, _details) = {
                let mut builder = wallet.build_tx();
                builder
                    .add_recipient(to_address.script_pubkey(), amount)
                    .enable_rbf();
                builder.finish()?
            };

            // Create spending proposal
            let proposal = SpendingProposal::new(memo, to_address, amount, psbt);
            let extracted_pubkeys =
                coinstr_core::util::extract_public_keys(policy.descriptor.to_string())?;
            let mut tags: Vec<Tag> = extracted_pubkeys
                .iter()
                .map(|p| Tag::PubKey(*p, None))
                .collect();
            tags.push(Tag::Event(policy_id, None, None));
            let content = nips::nip04::encrypt(
                &global_keys.secret_key()?,
                &global_keys.public_key(),
                proposal.as_json(),
            )?;
            let event =
                EventBuilder::new(SPENDING_PROPOSAL_KIND, content, &tags).to_event(&keys)?;
            let proposal_id = client.send_event(event)?;
            println!("Spending proposal {proposal_id} sent");

            Ok(())
        }

        Command::Get { command } => match command {
            GetCommand::Contacts { name } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(vec![DEFAULT_RELAY.to_string()])?;
                let timeout = Some(Duration::from_secs(60));
                let contacts = client.get_contact_list_metadata(timeout)?;
                util::print_contacts(contacts);
                Ok(())
            }
            GetCommand::Policies { name } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(vec![DEFAULT_RELAY.to_string()])?;

                let keys = client.keys();
                let timeout = Some(Duration::from_secs(300));

                // Get policies
                let filter = Filter::new().pubkey(keys.public_key()).kind(POLICY_KIND);
                let policies_events = client.get_events_of(vec![filter], timeout)?;

                // Get global shared keys
                let filter = Filter::new()
                    .pubkey(keys.public_key())
                    .kind(SHARED_GLOBAL_KEY_KIND);
                let global_shared_key_events = client.get_events_of(vec![filter], timeout)?;

                // Index global keys by policy id
                let mut global_keys: HashMap<EventId, Keys> = HashMap::new();
                for event in global_shared_key_events.into_iter() {
                    for tag in event.tags {
                        if let Tag::Event(event_id, ..) = tag {
                            let content = nips::nip04::decrypt(
                                &keys.secret_key()?,
                                &event.pubkey,
                                &event.content,
                            )?;
                            let sk = SecretKey::from_str(&content)?;
                            let keys = Keys::new(sk);
                            global_keys.insert(event_id, keys);
                        }
                    }
                }

                println!();

                for event in policies_events.into_iter() {
                    let global_key = global_keys.get(&event.id).expect("Global key not found");
                    let content = nips::nip04::decrypt(
                        &global_key.secret_key()?,
                        &global_key.public_key(),
                        &event.content,
                    )?;

                    let policy = Policy::from_json(&content)?;
                    println!("- Policy id: {}", &event.id);
                    println!("- Name: {}", &policy.name);
                    println!("- Description: {}", &policy.description);
                    println!("- Descriptor: {}", policy.descriptor);
                    println!();
                }

                Ok(())
            }
            GetCommand::Policy { name, policy_id } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(vec![DEFAULT_RELAY.to_string()])?;

                // Get policy
                let timeout = Some(Duration::from_secs(300));
                let (policy, _global_keys) = get_policy_by_id(&client, policy_id, timeout)?;

                // TODO: improve printed output
                println!();
                println!("- Policy id: {}", policy_id);
                println!("- Name: {}", policy.name);
                println!("- Description: {}", policy.description);
                println!("- Descriptor: {}", policy.descriptor);
                println!();

                Ok(())
            }
            GetCommand::Proposals { name } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(vec![DEFAULT_RELAY.to_string()])?;

                let keys = client.keys();
                let timeout = Some(Duration::from_secs(300));

                // Get proposals
                let filter = Filter::new()
                    .pubkey(keys.public_key())
                    .kind(SPENDING_PROPOSAL_KIND);
                let proposals_events = client.get_events_of(vec![filter], timeout)?;

                // Get global shared keys
                let filter = Filter::new()
                    .pubkey(keys.public_key())
                    .kind(SHARED_GLOBAL_KEY_KIND);
                let global_shared_key_events = client.get_events_of(vec![filter], timeout)?;

                // Index global keys by policy id
                let mut global_keys: HashMap<EventId, Keys> = HashMap::new();
                for event in global_shared_key_events.into_iter() {
                    for tag in event.tags {
                        if let Tag::Event(event_id, ..) = tag {
                            let content = nips::nip04::decrypt(
                                &keys.secret_key()?,
                                &event.pubkey,
                                &event.content,
                            )?;
                            let sk = SecretKey::from_str(&content)?;
                            let keys = Keys::new(sk);
                            global_keys.insert(event_id, keys);
                        }
                    }
                }

                println!();

                for event in proposals_events.into_iter() {
                    let global_key: &Keys = {
                        let mut key = None;
                        for tag in event.tags {
                            if let Tag::Event(event_id, ..) = tag {
                                key =
                                    Some(global_keys.get(&event_id).expect("Global key not found"));
                            }
                        }
                        key
                    }
                    .unwrap();

                    let content = nips::nip04::decrypt(
                        &global_key.secret_key()?,
                        &global_key.public_key(),
                        &event.content,
                    )?;

                    let proposal = SpendingProposal::from_json(&content)?;
                    println!();
                    println!("- Proposal id: {}", &event.id);
                    println!("- Memo: {}", &proposal.memo);
                    println!("- To address: {}", &proposal.to_address);
                    println!("- Amount: {}", &proposal.amount);
                    println!();
                }

                Ok(())
            }
            GetCommand::Proposal {
                name: _,
                proposal_id: _,
            } => {
                todo!()
                /* let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(vec![DEFAULT_RELAY.to_string()])?;

                let keys = coinstr.keychain().nostr_keys()?;
                let timeout = Some(Duration::from_secs(300));
                let filter = Filter::new().id(proposal_id).kind(SPENDING_PROPOSAL_KIND);
                let events = client.get_events_of(vec![filter], timeout)?;
                let event = events.first().expect("Proposal not found");
                let content =
                    nips::nip04::decrypt(&keys.secret_key()?, &keys.public_key(), &event.content)?;

                // TODO: improve printed output

                let proposal = SpendingProposal::from_json(content)?;
                println!();
                println!("- Proposal id: {}", &event.id);
                println!("- Memo: {}", &proposal.memo);
                println!("- To address: {}", &proposal.to_address);
                println!("- Amount: {}", &proposal.amount);
                println!();

                Ok(()) */
            }
        },
        Command::Setting { command } => match command {
            SettingCommand::Rename { name, new_name } => {
                let path = dir::get_keychain_file(&keychains, name)?;
                let mut coinstr = Coinstr::open(path, io::get_password, network)?;
                let new_path = dir::get_keychain_file(keychains, new_name)?;
                Ok(coinstr.rename(new_path)?)
            }
            SettingCommand::ChangePassword { name } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let mut coinstr = Coinstr::open(path, io::get_password, network)?;
                Ok(coinstr.change_password(io::get_password_with_confirmation)?)
            }
        },
    }
}

fn get_policy_by_id(
    client: &Client,
    policy_id: EventId,
    timeout: Option<Duration>,
) -> Result<(Policy, Keys)> {
    let keys = client.keys();

    // Get policy event
    let filter = Filter::new()
        .id(policy_id)
        .author(keys.public_key())
        .kind(POLICY_KIND);
    let events = client.get_events_of(vec![filter], timeout)?;
    let policy_event = events.first().expect("Policy not found");

    // Get global shared key
    let filter = Filter::new()
        .pubkey(keys.public_key())
        .event(policy_id)
        .kind(SHARED_GLOBAL_KEY_KIND);
    let events = client.get_events_of(vec![filter], timeout)?;
    let global_shared_key_event = events.first().expect("Shared key not found");
    let content = nips::nip04::decrypt(
        &keys.secret_key()?,
        &global_shared_key_event.pubkey,
        &global_shared_key_event.content,
    )?;
    let sk = SecretKey::from_str(&content)?;
    let global_keys = Keys::new(sk);

    // Decrypt and deserialize the policy
    let content = nips::nip04::decrypt(
        &global_keys.secret_key()?,
        &global_keys.public_key(),
        &policy_event.content,
    )?;
    Ok((Policy::from_json(content)?, global_keys))
}
