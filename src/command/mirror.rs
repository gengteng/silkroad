use crate::error::{SkrdError, SkrdResult};
use crate::registry::Registry;
use std::io::BufRead;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Mirror {
    #[structopt(
        long = "name",
        help = "Set the registry name",
        value_name = "REGISTRY NAME"
    )]
    name: Option<String>,

    #[structopt(help = "Set the registry path", value_name = "path")]
    path: PathBuf,

    #[structopt(
        help = "Set the url of the server to be mirrored",
        value_name = "source",
        default_value = "https://github.com/rust-lang/crates.io-index"
    )]
    source: String,
}

impl Mirror {
    pub fn mirror(self) -> SkrdResult<()> {
        /* TODO: Check if the path is already a mirror, if so, update the index and download crates, if not, create a new one */

        // check name
        let name = if let Some(name) = &self.name {
            name.clone()
        } else {
            self.path
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| SkrdError::StaticCustom("the registry path provided is invalid"))?
                .to_owned()
        };

        let registry = Registry::create(&self.path, &name)?;

        info!(
            "{} is being cloned into {:?} ...",
            self.source,
            registry.index_path()
        );

        git2::Repository::clone(&self.source, registry.index_path())?;

        info!("{} cloned.", self.source);

        info!("Continue to download crates?(Y/n)");

        let stdin = std::io::stdin();

        let mut lines = stdin.lock().lines();

        let download = if let Some(Ok(download)) = lines.next() {
            download == "Y"
        } else {
            false
        };

        if download {
            info!("Start to download crates...");
        }

        info!("Mirror is created.");
        Ok(())
    }
}
