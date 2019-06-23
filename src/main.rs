use actix_web::{guard, http::header, web, App, HttpRequest, HttpResponse, HttpServer};
use rustls::{
    internal::pemfile::{certs, rsa_private_keys},
    NoClientAuth, ServerConfig,
};
use std::{collections::HashMap, fs::File, io::BufReader, process::Command};

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
            Some(u) => Some(&query[start..u+start]),
            None => Some(&query[start..])
        }
    })
}

// http://localhost/crates.io-index/info/refs?service=git-upload-pack
fn git_info_refs(request: HttpRequest, index_path: web::Data<&str>) -> HttpResponse {
    match get_service(request.query_string()) {
        Some("git-upload-pack") => {
            let mut output = Command::new("git")
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

fn main() {
    let sys = actix_rt::System::new("silk_road");

    //    // ssl initialize
    //    let mut config = ServerConfig::new(NoClientAuth::new());
    //    let cert_file = &mut BufReader::new(File::open("cert.pem").unwrap());
    //    let key_file = &mut BufReader::new(File::open("key.pem").unwrap());
    //    let cert_chain = certs(cert_file).unwrap();
    //    let mut keys = rsa_private_keys(key_file).unwrap();
    //    config.set_single_cert(cert_chain, keys.remove(0)).unwrap();

    HttpServer::new(|| {
        let index_path: &str = r#"E:\crates-mirror\crates.io-index"#;
        App::new()
            .data(index_path.clone())
            .service(
                web::resource("/api/v1/crates/{name}/{version}/download")
                    .route(web::get().to(redirect_download)),
            )
            .service(
                actix_files::Files::new("/crates", r#"E:\crates-mirror\crates"#)
                    .show_files_listing(),
            )
            .service(web::resource("/crates.io-index/info/refs").to(git_info_refs))
            .service(actix_files::Files::new("/crates.io-index", index_path).show_files_listing())
            .default_service(
                web::resource("")
                    .route(web::get().to(p404))
                    // all requests that are not `GET`
                    .route(
                        web::route()
                            .guard(guard::Not(guard::Get()))
                            .to(|| HttpResponse::MethodNotAllowed()),
                    ),
            )
    })
    .bind("0.0.0.0:80")
    .expect("bind error")
    .start();

    println!("Server started...");
    sys.run().expect("run error");
}
