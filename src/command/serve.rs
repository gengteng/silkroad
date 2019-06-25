use actix_web::{guard, http::header, web, App, HttpRequest, HttpResponse, HttpServer};
use rustls::{
    internal::pemfile::{certs, rsa_private_keys},
    NoClientAuth, ServerConfig,
};
use std::{fs::File, io::BufReader, process::Command as PsCommand};

use crate::error::{SkrdError, SkrdResult};
use actix_web::Responder;
use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "serve")]
pub struct Serve {
    #[structopt(
        long,
        short = "a",
        help = "Set the listening address",
        value_name = "IP:PORT",
        parse(try_from_str)
    )]
    addr: Option<SocketAddr>,

    #[structopt(
        long = "index-path",
        short = "i",
        help = "Set the index path",
        value_name = "PATH",
        parse(try_from_str)
    )]
    index_path: PathBuf,

    #[structopt(
        long = "crates-path",
        short = "c",
        help = "Set the crates path",
        value_name = "PATH",
        parse(try_from_str)
    )]
    crates_path: PathBuf,

    #[structopt(
        long = "ssl-files",
        short = "s",
        help = "Set the certificate path and the RSA private key path",
        value_name = "CERTPATH,KEYPATH",
        parse(try_from_str)
    )]
    keys: Option<CertAndKey>,
}

struct CertAndKey {
    raw: String,
    config: ServerConfig,
}

impl Debug for CertAndKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "CertAndKey {{ {:?} }}", self.raw)
    }
}

impl std::str::FromStr for CertAndKey {
    type Err = SkrdError;

    fn from_str(s: &str) -> SkrdResult<Self> {
        let raw = s.to_owned();
        match s.find(',') {
            Some(n) => {
                let mut config = ServerConfig::new(NoClientAuth::new());

                let cert_file = &mut BufReader::new(File::open(&s[..n])?);
                let key_file = &mut BufReader::new(File::open(&s[n + 1..])?);
                let cert_chain = certs(cert_file).map_err(|_| {
                    rustls::TLSError::General("Extract certificates error".to_owned())
                })?;
                let mut keys = rsa_private_keys(key_file).map_err(|_| {
                    rustls::TLSError::General("Extract RSA private keys error".to_owned())
                })?;
                config.set_single_cert(cert_chain, keys.remove(0))?;

                Ok(CertAndKey { raw, config })
            }
            None => Err(SkrdError::Custom("Ssl-files format error".to_owned())),
        }
    }
}

impl Serve {
    pub fn serve(&self) -> SkrdResult<()> {
        let sys = actix_rt::System::new("silk_road");

        let index_path = self.index_path.clone();
        let crates_path = self.crates_path.clone();

        let server = HttpServer::new(move || {
            App::new()
                .data(index_path.clone())
                .service(
                    web::resource("/api/v1/crates/{name}/{version}/download")
                        .route(web::get().to(redirect_download)),
                )
                .service(
                    actix_files::Files::new("/crates", crates_path.clone()).show_files_listing(),
                )
                .service(web::resource("/crates.io-index/info/refs").to(get_info_refs))
                .service(web::resource("/crates.io-index/HEAD").to(get_head))
                .service(
                    actix_files::Files::new("/crates.io-index", &index_path) // http://localhost/crates.io-index/to/ki/tokio
                        .show_files_listing(),
                )
                .default_service(
                    web::resource("")
                        .route(web::get().to(return_404))
                        // all requests that are not `GET`
                        .route(
                            web::route()
                                .guard(guard::Not(guard::Get()))
                                .to(HttpResponse::MethodNotAllowed),
                        ),
                )
        });

        match &self.keys {
            Some(cert_key) => {
                let addr = self.addr.unwrap_or(sock_addr_v4(127, 0, 0, 1, 443));
                server.bind_rustls(addr, cert_key.config.clone())?.start();
                info!("Registry started at https://{}:{}.", addr.ip(), addr.port());
            }
            None => {
                let addr = self.addr.unwrap_or(sock_addr_v4(127, 0, 0, 1, 80));
                server.bind(addr)?.start();
                info!("Registry started at http://{}:{}.", addr.ip(), addr.port());
            }
        }
        sys.run()?;
        Ok(())
    }
}

fn sock_addr_v4(a: u8, b: u8, c: u8, d: u8, port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(a, b, c, d)), port)
}

