use crate::error::{SkrdError, SkrdResult};
use rustls::internal::pemfile::{certs, rsa_private_keys};
use rustls::{NoClientAuth, ServerConfig};
use serde_derive::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{BufReader, Write};
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
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
    index_path: PathBuf,
    index_git_path: PathBuf,
    crates_path: PathBuf,
}

impl Registry {
    pub const INDEX_GIT_DIRECTORY: &'static str = ".git";
    pub const INDEX_DIRECTORY: &'static str = "index";
    pub const CRATES_DIRECTORY: &'static str = "crates";
    pub const TOML_FILE: &'static str = "registry.toml";

    pub fn open<P: Into<PathBuf>>(root: P) -> SkrdResult<Self> {
        let root = root.into();

        let config = RegistryConfig::open(root.join(Registry::TOML_FILE))?;

        let index_path = root.join(Registry::INDEX_DIRECTORY);
        let crates_path = root.join(Registry::CRATES_DIRECTORY);

        let registry = Registry {
            index_git_path: index_path.join(Registry::INDEX_GIT_DIRECTORY),
            index_path,
            crates_path,

            root,
            config,
        };

        Ok(registry)
    }

    pub fn mirror<P: Into<PathBuf>>(root: P, name: &str, source: &str) -> SkrdResult<Self> {
        let root = root.into();

        let (index_path, crates_path) = create_registry_dirs(&root)?;

        let toml_path = root.join(Registry::TOML_FILE);
        let config = RegistryConfig::mirror(name, source);
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&toml_path)?;
        let toml = toml::to_string_pretty(&config)?;
        file.write_all(toml.as_bytes())?;
        drop(file);
        info!("Registry config file {:?} is created.", toml_path);

        let index_git_path = index_path.join(Registry::INDEX_GIT_DIRECTORY);

        let registry = Registry {
            index_path,
            index_git_path,
            crates_path,
            root,
            config,
        };

        Ok(registry)
    }

    pub fn create<P: Into<PathBuf>>(root: P, name: &str) -> SkrdResult<Self> {
        let root = root.into();

        let (index_path, crates_path) = create_registry_dirs(&root)?;

        let toml_path = root.join(Registry::TOML_FILE);
        let config = RegistryConfig::create(name);
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&toml_path)?;
        let toml = toml::to_string_pretty(&config)?;
        file.write_all(toml.as_bytes())?;
        drop(file);
        info!("Registry config file {:?} is created.", toml_path);

        let index_git_path = index_path.join(Registry::INDEX_GIT_DIRECTORY);

        let registry = Registry {
            index_path,
            index_git_path,
            crates_path,
            root,
            config,
        };

        Ok(registry)
    }

    //    pub fn root(&self) -> &PathBuf {
    //        &self.root
    //    }

    pub fn index_path(&self) -> &PathBuf {
        &self.index_path
    }

    pub fn index_git_path(&self) -> &PathBuf {
        &self.index_git_path
    }

    pub fn crates_path(&self) -> &PathBuf {
        &self.crates_path
    }

    pub fn config(&self) -> &RegistryConfig {
        &self.config
    }

    pub fn base_url(&self) -> String {
        let config = self.config();
        format!(
            "{}://{}{}/{}",
            if config.ssl() { "https" } else { "http" },
            config.domain(),
            if is_default_port(config.port(), config.ssl()) {
                "".to_owned()
            } else {
                format!(":{}", config.port())
            },
            config.name()
        )
    }
}

fn create_registry_dirs(root: &PathBuf) -> SkrdResult<(PathBuf, PathBuf)> {
    std::fs::create_dir(root)?;
    info!("Root path {:?} is created.", root);

    let index_path = root.join(Registry::INDEX_DIRECTORY);
    std::fs::create_dir(&index_path)?;
    info!("Index path {:?} is created.", index_path);

    let crates_path = root.join(Registry::CRATES_DIRECTORY);
    std::fs::create_dir(&crates_path)?;
    info!("Crates path {:?} is created.", crates_path);

    Ok((index_path, crates_path))
}

impl FromStr for Registry {
    type Err = SkrdError;

    fn from_str(s: &str) -> SkrdResult<Self> {
        Registry::open(s)
    }
}

