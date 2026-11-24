//! Outbound address guard: refuse requests to loopback, private, link-local
//! and other non-public addresses.
//!
//! Anything that fetches a URL a user typed in — a Slack or webhook URL saved
//! per team, an avatar URL, an import feed — can be pointed at
//! `http://localhost:6379`, `http://10.0.0.5/admin` or the cloud metadata
//! endpoint `http://169.254.169.254/`. This is server-side request forgery
//! (SSRF); Ruby apps reach for the `ssrf_filter` gem. [`OutboundGuard`] is the
//! Loco equivalent.
//!
//! The check runs on the address the client actually connects to, not on a
//! name looked up beforehand, so a host whose DNS answer changes between the
//! check and the connection (DNS rebinding) is still caught:
//!
//! * [`OutboundGuard::apply`] installs a DNS resolver on a
//!   `reqwest::ClientBuilder` that drops refused addresses from every lookup,
//!   and a redirect policy that checks every redirect target.
//! * [`OutboundGuard::check_url`] refuses a URL whose host is an IP literal
//!   (`http://127.0.0.1/`), which never reaches a resolver. Call it before
//!   sending.
//!
//! ```rust,ignore
//! use loco_rs::net::OutboundGuard;
//!
//! let guard = OutboundGuard::default();
//! guard.check_url(&url)?;
//! let client = guard.apply(reqwest::Client::builder()).build()?;
//! client.post(&url).json(&payload).send().await?;
//! ```
//!
//! The guarded client does not use `HTTP_PROXY`/`HTTPS_PROXY`: a proxy
//! resolves the name itself, out of the guard's reach.
//!
//! Notifications use the guard for Slack and webhook deliveries, configured
//! under `notifications.outbound`.

use std::{
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use serde::{Deserialize, Serialize};

/// Which outbound addresses are allowed.
///
/// The default refuses every address that is not publicly routable (see
/// [`is_public`]). In development and tests, where the receiving end is often
/// `localhost`, allow it explicitly:
///
/// ```yaml
/// notifications:
///   outbound:
///     # any private address: development only
///     allow_private: true
///     # or: specific hosts, IPs and CIDR ranges
///     allow: [localhost, 127.0.0.1, "10.20.0.0/16", hooks.internal.example]
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct OutboundGuard {
    /// Allow every address, public or not. Turns the guard off.
    #[serde(default)]
    pub allow_private: bool,
    /// Hosts, IP addresses and CIDR ranges (`10.0.0.0/8`, `fd00::/8`) that
    /// are allowed even though they are not public. A host name matches the
    /// URL's host exactly (case-insensitive), whatever it resolves to.
    #[serde(default)]
    pub allow: Vec<String>,
}

/// The error for a refused destination. Its message names the host, never
/// the full URL, since URLs such as Slack webhooks carry secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockedAddress {
    host: String,
}

impl BlockedAddress {
    /// The refused host or address.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }
}

impl fmt::Display for BlockedAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "outbound request to `{}` refused: not a public address",
            self.host
        )
    }
}

impl std::error::Error for BlockedAddress {}

impl From<BlockedAddress> for crate::Error {
    fn from(err: BlockedAddress) -> Self {
        Self::Message(err.to_string())
    }
}

impl OutboundGuard {
    /// A guard that allows everything.
    #[must_use]
    pub const fn allow_all() -> Self {
        Self {
            allow_private: true,
            allow: Vec::new(),
        }
    }

    /// Whether a connection to `ip` is allowed: it is public, or the guard
    /// allows private addresses, or an `allow` entry covers it.
    #[must_use]
    pub fn is_allowed(&self, ip: IpAddr) -> bool {
        self.allow_private || is_public(ip) || self.allow.iter().any(|e| entry_contains(e, ip))
    }

