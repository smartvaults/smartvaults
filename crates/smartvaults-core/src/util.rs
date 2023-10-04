// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::str::FromStr;

use ::serde::{Deserialize, Deserializer, Serializer};
use keechain_core::bdk::descriptor::{DescriptorError, IntoWalletDescriptor};
use keechain_core::bdk::keys::KeyError;
use keechain_core::bitcoin::psbt::PartiallySignedTransaction;
use keechain_core::bitcoin::secp256k1::rand::rngs::OsRng;
use keechain_core::bitcoin::Network;
use keechain_core::miniscript::Descriptor;
use keechain_core::secp256k1::{Secp256k1, Signing, XOnlyPublicKey};
pub use keechain_core::util::*;

use crate::SECP256K1;

pub trait Unspendable {
    fn unspendable<C>(secp: &Secp256k1<C>) -> Self
    where
        C: Signing;
}

impl Unspendable for XOnlyPublicKey {
    fn unspendable<C>(secp: &Secp256k1<C>) -> Self
    where
        C: Signing,
    {
        let mut rng = OsRng;
        let (_, public_key) = secp.generate_keypair(&mut rng);
        let (public_key, _) = public_key.x_only_public_key();
        public_key
    }
}

pub(crate) fn serialize_psbt<S>(
    psbt: &PartiallySignedTransaction,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&psbt.to_string())
}

pub(crate) fn deserialize_psbt<'de, D>(
    deserializer: D,
) -> Result<PartiallySignedTransaction, D::Error>
where
    D: Deserializer<'de>,
{
    let psbt = String::deserialize(deserializer)?;
    PartiallySignedTransaction::from_str(&psbt).map_err(::serde::de::Error::custom)
}

/// Search the [`Network`] of the descriptor
#[tracing::instrument(skip_all, level = "trace")]
pub fn search_network_for_descriptor(desc: &Descriptor<String>) -> Option<Network> {
    let descriptor: String = desc.to_string();
    let descriptor: &str = descriptor.as_str();
    for network in [
        Network::Bitcoin,
        Network::Testnet,
        Network::Signet,
        Network::Regtest,
    ]
    .into_iter()
    {
        match descriptor.into_wallet_descriptor(&SECP256K1, network) {
            Ok(_) => return Some(network),
            Err(DescriptorError::Key(KeyError::InvalidNetwork)) => continue,
            _ => return None,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use core::str::FromStr;

    use super::*;

    #[test]
    fn test_search_network_for_descriptor() {
        let desc = Descriptor::from_str("tr([e157a520/86'/1'/784923']tpubDCCYFYCyDkxo1xAzDpoFNdtGcjD5BPLZbEJswjJmwqp67Weqd2C7fg6Jy1SBjgn3wYnKyUtoYKXG4VdQczjqb6FJnqHe3NmFdgy8vNBSty4/0/*,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*))#m9nhr276").unwrap();
        let network = search_network_for_descriptor(&desc);
        assert_eq!(network, Some(Network::Testnet));

        let desc = Descriptor::from_str("tr([5cb492a5/86'/1'/784923']tpubDD56LAR1MR7X5EeZYMpvivk2Lh3HMo4vdDNQ8jAv4oBjLPEddQwxaxNypvrHbMk2qTxAj44YLzqHrzwy5LDNmVyYZBesm6aShhmhYrA8veT/0/*,{pk([76fdbca2/86'/1'/784923']tpubDCDepsNyAPWySAgXx1Por6sHpSWzxsTB9XJp5erEN7NumgdZMhhmycJGMQ1cHZwx66KyZr6psjttDDQ7mV4uJGV2DvB9Mri1nTVmpquvTDR/0/*),pk([3b8ae29b/86'/1'/784923']tpubDDpkQsJQTpHi2bH5Cg7L1pThUxeEStcn9ZsQ53XHkW8Fs81h71XobqpwYf2Jb8ECmW1mUUJxQhZstmwFUg5wQ6EVzH5HmF3cpHcyxjvF1Ep/0/*)})#yxpuntg3").unwrap();
        let network = search_network_for_descriptor(&desc);
        assert_eq!(network, Some(Network::Testnet));

        let desc = Descriptor::from_str("wsh(multi(1,xpub661MyMwAqRbcFW31YEwpkMuc5THy2PSt5bDMsktWQcFF8syAmRUapSCGu8ED9W6oDMSgv6Zz8idoc4a6mr8BDzTJY47LJhkJ8UB7WEGuduB/1/0/*,xpub69H7F5d8KSRgmmdJg2KhpAK8SR3DjMwAdkxj3ZuxV27CprR9LgpeyGmXUbC6wb7ERfvrnKZjXoUmmDznezpbZb7ap6r1D3tgFxHmwMkQTPH/0/0/*))").unwrap();
        let network = search_network_for_descriptor(&desc);
        assert_eq!(network, Some(Network::Bitcoin));
    }
}

#[cfg(bench)]
mod benches {
    use test::{black_box, Bencher};

    use super::*;

    #[bench]
    pub fn bench_search_network_for_descriptor(bh: &mut Bencher) {
        let desc = Descriptor::from_str("tr([e157a520/86'/1'/784923']tpubDCCYFYCyDkxo1xAzDpoFNdtGcjD5BPLZbEJswjJmwqp67Weqd2C7fg6Jy1SBjgn3wYnKyUtoYKXG4VdQczjqb6FJnqHe3NmFdgy8vNBSty4/0/*,pk([7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*))#m9nhr276").unwrap();
        bh.iter(|| {
            black_box(search_network_for_descriptor(&desc));
        });
    }
}
