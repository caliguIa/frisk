use anyhow::Result;
use clap::Parser;
use log::info;
use std::path::PathBuf;

mod apps;
mod calculator;
mod config;
mod element;
mod gui;

use config::Config;

#[derive(Parser)]
#[command(name = "kickoff")]
#[command(about = "A fast and minimal program launcher for macOS")]
pub struct Args {
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[arg(short, long)]
    prompt: Option<String>,
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    info!("Starting kickoff");

    let mut config = Config::load(args.config)?;

    if let Some(prompt) = args.prompt {
        config.prompt = prompt;
    }

    let elements = apps::discover_applications()?;

    info!("Discovered {} applications", elements.len());

    gui::run(config, elements)?;

    Ok(())
}
