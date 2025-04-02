//! Remote IP Middleware for inferring the client's IP address based on the
//! `X-Forwarded-For` header.
//!
//! This middleware is useful when running behind proxies or load balancers that
//! add the `X-Forwarded-For` header, which includes the original client IP
//! address.
//!
//! The middleware provides a mechanism to configure trusted proxies and extract
//! the most likely client IP from the `X-Forwarded-For` header, skipping any
//! trusted proxy IPs.
use std::{
    fmt,
    iter::Iterator,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::OnceLock,
    task::{Context, Poll},
};

use axum::{
    body::Body,
    extract::{ConnectInfo, FromRequestParts, Request},
    http::{header::HeaderMap, request::Parts},
    response::Response,
    Router as AXRouter,
};
use futures_util::future::BoxFuture;
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use tower::{Layer, Service};
use tracing::error;

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Error, Result};

static LOCAL_TRUSTED_PROXIES: OnceLock<Vec<IpNetwork>> = OnceLock::new();

fn get_local_trusted_proxies() -> &'static Vec<IpNetwork> {
    LOCAL_TRUSTED_PROXIES.get_or_init(|| {
        [
            "127.0.0.0/8",   // localhost IPv4 range, per RFC-3330
            "::1",           // localhost IPv6
            "fc00::/7",      // private IPv6 range fc00::/7
            "10.0.0.0/8",    // private IPv4 range 10.x.x.x
            "172.16.0.0/12", // private IPv4 range 172.16.0.0 .. 172.31.255.255
            "192.168.0.0/16",
        ]
        .iter()
        .map(|ip| IpNetwork::from_str(ip).unwrap())
        .collect()
    })
}

const X_FORWARDED_FOR: &str = "X-Forwarded-For";

///
/// Performs a remote ip "calculation", inferring the most likely
/// client IP from the `X-Forwarded-For` header that is used by
/// load balancers and proxies.
///
/// WARNING
/// =======
///
/// LIKE ANY SUCH REMOTE IP MIDDLEWARE, IN THE WRONG ARCHITECTURE IT CAN MAKE
/// YOU VULNERABLE TO IP SPOOFING.
///
/// This middleware assumes that there is at least one proxy sitting around and
/// setting headers with the client's remote IP address. Otherwise any client
/// can claim to have any IP address by setting the `X-Forwarded-For` header.
///
/// DO NOT USE THIS MIDDLEWARE IF YOU DONT KNOW THAT YOU NEED IT
///
/// -- But if you need it, it's crucial to use it (since it's the only way to
/// get the original client IP)
///
/// This middleware is mostly implemented after the Rails `remote_ip`
/// middleware, and looking at other production Rust services with Axum, taking
/// the best of both worlds to balance performance and pragmatism.
///
/// Similarities to the Rails `remote_ip` middleware:
///
/// * Uses `X-Forwarded-For`
/// * Uses the same built-in trusted proxies list
/// * You can provide a list of `trusted_proxies` which will **replace** the
///   built-in trusted proxies
///
/// Differences from the Rails `remote_ip` middleware:
///
/// * You get an indication if the remote IP is actually resolved or is the
///   socket IP (no `X-Forwarded-For` header or could not parse)
/// * We do not not use the `Client-IP` header, or try to detect "spoofing"
///   (spoofing while doing remote IP resolution is virtually non-detectable)
/// * Order of filtering IPs from `X-Forwarded-For` is done according to [the de
///   facto spec](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Forwarded-For#selecting_an_ip_address)
///   "Trusted proxy list"
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct RemoteIpMiddleware {
    #[serde(default)]
    pub enable: bool,
    /// A list of alternative proxy list IP ranges and/or network range (will
    /// replace built-in proxy list)
    pub trusted_proxies: Option<Vec<String>>,
}

impl MiddlewareLayer for RemoteIpMiddleware {
    /// Returns the name of the middleware
    fn name(&self) -> &'static str {
        "remote_ip"
    }

    /// Returns whether the middleware is enabled or not
    fn is_enabled(&self) -> bool {
        self.enable
            && (self.trusted_proxies.is_none()
                || self.trusted_proxies.as_ref().is_some_and(|t| !t.is_empty()))
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    /// Applies the Remote IP middleware to the given Axum router.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Ok(app.layer(RemoteIPLayer::new(self)?))
    }
}

// implementation reference: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Forwarded-For
fn maybe_get_forwarded(
    headers: &HeaderMap,
    trusted_proxies: Option<&Vec<IpNetwork>>,
) -> Option<IpAddr> {
    /*
    > There may be multiple X-Forwarded-For headers present in a request. The IP addresses in these headers must be treated as a single list,
    > starting with the first IP address of the first header and continuing to the last IP address of the last header.
    > There are two ways of making this single list:
    > join the X-Forwarded-For full header values with commas and then split by comma into a list, or
    > split each X-Forwarded-For header by comma into lists and then join the lists
     */
    let xffs = headers
        .get_all(X_FORWARDED_FOR)
        .iter()
        .map(|hdr| hdr.to_str())
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    if xffs.is_empty() {
        return None;
    }

    let forwarded = xffs.join(",");

    forwarded
        .split(',')
        .map(str::trim)
        .map(str::parse)
        .filter_map(Result::ok)
        /*
        > Trusted proxy list: The IPs or IP ranges of the trusted reverse proxies are configured.
        > The X-Forwarded-For IP list is searched from the rightmost, skipping all addresses that
        > are on the trusted proxy list. The first non-matching address is the target address.
        */
        .filter(|ip| {
            // trusted proxies provided REPLACES our default local proxies
            let proxies = trusted_proxies.unwrap_or_else(|| get_local_trusted_proxies());
            !proxies
                .iter()
                .any(|trusted_proxy| trusted_proxy.contains(*ip))
        })
        /*
        > When choosing the X-Forwarded-For client IP address closest to the client (untrustworthy
        > and not for security-related purposes), the first IP from the leftmost that is a valid
        > address and not private/internal should be selected.
        >
        NOTE:
        > The first trustworthy X-Forwarded-For IP address may belong to an untrusted intermediate
        > proxy rather than the actual client computer, but it is the only IP suitable for security uses.
        */
        .next_back()
}

