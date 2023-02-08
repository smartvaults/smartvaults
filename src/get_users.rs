use crate::users::User;

#[derive(Debug, Clone, clap::Parser)]
#[command(name = "users", about = "Get a list of known users")]
pub struct GetUsersCmd {}

use clap::Error;
impl GetUsersCmd {
    /// Run the command
    pub fn run(&self) -> Result<(), Error> {

        // let subscriber = User::get(&self.subscriber);
        // let publisher = User::get(&self.publisher);

        for user in User::known_users() {
            println!("{}", user.name.unwrap());
        };

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
