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

    let start = std::time::Instant::now();

    let config = Config::load(args.config)?;

    let elements = apps::discover_applications()?;

    crate::log!(
        "Startup took {:?}, found {} apps",
        start.elapsed(),
        elements.len()
    );

    gui::run(config, elements)?;

    Ok(())
}
