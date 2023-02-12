use crate::user::User;

#[derive(Debug, Clone, clap::Parser)]
#[command(name = "user", about = "Print data about a known user by name")]
pub struct GetUserCmd {
    /// name of the user to show data for
    #[arg(short, long)]
    user: String,
}

use clap::Error;
impl GetUserCmd {
    /// Run the command
    pub fn run(&self) -> Result<(), Error> {
        let user = User::get(&self.user).expect("User not found");
        println!("{}", user);

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_get_user() {
        let get_user_cmd = GetUserCmd {
            user: "alice".to_string(),
        };
        get_user_cmd.run().expect("Cannot get list of users");
    }
}
