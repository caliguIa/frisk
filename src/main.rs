use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
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

static LOCK_FILE: Mutex<Option<PathBuf>> = Mutex::new(None);

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
        None => {
            let result = run_gui(cli);
            cleanup_lock_file();
            result
        }
    }
}

fn check_single_instance() -> Result<()> {
    let lock_file = get_lock_file_path()?;

    if lock_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(&lock_file) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                let status = std::process::Command::new("kill")
                    .arg("-0")
                    .arg(pid.to_string())
                    .stderr(std::process::Stdio::null())
                    .status();

                if status.is_ok() && status.unwrap().success() {
                    return Err(error::Error::new(
                        "Another instance of frisk is already running",
                    ));
                }
            }
        }
        let _ = fs::remove_file(&lock_file);
    }

    let pid = std::process::id();
    fs::write(&lock_file, pid.to_string())?;

    *LOCK_FILE.lock().unwrap() = Some(lock_file);

    Ok(())
}

fn cleanup_lock_file() {
    if let Some(path) = LOCK_FILE.lock().unwrap().take() {
        let _ = fs::remove_file(path);
    }
}

fn get_lock_file_path() -> Result<PathBuf> {
    let runtime_dir = if let Ok(dir) = std::env::var("TMPDIR") {
        PathBuf::from(dir)
    } else {
        PathBuf::from("/tmp")
    };
    Ok(runtime_dir.join("frisk.lock"))
}

fn run_gui(cli: Cli) -> Result<()> {
    check_single_instance()?;

    let start = Instant::now();

    let config = if let Some(config_path) = cli.config {
        Config::load(Some(config_path))?
    } else {
        Config::load(None)?
    };

    let config = if let Some(prompt) = cli.prompt {
        let mut config = config;
        config.prompt = prompt;
        config
    } else {
        config
    };

    let after_config = Instant::now();

    let mut elements = element::ElementList::new();

    if cli.apps {
        if let Some(apps) = load_binary_source("apps.bin")? {
            let count = apps.len();
            for app in apps {
                elements.add(app);
            }
            crate::log!("Loaded {} apps from apps.bin", count);
        } else {
            eprintln!("Warning: --apps specified but apps.bin not found");
            eprintln!("Run: frisk service install apps && frisk service start apps");
        }
    }

    if cli.homebrew {
        if let Some(homebrew_apps) = load_binary_source("homebrew.bin")? {
            let count = homebrew_apps.len();
            for app in homebrew_apps {
                elements.add(app);
            }
            crate::log!("Loaded {} homebrew packages from homebrew.bin", count);
        } else {
            eprintln!("Warning: --homebrew specified but homebrew.bin not found");
            eprintln!("Run: frisk service install homebrew && frisk service start homebrew");
        }
    }

    if cli.clipboard {
        if let Some(clipboard_items) = load_binary_source("clipboard.bin")? {
            let count = clipboard_items.len();
            for item in clipboard_items {
                elements.add(item);
            }
            crate::log!("Loaded {} clipboard items from clipboard.bin", count);
        } else {
            eprintln!("Warning: --clipboard specified but clipboard.bin not found");
            eprintln!("Run: frisk service install clipboard && frisk service start clipboard");
        }
    }

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
        elements.len() * 60 / 1024
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

    Ok(())
}