    /// Whether `host` (a name, not an address) is allowed by name.
    #[must_use]
    pub fn allows_host(&self, host: &str) -> bool {
        let host = host.trim_end_matches('.');
        self.allow_private
            || self
                .allow
                .iter()
                .any(|e| e.trim_end_matches('.').eq_ignore_ascii_case(host))
    }

    /// Checks a host as it appears in a URL: an IP literal (`127.0.0.1`,
    /// `[::1]`) is checked as an address, a name only against the `allow`
    /// list here — its addresses are checked when it is resolved.
    ///
    /// # Errors
    ///
    /// [`BlockedAddress`] when `host` is an address the guard refuses.
    pub fn check_host(&self, host: &str) -> Result<(), BlockedAddress> {
        let bare = host.trim_start_matches('[').trim_end_matches(']');
        match bare.parse::<IpAddr>() {
            Ok(ip) if !self.is_allowed(ip) => Err(BlockedAddress {
                host: bare.to_string(),
            }),
            _ => Ok(()),
        }
    }
}

/// Whether `entry` (an IP or a CIDR range) contains `ip`. Host names never
/// match an address.
fn entry_contains(entry: &str, ip: IpAddr) -> bool {
    let entry = entry.trim();
    let (addr, prefix) = match entry.split_once('/') {
        Some((addr, prefix)) => match prefix.parse::<u8>() {
            Ok(prefix) => (addr, Some(prefix)),
            Err(_) => return false,
        },
        None => (entry, None),
    };
    let Ok(net) = addr
        .trim_start_matches('[')
        .trim_end_matches(']')
        .parse::<IpAddr>()
    else {
        return false;
    };
    let ip = canonical(ip);
    match (canonical(net), ip) {
        (IpAddr::V4(net), IpAddr::V4(ip)) => {
            let prefix = prefix.unwrap_or(32);
            prefix <= 32 && in_v4(ip, u32::from(net), prefix)
        }
        (IpAddr::V6(net), IpAddr::V6(ip)) => {
            let prefix = prefix.unwrap_or(128);
            prefix <= 128 && in_v6(ip, u128::from(net), prefix)
        }
        _ => false,
    }
}

/// An IPv4-mapped IPv6 address (`::ffff:127.0.0.1`) as the IPv4 address it
/// reaches.
fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(IpAddr::V6(v6), IpAddr::V4),
        v4 @ IpAddr::V4(_) => v4,
    }
}

const fn in_v4(ip: Ipv4Addr, net: u32, prefix: u8) -> bool {
    if prefix == 0 {
        return true;
    }
    let mask = u32::MAX << (32 - prefix);
    ip.to_bits() & mask == net & mask
}

const fn in_v6(ip: Ipv6Addr, net: u128, prefix: u8) -> bool {
    if prefix == 0 {
        return true;
    }
    let mask = u128::MAX << (128 - prefix);
    ip.to_bits() & mask == net & mask
}

/// IPv4 ranges that are not publicly routable.
const V4_BLOCKED: &[(u32, u8)] = &[
    (0x0000_0000, 8),  // 0.0.0.0/8 "this network", incl. unspecified
    (0x0A00_0000, 8),  // 10.0.0.0/8 private
    (0x6440_0000, 10), // 100.64.0.0/10 carrier-grade NAT
    (0x7F00_0000, 8),  // 127.0.0.0/8 loopback
    (0xA9FE_0000, 16), // 169.254.0.0/16 link-local, incl. cloud metadata
    (0xAC10_0000, 12), // 172.16.0.0/12 private
    (0xC000_0000, 24), // 192.0.0.0/24 IETF protocol assignments
    (0xC000_0200, 24), // 192.0.2.0/24 documentation
    (0xC058_6300, 24), // 192.88.99.0/24 6to4 relay
    (0xC0A8_0000, 16), // 192.168.0.0/16 private
    (0xC612_0000, 15), // 198.18.0.0/15 benchmarking
    (0xC633_6400, 24), // 198.51.100.0/24 documentation
    (0xCB00_7100, 24), // 203.0.113.0/24 documentation
    (0xE000_0000, 4),  // 224.0.0.0/4 multicast
    (0xF000_0000, 4),  // 240.0.0.0/4 reserved, incl. broadcast
];

