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
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct UserClaims {
    pub pid: String,
    exp: u64,
    #[serde(default, flatten)]
    // TODO: should we wrap this in an Option? `Option<Map<String, Value>>`
    // so we can use `auth::jwt::JWT::new("PqRwLF2rhHe8J22oBeHy").generate_token(&604800, "PID".to_string(), None);
    // TODO: serde_json::Map or std::collections::HashMap?
    // TODO: is it ok to use a generic Map<String, Value> here? Or should we let the user specify their desired typed claim and
    // use generics to serialize/deserialize it?
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
    #[case("valid token", 60, Map::new())]
    #[case("token expired", 1, Map::new())]
    #[case("valid token and custom string claims", 60, json!({ "custom": "claim",}).as_object().unwrap().clone())]
    #[case("valid token and custom boolean claims",60, json!({ "custom": true,}).as_object().unwrap().clone())]
    #[case("valid token and custom nested claims",60, json!({ "level1": { "level2": { "level3": "claim" } } }).as_object().unwrap().clone())]
    #[case("valid token and custom array claims",60, json!({ "array": [1, 2, 3] }).as_object().unwrap().clone())]
    #[case("valid token and custom nested array claims",60, json!({ "level1": { "level2": { "level3": [1, 2, 3] } } }).as_object().unwrap().clone())]
    fn can_generate_token(
        #[case] test_name: &str,
        #[case] expiration: u64,
        #[case] claims: Map<String, Value>,
    ) {
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

    #[test]
    fn serialize_user_claims_without_custom_claims() {
        let user_claims = UserClaims {
            pid: "pid".to_string(),
            exp: 60,
            claims: Map::new(),
        };

        let expected_value = json!({
            "pid" : "pid",
            "exp": 60
        });
        assert_eq!(expected_value, serde_json::to_value(user_claims).unwrap());
    }

    #[test]
    fn serialize_user_claims_with_custom_string_claims() {
        let claims = json!({ "custom": "claim",}).as_object().unwrap().clone();
        let user_claims = UserClaims {
            pid: "pid".to_string(),
            exp: 60,
            claims,
        };

        let expected_value = json!({
            "pid" : "pid",
            "exp": 60,
            "custom": "claim"
        });
        assert_eq!(expected_value, serde_json::to_value(user_claims).unwrap());
    }

    #[test]
    fn serialize_user_claims_with_custom_boolean_claims() {
        let claims = json!({ "custom": true,}).as_object().unwrap().clone();
        let user_claims = UserClaims {
            pid: "pid".to_string(),
            exp: 60,
            claims,
        };

        let expected_value = json!({
            "pid" : "pid",
            "exp": 60,
            "custom": true
        });
        assert_eq!(expected_value, serde_json::to_value(user_claims).unwrap());
    }

    #[test]
    fn serialize_user_claims_with_custom_nested_claims() {
        let claims = json!({ "level1": { "level2": { "level3": "claim" } } })
            .as_object()
            .unwrap()
            .clone();
        let user_claims = UserClaims {
            pid: "pid".to_string(),
            exp: 60,
            claims,
        };

        let expected_value = json!({
            "pid" : "pid",
            "exp": 60,
            "level1": {
                "level2": {
                    "level3": "claim"
                }
            }
        });
        assert_eq!(expected_value, serde_json::to_value(user_claims).unwrap());
    }

    #[test]
    fn serialize_user_claims_with_custom_array_claims() {
        let claims = json!({ "array": [1, 2, 3] }).as_object().unwrap().clone();
        let user_claims = UserClaims {
            pid: "pid".to_string(),
            exp: 60,
            claims,
        };

        let expected_value = json!({
            "pid" : "pid",
            "exp": 60,
            "array": [1, 2, 3]
        });
        assert_eq!(expected_value, serde_json::to_value(user_claims).unwrap());
    }

    #[test]
    fn serialize_user_claims_with_custom_nested_array_claims() {
        let claims = json!({ "level1": { "level2": { "level3": [1, 2, 3] } } })
            .as_object()
            .unwrap()
            .clone();
        let user_claims = UserClaims {
            pid: "pid".to_string(),
            exp: 60,
            claims,
        };

        let expected_value = json!({
            "pid" : "pid",
            "exp": 60,
            "level1": {
                "level2": {
                    "level3": [1, 2, 3]
                }
            }
        });
        assert_eq!(expected_value, serde_json::to_value(user_claims).unwrap());
    }

    #[test]
    fn deserialize_user_claims_without_custom_claims() {
        let json_claims = json!({
            "pid" : "pid",
            "exp": 60
        })
        .to_string();

        let expected_user_claims = UserClaims {
            pid: "pid".to_string(),
            exp: 60,
            claims: Map::new(),
        };

        assert_eq!(
            expected_user_claims,
            serde_json::from_str(&json_claims).unwrap()
        );
    }

    #[test]
    fn deserialize_user_claims_with_custom_string_claims() {
        let json_claims = json!({
            "pid" : "pid",
            "exp": 60,
            "custom": "claim"
        })
        .to_string();

        let expected_claims = json!({ "custom": "claim",}).as_object().unwrap().clone();
        let expected_user_claims = UserClaims {
            pid: "pid".to_string(),
            exp: 60,
            claims: expected_claims,
        };

        assert_eq!(
            expected_user_claims,
            serde_json::from_str(&json_claims).unwrap()
        );
    }

    #[test]
    fn deserialize_user_claims_with_custom_boolean_claims() {
        let json_claims = json!({
            "pid" : "pid",
            "exp": 60,
            "custom": true
        })
        .to_string();

        let expected_claims = json!({ "custom": true,}).as_object().unwrap().clone();
        let expected_user_claims = UserClaims {
            pid: "pid".to_string(),
            exp: 60,
            claims: expected_claims,
        };

        assert_eq!(
            expected_user_claims,
            serde_json::from_str(&json_claims).unwrap()
        );
    }

    #[test]
    fn deserialize_user_claims_with_custom_nested_claims() {
        let json_claims = json!({
            "pid" : "pid",
            "exp": 60,
            "level1": {
                "level2": {
                    "level3": "claim"
                }
            }
        })
        .to_string();

        let expected_claims = json!({ "level1": { "level2": { "level3": "claim" } } })
            .as_object()
            .unwrap()
            .clone();
        let expected_user_claims = UserClaims {
            pid: "pid".to_string(),
            exp: 60,
            claims: expected_claims,
        };

        assert_eq!(
            expected_user_claims,
            serde_json::from_str(&json_claims).unwrap()
        );
    }

    #[test]
    fn deserialize_user_claims_with_custom_array_claims() {
        let json_claims = json!({
            "pid" : "pid",
            "exp": 60,
            "array": [1, 2, 3]
        })
        .to_string();

        let expected_claims = json!({ "array": [1, 2, 3] }).as_object().unwrap().clone();
        let expected_user_claims = UserClaims {
            pid: "pid".to_string(),
            exp: 60,
            claims: expected_claims,
        };

        assert_eq!(
            expected_user_claims,
            serde_json::from_str(&json_claims).unwrap()
        );
    }

    #[test]
    fn deserialize_user_claims_with_custom_nested_array_claims() {
        let json_claims = json!({
            "pid" : "pid",
            "exp": 60,
            "level1": {
                "level2": {
                    "level3": [1, 2, 3]
                }
            }
        })
        .to_string();

        let expected_claims = json!({ "level1": { "level2": { "level3": [1, 2, 3] } } })
            .as_object()
            .unwrap()
            .clone();
        let expected_user_claims = UserClaims {
            pid: "pid".to_string(),
            exp: 60,
            claims: expected_claims,
        };

        assert_eq!(
            expected_user_claims,
            serde_json::from_str(&json_claims).unwrap()
        );
    }
}
