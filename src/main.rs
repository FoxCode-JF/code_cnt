use clap::Parser;
use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fs::{read_dir, File};
use std::io::{BufRead, BufReader, Error};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Copy, Debug)]
struct LangId(usize);

#[derive(Parser)]
struct Args {
    dir: PathBuf,
}

#[derive(Debug)]
struct CommentTypes {
    line: HashSet<String>,
    block: HashSet<(String, String)>,
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

    fn get_entry_id(&mut self, ext: &OsStr) -> Option<LangId> {
        self.map_ext_id.get(ext).copied()
    }
    fn clear_locs(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.stats.loc = 0;
        }
    }

    pub fn show_stats(self) {
        println!("STATS for directory: {}", self.dir.display());
        for entry in self.entries {
            println!("{}, loc {}", entry.spec.name, entry.stats.loc);
        }
    }
    pub fn new() -> Self {
        Self {
            dir: "".into(),
            entries: Vec::new(),
            map_ext_id: HashMap::new(),
        }
    }

    pub fn default(dir: &Path) -> Self {
        let mut reg = LangRegistry::new();

        reg.dir = dir.to_path_buf();
        reg.add_entry(
            LangSpec::new(
                String::from("Rust"),
                vec![OsString::from("rs")],
                CommentTypes {
                    line: HashSet::from_iter(["//".to_string()]),
                    block: HashSet::from_iter([("/*".to_string(), "*/".to_string())]),
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
                    line: HashSet::from_iter(["//".to_string()]),
                    block: HashSet::from_iter([("/*".to_string(), "*/".to_string())]),
                },
            ),
            LangStats {
                files: HashSet::new(),
                loc: 0,
            },
        );
        reg
    }
    //pub fn init() {
    //
    //}
    pub fn update_stats(&mut self) -> std::result::Result<(), std::io::Error> {
        self.clear_locs();

        for item in WalkDir::new(self.dir.clone()).into_iter().flatten() {
            let path = item.into_path();
            if !path.is_file() {
                continue;
            }
            if let Some(id) = path.extension().and_then(|ext| self.get_entry_id(ext)) {
                let comments = &self.get_spec(id).comments;

                let loc = count_lines(&path, comments)?;
                let stats = self.stats_mut(id);
                //let spec = self.get_spec(id);
                stats.files.insert(path.clone());
                stats.loc += loc;
            }
        }
        Ok(())
    }

    //fn get_entry_id() {}
    //pub fn update() {}
}

fn main() {
    let args = Args::parse();
    let arg_dir = args.dir;

    println!("Processing directory: {}", arg_dir.display());

    let mut lang_registry = LangRegistry::default(&arg_dir);
    let _ = lang_registry.update_stats();

    lang_registry.show_stats();
}

fn count_lines(path: &Path, comments: &CommentTypes) -> Result<u64, std::io::Error> {
    if !path.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        ));
    }
    let cnt = BufReader::new(File::open(path)?)
        .lines()
        .map_while(Result::ok)
        .filter(|line| {
            let trimmed = line.trim();

            //println!("{} :: !is_empty:{}", trimmed, !trimmed.is_empty(),);
            !(trimmed.len() >= 2 && comments.line.contains(&trimmed[..2]) || trimmed.is_empty())
        })
        .count() as u64;

    Ok(cnt)
}
