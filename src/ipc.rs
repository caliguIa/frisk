use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcMessage {
    Reload {
        apps: bool,
        homebrew: bool,
        clipboard: bool,
        commands: bool,
        sources: Vec<String>,
    },
    Search {
        query: String,
        source: SearchSource,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchSource {
    Nixpkgs,
}

pub fn socket_path() -> Result<PathBuf> {
    let runtime_dir = PathBuf::from("/tmp");
    Ok(runtime_dir.join("frisk.sock"))
}

pub fn start_listener() -> Result<Receiver<IpcMessage>> {
    let socket_path = socket_path()?;

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

pub fn send_message(msg: &IpcMessage) -> Result<()> {
    send_message_to_socket(&socket_path()?, msg)
}

fn send_message_to_socket(socket_path: &PathBuf, msg: &IpcMessage) -> Result<()> {
    if !socket_path.exists() {
        return Err(Error::new("Socket doesn't exist"));
    }

    let mut stream = UnixStream::connect(socket_path)
        .map_err(|e| Error::new(format!("Failed to connect to IPC socket: {}", e)))?;

    let json = serde_json::to_string(msg)?;
    writeln!(stream, "{}", json)?;
    stream.flush()?;

    crate::log!("Sent IPC message: {:?}", msg);

    Ok(())
}

pub fn cleanup() {
    if let Ok(socket_path) = socket_path() {
        let _ = fs::remove_file(socket_path);
    }
}
