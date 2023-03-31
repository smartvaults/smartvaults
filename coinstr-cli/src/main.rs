// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use bdk::blockchain::ElectrumBlockchain;
use bdk::electrum_client::Client as ElectrumClient;
use clap::Parser;
use cli::{DeleteCommand, GetCommand};
use coinstr_core::bip39::Mnemonic;
use coinstr_core::bitcoin::Network;
use coinstr_core::util::dir::{get_keychain_file, get_keychains_list};
use coinstr_core::{Coinstr, Keychain, Result};

mod cli;
mod dir;
mod util;

use self::cli::{io, Cli, Command, SettingCommand};

const DEFAULT_RELAY: &str = "wss://relay.rip";
const TIMEOUT: Option<Duration> = Some(Duration::from_secs(300));

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Cli::parse();
    let network: Network = args.network.into();
    let relays: Vec<String> = vec![args.relay];
    let keychains: PathBuf = dir::keychains()?;

    let bitcoin_endpoint: &str = match network {
        Network::Bitcoin => "ssl://blockstream.info:700",
        Network::Testnet => "ssl://blockstream.info:993",
        _ => panic!("Endpoints not availabe for this network"),
    };

    // Create path
    std::fs::create_dir_all(keychains.as_path())?;

    match args.command {
        Command::Generate {
            name,
            word_count,
            password,
            passphrase,
        } => {
            let path: PathBuf = get_keychain_file(keychains, name)?;
            let coinstr = Coinstr::generate(
                path,
                || {
                    if let Some(password) = password {
                        Ok(password)
                    } else {
                        io::get_password_with_confirmation()
                    }
                },
                word_count.into(),
                || {
                    if let Some(passphrase) = passphrase {
                        Ok(Some(passphrase))
                    } else if io::ask("Do you want to use a passphrase?")? {
                        Ok(Some(io::get_input("Passphrase")?))
                    } else {
                        Ok(None)
                    }
                },
                network,
            )?;
            let keychain: Keychain = coinstr.keychain();

            println!("\n!!! WRITE DOWN YOUR MNEMONIC !!!");
            println!("\n################################################################\n");
            println!("{}", keychain.seed.mnemonic());
            println!("\n################################################################\n");

            Ok(())
        }
        Command::Restore { name } => {
            let path = get_keychain_file(keychains, name)?;
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
            let names = get_keychains_list(keychains)?;
            for (index, name) in names.iter().enumerate() {
                println!("{}. {name}", index + 1);
            }
            Ok(())
        }
        Command::Inspect { name } => {
            let path = get_keychain_file(keychains, name)?;
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
            let path = get_keychain_file(keychains, name)?;
            let coinstr = Coinstr::open(path, io::get_password, network)?;
            let client = coinstr.client(relays).await?;
            let policy_id = client
                .save_policy(policy_name, policy_description, policy_descriptor)
                .await?;
            println!("Policy saved: {policy_id}");
            Ok(())
        }
        Command::Spend {
            name,
            policy_id,
            to_address,
            amount,
            memo,
        } => {
            let path = get_keychain_file(keychains, name)?;
            let coinstr = Coinstr::open(path, io::get_password, network)?;
            let client = coinstr.client(relays).await?;
            let blockchain = ElectrumBlockchain::from(ElectrumClient::new(bitcoin_endpoint)?);
            let proposal_id = client
                .spend(policy_id, to_address, amount, memo, blockchain, TIMEOUT)
                .await?;
            println!("Spending proposal {proposal_id} sent");
            Ok(())
        }
        Command::Approve { name, proposal_id } => {
            let path = get_keychain_file(keychains, name)?;
            let coinstr = Coinstr::open(path, io::get_password, network)?;
            let client = coinstr.client(relays).await?;
            let event_id = client.approve(proposal_id, TIMEOUT).await?;
            println!("Spending proposal {proposal_id} approved: {event_id}");
            Ok(())
        }
        Command::Broadcast { name, proposal_id } => {
            let path = get_keychain_file(keychains, name)?;
            let coinstr = Coinstr::open(path, io::get_password, network)?;
            let client = coinstr.client(relays).await?;
            let blockchain = ElectrumBlockchain::from(ElectrumClient::new(bitcoin_endpoint)?);
            let txid = client.broadcast(proposal_id, blockchain, TIMEOUT).await?;
            println!("Transaction {txid} broadcasted");

            match network {
                Network::Bitcoin => {
                    println!("\nExplorer: https://blockstream.info/tx/{txid} \n")
                }
                Network::Testnet => {
                    println!("\nExplorer: https://blockstream.info/testnet/tx/{txid} \n")
                }
                _ => (),
            };

            Ok(())
        }
        Command::Get { command } => match command {
            GetCommand::Contacts { name } => {
                let path = get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.client(relays).await?;
                let contacts = client.get_contacts(TIMEOUT).await?;
                util::print_contacts(contacts);
                Ok(())
            }
            GetCommand::Policies { name } => {
                let path = get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.client(relays).await?;
                let policies = client.get_policies(TIMEOUT).await?;
                util::print_policies(policies);
                Ok(())
            }
            GetCommand::Policy {
                name,
                policy_id,
                export,
            } => {
                let path = get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.client(relays).await?;

                // Get policy
                let (policy, _shared_keys) = client.get_policy_by_id(policy_id, TIMEOUT).await?;

                // Open wallet
                let wallet = client.wallet(policy.descriptor.to_string())?;

                // Print result
                if export {
                    println!("\n{}\n", policy.descriptor);
                    Ok(())
                } else {
                    util::print_policy(policy, policy_id, wallet, bitcoin_endpoint)
                }
            }
            GetCommand::Proposals { name } => {
                let path = get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.client(relays).await?;
                let proposals = client.get_proposals(TIMEOUT).await?;
                util::print_proposals(proposals);
                Ok(())
            }
            GetCommand::Proposal { name, proposal_id } => {
                let path = get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.client(relays).await?;
                let (proposal, policy_id, _shared_keys) =
                    client.get_proposal_by_id(proposal_id, TIMEOUT).await?;
                util::print_proposal(proposal_id, proposal, policy_id);
                Ok(())
            }
        },
        Command::Delete { command } => match command {
            DeleteCommand::Policy { name, policy_id } => {
                let path = get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.client(relays).await?;
                Ok(client.delete_policy_by_id(policy_id, TIMEOUT).await?)
            }
            DeleteCommand::Proposal { name, proposal_id } => {
                let path = get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.client(relays).await?;
                Ok(client.delete_proposal_by_id(proposal_id, TIMEOUT).await?)
            }
        },
        Command::Setting { command } => match command {
            SettingCommand::Rename { name, new_name } => {
                let path = get_keychain_file(&keychains, name)?;
                let mut coinstr = Coinstr::open(path, io::get_password, network)?;
                let new_path = get_keychain_file(keychains, new_name)?;
                Ok(coinstr.rename(new_path)?)
            }
            SettingCommand::ChangePassword { name } => {
                let path = get_keychain_file(keychains, name)?;
                let mut coinstr = Coinstr::open(path, io::get_password, network)?;
                Ok(coinstr.change_password(io::get_password_with_confirmation)?)
            }
        },
    }
}
