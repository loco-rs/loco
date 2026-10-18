//! # JSON Web Token (JWT) and Password Hashing
//!
//! This module provides functionality for working with JSON Web Tokens (JWTs)
//! and password hashing.
//!
//! Loco signs tokens with a shared secret, so only the HMAC algorithms apply.
//! They are implemented here on `hmac` + `sha2` (RFC 7519 / RFC 7515 compact
//! serialization) instead of a general-purpose JWT crate, whose RSA and
//! elliptic-curve backends a shared-secret token never uses.
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use hmac::{Hmac, Mac};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Sha256, Sha384, Sha512};

/// Represents the default JWT algorithm used by the [`JWT`] struct.
const JWT_ALGORITHM: JWTAlgorithm = JWTAlgorithm::HS512;

/// The HMAC-based JWT signing algorithms supported by [`JWT`].
///
/// [`JWT`] always derives its signing/verification key from a shared
/// base64-encoded secret, so only the HMAC family of algorithms can ever work
/// here. Asymmetric algorithms (RSA, EC, `EdDSA`, ...) require a
/// public/private key pair, not a shared secret, so they are intentionally not
/// exposed through [`JWT::algorithm`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JWTAlgorithm {
    HS256,
    HS384,
    HS512,
}

impl JWTAlgorithm {
    fn sign(self, key: &[u8], message: &[u8]) -> Vec<u8> {
        fn run<M: Mac + hmac::digest::KeyInit>(key: &[u8], message: &[u8]) -> Vec<u8> {
            let mut mac = <M as Mac>::new_from_slice(key).expect("HMAC accepts any key length");
            mac.update(message);
            mac.finalize().into_bytes().to_vec()
        }
        match self {
            Self::HS256 => run::<Hmac<Sha256>>(key, message),
            Self::HS384 => run::<Hmac<Sha384>>(key, message),
            Self::HS512 => run::<Hmac<Sha512>>(key, message),
        }
    }

    /// Constant-time signature check.
    fn verify(self, key: &[u8], message: &[u8], signature: &[u8]) -> bool {
        fn run<M: Mac + hmac::digest::KeyInit>(key: &[u8], message: &[u8], sig: &[u8]) -> bool {
            let mut mac = <M as Mac>::new_from_slice(key).expect("HMAC accepts any key length");
            mac.update(message);
            mac.verify_slice(sig).is_ok()
        }
        match self {
            Self::HS256 => run::<Hmac<Sha256>>(key, message, signature),
            Self::HS384 => run::<Hmac<Sha384>>(key, message, signature),
            Self::HS512 => run::<Hmac<Sha512>>(key, message, signature),
        }
    }
}

/// Why a token could not be generated or validated.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum JWTError {
    #[error("the JWT secret is not valid base64: {0}")]
    InvalidSecret(base64::DecodeError),
    #[error("the token is not a `header.claims.signature` triple")]
    InvalidToken,
    #[error("a token segment is not valid base64url: {0}")]
    Base64(base64::DecodeError),
    #[error("the token header or claims are not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the token was signed with a different algorithm")]
    InvalidAlgorithm,
    #[error("the token signature does not match")]
    InvalidSignature,
    #[error("the token has expired")]
    ExpiredSignature,
    #[error("the token carries an `aud` claim, which this validator does not accept")]
    InvalidAudience,
}

pub type JWTResult<T> = std::result::Result<T, JWTError>;

/// The JOSE header of a token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typ: Option<String>,
    pub alg: JWTAlgorithm,
}

/// A validated token: its header and decoded claims.
#[derive(Debug, Clone)]
pub struct TokenData<T> {
    pub header: Header,
    pub claims: T,
}

/// Represents the claims associated with a user JWT.
#[cfg_attr(test, derive(Eq, PartialEq))]
#[derive(Debug, Serialize, Deserialize)]
pub struct UserClaims {
    pub pid: String,
    exp: u64,
    #[serde(default, flatten)]
    pub claims: Map<String, Value>,
}

