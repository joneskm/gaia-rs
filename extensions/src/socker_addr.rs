//! Macros to simplify creation of `SocketAddr`

/// Macros to simplify creation of `SocketAddr`
#[macro_export]
macro_rules! socket_addr {
    ($a:expr, $b:expr, $c:expr, $d:expr , $port:expr) => {
        const {
            std::net::SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::new($a, $b, $c, $d)),
                $port,
            )
        }
    };
}