/// IPv6 ranges that are not publicly routable. Ranges that embed an IPv4
/// address are handled in [`is_public`].
const V6_BLOCKED: &[(u128, u8)] = &[
    (0, 128),                          // :: unspecified
    (1, 128),                          // ::1 loopback
    (0x0100_0000_0000_0000 << 64, 64), // 100::/64 discard
    (0x2001_0000_u128 << 96, 32),      // 2001::/32 Teredo
    (0x2001_0db8_u128 << 96, 32),      // 2001:db8::/32 documentation
    (0x3fff_u128 << 112, 20),          // 3fff::/20 documentation
    (0xfc00_u128 << 112, 7),           // fc00::/7 unique local
    (0xfe80_u128 << 112, 10),          // fe80::/10 link-local
    (0xfec0_u128 << 112, 10),          // fec0::/10 site-local
    (0xff00_u128 << 112, 8),           // ff00::/8 multicast
];

/// Whether `ip` is a publicly routable address.
///
/// Refused: loopback, private (RFC 1918), carrier-grade NAT, link-local
/// (including the cloud metadata address `169.254.169.254`), unspecified,
/// multicast, broadcast and reserved, documentation and benchmarking ranges,
/// IPv6 unique-local, link-local, site-local and Teredo — and any IPv6 form
/// that embeds one of the refused IPv4 addresses (IPv4-mapped
/// `::ffff:127.0.0.1`, IPv4-compatible `::127.0.0.1`, NAT64 `64:ff9b::/96` and
/// 6to4 `2002::/16`).
#[must_use]
pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => !V4_BLOCKED
            .iter()
            .any(|&(net, prefix)| in_v4(v4, net, prefix)),
        IpAddr::V6(v6) => {
            if let Some(v4) = embedded_v4(v6) {
                return is_public(IpAddr::V4(v4));
            }
            !V6_BLOCKED
                .iter()
                .any(|&(net, prefix)| in_v6(v6, net, prefix))
        }
    }
}

/// The IPv4 address an IPv6 address embeds, for the forms a connection can
/// end up at.
fn embedded_v4(v6: Ipv6Addr) -> Option<Ipv4Addr> {
    let bits = v6.to_bits();
    let low32 = Ipv4Addr::from_bits((bits & 0xffff_ffff) as u32);
    if let Some(v4) = v6.to_ipv4_mapped() {
        return Some(v4);
    }
    // ::a.b.c.d (IPv4-compatible, deprecated). `::` and `::1` are left to
    // the IPv6 table.
    if bits >> 32 == 0 && bits > 1 {
        return Some(low32);
    }
    // 64:ff9b::/96 NAT64
    if in_v6(v6, 0x0064_ff9b_u128 << 96, 96) {
        return Some(low32);
    }
    // 2002::/16 6to4: the IPv4 address is in bits 16..48.
    if in_v6(v6, 0x2002_u128 << 112, 16) {
        return Some(Ipv4Addr::from_bits(((bits >> 80) & 0xffff_ffff) as u32));
    }
    None
}

#[cfg(any(feature = "notifications", feature = "auth_oauth"))]
mod client {
    use std::{net::SocketAddr, sync::Arc};

    use reqwest::{
        dns::{Addrs, Name, Resolve, Resolving},
        redirect, ClientBuilder, Url,
    };

    use super::{BlockedAddress, OutboundGuard};

    /// Redirects a guarded client follows before giving up, as reqwest's
    /// default.
    const MAX_REDIRECTS: usize = 10;

    /// A DNS resolver that keeps only the addresses an [`OutboundGuard`]
    /// allows, and fails a lookup that has none left. Installed by
    /// [`OutboundGuard::apply`].
    #[derive(Debug, Clone)]
    pub struct GuardedResolver {
        guard: Arc<OutboundGuard>,
    }

