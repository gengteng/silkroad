#[macro_use]
extern crate slog;

#[macro_use]
extern crate slog_global;

use structopt::StructOpt;

mod command;
use command::Command;

mod error;
use crate::error::{SkrdError, SkrdResult};

mod log;
use crate::log::LoggerGuard;

fn main() -> SkrdResult<()> {
    let _logger_guard = LoggerGuard::init("SilkRoad");

    let command = Command::from_args();

    match command {
        Command::Update(_update) => Err(SkrdError::Custom(
            "Subcommand update is unimplemented!".to_owned(),
        )),
        Command::Serve(serve) => serve.serve(),
        Command::Pack(_pack) => Err(SkrdError::Custom(
            "Subcommand pack is unimplemented!".to_owned(),
        )),
    }
}