/// Represents the JWT configuration and operations.
///
/// # Example
/// ```rust
/// use loco_rs::auth;
///
/// auth::jwt::JWT::new("PqRwLF2rhHe8J22oBeHy");
/// ```
#[derive(Debug)]
pub struct JWT {
    secret: String,
    algorithm: JWTAlgorithm,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn decode_segment<T: DeserializeOwned>(segment: &str) -> JWTResult<T> {
    let bytes = URL_SAFE_NO_PAD.decode(segment).map_err(JWTError::Base64)?;
    Ok(serde_json::from_slice(&bytes)?)
}

impl JWT {
    /// Creates a new [`JWT`] instance with the specified secret key.
    #[must_use]
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.to_string(),
            algorithm: JWT_ALGORITHM,
        }
    }

    /// Override the default JWT algorithm to be used.
    ///
    /// Only HMAC algorithms ([`JWTAlgorithm::HS256`], [`JWTAlgorithm::HS384`],
    /// [`JWTAlgorithm::HS512`]) are accepted, since [`JWT`] always signs and
    /// verifies with a shared base64-encoded secret. Asymmetric algorithms
    /// cannot work with a shared secret and are therefore not selectable.
    #[must_use]
    pub fn algorithm(mut self, algorithm: JWTAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    fn key(&self) -> JWTResult<Vec<u8>> {
        STANDARD
            .decode(&self.secret)
            .map_err(JWTError::InvalidSecret)
    }

    /// Generates a new JWT with specified claims and an expiration time.
    ///
    /// # Errors
    ///
    /// returns [`JWTResult`] error when could not generate JWT token. can be an
    /// invalid secret.
    ///
    /// # Example
    /// ```rust
    /// use serde_json::Map;
    /// use loco_rs::auth;
    ///
    /// auth::jwt::JWT::new("PqRwLF2rhHe8J22oBeHy").generate_token(604800, "PID".to_string(), Map::new());
    /// ```
    pub fn generate_token(
        &self,
        expiration: u64,
        pid: String,
        claims: Map<String, Value>,
    ) -> JWTResult<String> {
        let key = self.key()?;
        let exp = now().saturating_add(expiration);
        let claims = UserClaims { pid, exp, claims };
        let header = Header {
            typ: Some("JWT".to_string()),
            alg: self.algorithm,
        };

        let mut token = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header)?);
        token.push('.');
        token.push_str(&URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims)?));
        let signature = self.algorithm.sign(&key, token.as_bytes());
        token.push('.');
        token.push_str(&URL_SAFE_NO_PAD.encode(signature));
        Ok(token)
    }

    /// Validates the authenticity and expiration of a given JWT.
    /// If Token is valid, decode the Token Claims.
    ///
    /// # Errors
    ///
    /// returns [`JWTResult`] error when could not convert the given token to
    /// [`UserClaims`], if the `secret` is invalid or token is expired.
    ///
    /// # Example
    /// ```rust
    /// use loco_rs::auth;
    /// auth::jwt::JWT::new("PqRwLF2rhHe8J22oBeHy").validate("JWT-TOKEN");
    /// ```
    pub fn validate(&self, token: &str) -> JWTResult<TokenData<UserClaims>> {
        let (message, signature) = token.rsplit_once('.').ok_or(JWTError::InvalidToken)?;
        let (header, claims) = message.split_once('.').ok_or(JWTError::InvalidToken)?;
        if claims.contains('.') {
            return Err(JWTError::InvalidToken);
        }

        let key = self.key()?;
        let header: Header = decode_segment(header)?;
        if header.alg != self.algorithm {
            return Err(JWTError::InvalidAlgorithm);
        }
        let signature = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(JWTError::Base64)?;
        if !self.algorithm.verify(&key, message.as_bytes(), &signature) {
            return Err(JWTError::InvalidSignature);
        }

        let raw: Map<String, Value> = decode_segment(claims)?;
        // Kept from the previous `jsonwebtoken` validator: an `aud` claim is
        // rejected unless an audience is configured, and none can be.
        if matches!(raw.get("aud"), Some(Value::String(_) | Value::Array(_))) {
            return Err(JWTError::InvalidAudience);
        }
        let claims: UserClaims = serde_json::from_value(Value::Object(raw))?;
        if claims.exp < now() {
            return Err(JWTError::ExpiredSignature);
        }

        Ok(TokenData { header, claims })
    }
}

#[cfg(test)]
mod tests {

    use insta::{assert_debug_snapshot, with_settings};
    use rstest::rstest;
    use serde_json::json;

    use super::*;

