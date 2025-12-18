use crate::analysis::count_lines;
use crate::config_reader::{CfgBlock, CfgCommentType, CfgLangEntry, Config, ConfigError};
use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Copy, Debug)]
struct LangId(usize);

#[derive(Debug)]
pub(crate) struct CommentType {
    pub(crate) line: Vec<String>,
    pub(crate) block: Option<Block>,
}

impl TryFrom<CfgCommentType> for CommentType {
    type Error = ConfigError;

    fn try_from(comment: CfgCommentType) -> Result<Self, Self::Error> {
        let line = match comment.line {
            Some(line) if !line.is_empty() => line,
            _ => return Err(ConfigError::LineCommentMissing),
        };

        let block = match comment.block {
            Some(block) => Some(block.try_into()?),
            _ => None,
        };
        Ok(Self { line, block })
    }
}

#[derive(Debug)]
pub(crate) struct Block {
    pub(crate) open: String,
    pub(crate) close: String,
}

impl TryFrom<CfgBlock> for Block {
    type Error = ConfigError;

    fn try_from(cfg_block: CfgBlock) -> Result<Self, Self::Error> {
        let open = match cfg_block.open {
            Some(open) if !open.is_empty() => open,
            _ => return Err(ConfigError::InvalidBlockComment),
        };
        let close = match cfg_block.close {
            Some(close) if !close.is_empty() => close,
            _ => return Err(ConfigError::InvalidBlockComment),
        };
        Ok(Self { open, close })
    }
}

#[derive(Debug)]
struct LangSpec {
    name: String,
    extensions: Vec<OsString>,
    comments: CommentType,
}

impl LangSpec {
    fn new(name: String, extensions: Vec<OsString>, comments: CommentType) -> Self {
        Self {
            name,
            extensions,
            comments,
        }
    }
}

#[derive(Debug)]
struct LangStats {
    files: HashSet<PathBuf>,
    loc: u64,
}

#[derive(Debug)]
pub(crate) struct LangEntry {
    spec: LangSpec,
    stats: LangStats,
}

impl TryFrom<CfgLangEntry> for LangEntry {
    type Error = ConfigError;

    fn try_from(cfg_lang: CfgLangEntry) -> Result<Self, Self::Error> {
        let name = match cfg_lang.name {
            Some(name) => name,
            _ => return Err(ConfigError::LanguageNameMissing),
        };
        let extensions = match cfg_lang.extensions {
            Some(extensions) if !extensions.is_empty() => {
                extensions.into_iter().map(OsString::from).collect()
            }
            _ => return Err(ConfigError::ExtensionMissing),
        };
        let comments = match cfg_lang.comments {
            Some(comments) => comments.try_into()?,
            _ => return Err(ConfigError::CommentsMissing),
        };
        let spec = LangSpec::new(name, extensions, comments);
        let stats = LangStats {
            files: HashSet::new(),
            loc: 0,
        };
        Ok(Self { spec, stats })
    }
}

#[derive(Debug)]
pub struct LangRegistry {
    dir: PathBuf,
    entries: Vec<LangEntry>,
    map_ext_id: HashMap<OsString, LangId>,
}

impl Default for LangRegistry {
    fn default() -> Self {
        LangRegistry::new()
    }
}

impl LangRegistry {
    fn add_entry(&mut self, spec: LangSpec, stats: LangStats) {
        for ext in spec.extensions.iter() {
            self.map_ext_id
                .insert(ext.clone(), LangId(self.entries.len()));
        }
        self.entries.push(LangEntry { spec, stats });
    }

    fn stats_mut(&mut self, id: LangId) -> &mut LangStats {
        &mut self.entries[id.0].stats
    }
    fn get_spec(&self, id: LangId) -> &LangSpec {
        &self.entries[id.0].spec
    }

    fn get_entry_id(&self, ext: &OsStr) -> Option<LangId> {
        self.map_ext_id.get(ext).copied()
    }
    fn clear_locs(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.stats.loc = 0;
        }
    }

    fn clear_paths(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.stats.files.clear();
        }
    }

    pub fn show_stats(&self) {
        println!("STATS for directory: {}", self.dir.display());
        for entry in &self.entries {
            println!(
                "{}, files: {} loc: {}",
                entry.spec.name,
                entry.stats.files.len(),
                entry.stats.loc
            );
        }
    }

    pub fn new() -> Self {
        Self {
            dir: PathBuf::new(),
            entries: Vec::new(),
            map_ext_id: HashMap::new(),
        }
    }
    pub fn with_config(cfg: Config) -> Result<Self, ConfigError> {
        let mut reg = LangRegistry::new();

        reg.dir = cfg.dir;
        for language in cfg.languages {
            let entry: LangEntry = language.try_into()?;
            reg.add_entry(entry.spec, entry.stats);
        }
        Ok(reg)
    }

    pub fn with_builtins_langs(dir: &Path) -> Self {
        let mut reg = LangRegistry::new();

        reg.dir = dir.to_path_buf();
        reg.add_entry(
            LangSpec::new(
                String::from("Rust"),
                vec![OsString::from("rs")],
                CommentType {
                    line: vec!["//".to_string(), "///".to_string(), "//!".to_string()],
                    block: Some(Block {
                        open: "/*".to_string(),
                        close: "*/".to_string(),
                    }),
                },
            ),
            LangStats {
                files: HashSet::new(),
                loc: 0,
            },
        );
        reg.add_entry(
            LangSpec::new(
                String::from("C"),
                vec![OsString::from("c"), OsString::from("h")],
                CommentType {
                    line: vec!["//".to_string()],
                    block: Some(Block {
                        open: "/*".to_string(),
                        close: "*/".to_string(),
                    }),
                },
            ),
            LangStats {
                files: HashSet::new(),
                loc: 0,
            },
        );
        reg
    }

    pub fn update_stats(&mut self) -> std::result::Result<(), std::io::Error> {
        self.clear_locs();
        self.clear_paths();

        for item in WalkDir::new(&self.dir).into_iter().flatten() {
            let path = item.into_path();
            if !path.is_file() {
                continue;
            }
            if let Some(id) = path.extension().and_then(|ext| self.get_entry_id(ext)) {
                let comments = &self.get_spec(id).comments;

                let loc = count_lines(&path, comments)?;
                let stats = self.stats_mut(id);
                stats.files.insert(path);
                stats.loc += loc;
            }
        }
        Ok(())
    }
}
