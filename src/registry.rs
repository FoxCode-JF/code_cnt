use crate::analysis::count_lines;
use crate::config_reader::{CfgBlock, CfgCommentType, CfgLangEntry, Config, ConfigError};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fmt::{self};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
enum LangRegistryError {
    LangEntryDuplicated { id: LangId, ext: OsString },
}

impl fmt::Display for LangRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LangRegistryError::LangEntryDuplicated { id, ext } => {
                write!(
                    f,
                    "Entry already exists (id = {:?}, ext {}), skipping...",
                    id,
                    ext.display()
                )
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct LangId(usize);

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
struct LangStats {
    files: HashSet<PathBuf>,
    loc: u64,
}

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
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
    fn add_entry(&mut self, spec: LangSpec, stats: LangStats) -> Result<(), LangRegistryError> {
        for ext in spec.extensions.iter() {
            let id = LangId(self.entries.len());
            match self.map_ext_id.entry(ext.clone()) {
                Entry::Vacant(v) => {
                    v.insert(id);
                }
                Entry::Occupied(e) => {
                    return Err(LangRegistryError::LangEntryDuplicated {
                        id: *e.get(),
                        ext: e.key().clone(),
                    })
                }
            }
            self.map_ext_id
                .insert(ext.clone(), LangId(self.entries.len()));
        }
        self.entries.push(LangEntry { spec, stats });
        Ok(())
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
            match reg.add_entry(entry.spec, entry.stats) {
                Ok(_) => { /* do nothing */ }
                Err(e) => {
                    println!("{}", e)
                }
            };
        }
        Ok(reg)
    }

    pub fn with_builtins_langs(dir: &Path) -> Self {
        let mut reg = LangRegistry::new();

        reg.dir = dir.to_path_buf();
        match reg.add_entry(
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
        ) {
            Ok(_) => { /* do nothing */ }
            Err(e) => {
                println!("{}", e)
            }
        };
        match reg.add_entry(
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
        ) {
            Ok(_) => { /* do nothing */ }
            Err(e) => {
                println!("{}", e)
            }
        };
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

#[cfg(test)]
mod tests {
    mod config_to_registry_types_mapping {
        use crate::{
            config_reader::{CfgBlock, CfgCommentType, CfgLangEntry, ConfigError},
            registry::{Block, CommentType, LangEntry, LangSpec, LangStats},
        };
        use std::collections::HashSet;
        use std::ffi::OsString;

        #[test]
        fn try_from_cfg_block_to_block_invalid_open() {
            let cfg_block = CfgBlock {
                open: Some("".to_string()),
                close: Some("*/".to_string()),
            };

            let res: Result<Block, ConfigError> = cfg_block.try_into();
            let err = res.unwrap_err();
            assert!(matches!(err, ConfigError::InvalidBlockComment));
        }
        #[test]
        fn try_from_cfg_block_to_block_invalid_close() {
            let cfg_block = CfgBlock {
                open: Some("/*".to_string()),
                close: Some("".to_string()),
            };

            let res: Result<Block, ConfigError> = cfg_block.try_into();
            let err = res.unwrap_err();
            assert!(matches!(err, ConfigError::InvalidBlockComment));
        }
        #[test]
        fn try_from_cfg_block_to_block_conversion_ok() {
            let cfg_block = CfgBlock {
                open: Some("/*".to_string()),
                close: Some("*/".to_string()),
            };

            let res: Block = cfg_block.try_into().unwrap();
            assert_eq!(
                res,
                Block {
                    open: "/*".to_string(),
                    close: "*/".to_string()
                }
            );
        }
        #[test]
        fn try_from_cfg_comment_type_to_comment_type_line_comment_missing() {
            let cfg_comment = CfgCommentType {
                line: None,
                block: Some(CfgBlock {
                    open: Some("/*".to_string()),
                    close: Some("*/".to_string()),
                }),
            };

            let res: Result<CommentType, ConfigError> = cfg_comment.try_into();
            let err = res.unwrap_err();
            assert!(matches!(err, ConfigError::LineCommentMissing));
        }
        #[test]
        fn try_from_cfg_comment_type_to_comment_type_block_comment_missing() {
            let cfg_comment = CfgCommentType {
                line: Some(vec!["//".to_string(), "///".to_string(), "//!".to_string()]),
                block: None,
            };

            let res: CommentType = cfg_comment.try_into().unwrap();
            assert!(res.block.is_none());
        }
        #[test]
        fn try_from_cfg_lang_entry_to_lang_entry_language_name_missing() {
            let cfg_lang_entry = CfgLangEntry {
                name: None,
                extensions: Some(vec![String::from("rs")]),
                comments: Some(CfgCommentType {
                    line: Some(vec!["//".to_string()]),
                    block: Some(CfgBlock {
                        open: Some("/*".to_string()),
                        close: Some("*/".to_string()),
                    }),
                }),
            };

            let res: Result<LangEntry, ConfigError> = cfg_lang_entry.try_into();
            let err = res.unwrap_err();
            assert!(matches!(err, ConfigError::LanguageNameMissing));
        }
        #[test]
        fn try_from_cfg_lang_entry_to_lang_entry_extension_missing() {
            let cfg_lang_entry = CfgLangEntry {
                name: Some("Rust".to_string()),
                extensions: None,
                comments: Some(CfgCommentType {
                    line: Some(vec!["//".to_string()]),
                    block: Some(CfgBlock {
                        open: Some("/*".to_string()),
                        close: Some("*/".to_string()),
                    }),
                }),
            };

            let res: Result<LangEntry, ConfigError> = cfg_lang_entry.try_into();
            let err = res.unwrap_err();
            assert!(matches!(err, ConfigError::ExtensionMissing));
        }
        #[test]
        fn try_from_cfg_lang_entry_to_lang_entry_comments_missing() {
            let cfg_lang_entry = CfgLangEntry {
                name: Some("Rust".to_string()),
                extensions: Some(vec![String::from("rs")]),
                comments: None,
            };

            let res: Result<LangEntry, ConfigError> = cfg_lang_entry.try_into();
            let err = res.unwrap_err();
            assert!(matches!(err, ConfigError::CommentsMissing));
        }
        #[test]
        fn try_from_cfg_lang_entry_to_lang_entry_conversion_ok() {
            let cfg_lang_entry = CfgLangEntry {
                name: Some("Rust".to_string()),
                extensions: Some(vec![String::from("rs")]),
                comments: Some(CfgCommentType {
                    line: Some(vec!["//".to_string()]),
                    block: Some(CfgBlock {
                        open: Some("/*".to_string()),
                        close: Some("*/".to_string()),
                    }),
                }),
            };

            let res: LangEntry = cfg_lang_entry.try_into().unwrap();
            assert_eq!(
                res,
                LangEntry {
                    spec: LangSpec {
                        name: "Rust".to_string(),
                        extensions: vec![OsString::from("rs")],
                        comments: CommentType {
                            line: vec!["//".to_string()],
                            block: Some(Block {
                                open: "/*".to_string(),
                                close: "*/".to_string()
                            })
                        }
                    },
                    stats: LangStats {
                        files: HashSet::new(),
                        loc: 0,
                    }
                }
            );
        }
    }
    mod lang_registry {
        use std::collections::HashMap;
        use std::collections::HashSet;
        use std::ffi::{OsStr, OsString};
        use std::io::Write;
        use std::path::{Path, PathBuf};
        use tempfile::{tempdir, Builder};

        use crate::config_reader::{CfgBlock, CfgCommentType, CfgLangEntry, Config, ConfigError};
        use crate::registry::{
            Block, CommentType, LangEntry, LangId, LangRegistry, LangRegistryError, LangSpec,
            LangStats,
        };

        #[test]
        fn add_entry_duplicate_entries() {
            let mut reg = LangRegistry::with_builtins_langs(Path::new("./dummy_dir/"));
            let err = reg
                .add_entry(
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
                )
                .unwrap_err();

            assert!(
                matches!(err, LangRegistryError::LangEntryDuplicated { ext, ..} if ext == OsStr::new("rs"))
            );
        }

        #[test]
        fn add_entry_ok() {
            let mut reg = LangRegistry::with_builtins_langs(Path::new("./dummy_dir/"));
            let size_before_add = reg.entries.len();
            reg.add_entry(
                LangSpec::new(
                    String::from("Python"),
                    vec![OsString::from("py")],
                    CommentType {
                        line: vec!["#".to_string()],
                        block: None,
                    },
                ),
                LangStats {
                    files: HashSet::new(),
                    loc: 0,
                },
            )
            .unwrap();
            let size_after_add = reg.entries.len();

            assert_eq!(size_before_add + 1, size_after_add);
        }

        #[test]
        fn with_config_err() {
            let cfg = Config {
                dir: "./dummy_dir/".into(),
                languages: vec![CfgLangEntry {
                    name: Some("Rust".to_string()),
                    extensions: None,
                    comments: Some(CfgCommentType {
                        line: Some(vec!["//".to_string()]),
                        block: None,
                    }),
                }],
            };

            let err = LangRegistry::with_config(cfg).unwrap_err();
            assert!(matches!(err, ConfigError::ExtensionMissing));
        }
        #[test]
        fn with_config_ok() {
            let cfg = Config {
                dir: "./dummy_dir/".into(),
                languages: vec![CfgLangEntry {
                    name: Some("Rust".to_string()),
                    extensions: Some(vec!["rs".to_string()]),
                    comments: Some(CfgCommentType {
                        line: Some(vec!["//".to_string()]),
                        block: Some(CfgBlock {
                            open: Some("/*".to_string()),
                            close: Some("*/".to_string()),
                        }),
                    }),
                }],
            };

            let mut map = HashMap::new();
            map.insert(OsString::from("rs"), LangId(0));
            let registry = LangRegistry::with_config(cfg).unwrap();
            assert_eq!(
                registry,
                LangRegistry {
                    dir: "./dummy_dir/".into(),
                    entries: vec![LangEntry {
                        spec: LangSpec {
                            name: "Rust".to_string(),
                            extensions: vec![OsString::from("rs")],
                            comments: CommentType {
                                line: vec!["//".to_string()],
                                block: Some(Block {
                                    open: "/*".to_string(),
                                    close: "*/".to_string()
                                })
                            }
                        },
                        stats: LangStats {
                            files: HashSet::new(),
                            loc: 0,
                        }
                    }],
                    map_ext_id: map,
                }
            );
        }
        #[test]
        fn with_builtins_langs_verify_registry() {
            let reg = LangRegistry::with_builtins_langs(Path::new("./dummy_dir/"));
            let mut reg_tst = LangRegistry::new();
            reg_tst.dir = PathBuf::from("./dummy_dir/");
            reg_tst
                .add_entry(
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
                )
                .unwrap();
            reg_tst
                .add_entry(
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
                )
                .unwrap();
            assert_eq!(reg, reg_tst);
        }
        #[test]
        fn update_stats_ok() {
            let dir = tempdir().unwrap();
            let mut file = Builder::new().suffix(".c").tempfile_in(dir.path()).unwrap();
            // add extension to the file that is why it fails
            let mut reg = LangRegistry::with_builtins_langs(file.path());
            let loc_before_update = reg.entries[1].stats.loc;
            let number_of_paths_before_update = reg.entries[1].stats.files.len();

            write!(
                file,
                r#"/* text
                   text
                */

                /* text
                    text
                */ code

                code /* text
                         text
                      */ // text

                /* text */

                // text
                code /* text */

                /* text */ code

                code // /* a */ code /* b */ code
                //
                /* text
                   text
                "#
            )
            .unwrap();
            let res = reg.update_stats();
            let number_of_paths_after_update = reg.entries[1].stats.files.len();
            let loc_after_update = reg.entries[1].stats.loc;
            assert!(res.is_ok());
            assert_ne!(number_of_paths_before_update, number_of_paths_after_update);
            assert_ne!(loc_before_update, loc_after_update);
            assert_eq!(loc_after_update, 5);
        }
    }
}
