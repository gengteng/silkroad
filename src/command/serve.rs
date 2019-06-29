use actix_http::http::Version;
use actix_web::{
    guard,
    http::header,
    middleware::{DefaultHeaders, Logger},
    web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use std::{
    fs::File,
    io::{ErrorKind, Read},
    net::SocketAddr,
    path::{Path, PathBuf},
    process::Command as PsCommand,
};
use structopt::StructOpt;

use crate::util::write_config_json;
use crate::{
    error::SkrdResult,
    registry::Registry,
    util::{cache_forever, get_service_from_query_string, no_cache},
};

#[derive(Debug, StructOpt)]
#[structopt(name = "serve")]
pub struct Serve {
    #[structopt(
        long = "registry-path",
        short = "p",
        help = "Set the registry path",
        value_name = "PATH",
        parse(try_from_str)
    )]
    registry: Registry,
}

impl Serve {
    pub fn serve(&self) -> SkrdResult<()> {
        let config = self.registry.config();

        info!("Registry '{}' loaded.", config.name());
        info!(
            "Access Control => git-receive-pack: {}, git-upload-pack: {}",
            config.receive_on(),
            config.upload_on()
        );

        let sys = actix_rt::System::new("silk_road");

        let reg = self.registry.clone();

        let server = HttpServer::new(move || {
            App::new()
                .data(reg.clone())
                .wrap(Logger::default())
                .wrap(DefaultHeaders::new().header(
                    "server",
                    format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
                ))
                .service(
                    web::scope(&("/".to_owned() + reg.config().name()))
                        .service(api_scope())
                        .service(index_scope(reg.index_path()))
                        .service(crates_scope(reg.crates_path())),
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

        let addr = SocketAddr::new(config.ip(), config.port());

        if config.ssl() {
            server
                .bind_rustls(addr, config.build_ssl_config()?)?
                .start();
        } else {
            server.bind(addr)?.start();
        };

        write_config_json(&self.registry).and_then(|o| {
            if let Some(oid) = o {
                info!(
                    "Custom url(dl and api) has been written to config.json.(commid id: {})",
                    oid
                );
            }

            Ok(o)
        })?;

        info!("Registry server started.");
        info!(
            "Users need to add this source to Cargo's configuration => {}/index",
            self.registry.base_url()
        );

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
    web::scope("/index")
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
fn redirect_download(
    registry: web::Data<Registry>,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let name = &path.0;
    let version = &path.1;

    let location = match name.len() {
        1 => format!(
            "/{}/crates/{}/{}/{}-{}.crate",
            registry.config().name(),
            1,
            name,
            name,
            version
        ),
        2 => format!(
            "/{}/crates/{}/{}/{}-{}.crate",
            registry.config().name(),
            2,
            name,
            name,
            version
        ),
        3 => format!(
            "/{}/crates/{}/{}/{}/{}-{}.crate",
            registry.config().name(),
            3,
            &name[..1],
            name,
            name,
            version
        ),
        _ => format!(
            "/{}/crates/{}/{}/{}/{}-{}.crate",
            registry.config().name(),
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
fn return_404() -> HttpResponse {
    HttpResponse::NotFound().finish()
}

// TODO: git_upload_pack
fn git_upload_pack(_request: HttpRequest, _registry: web::Data<Registry>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/x-git-upload-pack-result")
        .finish()
}

// TODO: git_upload_pack
fn git_receive_pack(_request: HttpRequest, _registry: web::Data<Registry>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/x-git-receive-pack-result")
        .finish()
}

// http://localhost:9090/crates.io-index/info/refs?service=git-upload-pack
// TODO: Now it only works using The Dumb Protocol ('git-fetch-with-cli = true')
//       refer to:  1. https://git-scm.com/book/en/v2/Git-Internals-Transfer-Protocols
//                  2. https://doc.rust-lang.org/cargo/reference/config.html#configuration-keys
//       The Smart Protocol is needed.
fn get_info_refs(
    request: HttpRequest,
    registry: web::Data<Registry>,
) -> std::io::Result<impl Responder> {
    // TODO: check Content-Type: application/x-git-xxxxx-pack-request
    match get_service_from_query_string(request.query_string()) {
        Some(service) => {
            let is_upload_pack = service == "upload-pack";
            let is_receive_pack = service == "receive-pack";

            // validate
            if !is_upload_pack && !is_receive_pack {
                return Ok(no_cache(
                    if request.version() == Version::HTTP_11 {
                        HttpResponse::MethodNotAllowed()
                    } else {
                        HttpResponse::BadRequest()
                    }
                    .finish(),
                ));
            }

            // access control
            if (is_upload_pack && !registry.config().upload_on())
                || (is_receive_pack && !registry.config().receive_on())
            {
                return Ok(no_cache(
                    HttpResponse::Ok()
                        .content_type(mime::TEXT_PLAIN_UTF_8.to_string())
                        .body(update_and_get_refs(registry)?),
                ));
            }

            // execute the git command
            // TODO: no dependency on `git`
            let result = PsCommand::new("git")
                .arg(service)
                .arg(registry.index_path())
                .arg("--stateless-rpc")
                .arg("--advertise-refs") // exit immediately after initial ref advertisement
                .output();

            match result {
                Ok(output) => {
                    let head = format!("# service=git-{}\n", service);
                    let head_len = format!("{:04x}", head.len() + 4);
                    match String::from_utf8(output.stdout) {
                        Ok(content) => Ok(no_cache(
                            HttpResponse::Ok()
                                .content_type(format!(
                                    "application/x-git-{}-advertisement",
                                    service
                                ))
                                .body(format!("{}{}0000{}", head_len, head, content)),
                        )),
                        Err(e) => {
                            error!("{} service error: {}", service, e);
                            Ok(no_cache(HttpResponse::NotFound().finish()))
                        }
                    }
                }
                Err(e) => {
                    error!("{} service error: {}", service, e);
                    Ok(no_cache(HttpResponse::NotFound().finish()))
                }
            }
        }
        _ => Ok(no_cache(
            HttpResponse::Ok()
                .content_type(mime::TEXT_PLAIN_UTF_8.to_string())
                .body(update_and_get_refs(registry)?),
        )),
    }
}

fn update_and_get_refs(registry: web::Data<Registry>) -> std::io::Result<String> {
    let status = PsCommand::new("git")
        .current_dir(registry.index_path())
        .arg("update-server-info") // exit immediately after initial ref advertisement
        .status()?;

    if status.success() {
        let ref_path = registry.index_git_path().join("info/refs");
        let mut body = String::new();
        let mut file = File::open(&ref_path)?;

        // TODO: optimize
        file.read_to_string(&mut body)?;
        Ok(body)
    } else {
        Err(std::io::Error::new(
            ErrorKind::Other,
            "git upload-server-info error",
        ))
    }
}

fn get_head(registry: web::Data<Registry>) -> std::io::Result<impl Responder> {
    send_text_file(registry.index_path().join("HEAD"))
}

fn get_alternates(registry: web::Data<Registry>) -> std::io::Result<impl Responder> {
    send_text_file(registry.index_path().join("objects/info/alternates"))
}

fn get_http_alternates(registry: web::Data<Registry>) -> std::io::Result<impl Responder> {
    send_text_file(registry.index_path().join("objects/info/http-alternates"))
}

fn get_info_packs(registry: web::Data<Registry>) -> std::io::Result<impl Responder> {
    send_file_with_custom_mime(
        "text/plain; charset=utf-8",
        registry.index_git_path().join("objects/info/packs"),
    )
    .map(cache_forever)
}

fn get_info_file(
    registry: web::Data<Registry>,
    path: web::Path<String>,
) -> std::io::Result<impl Responder> {
    send_text_file(registry.index_path().join(format!("objects/info/{}", path)))
}

// http://localhost/crates.io-index/objects/2f/d95367332005518f56b336634d85c099e2678a
fn get_loose_object(
    path: web::Path<(String, String)>,
    registry: web::Data<Registry>,
) -> std::io::Result<impl Responder> {
    send_file_with_custom_mime(
        "application/x-git-loose-object",
        registry
            .index_git_path()
            .join(format!("objects/{}/{}", path.0, path.1)),
    )
    .map(cache_forever)
}

// http://localhost/crates.io-index/objects/pack/pack-63c9d4a58e9d4e29c97b1afdd26c1d39be6c7d10.idx
// http://localhost/crates.io-index/objects/pack/pack-63c9d4a58e9d4e29c97b1afdd26c1d39be6c7d10.pack
fn get_pack_or_index_file(
    request: HttpRequest,
    file: web::Path<String>,
    registry: web::Data<Registry>,
) -> std::io::Result<impl Responder> {
    let url_path = request.path();
    let content_type = if url_path.ends_with(".idx") {
        "application/x-git-packed-objects-toc"
    } else if url_path.ends_with(".pack") {
        "application/x-git-packed-objects"
    } else {
        return Err(std::io::ErrorKind::NotFound.into());
    };

    send_file_with_custom_mime(
        content_type,
        registry
            .index_git_path()
            .join(format!("objects/pack/{}", file)),
    )
    .map(cache_forever)
}

fn send_text_file<P: AsRef<Path>>(filepath: P) -> std::io::Result<impl Responder> {
    send_file(mime::TEXT_PLAIN, filepath).map(no_cache)
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
fn send_file_with_custom_mime<P: AsRef<Path>>(
    content_type: &str,
    filepath: P,
) -> std::io::Result<impl Responder> {
    Ok(actix_files::NamedFile::open(filepath)?
        .use_last_modified(true)
        .with_header("Content-Type", content_type))
}
