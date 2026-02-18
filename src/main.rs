use ::code_cnt::logic::cli::{Cli, Mode};
use clap::{self, Parser};
use code_cnt::logic::cli;
use code_cnt::logic::config_reader::ConfigError;

fn main() -> Result<(), ConfigError> {
    let args = Cli::parse();

    let arg_mode = &args.mode;

    match arg_mode {
        Mode::Cli => cli::run_cli(&args),
        Mode::Ui => {
            todo!("implement sth like run_ui");
        }
    }
}
