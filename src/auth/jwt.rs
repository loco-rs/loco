//! # JSON Web Token (JWT) and Password Hashing
//!
//! This module provides functionality for working with JSON Web Tokens (JWTs)
//! and password hashing.
use jsonwebtoken::{
    decode, encode, errors::Result as JWTResult, get_current_timestamp, Algorithm, DecodingKey,
    EncodingKey, Header, TokenData, Validation,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Represents the default JWT algorithm used by the [`JWT`] struct.
const JWT_ALGORITHM: Algorithm = Algorithm::HS512;

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
    algorithm: Algorithm,
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

    /// Override the default  JWT algorithm to be used.
    #[must_use]
    pub fn algorithm(mut self, algorithm: Algorithm) -> Self {
        self.algorithm = algorithm;
        self
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
        let exp = get_current_timestamp().saturating_add(expiration);

        let claims = UserClaims { pid, exp, claims };

        let token = encode(
            &Header::new(self.algorithm),
            &claims,
            &EncodingKey::from_base64_secret(&self.secret)?,
        )?;

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
        let mut validate = Validation::new(self.algorithm);
        validate.leeway = 0;

        decode::<UserClaims>(
            token,
            &DecodingKey::from_base64_secret(&self.secret)?,
            &validate,
        )
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

        // We check between `Value` instead of `String` to avoid key ordering issues
        // when serializing. It is because `expected_value` has all the keys in
        // alphabetical order, as the `Value` serialization ensures that.
        // But when serializing `input_user_claims`, first the `pid` and `exp` fields
        // are serialized (in that order), and then the claims are serialized in
        // alfabetic order. So, the resulting JSON string from the `input_user_claims`
        // serialization may have the `pid` and `exp` fields unordered which
        // differs from the `Value` serialization.
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
