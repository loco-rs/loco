use std::fmt;

use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};

/// User authentication configuration.
///
/// Example (development):
/// ```yaml
/// # config/development.yaml
/// auth:
///   jwt:
///     secret: <your secret>
///     expiration: 604800 # 7 days
///   # Signed cookies and the cookie session (`auth_session` feature).
///   session:
///     secret: <a long random string>
///   # Sign in with an OAuth2 provider (`auth_oauth` feature).
///   oauth:
///     providers:
///       github:
///         client_id: <client id>
///         client_secret: <client secret>
/// ```
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Auth {
    /// JWT authentication config
    pub jwt: Option<JWT>,
    /// Signed cookies and the cookie-backed session. Read by the
    /// `auth_session` feature; absent means development defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionConfig>,
    /// `OAuth2` / `OpenID` Connect sign-in providers. Read by the `auth_oauth`
    /// feature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth: Option<OAuthConfig>,
}

/// The secret every signed cookie is keyed off when `auth.session.secret` is
/// not set.
///
/// Fine on a laptop; `cargo loco doctor --production` reports it, and
/// a production app refuses to sign or read cookies with it.
pub const DEV_SESSION_SECRET: &str =
    "loco-development-session-secret-do-not-use-in-production-0123456789";

/// Signed cookies and the cookie session, Rails' `secret_key_base` plus its
/// cookie store.
///
/// ```yaml
/// auth:
///   session:
///     # Keys every signed cookie. Use 64+ random characters, e.g.
///     # `openssl rand -base64 64`, and inject it from the environment.
///     secret: <%= get_env(name="SESSION_SECRET") %>
///     # All optional:
///     cookie_name: loco_session
///     max_age: 2592000      # seconds; 30 days
///     secure: true          # default: true when `server.host` is https
///     sign_in_path: /login  # where `CurrentUser` sends signed-out browsers
/// ```
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SessionConfig {
    /// The key material for signed (and encrypted) cookies. Any length is
    /// accepted (it is stretched with SHA-512); use at least 32 random
    /// characters. Defaults to [`DEV_SESSION_SECRET`].
    pub secret: Option<String>,
    /// Name of the session cookie. Default `loco_session`.
    pub cookie_name: Option<String>,
    /// Session lifetime in seconds, enforced both by the cookie's `Max-Age`
    /// and by an expiry signed into the value. Default 30 days.
    pub max_age: Option<u64>,
    /// Force the `Secure` cookie flag on or off. Default: on when
    /// `server.host` starts with `https://`.
    pub secure: Option<bool>,
    /// Where `CurrentUser` redirects a signed-out browser (requests that
    /// accept HTML), with `?return_to=<original path>`. When unset, it answers
    /// `401 Unauthorized`.
    pub sign_in_path: Option<String>,
}

/// `OAuth2` / `OpenID` Connect sign-in, `OmniAuth`-style.
///
/// ```yaml
/// auth:
///   oauth:
///     # Public URL of the app, used to build each provider's callback URL
///     # (`{base_url}/auth/{provider}/callback`). Default: `server.host`
///     # plus `server.port`.
///     base_url: https://myapp.example.com
///     after_sign_in_path: /          # used when the flow carried no return_to
///     after_sign_out_path: /
///     failure_path: /login           # optional; see `OAuthHandler::on_failure`
///     providers:
///       github:
///         client_id: <%= get_env(name="GITHUB_CLIENT_ID") %>
///         client_secret: <%= get_env(name="GITHUB_CLIENT_SECRET") %>
///       google:
///         client_id: ...
///         client_secret: ...
///       gitlab:
///         strategy: generic
///         client_id: ...
///         client_secret: ...
///         authorize_url: https://gitlab.com/oauth/authorize
///         token_url: https://gitlab.com/oauth/token
///         userinfo_url: https://gitlab.com/oauth/userinfo
///         scopes: [openid, profile, email]
/// ```
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct OAuthConfig {
    /// Public base URL used for the `redirect_uri` sent to providers.
    pub base_url: Option<String>,
    /// Where to land after sign-in when the flow carried no `return_to` and
    /// the handler named no destination. Default `/`.
    pub after_sign_in_path: Option<String>,
    /// Where the sign-out route redirects. Default `/`.
    pub after_sign_out_path: Option<String>,
    /// When set, the default failure handler redirects here with
    /// `?error=<kind>&provider=<name>` (`OmniAuth`'s `/auth/failure`) instead of
    /// rendering a small error page.
    pub failure_path: Option<String>,
    /// Providers by name. The name is the URL segment: `github` is served at
    /// `/auth/github`.
    #[serde(default)]
    pub providers: std::collections::BTreeMap<String, OAuthProviderConfig>,
}

