use clap::Parser;
use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Copy, Debug)]
struct LangId(usize);

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    dir: PathBuf,
}

#[derive(Debug)]
struct CommentTypes {
    line: Vec<String>,
    block: Vec<(String, String)>,
}

#[derive(Debug)]
struct LangSpec {
    name: String,
    extensions: Vec<OsString>,
    comments: CommentTypes,
}

impl LangSpec {
    fn new(name: String, extensions: Vec<OsString>, comments: CommentTypes) -> Self {
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
struct LangEntry {
    spec: LangSpec,
    stats: LangStats,
}

#[derive(Debug)]
struct LangRegistry {
    dir: PathBuf,
    entries: Vec<LangEntry>,
    map_ext_id: HashMap<OsString, LangId>,
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

    pub fn with_builtins_langs(dir: &Path) -> Self {
        let mut reg = LangRegistry::new();

        reg.dir = dir.to_path_buf();
        reg.add_entry(
            LangSpec::new(
                String::from("Rust"),
                vec![OsString::from("rs")],
                CommentTypes {
                    line: vec!["//".to_string(), "///".to_string(), "//!".to_string()],
                    block: vec![("/*".to_string(), "*/".to_string())],
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
                CommentTypes {
                    line: vec!["//".to_string()],
                    block: vec![("/*".to_string(), "*/".to_string())],
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

fn main() {
    let args = Args::parse();
    let arg_dir = args.dir;

    println!("Processing directory: {}", arg_dir.display());

    let mut lang_registry = LangRegistry::with_builtins_langs(&arg_dir);
    if let Err(e) = lang_registry.update_stats() {
        eprintln!("Error updating stats: {e}");
        std::process::exit(1);
    }

    lang_registry.show_stats();
}

fn count_lines(path: &Path, comments: &CommentTypes) -> Result<u64, std::io::Error> {
    if !path.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        ));
    }
    let mut inside_block = false;
    let cnt = BufReader::new(File::open(path)?)
        .lines()
        .map_while(Result::ok)
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return false;
            }
            if comments.line.iter().any(|token| trimmed.starts_with(token)) {
                return false;
            }
            let mut code_present = false;
            for block_type in comments.block.iter() {
                let mut block_stack = Vec::new();
                let mut start = 0;

                if inside_block {
                    if let Some(idx) = trimmed.find(&block_type.1) {
                        start = idx;
                        inside_block = false;
                    } else {
                        return false;
                    }
                }

                while start < trimmed.len() {
                    if trimmed[start..].starts_with(&block_type.0) {
                        start += block_type.0.len();
                        block_stack.push(&block_type.0);
                        inside_block = true;
                    } else if trimmed[start..].starts_with(&block_type.1) {
                        start += block_type.1.len();
                        if block_stack.last() == Some(&&block_type.0) {
                            block_stack.pop();
                            if block_stack.is_empty() {
                                inside_block = false;
                            }
                        }
                    } else if !inside_block {
                        if comments
                            .line
                            .iter()
                            .any(|token| trimmed[start..].trim().starts_with(token))
                        {
                            return code_present;
                        }
                        code_present = true;
                    }
                    start += 1;
                }
            }
            code_present
        })
        .count() as u64;

    Ok(cnt)
}

#[cfg(test)]
mod tests {
    mod count_lines {
        use crate::CommentTypes;
        use std::{io::Write, path::Path};
        use tempfile::NamedTempFile;

        #[test]
        fn empty_file() {
            let file = NamedTempFile::new().unwrap();
            let comments = CommentTypes {
                line: vec!["//".to_string()],
                block: vec![("/*".to_string(), "*/".to_string())],
            };
            let res = crate::count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 0);
        }

        #[test]
        fn not_a_file() {
            let comments = CommentTypes {
                line: vec!["//".to_string()],
                block: vec![("/*".to_string(), "*/".to_string())],
            };
            let res = crate::count_lines(Path::new("./"), &comments);
            assert!(res.is_err());
        }

        #[test]
        fn only_comments_and_newlines() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentTypes {
                line: vec!["//".to_string()],
                block: vec![("/*".to_string(), "*/".to_string())],
            };
            write!(
                file,
                r#"//
                //
                // text
                //
                
                
                //
                    
                "#
            )
            .unwrap();

            let res = crate::count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 0);
        }
        #[test]
        fn single_line_comments_with_code() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentTypes {
                line: vec!["//".to_string()],
                block: vec![("/*".to_string(), "*/".to_string())],
            };
            write!(
                file,
                r#"//
                // text
                //
                // text
                code
                code
                    code
                code
                //

                "#
            )
            .unwrap();

