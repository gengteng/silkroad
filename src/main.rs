#[macro_use]
extern crate serde_derive;

use actix_web::{
    HttpServer,
    App,
    web,
    HttpResponse,
    Error
};
use futures::{
    Future,
    future
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

#[derive(Serialize, Debug)]
struct Data {
    value: i32
}
// TODO: fix me
fn create_something() -> impl Future<Item = HttpResponse, Error = Error> {
    let d = Data { value: 12 };

    future::ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(serde_json::to_string(&d).unwrap()))
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
            .service(web::resource("/api/v1/crates").route(web::get().to_async(create_something)))
            .service(actix_files::Files::new("/crates.io-index", r#"E:\crates-mirror\crates.io-index"#).show_files_listing())
    })
    .bind_rustls("0.0.0.0:443", config).expect("bind error")
    .start();

    sys.run().expect("run error");
}