use nostr_sdk::Result;
use std::{collections::HashMap, str::FromStr};

use super::User;
use keechain_core::{bip39::Mnemonic, bitcoin::Network, types::Seed};

pub fn get_known_user(user_info: (&str, &str, &str)) -> Result<User> {
	let mnemonic = Mnemonic::from_str(user_info.1)?;
	let seed = Seed::new(mnemonic, Some(user_info.2));
	User::new(seed, Some(user_info.0.to_string()), Network::Testnet)
}

#[allow(dead_code)]
pub struct UserConstants {
	pub name: &'static str,
	pub mnemonic: &'static str,
	pub passphrase: &'static str,
}

#[allow(dead_code)]
pub static ALICE: (&str, &str, &str) = (
	"Alice",
	"carry surface crater rude auction ritual banana elder shuffle much wonder decrease",
	"oy+hB/qeJ1AasCCR",
);

#[allow(dead_code)]
pub static BOB: (&str, &str, &str) = (
	"Bob",
	"market museum car noodle cream pool enhance please level price slide process",
	"B3Q0YHYYHmF798Jg",
);

#[allow(dead_code)]
pub static CHARLIE: (&str, &str, &str) = (
	"Charlie",
	"cry modify gallery home desert tongue immune address bunker bean tone giggle",
	"nTVuKiINc5TKMjfV",
);

#[allow(dead_code)]
pub static DAVID: (&str, &str, &str) = (
	"David",
	"alone hospital depth worth vapor lazy burst skill apart accuse maze evidence",
	"f5upOqUyG0iPY4n+",
);

#[allow(dead_code)]
pub static ERIKA: (&str, &str, &str) = (
	"Erika",
	"confirm rifle kit warrior aware clump shallow eternal real shift puzzle wife",
	"JBtdXy+2ut2fxplW",
);

// H20 fictional users
#[allow(dead_code)]
pub static SARAH: (&str, &str, &str) = (
	"Sarah",
	"height syrup aware bottom black sting easily priority weather cattle spread ethics",
	"",
);

#[allow(dead_code)]
pub static JOHN: (&str, &str, &str) =
	("John", "please gas allow carpet type twelve smoke perfect rotate shed clay rough", "");

#[allow(dead_code)]
pub static MARIA: (&str, &str, &str) =
	("Maria", "fox supreme basic limb total supply expect very seat invite play marine", "");

#[allow(dead_code)]
pub static LEE: (&str, &str, &str) =
	("Lee", "volume lyrics health attitude hidden enable afford grid ozone rotate wash blood", "");

#[allow(dead_code)]
pub static RACHEL: (&str, &str, &str) =
	("Rachel", "dwarf bike rocket decline exact shine pepper daughter fly cabbage door hockey", "");

#[allow(dead_code)]
pub static JAMES: (&str, &str, &str) =
	("James", "denial digital dutch toss final clerk ladder demise where oval border flip", "");

#[allow(dead_code)]
pub static KAREN: (&str, &str, &str) =
	("Karen", "social middle funny frown client mad claim reflect almost loud mesh wool", "");

#[allow(dead_code)]
pub static MARK: (&str, &str, &str) =
	("Mark", "cream vivid future inject spirit gaze predict vessel damp able wedding trouble", "");

#[allow(dead_code)]
pub static AMANDA: (&str, &str, &str) = (
	"Amanda",
	"print gorilla version install avoid surface famous live solve gasp trophy page",
	"",
);

#[allow(dead_code)]
pub static TREY: (&str, &str, &str) =
	("Trey", "isolate assist curve wing dial october hen tree radar canyon charge local", "");

#[allow(dead_code)]
pub fn user_constants() -> HashMap<String, UserConstants> {
	let mut users = HashMap::new();
	users.insert(
		ALICE.0.to_string(),
		UserConstants { name: ALICE.0, mnemonic: ALICE.1, passphrase: ALICE.2 },
	);
	users.insert(
		BOB.0.to_string(),
		UserConstants { name: BOB.0, mnemonic: BOB.1, passphrase: BOB.2 },
	);
	users.insert(
		CHARLIE.0.to_string(),
		UserConstants { name: CHARLIE.0, mnemonic: CHARLIE.1, passphrase: CHARLIE.2 },
	);
	users.insert(
		DAVID.0.to_string(),
		UserConstants { name: DAVID.0, mnemonic: DAVID.1, passphrase: DAVID.2 },
	);
	users.insert(
		ERIKA.0.to_string(),
		UserConstants { name: ERIKA.0, mnemonic: ERIKA.1, passphrase: ERIKA.2 },
	);
	users
}

#[allow(dead_code)]
pub fn known_user_names() -> Vec<String> {
	user_constants().into_keys().collect::<Vec<String>>()
}

#[cfg(test)]
mod tests {

	use super::*;

	#[test]
	fn dump_raw_users() {
		let users = user_constants();
		for (name_key, user_constant) in users.iter() {
			assert_eq!(name_key.clone(), user_constant.name.to_string());
			println!("Name          : {}", name_key);
			println!("Mnemonic      : {}", user_constant.mnemonic);
			println!("Passphrase    : {}", user_constant.passphrase);
			println!();
		}
	}

	#[test]
	fn test_h20_user() {
		let sarah = get_known_user(SARAH).unwrap();
		println!("{}", sarah);
		assert_eq!(sarah.name.unwrap(), "Sarah");
		assert_eq!(
			sarah.seed.mnemonic().to_string(),
			"height syrup aware bottom black sting easily priority weather cattle spread ethics"
				.to_string()
		);
	}
}
