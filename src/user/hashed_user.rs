use std::fmt;

use sp_core::crypto::Pair;
use subxt::{
    tx::PairSigner,
    OnlineClient,
    PolkadotConfig,
};

#[subxt::subxt(runtime_metadata_url = "wss://c1.hashed.network:443")]
pub mod polkadot {}


pub struct HashedUser {
    pub signer : PairSigner
}

impl HashedUser {
    pub fn new(mnemonic: String, passphrase: Option<String>) -> Result<HashedUser> {
		let pair = Pair::from_phrase(mnemonic.to_str(), passphrase.clone()).unwrap();
		Ok(HashedUser{ signer: PairSigner::new(pair.0) })
	}

    pub fn pub_key(&self) -> String {
        self.signer.account_id().to_string()
    }
}


impl fmt::Display for HashedUser {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "\nHashed Configuration")?;
		writeln!(f, "  Account ID   : {} ", &self.pub_key())?;
		writeln!(f, "  Address      : {} ", &self.signer.address.to_string())?;

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
		let alice_hashed = HashedUser::new(
			alice_constants.mnemonic.to_string(),
			Some(alice_constants.passphrase.to_string()),
		)
		.unwrap();

        println!("{}", alice);
    }
}
