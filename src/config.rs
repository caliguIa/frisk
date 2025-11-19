use crate::error::{Error, Result};
use objc2::rc::Retained;
use objc2_app_kit::{NSColor, NSFont};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
struct RawConfig {
    prompt: String,
    font_family: String,
    font_size: u8,
    window_opacity: f32,
    window_padding: u8,
    prompt_to_items: u8,
    item_spacing: u8,
    background: String,
    items: String,
    selected_item: String,
    query: String,
    caret: String,
}

impl Default for RawConfig {
    fn default() -> Self {
        Self {
            prompt: "Run: ".into(),
            font_family: "Berkeley Mono".into(),
            font_size: 32,
            window_opacity: 0.55,
            window_padding: 20,
            prompt_to_items: 60,
            item_spacing: 15,
            background: "#282c34".into(),
            items: "#ffffff".into(),
            selected_item: "#61afef".into(),
            query: "#e06c75".into(),
            caret: "#e06c75".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub prompt: String,
    pub font_size: u8,
    pub window_padding: u8,
    pub prompt_to_items: u8,
    pub item_spacing: u8,
    pub font: Retained<NSFont>,
    pub background_color: Retained<NSColor>,
    pub items_color: Retained<NSColor>,
    pub selected_item_color: Retained<NSColor>,
    pub query_color: Retained<NSColor>,
    pub caret_color: Retained<NSColor>,
}

impl Config {
    pub fn load(config_path: Option<PathBuf>) -> Result<Self> {
        let path = match config_path {
            Some(path) => path,
            None => PathBuf::from(
                std::env::var("XDG_CONFIG_HOME")
                    .or_else(|_| std::env::var("HOME").map(|home| format!("{}/.cache", home)))
                    .map_err(|_| {
                        Error::new("Neither $XDG_CONFIG_HOME or $HOME variables are set.")
                    })?,
            )
            .join("frisk")
            .join("config.toml"),
        };

        let raw = if path.exists() {
            let content = fs::read_to_string(&path)?;
            toml::from_str(&content)?
        } else {
            RawConfig::default()
        };

        Self::from_raw(raw)
    }

    fn from_raw(raw: RawConfig) -> Result<Self> {
        let font = Self::create_font(&raw.font_family, raw.font_size);
        let background_color =
            Self::parse_color_with_alpha(&raw.background, 40, 44, 52, raw.window_opacity)?;
        let items_color = Self::parse_color(&raw.items, 255, 255, 255)?;
        let selected_item_color = Self::parse_color(&raw.selected_item, 97, 175, 239)?;
        let query_color = Self::parse_color(&raw.query, 224, 108, 117)?;
        let caret_color = Self::parse_color(&raw.caret, 224, 108, 117)?;

        Ok(Self {
            prompt: raw.prompt,
            font_size: raw.font_size,
            window_padding: raw.window_padding,
            prompt_to_items: raw.prompt_to_items,
            item_spacing: raw.item_spacing,
            font,
            background_color,
            items_color,
            selected_item_color,
            query_color,
            caret_color,
        })
    }

    fn create_font(font_family: &str, font_size: u8) -> Retained<NSFont> {
        if font_family.is_empty() || font_family == "system" {
            return NSFont::systemFontOfSize(font_size as f64);
        }

        use objc2_foundation::NSString;
        let font_name_ns = NSString::from_str(font_family);
        NSFont::fontWithName_size(&font_name_ns, font_size as f64)
            .unwrap_or_else(|| NSFont::systemFontOfSize(font_size as f64))
    }

    fn parse_color(
        hex: &str,
        fallback_r: u8,
        fallback_g: u8,
        fallback_b: u8,
    ) -> Result<Retained<NSColor>> {
        let hex = hex.trim_start_matches('#');

        let (r, g, b) = match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16)?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16)?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16)?;
                (r, g, b)
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16)?;
                let g = u8::from_str_radix(&hex[2..4], 16)?;
                let b = u8::from_str_radix(&hex[4..6], 16)?;
                (r, g, b)
            }
            _ => (fallback_r, fallback_g, fallback_b),
        };

        Ok(NSColor::colorWithSRGBRed_green_blue_alpha(
            r as f64 / 255.0,
            g as f64 / 255.0,
            b as f64 / 255.0,
            1.0,
        ))
    }

    fn parse_color_with_alpha(
        hex: &str,
        fallback_r: u8,
        fallback_g: u8,
        fallback_b: u8,
        alpha: f32,
    ) -> Result<Retained<NSColor>> {
        let hex = hex.trim_start_matches('#');

        let (r, g, b) = match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16)?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16)?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16)?;
                (r, g, b)
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16)?;
                let g = u8::from_str_radix(&hex[2..4], 16)?;
                let b = u8::from_str_radix(&hex[4..6], 16)?;
                (r, g, b)
            }
            _ => (fallback_r, fallback_g, fallback_b),
        };

        Ok(NSColor::colorWithSRGBRed_green_blue_alpha(
            r as f64 / 255.0,
            g as f64 / 255.0,
            b as f64 / 255.0,
            alpha as f64,
        ))
    }
}
