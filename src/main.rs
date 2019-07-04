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
mod stream;

fn main() -> SkrdResult<()> {
    let _logger_guard = LoggerGuard::init("SilkRoad", Level::Info);

    let command = Command::from_args();

    match command {
        // private registry
        Command::Create(create) => create.create(),

        // mirroring
        Command::Mirror(mirror) => mirror.mirror(),
        Command::Update(update) => update.update(),

        // server
        Command::Serve(serve) => serve.serve(),

        // migration
        Command::Package(_pack) => {
            Err(SkrdError::StaticCustom("Subcommand pack is unimplemented!"))
        }

        // command line tool
        Command::Execute(_exec) => Err(SkrdError::StaticCustom("Subcommand exec is unimplemented")),
    }
}
