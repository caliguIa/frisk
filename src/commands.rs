use crate::element::Element;
use crate::error::Result;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct CommandsConfig {
    #[serde(default)]
    pub command: Vec<CustomCommand>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CustomCommand {
    pub name: String,
    pub action: String,
}

impl CommandsConfig {
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            let default_config = Self::default_config();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, default_config)?;
            eprintln!("Created default commands config at: {}", path.display());
        }

        let content = fs::read_to_string(&path)?;
        let config: CommandsConfig = toml::from_str(&content)?;
        Ok(config)
    }

    fn config_path() -> Result<PathBuf> {
        let mut path = if let Ok(home) = env::var("HOME") {
            PathBuf::from(home)
        } else {
            PathBuf::from(".")
        };
        path.push(".config");
        path.push("frisk");
        path.push("commands.toml");
        Ok(path)
    }

    fn default_config() -> String {
        r#"# Custom commands for frisk
# Add your own commands here

[[command]]
name = "Empty Trash"
action = "osascript -e 'tell application \"Finder\" to empty trash'"

[[command]]
name = "Show Trash"
action = "osascript -e 'tell application \"Finder\" to open trash' && open -a finder"

[[command]]
name = "Restart"
action = "osascript -e 'tell application \"System Events\" to restart'"

[[command]]
name = "Shut Down"
action = "osascript -e 'tell application \"System Events\" to shut down'"

[[command]]
name = "Sleep"
action = "osascript -e 'tell application \"System Events\" to sleep'"

[[command]]
name = "Lock Screen"
action = "pmset displaysleepnow"
"#
        .to_string()
    }

    pub fn to_elements(&self) -> Vec<Element> {
        self.command
            .iter()
            .map(|cmd| Element::new_system_command(cmd.name.clone(), cmd.action.clone()))
            .collect()
    }
}
