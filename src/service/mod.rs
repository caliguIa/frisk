use crate::cli::{parse_service_name, ServiceCommands};
use crate::error::{Error, Result};
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

const SERVICE_PREFIX: &str = "io.calrichards.frisk";

pub struct Service {
    name: String,
    bin_path: PathBuf,
}

impl Service {
    pub fn new(name: String) -> Result<Self> {
        let bin_path = env::current_exe()?;
        Ok(Self { name, bin_path })
    }

    fn label(&self) -> String {
        format!("{}.{}", SERVICE_PREFIX, &self.name)
    }

    fn plist_path(&self) -> PathBuf {
        let home = env::var("HOME").expect("HOME not set");
        PathBuf::from(format!(
            "{}/Library/LaunchAgents/{}.plist",
            home,
            self.label()
        ))
    }

    fn log_path(&self, kind: &str) -> PathBuf {
        let home = env::var("HOME").expect("HOME not set");
        PathBuf::from(format!(
            "{}/Library/Logs/frisk-{}.{}",
            home, &self.name, kind
        ))
    }

    pub fn is_installed(&self) -> bool {
        self.plist_path().exists()
    }

    pub fn install(&self) -> Result<()> {
        let plist_path = self.plist_path();

        if self.is_installed() {
            println!(
                "Service '{}' already installed at {}",
                &self.name,
                plist_path.display()
            );
            return Ok(());
        }

        // Create LaunchAgents directory if needed
        if let Some(parent) = plist_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Create Logs directory if needed
        if let Some(parent) = self.log_path("log").parent() {
            fs::create_dir_all(parent)?;
        }

        let plist_content = self.generate_plist();
        let mut file = fs::File::create(&plist_path)?;
        file.write_all(plist_content.as_bytes())?;

        println!(
            "Installed service '{}' to {}",
            &self.name,
            plist_path.display()
        );
        Ok(())
    }

    pub fn uninstall(&self) -> Result<()> {
        let plist_path = self.plist_path();

        if !self.is_installed() {
            println!(
                "Service '{}' not installed (no plist at {})",
                &self.name,
                plist_path.display()
            );
            return Ok(());
        }

        // Stop service first
        let _ = self.stop();

        fs::remove_file(&plist_path)?;
        println!(
            "Uninstalled service '{}' from {}",
            &self.name,
            plist_path.display()
        );
        Ok(())
    }

    pub fn start(&self) -> Result<()> {
        if !self.is_installed() {
            return Err(Error::new(format!(
                "Service '{}' not installed",
                &self.name
            )));
        }

        let output = Command::new("launchctl")
            .arg("load")
            .arg(self.plist_path())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already loaded") {
                println!("Service '{}' already running", &self.name);
                return Ok(());
            }
            return Err(Error::new(format!("Failed to start service: {}", stderr)));
        }

        println!("Started service '{}'", &self.name);
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        let output = Command::new("launchctl")
            .arg("unload")
            .arg(self.plist_path())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Could not find") {
                println!("Service '{}' not running", &self.name);
                return Ok(());
            }
            return Err(Error::new(format!("Failed to stop service: {}", stderr)));
        }

        println!("Stopped service '{}'", &self.name);
        Ok(())
    }

    fn generate_plist(&self) -> String {
        let daemon_args = match self.name.as_str() {
            "apps" => vec!["daemon".to_string(), "apps".to_string()],
            "homebrew" => vec!["daemon".to_string(), "homebrew".to_string()],
            "clipboard" => vec!["daemon".to_string(), "clipboard".to_string()],
            "nixpkgs" => vec!["daemon".to_string(), "nixpkgs".to_string()],
            _ => unreachable!("Invalid service name: {}", self.name),
        };

        let program_arguments = daemon_args
            .iter()
            .map(|arg| format!("        <string>{}</string>", arg))
            .collect::<Vec<_>>()
            .join("\n");

        let keep_alive = matches!(self.name.as_str(), "clipboard");
        // homebrew: hourly, nixpkgs: twice daily (12 hours)
        let start_interval = match self.name.as_str() {
            "homebrew" => Some(3600),        // 1 hour
            "nixpkgs" => Some(43200),        // 12 hours
            "apps" => Some(3600),             // 1 hour (changed from KeepAlive)
            _ => None,
        };

        let mut plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
{}
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <{}>
    <key>StandardOutPath</key>
    <string>{}</string>
    <key>StandardErrorPath</key>
    <string>{}</string>"#,
            self.label(),
            self.bin_path.display(),
            program_arguments,
            if keep_alive { "true/" } else { "false/" },
            self.log_path("log").display(),
            self.log_path("err").display(),
        );

        if let Some(interval) = start_interval {
            plist.push_str(&format!(
                r#"
    <key>StartInterval</key>
    <integer>{}</integer>"#,
                interval
            ));
        }

        plist.push_str(
            r#"
</dict>
</plist>
"#,
        );

        plist
    }
}

pub fn handle_service_command(cmd: ServiceCommands) -> Result<()> {
    match cmd {
        ServiceCommands::Install { name } => {
            let services = parse_service_name(&name)
                .ok_or_else(|| Error::new(format!("Unknown service name: {}", name)))?;
            
            for service_name in services {
                Service::new(service_name.to_string())?.install()?;
            }
        }
        ServiceCommands::Uninstall { name } => {
            let services = parse_service_name(&name)
                .ok_or_else(|| Error::new(format!("Unknown service name: {}", name)))?;
            
            for service_name in services {
                Service::new(service_name.to_string())?.uninstall()?;
            }
        }
        ServiceCommands::Start { name } => {
            let services = parse_service_name(&name)
                .ok_or_else(|| Error::new(format!("Unknown service name: {}", name)))?;
            
            for service_name in services {
                Service::new(service_name.to_string())?.start()?;
            }
        }
        ServiceCommands::Stop { name } => {
            let services = parse_service_name(&name)
                .ok_or_else(|| Error::new(format!("Unknown service name: {}", name)))?;
            
            for service_name in services {
                Service::new(service_name.to_string())?.stop()?;
            }
        }
        ServiceCommands::Status => {
            show_status()?;
        }
        ServiceCommands::List => {
            list_services();
        }
    }
    Ok(())
}

fn show_status() -> Result<()> {
    println!("Frisk Services Status:");
    println!();

    let all_services = vec!["apps", "homebrew", "clipboard", "nixpkgs"];
    
    for service_name in all_services {
        let service = Service::new(service_name.to_string())?;
        let installed = service.is_installed();
        let running = if installed {
            is_service_running(&service.label())
        } else {
            false
        };

        let status = if running {
            "●  running"
        } else if installed {
            "○  installed (not running)"
        } else {
            "✗  not installed"
        };

        println!("  {} - {}", &service.name, status);
    }

    Ok(())
}

fn is_service_running(label: &str) -> bool {
    let output = Command::new("launchctl")
        .arg("list")
        .arg(label)
        .output();

    match output {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

fn list_services() {
    println!("Available services:");
    println!();
    println!("  apps       - Refresh application cache hourly");
    println!("  homebrew   - Fetch homebrew packages hourly");
    println!("  clipboard  - Monitor clipboard for history (persistent)");
    println!("  nixpkgs    - Fetch nixpkgs packages twice daily");
    println!();
    println!("Use 'all' to operate on all services at once");
}
