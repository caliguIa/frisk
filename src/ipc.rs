use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// Messages that can be sent between frisk instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcMessage {
    /// Reload the UI with new sources
    Reload {
        apps: bool,
        homebrew: bool,
        clipboard: bool,
        commands: bool,
        sources: Vec<String>,
    },
}

/// Get the path to the IPC socket
pub fn socket_path() -> Result<PathBuf> {
    let runtime_dir = if let Ok(dir) = std::env::var("TMPDIR") {
        PathBuf::from(dir)
    } else {
        PathBuf::from("/tmp")
    };
    Ok(runtime_dir.join("frisk.sock"))
}

/// Start listening for IPC messages in a background thread
/// Returns a receiver that will get messages from other instances
pub fn start_listener() -> Result<Receiver<IpcMessage>> {
    let socket_path = socket_path()?;

    // Remove old socket if it exists
    if socket_path.exists() {
        fs::remove_file(&socket_path)?;
    }

    let listener = UnixListener::bind(&socket_path)
        .map_err(|e| Error::new(format!("Failed to bind IPC socket: {}", e)))?;

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        crate::log!("IPC listener started");
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = handle_connection(stream, tx.clone()) {
                        crate::log!("IPC connection error: {}", e);
                    }
                }
                Err(e) => {
                    crate::log!("IPC accept error: {}", e);
                }
            }
        }
    });

    Ok(rx)
}

/// Handle an incoming IPC connection
fn handle_connection(stream: UnixStream, tx: Sender<IpcMessage>) -> Result<()> {
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<IpcMessage>(&line) {
            Ok(msg) => {
                crate::log!("Received IPC message: {:?}", msg);
                tx.send(msg).ok();
            }
            Err(e) => {
                crate::log!("Failed to parse IPC message: {}", e);
            }
        }
    }
    Ok(())
}

/// Send a message to the running frisk instance
pub fn send_message(msg: &IpcMessage) -> Result<()> {
    let socket_path = socket_path()?;

    if !socket_path.exists() {
        return Err(Error::new(
            "No running frisk instance found (socket doesn't exist)",
        ));
    }

    let mut stream = UnixStream::connect(&socket_path)
        .map_err(|e| Error::new(format!("Failed to connect to IPC socket: {}", e)))?;

    let json = serde_json::to_string(msg)?;
    writeln!(stream, "{}", json)?;
    stream.flush()?;

    crate::log!("Sent IPC message: {:?}", msg);

    Ok(())
}

/// Clean up the IPC socket on exit
pub fn cleanup() {
    if let Ok(socket_path) = socket_path() {
        let _ = fs::remove_file(socket_path);
    }
}
