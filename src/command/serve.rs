use actix_web::{
    guard, http::header, middleware::DefaultHeaders, web, App, HttpRequest, HttpResponse,
    HttpServer, Responder,
};
use rustls::{
    internal::pemfile::{certs, rsa_private_keys},
    NoClientAuth, ServerConfig,
};
use std::fmt::Debug;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::{fs::File, io::BufReader, process::Command as PsCommand};
use structopt::StructOpt;

use crate::{
    error::{SkrdError, SkrdResult},
    util::{cache_forever, get_service_from_query_string, no_cache, sock_addr_v4},
};
use std::io::Read;

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
                .wrap(DefaultHeaders::new().header(
                    "server",
                    format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
                ))
                .service(api_scope())
                .service(index_scope(&index_path))
                .service(crates_scope(&crates_path))
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
                let addr = self.addr.unwrap_or_else(|| sock_addr_v4(127, 0, 0, 1, 443));
                server.bind_rustls(addr, cert_key.config.clone())?.start();
                info!(
                    "Registry server started at https://{}:{}.",
                    addr.ip(),
                    addr.port()
                );
            }
            None => {
                let addr = self.addr.unwrap_or_else(|| sock_addr_v4(127, 0, 0, 1, 80));
                server.bind(addr)?.start();
                info!(
                    "Registry server started at http://{}:{}.",
                    addr.ip(),
                    addr.port()
                );
            }
        }
        sys.run()?;
        Ok(())
    }
}

fn api_scope() -> actix_web::Scope {
    web::scope("/api/v1").route(
        "/crates/{name}/{version}/download",
        web::get().to(redirect_download),
    )
}

fn index_scope<P: Into<PathBuf>>(index_path: P) -> actix_web::Scope {
    web::scope("/crates.io-index")
        .route("/git-upload-pack", web::post().to(git_upload_pack))
        .route("/git-receive-pack", web::post().to(git_receive_pack))
        .route("/info/refs", web::get().to(get_info_refs))
        .route("/HEAD", web::get().to(get_head))
        .service(
            web::scope("/objects")
                .service(
                    web::scope("/info")
                        .route("/alternates", web::get().to(get_alternates))
                        .route("/http-alternates", web::get().to(get_http_alternates))
                        .route("/packs", web::get().to(get_info_packs))
                        .route("/{file}", web::get().to(get_info_file)),
                )
                .service(
                    web::scope("/pack").route("/{file}", web::get().to(get_pack_or_index_file)), //.route("/{file:pack-[0-9a-f]{40}\\.idx}", web::get().to(get_index_file))
                )
                .route(
                    "/{dir:[0-9a-f]{2}}/{file:[0-9a-f]{38}}",
                    web::get().to(get_loose_object),
                ),
        )
        .default_service(
            actix_files::Files::new("", index_path.into()) // http://localhost/crates.io-index/to/ki/tokio
                .show_files_listing(),
        )
}

fn crates_scope<P: Into<PathBuf>>(crates_path: P) -> actix_web::Scope {
    web::scope("/crates")
        .service(actix_files::Files::new("/", crates_path.into()).show_files_listing())
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
    HttpResponse::NotFound().finish()
}

fn git_upload_pack(_request: HttpRequest, _index_path: web::Data<PathBuf>) -> HttpResponse {
    HttpResponse::Ok().finish()
}

fn git_receive_pack(_request: HttpRequest, _index_path: web::Data<PathBuf>) -> HttpResponse {
    HttpResponse::Ok().finish()
}