    impl GuardedResolver {
        /// A resolver enforcing `guard`.
        #[must_use]
        pub fn new(guard: OutboundGuard) -> Self {
            Self {
                guard: Arc::new(guard),
            }
        }

        /// Keeps the allowed addresses among `addrs` resolved for `host`.
        ///
        /// # Errors
        ///
        /// [`BlockedAddress`] when none are allowed.
        pub fn filter(
            &self,
            host: &str,
            addrs: impl IntoIterator<Item = SocketAddr>,
        ) -> Result<Vec<SocketAddr>, BlockedAddress> {
            let by_name = self.guard.allows_host(host);
            let mut seen = false;
            let kept: Vec<SocketAddr> = addrs
                .into_iter()
                .inspect(|_| seen = true)
                .filter(|addr| by_name || self.guard.is_allowed(addr.ip()))
                .collect();
            if seen && kept.is_empty() {
                tracing::warn!(
                    host,
                    "outbound request refused: resolves to non-public addresses only"
                );
                return Err(BlockedAddress {
                    host: host.to_string(),
                });
            }
            Ok(kept)
        }
    }

    impl Resolve for GuardedResolver {
        fn resolve(&self, name: Name) -> Resolving {
            let resolver = self.clone();
            Box::pin(async move {
                let host = name.as_str().to_string();
                let addrs = tokio::net::lookup_host((host.as_str(), 0)).await?;
                let kept = resolver.filter(&host, addrs)?;
                let addrs: Addrs = Box::new(kept.into_iter());
                Ok(addrs)
            })
        }
    }

    impl OutboundGuard {
        /// Checks a URL before sending: it must parse and have a host, and
        /// an IP-literal host must be allowed. Host names are checked when
        /// they resolve, by a client built with [`OutboundGuard::apply`].
        ///
        /// # Errors
        ///
        /// [`BlockedAddress`] for a refused or unparseable URL.
        pub fn check_url(&self, url: &str) -> Result<(), BlockedAddress> {
            let parsed = Url::parse(url).map_err(|_| BlockedAddress {
                host: "<invalid url>".to_string(),
            })?;
            self.check_parsed(&parsed)
        }

        fn check_parsed(&self, url: &Url) -> Result<(), BlockedAddress> {
            url.host_str().map_or_else(
                || {
                    Err(BlockedAddress {
                        host: "<no host>".to_string(),
                    })
                },
                |host| self.check_host(host),
            )
        }

        /// Installs the guard on a reqwest client: a resolver that drops
        /// refused addresses, a redirect policy that checks each target, and
        /// no system proxy. Pair it with [`OutboundGuard::check_url`] for
        /// IP-literal URLs. A guard with `allow_private` set leaves the
        /// builder as it is.
        pub fn apply(&self, builder: ClientBuilder) -> ClientBuilder {
            if self.allow_private {
                return builder;
            }
            let guard = self.clone();
            builder
                .no_proxy()
                .dns_resolver(Arc::new(GuardedResolver::new(self.clone())))
                .redirect(redirect::Policy::custom(move |attempt| {
                    if attempt.previous().len() >= MAX_REDIRECTS {
                        return attempt.error("too many redirects");
                    }
                    match guard.check_parsed(attempt.url()) {
                        Ok(()) => attempt.follow(),
                        Err(blocked) => attempt.error(blocked),
                    }
                }))
        }
    }

