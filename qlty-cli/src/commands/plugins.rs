use crate::{CommandError, CommandSuccess};
use anyhow::Result;
use clap::{Args, Subcommand};

mod enable;
mod list;

pub use enable::Enable;
pub use list::List;

#[derive(Debug, Args)]
pub struct Arguments {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]

pub enum Commands {
    /// Enable plugins
    Enable(Enable),

    /// List all available plugins
    List(List),
}

impl Arguments {
    pub fn execute(&self, args: &crate::Arguments) -> Result<CommandSuccess, CommandError> {
        match &self.command {
            Commands::Enable(command) => command.execute(args),
            Commands::List(command) => command.execute(args),
        }
    }
}