            let res = crate::count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 4);
        }

        #[test]
        fn single_line_comment_after_code() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentTypes {
                line: vec!["//".to_string()],
                block: vec![("/*".to_string(), "*/".to_string())],
            };
            write!(
                file,
                r#"//
                code
                code

                code // text
                //text 
                "#
            )
            .unwrap();

            let res = crate::count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 3);
        }

        #[test]
        fn comments_inside_string() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentTypes {
                line: vec!["//".to_string()],
                block: vec![("/*".to_string(), "*/".to_string())],
            };
            write!(
                file,
                r#"
                let s = "/* not a comment */";
                let s = "// not a comment "#
            )
            .unwrap();

            let res = crate::count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 2);
        }

        #[test]
        fn block_multi_line_no_code() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentTypes {
                line: vec!["//".to_string()],
                block: vec![("/*".to_string(), "*/".to_string())],
            };
            write!(
                file,
                r#"/* text
                   text
                */
                 "#
            )
            .unwrap();
            let res = crate::count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 0);
        }

        #[test]
        fn block_multi_line_code_after() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentTypes {
                line: vec!["//".to_string()],
                block: vec![("/*".to_string(), "*/".to_string())],
            };
            write!(
                file,
                r#"
                 /* text
                    text
                 */ code
                 "#
            )
            .unwrap();
            let res = crate::count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 1);
        }
        #[test]
        fn block_comments_multi_line_code_before() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentTypes {
                line: vec!["//".to_string()],
                block: vec![("/*".to_string(), "*/".to_string())],
            };
            write!(
                file,
                r#"
                 code /* text
                         text
                      */
                 "#
            )
            .unwrap();
            let res = crate::count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 1);
        }
        #[test]
        fn block_comments_single_line_no_code() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentTypes {
                line: vec!["//".to_string()],
                block: vec![("/*".to_string(), "*/".to_string())],
            };
            write!(
                file,
                r#"
                 /* text */
                 "#
            )
            .unwrap();
            let res = crate::count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 0);
        }
        #[test]
        fn block_comments_single_line_code_before() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentTypes {
                line: vec!["//".to_string()],
                block: vec![("/*".to_string(), "*/".to_string())],
            };
            write!(
                file,
                r#"
                 code /* text */
                 "#
            )
            .unwrap();
            let res = crate::count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 1);
        }
        #[test]
        fn block_comments_single_line_code_after() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentTypes {
                line: vec!["//".to_string()],
                block: vec![("/*".to_string(), "*/".to_string())],
            };
            write!(
                file,
                r#"
                 /* text */ code
                 "#
            )
            .unwrap();
            let res = crate::count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 1);
        }
        #[test]
        fn block_comments_single_line_interleaved() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentTypes {
                line: vec!["//".to_string()],
                block: vec![("/*".to_string(), "*/".to_string())],
            };
            write!(
                file,
                r#"
                 code /* a */ code /* b */ code
                 "#
            )
            .unwrap();
            let res = crate::count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 1);
        }
        #[test]
        fn block_comments_no_end() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentTypes {
                line: vec!["//".to_string()],
                block: vec![("/*".to_string(), "*/".to_string())],
            };
            write!(
                file,
                r#"
                /* text
                    text
                 "#
            )
            .unwrap();
            let res = crate::count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 0);
        }

        #[test]
        fn mixed_line_and_block_comments() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentTypes {
                line: vec!["//".to_string()],
                block: vec![("/*".to_string(), "*/".to_string())],
            };
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
            let res = crate::count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 5);
        }

        // TODO:
        // below test cases do not pass yet.
        // They might be handled in the future, however at the moment counter
        // is accurate enough for a rough estimate Heuristics FTW!
        //
        //#[test]
        //fn multi_line_string() {
        //    let mut file = NamedTempFile::new().unwrap();
        //    let comments = CommentTypes {
        //        line: vec!["//".to_string()],
        //        block: vec![("/*".to_string(), "*/".to_string())],
        //    };
        //    write!(
        //        file,
        //        r#"
        //        let s = "line1
        //        /* block comment inside string
        //         * just ingnore it?
        //         * */
        //        line3";
        //        "#
        //    )
        //    .unwrap();
        //    let res = crate::count_lines(file.path(), &comments);
        //    assert_eq!(res.unwrap(), 5);
        //}
        //#[test]
        //fn windows_newlines() {
        //    let mut file = NamedTempFile::new().unwrap();
        //    let comments = CommentTypes {
        //        line: vec!["//".to_string()],
        //        block: vec![("/*".to_string(), "*/".to_string())],
        //    };
        //    write!(file, "\r\ncode\n//text\r\n").unwrap();
        //    let res = crate::count_lines(file.path(), &comments);
        //    assert_eq!(res.unwrap(), 0);
        //}
    }
}
