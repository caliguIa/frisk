mod cache;
mod cli;
mod core;
mod gui;
mod instance;
mod ipc;
mod loader;
mod picker;
mod services;

#[macro_use]
mod log;

use clap::Parser;
use cli::{Cli, Commands};
use core::error::Result;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Service { command }) => {
            services::handle_service_command(command)?;
            Ok(())
        }
        Some(Commands::Daemon { command }) => {
            use cli::DaemonCommands;
            match command {
                DaemonCommands::Apps => services::apps::run(),
                DaemonCommands::Homebrew => services::homebrew::run(),
                DaemonCommands::Clipboard => services::clipboard::run(),
                DaemonCommands::Nixpkgs => services::nixpkgs::run(),
            }
        }
        None => {
            let result = gui::run(cli);
            instance::cleanup_lock_file();
            ipc::cleanup();
            result
        }
    }
}
