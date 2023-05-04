// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use clap::Parser;
use coinstr_core::bip39::Mnemonic;
use coinstr_core::bitcoin::Network;
use coinstr_core::util::dir::{get_keychain_file, get_keychains_list};
use coinstr_core::util::format;
use coinstr_core::{Amount, Coinstr, FeeRate, Keychain, Result};

mod cli;
mod util;

use self::cli::{io, Cli, Command, DeleteCommand, GetCommand, ProofCommand, SettingCommand};

const DEFAULT_RELAY: &str = "wss://relay.rip";
const TIMEOUT: Option<Duration> = Some(Duration::from_secs(300));

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{e}");
    }
}

async fn run() -> Result<()> {
    env_logger::init();

    let args = Cli::parse();
    let network: Network = args.network.into();
    let relays: Vec<String> = vec![args.relay];
    let keychains: PathBuf = coinstr_common::keychains()?;

    let endpoint: &str = match network {
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
            coinstr.add_relays_and_connect(relays).await?;
            let (policy_id, _policy) = coinstr
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
            description,
            target_blocks,
        } => {
            let path = get_keychain_file(keychains, name)?;
            let coinstr = Coinstr::open(path, io::get_password, network)?;
            coinstr.add_relays_and_connect(relays).await?;
            let (proposal_id, _proposal) = coinstr
                .spend(
                    policy_id,
                    to_address,
                    Amount::Custom(amount),
                    description,
                    FeeRate::Custom(target_blocks),
                    TIMEOUT,
                )
                .await?;
            println!("Spending proposal {proposal_id} sent");
            Ok(())
        }
        Command::SpendAll {
            name,
            policy_id,
            to_address,
            description,
            target_blocks,
        } => {
            let path = get_keychain_file(keychains, name)?;
            let coinstr = Coinstr::open(path, io::get_password, network)?;
            coinstr.add_relays_and_connect(relays).await?;
            coinstr.set_electrum_endpoint(endpoint).await;
            let (proposal_id, _proposal) = coinstr
                .spend(
                    policy_id,
                    to_address,
                    Amount::Max,
                    description,
                    FeeRate::Custom(target_blocks),
                    TIMEOUT,
                )
                .await?;
            println!("Spending proposal {proposal_id} sent");
            Ok(())
        }
        Command::Approve { name, proposal_id } => {
            let path = get_keychain_file(keychains, name)?;
            let coinstr = Coinstr::open(path, io::get_password, network)?;
            coinstr.add_relays_and_connect(relays).await?;
            let (event, _) = coinstr.approve(proposal_id, TIMEOUT).await?;
            println!("Proposal {proposal_id} approved: {}", event.id);
            Ok(())
        }
        Command::Broadcast { name, proposal_id } => {
            let path = get_keychain_file(keychains, name)?;
            let coinstr = Coinstr::open(path, io::get_password, network)?;
            coinstr.add_relays_and_connect(relays).await?;
            coinstr.set_electrum_endpoint(endpoint).await;
            let txid = coinstr.broadcast(proposal_id, TIMEOUT).await?;

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
        Command::Proof { command } => match command {
            ProofCommand::New {
                name,
                policy_id,
                message,
            } => {
                let path = get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                coinstr.add_relays_and_connect(relays).await?;
                coinstr.set_electrum_endpoint(endpoint).await;
                let (proposal_id, ..) = coinstr
                    .new_proof_proposal(policy_id, message, TIMEOUT)
                    .await?;
                println!("Proof of Reserve proposal {proposal_id} sent");
                Ok(())
            }
            ProofCommand::Finalize { name, proposal_id } => {
                let path = get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                coinstr.add_relays_and_connect(relays).await?;
                coinstr.finalize_proof(proposal_id, TIMEOUT).await?;
                println!("Proof of Reserve finalized");
                Ok(())
            }
            ProofCommand::Verify { name, proposal_id } => {
                let path = get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                coinstr.add_relays_and_connect(relays).await?;
                coinstr.set_electrum_endpoint(endpoint).await;
                let spendable = coinstr.verify_proof(proposal_id, TIMEOUT).await?;
                println!(
                    "Valid Proof - Spendable amount: {} sat",
                    format::number(spendable)
                );
                Ok(())
            }
        },
        Command::Get { command } => match command {
            GetCommand::Contacts { name } => {
                let path = get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                coinstr.add_relays_and_connect(relays).await?;
                let contacts = coinstr.get_contacts(TIMEOUT).await?;
                util::print_contacts(contacts);
                Ok(())
            }
            GetCommand::Policies { name } => {
                let path = get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                coinstr.add_relays_and_connect(relays).await?;
                let policies = coinstr.get_policies(TIMEOUT).await?;
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
                coinstr.add_relays_and_connect(relays).await?;

                // Get policy
                let (policy, _shared_keys) = coinstr.get_policy_by_id(policy_id, TIMEOUT).await?;

                // Open wallet
                let wallet = coinstr.wallet(policy.descriptor.to_string())?;

                // Print result
                if export {
                    println!("\n{}\n", policy.descriptor);
                    Ok(())
                } else {
                    util::print_policy(policy, policy_id, wallet, endpoint)
                }
            }
            GetCommand::Proposals { name, completed } => {
                let path = get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                coinstr.add_relays_and_connect(relays).await?;
                if completed {
                    let proposals = coinstr.get_completed_proposals(TIMEOUT).await?;
                    util::print_completed_proposals(proposals);
                } else {
                    let proposals = coinstr.get_proposals(TIMEOUT).await?;
                    util::print_proposals(proposals);
                }
                Ok(())
            }
            GetCommand::Proposal { name, proposal_id } => {
                let path = get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                coinstr.add_relays_and_connect(relays).await?;
                let (proposal, policy_id, _shared_keys) =
                    coinstr.get_proposal_by_id(proposal_id, TIMEOUT).await?;
                util::print_proposal(proposal_id, proposal, policy_id);
                Ok(())
            }
        },
        Command::Delete { command } => match command {
            DeleteCommand::Policy { name, policy_id } => {
                let path = get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                coinstr.add_relays_and_connect(relays).await?;
                Ok(coinstr.delete_policy_by_id(policy_id, TIMEOUT).await?)
            }
            DeleteCommand::Proposal { name, proposal_id } => {
                let path = get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                coinstr.add_relays_and_connect(relays).await?;
                Ok(coinstr.delete_proposal_by_id(proposal_id, TIMEOUT).await?)
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
