use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use cli::{DeleteCommand, GetCommand};
use coinstr_core::bdk::blockchain::ElectrumBlockchain;
use coinstr_core::bdk::electrum_client::Client as ElectrumClient;
use coinstr_core::bdk::signer::{SignerContext, SignerOrdering, SignerWrapper};
use coinstr_core::bdk::{KeychainKind, SignOptions, SyncOptions};
use coinstr_core::bip39::Mnemonic;
use coinstr_core::bitcoin::{Network, PrivateKey};
use coinstr_core::constants::{APPROVED_PROPOSAL_KIND, SPENDING_PROPOSAL_KIND};
use coinstr_core::nostr_sdk::{nips, EventBuilder, Tag};
use coinstr_core::proposal::SpendingProposal;
use coinstr_core::util::dir;
use coinstr_core::{Coinstr, CoinstrNostr, Keychain, Result};

mod cli;
mod util;

use self::cli::{io, Cli, Command, SettingCommand};

const DEFAULT_RELAY: &str = "wss://relay.rip";
const TIMEOUT: Option<Duration> = Some(Duration::from_secs(300));

fn main() -> Result<()> {
    env_logger::init();

    let args = Cli::parse();
    let network: Network = args.network.into();
    let relays: Vec<String> = vec![args.relay];
    let keychains: PathBuf = Path::new("./keychains").to_path_buf();

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
            let path: PathBuf = dir::get_keychain_file(keychains, name)?;
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
            let client = coinstr.nostr_client(relays)?;
            let policy_id =
                client.save_policy(policy_name, policy_description, policy_descriptor)?;
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
            let path = dir::get_keychain_file(keychains, name)?;
            let coinstr = Coinstr::open(path, io::get_password, network)?;
            let client = coinstr.nostr_client(relays)?;

            // Get policy
            let (policy, shared_keys) = client.get_policy_by_id(policy_id, TIMEOUT)?;

            // Sync balance
            let blockchain = ElectrumBlockchain::from(ElectrumClient::new(bitcoin_endpoint)?);
            let wallet = coinstr.wallet(policy.descriptor.to_string())?;
            wallet.sync(&blockchain, SyncOptions::default())?;

            // Get policies and specify which ones to use
            let wallet_policy = wallet.policies(KeychainKind::External)?.unwrap();
            let mut path = BTreeMap::new();
            path.insert(wallet_policy.id, vec![1]);

            // Build the transaction
            let mut builder = wallet.build_tx();
            builder
                .add_recipient(to_address.script_pubkey(), amount)
                .policy_path(path, KeychainKind::External);

            // Build the PSBT
            let (psbt, _details) = builder.finish()?;

            // Create spending proposal
            let proposal = SpendingProposal::new(to_address, amount, &memo, psbt);
            let extracted_pubkeys =
                coinstr_core::util::extract_public_keys(policy.descriptor.to_string())?;
            let mut tags: Vec<Tag> = extracted_pubkeys
                .iter()
                .map(|p| Tag::PubKey(*p, None))
                .collect();
            tags.push(Tag::Event(policy_id, None, None));
            let content = nips::nip04::encrypt(
                &shared_keys.secret_key()?,
                &shared_keys.public_key(),
                proposal.as_json(),
            )?;
            // Publish proposal with `shared_key` so every owner can delete it
            let event =
                EventBuilder::new(SPENDING_PROPOSAL_KIND, content, &tags).to_event(&shared_keys)?;
            let proposal_id = client.send_event(event)?;
            println!("Spending proposal {proposal_id} sent");

            // Send DM msg
            let sender = client.keys().public_key();
            let mut msg = String::from("New spending proposal:\n");
            msg.push_str(&format!("- Amount: {amount}\n"));
            msg.push_str(&format!("- Memo: {memo}"));
            for pubkey in extracted_pubkeys.into_iter() {
                if sender != pubkey {
                    client.send_direct_msg(pubkey, &msg)?;
                }
            }

            Ok(())
        }
        Command::Approve { name, proposal_id } => {
            let path = dir::get_keychain_file(keychains, name)?;
            let coinstr = Coinstr::open(path, io::get_password, network)?;
            let client = coinstr.nostr_client(relays)?;

            let keys = client.keys();

            // Get proposal
            let (proposal, policy_id, shared_keys) =
                client.get_proposal_by_id(proposal_id, TIMEOUT)?;

            // Get policy id
            let (policy, _shared_keys) = client.get_policy_by_id(policy_id, TIMEOUT)?;

            // Create a BDK wallet
            let mut wallet = coinstr.wallet(policy.descriptor.to_string())?;

            // Add the BDK signer
            let private_key = PrivateKey::new(keys.secret_key()?, network);
            let signer = SignerWrapper::new(
                private_key,
                SignerContext::Tap {
                    is_internal_key: false,
                },
            );

            wallet.add_signer(KeychainKind::External, SignerOrdering(0), Arc::new(signer));

            // Sign the transaction
            let mut psbt = proposal.psbt.clone();
            let _finalized = wallet.sign(&mut psbt, SignOptions::default())?;
            if psbt != proposal.psbt {
                let content = nips::nip04::encrypt(
                    &shared_keys.secret_key()?,
                    &shared_keys.public_key(),
                    psbt.to_string(),
                )?;
                // Publish approved proposal with `shared_key` so after the broadcast
                // of the transaction it can be deleted
                let event = EventBuilder::new(
                    APPROVED_PROPOSAL_KIND,
                    content,
                    &[
                        Tag::Event(proposal_id, None, None),
                        Tag::Event(policy_id, None, None),
                    ],
                )
                .to_event(&shared_keys)?;
                let event_id = client.send_event(event)?;
                println!("Spending proposal {proposal_id} approved: {event_id}");
            } else {
                println!("PSBT not signed")
            }

            Ok(())
        }
        Command::Broadcast { name, proposal_id } => {
            let path = dir::get_keychain_file(keychains, name)?;
            let coinstr = Coinstr::open(path, io::get_password, network)?;
            let client = coinstr.nostr_client(relays)?;
            let blockchain = ElectrumBlockchain::from(ElectrumClient::new(bitcoin_endpoint)?);
            let txid = client.broadcast(proposal_id, blockchain, TIMEOUT)?;
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
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(relays)?;
                let contacts = client.get_contact_list_metadata(TIMEOUT)?;
                util::print_contacts(contacts);
                Ok(())
            }
            GetCommand::Policies { name } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(relays)?;
                let policies = client.get_policies(TIMEOUT)?;
                util::print_policies(policies);
                Ok(())
            }
            GetCommand::Policy {
                name,
                policy_id,
                export,
            } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(relays)?;

                // Get policy
                let (policy, _shared_keys) = client.get_policy_by_id(policy_id, TIMEOUT)?;

                // Open wallet
                let wallet = coinstr.wallet(policy.descriptor.to_string())?;

                // Print result
                if export {
                    println!("\n{}\n", policy.descriptor);
                    Ok(())
                } else {
                    util::print_policy(policy, policy_id, wallet, bitcoin_endpoint)
                }
            }
            GetCommand::Proposals { name } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(relays)?;
                let proposals = client.get_proposals(TIMEOUT)?;
                util::print_proposals(proposals);
                Ok(())
            }
            GetCommand::Proposal { name, proposal_id } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(relays)?;
                let (proposal, policy_id, _shared_keys) =
                    client.get_proposal_by_id(proposal_id, TIMEOUT)?;
                util::print_proposal(proposal_id, proposal, policy_id);
                Ok(())
            }
        },
        Command::Delete { command } => match command {
            DeleteCommand::Policy { name, policy_id } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(relays)?;
                client.delete_policy_by_id(policy_id, TIMEOUT)
            }
            DeleteCommand::Proposal { name, proposal_id } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(relays)?;
                client.delete_proposal_by_id(proposal_id, TIMEOUT)
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