    /// Whether a reqwest error was caused by the guard refusing an address
    /// (at resolve time or on a redirect).
    #[must_use]
    pub fn is_blocked(err: &reqwest::Error) -> bool {
        let mut source: Option<&(dyn std::error::Error + 'static)> = Some(err);
        while let Some(e) = source {
            if e.is::<BlockedAddress>() {
                return true;
            }
            source = e.source();
        }
        false
    }
}

#[cfg(any(feature = "notifications", feature = "auth_oauth"))]
pub use client::{is_blocked, GuardedResolver};

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn classifies_addresses() {
        let refused = [
            "0.0.0.0",
            "0.1.2.3",
            "10.0.0.1",
            "100.64.0.1",
            "100.127.255.254",
            "127.0.0.1",
            "127.255.255.254",
            "169.254.169.254",
            "172.16.0.1",
            "172.31.255.255",
            "192.0.0.8",
            "192.0.2.1",
            "192.168.1.1",
            "198.18.0.1",
            "198.51.100.7",
            "203.0.113.9",
            "224.0.0.1",
            "239.255.255.250",
            "240.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "::ffff:127.0.0.1",
            "::ffff:10.0.0.1",
            "::ffff:169.254.169.254",
            "::127.0.0.1",
            "64:ff9b::7f00:1",
            "2002:7f00:1::",
            "2002:a00:1::1",
            "fc00::1",
            "fd12:3456::1",
            "fe80::1",
            "fec0::1",
            "ff02::1",
            "2001:db8::1",
            "2001::1",
            "100::1",
        ];
        for addr in refused {
            assert!(!is_public(ip(addr)), "{addr} should be refused");
        }
        let allowed = [
            "1.1.1.1",
            "8.8.8.8",
            "100.63.255.255",
            "100.128.0.1",
            "172.15.255.255",
            "172.32.0.1",
            "192.0.1.1",
            "192.169.0.1",
            "198.20.0.1",
            "223.255.255.255",
            "2606:4700:4700::1111",
            "2a00:1450:4001::200e",
            "::ffff:8.8.8.8",
            "64:ff9b::808:808",
            "2002:808:808::1",
        ];
        for addr in allowed {
            assert!(is_public(ip(addr)), "{addr} should be allowed");
        }
    }

    #[test]
    fn default_guard_refuses_private_and_allows_public() {
        let guard = OutboundGuard::default();
        assert!(!guard.is_allowed(ip("127.0.0.1")));
        assert!(guard.is_allowed(ip("93.184.216.34")));
        assert!(guard.check_host("[::1]").is_err());
        assert!(guard.check_host("127.0.0.1").is_err());
        assert!(guard.check_host("example.com").is_ok());
        assert_eq!(
            guard.check_host("10.1.2.3").unwrap_err().to_string(),
            "outbound request to `10.1.2.3` refused: not a public address"
        );
    }

    #[test]
    fn allow_list_takes_ips_cidrs_and_hosts() {
        let guard = OutboundGuard {
            allow_private: false,
            allow: vec![
                "127.0.0.1".to_string(),
                "10.20.0.0/16".to_string(),
                "fd00::/8".to_string(),
                "Hooks.Internal".to_string(),
                "not/a-cidr".to_string(),
            ],
        };
        assert!(guard.is_allowed(ip("127.0.0.1")));
        assert!(guard.is_allowed(ip("::ffff:127.0.0.1")));
        assert!(!guard.is_allowed(ip("127.0.0.2")));
        assert!(guard.is_allowed(ip("10.20.3.4")));
        assert!(!guard.is_allowed(ip("10.21.0.1")));
        assert!(guard.is_allowed(ip("fd00::5")));
        assert!(!guard.is_allowed(ip("fc00::5")));
        assert!(guard.allows_host("hooks.internal"));
        assert!(guard.allows_host("hooks.internal."));
        assert!(!guard.allows_host("other.internal"));
    }

    #[test]
    fn allow_private_allows_everything() {
        let guard = OutboundGuard::allow_all();
        assert!(guard.is_allowed(ip("127.0.0.1")));
        assert!(guard.is_allowed(ip("169.254.169.254")));
        assert!(guard.allows_host("anything"));
    }

    #[test]
    fn deserializes_from_yaml() {
        let guard: OutboundGuard = serde_yaml::from_str("allow: [localhost, 127.0.0.1]").unwrap();
        assert!(!guard.allow_private);
        assert_eq!(guard.allow, vec!["localhost", "127.0.0.1"]);
        let empty: OutboundGuard = serde_yaml::from_str("{}").unwrap();
        assert_eq!(empty, OutboundGuard::default());
    }

