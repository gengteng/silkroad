use structopt::StructOpt;

pub mod execute;
pub mod new;
pub mod pack;
pub mod serve;

/// SilkRoad Command
#[derive(Debug, StructOpt)]
#[structopt(name = "skrd")]
pub enum Command {
    /// create a new registry directory
    #[structopt(name = "new")]
    New(new::New),

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
