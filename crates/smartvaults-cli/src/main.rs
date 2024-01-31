// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

#![forbid(unsafe_code)]

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use smartvaults_sdk::config::Config;
use smartvaults_sdk::core::bips::bip39::Mnemonic;
use smartvaults_sdk::core::bitcoin::{Address, Amount, Network};
use smartvaults_sdk::core::types::Priority;
use smartvaults_sdk::core::{ColdcardGenericJson, Destination, FeeRate, Keychain, Result};
use smartvaults_sdk::nostr::{EventId, Metadata};
use smartvaults_sdk::protocol::v1::{Label, SignerOffering};
use smartvaults_sdk::protocol::v2::{CompletedProposal, ProposalStatus, Signer};
use smartvaults_sdk::types::GetVault;
use smartvaults_sdk::{logger, SmartVaults};

mod cli;
mod util;

use crate::cli::batch::BatchCommand;
use crate::cli::{
    io, AddCommand, AddSignerCommand, Cli, CliCommand, Command, ConfigCommand, ConnectCommand,
    DeleteCommand, GetCommand, KeyAgentCommand, ProofCommand, SetCommand, SettingCommand,
    SignerCommand, VaultCommand,
};

fn base_path() -> Result<PathBuf> {
    let home_path = dirs::home_dir().expect("Imposible to get the HOME dir");
    let old_path = home_path.join(".coinstr");
    let path = home_path.join(".smartvaults");
    if old_path.exists() && !path.exists() {
        std::fs::rename(old_path, &path).unwrap();
    }
    std::fs::create_dir_all(path.as_path())?;
    Ok(path)
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{e}");
    }
}

