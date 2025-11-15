use anyhow::Result;
use std::time::Instant;

mod apps;
mod args;
mod calculator;
mod clipboard;
mod config;
mod crates;
mod element;
mod gui;
mod homebrew;
mod nixpkgs;

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

    let mut elements = apps::discover_applications()?;
    
    // Add system commands
    use element::Element;
    elements.add(Element::new_system_command(
        "Empty Trash".to_string(),
        "osascript -e 'tell application \"Finder\" to empty trash'".to_string(),
    ));
    elements.add(Element::new_system_command(
        "Show Trash".to_string(),
        "osascript -e 'tell application \"Finder\" to open trash' && open -a finder".to_string(),
    ));
    elements.add(Element::new_system_command(
        "Restart".to_string(),
        "osascript -e 'tell application \"System Events\" to restart'".to_string(),
    ));
    elements.add(Element::new_system_command(
        "Shut Down".to_string(),
        "osascript -e 'tell application \"System Events\" to shut down'".to_string(),
    ));
    elements.add(Element::new_system_command(
        "Clipboard History".to_string(),
        "__clipboard_history__".to_string(),
    ));
    elements.add(Element::new_system_command(
        "Nixpkgs".to_string(),
        "__nixpkgs__".to_string(),
    ));
    elements.add(Element::new_system_command(
        "Crates.io".to_string(),
        "__crates__".to_string(),
    ));
    elements.add(Element::new_system_command(
        "Homebrew".to_string(),
        "__homebrew__".to_string(),
    ));
    
    let after_discovery = Instant::now();

    crate::log!(
        "Loaded {} items (apps + system commands), estimated memory: ~{} KB",
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
