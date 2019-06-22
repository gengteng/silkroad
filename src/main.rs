use actix_web::{
    HttpServer,
    App,
    web,
    HttpResponse,
    http:: {
        header
    }
};
use rustls::{
    internal::pemfile::{
        certs,
        rsa_private_keys
    },
    NoClientAuth,
    ServerConfig
};
use std::io::BufReader;
use std::fs::File;

// /api/v1/crates/tokio/0.1.21/download
fn redirect_download(path: web::Path<(String, String)>) -> HttpResponse {
    let name = &path.0;
    let version = &path.1;

    let location = match name.len() {
        1 => {
            format!("/crates/{}/{}/{}-{}.crate", 1, name, name, version)
        }
        2 => {
            format!("/crates/{}/{}/{}-{}.crate", 2, name, name, version)
        }
        3 => {
            format!("/crates/{}/{}/{}/{}-{}.crate", 3, &name[..1], name, name, version)
        }
        _ => {
            format!("/crates/{}/{}/{}/{}-{}.crate", &name[..2], &name[2..4], name, name, version)
        }
    };

    HttpResponse::Found()
        .header(header::LOCATION, location)
        .finish()
}

fn main() {

    let sys = actix_rt::System::new("silk_road");

    // ssl initialize
    let mut config = ServerConfig::new(NoClientAuth::new());
    let cert_file = &mut BufReader::new(File::open("cert.pem").unwrap());
    let key_file = &mut BufReader::new(File::open("key.pem").unwrap());
    let cert_chain = certs(cert_file).unwrap();
    let mut keys = rsa_private_keys(key_file).unwrap();
    config.set_single_cert(cert_chain, keys.remove(0)).unwrap();

    HttpServer::new(|| {
        App::new()
            .service(web::resource("/api/v1/crates/{name}/{version}/download").route(web::get().to(redirect_download)))
            .service(actix_files::Files::new("/crates", r#"E:\crates-mirror\crates"#).show_files_listing())
            .service(actix_files::Files::new("/crates.io-index", r#"E:\crates-mirror\crates.io-index"#).show_files_listing())
    })
    .bind_rustls("0.0.0.0:443", config).expect("bind error")
    .start();

    sys.run().expect("run error");
}