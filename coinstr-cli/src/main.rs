use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use cli::{DeleteCommand, GetCommand};
use coinstr_core::bdk::blockchain::{Blockchain, ElectrumBlockchain};
use coinstr_core::bdk::electrum_client::Client as ElectrumClient;
use coinstr_core::bdk::miniscript::psbt::PsbtExt;
use coinstr_core::bdk::signer::{SignerContext, SignerOrdering, SignerWrapper};
use coinstr_core::bdk::{KeychainKind, SignOptions, SyncOptions};
use coinstr_core::bip39::Mnemonic;
use coinstr_core::bitcoin::psbt::PartiallySignedTransaction;
use coinstr_core::bitcoin::{Network, PrivateKey};
use coinstr_core::constants::{
    APPROVED_PROPOSAL_KIND, POLICY_KIND, SHARED_KEY_KIND, SPENDING_PROPOSAL_KIND,
};
use coinstr_core::nostr_sdk::blocking::Client;
use coinstr_core::nostr_sdk::secp256k1::SecretKey;
use coinstr_core::nostr_sdk::{nips, Event, EventBuilder, EventId, Filter, Keys, Tag, SECP256K1};
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

            println!("\n!!! WRITE DOWN YOUT MNEMONIC !!!");
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
            let keys = client.keys();

            let extracted_pubkeys = coinstr_core::util::extract_public_keys(&policy_descriptor)?;

            // Generate a shared key
            let shared_key = Keys::generate();
            let policy =
                Policy::from_desc_or_policy(policy_name, policy_description, policy_descriptor)?;
            let content = nips::nip04::encrypt(
                &shared_key.secret_key()?,
                &shared_key.public_key(),
                policy.as_json(),
            )?;
            let tags: Vec<Tag> = extracted_pubkeys
                .iter()
                .map(|p| Tag::PubKey(*p, None))
                .collect();
            // Publish policy with `shared_key` so every owner can delete it
            let policy_event =
                EventBuilder::new(POLICY_KIND, content, &tags).to_event(&shared_key)?;
            let policy_id = client.send_event(policy_event)?;

            // Publish the shared key
            for pubkey in extracted_pubkeys.into_iter() {
                let encrypted_shared_key = nips::nip04::encrypt(
                    &keys.secret_key()?,
                    &pubkey,
                    shared_key.secret_key()?.display_secret().to_string(),
                )?;
                let event = EventBuilder::new(
                    SHARED_KEY_KIND,
                    encrypted_shared_key,
                    &[Tag::Event(policy_id, None, None), Tag::PubKey(pubkey, None)],
                )
                .to_event(&keys)?;
                let event_id = client.send_event(event)?;
                println!("Published shared key for {pubkey} at event {event_id}");
            }

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
            let timeout = Some(Duration::from_secs(300));
            let (policy, shared_keys) = get_policy_by_id(&client, policy_id, timeout)?;

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
            let proposal = SpendingProposal::new(memo, to_address, amount, psbt);
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

            Ok(())
        }
        Command::Approve { name, proposal_id } => {
            let path = dir::get_keychain_file(keychains, name)?;
            let coinstr = Coinstr::open(path, io::get_password, network)?;
            let client = coinstr.nostr_client(relays)?;

            let keys = client.keys();
            let timeout = Some(Duration::from_secs(300));

            // Get proposal
            let (proposal, policy_id, shared_keys) =
                get_proposal_by_id(&client, proposal_id, timeout)?;

            // Get policy id
            let (policy, _shared_keys) = get_policy_by_id(&client, policy_id, timeout)?;

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

            // Get PSBTs
            let timeout = Some(Duration::from_secs(300));
            let (mut base_psbt, psbts) =
                get_signed_psbts_by_proposal_id(&client, proposal_id, timeout)?;

            // Combine PSBTs
            for psbt in psbts {
                base_psbt.combine(psbt)?;
            }

            // Finalize and broadcast the transaction
            match base_psbt.finalize_mut(SECP256K1) {
                Ok(_) => {
                    let finalized_tx = base_psbt.extract_tx();
                    let blockchain =
                        ElectrumBlockchain::from(ElectrumClient::new(bitcoin_endpoint)?);
                    blockchain.broadcast(&finalized_tx)?;
                    println!("Transaction {} broadcasted", finalized_tx.txid());

                    // Delete the proposal
                    delete_proposal_by_id(&client, proposal_id, timeout)?;
                }
                Err(e) => eprintln!("PSBT not finalized: {e:?}"),
            }

            Ok(())
        }
        Command::Get { command } => match command {
            GetCommand::Contacts { name } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(relays)?;
                let timeout = Some(Duration::from_secs(60));
                let contacts = client.get_contact_list_metadata(timeout)?;
                util::print_contacts(contacts);
                Ok(())
            }
            GetCommand::Policies { name } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(relays)?;

                let keys = client.keys();
                let timeout = Some(Duration::from_secs(300));

                // Get policies
                let filter = Filter::new().pubkey(keys.public_key()).kind(POLICY_KIND);
                let policies_events = client.get_events_of(vec![filter], timeout)?;

                // Get shared keys
                let shared_keys: HashMap<EventId, Keys> = get_shared_keys(&client)?;

                let mut policies: Vec<(EventId, Policy)> = Vec::new();

                for event in policies_events.into_iter() {
                    let global_key = shared_keys.get(&event.id).expect("Global key not found");
                    let content = nips::nip04::decrypt(
                        &global_key.secret_key()?,
                        &global_key.public_key(),
                        &event.content,
                    )?;
                    policies.push((event.id, Policy::from_json(&content)?));
                }

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
                let timeout = Some(Duration::from_secs(300));
                let (policy, _shared_keys) = get_policy_by_id(&client, policy_id, timeout)?;

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

                let keys = client.keys();
                let timeout = Some(Duration::from_secs(300));

                // Get proposals
                let filter = Filter::new()
                    .pubkey(keys.public_key())
                    .kind(SPENDING_PROPOSAL_KIND);
                let proposals_events = client.get_events_of(vec![filter], timeout)?;

                // Get shared keys
                let shared_keys: HashMap<EventId, Keys> = get_shared_keys(&client)?;

                let mut proposals: Vec<(EventId, SpendingProposal, EventId)> = Vec::new();

                for event in proposals_events.into_iter() {
                    let policy_id = extract_first_event_id(&event).expect("Policy id not found");
                    let global_key: &Keys =
                        shared_keys.get(&policy_id).expect("Global key not found");

                    let content = nips::nip04::decrypt(
                        &global_key.secret_key()?,
                        &global_key.public_key(),
                        &event.content,
                    )?;

                    proposals.push((event.id, SpendingProposal::from_json(&content)?, policy_id));
                }

                util::print_proposals(proposals);

                Ok(())
            }
            GetCommand::Proposal { name, proposal_id } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(relays)?;

                let timeout = Some(Duration::from_secs(300));
                let (proposal, policy_id, _shared_keys) =
                    get_proposal_by_id(&client, proposal_id, timeout)?;

                // TODO: improve printed output

                println!();
                println!("- Proposal id: {proposal_id}");
                println!("- Policy id: {policy_id}");
                println!("- Memo: {}", proposal.memo);
                println!("- To address: {}", proposal.to_address);
                println!("- Amount: {}", proposal.amount);
                println!();

                Ok(())
            }
        },
        Command::Delete { command } => match command {
            DeleteCommand::Policy { name, policy_id } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(relays)?;
                let timeout = Some(Duration::from_secs(300));
                delete_policy_by_id(&client, policy_id, timeout)
            }
            DeleteCommand::Proposal { name, proposal_id } => {
                let path = dir::get_keychain_file(keychains, name)?;
                let coinstr = Coinstr::open(path, io::get_password, network)?;
                let client = coinstr.nostr_client(relays)?;
                let timeout = Some(Duration::from_secs(300));
                delete_proposal_by_id(&client, proposal_id, timeout)
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

fn get_shared_keys(client: &Client) -> Result<HashMap<EventId, Keys>> {
    let timeout = Some(Duration::from_secs(300));
    let keys = client.keys();

    let filter = Filter::new()
        .pubkey(keys.public_key())
        .kind(SHARED_KEY_KIND);
    let global_shared_key_events = client.get_events_of(vec![filter], timeout)?;

    // Index global keys by policy id
    let mut shared_keys: HashMap<EventId, Keys> = HashMap::new();
    for event in global_shared_key_events.into_iter() {
        for tag in event.tags {
            if let Tag::Event(event_id, ..) = tag {
                let content =
                    nips::nip04::decrypt(&keys.secret_key()?, &event.pubkey, &event.content)?;
                let sk = SecretKey::from_str(&content)?;
                let keys = Keys::new(sk);
                shared_keys.insert(event_id, keys);
            }
        }
    }
    Ok(shared_keys)
}

fn get_policy_by_id(
    client: &Client,
    policy_id: EventId,
    timeout: Option<Duration>,
) -> Result<(Policy, Keys)> {
    let keys = client.keys();

    // Get policy event
    let filter = Filter::new().id(policy_id).kind(POLICY_KIND);
    let events = client.get_events_of(vec![filter], timeout)?;
    let policy_event = events.first().expect("Policy not found");

    // Get global shared key
    let filter = Filter::new()
        .pubkey(keys.public_key())
        .event(policy_id)
        .kind(SHARED_KEY_KIND);
    let events = client.get_events_of(vec![filter], timeout)?;
    let global_shared_key_event = events.first().expect("Shared key not found");
    let content = nips::nip04::decrypt(
        &keys.secret_key()?,
        &global_shared_key_event.pubkey,
        &global_shared_key_event.content,
    )?;
    let sk = SecretKey::from_str(&content)?;
    let shared_keys = Keys::new(sk);

    // Decrypt and deserialize the policy
    let content = nips::nip04::decrypt(
        &shared_keys.secret_key()?,
        &shared_keys.public_key(),
        &policy_event.content,
    )?;
    Ok((Policy::from_json(content)?, shared_keys))
}

fn get_proposal_by_id(
    client: &Client,
    proposal_id: EventId,
    timeout: Option<Duration>,
) -> Result<(SpendingProposal, EventId, Keys)> {
    let keys = client.keys();

    // Get proposal event
    let filter = Filter::new().id(proposal_id).kind(SPENDING_PROPOSAL_KIND);
    let events = client.get_events_of(vec![filter], timeout)?;
    let proposal_event = events.first().expect("Spending proposal not found");
    let policy_id = extract_first_event_id(proposal_event).expect("Policy id not found");

    // Get global shared key
    let filter = Filter::new()
        .pubkey(keys.public_key())
        .event(policy_id)
        .kind(SHARED_KEY_KIND);
    let events = client.get_events_of(vec![filter], timeout)?;
    let global_shared_key_event = events.first().expect("Shared key not found");
    let content = nips::nip04::decrypt(
        &keys.secret_key()?,
        &global_shared_key_event.pubkey,
        &global_shared_key_event.content,
    )?;
    let sk = SecretKey::from_str(&content)?;
    let shared_keys = Keys::new(sk);

    // Decrypt and deserialize the spending proposal
    let content = nips::nip04::decrypt(
        &shared_keys.secret_key()?,
        &shared_keys.public_key(),
        &proposal_event.content,
    )?;
    Ok((
        SpendingProposal::from_json(content)?,
        policy_id,
        shared_keys,
    ))
}

fn get_signed_psbts_by_proposal_id(
    client: &Client,
    proposal_id: EventId,
    timeout: Option<Duration>,
) -> Result<(PartiallySignedTransaction, Vec<PartiallySignedTransaction>)> {
    // Get approved proposals
    let filter = Filter::new()
        .event(proposal_id)
        .kind(APPROVED_PROPOSAL_KIND);
    let proposals_events = client.get_events_of(vec![filter], timeout)?;
    let first_event = proposals_events
        .first()
        .expect("Approved proposals not found");
    let proposal_id = extract_first_event_id(first_event).expect("Proposal id not found");

    // Get global shared key
    let (proposal, _, shared_keys) = get_proposal_by_id(client, proposal_id, timeout)?;

    let mut psbts: Vec<PartiallySignedTransaction> = Vec::new();

    for event in proposals_events.into_iter() {
        let content = nips::nip04::decrypt(
            &shared_keys.secret_key()?,
            &shared_keys.public_key(),
            &event.content,
        )?;
        psbts.push(PartiallySignedTransaction::from_str(&content)?);
    }

    Ok((proposal.psbt, psbts))
}

fn extract_first_event_id(event: &Event) -> Option<EventId> {
    for tag in event.tags.iter() {
        if let Tag::Event(event_id, ..) = tag {
            return Some(*event_id);
        }
    }
    None
}

fn delete_policy_by_id(
    client: &Client,
    policy_id: EventId,
    timeout: Option<Duration>,
) -> Result<()> {
    let keys = client.keys();

    // Get global shared key
    let filter = Filter::new()
        .pubkey(keys.public_key())
        .event(policy_id)
        .kind(SHARED_KEY_KIND);
    let events = client.get_events_of(vec![filter], timeout)?;
    let global_shared_key_event = events.first().expect("Shared key not found");
    let content = nips::nip04::decrypt(
        &keys.secret_key()?,
        &global_shared_key_event.pubkey,
        &global_shared_key_event.content,
    )?;
    let sk = SecretKey::from_str(&content)?;
    let shared_keys = Keys::new(sk);

    // Get all events linked to the policy
    let filter = Filter::new().event(policy_id);
    let events = client.get_events_of(vec![filter], timeout)?;

    let mut ids: Vec<EventId> = events.iter().map(|e| e.id).collect();
    ids.push(policy_id);

    let event = EventBuilder::delete::<String>(ids, None).to_event(&shared_keys)?;
    client.send_event(event)?;

    Ok(())
}

fn delete_proposal_by_id(
    client: &Client,
    proposal_id: EventId,
    timeout: Option<Duration>,
) -> Result<()> {
    let keys = client.keys();

    // Get the proposal
    let filter = Filter::new().id(proposal_id);
    let events = client.get_events_of(vec![filter], timeout)?;
    let proposal_event = events.first().expect("Spending proposal not found");
    let policy_id = extract_first_event_id(proposal_event).expect("Policy id not found");

    // Get global shared key
    let filter = Filter::new()
        .pubkey(keys.public_key())
        .event(policy_id)
        .kind(SHARED_KEY_KIND);
    let events = client.get_events_of(vec![filter], timeout)?;
    let global_shared_key_event = events.first().expect("Shared key not found");
    let content = nips::nip04::decrypt(
        &keys.secret_key()?,
        &global_shared_key_event.pubkey,
        &global_shared_key_event.content,
    )?;
    let sk = SecretKey::from_str(&content)?;
    let shared_keys = Keys::new(sk);

    // Get all events linked to the proposal
    let filter = Filter::new().event(proposal_id);
    let events = client.get_events_of(vec![filter], timeout)?;

    let mut ids: Vec<EventId> = events.iter().map(|e| e.id).collect();
    ids.push(proposal_id);

    let event = EventBuilder::delete::<String>(ids, None).to_event(&shared_keys)?;
    client.send_event(event)?;

    Ok(())
}