    #[cfg(any(feature = "notifications", feature = "auth_oauth"))]
    mod client {
        use std::net::SocketAddr;

        use super::super::*;

        fn sock(s: &str) -> SocketAddr {
            s.parse().unwrap()
        }

        #[test]
        fn check_url_refuses_ip_literals() {
            let guard = OutboundGuard::default();
            assert!(guard.check_url("http://127.0.0.1:8080/x").is_err());
            assert!(guard.check_url("http://[::1]/x").is_err());
            assert!(guard.check_url("http://[::ffff:7f00:1]/x").is_err());
            assert!(guard.check_url("http://169.254.169.254/latest").is_err());
            assert!(guard.check_url("http://2130706433/").is_err()); // 127.0.0.1 as an integer
            assert!(guard.check_url("http://0x7f.1/").is_err());
            assert!(guard.check_url("not a url").is_err());
            assert!(guard
                .check_url("https://hooks.slack.com/services/x")
                .is_ok());
            assert!(guard.check_url("https://8.8.8.8/").is_ok());
        }

        #[test]
        fn resolver_keeps_only_allowed_addresses() {
            let resolver = GuardedResolver::new(OutboundGuard::default());
            let kept = resolver
                .filter(
                    "mixed.example",
                    [sock("127.0.0.1:0"), sock("8.8.8.8:0"), sock("[fd00::1]:0")],
                )
                .unwrap();
            assert_eq!(kept, vec![sock("8.8.8.8:0")]);

            let err = resolver
                .filter("rebind.example", [sock("127.0.0.1:0"), sock("10.0.0.1:0")])
                .unwrap_err();
            assert_eq!(err.host(), "rebind.example");

            // A name on the allow list keeps whatever it resolves to.
            let resolver = GuardedResolver::new(OutboundGuard {
                allow_private: false,
                allow: vec!["localhost".to_string()],
            });
            let kept = resolver.filter("localhost", [sock("127.0.0.1:0")]).unwrap();
            assert_eq!(kept, vec![sock("127.0.0.1:0")]);
        }

        #[tokio::test]
        async fn guarded_client_refuses_names_resolving_to_loopback() {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let app = axum::Router::new().fallback(|| async { "ok" });
            tokio::spawn(async move { axum::serve(listener, app).await });

            let guard = OutboundGuard::default();
            let client = guard.apply(reqwest::Client::builder()).build().unwrap();
            let err = client
                .get(format!("http://localhost:{port}/"))
                .send()
                .await
                .unwrap_err();
            assert!(is_blocked(&err), "{err:?}");

            let allowed = OutboundGuard {
                allow_private: false,
                allow: vec!["localhost".to_string()],
            };
            let client = allowed.apply(reqwest::Client::builder()).build().unwrap();
            let body = client
                .get(format!("http://localhost:{port}/"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            assert_eq!(body, "ok");
        }

        #[tokio::test]
        async fn guarded_client_refuses_redirects_to_private_addresses() {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let app = axum::Router::new().route(
                "/hop",
                axum::routing::get(|| async {
                    axum::response::Redirect::temporary("http://169.254.169.254/latest")
                }),
            );
            tokio::spawn(async move { axum::serve(listener, app).await });

            // Allow the first hop by name only; the redirect target is an
            // IP literal the guard refuses.
            let guard = OutboundGuard {
                allow_private: false,
                allow: vec!["localhost".to_string()],
            };
            let client = guard.apply(reqwest::Client::builder()).build().unwrap();
            let err = client
                .get(format!("http://localhost:{port}/hop"))
                .send()
                .await
                .unwrap_err();
            assert!(err.is_redirect(), "{err:?}");
            assert!(is_blocked(&err), "{err:?}");
        }
    }
}
