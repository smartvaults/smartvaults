use anyhow::Result;
use nostr::{
	nips::nip19::ToBech32,
	prelude::{FromMnemonic, Secp256k1},
	Keys,
};
use nostr_sdk::prelude::*;
use std::fmt;

pub struct NostrUser {
	pub keys: nostr::key::Keys,
}

impl NostrUser {
	pub fn new(mnemonic: String, passphrase: Option<String>) -> Result<NostrUser> {
		let keys = Keys::from_mnemonic(mnemonic.clone(), passphrase.clone()).unwrap();
		Ok(NostrUser { keys })
	}

	pub fn pub_key(&self) -> String {
		let secp = Secp256k1::new();

		self.keys.secret_key().unwrap().x_only_public_key(&secp).0.to_bech32().unwrap()
	}
	pub fn prv_key(&self) -> String {
		self.keys.secret_key().unwrap().to_bech32().unwrap().to_string()
	}

	pub fn pub_key_hex(&self) -> String {
		let secp = Secp256k1::new();
		self.keys.secret_key().unwrap().x_only_public_key(&secp).0.to_string()
	}

	pub fn prv_key_hex(&self) -> String {
		self.keys.secret_key().unwrap().display_secret().to_string()
	}
}

impl fmt::Display for NostrUser {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "\nNostr Configuration")?;

		writeln!(f, " Bech32 Keys")?;
		writeln!(f, "  Public   : {} ", &self.pub_key())?;
		writeln!(f, "  Private  : {} ", &self.prv_key())?;

		writeln!(f, " Hex Keys")?;
		writeln!(f, "  Public   : {} ", &self.pub_key_hex())?;
		writeln!(f, "  Private  : {} ", &self.prv_key_hex())?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {

	use super::*;
	use crate::user::constants::user_constants;

	#[test]
	fn test_alice_keys() {
		let user_constants = user_constants();
		let alice_constants = user_constants.get(&String::from("Alice")).unwrap();
		let alice_nostr = NostrUser::new(
			alice_constants.mnemonic.to_string(),
			Some(alice_constants.passphrase.to_string()),
		)
		.unwrap();

		assert_eq!(
			alice_nostr.prv_key(),
			"nsec1pcwmwsvd78ry208y9el52pacsgluy05xu860x0tl4lyr6dnwd6tsdak7nt"
		);
		assert_eq!(
			alice_nostr.pub_key(),
			"npub1xr59p9wquc3twvtq5vy93hm3srs8k8a25j2gxd5u6nv4a6k9f58schcx7v"
		);
		assert_eq!(
			alice_nostr.prv_key_hex(),
			"0e1db7418df1c6453ce42e7f4507b8823fc23e86e1f4f33d7fafc83d366e6e97"
		);
		assert_eq!(
			alice_nostr.pub_key_hex(),
			"30e85095c0e622b73160a30858df7180e07b1faaa49483369cd4d95eeac54d0f"
		);

		println!("Alice : {} ", alice_nostr);
	}
}
