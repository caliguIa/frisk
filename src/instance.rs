use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::cli::Cli;
use crate::core::error::Result;
use crate::ipc;

static LOCK_FILE: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn check_single_instance(cli: &Cli) -> Result<bool> {
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

pub fn cleanup_lock_file() {
    if let Some(path) = LOCK_FILE.lock().unwrap().take() {
        let _ = fs::remove_file(path);
    }
}

fn get_lock_file_path() -> Result<PathBuf> {
    let runtime_dir = PathBuf::from("/tmp");
    Ok(runtime_dir.join("frisk.lock"))
}