/// Which provider preset a configured provider uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OAuthStrategy {
    /// github.com, or GitHub Enterprise with `base_url`/`api_url`.
    #[serde(alias = "GitHub")]
    Github,
    /// Google (`OpenID` Connect).
    Google,
    /// Any `OAuth2` / OIDC provider; `authorize_url`, `token_url` and
    /// `userinfo_url` are required.
    #[serde(alias = "oidc", alias = "oauth2")]
    Generic,
}

/// How the client authenticates to the token endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthTokenAuth {
    /// `client_id`/`client_secret` in the form body (`client_secret_post`).
    #[default]
    Post,
    /// HTTP Basic auth (`client_secret_basic`).
    Basic,
}

/// One OAuth provider. Every URL is overridable, for GitHub Enterprise,
/// self-hosted providers and tests against a fake server.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct OAuthProviderConfig {
    /// The preset. Defaults from the provider's name (`github`, `google`);
    /// any other name needs `strategy: generic`.
    pub strategy: Option<OAuthStrategy>,
    /// OAuth client id.
    pub client_id: String,
    /// OAuth client secret.
    pub client_secret: String,
    /// Scopes to request. Defaults: GitHub `[read:user, user:email]`,
    /// Google and generic `[openid, email, profile]`.
    pub scopes: Option<Vec<String>>,
    /// GitHub only: web base URL, e.g. `https://github.example.com` for GitHub
    /// Enterprise. Default `https://github.com`.
    pub base_url: Option<String>,
    /// GitHub only: API base URL. Default `https://api.github.com`, or
    /// `{base_url}/api/v3` when `base_url` is set.
    pub api_url: Option<String>,
    /// Authorization endpoint (the page the browser is sent to).
    pub authorize_url: Option<String>,
    /// Token endpoint (code exchange).
    pub token_url: Option<String>,
    /// Profile endpoint, called with the access token.
    pub userinfo_url: Option<String>,
    /// GitHub only: the emails endpoint used to find the primary, verified
    /// address. Default `{api_url}/user/emails`.
    pub emails_url: Option<String>,
    /// Exact callback URL to register with the provider. Default
    /// `{auth.oauth.base_url}{mount}/{provider}/callback`.
    pub redirect_uri: Option<String>,
    /// Extra query parameters for the authorization request, e.g.
    /// `prompt: select_account` or `allow_signup: "false"`.
    #[serde(default)]
    pub authorize_params: std::collections::BTreeMap<String, String>,
    /// Send a PKCE (S256) challenge. Default `true`; providers that do not
    /// support PKCE ignore it.
    pub pkce: Option<bool>,
    /// Token endpoint client authentication. Default `post`.
    pub token_auth: Option<OAuthTokenAuth>,
}

/// JWT configuration structure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JWT {
    /// The location(s) where JWT tokens are expected to be found during
    /// authentication. Can be a single location or an array of locations.
    pub location: Option<JWTLocationConfig>,
    /// The secret key For JWT token
    pub secret: String,
    /// The expiration time for authentication tokens
    pub expiration: u64,
}

