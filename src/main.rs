use anyhow::Result;
use std::time::Instant;

mod apps;
mod args;
mod calculator;
mod config;
mod element;
mod gui;

#[macro_use]
mod log;

use args::Args;
use config::Config;

fn main() -> Result<()> {
    let start = Instant::now();

    let args = Args::parse();
    let after_args = Instant::now();

    let config = Config::load(args.config)?;
    let after_config = Instant::now();

    let elements = apps::discover_applications()?;
    let after_discovery = Instant::now();

    crate::log!(
        "Loaded {} apps, estimated memory: ~{} KB",
        elements.len(),
        elements.len() * 60 / 1024 // rough estimate
    );

    crate::log!("⏱️  Timing breakdown:");
    crate::log!(
        "  Args parsing:    {:>6.2}ms",
        (after_args - start).as_secs_f64() * 1000.0
    );
    crate::log!(
        "  Config loading:  {:>6.2}ms",
        (after_config - after_args).as_secs_f64() * 1000.0
    );
    crate::log!(
        "  App discovery:   {:>6.2}ms",
        (after_discovery - after_config).as_secs_f64() * 1000.0
    );
    crate::log!(
        "  Total to GUI:    {:>6.2}ms",
        (after_discovery - start).as_secs_f64() * 1000.0
    );

    gui::run(config, elements)?;

    Ok(())
}