// /api/v1/crates/tokio/0.1.21/download
fn redirect_download(path: web::Path<(String, String)>) -> HttpResponse {
    let name = &path.0;
    let version = &path.1;

    let location = match name.len() {
        1 => format!("/crates/{}/{}/{}-{}.crate", 1, name, name, version),
        2 => format!("/crates/{}/{}/{}-{}.crate", 2, name, name, version),
        3 => format!(
            "/crates/{}/{}/{}/{}-{}.crate",
            3,
            &name[..1],
            name,
            name,
            version
        ),
        _ => format!(
            "/crates/{}/{}/{}/{}-{}.crate",
            &name[..2],
            &name[2..4],
            name,
            name,
            version
        ),
    };

    HttpResponse::Found()
        .header(header::LOCATION, location)
        .finish()
}

/// 404 handler
fn return_404(request: HttpRequest) -> HttpResponse {
    info!(
        "REQ: {:?} {}:{} <= RESP: 404 Not Found",
        request.version(),
        request.method(),
        request.path()
    );
    debug!("request:{:?}", request);
    HttpResponse::NotFound().finish()
}

fn get_service(query: &str) -> Option<&str> {
    let head = "service=git-";
    query.find(head).and_then(|i| {
        let start = i + head.len();
        match &query[start..].find('&') {
            Some(u) => Some(&query[start..u + start]),
            None => Some(&query[start..]),
        }
    })
}

// http://localhost/crates.io-index/info/refs?service=git-xxxxxx-pack
fn get_info_refs(request: HttpRequest, index_path: web::Data<PathBuf>) -> HttpResponse {
    // TODO: check Content-Type: application/x-git-xxxxx-pack-request
    match get_service(request.query_string()) {
        Some(service) => {
            // TODO: configurable permission
            if service != "upload-pack" && service != "receive-pack" {
                return HttpResponse::NotFound().finish();
            }

            let result = PsCommand::new("git")
                .arg(service)
                .arg(index_path.get_ref())
                .arg("--advertise-refs") // exit immediately after initial ref advertisement
                .output();

            match result {
                Ok(mut output) => {
                    let mut body = Vec::from(format!("001e# service={}\n", service));
                    body.append(&mut output.stdout);

                    HttpResponse::Ok()
                        .content_type(format!("application/x-{}-advertisement", service))
                        .body(body)
                }
                Err(e) => {
                    error!("{} service error: {}", service, e);
                    HttpResponse::InternalServerError().finish()
                }
            }
        }
        _ => {
            error!("get service error: {:?}", request);
            HttpResponse::NotFound().finish()
        }
    }
}

fn get_head(
    request: HttpRequest,
    index_path: web::Data<PathBuf>,
) -> std::io::Result<impl Responder> {
    get_text_file(request, index_path, "HEAD")
}

// /objects/info/packs
fn get_info_packs(request: HttpRequest, index_path: web::Data<PathBuf>) -> HttpResponse {
    HttpResponse::NotFound().finish()
}

fn get_loose_object(request: HttpRequest, index_path: web::Data<PathBuf>) -> HttpResponse {
    HttpResponse::NotFound().finish()
}

fn get_pack_file(request: HttpRequest, index_path: web::Data<PathBuf>) -> HttpResponse {
    HttpResponse::NotFound().finish()
}

fn get_index_file(
    request: HttpRequest,
    index_path: web::Data<PathBuf>,
    filename: &str,
) -> HttpResponse {
    HttpResponse::NotFound().finish()
}

fn get_text_file(
    request: HttpRequest,
    index_path: web::Data<PathBuf>,
    filename: &str,
) -> std::io::Result<impl Responder> {
    send_file(mime::TEXT_PLAIN, index_path, filename)
}

fn send_file(
    content_type: mime::Mime,
    index_path: web::Data<PathBuf>,
    filename: &str,
) -> std::io::Result<impl Responder> {
    Ok(
        actix_files::NamedFile::open(index_path.get_ref().to_owned().join(".git").join(filename))?
            .set_content_type(content_type)
            .use_last_modified(true),
    )
}

// say goodbye to strongly typed mime
fn send_file_without_strongly_typed_mime(
    content_type: &str,
    index_path: web::Data<PathBuf>,
    filename: &str,
) -> std::io::Result<impl Responder> {
    Ok(
        actix_files::NamedFile::open(index_path.get_ref().to_owned().join(".git").join(filename))?
            .use_last_modified(true)
            .with_header("Content-Type", content_type),
    )
}