// http://localhost/crates.io-index/info/refs?service=git-xxxxxx-pack
// TODO: # git command clone error
//       > git clone http://localhost/crates.io-index
//       Cloning into 'crates.io-index'...
//       fatal: http://localhost/crates.io-index/info/refs not valid: is this a git repository?
fn get_info_refs(
    request: HttpRequest,
    index_path: web::Data<PathBuf>,
) -> std::io::Result<impl Responder> {
    // TODO: check Content-Type: application/x-git-xxxxx-pack-request
    match get_service_from_query_string(request.query_string()) {
        Some(service) => {
            // TODO: configurable permission
            if service != "upload-pack" && service != "receive-pack" {
                return Ok(no_cache(HttpResponse::NotFound().finish()));
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

                    Ok(no_cache(
                        HttpResponse::Ok()
                            .content_type(format!("application/x-{}-advertisement", service))
                            .body(body),
                    ))
                }
                Err(e) => {
                    error!("{} service error: {}", service, e);
                    Ok(no_cache(HttpResponse::NotFound().finish()))
                }
            }
        }
        _ => {
            let mut body = String::new();
            File::open(index_path.get_ref().join(".git/info/refs"))?.read_to_string(&mut body)?;
            Ok(no_cache(
                HttpResponse::Ok()
                    .content_type(mime::TEXT_PLAIN_UTF_8.to_string())
                    .body(body),
            ))
        }
    }
}

fn get_head(index_path: web::Data<PathBuf>) -> std::io::Result<impl Responder> {
    send_text_file(index_path, "HEAD")
}

fn get_alternates(index_path: web::Data<PathBuf>) -> std::io::Result<impl Responder> {
    send_text_file(index_path, "objects/info/alternates")
}

fn get_http_alternates(index_path: web::Data<PathBuf>) -> std::io::Result<impl Responder> {
    send_text_file(index_path, "objects/info/http-alternates")
}

fn get_info_packs(index_path: web::Data<PathBuf>) -> std::io::Result<impl Responder> {
    send_file_with_custom_mime(
        "text/plain; charset=utf-8",
        index_path,
        "objects/info/packs",
    )
    .map(cache_forever)
}

fn get_info_file(
    index_path: web::Data<PathBuf>,
    path: web::Path<String>,
) -> std::io::Result<impl Responder> {
    send_text_file(index_path, &format!("objects/info/{}", path))
}

// http://localhost/crates.io-index/objects/2f/d95367332005518f56b336634d85c099e2678a
fn get_loose_object(
    path: web::Path<(String, String)>,
    index_path: web::Data<PathBuf>,
) -> std::io::Result<impl Responder> {
    send_file_with_custom_mime(
        "application/x-git-loose-object",
        index_path,
        &format!("objects/{}/{}", path.0, path.1),
    )
    .map(cache_forever)
}

// http://localhost/crates.io-index/objects/pack/pack-63c9d4a58e9d4e29c97b1afdd26c1d39be6c7d10.idx
// http://localhost/crates.io-index/objects/pack/pack-63c9d4a58e9d4e29c97b1afdd26c1d39be6c7d10.pack
fn get_pack_or_index_file(
    request: HttpRequest,
    path: web::Path<String>,
    index_path: web::Data<PathBuf>,
) -> std::io::Result<impl Responder> {
    let url_path = request.path();
    let content_type = if url_path.ends_with(".idx") {
        "application/x-git-packed-objects-toc"
    } else if url_path.ends_with(".pack") {
        "application/x-git-packed-objects"
    } else {
        return Err(std::io::ErrorKind::NotFound.into());
    };

    send_file_with_custom_mime(content_type, index_path, &format!("objects/pack/{}", path))
        .map(cache_forever)
}

fn send_text_file(
    index_path: web::Data<PathBuf>,
    filename: &str,
) -> std::io::Result<impl Responder> {
    send_file(mime::TEXT_PLAIN, index_path.get_ref().join(filename)).map(no_cache)
}

fn send_file<P: AsRef<Path>>(
    content_type: mime::Mime,
    filepath: P,
) -> std::io::Result<impl Responder> {
    Ok(actix_files::NamedFile::open(filepath)?
        .set_content_type(content_type)
        .use_last_modified(true))
}

// say goodbye to strongly typed mime
fn send_file_with_custom_mime(
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
