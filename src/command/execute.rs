use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "exec")]
pub struct Execute {
    #[structopt(
        long = "toml",
        short = "f",
        help = "Set the TOML file path",
        value_name = "PATH",
        parse(try_from_str)
    )]
    toml: PathBuf,
}
