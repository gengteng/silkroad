use actix_web::{guard, http::header, web, App, HttpRequest, HttpResponse, HttpServer};
use rustls::{
    internal::pemfile::{certs, rsa_private_keys},
    NoClientAuth, ServerConfig,
};
use std::{fs::File, io::BufReader, process::Command as PsCommand};

use crate::error::SkrdResult;
use std::net::SocketAddr;
use std::path::PathBuf;
use structopt::StructOpt;

const DEFAULT_LISTENING_ADDRESS: &str = "127.0.0.1:4000";

#[derive(Debug, StructOpt)]
#[structopt(name = "serve")]
pub struct Serve {
    #[structopt(
        long,
        short = "a",
        help = "Set the listening address",
        value_name = "IP:PORT",
        raw(default_value = "DEFAULT_LISTENING_ADDRESS"),
        parse(try_from_str)
    )]
    addr: SocketAddr,

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
}

impl Serve {
    pub fn serve(&self) -> SkrdResult<()> {
        let sys = actix_rt::System::new("silk_road");

        // ssl initialize
        let mut config = ServerConfig::new(NoClientAuth::new());
        let cert_file = &mut BufReader::new(File::open("cert.pem")?);
        let key_file = &mut BufReader::new(File::open("key.pem")?);
        let cert_chain = certs(cert_file)
            .map_err(|_| rustls::TLSError::General("Extract certificates error".to_owned()))?;
        let mut keys = rsa_private_keys(key_file)
            .map_err(|_| rustls::TLSError::General("Extract RSA private keys error".to_owned()))?;
        config.set_single_cert(cert_chain, keys.remove(0))?;

        let index_path = self.index_path.clone();
        let crates_path = self.crates_path.clone();

        HttpServer::new(move || {
            App::new()
                .data(index_path.clone())
                .service(
                    web::resource("/api/v1/crates/{name}/{version}/download")
                        .route(web::get().to(redirect_download)),
                )
                .service(
                    actix_files::Files::new("/crates", crates_path.clone()).show_files_listing(),
                )
                .service(web::resource("/crates.io-index/info/refs").to(git_info_refs))
                .service(
                    actix_files::Files::new("/crates.io-index", index_path.clone())
                        .show_files_listing(),
                )
                .default_service(
                    web::resource("")
                        .route(web::get().to(p404))
                        // all requests that are not `GET`
                        .route(
                            web::route()
                                .guard(guard::Not(guard::Get()))
                                .to(HttpResponse::MethodNotAllowed),
                        ),
                )
        })
        .bind_rustls(self.addr, config)?
        .start();

        // log
        sys.run()?;
        Ok(())
    }
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
fn p404(request: HttpRequest) -> HttpResponse {
    println!("{:?}", request);
    HttpResponse::NotFound().finish()
}

fn get_service(query: &str) -> Option<&str> {
    let key = "service=";
    query.find(key).and_then(|i| {
        let start = i + key.len();
        match &query[start..].find('&') {
            Some(u) => Some(&query[start..u + start]),
            None => Some(&query[start..]),
        }
    })
}

// http://localhost/crates.io-index/info/refs?service=git-upload-pack
fn git_info_refs(request: HttpRequest, index_path: web::Data<PathBuf>) -> HttpResponse {
    match get_service(request.query_string()) {
        Some("git-upload-pack") => {
            let mut output = PsCommand::new("git")
                .arg("upload-pack")
                .arg(index_path.get_ref())
                .arg("--advertise-refs") // exit immediately after initial ref advertisement
                .output()
                .expect("output error");

            let mut body = Vec::from("001e# service=git-upload-pack\n");
            body.append(&mut output.stdout);

            HttpResponse::Ok()
                .content_type("application/x-git-upload-pack-advertisement")
                .body(body)
        }
        _ => {
            eprintln!("get service error: {:?}", request);
            HttpResponse::NotFound().finish()
        }
    }
}
