//! Remote IP Middleware for inferring the client's IP address.
//!
//! This middleware is useful when running behind proxies or load balancers
//! that record the original client IP in a header (e.g. `X-Forwarded-For`,
//! `CF-Connecting-IP`, ...).
//!
//! This is a thin, Loco-flavored wrapper around the [`axum-client-ip`
//! crate](https://docs.rs/axum-client-ip), which does the actual header
//! parsing. Loco configures *which* source to trust
//! (see [`axum_client_ip::ClientIpSource`]) and exposes the result through the
//! familiar [`RemoteIP`] extractor.
use std::net::{IpAddr, SocketAddr};

use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::request::Parts,
    Router as AXRouter,
};
use axum_client_ip::ClientIp;
pub use axum_client_ip::ClientIpSource;
use serde::{Deserialize, Serialize};

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Result};

///
/// Resolves the client's IP address from a single, configured, trusted
/// source.
///
/// WARNING
/// =======
///
/// LIKE ANY SUCH REMOTE IP MIDDLEWARE, IN THE WRONG ARCHITECTURE IT CAN MAKE
/// YOU VULNERABLE TO IP SPOOFING.
///
/// This middleware assumes there is exactly **one** trusted edge (a proxy or
/// load balancer you control, sitting directly in front of the app) that is
/// responsible for setting the configured header correctly. Anything upstream
/// of that edge (a CDN, an outer load balancer, ...) is *not* modeled here:
/// this middleware does not walk a proxy chain, and it does not maintain a
/// trusted-proxy CIDR allow-list. If your edge proxy blindly forwards
/// whatever header value it received, a malicious client can forge it.
///
/// DO NOT USE THIS MIDDLEWARE IF YOU DON'T KNOW THAT YOU NEED IT.
///
/// -- But if you need it, it's crucial to use it (since it's the only way to
/// get the original client IP).
///
/// BREAKING CHANGE (0.17 -> next)
/// ===============================
///
/// Earlier versions of this middleware hand-rolled `X-Forwarded-For` parsing:
/// it walked the header from right to left, skipping any IP found in a
/// configurable `trusted_proxies` CIDR list (falling back to a built-in
/// RFC-1918 + loopback list), and returned the first non-trusted address —
/// i.e. it could "see through" a chain of one or more trusted proxies.
///
/// This middleware is now a thin wrapper around the [`axum-client-ip`] crate.
/// There is no more proxy-chain walking and no more `trusted_proxies`
/// CIDR list: instead you pick *one* [`ClientIpSource`] — the single header
/// (or the raw socket address) that your **innermost** trusted proxy is
/// responsible for setting correctly. The default, [`ClientIpSource::RightmostXForwardedFor`],
/// simply takes the last (rightmost) comma-separated value of the last
/// `X-Forwarded-For` header, with **no filtering of private/internal
/// addresses whatsoever**.
///
/// If you run multiple hops in front of your app (e.g. CDN -> load balancer
/// -> ingress), configure your **innermost** hop (the one talking directly to
/// this app) to compute and set the correct client IP itself (most reverse
/// proxies support this, e.g. nginx's `set_real_ip_from`/`real_ip_recursive`),
/// and let this middleware simply trust that hop's output. Alternatively, if
/// you sit directly behind a well-known provider, pick its dedicated header,
/// e.g. [`ClientIpSource::CfConnectingIp`] for Cloudflare or
/// [`ClientIpSource::CloudFrontViewerAddress`] for AWS CloudFront.
///
/// [`axum-client-ip`]: https://docs.rs/axum-client-ip
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RemoteIpMiddleware {
    #[serde(default)]
    pub enable: bool,
    /// The single, trusted source to resolve the client IP from.
    ///
    /// Defaults to [`ClientIpSource::RightmostXForwardedFor`], which matches
    /// the common single reverse-proxy/load-balancer deployment.
    #[serde(default = "default_source")]
    pub source: ClientIpSource,
}

impl Default for RemoteIpMiddleware {
    fn default() -> Self {
        Self {
            enable: false,
            source: default_source(),
        }
    }
}

fn default_source() -> ClientIpSource {
    ClientIpSource::RightmostXForwardedFor
}

impl MiddlewareLayer for RemoteIpMiddleware {
    /// Returns the name of the middleware
    fn name(&self) -> &'static str {
        "remote_ip"
    }

    /// Returns whether the middleware is enabled or not
    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    /// Applies the Remote IP middleware to the given Axum router.
    ///
    /// Under the hood this just makes the configured [`ClientIpSource`]
    /// available in request extensions, which the [`RemoteIP`] extractor (and
    /// [`axum_client_ip::ClientIp`] directly, if you prefer to use it)
    /// consumes.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Ok(app.layer(self.source.clone().into_extension()))
    }
}

/// The resolved remote IP for a request, as determined by the
/// [`RemoteIpMiddleware`] (if enabled).
#[derive(Copy, Clone, Debug)]
pub enum RemoteIP {
    /// Resolved from the configured header-based [`ClientIpSource`] (e.g.
    /// `X-Forwarded-For`, `CF-Connecting-IP`, ...).
    Forwarded(IpAddr),
    /// Resolved from the raw socket peer address: either because the
    /// middleware is configured with [`ClientIpSource::ConnectInfo`], or
    /// because the configured header was missing/malformed and we fell back
    /// to the socket address.
    Socket(IpAddr),
    /// The remote-ip middleware is not enabled, or no IP could be resolved by
    /// any means.
    None,
}

