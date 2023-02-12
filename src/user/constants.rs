use std::collections::HashMap;

#[allow(dead_code)]
pub struct UserConstants {
	pub name: &'static str,
	pub mnemonic: &'static str,
	pub passphrase: &'static str,
}

#[allow(dead_code)]
static ALICE: (&str, &str, &str) = (
	"Alice",
	"carry surface crater rude auction ritual banana elder shuffle much wonder decrease",
	"oy+hB/qeJ1AasCCR",
);

#[allow(dead_code)]
static BOB: (&str, &str, &str) = (
	"Bob",
	"market museum car noodle cream pool enhance please level price slide process",
	"B3Q0YHYYHmF798Jg",
);

#[allow(dead_code)]
static CHARLIE: (&str, &str, &str) = (
	"Chalie",
	"cry modify gallery home desert tongue immune address bunker bean tone giggle",
	"nTVuKiINc5TKMjfV",
);

#[allow(dead_code)]
static DAVID: (&str, &str, &str) = (
	"David",
	"alone hospital depth worth vapor lazy burst skill apart accuse maze evidence",
	"f5upOqUyG0iPY4n+",
);

#[allow(dead_code)]
static ERIKA: (&'static str, &str, &str) = (
	"Erika",
	"confirm rifle kit warrior aware clump shallow eternal real shift puzzle wife",
	"JBtdXy+2ut2fxplW",
);

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
}
