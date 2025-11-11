use anyhow::Result;
use clap::Parser;
use log::info;
use std::path::PathBuf;

mod apps;
mod calculator;
mod config;
mod element;
mod gui;

use apps::AppDiscovery;
use config::Config;

#[derive(Parser, Debug)]
#[command(name = "kickoff")]
#[command(about = "A fast and minimal program launcher for macOS")]
pub struct Args {
    /// Custom configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Set custom prompt, overwrites config if set
    #[arg(short, long)]
    prompt: Option<String>,

    /// Include PATH executables in search
    #[arg(long)]
    include_path: bool,

    /// Include applications in search (default: true, use --no-include-applications to disable)
    #[arg(long, default_value_t = true)]
    include_applications: bool,
    
    /// Disable application search
    #[arg(long)]
    no_include_applications: bool,

    /// Show version information
    #[arg(short, long)]
    version: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    let args = Args::parse();
    
    if args.version {
        println!("kickoff-macos {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    info!("Starting kickoff for macOS");
    
    // Load configuration
    let mut config = Config::load(args.config)?;
    
    // Override prompt if provided
    if let Some(prompt) = args.prompt {
        config.prompt = prompt;
    }
    
    info!("Loaded configuration");
    
    // Discover applications based on flags
    let include_apps = args.include_applications && !args.no_include_applications;
    let discovery = AppDiscovery::new(args.include_path);
    let elements = match (include_apps, args.include_path) {
        (true, true) => discovery.discover_all().await?,
        (true, false) => discovery.discover_apps_only().await?,
        (false, true) => discovery.discover_path_only().await?,
        (false, false) => {
            // If both are disabled, default to applications only to prevent empty launcher
            eprintln!("Warning: Both applications and PATH search are disabled. Defaulting to applications only.");
            discovery.discover_apps_only().await?
        }
    };
    
    info!("Discovered {} applications", elements.len());
    
    // Run GUI
    info!("Starting GUI");
    gui::run(config, elements)?;
    
    Ok(())
}