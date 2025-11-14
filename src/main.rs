use anyhow::Result;

mod args;
mod apps;
mod calculator;
mod config;
mod element;
mod gui;

#[macro_use]
mod log;

use args::Args;
use config::Config;

fn main() -> Result<()> {
    let args = Args::parse();

    let config = Config::load(args.config)?;

    let elements = apps::discover_applications()?;

    crate::log!(
        "Loaded {} apps, estimated memory: ~{} KB",
        elements.len(),
        elements.len() * 60 / 1024  // rough estimate
    );

    gui::run(config, elements)?;

    Ok(())
}
