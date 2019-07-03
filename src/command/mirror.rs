use crate::error::{SkrdError, SkrdResult};
use crate::registry::Registry;
use crate::util::download_crates;
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

        let registry = Registry::mirror(&self.path, &name, &self.source)?;

        info!(
            "{} is being cloned into {:?} ...",
            self.source,
            registry.index_path()
        );

        drop(git2::Repository::clone(
            &self.source,
            registry.index_path(),
        )?);

        info!("{} cloned.", self.source);

        info!("Start to download crates...");

        download_crates(&registry)?;

        info!("Mirror is created.");
        Ok(())
    }
}

// 1. Check and create the directory
// 2. Create `registry.toml`.
// 3. Clone the index project
// 4. Follow the index to download crates
// 5. Use the database to record downloads
