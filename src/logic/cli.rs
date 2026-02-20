use super::config_reader::Config;
use super::registry::LangRegistry;
use clap::{CommandFactory, Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, ValueEnum)]
pub enum Mode {
    Cli,
    Ui,
}

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    /// Software mode terminal UI or command line)
    #[arg(short, long, value_enum, default_value_t = Mode::Ui)]
    pub mode: Mode,

    /// Directory where the files are. Runs with default configuration.
    #[arg(short, long)]
    dir: Option<PathBuf>,

    /// Config path
    #[arg(short, long)]
    cfg: Option<String>,
}

pub fn run_cli(args: &Cli) -> color_eyre::Result<()> {
    let arg_dir = &args.dir;
    let arg_cfg = &args.cfg;

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
            let config = Config::load(arg_cfg)?;
            println!("Config read successfully...");
            let mut reg = LangRegistry::with_config(config)?;
            reg.update_stats()?;
            reg.show_stats();
            return Ok(());
        }
    };
    println!("No external configuration provided. Running with defaults...");
    let mut reg = LangRegistry::with_builtins_langs(arg_dir);
    reg.update_stats()?;
    reg.show_stats();
    Ok(())
}
