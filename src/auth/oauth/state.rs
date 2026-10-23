//! The short-lived state cookie that ties a callback to the browser that
//! started the flow (CSRF), plus the PKCE verifier and `return_to`.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// How long a user has to finish at the provider: 10 minutes.
pub const STATE_TTL_SECONDS: u64 = 600;

const MAX_RETURN_TO: usize = 2048;

/// What the start of the flow leaves for the callback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowState {
    /// The `state` sent to the provider.
    pub state: String,
    /// PKCE code verifier, when PKCE is on.
    pub verifier: Option<String>,
    /// Same-site path to land on after sign-in.
    pub return_to: Option<String>,
}

impl FlowState {
    /// A fresh state (and verifier, when `pkce`).
    #[must_use]
    pub fn new(pkce: bool, return_to: Option<&str>) -> Self {
        Self {
            state: random_token(),
            verifier: pkce.then(random_token),
            return_to: return_to.and_then(safe_return_to),
        }
    }

    /// Whether `state` (from the callback query) matches, in constant time.
    #[must_use]
    pub fn matches(&self, state: &str) -> bool {
        let (a, b) = (self.state.as_bytes(), state.as_bytes());
        a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
    }

    /// The PKCE S256 challenge for the verifier.
    #[must_use]
    pub fn challenge(&self) -> Option<String> {
        self.verifier.as_deref().map(pkce_challenge)
    }
}

/// The state cookie for `provider`. One per provider, so two tabs signing in
/// with different providers do not clobber each other.
#[must_use]
pub fn cookie_name(provider: &str) -> String {
    format!("loco_oauth_state_{provider}")
}

/// 32 random bytes, base64url.
#[must_use]
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// `BASE64URL(SHA256(verifier))` (RFC 7636 S256).
#[must_use]
pub fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// `value` if it is a path on this site, else `None`.
///
/// Accepts only absolute paths. Rejects `//evil.example` and `/\evil.example`
/// (both protocol-relative to browsers), any backslash, control characters,
/// and anything with a scheme — so a crafted sign-in link cannot bounce the
/// user to another site after they authenticate (open redirect).
#[must_use]
pub fn safe_return_to(value: &str) -> Option<String> {
    let ok = value.starts_with('/')
        && !value.starts_with("//")
        && value.len() <= MAX_RETURN_TO
        && !value.contains('\\')
        && !value.chars().any(char::is_control);
    ok.then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_to_accepts_only_same_site_paths() {
        for good in ["/", "/orgs/acme", "/a?b=c#d", "/path//double"] {
            assert_eq!(safe_return_to(good).as_deref(), Some(good), "{good}");
        }
        for bad in [
            "",
            "https://evil.example",
            "//evil.example",
            "/\\evil.example",
            "\\\\evil.example",
            "javascript:alert(1)",
            "evil.example/path",
            "/\tx",
            "/a\r\nSet-Cookie: x=1",
        ] {
            assert_eq!(safe_return_to(bad), None, "{bad:?}");
        }
        assert_eq!(safe_return_to(&format!("/{}", "a".repeat(5000))), None);
    }

    #[test]
    fn states_are_random_and_compared_exactly() {
        let a = FlowState::new(true, Some("//evil"));
        let b = FlowState::new(true, None);
        assert_ne!(a.state, b.state);
        assert_ne!(a.verifier, b.verifier);
        assert_eq!(a.return_to, None, "an unsafe return_to is dropped");
        assert!(a.matches(&a.state));
        assert!(!a.matches(&b.state));
        assert!(!a.matches(""));
        assert!(!a.matches(&a.state[..a.state.len() - 1]));
        assert_eq!(FlowState::new(false, None).verifier, None);
    }

    #[test]
    fn pkce_matches_the_rfc_7636_example() {
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }
}
