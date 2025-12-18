use clap::Parser;
use code_cnt::config_reader::{Config, ConfigError};
use code_cnt::registry::LangRegistry;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    dir: PathBuf,
}

fn main() -> Result<(), ConfigError> {
    let args = Args::parse();
    let arg_dir = args.dir;

    println!("Processing directory: {}", arg_dir.display());
    let config = Config::load("example.toml")?;

    println!("MAIN: {}", config.dir.display());
    println!("CONFIG\n {:#?}\n ******************\n\n", config);
    //
    //let mut lang_registry = LangRegistry::with_builtins_langs(&arg_dir);
    //if let Err(e) = lang_registry.update_stats() {
    //    eprintln!("Error updating stats: {e}");
    //    std::process::exit(1);
    //}

    //lang_registry.show_stats();
    let mut lang_registry_toml = LangRegistry::with_config(config)?; // config consumed here and
                                                                     // will not be used after
    if let Err(e) = lang_registry_toml.update_stats() {
        eprintln!("Error updating stats: {e}");
        std::process::exit(1);
    }
    lang_registry_toml.show_stats();
    Ok(())
}
