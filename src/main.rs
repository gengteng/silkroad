#[macro_use]
extern crate slog;

#[macro_use]
extern crate log;

use structopt::StructOpt;

mod command;
use command::Command;

mod error;
use crate::error::{SkrdError, SkrdResult};

mod logger;
use crate::logger::LoggerGuard;
use slog::Level;

mod util;

fn main() -> SkrdResult<()> {
    let _logger_guard = LoggerGuard::init("SilkRoad", Level::Info);

    let command = Command::from_args();

    match command {
        Command::Update(_update) => Err(SkrdError::Custom(
            "Subcommand update is unimplemented!".to_owned(),
        )),
        Command::Serve(serve) => serve.serve(),
        Command::Pack(_pack) => Err(SkrdError::Custom(
            "Subcommand pack is unimplemented!".to_owned(),
        )),
        Command::Execute(_exec) => Err(SkrdError::Custom(
            "Subcommand exec is unimplemented".to_owned(),
        )),
    }
}
