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

use crate::error::SkrdError;
use crate::util::{get_crate_path, write_config_json};
use crate::{
    error::SkrdResult,
    registry::Registry,
    util::{cache_forever, get_service_from_query_string, no_cache},
};
use actix_http::httpmessage::HttpMessage;
use mime::Mime;
use std::io::Write;
use std::process::Stdio;
use std::str::FromStr;

#[derive(Debug, StructOpt)]
#[structopt(name = "serve")]
pub struct Serve {
    #[structopt(
        help = "Set the registry path",
        value_name = "REGISTRY PATH",
        parse(try_from_str)
    )]
    registry: Option<Registry>,
}

impl Serve {
    pub fn serve(self) -> SkrdResult<()> {
        // if registry is not specified, try current directory
        let registry = if let Some(registry) = self.registry {
            registry
        } else {
            let current_dir = std::env::current_dir()?;
            Registry::open(current_dir)?
        };

        let config = registry.config();
        info!("Registry '{}' loaded.", config.name());
        info!(
            "Access Control => git-receive-pack: {}, git-upload-pack: {}",
            config.receive_on(),
            config.upload_on()
        );

        let sys = actix_rt::System::new("silk_road");

        // HttpServer shared data
        let reg = registry.clone();
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

        write_config_json(&registry).and_then(|o| {
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
            registry.base_url()
        );

        sys.run()?;
        Ok(())
    }
}

fn api_scope() -> actix_web::Scope {
    web::scope("/api/v1/crates")
        .service(web::resource("").route(web::get().to(search)))
        .route("/new", web::put().to(publish))
        .service(
            web::scope("/{name}")
                .service(
                    web::resource("/owners")
                        .route(web::get().to(get_owners))
                        .route(web::put().to(add_owners))
                        .route(web::delete().to(delete_owners)),
                )
                .service(
                    web::scope("/{version}")
                        .route("/download", web::get().to(redirect_download))
                        .route("/yank", web::put().to(yank))
                        .route("/unyank", web::put().to(unyank)),
                ),
        )
}

fn index_scope<P: Into<PathBuf>>(index_path: P) -> actix_web::Scope {
    web::scope("/index")
        .route("/git-upload-pack", web::post().to_async(git_upload_pack))
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

    let location = format!(
        "/{}/crates/{}",
        registry.config().name(),
        get_crate_path(name, version)
    );

    HttpResponse::Found()
        .header(header::LOCATION, location)
        .finish()
}

/// 404 handler
fn return_404() -> HttpResponse {
    HttpResponse::NotFound().finish()
}

fn git_upload_pack(
    request: HttpRequest,
    body: web::Bytes,
    registry: web::Data<Registry>,
) -> SkrdResult<HttpResponse> {
    if request.content_type() != "application/x-git-upload-pack-request" {
        return Ok(HttpResponse::Forbidden().finish());
    }

    if !registry.config().upload_on() {
        return Ok(HttpResponse::Forbidden().finish());
    }

    let mut child = PsCommand::new("git")
        .arg("upload-pack")
        .arg(registry.index_path())
        .arg("--stateless-rpc")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| SkrdError::StaticCustom("get command line stdin error"))?;
        stdin.write_all(&body)?;
    }

    let output = child.wait_with_output()?;
    Ok(HttpResponse::Ok()
        .content_type("application/x-git-upload-pack-result")
        .body(output.stdout))
}

// TODO: git_receive_pack
fn git_receive_pack(request: HttpRequest) -> SkrdResult<HttpResponse> {
    info!("{:?}", request);
    Ok(HttpResponse::Ok()
        .content_type("application/x-git-receive-pack-result")
        .finish())
}

