use crate::{Arguments, CommandError, CommandSuccess};
use anyhow::Result;
use clap::Args;
use qlty_cloud::Client;
#[derive(Args, Debug)]
pub struct Whoami {}

impl Whoami {
    pub fn execute(&self, _args: &Arguments) -> Result<CommandSuccess, CommandError> {
        let token = qlty_cloud::Token::default();

        match token.get_with_interactive_prompt() {
            Ok(token) => {
                let client = Client {
                    base_url: "https://api.qlty.sh".to_string(),
                    token: Some(token),
                };

                let json = client
                    .get("/user")
                    .call()?
                    .into_json::<serde_json::Value>()?;

                match json.get("email") {
                    Some(email_value) => match email_value.as_str() {
                        Some(email) => println!("{}", email),
                        None => return CommandError::err("Invalid email format"),
                    },
                    None => return CommandError::err("Email not found"),
                };
            }
            Err(_) => {
                return CommandError::err(
                    "No access token available. Please login with 'qlty auth login'",
                );
            }
        }

        CommandSuccess::ok()
    }
}