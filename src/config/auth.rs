use serde::{Deserialize, Serialize};

/// User authentication configuration.
///
/// Example (development):
/// ```yaml
/// # config/development.yaml
/// auth:
///   jwt:
///     secret: <your secret>
///     expiration: 604800 # 7 days
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Auth {
    /// JWT authentication config
    pub jwt: Option<JWT>,
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
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum JWTLocationConfig {
    /// Single authentication location
    Single(JWTLocation),
    /// Multiple authentication locations (tried in order)
    Multiple(Vec<JWTLocation>),
}
