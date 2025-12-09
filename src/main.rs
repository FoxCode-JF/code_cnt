use clap::Parser;
use code_cnt::registry::LangRegistry;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    dir: PathBuf,
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
