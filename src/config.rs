use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub prompt: String,
    pub font_family: String,
    pub font_size: u8,
    pub styles: StyleConfig,
    pub spacing: SpacingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct StyleConfig {
    pub background: String,
    pub items: String,
    pub selected_item: String,
    pub query: String,
    pub caret: String,
    pub window_opacity: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct SpacingConfig {
    pub window_padding: u8,
    pub prompt_to_items: u8,
    pub item_spacing: u8,
}

#[derive(Debug, Clone)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    pub fn from_hex(hex: &str) -> Result<Self> {
        let hex = hex.trim_start_matches('#');
        
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16)?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16)?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16)?;
                Ok(Self::rgb(r, g, b))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16)?;
                let g = u8::from_str_radix(&hex[2..4], 16)?;
                let b = u8::from_str_radix(&hex[4..6], 16)?;
                Ok(Self::rgb(r, g, b))
            }
            _ => Err(anyhow::anyhow!("Invalid hex color")),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prompt: "Run: ".to_string(),
            font_family: "Berkeley Mono".to_string(),
            font_size: 32,
            styles: StyleConfig::default(),
            spacing: SpacingConfig::default(),
        }
    }
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            background: "#282c34".to_string(),
            items: "#ffffff".to_string(),
            selected_item: "#61afef".to_string(),
            query: "#e06c75".to_string(),
            caret: "#e06c75".to_string(),
            window_opacity: 85,
        }
    }
}

impl Default for SpacingConfig {
    fn default() -> Self {
        Self {
            window_padding: 20,
            prompt_to_items: 60,
            item_spacing: 15,
        }
    }
}

impl Config {
    pub fn load(config_path: Option<PathBuf>) -> Result<Self> {
        let path = match config_path {
            Some(path) => path,
            None => {
                let home = std::env::var("HOME").context("HOME not set")?;
                PathBuf::from(home)
                    .join(".config")
                    .join("kickoff-macos")
                    .join("config.toml")
            }
        };

        if path.exists() {
            let content = fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            let config = Config::default();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, toml::to_string_pretty(&config)?)?;
            Ok(config)
        }
    }

    pub fn background_color(&self) -> Color {
        let mut color = Color::from_hex(&self.styles.background).unwrap_or_else(|_| Color::rgb(40, 44, 52));
        color.a = (self.styles.window_opacity as f32 / 100.0).clamp(0.0, 1.0);
        color
    }

    pub fn items_color(&self) -> Color {
        Color::from_hex(&self.styles.items).unwrap_or_else(|_| Color::rgb(255, 255, 255))
    }

    pub fn selected_item_color(&self) -> Color {
        Color::from_hex(&self.styles.selected_item).unwrap_or_else(|_| Color::rgb(97, 175, 239))
    }

    pub fn query_color(&self) -> Color {
        Color::from_hex(&self.styles.query).unwrap_or_else(|_| Color::rgb(224, 108, 117))
    }

    pub fn caret_color(&self) -> Color {
        Color::from_hex(&self.styles.caret).unwrap_or_else(|_| Color::rgb(224, 108, 117))
    }
}
