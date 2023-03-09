use nostr_sdk::Result;

use crate::user::User;

#[derive(Debug, Clone, clap::Parser)]
#[command(name = "users", about = "Get a list of known users")]
pub struct GetUsersCmd {}

impl GetUsersCmd {
	pub fn run(&self) -> Result<()> {
		for user in User::known_users() {
			println!("{}", user.name.unwrap());
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {

	use super::*;

	#[test]
	fn test_get_users() {
		let get_users_cmd = GetUsersCmd {};
		get_users_cmd.run().expect("Cannot get list of users");
	}
}