async fn run() -> Result<()> {
    let args = Cli::parse();
    let network: Network = args.network.into();
    let base_path: PathBuf = base_path()?;

    logger::init(base_path.clone(), network, false)?;

    match args.command {
        CliCommand::Generate {
            name,
            word_count,
            passphrase,
        } => {
            let password_from_env: Option<String> = io::get_password_from_env();
            let confirm_password_from_env: Option<String> = password_from_env.clone();

            let password = if let Some(password) = password_from_env {
                password
            } else {
                io::get_password()?
            };

            let client = SmartVaults::generate(
                base_path,
                name,
                || Ok(password.clone()),
                || {
                    if let Some(password) = confirm_password_from_env {
                        Ok(password)
                    } else {
                        io::get_confirmation_password()
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
            )
            .await?;
            let keychain: Keychain = client.keychain(password)?;

            println!("\n!!! WRITE DOWN YOUR MNEMONIC !!!");
            println!("\n################################################################\n");
            println!("{}", keychain.seed.mnemonic());
            println!("\n################################################################\n");

            Ok(())
        }
        CliCommand::Restore { name } => {
            SmartVaults::restore(
                base_path,
                name,
                io::get_password,
                io::get_confirmation_password,
                || Ok(Mnemonic::from_str(&io::get_input("Mnemonic")?)?),
                || {
                    if io::ask("Do you want to use a passphrase?")? {
                        Ok(Some(io::get_input("Passphrase")?))
                    } else {
                        Ok(None)
                    }
                },
                network,
            )
            .await?;
            Ok(())
        }
        CliCommand::Open { name } => {
            let password: String = io::get_password()?;
            let client = SmartVaults::open(base_path, name, password, network).await?;

            let rl = &mut DefaultEditor::new()?;

            loop {
                let readline = rl.readline("smartvaults> ");
                match readline {
                    Ok(line) => {
                        let _ = rl.add_history_entry(line.as_str());
                        let mut vec: Vec<String> = cli::parser::split(&line)?;
                        vec.insert(0, String::new());
                        match Command::try_parse_from(vec) {
                            Ok(command) => {
                                if let Err(e) = handle_command(command, &client).await {
                                    eprintln!("Error: {e}");
                                }
                            }
                            Err(e) => {
                                eprintln!("{e}");
                            }
                        }
                        continue;
                    }
                    Err(ReadlineError::Interrupted) => {
                        // Ctrl-C
                        continue;
                    }
                    Err(ReadlineError::Eof) => break,
                    Err(e) => {
                        eprintln!("Error: {e}");
                        break;
                    }
                }
            }

            client.shutdown().await?;

            Ok(())
        }
        CliCommand::Batch { name, path } => {
            let password: String = io::get_password()?;
            let client = SmartVaults::open(base_path, name, password, network).await?;

            let file = File::open(path)?;
            let reader = BufReader::new(file);

            for line in reader.lines().map_while(Result::ok) {
                let mut vec: Vec<String> = cli::parser::split(&line)?;
                vec.insert(0, String::new());
                println!("{line}");
                match BatchCommand::try_parse_from(vec) {
                    Ok(command) => {
                        if let Err(e) = handle_command(command.into(), &client).await {
                            eprintln!("Error: {e}");
                        }
                    }
                    Err(e) => {
                        eprintln!("{e}");
                    }
                }
            }

            println!("Shutting down...");
            client.shutdown().await?;

            Ok(())
        }
        CliCommand::List => {
            let names: Vec<String> = SmartVaults::list_keychains(base_path, network)?;
            for (index, name) in names.iter().enumerate() {
                println!("{}. {name}", index + 1);
            }
            Ok(())
        }
        CliCommand::Config { command } => match command {
            ConfigCommand::View => {
                let config = Config::try_from_file(base_path, network)?;
                println!("{}", config.as_pretty_json().await?);
                Ok(())
            }
            ConfigCommand::Set {
                electrum_server,
                proxy,
                block_explorer,
            } => {
                let config = Config::try_from_file(base_path, network)?;

                if let Some(endpoint) = electrum_server {
                    config.set_electrum_endpoint(Some(endpoint)).await?;
                }

                if let Some(proxy) = proxy {
                    config.set_proxy(Some(proxy)).await;
                }

                if let Some(block_explorer) = block_explorer {
                    config.set_block_explorer(Some(block_explorer)).await;
                }

                config.save().await?;

                Ok(())
            }
            ConfigCommand::Unset {
                electrum_server,
                proxy,
                block_explorer,
            } => {
                let config = Config::try_from_file(base_path, network)?;

                if electrum_server {
                    config.set_electrum_endpoint::<String>(None).await?;
                }

                if proxy {
                    config.set_proxy(None).await;
                }

                if block_explorer {
                    config.set_block_explorer(None).await;
                }

                config.save().await?;

                Ok(())
            }
        },
    }
}

async fn handle_command(command: Command, client: &SmartVaults) -> Result<()> {
    match command {
        Command::Inspect => {
            let password: String = io::get_password()?;
            let keychain = client.keychain(password)?;
            util::print_secrets(keychain, client.network())
        }
        Command::Spend {
            vault_id,
            address,
            amount,
            description,
            target_blocks,
        } => {
            let address: Address = address.require_network(client.network())?;
            let proposal = client
                .spend(
                    &vault_id,
                    Destination::single(address, Amount::from_sat(amount)),
                    description,
                    FeeRate::Priority(Priority::Custom(target_blocks)),
                    None,
                    None,
                    false,
                )
                .await?;
            println!("Spending proposal {} sent", proposal.compute_id());
            Ok(())
        }
        Command::SpendAll {
            vault_id,
            address,
            description,
            target_blocks,
        } => {
            let address: Address = address.require_network(client.network())?;
            let proposal = client
                .spend(
                    &vault_id,
                    Destination::drain(address),
                    description,
                    FeeRate::Priority(Priority::Custom(target_blocks)),
                    None,
                    None,
                    false,
                )
                .await?;
            println!("Spending proposal {} sent", proposal.compute_id());
            Ok(())
        }
        Command::Approve { proposal_id } => {
            let password: String = io::get_password()?;
            client.approve(&proposal_id, password).await?;
            println!("Proposal {proposal_id} approved");
            Ok(())
        }
        Command::Finalize { proposal_id } => {
            let proposal = client.finalize(&proposal_id).await?;

            if let ProposalStatus::Completed(status) = proposal.status() {
                match status {
                    CompletedProposal::Spending { tx, .. } => {
                        let txid = tx.txid();

                        println!("Transaction {txid} broadcasted");

                        match client.network() {
                            Network::Bitcoin => {
                                println!("\nExplorer: https://blockstream.info/tx/{txid} \n")
                            }
                            Network::Testnet => {
                                println!(
                                    "\nExplorer: https://blockstream.info/testnet/tx/{txid} \n"
                                )
                            }
                            _ => (),
                        };
                    }
                    CompletedProposal::KeyAgentPayment { tx, .. } => {
                        let txid = tx.txid();

                        println!("Key agent payment broadcasted: {txid}");

                        match client.network() {
                            Network::Bitcoin => {
                                println!("\nExplorer: https://blockstream.info/tx/{txid} \n")
                            }
                            Network::Testnet => {
                                println!(
                                    "\nExplorer: https://blockstream.info/testnet/tx/{txid} \n"
                                )
                            }
                            _ => (),
                        };
                    }
                    CompletedProposal::ProofOfReserve { .. } => {
                        println!("Proof of Reserve finalized")
                    }
                }
            } else {
                eprintln!("Proposal not finalized");
            }

            Ok(())
        }
        Command::Vault { command } => match command {
            VaultCommand::Add {
                name,
                description,
                descriptor,
            } => {
                let vault_id = client.save_vault(name, description, descriptor).await?;
                println!("Vault saved: {vault_id}");
                Ok(())
            }
            VaultCommand::Invite {
                vault_id,
                public_key,
                message,
            } => {
                client
                    .invite_to_vault(&vault_id, public_key, message.unwrap_or_default())
                    .await?;
                println!("Invite sent!");
                Ok(())
            }
            VaultCommand::Invites => {
                let invites = client.vault_invites().await?;
                util::print_vaults_invites(invites);
                Ok(())
            }
            VaultCommand::AcceptInvite { vault_id } => {
                client.accept_vault_invite(&vault_id).await?;
                println!("Vault invite accepted!");
                Ok(())
            }
            VaultCommand::Metadata {
                vault_id,
                name,
                description,
            } => {
                client
                    .edit_vault_metadata(&vault_id, name, description)
                    .await?;
                Ok(())
            }
            VaultCommand::Get { vault_id, export } => {
                // Get vault
                let vault: GetVault = client.get_vault_by_id(&vault_id).await?;

                // Print result
                if export {
                    println!("\n{}\n", vault.as_descriptor());
                    Ok(())
                } else {
                    let item = vault.satisfiable_item()?.clone();
                    let address = client.get_last_unused_address(&vault_id).await?;
                    let txs = client.get_txs(&vault_id).await.unwrap_or_default();
                    let utxos = client.get_utxos(&vault_id).await.unwrap_or_default();
                    util::print_vault(vault, item, address, txs, utxos);
                    Ok(())
                }
            }
            VaultCommand::List => {
                let vaults = client.vaults().await?;
                util::print_vaults(vaults);
                Ok(())
            }
            VaultCommand::Delete { vault_id } => Ok(client.delete_vault_by_id(&vault_id).await?),
            VaultCommand::Members { vault_id } => {
                let members = client.get_members_of_vault(&vault_id).await?;
                util::print_profiles(members);
                Ok(())
            }
        },
        Command::Signer { command } => match command {
            SignerCommand::Add { command } => match command {
                AddSignerCommand::Default => {
                    client.save_smartvaults_signer().await?;
                    Ok(())
                }
                AddSignerCommand::Coldcard { name, path } => {
                    let coldcard = ColdcardGenericJson::from_file(path)?;
                    let mut signer = Signer::from_coldcard(&coldcard, client.network())?;
                    signer.change_name(name);
                    let signer_id = client.save_signer(signer).await?;
                    println!("Saved coldcard signer: {signer_id}");
                    Ok(())
                }
            },
            SignerCommand::Metadata {
                signer_id,
                name,
                description,
            } => {
                client
                    .edit_signer_metadata(&signer_id, name, description)
                    .await?;
                Ok(())
            }
            SignerCommand::Get { signer_id } => {
                let signer = client.get_signer_by_id(&signer_id).await?;
                util::print_signer(signer);
                Ok(())
            }
            SignerCommand::List => {
                let signers = client.signers().await;
                util::print_signers(signers);
                Ok(())
            }
            SignerCommand::ListShared => {
                let signers = client.shared_signers().await?;
                util::print_shared_signers(signers);
                Ok(())
            }
            SignerCommand::Delete { signer_id } => {
                Ok(client.delete_signer_by_id(&signer_id).await?)
            }
            SignerCommand::Share {
                signer_id,
                public_key,
                message,
            } => {
                client
                    .share_signer(&signer_id, public_key, message.unwrap_or_default())
                    .await?;
                println!(
                    "Shared signerigner invite sent to {}",
                    smartvaults_sdk::util::cut_public_key(public_key)
                );
                Ok(())
            }
            SignerCommand::Invites => {
                let invites = client.shared_signer_invites().await?;
                util::print_shared_signer_invites(invites);
                Ok(())
            }
            SignerCommand::AcceptInvite { shared_signer_id } => {
                client
                    .accept_shared_signer_invite(&shared_signer_id)
                    .await?;
                println!("Signer invite accepted!");
                Ok(())
            }
        },
        Command::Proof { command } => match command {
            ProofCommand::New { .. } => {
                // let (proposal_id, ..) = client.new_proof_proposal(policy_id, message).await?;
                // println!("Proof of Reserve proposal {proposal_id} sent");
                Ok(())
            }
            ProofCommand::Verify { .. } => {
                // let spendable = client.verify_proof_by_id(proposal_id).await?;
                // println!(
                // "Valid Proof - Spendable amount: {} sat",
                // format::number(spendable)
                // );
                Ok(())
            }
        },
        Command::Connect { command } => match command {
            ConnectCommand::New { uri } => {
                client.new_nostr_connect_session(uri).await?;
                Ok(())
            }
            ConnectCommand::Disconnect { app_public_key } => {
                client
                    .disconnect_nostr_connect_session(app_public_key)
                    .await?;
                Ok(())
            }
            ConnectCommand::Sessions => {
                let sessions = client.get_nostr_connect_sessions().await?;
                util::print_sessions(sessions);
                Ok(())
            }
            ConnectCommand::Requests { approved } => {
                let requests = client.get_nostr_connect_requests(approved).await?;
                util::print_requests(requests)?;
                Ok(())
            }
            ConnectCommand::Approve { request_id } => {
                client.approve_nostr_connect_request(request_id).await?;
                Ok(())
            }
            ConnectCommand::Autoapprove {
                app_public_key,
                seconds,
            } => {
                client
                    .auto_approve_nostr_connect_requests(
                        app_public_key,
                        Duration::from_secs(seconds),
                    )
                    .await;
                Ok(())
            }
            ConnectCommand::Authorizations => {
                let authorizations = client.get_nostr_connect_pre_authorizations().await;
                util::print_authorizations(authorizations);
                Ok(())
            }
            ConnectCommand::Revoke { app_public_key } => {
                client
                    .revoke_nostr_connect_auto_approve(app_public_key)
                    .await;
                Ok(())
            }
        },
        Command::KeyAgent { command } => match command {
            KeyAgentCommand::Signer {
                signer_id,
                temperature,
                device_type,
                response_time,
                cost_per_signature,
                yearly_cost_basis_points,
                yearly_cost,
            } => {
                let signer: Signer = client.get_signer_by_id(&signer_id).await?;

                let offering: SignerOffering = SignerOffering {
                    temperature,
                    device_type,
                    response_time,
                    cost_per_signature,
                    yearly_cost_basis_points,
                    yearly_cost,
                    network: client.network(),
                };

                let event_id: EventId = client.signer_offering(&signer, offering).await?;
                println!("Signer offering published: {event_id}");

                Ok(())
            }
            KeyAgentCommand::ListSigners => {
                let offerings = client.my_signer_offerings().await?;
                util::print_key_agents_signer_offersing(offerings);
                Ok(())
            }
        },
        Command::Add { command } => match command {
            AddCommand::Relay { url, proxy } => {
                client.add_relay(url, proxy).await?;
                Ok(())
            }
            AddCommand::Contact { public_key } => {
                client.add_contact(public_key).await?;
                Ok(())
            }
        },
        Command::Get { command } => match command {
            GetCommand::Contacts => {
                let contacts = client.get_contacts().await?;
                util::print_profiles(contacts);
                Ok(())
            }
            GetCommand::Proposals { all, completed } => {
                if all {
                    // let proposals = client.get_completed_proposals().await?;
                    // util::print_completed_proposals(proposals);
                } else if completed {
                } else {
                    let proposals = client.proposals().await?;
                    util::print_proposals(proposals);
                }
                Ok(())
            }
            GetCommand::Proposal { proposal_id } => {
                let proposal = client.get_proposal_by_id(&proposal_id).await?;
                util::print_proposal(proposal);
                Ok(())
            }
            GetCommand::Relays => {
                let relays = client.relays().await;
                util::print_relays(relays).await;
                Ok(())
            }
            GetCommand::Addresses { vault_id } => {
                let addresses = client.get_addresses(&vault_id).await?;
                let balances = client.get_addresses_balances(&vault_id).await?;
                util::print_addresses(addresses, balances);
                Ok(())
            }
        },
        Command::Set { command } => match command {
            SetCommand::Metadata {
                name,
                display_name,
                nip05,
                empty,
            } => {
                let mut metadata = Metadata::new();
                metadata.name = name;
                metadata.display_name = display_name;
                metadata.nip05 = nip05;

                if metadata != Metadata::default() || empty {
                    client.set_metadata(&metadata).await?;
                } else {
                    println!("No metadata passed with args! If you want to set empty metadata, use --empty flag");
                }

                Ok(())
            }
            SetCommand::Label {
                vault_id,
                data,
                text,
            } => {
                let label = Label::new(data, text);
                let event_id = client.save_label(&vault_id, label).await?;
                println!("Label saved at event {event_id}");
                Ok(())
            }
        },
        Command::Delete { command } => match command {
            DeleteCommand::Relay { url } => {
                client.remove_relay(url).await?;
                Ok(())
            }
            DeleteCommand::Proposal { proposal_id } => {
                Ok(client.delete_proposal_by_id(&proposal_id).await?)
            }
            // DeleteCommand::Approval { approval_id } => {
            // client.revoke_approval(approval_id).await?;
            // Ok(())
            // }
            // DeleteCommand::SharedSigner { shared_signer_id } => {
            // Ok(client.revoke_shared_signer(shared_signer_id).await?)
            // }
            DeleteCommand::Cache => Ok(client.clear_cache().await?),
        },
        Command::Rebroadcast => {
            client.rebroadcast_all_events().await?;
            Ok(())
        }
        Command::Setting { command } => match command {
            SettingCommand::Rename { new_name } => Ok(client.rename(new_name)?),
            SettingCommand::ChangePassword => Ok(client.change_password(
                io::get_password,
                io::get_new_password,
                io::get_confirmation_password,
            )?),
        },
        Command::Exit => std::process::exit(0x01),
    }
}
