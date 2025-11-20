use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

mod cache;
mod cli;
mod core;
mod ipc;
mod loader;
mod picker;
mod services;

#[macro_use]
mod log;

use clap::Parser;
use cli::{Cli, Commands};
use core::config::Config;
use core::error::Result;
use loader::{load_binary_file, load_binary_source};

static LOCK_FILE: Mutex<Option<PathBuf>> = Mutex::new(None);

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Service { command }) => {
            services::handle_service_command(command)?;
            Ok(())
        }
        Some(Commands::Daemon { command }) => {
            use cli::DaemonCommands;
            match command {
                DaemonCommands::Apps => services::apps::run(),
                DaemonCommands::Homebrew => services::homebrew::run(),
                DaemonCommands::Clipboard => services::clipboard::run(),
                DaemonCommands::Nixpkgs => services::nixpkgs::run(),
            }
        }
        None => {
            let result = run_gui(cli);
            cleanup_lock_file();
            ipc::cleanup();
            result
        }
    }
}

fn check_single_instance(cli: &Cli) -> Result<bool> {
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
                    // Another instance is running - send IPC message
                    let msg = ipc::IpcMessage::Reload {
                        apps: cli.apps,
                        homebrew: cli.homebrew,
                        clipboard: cli.clipboard,
                        commands: cli.commands,
                        nixpkgs: cli.nixpkgs,
                        sources: cli.source.iter().map(|p| p.display().to_string()).collect(),
                        prompt: cli.prompt.clone(),
                    };

                    if ipc::send_message(&msg).is_ok() {
                        return Ok(true);
                    }
                    // If socket doesn't exist remove stale lock and continue
                    let _ = fs::remove_file(&lock_file);
                }
            }
        }
        let _ = fs::remove_file(&lock_file);
    }

    let pid = std::process::id();
    fs::write(&lock_file, pid.to_string())?;

    *LOCK_FILE.lock().unwrap() = Some(lock_file);

    Ok(false)
}

fn cleanup_lock_file() {
    if let Some(path) = LOCK_FILE.lock().unwrap().take() {
        let _ = fs::remove_file(path);
    }
}

fn get_lock_file_path() -> Result<PathBuf> {
    let runtime_dir = PathBuf::from("/tmp");
    Ok(runtime_dir.join("frisk.lock"))
}

fn run_gui(cli: Cli) -> Result<()> {
    let sent_ipc = check_single_instance(&cli)?;
    if sent_ipc {
        crate::log!("Sent reload message to existing instance");
        return Ok(());
    }

    let ipc_rx = ipc::start_listener()?;

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

    let mut elements = core::element::ElementList::new();

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
        match core::commands::CommandsConfig::load() {
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

    if cli.nixpkgs {
        if let Some(nixpkgs) = load_binary_source("nixpkgs.bin")? {
            let count = nixpkgs.len();
            for pkg in nixpkgs {
                elements.add(pkg);
            }
            crate::log!("Loaded {} nixpkgs packages from nixpkgs.bin", count);
        } else {
            eprintln!("Warning: --nixpkgs specified but nixpkgs.bin not found");
            eprintln!("Run: frisk daemon nixpkgs");
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

    picker::run(config, elements, Some(ipc_rx))?;

    Ok(())
}
