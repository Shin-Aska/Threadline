use super::OAuthError;
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use url::Url;

pub async fn pinned_client(url: &Url) -> Result<reqwest::Client, OAuthError> {
    let host = url
        .host_str()
        .ok_or_else(|| OAuthError::Configuration("OAuth destination host is missing".into()))?;
    let port = url.port_or_known_default().ok_or_else(|| {
        OAuthError::Configuration("OAuth destination port could not be determined".into())
    })?;
    let addresses = tokio::net::lookup_host((host, port))
        .await
        .map_err(|error| OAuthError::Provider(format!("OAuth destination lookup failed: {error}")))?
        .collect::<Vec<SocketAddr>>();
    if addresses.is_empty()
        || addresses
            .iter()
            .any(|address| private_address(address.ip()))
    {
        return Err(OAuthError::Configuration(
            "OAuth destinations must resolve only to public network addresses".into(),
        ));
    }
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("Threadline/", env!("CARGO_PKG_VERSION")))
        .resolve_to_addrs(host, &addresses)
        .build()
        .map_err(|error| OAuthError::Provider(error.to_string()))
}

fn private_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(value) => {
            let [first, second, ..] = value.octets();
            value.is_private()
                || value.is_loopback()
                || value.is_link_local()
                || value.is_unspecified()
                || value.is_broadcast()
                || value.is_documentation()
                || value.is_multicast()
                || first == 0
                || (first == 100 && (64..=127).contains(&second))
                || (first == 192 && second == 0)
                || (first == 192 && second == 88)
                || (first == 198 && (18..=19).contains(&second))
                || first >= 240
        }
        IpAddr::V6(value) => {
            let segments = value.segments();
            value
                .to_ipv4_mapped()
                .is_some_and(|mapped| private_address(IpAddr::V4(mapped)))
                || value.is_loopback()
                || value.is_unspecified()
                || value.is_multicast()
                || segments[..6] == [0; 6]
                || segments[0] == 0x0064
                || segments[0] == 0x0100
                || (segments[0] == 0x2001 && segments[1] < 0x0200)
                || segments[0] == 0x2002
                || (segments[0] == 0x3fff && (segments[1] & 0xf000) == 0)
                || (segments[0] & 0xfe00) == 0xfc00
                || (segments[0] & 0xffc0) == 0xfe80
                || (segments[0] & 0xffc0) == 0xfec0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::private_address;
    use std::net::IpAddr;

    #[test]
    fn ipv4_mapped_loopback_is_private() {
        // Given an IPv4 loopback address encoded in IPv6 form.
        let address: IpAddr = "::ffff:127.0.0.1".parse().expect("mapped address");

        // When the OAuth destination classifier checks it.
        let private = private_address(address);

        // Then the destination is blocked before reqwest receives the pinned address.
        assert!(private);
    }

    #[test]
    fn special_use_destinations_are_private() {
        // Given addresses reserved for carrier NAT, benchmarking, protocol use, and tunnelling.
        let addresses = [
            "100.64.0.1",
            "198.18.0.1",
            "192.0.0.1",
            "192.88.99.1",
            "64:ff9b::1",
            "2001::1",
            "2002::1",
            "::127.0.0.1",
        ];

        // When each resolved OAuth destination is classified.
        let results = addresses.map(|address| {
            private_address(address.parse::<IpAddr>().expect("special-use address"))
        });

        // Then none can reach reqwest as an allowed pinned destination.
        assert!(results.into_iter().all(|private| private));
    }
}
