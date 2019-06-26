use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub fn sock_addr_v4(a: u8, b: u8, c: u8, d: u8, port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(a, b, c, d)), port)
}

pub fn get_service_from_query_string(query: &str) -> Option<&str> {
    let head = "service=git-";
    query.find(head).and_then(|i| {
        let start = i + head.len();
        match &query[start..].find('&') {
            Some(u) => Some(&query[start..u + start]),
            None => Some(&query[start..]),
        }
    })
}
