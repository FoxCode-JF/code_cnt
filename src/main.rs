use clap::Parser;
use ignore::WalkBuilder;
use std::fs::{self, read_dir, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
struct Args {
    dir: PathBuf,
}

fn main() {
    let args = Args::parse();
    let arg_dir = args.dir;

    println!("Processing directory: {}", arg_dir.display());

    let cnt = count_lines(&arg_dir).unwrap();
    println!(
        "Lines in {} : {}",
        arg_dir.file_name().unwrap().display(),
        cnt
    );
}

fn count_lines(path: &Path) -> Result<u32, std::io::Error> {
    if !path.is_file() {
        println!("Not a file")
    }
    let cnt = BufReader::new(File::open(path)?)
        .lines()
        .map_while(Result::ok)
        .filter(|line| {
            let trimmed = line.trim();
            let mut oneline_comment = false;

            if trimmed.len() >= 2 && &trimmed[..2] == "//" {
                oneline_comment = true;
            }
            println!(
                "{} :: !is_empty:{} || !oneliner {}",
                trimmed,
                !trimmed.is_empty(),
                !oneline_comment,
            );
            !trimmed.is_empty() && !oneline_comment
        })
        .count() as u32;

    Ok(cnt)
}

fn get_file_list_auto_walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut res = Vec::new();
    println!("WalkDir start");
    for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
        //println!("entry {}", entry.clone().into_path().display());
        if entry.file_type().is_file() {
            res.push(entry.into_path());
        }
    }
    println!("WalkDir end");
    res
}

fn get_file_list_auto_ignore(dir: &Path) -> Vec<PathBuf> {
    let mut res = Vec::new();
    println!("ignore start");
    for entry in WalkBuilder::new(dir)
        .git_ignore(false)
        .git_exclude(false)
        .git_global(false)
        .build()
        .filter_map(Result::ok)
    {
        //println!("entry {}", entry.clone().into_path().display());
        if entry.path().is_file() {
            res.push(entry.into_path());
        }
    }
    println!("ignore end");
    res
}

fn get_file_list_manual(dir: &Path) -> Vec<PathBuf> {
    println!("manual start");
    let mut dirs = Vec::new();
    let mut res = Vec::new();

    dirs.push(dir.to_path_buf());

    while !dirs.is_empty() {
        let paths = read_dir(dirs.pop().unwrap());
        match paths {
            Ok(paths) => {
                for entry in paths.flatten() {
                    let cur_path = entry.path();
                    if cur_path.is_dir() {
                        dirs.push(cur_path);
                    } else if cur_path.is_file() {
                        res.push(entry.path());
                    } else {
                        // ignore symlinks
                    }
                }
            }
            Err(_) => {
                println!("read_dir() failed");
            }
        }
    }
    println!("{:#?}", res);
    println!("manual end");
    res
}

#[cfg(test)]
use itertools::izip;
#[test]
fn verify_variants_return_same_files() {
    use std::fs::{create_dir, File};
    use tempfile::{tempdir, TempDir};

    let root = tempdir().unwrap();
    fn build_test_tree(root: &TempDir) -> PathBuf {
        let root_path = root.path().join("tst/");
        create_dir(&root_path).unwrap();

        let dirs = vec!["a", "a/b", "a/b/c"];
        for d in dirs {
            let full = root_path.join(d);
            create_dir(&full).unwrap();

            for i in 0..(2 + (d.len() % 4)) {
                let fpath = full.join(format!("file_{i}.txt"));
                File::create(fpath).unwrap();
            }
        }
        root_path
    }

    let dir = build_test_tree(&root);
    println!("{}", dir.display());
    let mut walkdir_paths = get_file_list_auto_walkdir(&dir);
    let mut ignore_paths = get_file_list_auto_ignore(&dir);
    let mut manual_paths = get_file_list_manual(&dir);

    ignore_paths.sort();
    walkdir_paths.sort();
    manual_paths.sort();

    for (path_w, path_i, path_m) in izip!(
        walkdir_paths.iter(),
        ignore_paths.iter(),
        manual_paths.iter(),
    ) {
        assert!(path_m == path_i && path_i == path_w);
        if path_w != path_i || path_i != path_m {
            println!(
                "{} != {} != {}",
                path_w.display(),
                path_i.display(),
                path_m.display()
            );
        } else {
            println!(
                "{} == {} == {}",
                path_w.display(),
                path_i.display(),
                path_m.display()
            );
        }
    }
}
