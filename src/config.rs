use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub prompt: String,
    pub font_family: String,
    pub font_size: f32,
    pub styles: StyleConfig,
    pub spacing: SpacingConfig,
    pub keymap: KeymapConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct StyleConfig {
    pub background: String,
    pub items: String,
    pub selected_item: String,
    pub prompt: String,
    pub query: String,
    pub caret: String,
    pub window_opacity: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct SpacingConfig {
    pub window_padding_x: f32,
    pub window_padding_y: f32,
    pub prompt_to_items: f32,
    pub item_spacing: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct KeymapConfig {
    pub exit: Vec<String>,
    pub execute: Vec<String>,
    pub nav_up: Vec<String>,
    pub nav_down: Vec<String>,
    pub autocomplete: Vec<String>,
}

impl Default for KeymapConfig {
    fn default() -> Self {
        Self {
            exit: vec!["Escape".to_string()],
            execute: vec!["Enter".to_string()],
            nav_up: vec!["Up".to_string(), "ctrl+p".to_string()],
            nav_down: vec!["Down".to_string(), "ctrl+n".to_string()],
            autocomplete: vec!["Tab".to_string()],
        }
    }
}

/// Represents a key binding parsed from config
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub key_code: u16,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub cmd: bool,
}

impl KeyBinding {
    /// Parse a key binding string like "ctrl+w" or "Backspace"
    pub fn parse(binding: &str) -> Result<Self> {
        let parts: Vec<&str> = binding.split('+').map(|s| s.trim()).collect();
        
        let mut ctrl = false;
        let mut shift = false;
        let mut alt = false;
        let mut cmd = false;
        let mut key_name = "";
        
        for part in &parts {
            match part.to_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "shift" => shift = true,
                "alt" | "option" => alt = true,
                "cmd" | "command" => cmd = true,
                _ => key_name = part,
            }
        }
        
        let key_code = match key_name.to_lowercase().as_str() {
            "escape" | "esc" => 53,
            "enter" | "return" => 36,
            "tab" => 48,
            "backspace" | "delete" => 51,
            "left" => 123,
            "right" => 124,
            "down" => 125,
            "up" => 126,
            "space" => 49,
            // Letter keys (QWERTY layout)
            "a" => 0, "b" => 11, "c" => 8, "d" => 2, "e" => 14,
            "f" => 3, "g" => 5, "h" => 4, "i" => 34, "j" => 38,
            "k" => 40, "l" => 37, "m" => 46, "n" => 45, "o" => 31,
            "p" => 35, "q" => 12, "r" => 15, "s" => 1, "t" => 17,
            "u" => 32, "v" => 9, "w" => 13, "x" => 7, "y" => 16,
            "z" => 6,
            // Number keys
            "0" => 29, "1" => 18, "2" => 19, "3" => 20, "4" => 21,
            "5" => 23, "6" => 22, "7" => 26, "8" => 28, "9" => 25,
            _ => return Err(anyhow::anyhow!("Unknown key: {}", key_name)),
        };
        
        Ok(KeyBinding {
            key_code,
            ctrl,
            shift,
            alt,
            cmd,
        })
    }
    
    /// Check if this binding matches the given key event
    pub fn matches(&self, key_code: u16, ctrl: bool, shift: bool, alt: bool, cmd: bool) -> bool {
        self.key_code == key_code
            && self.ctrl == ctrl
            && self.shift == shift
            && self.alt == alt
            && self.cmd == cmd
    }
}



#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// Parse a hex color string into a Color (RGB only, no alpha)
    /// Supports formats: #RGB, #RRGGBB
    pub fn from_hex(hex: &str) -> Result<Self> {
        let hex = hex.trim_start_matches('#');
        
        match hex.len() {
            3 => {
                // #RGB -> #RRGGBB
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16)?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16)?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16)?;
                Ok(Self::rgb(r, g, b))
            }
            6 => {
                // #RRGGBB
                let r = u8::from_str_radix(&hex[0..2], 16)?;
                let g = u8::from_str_radix(&hex[2..4], 16)?;
                let b = u8::from_str_radix(&hex[4..6], 16)?;
                Ok(Self::rgb(r, g, b))
            }
            _ => Err(anyhow::anyhow!("Invalid hex color format: #{}", hex)),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prompt: "Run: ".to_string(),
            font_family: "Berkeley Mono".to_string(),
            font_size: 32.0,
            styles: StyleConfig::default(),
            spacing: SpacingConfig::default(),
            keymap: KeymapConfig::default(),
        }
    }
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            background: "#282c34".to_string(), // Dark background (no alpha)
            items: "#ffffff".to_string(), // White text for items
            selected_item: "#61afef".to_string(), // Blue for selected item
            prompt: "#98c379".to_string(), // Green for prompt
            query: "#e06c75".to_string(), // Red for query text
            caret: "#e06c75".to_string(), // Red for caret
            window_opacity: 0.85, // Default 85% opacity
        }
    }
}

impl Default for SpacingConfig {
    fn default() -> Self {
        Self {
            window_padding_x: 20.0,
            window_padding_y: 20.0,
            prompt_to_items: 60.0,
            item_spacing: 15.0,
        }
    }
}



impl Config {
    pub fn load(config_path: Option<PathBuf>) -> Result<Self> {
        let path = match config_path {
            Some(path) => path,
            None => {
                let home = std::env::var("HOME").context("HOME environment variable not set")?;
                PathBuf::from(home)
                    .join(".config")
                    .join("kickoff-macos")
                    .join("config.toml")
            }
        };

        if path.exists() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config file: {}", path.display()))?;
            
            let config: Config = toml::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
            
            Ok(config)
        } else {
            // Create default config
            let config = Config::default();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
            }
            
            let toml = toml::to_string_pretty(&config)
                .context("Failed to serialize default config")?;
            
            fs::write(&path, toml)
                .with_context(|| format!("Failed to write default config to: {}", path.display()))?;
            
            Ok(config)
        }
    }

    // Helper methods to convert hex strings to Color objects
    pub fn background_color(&self) -> Result<Color> {
        let mut color = Color::from_hex(&self.styles.background)?;
        color.a = self.styles.window_opacity.max(0.0).min(1.0); // Clamp to 0.0-1.0
        Ok(color)
    }

    pub fn items_color(&self) -> Result<Color> {
        Color::from_hex(&self.styles.items)
    }

    pub fn selected_item_color(&self) -> Result<Color> {
        Color::from_hex(&self.styles.selected_item)
    }

    pub fn prompt_color(&self) -> Result<Color> {
        Color::from_hex(&self.styles.prompt)
    }

    pub fn query_color(&self) -> Result<Color> {
        Color::from_hex(&self.styles.query)
    }

    pub fn caret_color(&self) -> Result<Color> {
        Color::from_hex(&self.styles.caret)
    }

    pub fn selection_background_color(&self) -> Result<Color> {
        // Create a semi-transparent version of the selected item color for background
        let mut color = Color::from_hex(&self.styles.selected_item)?;
        color.a = 0.3; // 30% opacity for selection background
        Ok(color)
    }
    
    /// Check if a key event matches any binding for the given action
    pub fn check_binding(&self, action_bindings: &[String], key_code: u16, ctrl: bool, shift: bool, alt: bool, cmd: bool) -> bool {
        for binding_str in action_bindings {
            if let Ok(binding) = KeyBinding::parse(binding_str) {
                if binding.matches(key_code, ctrl, shift, alt, cmd) {
                    return true;
                }
            }
        }
        false
    }
}