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
