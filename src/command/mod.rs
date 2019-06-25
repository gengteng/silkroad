use structopt::StructOpt;

pub mod execute;
pub mod pack;
pub mod serve;
pub mod update;

/// SilkRoad Command
#[derive(Debug, StructOpt)]
#[structopt(name = "skrd")]
pub enum Command {
    /// Update crate.io-index or crates or all
    #[structopt(name = "update")]
    Update(update::Update),

    /// Start a full featured registry
    #[structopt(name = "serve")]
    Serve(serve::Serve),

    /// Pack up index and crates for use in a LAN
    #[structopt(name = "pack")]
    Pack(pack::Pack),

    /// Execute the commands in a TOML file
    #[structopt(name = "exec")]
    Execute(execute::Execute),
}