/// Defines the authentication mechanism for middleware.
///
/// This enum represents various ways to authenticate using JSON Web Tokens
/// (JWT) within middleware.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "from")]
pub enum JWTLocation {
    /// Authenticate using a Bearer token.
    Bearer,
    /// Authenticate using a token passed as a query parameter.
    Query { name: String },
    /// Authenticate using a token stored in a cookie.
    Cookie { name: String },
}

/// Configuration for JWT location(s) - supports both single location and multiple locations
///
/// `Deserialize` is written by hand rather than derived with
/// `#[serde(untagged)]`. An untagged enum reports only that the input "did not
/// match any variant", discarding *why* each variant failed — so a config with
/// `from: cookie` (wrong case) or a `Cookie` missing its `name` produced the
/// same opaque message, which was the single hardest thing to debug about this
/// setting. Dispatching on the input's shape instead lets the inner error
/// through: a map is a single location, a sequence is a fallback list, and
/// anything else names both.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum JWTLocationConfig {
    /// Single authentication location
    Single(JWTLocation),
    /// Multiple authentication locations (tried in order)
    Multiple(Vec<JWTLocation>),
}

impl<'de> Deserialize<'de> for JWTLocationConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LocationConfigVisitor;

        impl<'de> Visitor<'de> for LocationConfigVisitor {
            type Value = JWTLocationConfig;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(
                    "a JWT location map such as `{from: Cookie, name: auth_token}`, or a list of \
                     them to try in order",
                )
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                JWTLocation::deserialize(de::value::MapAccessDeserializer::new(map))
                    .map(JWTLocationConfig::Single)
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                Vec::<JWTLocation>::deserialize(de::value::SeqAccessDeserializer::new(seq))
                    .map(JWTLocationConfig::Multiple)
            }
        }

        deserializer.deserialize_any(LocationConfigVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(yaml: &str) -> Result<JWTLocationConfig, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    #[test]
    fn a_map_is_a_single_location() {
        let config = parse("from: Cookie\nname: auth_token").expect("a map should parse");
        assert!(matches!(
            config,
            JWTLocationConfig::Single(JWTLocation::Cookie { name }) if name == "auth_token"
        ));
    }

    #[test]
    fn a_list_is_a_fallback_chain() {
        let config = parse("- from: Cookie\n  name: auth_token\n- from: Bearer")
            .expect("a list should parse");
        let JWTLocationConfig::Multiple(locations) = config else {
            panic!("a YAML list should deserialize to `Multiple`");
        };
        assert_eq!(locations.len(), 2);
    }

    /// The point of the hand-written `Deserialize`: an untagged enum reported
    /// `data did not match any variant of untagged enum JWTLocationConfig` for
    /// every mistake below, naming neither the field at fault nor the accepted
    /// values.
    #[test]
    fn a_misspelled_source_names_the_accepted_values() {
        let err = parse("from: cookie\nname: auth_token").expect_err("`cookie` is not a variant");
        let message = err.to_string();

        assert!(
            message.contains("unknown variant") && message.contains("Cookie"),
            "the error should name the accepted sources, got: {message}"
        );
        assert!(
            !message.contains("untagged"),
            "the opaque untagged message should be gone, got: {message}"
        );
    }

    #[test]
    fn a_cookie_without_a_name_says_which_field_is_missing() {
        let err = parse("from: Cookie").expect_err("`Cookie` requires a `name`");
        let message = err.to_string();

        assert!(
            message.contains("name"),
            "the error should name the missing field, got: {message}"
        );
        assert!(
            !message.contains("untagged"),
            "the opaque untagged message should be gone, got: {message}"
        );
    }

    #[test]
    fn a_bare_scalar_describes_both_accepted_shapes() {
        let err = parse("Bearer").expect_err("a bare string is not a location");
        let message = err.to_string();

        assert!(
            message.contains("from: Cookie") && message.contains("list"),
            "the error should describe the map and the list forms, got: {message}"
        );
    }
}
