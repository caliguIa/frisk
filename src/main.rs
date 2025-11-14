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
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    let start = std::time::Instant::now();

    let config = Config::load(args.config)?;

    let elements = apps::discover_applications()?;

    info!(
        "Startup took {:?}, found {} apps",
        start.elapsed(),
        elements.len()
    );

    gui::run(config, elements)?;

    Ok(())
}
