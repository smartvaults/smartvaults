use nostr_rust::bech32::{ToBech32Kind, to_bech32};
use nostr_rust::keys::*;
use nostr_rust::{nostr_client::Client, Identity,  req::ReqFilter,  Message, events::extract_events_ws};
use std::{env,
    str::FromStr,
    sync::{Arc, Mutex},
    thread,
};
extern crate serde_json;

/*  Alice
Secret Key (HEX): "d15949d0e9f2f06080f2ed40a115bac037f3dce28cbdeda52c168b4f374b01ea" 
Public Key (HEX): "5c5ef19b5bd5c74a6826c27dca599d66623c2c4da423a7b8c42a1fc8f04b945e" 
Public Key (bech32): "npub1t300rx6m6hr556pxcf7u5kvave3rctzd5s360wxy9g0u3uztj30qwelvqe" 
Secret Key (bech32): "nsec169v5n58f7tcxpq8ja4q2z9d6cqml8h8z3j77mffvz6957d6tq84qjfym6p" 
*/
fn alice_keys() -> (String, String) {
    ("d15949d0e9f2f06080f2ed40a115bac037f3dce28cbdeda52c168b4f374b01ea".to_string(),
    "5c5ef19b5bd5c74a6826c27dca599d66623c2c4da423a7b8c42a1fc8f04b945e".to_string())
}

/*  Bob 
Secret Key (HEX): "f21e8748241b4fe5ca129a265866dba80387758c3cdfeb539648672e3c5b2621" 
Public Key (HEX): "875788b34464a298081521442eae87fdb0463ead9dd26c5a11e280550fb14c34" 
Public Key (bech32): "npub1satc3v6yvj3fszq4y9zzat58lkcyv04dnhfxcks3u2q92ra3fs6qmgvwxe" 
Secret Key (bech32): "nsec17g0gwjpyrd87tjsjngn9sekm4qpcwavv8n07k5ukfpnju0zmycsswzpe32" 
*/
fn bob_keys() -> (String, String) {
    ("f21e8748241b4fe5ca129a265866dba80387758c3cdfeb539648672e3c5b2621".to_string(),
    "875788b34464a298081521442eae87fdb0463ead9dd26c5a11e280550fb14c34".to_string())
}

/*  Charlie 
Secret Key (HEX): "11710748a7e6774514b781f8328d030d2f74ad3cf4a79c244c12e53b110d9852" 
Public Key (HEX): "c05ffa62d26beb3fefc336fc95b7669f8890072db78930f9f8ad4838d9d9560f" 
Public Key (bech32): "npub1cp0l5ckjd04nlm7rxm7ftdmxn7yfqpedk7ynp70c44yr3kwe2c8sxkavnz" 
Secret Key (bech32): "nsec1z9cswj98uem5299hs8ur9rgrp5hhftfu7jnecfzvztjnkygdnpfq4t03q9" 
*/
fn charlie_keys() -> (String, String) {
    ("11710748a7e6774514b781f8328d030d2f74ad3cf4a79c244c12e53b110d9852".to_string(),
    "c05ffa62d26beb3fefc336fc95b7669f8890072db78930f9f8ad4838d9d9560f".to_string())
}

fn _random_account() -> (String, String) {
    let (secret_key_1, _public_key_1) = get_random_secret_key();

    let (secret_str_1, public_str_1) = get_str_keys_from_secret(&secret_key_1);

    println!("Secret Key (HEX): {:?} ", secret_str_1); 
    println!("Public Key (HEX): {:?} ", public_str_1); 

    let bech32_pub = to_bech32(ToBech32Kind::PublicKey, &public_str_1);
    let bech32_prv = to_bech32(ToBech32Kind::SecretKey, &secret_str_1);

    println!("Public Key (bech32): {:?} ", bech32_pub.unwrap());
    println!("Secret Key (bech32): {:?} ", bech32_prv.unwrap());

    (secret_str_1, public_str_1)
}

fn handle_message(message: &Message) -> Result<(), String> {
    let events = extract_events_ws(message);
    println!("{}", serde_json::to_string_pretty(&events).unwrap());

    Ok(())
}

fn subscribe(nostr_client: Arc<Mutex<Client>>) {
    
     // Run a new thread to handle messages
     let subscription_id = nostr_client
     .lock()
     .unwrap()
     .subscribe(vec![ReqFilter {
         ids: None,
         authors: Some(vec![
            alice_keys().1, bob_keys().1, charlie_keys().1
         ]),
         kinds: None,
         e: None,
         p: None,
         since: None,
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
        // coinstr post bob "hello everyone"
        let mut prv_key = "".to_string();
        if args[2] == "bob" {
            prv_key = bob_keys().0;
        } else if args[2] == "alice" {
            prv_key = alice_keys().0;
        } else if args[2] == "charlie" {
            prv_key = charlie_keys().0;
        }
        let message = &args[3];

        let poster_identity = Identity::from_str(&prv_key).unwrap();
        
        nostr_client
            .lock()
            .unwrap()
            .publish_text_note(&poster_identity, &message, &[], 0)
            .unwrap();

    }


    // 

    // let (secret_str_1, public_str_1 ) = random_account(); // random_account();

    



    // // Run a new thread to handle messages
    // let nostr_clone = nostr_client.clone();
    // let handle_thread = thread::spawn(move || {
    //     println!("Listening...");
    //     let events = nostr_clone.lock().unwrap().next_data().unwrap();

    //     for (relay_url, message) in events.iter() {
    //         handle_message(relay_url, message).unwrap();
    //     }
    // });

     // Change metadata
    //  nostr_client
    //  .lock()
    //  .unwrap()
    //  .set_metadata(
    //      &my_identity,
    //      Some("Rust Nostr Client test account"),
    //      Some("Hello Nostr! #5"),
    //      None,
    //      None,
    //      0,
    //  )
    //  .unwrap();

    //  let subscription_id = nostr_client
    //  .lock()
    //  .unwrap()
    //  .subscribe(vec![ReqFilter {
    //      ids: None,
    //      authors: Some(vec![
    //         alice_keys().1, bob_keys().1, charlie_keys().1
    //      ]),
    //      kinds: None,
    //      e: None,
    //      p: None,
    //      since: None,
    //      until: None,
    //      limit: Some(1),
    //  }])
    //  .unwrap();

    //  nostr_client
    //     .lock()
    //     .unwrap()
    //     .publish_text_note(&my_identity, "bamboozler :)", &[], 0)
    //     .unwrap();

    // // Unsubscribe
    // nostr_client
    //     .lock()
    //     .unwrap()
    //     .unsubscribe(&subscription_id)
    //     .unwrap();

    // handle_thread.join().unwrap();
}

