use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub default_page_attributes: PageAttributes,

    #[serde(default = "default_pages")]
    pub pages: HashMap<String, Page>,

    #[serde(default)]
    pub env_files: EnvFiles,

    #[serde(skip)]
    pub env: HashMap<String, String>,

    #[serde(default)]
    pub pre_hook: Vec<String>,

    #[serde(default = "default_output_dir")]
    pub output_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_page_attributes: PageAttributes::default(),
            pages: default_pages(),
            env_files: EnvFiles::default(),
            env: HashMap::new(),
            pre_hook: Vec::new(),
            output_dir: default_output_dir(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageAttributes {
    #[serde(default = "default_title")]
    pub title: String,

    #[serde(default = "default_favicon")]
    pub favicon: String,

    #[serde(default = "default_author")]
    pub author: String,

    #[serde(default = "default_description")]
    pub description: String,

    #[serde(default)]
    pub scripts: Vec<String>,
}

impl Default for PageAttributes {
    fn default() -> Self {
        Self {
            title: default_title(),
            favicon: default_favicon(),
            author: default_author(),
            description: default_description(),
            scripts: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Page {
    #[serde(flatten)]
    pub attributes: PageAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvFiles {
    #[serde(default)]
    pub dev: Option<String>,
    #[serde(default)]
    pub prd: Option<String>,
}

fn default_title() -> String {
    "Bageri App".to_string()
}

fn default_author() -> String {
    "Your Name".to_string()
}

fn default_description() -> String {
    "Description".to_string()
}

fn default_favicon() -> String {
    "favicon.ico".to_string()
}

fn default_pages() -> HashMap<String, Page> {
    let mut pages = HashMap::new();
    pages.insert(
        "index".to_string(),
        Page {
            attributes: PageAttributes::default(),
        },
    );
    pages
}

fn default_output_dir() -> String {
    "dist".to_string()
}

impl Config {
    pub async fn load() -> Result<Self> {
        Self::load_from("bageri.json5").await
    }

    pub async fn load_from<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;

        let mut config: Config = serde_json5::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.as_ref().display()))?;

        config.env = HashMap::new();

        // Load environment variables from specified env files
        let env_file = match std::env::var("NODE_ENV").unwrap_or_default().as_str() {
            "production" => config.env_files.prd.as_deref().unwrap_or(".env.prd"),
            _ => config.env_files.dev.as_deref().unwrap_or(".env"),
        };

        if let Ok(env_content) = fs::read_to_string(env_file).await {
            for line in env_content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim().to_string();
                    let value = value.trim().trim_matches('"').to_string();
                    config.env.insert(key, value);
                }
            }
        }

        Ok(config)
    }
}
