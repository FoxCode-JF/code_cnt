use serde::{self, Deserialize};
use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Invalid Config: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid Config: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("Directory path missing")]
    DirectoryPathMissing,
    #[error("Languages list missing")]
    LanguagesMissing,
    #[error("Language name missing")]
    LanguageNameMissing,
    #[error("Comment not defined")]
    CommentsMissing,
    #[error("Block comment not defined")]
    BlockCommentMissing,
    #[error("Invalid Block comment")]
    InvalidBlockComment,
    #[error("Line comment not defined")]
    LineCommentMissing,
    #[error("Invalid line comment")]
    InvalidLineComment,
    #[error("File extension not defined")]
    ExtensionMissing,
    #[error("Invalid file extension")]
    InvalidExtension,
}

//impl From<std::io::Error> for ConfigError {
//    fn from(value: std::io::Error) -> Self {
//        ConfigError::Io(value)
//    }
//}
//impl From<toml::de::Error> for ConfigError {
//    fn from(value: toml::de::Error) -> Self {
//        ConfigError::Toml(value)
//    }
//}

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
    pub(crate) languages: Vec<CfgLangEntry>,
}

impl Config {
    pub fn load(cfg_path: &str) -> Result<Config, ConfigError> {
        let text = std::fs::read_to_string(cfg_path)?;
        let cfg: Config = toml::from_str(&text)?;
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