// http://localhost:9090/crates.io-index/info/refs?service=git-upload-pack
fn get_info_refs(
    request: HttpRequest,
    registry: web::Data<Registry>,
) -> SkrdResult<impl Responder> {
    match get_service_from_query_string(request.query_string()) {
        Some(service) => {
            let is_upload_pack = service == "upload-pack";
            let is_receive_pack = service == "receive-pack";

            // access control
            if (!is_upload_pack && !is_receive_pack) // from query string
                || (is_upload_pack && !registry.config().upload_on()) // from registry config(.toml)
                || (is_receive_pack && !registry.config().receive_on())
            {
                return Ok(no_cache(
                    HttpResponse::Ok()
                        .content_type(mime::TEXT_PLAIN_UTF_8.to_string())
                        .body(update_and_get_refs(&registry)?),
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
                .body(update_and_get_refs(&registry)?),
        )),
    }
}

fn update_and_get_refs(registry: &Registry) -> SkrdResult<String> {
    let wd = walkdir::WalkDir::new(registry.index_git_path().join("refs"));
    let mut refs = String::with_capacity(512);
    let mut first = true;
    for result in wd {
        match result {
            Ok(entry) => match entry.metadata() {
                Ok(meta) => {
                    if !meta.is_file() {
                        continue;
                    }

                    match entry.path().strip_prefix(registry.index_git_path()) {
                        Ok(path) => {
                            if !first {
                                refs.push('\n');
                            }
                            first = false;

                            let mut file = File::open(entry.path())?;
                            let mut buff = [0u8; 40];
                            file.read_exact(&mut buff)?;
                            drop(file);

                            refs.push_str(&String::from_utf8(buff.to_vec())?);
                            refs.push('\t');
                            refs.push_str(&path.display().to_string());
                        }
                        Err(_) => {}
                    }
                }
                Err(_) => {}
            },
            Err(_) => {}
        }
    }

    Ok(refs)
}

fn get_head(registry: web::Data<Registry>) -> SkrdResult<impl Responder> {
    send_text_file(registry.index_path().join("HEAD"))
}

fn get_alternates(registry: web::Data<Registry>) -> SkrdResult<impl Responder> {
    send_text_file(registry.index_path().join("objects/info/alternates"))
}

fn get_http_alternates(registry: web::Data<Registry>) -> SkrdResult<impl Responder> {
    send_text_file(registry.index_path().join("objects/info/http-alternates"))
}

fn get_info_packs(registry: web::Data<Registry>) -> SkrdResult<impl Responder> {
    send_file_with_custom_mime(
        "text/plain; charset=utf-8",
        registry.index_git_path().join("objects/info/packs"),
    )
    .map(cache_forever)
}

fn get_info_file(
    registry: web::Data<Registry>,
    path: web::Path<String>,
) -> SkrdResult<impl Responder> {
    send_text_file(registry.index_path().join(format!("objects/info/{}", path)))
}

// http://localhost/crates.io-index/objects/2f/d95367332005518f56b336634d85c099e2678a
fn get_loose_object(
    path: web::Path<(String, String)>,
    registry: web::Data<Registry>,
) -> SkrdResult<impl Responder> {
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
) -> SkrdResult<impl Responder> {
    let url_path = request.path();
    let content_type = if url_path.ends_with(".idx") {
        "application/x-git-packed-objects-toc"
    } else if url_path.ends_with(".pack") {
        "application/x-git-packed-objects"
    } else {
        return Err(SkrdError::Custom(format!(
            "error file extension: {}",
            url_path
        )));
    };

    send_file_with_custom_mime(
        content_type,
        registry
            .index_git_path()
            .join(format!("objects/pack/{}", file)),
    )
    .map(cache_forever)
}

fn send_text_file<P: AsRef<Path>>(filepath: P) -> SkrdResult<impl Responder> {
    send_file(mime::TEXT_PLAIN, filepath).map(no_cache)
}

fn send_file<P: AsRef<Path>>(content_type: mime::Mime, filepath: P) -> SkrdResult<impl Responder> {
    Ok(actix_files::NamedFile::open(filepath)?
        .set_content_type(content_type)
        .use_last_modified(true))
}

// say goodbye to strongly typed mime
fn send_file_with_custom_mime<P: AsRef<Path>>(
    content_type: &str,
    filepath: P,
) -> SkrdResult<impl Responder> {
    let content_type =
        Mime::from_str(content_type).map_err(|e| std::io::Error::new(ErrorKind::Other, e))?;
    Ok(actix_files::NamedFile::open(filepath)?
        .set_content_type(content_type)
        .use_last_modified(true))
}

fn publish() -> SkrdResult<impl Responder> {
    Ok(HttpResponse::Ok().finish())
}

fn get_owners() -> SkrdResult<impl Responder> {
    Ok(HttpResponse::Ok().finish())
}

fn add_owners() -> SkrdResult<impl Responder> {
    Ok(HttpResponse::Ok().finish())
}

fn delete_owners() -> SkrdResult<impl Responder> {
    Ok(HttpResponse::Ok().finish())
}

fn yank() -> SkrdResult<impl Responder> {
    Ok(HttpResponse::Ok().finish())
}

fn unyank() -> SkrdResult<impl Responder> {
    Ok(HttpResponse::Ok().finish())
}

fn search() -> SkrdResult<impl Responder> {
    Ok(HttpResponse::Ok().finish())
}
