use crate::registry::UrlConfig;
use crate::{
    error::{SkrdError, SkrdResult},
    registry::Registry,
};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
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

        let repo = git2::Repository::init(registry.index_path())?;

        let mut index = repo.index()?;

        let content = serde_json::to_string_pretty(&UrlConfig::from(&registry))?;

        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(registry.index_path().join("config.json"))?;
        file.write_all(content.as_bytes())?;
        drop(file);

        index.add_path(Path::new("config.json"))?;
        index.write()?;

        let tree = index.write_tree().and_then(|id| repo.find_tree(id))?;
        let sig = repo.signature()?;

        repo.commit(Some("HEAD"), &sig, &sig, "base_url", &tree, &[])?;

        Ok(())
    }
}
