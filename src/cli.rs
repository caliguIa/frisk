use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "kickoff")]
#[command(about = "Fast and minimal program launcher for macOS", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to config file
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Custom prompt text
    #[arg(short, long)]
    pub prompt: Option<String>,

    /// Additional binary source files to load
    #[arg(short, long)]
    pub source: Vec<PathBuf>,

    /// Load apps.bin
    #[arg(long)]
    pub apps: bool,

    /// Load homebrew.bin
    #[arg(long, alias = "brew")]
    pub homebrew: bool,

    /// Load clipboard.bin
    #[arg(long, alias = "clip")]
    pub clipboard: bool,

    /// Load custom commands from config
    #[arg(long)]
    pub commands: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage LaunchAgent services
    Service {
        #[command(subcommand)]
        command: ServiceCommands,
    },
    /// Run a daemon (called by LaunchAgent)
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum ServiceCommands {
    /// Install a LaunchAgent service
    Install {
        /// Service name: apps, homebrew, clipboard, or all
        name: String,
    },
    /// Uninstall a LaunchAgent service
    Uninstall {
        /// Service name: apps, homebrew, clipboard, or all
        name: String,
    },
    /// Start a LaunchAgent service
    Start {
        /// Service name: apps, homebrew, clipboard, or all
        name: String,
    },
    /// Stop a LaunchAgent service
    Stop {
        /// Service name: apps, homebrew, clipboard, or all
        name: String,
    },
    /// Show status of all services
    Status,
    /// List available services
    List,
}

#[derive(Subcommand, Debug)]
pub enum DaemonCommands {
    /// Watch application directories and update apps.bin
    Apps,
    /// Fetch homebrew packages and update homebrew.bin
    Homebrew,
    /// Monitor clipboard and update clipboard.bin
    Clipboard,
}

pub fn parse_service_name(name: &str) -> Option<Vec<&'static str>> {
    match name {
        "apps" => Some(vec!["apps"]),
        "homebrew" | "brew" => Some(vec!["homebrew"]),
        "clipboard" | "clip" => Some(vec!["clipboard"]),
        "all" => Some(vec!["apps", "homebrew", "clipboard"]),
        _ => None,
    }
}