    #[rstest]
    #[case("valid token", 60, json!({}))]
    #[case("token expired", 1, json!({}))]
    #[case("valid token and custom string claims", 60, json!({ "custom": "claim",}))]
    #[case("valid token and custom boolean claims",60, json!({ "custom": true,}))]
    #[case("valid token and custom number claims",60, json!({ "custom": 123,}))]
    #[case("valid token and custom nested claims",60, json!({ "level1": { "level2": { "level3": "claim" } } }))]
    #[case("valid token and custom array claims",60, json!({ "array": [1, 2, 3] }))]
    #[case("valid token and custom nested array claims",60, json!({ "level1": { "level2": { "level3": [1, 2, 3] } } }))]
    fn can_generate_token(
        #[case] test_name: &str,
        #[case] expiration: u64,
        #[case] json_claims: Value,
    ) {
        let claims = json_claims
            .as_object()
            .expect("case input claims must be an object")
            .clone();
        let jwt = JWT::new("PqRwLF2rhHe8J22oBeHy");

        let token = jwt
            .generate_token(expiration, "pid".to_string(), claims)
            .unwrap();

        std::thread::sleep(std::time::Duration::from_secs(3));
        with_settings!({filters => vec![
            (r"exp: (\d+),", "exp: EXP,")
        ]}, {
            assert_debug_snapshot!(test_name, jwt.validate(&token));
        });
    }

    #[rstest]
    #[case(JWTAlgorithm::HS256)]
    #[case(JWTAlgorithm::HS384)]
    #[case(JWTAlgorithm::HS512)]
    fn can_round_trip_with_hmac_algorithm(#[case] algorithm: JWTAlgorithm) {
        let jwt = JWT::new("PqRwLF2rhHe8J22oBeHy").algorithm(algorithm);

        let token = jwt
            .generate_token(60, "pid".to_string(), Map::new())
            .unwrap();

        let claims = jwt.validate(&token).unwrap();
        assert_eq!(claims.claims.pid, "pid");
        assert_eq!(algorithm, jwt.algorithm);
    }

    #[rstest]
    #[case::without_custom_claims(json!({}))]
    #[case::with_custom_string_claims(json!({ "custom": "claim",}))]
    #[case::with_custom_boolean_claims(json!({ "custom": true,}))]
    #[case::with_custom_number_claims(json!({ "custom": 123,}))]
    #[case::with_custom_nested_claims(json!({ "level1": { "level2": { "level3": "claim" } } }))]
    #[case::with_custom_array_claims(json!({ "array": [1, 2, 3] }))]
    #[case::with_custom_nested_array_claims(json!({ "level1": { "level2": { "level3": [1, 2, 3] } } }))]
    // we use `Value` to reduce code duplicity in the case inputs
    fn serialize_user_claims(#[case] json_claims: Value) {
        let claims = json_claims
            .as_object()
            .expect("case input claims must be an object")
            .clone();
        let input_user_claims = UserClaims {
            pid: "pid".to_string(),
            exp: 60,
            claims: claims.clone(),
        };

        let mut expected_claim = Map::new();
        expected_claim.insert("pid".to_string(), "pid".into());
        expected_claim.insert("exp".to_string(), 60.into());
        // we add the claims in a flattened way
        expected_claim.extend(claims);
        let expected_value = Value::from(expected_claim);

        // We check between `Value` instead of `String` to avoid key ordering issues when serializing.
        // It is because `expected_value` has all the keys in alphabetical order, as the `Value` serialization ensures that.
        // But when serializing `input_user_claims`, first the `pid` and `exp` fields are serialized (in that order),
        // and then the claims are serialized in alfabetic order. So, the resulting JSON string from the `input_user_claims` serialization
        // may have the `pid` and `exp` fields unordered which differs from the `Value` serialization.
        assert_eq!(
            expected_value,
            serde_json::to_value(&input_user_claims).unwrap()
        );
    }

    #[rstest]
    #[case::without_custom_claims(json!({}))]
    #[case::with_custom_string_claims(json!({ "custom": "claim",}))]
    #[case::with_custom_boolean_claims(json!({ "custom": true,}))]
    #[case::with_custom_number_claims(json!({ "custom": 123,}))]
    #[case::with_custom_nested_claims(json!({ "level1": { "level2": { "level3": "claim" } } }))]
    #[case::with_custom_array_claims(json!({ "array": [1, 2, 3] }))]
    #[case::with_custom_nested_array_claims(json!({ "level1": { "level2": { "level3": [1, 2, 3] } } }))]
    // we use `Value` to reduce code duplicity in the case inputs
    fn deserialize_user_claims(#[case] json_claims: Value) {
        let claims = json_claims
            .as_object()
            .expect("case input claims must be an object")
            .clone();

        let mut input_claims = Map::new();
        input_claims.insert("pid".to_string(), "pid".into());
        input_claims.insert("exp".to_string(), 60.into());
        // we add the claims in a flattened way
        input_claims.extend(claims.clone());
        let input_json = Value::from(input_claims).to_string();

        let expected_user_claims = UserClaims {
            pid: "pid".to_string(),
            exp: 60,
            claims,
        };

        assert_eq!(
            expected_user_claims,
            serde_json::from_str(&input_json).unwrap()
        );
    }
}
