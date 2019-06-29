use crate::error::{SkrdError, SkrdResult};
use rustls::internal::pemfile::{certs, rsa_private_keys};
use rustls::{NoClientAuth, ServerConfig};
use serde_derive::{Deserialize, Serialize};
use std::io::BufReader;
use std::net::IpAddr;
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

        let config = RegistryConfig::open(root.join(Registry::TOML_FILE))?;

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

    pub fn base_url(&self) -> String {
        let config = self.config();
        format!(
            "{}://{}{}/{}",
            if config.ssl() { "http" } else { "http" },
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
    http: HttpConfig,
    access: AccessControl,
}

impl RegistryConfig {
    fn open<P: Into<PathBuf>>(path: P) -> SkrdResult<Self> {
        let mut file = File::open(path.into())?;
        let mut content = String::with_capacity(file.metadata()?.len() as usize);
        file.read_to_string(&mut content)?;

        Ok(toml::from_str::<RegistryConfig>(&content)?)
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
struct HttpConfig {
    domain: String,
    ip: IpAddr,
    port: u16,
    ssl: bool,
    cert: PathBuf,
    key: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AccessControl {
    #[serde(rename = "git-receive-pack")]
    receive: bool,
    #[serde(rename = "git-upload-pack")]
    upload: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct UrlConfig {
    pub dl: String,
    pub api: String,
}

fn is_default_port(port: u16, ssl: bool) -> bool {
    if ssl {
        port == 443u16
    } else {
        port == 80u16
    }
}
