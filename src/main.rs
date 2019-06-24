use structopt::StructOpt;

mod command;
use command::Command;

mod error;
use crate::error::{SkrdError, SkrdResult};

fn main() -> SkrdResult<()> {
    let command = Command::from_args();

    match command {
        Command::Update(_update) => Err(SkrdError::General(
            "Subcommand update is unimplemented!".to_owned(),
        )),
        Command::Serve(serve) => serve.serve(),
        Command::Pack(_pack) => Err(SkrdError::General(
            "Subcommand pack is unimplemented!".to_owned(),
        )),
    }
}
