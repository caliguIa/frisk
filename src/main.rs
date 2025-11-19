use std::fs;
use std::path::PathBuf;
use std::time::Instant;

mod cache;
mod calculator;
mod cli;
mod commands;
mod config;
mod daemons;
mod element;
mod error;
mod gui;
mod loader;
mod service;

#[macro_use]
mod log;

use clap::Parser;
use cli::{Cli, Commands};
use config::Config;
use error::Result;
use loader::{load_binary_file, load_binary_source};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Service { command }) => {
            service::handle_service_command(command)?;
            Ok(())
        }
        Some(Commands::Daemon { command }) => {
            use cli::DaemonCommands;
            match command {
                DaemonCommands::Apps => daemons::apps::run(),
                DaemonCommands::Homebrew => daemons::homebrew::run(),
                DaemonCommands::Clipboard => daemons::clipboard::run(),
            }
        }
        None => run_gui(cli),
    }
}

/// Check if another instance is already running
fn check_single_instance() -> Result<()> {
    let lock_file = get_lock_file_path()?;

    // Check if lock file exists and process is still running
    if lock_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(&lock_file) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                // Check if process is still running using kill -0
                let status = std::process::Command::new("kill")
                    .arg("-0")
                    .arg(pid.to_string())
                    .status();

                if status.is_ok() && status.unwrap().success() {
                    // Process is still running
                    return Err(error::Error::new(
                        "Another instance of kickoff is already running",
                    ));
                }
            }
        }
        // Old lock file from crashed process, remove it
        let _ = fs::remove_file(&lock_file);
    }

    // Write our PID to lock file
    let pid = std::process::id();
    fs::write(&lock_file, pid.to_string())?;

    Ok(())
}

/// Get path to lock file
fn get_lock_file_path() -> Result<PathBuf> {
    let runtime_dir = if let Ok(dir) = std::env::var("TMPDIR") {
        PathBuf::from(dir)
    } else {
        PathBuf::from("/tmp")
    };
    Ok(runtime_dir.join("kickoff.lock"))
}

fn run_gui(cli: Cli) -> Result<()> {
    // Ensure only one instance is running
    check_single_instance()?;

    let start = Instant::now();

    let config = if let Some(config_path) = cli.config {
        Config::load(Some(config_path))?
    } else {
        Config::load(None)?
    };

    // Override prompt if specified
    let config = if let Some(prompt) = cli.prompt {
        let mut config = config;
        config.prompt = prompt;
        config
    } else {
        config
    };

    let after_config = Instant::now();

    // Load elements from binary sources
    let mut elements = element::ElementList::new();

    // Load apps from apps.bin if requested
    if cli.apps {
        if let Some(apps) = load_binary_source("apps.bin")? {
            let count = apps.len();
            for app in apps {
                elements.add(app);
            }
            crate::log!("Loaded {} apps from apps.bin", count);
        } else {
            eprintln!("Warning: --apps specified but apps.bin not found");
            eprintln!("Run: kickoff service install apps && kickoff service start apps");
        }
    }

    // Load homebrew from homebrew.bin if requested
    if cli.homebrew {
        if let Some(homebrew_apps) = load_binary_source("homebrew.bin")? {
            let count = homebrew_apps.len();
            for app in homebrew_apps {
                elements.add(app);
            }
            crate::log!("Loaded {} homebrew packages from homebrew.bin", count);
        } else {
            eprintln!("Warning: --homebrew specified but homebrew.bin not found");
            eprintln!("Run: kickoff service install homebrew && kickoff service start homebrew");
        }
    }

    // Load clipboard from clipboard.bin if requested
    if cli.clipboard {
        if let Some(clipboard_items) = load_binary_source("clipboard.bin")? {
            let count = clipboard_items.len();
            for item in clipboard_items {
                elements.add(item);
            }
            crate::log!("Loaded {} clipboard items from clipboard.bin", count);
        } else {
            eprintln!("Warning: --clipboard specified but clipboard.bin not found");
            eprintln!("Run: kickoff service install clipboard && kickoff service start clipboard");
        }
    }

    // Load additional custom sources
    for source_path in &cli.source {
        match load_binary_file(source_path) {
            Ok(items) => {
                let count = items.len();
                for item in items {
                    elements.add(item);
                }
                crate::log!("Loaded {} items from {:?}", count, source_path);
            }
            Err(e) => {
                crate::log!("Failed to load source {:?}: {}", source_path, e);
            }
        }
    }

    // Load custom commands from config (only if --commands flag is set)
    if cli.commands {
        match commands::CommandsConfig::load() {
            Ok(commands_config) => {
                for cmd in commands_config.to_elements() {
                    elements.add(cmd);
                }
                crate::log!("Loaded {} custom commands", commands_config.command.len());
            }
            Err(e) => {
                eprintln!("Warning: Failed to load commands config: {}", e);
            }
        }
    }

    let after_discovery = Instant::now();

    crate::log!(
        "Loaded {} items (apps + system commands), estimated memory: ~{} KB",
        elements.len(),
        elements.len() * 60 / 1024 // rough estimate
    );

    crate::log!("⏱️  Timing breakdown:");
    crate::log!(
        "  Config loading:  {:>6.2}ms",
        (after_config - start).as_secs_f64() * 1000.0
    );
    crate::log!(
        "  Data loading:    {:>6.2}ms",
        (after_discovery - after_config).as_secs_f64() * 1000.0
    );
    crate::log!(
        "  Total to GUI:    {:>6.2}ms",
        (after_discovery - start).as_secs_f64() * 1000.0
    );

    gui::run(config, elements)?;

    // Clean up lock file on normal exit
    if let Ok(lock_file) = get_lock_file_path() {
        let _ = fs::remove_file(lock_file);
    }

    Ok(())
}
