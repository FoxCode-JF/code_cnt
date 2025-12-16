use std::path::PathBuf;

use serde::{self, Deserialize};

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    DirectoryPathMissing,
    LanguagesMissing,
    LanguageNameMissing,
    CommentsMissing,
    BlockCommentMissing,
    InvalidBlockComment,
    LineCommentMissing,
    InvalidLineComment,
    ExtensionMissing,
    InvalidExtension,
}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        ConfigError::Io(value)
    }
}
impl From<toml::de::Error> for ConfigError {
    fn from(value: toml::de::Error) -> Self {
        ConfigError::Toml(value)
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct CfgBlock {
    pub(crate) open: Option<String>,
    pub(crate) close: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CfgCommentType {
    pub(crate) line: Option<Vec<String>>,
    pub(crate) block: Option<CfgBlock>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CfgLangEntry {
    pub(crate) name: Option<String>,
    pub(crate) extensions: Option<Vec<String>>,
    pub(crate) comments: Option<CfgCommentType>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub dir: PathBuf,
    languages: Vec<CfgLangEntry>,
}

impl Config {
    pub fn load(cfg_path: &str) -> Result<Config, ConfigError> {
        let text = std::fs::read_to_string(cfg_path)?;
        println!("\tTEXT: {}", text);
        let cfg: Config = toml::from_str(&text)?;
        //println!(" PATH: {}", cfg.dir.clone().unwrap().display());
        cfg.validate_dir()?;
        cfg.validate_languages()?;
        Ok(cfg)
    }
    fn validate_dir(&self) -> std::io::Result<()> {
        let metadata = std::fs::metadata(&self.dir)?;
        if metadata.is_dir() {
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "path is not a directory",
            ))
        }
    }
    fn validate_languages(&self) -> Result<(), ConfigError> {
        if self.languages.is_empty() {
            return Err(ConfigError::LanguagesMissing);
        }
        Ok(())
    }
}