impl<S> FromRequestParts<S> for RemoteIP
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // No `ClientIpSource` in extensions means the middleware was never
        // applied to this router (it's disabled) — nothing to resolve.
        let Some(source) = parts.extensions.get::<ClientIpSource>().cloned() else {
            return Ok(Self::None);
        };

        match ClientIp::from_request_parts(parts, state).await {
            Ok(ClientIp(ip)) if source == ClientIpSource::ConnectInfo => Ok(Self::Socket(ip)),
            Ok(ClientIp(ip)) => Ok(Self::Forwarded(ip)),
            // The configured header was absent/malformed (or, in principle,
            // `ConnectInfo` itself was missing) — fall back to the raw socket
            // peer address, mirroring the old middleware's graceful
            // degradation when no `X-Forwarded-For` header was present.
            Err(_) => Ok(parts
                .extensions
                .get::<ConnectInfo<SocketAddr>>()
                .map_or(Self::None, |ConnectInfo(addr)| Self::Socket(addr.ip()))),
        }
    }
}

impl std::fmt::Display for RemoteIP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forwarded(ip) => write!(f, "remote: {ip}"),
            Self::Socket(ip) => write!(f, "socket: {ip}"),
            Self::None => write!(f, "--"),
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        extract::{ConnectInfo, Request},
        http::{HeaderName, HeaderValue},
    };

    use super::*;

    fn request(headers: &[(&str, &str)], connect_info: Option<SocketAddr>) -> Request {
        let mut builder = Request::builder().uri("/");
        for (name, value) in headers {
            builder = builder.header(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        let mut req = builder.body(axum::body::Body::empty()).unwrap();
        if let Some(addr) = connect_info {
            req.extensions_mut().insert(ConnectInfo(addr));
        }
        req
    }

    async fn resolve(source: Option<ClientIpSource>, req: Request) -> RemoteIP {
        let (mut parts, _body) = req.into_parts();
        if let Some(source) = source {
            parts.extensions.insert(source);
        }
        RemoteIP::from_request_parts(&mut parts, &()).await.unwrap()
    }

    #[test]
    fn default_is_disabled_with_rightmost_xff_source() {
        let middleware = RemoteIpMiddleware::default();
        assert!(!middleware.is_enabled());
        assert_eq!(middleware.source, ClientIpSource::RightmostXForwardedFor);
    }

    #[test]
    fn is_enabled_follows_the_enable_flag() {
        assert!(RemoteIpMiddleware {
            enable: true,
            source: ClientIpSource::RightmostXForwardedFor
        }
        .is_enabled());
        assert!(!RemoteIpMiddleware {
            enable: false,
            source: ClientIpSource::RightmostXForwardedFor
        }
        .is_enabled());
    }

    #[tokio::test]
    async fn disabled_middleware_yields_none() {
        // no `ClientIpSource` extension inserted => middleware was never
        // applied to the router.
        let req = request(&[("x-forwarded-for", "51.50.51.50")], None);
        let ip = resolve(None, req).await;
        assert!(matches!(ip, RemoteIP::None));
    }

    #[tokio::test]
    async fn rightmost_xff_takes_the_last_hop_verbatim_no_trusted_proxy_skipping() {
        // NEW behavior: unlike the old hand-rolled middleware, nothing here
        // is skipped for being a "trusted"/private proxy IP — the rightmost
        // (last) value is taken as-is, even though it's a private address.
        let req = request(
            &[("x-forwarded-for", "51.50.51.50,10.0.0.1,192.168.1.1")],
            None,
        );
        let ip = resolve(Some(ClientIpSource::RightmostXForwardedFor), req).await;
        assert!(
            matches!(ip, RemoteIP::Forwarded(ip) if ip == "192.168.1.1".parse::<IpAddr>().unwrap())
        );
    }

    #[tokio::test]
    async fn rightmost_xff_single_hop_matches_the_client() {
        let req = request(&[("x-forwarded-for", "51.50.51.50")], None);
        let ip = resolve(Some(ClientIpSource::RightmostXForwardedFor), req).await;
        assert!(
            matches!(ip, RemoteIP::Forwarded(ip) if ip == "51.50.51.50".parse::<IpAddr>().unwrap())
        );
    }

    #[tokio::test]
    async fn missing_header_falls_back_to_socket() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let req = request(&[], Some(addr));
        let ip = resolve(Some(ClientIpSource::RightmostXForwardedFor), req).await;
        assert!(matches!(ip, RemoteIP::Socket(ip) if ip == addr.ip()));
    }

    #[tokio::test]
    async fn missing_header_and_no_connect_info_yields_none() {
        let req = request(&[], None);
        let ip = resolve(Some(ClientIpSource::RightmostXForwardedFor), req).await;
        assert!(matches!(ip, RemoteIP::None));
    }

    #[tokio::test]
    async fn connect_info_source_is_reported_as_socket() {
        let addr: SocketAddr = "10.1.1.1:9999".parse().unwrap();
        let req = request(&[], Some(addr));
        let ip = resolve(Some(ClientIpSource::ConnectInfo), req).await;
        assert!(matches!(ip, RemoteIP::Socket(ip) if ip == addr.ip()));
    }

    #[tokio::test]
    async fn dedicated_provider_header_is_supported() {
        let req = request(&[("cf-connecting-ip", "8.8.8.8")], None);
        let ip = resolve(Some(ClientIpSource::CfConnectingIp), req).await;
        assert!(
            matches!(ip, RemoteIP::Forwarded(ip) if ip == "8.8.8.8".parse::<IpAddr>().unwrap())
        );
    }
}
