// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use clap::Parser;
use coinstr_sdk::core::bdk::blockchain::{Blockchain, ElectrumBlockchain};
use coinstr_sdk::core::bdk::electrum_client::Client as ElectrumClient;
use coinstr_sdk::core::bips::bip39::Mnemonic;
use coinstr_sdk::core::bitcoin::Network;
use coinstr_sdk::core::{Amount, CompletedProposal, Keychain, Result};
use coinstr_sdk::util::format;
use coinstr_sdk::Coinstr;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

mod cli;
mod util;

use self::cli::{
    io, Cli, CliCommand, Command, DeleteCommand, GetCommand, ProofCommand, SettingCommand,
};

const DEFAULT_RELAY: &str = "wss://relay.rip";
const TIMEOUT: Option<Duration> = Some(Duration::from_secs(300));

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{e}");
    }
}

async fn run() -> Result<()> {
    let args = Cli::parse();
    let network: Network = args.network.into();
    let relays: Vec<String> = vec![args.relay];
    let base_path: PathBuf = coinstr_common::base_path()?;

    let endpoint: &str = match network {
        Network::Bitcoin => "ssl://blockstream.info:700",
        Network::Testnet => "ssl://blockstream.info:993",
        _ => panic!("Endpoints not availabe for this network"),
    };

    match args.command {
        CliCommand::Generate {
            name,
            word_count,
            password,
            passphrase,
        } => {
            let coinstr = Coinstr::generate(
                base_path,
                name,
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
        CliCommand::Restore { name } => {
            Coinstr::restore(
                base_path,
                name,
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
        CliCommand::Open { name } => {
            let coinstr = Coinstr::open(base_path, name, io::get_password, network)?;
            coinstr.add_relays_and_connect(relays).await?;
            coinstr.set_electrum_endpoint(endpoint);
            coinstr.sync();

            let rl = &mut DefaultEditor::new()?;

            loop {
                let readline = rl.readline("coinstr> ");
                match readline {
                    Ok(line) => {
                        let _ = rl.add_history_entry(line.as_str());
                        let mut vec: Vec<&str> = line.as_str().split_whitespace().collect();
                        vec.insert(0, "");
                        match Command::try_parse_from(vec) {
                            Ok(command) => {
                                if let Err(e) = handle_command(command, &coinstr).await {
                                    eprintln!("Error: {e}");
                                }
                            }
                            Err(e) => {
                                eprintln!("{e}");
                            }
                        }
                        continue;
                    }
                    Err(ReadlineError::Interrupted) => break,
                    Err(ReadlineError::Eof) => break,
                    Err(e) => {
                        eprintln!("Error: {e}");
                        break;
                    }
                }
            }

            coinstr.shutdown().await?;

            Ok(())
        }
        CliCommand::List => {
            let names: Vec<String> = Coinstr::list_keychains(base_path, network)?;
            for (index, name) in names.iter().enumerate() {
                println!("{}. {name}", index + 1);
            }
            Ok(())
        }
        CliCommand::Setting { command } => match command {
            SettingCommand::Rename { name, new_name } => {
                let mut coinstr = Coinstr::open(base_path, name, io::get_password, network)?;
                Ok(coinstr.rename(new_name)?)
            }
            SettingCommand::ChangePassword { name } => {
                let mut coinstr = Coinstr::open(base_path, name, io::get_password, network)?;
                Ok(coinstr.change_password(io::get_password_with_confirmation)?)
            }
        },
    }
}

async fn handle_command(command: Command, coinstr: &Coinstr) -> Result<()> {
    match command {
        Command::Inspect => {
            let keychain = coinstr.keychain();
            util::print_secrets(keychain, coinstr.network())
        }
        Command::SavePolicy {
            name,
            description,
            descriptor,
            custom_pubkeys,
        } => {
            let policy_id = coinstr
                .save_policy(name, description, descriptor, custom_pubkeys)
                .await?;
            println!("Policy saved: {policy_id}");
            Ok(())
        }
        Command::Spend {
            policy_id,
            to_address,
            amount,
            description,
            target_blocks,
        } => {
            let endpoint: String = coinstr.electrum_endpoint()?;
            let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&endpoint)?);
            let fee_rate = blockchain.estimate_fee(target_blocks)?;

            let (proposal_id, _proposal) = coinstr
                .spend(
                    policy_id,
                    to_address,
                    Amount::Custom(amount),
                    description,
                    fee_rate,
                    TIMEOUT,
                )
                .await?;
            println!("Spending proposal {proposal_id} sent");
            Ok(())
        }
        Command::SpendAll {
            policy_id,
            to_address,
            description,
            target_blocks,
        } => {
            let endpoint: String = coinstr.electrum_endpoint()?;
            let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&endpoint)?);
            let fee_rate = blockchain.estimate_fee(target_blocks)?;

            let (proposal_id, _proposal) = coinstr
                .spend(
                    policy_id,
                    to_address,
                    Amount::Max,
                    description,
                    fee_rate,
                    TIMEOUT,
                )
                .await?;
            println!("Spending proposal {proposal_id} sent");
            Ok(())
        }
        Command::Approve { proposal_id } => {
            let (event_id, _) = coinstr.approve(proposal_id, TIMEOUT).await?;
            println!("Proposal {proposal_id} approved: {event_id}");
            Ok(())
        }
        Command::Finalize { proposal_id } => {
            let completed_proposal: CompletedProposal =
                coinstr.finalize(proposal_id, TIMEOUT).await?;

            match completed_proposal {
                CompletedProposal::Spending { tx, .. } => {
                    let txid = tx.txid();

                    println!("Transaction {txid} broadcasted");

                    match coinstr.network() {
                        Network::Bitcoin => {
                            println!("\nExplorer: https://blockstream.info/tx/{txid} \n")
                        }
                        Network::Testnet => {
                            println!("\nExplorer: https://blockstream.info/testnet/tx/{txid} \n")
                        }
                        _ => (),
                    };
                }
                CompletedProposal::ProofOfReserve { .. } => println!("Proof of Reserve finalized"),
            };

            Ok(())
        }
        Command::Proof { command } => match command {
            ProofCommand::New { policy_id, message } => {
                let (proposal_id, ..) = coinstr
                    .new_proof_proposal(policy_id, message, TIMEOUT)
                    .await?;
                println!("Proof of Reserve proposal {proposal_id} sent");
                Ok(())
            }
            ProofCommand::Verify { proposal_id } => {
                let spendable = coinstr.verify_proof_by_id(proposal_id).await?;
                println!(
                    "Valid Proof - Spendable amount: {} sat",
                    format::number(spendable)
                );
                Ok(())
            }
        },
        Command::Get { command } => match command {
            GetCommand::Contacts => {
                let contacts = coinstr.get_contacts(TIMEOUT).await?;
                util::print_contacts(contacts);
                Ok(())
            }
            GetCommand::Policies => {
                let policies = coinstr.get_policies()?;
                util::print_policies(policies);
                Ok(())
            }
            GetCommand::Policy { policy_id, export } => {
                // Get policy
                let policy = coinstr.get_policy_by_id(policy_id)?;

                // Print result
                if export {
                    println!("\n{}\n", policy.descriptor);
                    Ok(())
                } else {
                    let endpoint = coinstr.electrum_endpoint()?;
                    let wallet = coinstr.wallet(policy_id, policy.descriptor.to_string())?;
                    util::print_policy(policy, policy_id, wallet, endpoint)
                }
            }
            GetCommand::Proposals { completed } => {
                if completed {
                    let proposals = coinstr.get_completed_proposals()?;
                    util::print_completed_proposals(proposals);
                } else {
                    let proposals = coinstr.get_proposals()?;
                    util::print_proposals(proposals);
                }
                Ok(())
            }
            GetCommand::Proposal { proposal_id } => {
                let (policy_id, proposal) = coinstr.get_proposal_by_id(proposal_id)?;
                util::print_proposal(proposal_id, proposal, policy_id);
                Ok(())
            }
        },
        Command::Delete { command } => match command {
            DeleteCommand::Policy { policy_id } => {
                Ok(coinstr.delete_policy_by_id(policy_id, TIMEOUT).await?)
            }
            DeleteCommand::Proposal {
                proposal_id,
                completed,
            } => {
                if completed {
                    Ok(coinstr
                        .delete_completed_proposal_by_id(proposal_id, TIMEOUT)
                        .await?)
                } else {
                    Ok(coinstr.delete_proposal_by_id(proposal_id, TIMEOUT).await?)
                }
            }
        },
        Command::Exit => std::process::exit(0x01),
    }
}
