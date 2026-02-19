use ::code_cnt::logic::cli::{Cli, Mode};
use ::code_cnt::ui::model;
use clap::{self, Parser};
use code_cnt::logic::cli;
use color_eyre::eyre::Result;

fn main() -> Result<()> {
    let args = Cli::parse();

    let arg_mode = &args.mode;

    match arg_mode {
        Mode::Cli => cli::run_cli(&args),
        Mode::Ui => {
            let model = model::Model::default();
            model.run()
        }
    }
}
