use nostr_rust::bech32::{ToBech32Kind, to_bech32};
use nostr_rust::events::{Event, EventPrepare};
use nostr_rust::keys::*;
use nostr_rust::nips::nip1::NIP1Error;
use nostr_rust::{nostr_client::Client, Identity,  req::ReqFilter,  Message, events::extract_events_ws};
use std::{env,
    str::FromStr,
    sync::{Arc, Mutex},
    thread,
};
use nostr_rust::utils::get_timestamp;
extern crate serde_json;
use bdk::template::Bip84;
use bdk::{miniscript, Wallet, KeychainKind};
mod users;
use bdk::bitcoin::Network;
use bdk::database::MemoryDatabase;
use bdk::keys::{DerivableKey, GeneratableKey, GeneratedKey, ExtendedKey, bip39::{Mnemonic, WordCount, Language}};

fn dump_keys(sk_str: String) {

    let sk = secret_key_from_str(&sk_str);

    let (secret_str_1, public_str_1) = get_str_keys_from_secret(&sk.unwrap());

    println!("Secret Key (HEX): {:?} ", secret_str_1); 
    println!("Public Key (HEX): {:?} ", public_str_1); 

    let bech32_pub = to_bech32(ToBech32Kind::PublicKey, &public_str_1);
    let bech32_prv = to_bech32(ToBech32Kind::SecretKey, &secret_str_1);

    println!("Public Key (bech32): {:?} ", bech32_pub.unwrap());
    println!("Secret Key (bech32): {:?} ", bech32_prv.unwrap());

    let network = Network::Testnet; // Or this can be Network::Bitcoin, Network::Signet or Network::Regtest

    // Generate fresh mnemonic
    let mnemonic: GeneratedKey<_, miniscript::Segwitv0> = Mnemonic::generate((WordCount::Words12, Language::English)).unwrap();

    // Convert mnemonic to string
    let mnemonic_words = mnemonic.to_string();
    println!("Mnemonic: {:?} ", &mnemonic_words);

    // Parse a mnemonic
    let mnemonic  = Mnemonic::parse(&mnemonic_words).unwrap();
    
    // Generate the extended key
    let xkey: ExtendedKey = mnemonic.into_extended_key().unwrap();
    // Get xprv from the extended key
    let xprv = xkey.into_xprv(network).unwrap();
    println!("Secret Key xprv: {:?} ", xprv);


}

fn _random_account() -> (String, String) {
    let (secret_key_1, _public_key_1) = get_random_secret_key();

    let (secret_str_1, public_str_1) = get_str_keys_from_secret(&secret_key_1);
    dump_keys(secret_str_1.clone());
    (secret_str_1, public_str_1)
}

fn publish_psbt(
    nostr_client: Arc<Mutex<Client>>,
    identity: &Identity,
    content: &str,
    tags: &[Vec<String>],
    difficulty_target: u16,
) -> Result<Event, NIP1Error> {
    let event = EventPrepare {
        pub_key: identity.public_key_str.clone(),
        created_at: get_timestamp(),
        kind: 21,
        tags: tags.to_vec(),
        content: content.to_string(),
    }
    .to_event(identity, difficulty_target);

    nostr_client.lock().unwrap().publish_event(&event)?;
    Ok(event)
}

// fn send_psbt(nostr_client: Arc<Mutex<Client>>, identity: &Identity, psbt: &String) -> Result<(), String> {
//     let message = psbt;

//     nostr_client
//        .lock()
//        .unwrap()
//        .publish_text_note(&identity, &message, &[], 0)
//        .unwrap();

//     Ok(())
// }


fn handle_message(message: &Message) -> Result<(), String> {
    let events = extract_events_ws(message);
    println!("{}", serde_json::to_string_pretty(&events).unwrap());

    Ok(())
}

fn subscribe(nostr_client: Arc<Mutex<Client>>) {
    
     // Run a new thread to handle messages
     let _subscription_id = nostr_client
     .lock()
     .unwrap()
     .subscribe(vec![ReqFilter {
         ids: None,
         authors: Some(vec![
            users::alice_keys().1, users::bob_keys().1, users::charlie_keys().1   //, elephant_keys().1
         ]),
         kinds: None,
         e: None,
         p: None,
         since: Some(1673908031),
         until: None,
         limit: Some(10),
     }])
     .unwrap();

     let nostr_clone = nostr_client.clone();
     let handle_thread = thread::spawn(move || {
        println!("Listening...");

        loop {
            let events = nostr_clone.lock().unwrap().next_data().unwrap();

            for (_relay_url, message) in events.iter() {
                handle_message(message).unwrap();
            }
        }
    });

     handle_thread.join().unwrap();
}


fn main()  {
    
    let args: Vec<String> = env::args().collect();
    dbg!(&args);

    let nostr_client = Arc::new(Mutex::new(
        Client::new(vec!["ws://127.0.0.1:8080"]).unwrap(),
    ));

    if &args.len() > &1 && args[1] == "subscribe".to_string() {
        subscribe(nostr_client);
    } else if args[1] == "post".to_string() {

        let mut prv_key = "".to_string();
        if args[2] == "bob" {
            prv_key = users::bob_keys().0;
        } else if args[2] == "alice" {
            prv_key = users::alice_keys().0;
        } else if args[2] == "charlie" {
            prv_key = users::charlie_keys().0;
        } 

        let message = &args[3];
        let poster_identity = Identity::from_str(&prv_key).unwrap();
        nostr_client
            .lock()
            .unwrap()
            .publish_text_note(&poster_identity, &message, &[], 0)
            .unwrap();

    } else if args[1] == "key".to_string() {
        dump_keys(args[2].to_string());
    } else if args[1] == "dump".to_string() {
        dump_keys(users::alice_keys().0);
        dump_keys(users::bob_keys().0);
        dump_keys(users::charlie_keys().0);
    } 
    else if args[1] == "psbt".to_string() {
        let poster_identity = Identity::from_str(&users::alice_keys().0).unwrap();
        publish_psbt(nostr_client, &poster_identity, "my psbt", &[], 0).ok();
    }
    else if args[1] == "random".to_string() {
        // users::random_account();
    }
 
}

// basic 2 of 3 multisig with Alice Bob and Charlie
/* thresh(2,pk(cPatMiTiN4gWsBQpKuHPY2d3Z41NWGu2xEvNumubhPADh7VHzqqV),
    pk(02476b018f75b1084e4b2bd652a747a37de9727183bcfe4113fe0b9390767e3543),
    pk(023254bcb92a82208ac8d864f3772c1576eb12dd97f1110d858cedb58251ba5043))
*/