use structopt::StructOpt;

pub mod create;
pub mod execute;
pub mod mirror;
pub mod package;
pub mod serve;

/// SilkRoad Command
#[derive(Debug, StructOpt)]
#[structopt(name = "skrd")]
pub enum Command {
    /// Create a new registry directory
    #[structopt(name = "create")]
    Create(create::Create),

    /// Mirror an existing source
    #[structopt(name = "mirror")]
    Mirror(mirror::Mirror),

    /// Start a full featured registry
    #[structopt(name = "serve")]
    Serve(serve::Serve),

    /// Pack up index and crates for use in a LAN
    #[structopt(name = "package")]
    Package(package::Package),

    /// Execute the commands in a TOML file
    #[structopt(name = "exec")]
    Execute(execute::Execute),
}
