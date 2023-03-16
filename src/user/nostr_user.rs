use std::fmt;

use keechain_core::types::Seed;
use nostr_sdk::prelude::*;

pub struct NostrUser {
	pub keys: Keys,
}

impl NostrUser {
	pub fn new(seed: Seed) -> Result<Self> {
		let keys = Keys::from_mnemonic(seed.mnemonic().to_string(), seed.passphrase())?;
		Ok(Self { keys })
	}

	pub fn pub_key(&self) -> Result<String> {
		Ok(self.keys.public_key().to_bech32()?)
	}

	pub fn prv_key(&self) -> Result<String> {
		Ok(self.keys.secret_key()?.to_bech32()?)
	}

	pub fn pub_key_hex(&self) -> XOnlyPublicKey {
		self.keys.public_key()
	}

	pub fn prv_key_hex(&self) -> Result<String> {
		Ok(self.keys.secret_key()?.display_secret().to_string())
	}

	pub fn pub_key_btc(&self) -> Result<String> {
		Ok(self.keys.secret_key()?.public_key(SECP256K1).to_string())
	}

}

impl fmt::Display for NostrUser {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "\nNostr Configuration")?;

		writeln!(f, " Bech32 Keys")?;
		writeln!(f, "  Public   : {} ", &self.pub_key().unwrap())?;
		writeln!(f, "  Private  : {} ", &self.prv_key().unwrap())?;

		writeln!(f, " Hex Keys")?;
		writeln!(f, "  Public   : {} ", &self.pub_key_hex())?;
		writeln!(f, "  Private  : {} ", &self.prv_key_hex().unwrap())?;
		
		writeln!(f, "\n  Public (BTC): {} ", &self.pub_key_btc().unwrap())?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use std::str::FromStr;

	use super::*;
	use crate::user::constants::user_constants;

	#[test]
	fn test_alice_keys() {
		let user_constants = user_constants();
		let alice_constants = user_constants.get(&String::from("Alice")).unwrap();
		let mnemonic = Mnemonic::from_str(alice_constants.mnemonic).unwrap();
		let seed = Seed::new(mnemonic, Some(alice_constants.passphrase));
		let alice_nostr = NostrUser::new(seed).unwrap();

		assert_eq!(
			alice_nostr.prv_key().unwrap(),
			"nsec1pcwmwsvd78ry208y9el52pacsgluy05xu860x0tl4lyr6dnwd6tsdak7nt"
		);
		assert_eq!(
			alice_nostr.pub_key().unwrap(),
			"npub1xr59p9wquc3twvtq5vy93hm3srs8k8a25j2gxd5u6nv4a6k9f58schcx7v"
		);
		assert_eq!(
			alice_nostr.prv_key_hex().unwrap(),
			"0e1db7418df1c6453ce42e7f4507b8823fc23e86e1f4f33d7fafc83d366e6e97"
		);
		assert_eq!(
			alice_nostr.pub_key_hex().to_string(),
			"30e85095c0e622b73160a30858df7180e07b1faaa49483369cd4d95eeac54d0f"
		);

		println!("Alice : {} ", alice_nostr);
	}
}
