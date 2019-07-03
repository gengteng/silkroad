use crate::{
    error::{SkrdError, SkrdResult},
    registry::Registry,
    util::download_crates,
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Update {
    #[structopt(
        help = "Set the registry path",
        value_name = "REGISTRY PATH",
        parse(try_from_str)
    )]
    registry: Option<Registry>,
}

impl Update {
    pub fn update(self) -> SkrdResult<()> {
        // if registry is not specified, try current directory
        let registry = if let Some(registry) = self.registry {
            registry
        } else {
            let current_dir = std::env::current_dir()?;
            Registry::open(current_dir)?
        };

        info!("Start to update mirror '{}' ...", registry.config().name());

        info!("{:?}", registry);

        let repo = git2::Repository::open(registry.index_path())?;

        let remotes = repo.remotes()?;
        if remotes.len() == 0 {
            return Err(SkrdError::StaticCustom(
                "This registry does not seem to be a mirror",
            ));
        }

        repo.find_remote("origin")?.fetch(&["master"], None, None)?;
        drop(repo);
        info!("Index synchronization is complete.");

        download_crates(&registry)?;

        Ok(())
    }
}
