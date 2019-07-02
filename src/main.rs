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

mod registry;
mod util;

fn main() -> SkrdResult<()> {
    let _logger_guard = LoggerGuard::init("SilkRoad", Level::Info);

    let command = Command::from_args();

    match command {
        Command::Create(create) => create.create(),
        Command::Mirror(mirror) => mirror.mirror(),
        Command::Serve(serve) => serve.serve(),
        Command::Package(_pack) => {
            Err(SkrdError::StaticCustom("Subcommand pack is unimplemented!"))
        }
        Command::Execute(_exec) => Err(SkrdError::StaticCustom("Subcommand exec is unimplemented")),
    }
}
