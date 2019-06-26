use actix_web::Responder;
use actix_web::{guard, http::header, web, App, HttpRequest, HttpResponse, HttpServer};
use rustls::{
    internal::pemfile::{certs, rsa_private_keys},
    NoClientAuth, ServerConfig,
};
use std::fmt::Debug;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::{fs::File, io::BufReader, process::Command as PsCommand};
use structopt::StructOpt;

use crate::{
    error::{SkrdError, SkrdResult},
    util::{get_service_from_query_string, sock_addr_v4},
};

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
                    web::scope("/pack").route("/{file}", web::get().to(get_pack_file)), //                .route("/{file:pack-[0-9a-f]{40}\\.idx}", web::get().to(get_index_file))
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

fn git_upload_pack(request: HttpRequest, index_path: web::Data<PathBuf>) -> HttpResponse {
    HttpResponse::Ok().finish()
}

fn git_receive_pack(request: HttpRequest, index_path: web::Data<PathBuf>) -> HttpResponse {
    HttpResponse::Ok().finish()
}

// http://localhost/crates.io-index/info/refs?service=git-xxxxxx-pack
fn get_info_refs(request: HttpRequest, index_path: web::Data<PathBuf>) -> HttpResponse {
    // TODO: check Content-Type: application/x-git-xxxxx-pack-request
    match get_service_from_query_string(request.query_string()) {
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

fn get_alternates(request: HttpRequest, index_path: web::Data<PathBuf>) -> HttpResponse {
    HttpResponse::Ok().body(format!("get alternates: {}", request.path()))
}

fn get_http_alternates(request: HttpRequest, index_path: web::Data<PathBuf>) -> HttpResponse {
    HttpResponse::Ok().body(format!("get http alternates: {}", request.path()))
}

fn get_info_packs(request: HttpRequest, index_path: web::Data<PathBuf>) -> HttpResponse {
    HttpResponse::Ok().body(format!("get info packs: {}", request.path()))
}

fn get_info_file(request: HttpRequest, path: web::Path<String>) -> HttpResponse {
    HttpResponse::Ok().body(format!("get info file: {}", request.path()))
}

fn get_loose_object(
    request: HttpRequest,
    path: web::Path<(String, String)>,
    index_path: web::Data<PathBuf>,
) -> HttpResponse {
    HttpResponse::Ok().body(format!("get loose object packs: {}", request.path()))
}

fn get_index_file(request: HttpRequest, index_path: web::Data<PathBuf>) -> HttpResponse {
    HttpResponse::Ok().body(format!("get index file: {}", request.path()))
}

fn get_pack_file(request: HttpRequest, index_path: web::Data<PathBuf>) -> HttpResponse {
    HttpResponse::Ok().body(format!("get pack file: {}", request.path()))
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
