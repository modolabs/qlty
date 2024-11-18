use crate::{CommandError, CommandSuccess};
use anyhow::Result;
use clap::{Args, Subcommand};

mod migrate;
mod show;
mod validate;

use migrate::Migrate;
pub use show::Show;
pub use validate::Validate;

#[derive(Debug, Args, Clone)]
pub struct Arguments {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, Clone)]

pub enum Commands {
    /// Print the full configuration
    Show(Show),

    /// Validate qlty.toml
    Validate(Validate),

    /// Migrate fetch directives from .codeclimate.yml to qlty.toml
    Migrate(Migrate),
}

impl Arguments {
    pub fn execute(&self, args: &crate::Arguments) -> Result<CommandSuccess, CommandError> {
        match &self.command {
            Commands::Show(command) => command.execute(args),
            Commands::Validate(command) => command.execute(args),
            Commands::Migrate(command) => command.execute(args),
        }
    }
}
