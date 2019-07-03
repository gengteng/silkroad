use structopt::StructOpt;

pub mod create;
pub mod execute;
pub mod mirror;
pub mod package;
pub mod serve;
pub mod update;

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

    /// Update an existing mirror
    #[structopt(name = "update")]
    Update(update::Update),

    /// Start a full featured registry
    #[structopt(name = "serve")]
    Serve(serve::Serve),

    /// Pack up index and crates for use in a LAN
    #[structopt(name = "package")]
    Package(package::Package),

    /// Execute a command in a TOML file
    #[structopt(name = "exec")]
    Execute(execute::Execute),
}