///
/// Registry Configuration read from `registry.toml`
///
/// .toml example:
///
/// ```toml
///
/// [meta]
/// name = "goe2"
///
/// [http]
/// domain = "goe2.net"
/// ip = "127.0.0.1"
/// port = 443
/// ssl = true
/// cert = "path/to/cert.pem"
/// key = "path/to/key.pem"
///
/// [access]
/// git-receive-pack = true
/// git-upload-pack = false
///
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistryConfig {
    meta: Meta,
    mirror: Option<Mirror>,
    http: HttpConfig,
    access: AccessControl,
}

impl RegistryConfig {
    pub fn open<P: Into<PathBuf>>(path: P) -> SkrdResult<Self> {
        let mut file = File::open(path.into())?;
        let mut content = String::with_capacity(file.metadata()?.len() as usize);
        file.read_to_string(&mut content)?;

        Ok(toml::from_str::<RegistryConfig>(&content)?)
    }

    pub fn create(name: &str) -> Self {
        RegistryConfig {
            meta: Meta {
                name: name.to_owned(),
            },
            mirror: None,
            http: HttpConfig::default(),
            access: AccessControl::default(),
        }
    }

    pub fn mirror(name: &str, source: &str) -> Self {
        RegistryConfig {
            meta: Meta {
                name: name.to_owned(),
            },
            mirror: Some(Mirror {
                source: source.to_owned(),
                sync: true,
                index_update_interval: 60u32,
            }),
            http: HttpConfig::default(),
            access: AccessControl::default(),
        }
    }

    pub fn name(&self) -> &str {
        &self.meta.name
    }

    pub fn domain(&self) -> &str {
        &self.http.domain
    }

    pub fn ip(&self) -> IpAddr {
        self.http.ip
    }

    pub fn port(&self) -> u16 {
        self.http.port
    }

    pub fn ssl(&self) -> bool {
        self.http.ssl
    }

    pub fn receive_on(&self) -> bool {
        self.access.receive
    }

    pub fn upload_on(&self) -> bool {
        self.access.upload
    }

    pub fn source(&self) -> Option<&str> {
        match &self.mirror {
            Some(mirror) => {
                Some(&mirror.source)
            }
            None => None
        }
    }

    pub fn build_ssl_config(&self) -> SkrdResult<ServerConfig> {
        let mut config = ServerConfig::new(NoClientAuth::new());

        let cert_file = &mut BufReader::new(File::open(&self.http.cert)?);
        let key_file = &mut BufReader::new(File::open(&self.http.key)?);
        let cert_chain = certs(cert_file)
            .map_err(|_| rustls::TLSError::General("Extract certificates error".to_owned()))?;
        let mut keys = rsa_private_keys(key_file)
            .map_err(|_| rustls::TLSError::General("Extract RSA private keys error".to_owned()))?;
        config.set_single_cert(cert_chain, keys.remove(0))?;

        Ok(config)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Meta {
    name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Mirror {
    source: String,
    sync: bool,
    #[serde(rename = "index-update-interval")]
    index_update_interval: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct HttpConfig {
    domain: String,
    ip: IpAddr,
    port: u16,
    ssl: bool,
    cert: PathBuf,
    key: PathBuf,
}

impl Default for HttpConfig {
    fn default() -> Self {
        HttpConfig {
            domain: "localhost".to_owned(),
            ip: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            port: 80,
            ssl: false,
            cert: PathBuf::new(),
            key: PathBuf::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AccessControl {
    #[serde(rename = "git-receive-pack")]
    receive: bool,
    #[serde(rename = "git-upload-pack")]
    upload: bool,
}

impl Default for AccessControl {
    fn default() -> Self {
        AccessControl {
            receive: true,
            upload: true,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct UrlConfig {
    pub dl: String,
    pub api: String,
}

impl From<&Registry> for UrlConfig {
    fn from(registry: &Registry) -> Self {
        UrlConfig {
            dl: format!("{}{}", registry.base_url(), "/api/v1/crates"),
            api: registry.base_url(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Crate {
    pub name: String,
    #[serde(rename = "vers")]
    pub version: String,
    #[serde(rename = "cksum")]
    pub checksum: String,
    pub yanked: bool,
}

fn is_default_port(port: u16, ssl: bool) -> bool {
    if ssl {
        port == 443u16
    } else {
        port == 80u16
    }
}
