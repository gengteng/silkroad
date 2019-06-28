use crate::error::SkrdResult;
use serde_derive::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::PathBuf};

/// Registry
///
/// Directory structure:
///
/// root
///   ├─registry.toml
///   ├─index
///   │  ├─.git
///   │  └─ ...
///   └─crates
///      └─ ...
///
#[derive(Debug, Clone)]
pub struct Registry {
    root: PathBuf,
    config: RegistryConfig,
    index_git_path: PathBuf,
    index_path: PathBuf,
    crates_path: PathBuf,
}

impl Registry {
    pub const INDEX_GIT_DIRECTORY: &'static str = ".git";
    pub const INDEX_DIRECTORY: &'static str = "index";
    pub const CRATES_DIRECTORY: &'static str = "crates";
    pub const TOML_FILE: &'static str = "registry.toml";

    pub fn open<P: Into<PathBuf>>(root: P) -> SkrdResult<Self> {
        let root = root.into();

        let config = RegistryConfig::from(root.join(Registry::TOML_FILE))?;

        Ok(Registry {
            // join before `config` moved
            index_git_path: root
                .join(Registry::INDEX_DIRECTORY)
                .join(Registry::INDEX_GIT_DIRECTORY),
            index_path: root.join(Registry::INDEX_DIRECTORY),
            crates_path: root.join(Registry::CRATES_DIRECTORY),

            root,
            config,
        })
    }

    //    pub fn root(&self) -> &PathBuf {
    //        &self.root
    //    }

    pub fn index_git_path(&self) -> &PathBuf {
        &self.index_git_path
    }

    pub fn index_path(&self) -> &PathBuf {
        &self.index_path
    }

    pub fn crates_path(&self) -> &PathBuf {
        &self.crates_path
    }

    pub fn config(&self) -> &RegistryConfig {
        &self.config
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistryConfig {
    meta: Meta,
    access: Access,
}

impl RegistryConfig {
    fn from<P: Into<PathBuf>>(path: P) -> SkrdResult<Self> {
        let mut file = File::open(path.into())?;
        let mut content = String::with_capacity(file.metadata()?.len() as usize);
        file.read_to_string(&mut content)?;

        Ok(toml::from_str::<RegistryConfig>(&content)?)
    }

    pub fn name(&self) -> &str {
        &self.meta.name
    }

    pub fn receive_on(&self) -> bool {
        self.access.receive
    }

    pub fn upload_on(&self) -> bool {
        self.access.upload
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Meta {
    name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Access {
    #[serde(rename = "git-receive-pack")]
    receive: bool,
    #[serde(rename = "git-upload-pack")]
    upload: bool,
}
