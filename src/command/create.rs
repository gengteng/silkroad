use crate::error::SkrdResult;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Create {
    #[structopt(
        long = "name",
        help = "Set the registry name",
        value_name = "REGISTRY NAME"
    )]
    name: Option<String>,

    #[structopt(help = "Set the registry path", value_name = "path")]
    path: PathBuf,
}

impl Create {
    pub fn create(self) -> SkrdResult<()> {
        Ok(())
    }
}

// 1. Check and create the directory
// 2. Create `registry.toml`.
// 3. Clone the index project
// 4. Follow the index to download crates
// 5. Use the database to record downloads