#[derive(Copy, Clone, Debug)]
pub enum RemoteIP {
    Forwarded(IpAddr),
    Socket(IpAddr),
    None,
}

impl<S> FromRequestParts<S> for RemoteIP
where
    S: Send + Sync,
{
    type Rejection = ();

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let ip = parts.extensions.get::<Self>();
        Ok(*ip.unwrap_or(&Self::None))
    }
}

impl fmt::Display for RemoteIP {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Forwarded(ip) => write!(f, "remote: {ip}"),
            Self::Socket(ip) => write!(f, "socket: {ip}"),
            Self::None => write!(f, "--"),
        }
    }
}

#[derive(Clone, Debug)]
struct RemoteIPLayer {
    trusted_proxies: Option<Vec<IpNetwork>>,
}

impl RemoteIPLayer {
    /// Returns new secure headers middleware
    ///
    /// # Errors
    /// Fails if invalid header values found
    pub fn new(config: &RemoteIpMiddleware) -> Result<Self> {
        Ok(Self {
            trusted_proxies: config
                .trusted_proxies
                .as_ref()
                .map(|proxies| {
                    proxies
                        .iter()
                        .map(|proxy| {
                            IpNetwork::from_str(proxy).map_err(|err| {
                                Error::Message(format!(
                                    "remote ip middleare cannot parse trusted proxy \
                                     configuration: `{proxy}`, reason: `{err}`",
                                ))
                            })
                        })
                        .collect::<Result<Vec<_>>>()
                })
                .transpose()?,
        })
    }
}

impl<S> Layer<S> for RemoteIPLayer {
    type Service = RemoteIPMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RemoteIPMiddleware {
            inner,
            layer: self.clone(),
        }
    }
}

/// Remote IP Detection Middleware
#[derive(Clone, Debug)]
#[must_use]
pub struct RemoteIPMiddleware<S> {
    inner: S,
    layer: RemoteIPLayer,
}

impl<S> Service<Request<Body>> for RemoteIPMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let layer = self.layer.clone();
        let xff_ip = maybe_get_forwarded(req.headers(), layer.trusted_proxies.as_ref());
        let remote_ip = xff_ip.map_or_else(
            || {
                let ip = req
                    .extensions()
                    .get::<ConnectInfo<SocketAddr>>()
                    .map_or_else(
                        || {
                            error!(
                                "remote ip middleware cannot get socket IP (not set in axum \
                                 extensions): setting IP to `127.0.0.1`"
                            );
                            RemoteIP::None
                        },
                        |info| RemoteIP::Socket(info.ip()),
                    );
                ip
            },
            RemoteIP::Forwarded,
        );

        req.extensions_mut().insert(remote_ip);

        Box::pin(self.inner.call(req))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use axum::http::{HeaderMap, HeaderName, HeaderValue};
    use insta::assert_debug_snapshot;
    use ipnetwork::IpNetwork;

    use super::maybe_get_forwarded;

    fn xff(val: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();

        headers.insert(
            HeaderName::from_static("x-forwarded-for"),
            HeaderValue::from_str(val).unwrap(),
        );
        headers
    }

    #[test]
    pub fn test_parsing() {
        let res = maybe_get_forwarded(&xff(""), None);
        assert_debug_snapshot!(res);
        let res = maybe_get_forwarded(&xff("foobar"), None);
        assert_debug_snapshot!(res);
        let res = maybe_get_forwarded(&xff("192.1.1.1"), None);
        assert_debug_snapshot!(res);
        let res = maybe_get_forwarded(&xff("51.50.51.50,10.0.0.1,192.168.1.1"), None);
        assert_debug_snapshot!(res);
        let res = maybe_get_forwarded(&xff("19.84.19.84,192.168.0.1"), None);
        assert_debug_snapshot!(res);
        let res = maybe_get_forwarded(&xff("b51.50.51.50b,/10.0.0.1-,192.168.1.1"), None);
        assert_debug_snapshot!(res);
        let res = maybe_get_forwarded(
            &xff("51.50.51.50,192.1.1.1"),
            Some(&vec![IpNetwork::from_str("192.1.1.1/8").unwrap()]),
        );
        assert_debug_snapshot!(res);

        // we replaced the proxy list, which is why 192.168.1.1 should appear as a valid
        // remote IP and not skipped
        let res = maybe_get_forwarded(
            &xff("51.50.51.50,192.168.1.1"),
            Some(&vec![IpNetwork::from_str("192.1.1.1/16").unwrap()]),
        );
        assert_debug_snapshot!(res);
    }
}
