use clap::{CommandFactory, Parser};
use code_cnt::config_reader::{Config, ConfigError};
use code_cnt::registry::LangRegistry;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Directory where the files are. Runs with default configuration.
    #[arg(short, long)]
    dir: Option<PathBuf>,

    /// Config path
    #[arg(short, long)]
    cfg: Option<String>,
}

fn main() -> Result<(), ConfigError> {
    let args = Cli::parse();
    let arg_dir = args.dir;
    let arg_cfg = args.cfg;

    if arg_cfg.is_none() && arg_dir.is_none() {
        Cli::command().print_long_help()?;
        return Ok(());
    }

    let arg_dir = match arg_dir {
        Some(d) => d,
        None => {
            let arg_cfg = match arg_cfg {
                Some(p) => p,
                None => {
                    Cli::command().print_long_help()?;
                    return Ok(());
                }
            };
            let config = Config::load(&arg_cfg)?;
            println!("Config read successfully...");
            let mut reg = LangRegistry::with_config(config)?;
            reg.update_stats()?;
            reg.show_stats();
            return Ok(());
        }
    };
    println!("No external configuration provided. Running with defaults...");
    let mut reg = LangRegistry::with_builtins_langs(&arg_dir);
    reg.update_stats()?;
    reg.show_stats();
    Ok(())
}
