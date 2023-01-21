use bdk::template::Bip84;
use bdk::{miniscript, Wallet, KeychainKind};
use bdk::bitcoin::Network;
use bdk::database::MemoryDatabase;
use bdk::keys::{DerivableKey, GeneratableKey, GeneratedKey, ExtendedKey, bip39::{Mnemonic, WordCount, Language}};
use bdk::bitcoin::secp256k1::SecretKey;
use bdk::bitcoin::{PrivateKey};


// pub fn random_account() -> (String, String) {

//     let hash = secp256k1::bitcoin_hashes::sha256::Hash(alias.as_bytes());

//     let sk = PrivateKey {
//         compressed: true,
//         network: Network::Testnet,
//         inner: SecretKey::from_slice(&hash).expect("32 bytes, within curve order"),
//     };

//     let network = Network::Testnet; // Or this can be Network::Bitcoin, Network::Signet or Network::Regtest

//     // Generate fresh mnemonic
//     let mnemonic: GeneratedKey<_, miniscript::Segwitv0> = Mnemonic::generate((WordCount::Words12, Language::English)).unwrap();

//     // Convert mnemonic to string
//     let mnemonic_words = mnemonic.to_string();
//     println!("Mnemonic: {:?} ", &mnemonic_words);

//     // Parse a mnemonic
//     let mnemonic  = Mnemonic::parse(&mnemonic_words).unwrap();
    
//     // Generate the extended key
//     let xkey: ExtendedKey = mnemonic.into_extended_key().unwrap();
//     // Get xprv from the extended key
//     let xprv = xkey.into_xprv(network).unwrap();
//     println!("Secret Key xprv: {:?} ", xprv);

//     (xprv.to_string(), mnemonic_words)
// }
/*  Alice
Secret Key (HEX): "3bc51062973c458d5a6f2d8d64a023246354ad7e064b1e4e009ec8a0699a3043" 
Public Key (HEX): "7e5ccd015578969febb42468f8d0be54c6b39331b7285d88040d5f0ba9606aa4" 
Public Key (bech32): "npub10ewv6q240ztfl6a5y35035972nrt8ye3ku59mzqyp40sh2tqd2jqveljzy" 
Secret Key (bech32): "nsec180z3qc5h83zc6kn09kxkfgpry334ftt7qe93unsqnmy2q6v6xppsv4ck26" 
*/
pub fn alice_keys() -> (String, String) {
    ("3bc51062973c458d5a6f2d8d64a023246354ad7e064b1e4e009ec8a0699a3043".to_string(), // updated
    "7e5ccd015578969febb42468f8d0be54c6b39331b7285d88040d5f0ba9606aa4".to_string())
}

/*  Bob 
Secret Key (HEX): "cd9fb1e148ccd8442e5aa74904cc73bf6fb54d1d54d333bd596aa9bb4bb4e961" 
Public Key (HEX): "476b018f75b1084e4b2bd652a747a37de9727183bcfe4113fe0b9390767e3543" 
Public Key (bech32): "npub1ga4srrm4kyyyujet6ef2w3ar0h5hyuvrhnlyzyl7pwfeqan7x4psmtdtkk" 
Secret Key (bech32): "nsec1ek0mrc2genvygtj65aysfnrnhahm2nga2nfn802ed25mkja5a9sstwpm9k" 
*/
pub fn bob_keys() -> (String, String) {
    ("cd9fb1e148ccd8442e5aa74904cc73bf6fb54d1d54d333bd596aa9bb4bb4e961".to_string(),
    "476b018f75b1084e4b2bd652a747a37de9727183bcfe4113fe0b9390767e3543".to_string())
}

/*  Charlie 
Secret Key (HEX): "6e81b1255ad51bb201a2b8afa9b66653297ae0217f833b14b39b5231228bf968" 
Public Key (HEX): "3254bcb92a82208ac8d864f3772c1576eb12dd97f1110d858cedb58251ba5043" 
Public Key (bech32): "npub1xf2tewf2sgsg4jxcvnehwtq4wm439hvh7ygsmpvvak6cy5d62ppsk84lf4" 
Secret Key (bech32): "nsec1d6qmzf2665dmyqdzhzh6ndnx2v5h4cpp07pnk99nndfrzg5tl95qwfu7cz" 
*/
pub fn charlie_keys() -> (String, String) {
    ("6e81b1255ad51bb201a2b8afa9b66653297ae0217f833b14b39b5231228bf968".to_string(),
    "3254bcb92a82208ac8d864f3772c1576eb12dd97f1110d858cedb58251ba5043".to_string())
}
