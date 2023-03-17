use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use clap::Parser;
use cli::GetCommand;
use coinstr_core::bip39::Mnemonic;
use coinstr_core::bitcoin::Network;
use coinstr_core::constants::{POLICY_KIND, SPENDING_PROPOSAL_KIND};
use coinstr_core::nostr_sdk::{nips, EventBuilder, Filter};
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
            let policy =
                Policy::from_desc_or_policy(policy_name, policy_description, policy_descriptor)?;
            let content =
                nips::nip04::encrypt(&keys.secret_key()?, &keys.public_key(), policy.as_json())?;
            let event = EventBuilder::new(POLICY_KIND, content, &[]).to_event(&keys)?;
            let event_id = client.send_event(event)?;
            println!("Policy saved: {event_id}");
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

                let keys = coinstr.keychain().nostr_keys()?;
                let timeout = Some(Duration::from_secs(300));
                let filter = Filter::new().author(keys.public_key()).kind(POLICY_KIND);
                let events = client.get_events_of(vec![filter], timeout)?;

                println!();

                for event in events.into_iter() {
                    let content = nips::nip04::decrypt(
                        &keys.secret_key()?,
                        &keys.public_key(),
                        &event.content,
                    )?;

                    let policy = Policy::from_json(&content)?;
                    println!("- Policy id: {}", &event.id);
                    println!("- Name: {}", &policy.name);
                    println!("- Description: {}", &policy.description);
                    println!("- Descriptor: {}", policy.descriptor);
                    println!();

                    //println!("{}", policy);
                }

                Ok(())
            }
            GetCommand::Policy { name, policy_id } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(vec![DEFAULT_RELAY.to_string()])?;

                let keys = coinstr.keychain().nostr_keys()?;
                let timeout = Some(Duration::from_secs(300));
                let filter = Filter::new()
                    .id(policy_id)
                    .author(keys.public_key())
                    .kind(POLICY_KIND);
                let events = client.get_events_of(vec![filter], timeout)?;
                let event = events.first().expect("Policy not found");
                let content =
                    nips::nip04::decrypt(&keys.secret_key()?, &keys.public_key(), &event.content)?;

                println!();

                let policy = Policy::from_json(content)?;
                println!("- Policy id: {}", &event.id);
                println!("- Name: {}", &policy.name);
                println!("- Description: {}", &policy.description);
                println!("- Descriptor: {}", policy.descriptor);
                println!();

                //println!("{}", policy);

                Ok(())
            }
            GetCommand::Proposals { name } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(vec![DEFAULT_RELAY.to_string()])?;

                let keys = coinstr.keychain().nostr_keys()?;
                let timeout = Some(Duration::from_secs(300));
                let filter = Filter::new()
                    .author(keys.public_key())
                    .kind(SPENDING_PROPOSAL_KIND);
                let events = client.get_events_of(vec![filter], timeout)?;

                for event in events.into_iter() {
                    let content = nips::nip04::decrypt(
                        &keys.secret_key()?,
                        &keys.public_key(),
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
            GetCommand::Proposal { name, proposal_id } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(vec![DEFAULT_RELAY.to_string()])?;

                let keys = coinstr.keychain().nostr_keys()?;
                let timeout = Some(Duration::from_secs(300));
                let filter = Filter::new().id(proposal_id).kind(SPENDING_PROPOSAL_KIND);
                let events = client.get_events_of(vec![filter], timeout)?;
                let event = events.first().expect("Proposal not found");
                let content =
                    nips::nip04::decrypt(&keys.secret_key()?, &keys.public_key(), &event.content)?;

                let proposal = SpendingProposal::from_json(content)?;
                println!();
                println!("- Proposal id: {}", &event.id);
                println!("- Memo: {}", &proposal.memo);
                println!("- To address: {}", &proposal.to_address);
                println!("- Amount: {}", &proposal.amount);
                println!();

                Ok(())
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
