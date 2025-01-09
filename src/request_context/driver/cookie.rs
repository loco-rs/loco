use std::{collections::HashMap, convert::Infallible};

use axum::{
    http::HeaderMap,
    response::{IntoResponse, IntoResponseParts, ResponseParts},
};
use axum_extra::extract::cookie::{Cookie, Key, PrivateCookieJar};
use hyper::header::{COOKIE, SET_COOKIE};
use tower_sessions::{cookie, cookie::time};

use crate::controller::middleware::request_context::{SameSite, SessionCookieConfig};

/// `CookieMap` is a wrapper around a hashmap that stores the data for request
/// context
#[derive(Debug, Clone)]
pub struct CookieMap(HashMap<String, serde_json::Value>);

impl CookieMap {
    /// Create a new instance of the cookie map
    /// # Arguments
    /// * `map` - The hashmap to store the data
    /// # Return
    /// `Self` - The cookie map instance
    #[must_use]
    pub(crate) fn new(map: HashMap<String, serde_json::Value>) -> Self {
        Self(map)
    }
    /// Check if the cookie map is empty
    /// # Return
    /// * `bool` - True if the cookie map is empty, otherwise false
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Inserts a `impl Serialize` value into the cookie map.
    /// # Arguments
    /// * `key` - The key to store the value
    /// * `value` - The value to store
    /// # Errors
    /// * `CookieMapError` - When the value is unable to be serialized
    pub fn insert<T>(&mut self, key: &str, value: T) -> Result<(), CookieMapError>
    where
        T: serde::Serialize,
    {
        let value = serde_json::to_value(value).map_err(|e| {
            tracing::error!(?e, "Failed to serialize value");
            CookieMapError::Serde(e)
        })?;
        self.0.insert(key.to_string(), value);
        Ok(())
    }

    /// Gets a value from the cookie map.
    ///
    /// # Arguments
    /// * `key` - The key to get the value
    ///
    /// # Return
    /// * `Option<T>` - The value if found, otherwise None
    ///
    /// # Errors
    /// * `CookieMapError` - When the value is unable to be deserialized
    pub fn get<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, CookieMapError> {
        let value = self
            .0
            .get(key)
            .map(|value| serde_json::from_value(value.clone()));
        match value {
            Some(Ok(value)) => Ok(Some(value)),
            Some(Err(e)) => {
                tracing::error!(?e, "Failed to deserialize value");
                Err(CookieMapError::Serde(e))
            }
            None => Ok(None),
        }
    }

    /// Removes a value from the cookie map.
    ///
    /// # Arguments
    /// * `key` - The key to remove from the store
    ///
    /// # Return
    /// * `Option<T>` - The value if found, otherwise None
    ///
    /// # Errors
    /// * `CookieMapError` - When the value is unable to be deserialized
    pub fn remove<T: serde::de::DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, CookieMapError> {
        let value = self.0.remove(key);
        value.map_or_else(
            || Ok(None),
            |value| {
                let value = serde_json::from_value(value);
                match value {
                    Ok(value) => Ok(Some(value)),
                    Err(e) => {
                        tracing::error!(?e, "Failed to deserialize value");
                        Err(CookieMapError::Serde(e))
                    }
                }
            },
        )
    }

    /// Clears the cookie map.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Return if key exists
    #[must_use]
    pub fn exists(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }
}

impl Default for CookieMap {
    fn default() -> Self {
        Self::new(HashMap::new())
    }
}

impl TryFrom<CookieMap> for String {
    type Error = CookieMapError;
    fn try_from(value: CookieMap) -> Result<Self, Self::Error> {
        let value = serde_json::to_string(&value.0).map_err(|e| {
            tracing::error!(?e, "Failed to serialize hashmap to string");
            Self::Error::Serde(e)
        })?;
        Ok(value)
    }
}

impl TryFrom<String> for CookieMap {
    type Error = CookieMapError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let map: HashMap<String, serde_json::Value> =
            serde_json::from_str(&value).map_err(|e| {
                tracing::error!(?e, "Failed to deserialize hashmap string");
                Self::Error::Serde(e)
            })?;
        Ok(Self::new(map))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum CookieMapError {
    #[error("Serde error")]
    Serde(serde_json::Error),
    #[error("Max capacity error")]
    MaxCapacity,
}

impl PartialEq for CookieMapError {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Serde(_), Self::Serde(_)) | (Self::MaxCapacity, Self::MaxCapacity)
        )
    }
}
/// `SignedPrivateCookieJar` is for converting the incoming request headers into
/// a private cookie jar then cookie map and vice versa.
///
/// The private cookie jar is used to store the encrypted cookie map data in the
/// incoming request.
///
/// The [Aes256Gcm Algorithm](https://docs.rs/cookie/0.18.1/src/cookie/secure/private.rs.html#60-62) used by [`cookie::secure::PrivateJar`](https://docs.rs/cookie/0.18.1/src/cookie/secure/private.rs.html#60) which used by [`axum_extra::extract::PrivateCookieJar`](https://docs.rs/axum-extra/latest/src/axum_extra/extract/cookie/private.rs.html#108) to encrypt the cookie map data and provided confidentiality, integrity, and authenticity.
#[derive(Debug, Clone)]
pub struct SignedPrivateCookieJar {
    jar: PrivateCookieJar,
    config_map: HashMap<String, SessionCookieConfig>,
}

impl SignedPrivateCookieJar {
    /// Create a new instance of the signed private cookie jar
    ///
    /// # Arguments
    /// * `headers` - The headers from the incoming request
    /// * `private_key` - The private key to sign the cookie
    ///
    /// # Return
    /// * `Self` - The signed private cookie jar
    ///
    /// # Errors
    /// * `SignedPrivateCookieJarError` - When the cookie config is unable to be
    ///   extracted
    pub fn new(headers: &HeaderMap, private_key: Key) -> Result<Self, SignedPrivateCookieJarError> {
        let private_jar = PrivateCookieJar::from_headers(headers, private_key);
        let cookie_config_map = SessionCookieConfig::extract_cookie_config(headers)?;

        Ok(Self {
            jar: private_jar,
            config_map: cookie_config_map,
        })
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<Cookie<'static>> {
        self.jar.get(name).map(|mut cookie| {
            if let Some(attrs) = self.config_map.get(name) {
                attrs.apply_cookie_config(&mut cookie);
            }
            cookie
        })
    }

    /// Create a new instance of the signed private cookie jar if the cookie map
    /// is not empty
    ///
    /// # Arguments
    /// * `private_key` - The private key to sign the cookie
    /// * `map` - The cookie map to create the private cookie jar
    ///
    /// # Return
    /// * `Option<Self>` - The signed private cookie jar if the cookie map is
    ///   not empty, otherwise None
    ///
    /// # Errors
    /// * `SignedPrivateCookieJarError` - When cookie map unable to be converted
    ///   to string
    pub fn from_cookie_map(
        private_key: &Key,
        map: CookieMap,
        config: &SessionCookieConfig,
    ) -> Result<Option<Self>, SignedPrivateCookieJarError> {
        if map.is_empty() {
            return Ok(None);
        }
        let private_jar = PrivateCookieJar::new(private_key.clone());
        let map_string = String::try_from(map).map_err(|e| {
            tracing::error!(?e, "Failed to convert cookie map to string");
            SignedPrivateCookieJarError::CookieMap(e)
        })?;
        let mut cookie = Cookie::new(config.name.clone(), map_string);
        config.apply_cookie_config(&mut cookie);

        let mut cookie_config_map = HashMap::new();
        cookie_config_map.insert(config.name.clone(), config.clone());

        let private_jar = private_jar.add(cookie);
        Ok(Some(Self {
            jar: private_jar,
            config_map: cookie_config_map,
        }))
    }

    /// Convert the private cookie jar into cookie map if the private cookie jar
    /// is present in the incoming request
    ///
    /// # Arguments
    /// * `private_cookie_jar` -  An optional private cookie jar to convert into
    ///   cookie map
    ///
    /// # Return
    /// * `CookieMap` - The cookie map with data if the private cookie jar is
    ///   present, otherwise empty cookie map
    ///
    /// # Errors
    /// * `SignedPrivateCookieJarError` - When private cookie jar is present but
    ///   private cookie is not found within the jar
    pub fn into_cookie_map(
        self,
        session_cookie_config: &SessionCookieConfig,
    ) -> Result<CookieMap, SignedPrivateCookieJarError> {
        match self.jar.get(session_cookie_config.name.as_str()) {
            Some(private_cookie) => {
                let private_cookie_value = private_cookie.value().to_owned();
                let cookie_map = CookieMap::try_from(private_cookie_value)?;
                Ok(cookie_map)
            }
            None => Ok(CookieMap::default()),
        }
    }
}

impl IntoResponse for SignedPrivateCookieJar {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        self.jar.into_response()
    }
}

impl IntoResponseParts for SignedPrivateCookieJar {
    type Error = Infallible;
    fn into_response_parts(self, res: ResponseParts) -> Result<ResponseParts, Infallible> {
        self.jar.into_response_parts(res)
    }
}

impl SessionCookieConfig {
    /// Extract the cookie config from the headers
    ///
    /// # Arguments
    /// * `headers` - The headers to extract the cookie config
    ///
    /// # Return
    /// `HashMap<String, Self>` - The cookie config
    ///
    /// # Errors
    /// * `SignedPrivateCookieJarError` - When the cookie config is unable to be
    ///   extracted
    pub fn extract_cookie_config(
        headers: &HeaderMap,
    ) -> Result<HashMap<String, Self>, SignedPrivateCookieJarError> {
        let mut map = HashMap::new();
        if let Some(cookie) = headers.get(SET_COOKIE) {
            if let Ok(cookie_str) = cookie.to_str() {
                if let Ok(cookie) = Cookie::parse_encoded(cookie_str.trim()) {
                    let config = Self {
                        name: cookie.name().to_string(),
                        http_only: cookie.http_only().unwrap_or(false),
                        same_site: match cookie.same_site() {
                            Some(cookie::SameSite::Strict) => SameSite::Strict,
                            Some(cookie::SameSite::None) => SameSite::None,
                            Some(cookie::SameSite::Lax) | None => SameSite::Lax,
                        },
                        secure: cookie.secure().unwrap_or(false),
                        path: cookie.path().unwrap_or("/").to_string(),
                        domain: cookie.domain().map(std::string::ToString::to_string),
                        expiry: cookie
                            .max_age()
                            .map(|age| i32::try_from(age.whole_seconds()))
                            .transpose()
                            .map_err(|e| {
                                tracing::error!(?e, "Failed to convert max age to i32");
                                SignedPrivateCookieJarError::FromHeaders(e.to_string())
                            })?,
                    };
                    map.insert(cookie.name().to_string(), config);
                }
            }
        }
        if let Some(cookie) = headers.get(COOKIE) {
            if let Ok(cookie_str) = cookie.to_str() {
                if let Ok(cookie) = Cookie::parse_encoded(cookie_str.trim()) {
                    let config = Self {
                        name: cookie.name().to_string(),
                        http_only: cookie.http_only().unwrap_or(false),
                        same_site: match cookie.same_site() {
                            Some(cookie::SameSite::Strict) => SameSite::Strict,
                            Some(cookie::SameSite::None) => SameSite::None,
                            Some(cookie::SameSite::Lax) | None => SameSite::Lax,
                        },
                        secure: cookie.secure().unwrap_or(false),
                        path: cookie.path().unwrap_or("/").to_string(),
                        domain: cookie.domain().map(std::string::ToString::to_string),
                        expiry: cookie
                            .max_age()
                            .map(|age| i32::try_from(age.whole_seconds()))
                            .transpose()
                            .map_err(|e| {
                                tracing::error!(?e, "Failed to convert max age to i32");
                                SignedPrivateCookieJarError::FromHeaders(e.to_string())
                            })?,
                    };
                    map.insert(cookie.name().to_string(), config);
                }
            }
        }
        tracing::debug!("Extracted cookie config: {:?}", map);
        Ok(map)
    }

    pub fn apply_cookie_config(&self, cookie: &mut Cookie<'_>) {
        cookie.set_http_only(self.http_only);
        cookie.set_same_site(match self.same_site {
            SameSite::Strict => cookie::SameSite::Strict,
            SameSite::Lax => cookie::SameSite::Lax,
            SameSite::None => cookie::SameSite::None,
        });
        cookie.set_secure(self.secure);
        cookie.set_path(self.path.clone());
        if let Some(domain) = &self.domain {
            cookie.set_domain(domain.clone());
        }
        if let Some(expiry) = self.expiry {
            cookie.set_max_age(time::Duration::seconds(i64::from(expiry)));
        }
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum SignedPrivateCookieJarError {
    #[error("Cookie Map error")]
    CookieMap(#[from] CookieMapError),
    #[error("From headers error")]
    FromHeaders(String),
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use axum::response::IntoResponse;
    use axum_extra::extract::cookie::Key;
    use serde_json::Value;

    use super::*;

    const SET_COOKIE: &str = "set-cookie";

    fn get_cookies_from_response<T: IntoResponse>(response: T) -> Vec<String> {
        // Convert the response into a Response object
        let response = response.into_response();

        // Get the headers from the response
        let headers = response.headers();

        // Extract all Set-Cookie headers
        let cookies: Vec<String> = headers
            .get_all(SET_COOKIE)
            .into_iter()
            .filter_map(|value| value.to_str().ok().map(String::from))
            .collect();
        cookies
    }

    fn create_not_secure_session_config() -> SessionCookieConfig {
        SessionCookieConfig {
            name: "session".to_string(),
            http_only: false,
            same_site: SameSite::Lax,
            secure: false,
            path: "/".to_string(),
            domain: None,
            expiry: None,
        }
    }

    fn create_secure_session_config() -> SessionCookieConfig {
        SessionCookieConfig {
            name: "session".to_string(),
            http_only: true,
            same_site: SameSite::Strict,
            secure: true,
            path: "/".to_string(),
            domain: Some("localhost".to_string()),
            expiry: Some(3600),
        }
    }

    fn create_empty_header(
        private_key: &Key,
        config: &SessionCookieConfig,
    ) -> Result<HeaderMap, SignedPrivateCookieJarError> {
        let headers = HeaderMap::new();
        let jar = SignedPrivateCookieJar::new(&headers, private_key.clone())?;
        assert!(jar.into_cookie_map(config)?.is_empty());
        Ok(headers)
    }

    fn create_non_empty_header(
        private_key: &Key,
        map: HashMap<String, Value>,
        config: &SessionCookieConfig,
    ) -> Result<HeaderMap, SignedPrivateCookieJarError> {
        let cookie_map = CookieMap::new(map);
        let jar = SignedPrivateCookieJar::from_cookie_map(private_key, cookie_map, config)?;
        assert!(jar.is_some());
        let jar = jar.unwrap();
        let headers = signed_private_jar_to_headers(jar);
        Ok(headers)
    }

    fn signed_private_jar_to_headers(jar: SignedPrivateCookieJar) -> HeaderMap {
        let mut headers = jar.into_response().headers().clone();
        // Change set-cookie header to cookie header
        let value = headers.get(SET_COOKIE);
        assert!(value.is_some());
        let value = value.unwrap();
        headers.insert("cookie", value.clone());
        headers.remove(SET_COOKIE);
        headers
    }

    fn check_cookie_same_as_config(cookie: &Cookie<'_>, config: &SessionCookieConfig) -> bool {
        let same_site = match config.same_site {
            SameSite::Strict => cookie::SameSite::Strict,
            SameSite::Lax => cookie::SameSite::Lax,
            SameSite::None => cookie::SameSite::None,
        };
        // set secure to true if the config is secure
        let secure = cookie.secure() == Some(config.secure);
        let http_only = cookie.http_only() == Some(config.http_only);
        let same_site = cookie.same_site() == Some(same_site);
        let path = cookie.path() == Some(config.path.as_str());
        let domain = cookie.domain() == config.domain.as_deref();
        let max_age =
            cookie.max_age() == config.expiry.map(|e| time::Duration::seconds(i64::from(e)));
        // print out the cookie details if any check failed

        cookie.name() == config.name
            && secure
            && http_only
            && same_site
            && path
            && domain
            && max_age
    }

    #[test]
    fn test_cookie_map_process() -> Result<(), Box<dyn std::error::Error>> {
        let mut map = HashMap::new();
        map.insert(
            "foo".to_string(),
            serde_json::Value::String("bar".to_string()),
        );
        let cookie_map = CookieMap::new(map.clone());
        let map_string = String::try_from(cookie_map.clone())?;
        let new_cookie_map = CookieMap::try_from(map_string)?;
        assert_eq!(cookie_map.0, new_cookie_map.0);
        Ok(())
    }

    #[test]
    fn test_signed_private_cookie_jar_process_not_secure() -> Result<(), SignedPrivateCookieJarError>
    {
        let private_key = Key::generate();
        let mut map = HashMap::new();
        let config = create_not_secure_session_config();
        map.insert(
            "foo".to_string(),
            serde_json::Value::String("bar".to_string()),
        );
        let cookie_map = CookieMap::new(map.clone());
        let jar = SignedPrivateCookieJar::from_cookie_map(&private_key, cookie_map, &config)?;

        assert!(jar.is_some());
        let jar = jar.unwrap();
        let cookie = jar.get(config.name.as_str());
        assert!(cookie.is_some());
        let cookie_map = jar.into_cookie_map(&config)?;
        assert_eq!(cookie_map.0, map);
        assert!(check_cookie_same_as_config(&cookie.unwrap(), &config));
        Ok(())
    }

    #[test]
    fn test_signed_private_cookie_jar_process_secure() -> Result<(), SignedPrivateCookieJarError> {
        let private_key = Key::generate();
        let mut map = HashMap::new();
        let config = create_secure_session_config();
        map.insert(
            "foo".to_string(),
            serde_json::Value::String("bar".to_string()),
        );
        let cookie_map = CookieMap::new(map.clone());
        let jar = SignedPrivateCookieJar::from_cookie_map(&private_key, cookie_map, &config)?;
        assert!(jar.is_some());
        let jar = jar.unwrap();
        let cookie = jar.get(config.name.as_str());
        assert!(cookie.is_some());
        let cookie_map = jar.into_cookie_map(&config)?;
        assert_eq!(cookie_map.0, map);
        assert!(check_cookie_same_as_config(&cookie.unwrap(), &config));
        Ok(())
    }
    #[test]
    fn test_signed_private_cookie_jar_when_no_cookie() -> Result<(), SignedPrivateCookieJarError> {
        let private_key = Key::generate();
        let headers = HeaderMap::new();
        let config = create_not_secure_session_config();
        let jar = SignedPrivateCookieJar::new(&headers, private_key)?;
        let cookie = jar.get(config.name.as_str());
        assert!(cookie.is_none());
        // Create new cookie map driver when there is no private cookie jar from request
        let cookie_map = jar.into_cookie_map(&config)?;
        // expect empty hashmap
        assert_eq!(cookie_map.0, HashMap::new());
        Ok(())
    }

    // Check if empty cookie map doesn't create any private cookie jar
    #[test]
    fn test_signed_private_cookie_jar_when_empty_cookie_map_not_secure(
    ) -> Result<(), SignedPrivateCookieJarError> {
        let private_key = Key::generate();
        // Simulate empty request context
        let map = CookieMap::new(HashMap::new());
        // Try to create private cookie jar from empty cookie map
        let jar = SignedPrivateCookieJar::from_cookie_map(
            &private_key,
            map,
            &create_not_secure_session_config(),
        )?;
        // expect None
        assert!(jar.is_none());
        Ok(())
    }

    #[test]
    fn test_signed_private_cookie_jar_when_empty_cookie_map_secure(
    ) -> Result<(), SignedPrivateCookieJarError> {
        let private_key = Key::generate();
        // Simulate empty request context
        let map = CookieMap::new(HashMap::new());
        // Try to create private cookie jar from empty cookie map
        let jar = SignedPrivateCookieJar::from_cookie_map(
            &private_key,
            map,
            &create_secure_session_config(),
        )?;
        // expect None
        assert!(jar.is_none());
        Ok(())
    }

    // Check if can both sign and private cookie jars appear in the headers
    #[test]
    fn test_signed_private_cookie_jar_present_not_secure() -> Result<(), SignedPrivateCookieJarError>
    {
        let private_key = Key::generate();
        let mut map = HashMap::new();
        let config = create_not_secure_session_config();
        map.insert(
            "foo".to_string(),
            serde_json::Value::String("bar".to_string()),
        );
        let cookie_map = CookieMap::new(map.clone());
        let jar = SignedPrivateCookieJar::from_cookie_map(&private_key, cookie_map, &config)?;
        assert!(jar.is_some());
        let jar = jar.unwrap();
        let cookie = jar.get(config.name.as_str());
        assert!(cookie.is_some());
        let cookies = get_cookies_from_response(jar);
        assert_eq!(cookies.len(), 1);
        assert!(check_cookie_same_as_config(&cookie.unwrap(), &config));
        Ok(())
    }

    #[test]
    fn test_signed_private_cookie_jar_present_secure() -> Result<(), SignedPrivateCookieJarError> {
        let private_key = Key::generate();
        let config = create_secure_session_config();
        let mut map = HashMap::new();
        map.insert(
            "foo".to_string(),
            serde_json::Value::String("bar".to_string()),
        );
        let cookie_map = CookieMap::new(map.clone());
        let jar = SignedPrivateCookieJar::from_cookie_map(&private_key, cookie_map, &config)?;
        assert!(jar.is_some());
        let jar = jar.unwrap();
        let cookie = jar.get(config.name.as_str());
        assert!(cookie.is_some());
        assert!(check_cookie_same_as_config(&cookie.unwrap(), &config));
        Ok(())
    }

    // Scenario 1: Test if empty cookie map can be modified
    // Empty request -> Empty SignedPrivateCookieJar -> Empty CookieMap -> Modified
    // CookieMap ({ key: "value" }) -> Non-empty SignedPrivateCookieJar -> Response
    // -> Non-empty SignedPrivateCookieJar -> Non-empty CookieMap -> Assert
    #[test]
    fn test_empty_cookie_map_be_modified_not_secure() -> Result<(), SignedPrivateCookieJarError> {
        let private_key = Key::generate();
        let config = create_not_secure_session_config();
        let headers = create_empty_header(&private_key, &config)?;
        let jar = SignedPrivateCookieJar::new(&headers, private_key.clone())?;
        // Turn into empty cookie map
        let mut cookie_map = jar.into_cookie_map(&config)?;
        assert!(cookie_map.is_empty());

        // Add stuff to cookie map
        cookie_map
            .0
            .insert("key".to_string(), serde_json::json!("value"));

        // Turn back into SignedPrivateCookieJar
        let new_jar =
            SignedPrivateCookieJar::from_cookie_map(&private_key, cookie_map.clone(), &config)?;
        assert!(new_jar.is_some());
        let new_jar = new_jar.unwrap();
        let cookie = new_jar.get(config.name.as_str());
        assert!(cookie.is_some());
        assert!(check_cookie_same_as_config(&cookie.unwrap(), &config));

        // Turn into headers
        let headers = signed_private_jar_to_headers(new_jar);
        // create new jar from headers
        let new_jar = SignedPrivateCookieJar::new(&headers, private_key)?;
        // Turn into cookie map
        let new_cookie_map = new_jar.into_cookie_map(&config)?;
        assert_ne!(new_cookie_map.0, HashMap::new());

        // Add the key to the cookie map
        cookie_map
            .0
            .insert("key".to_string(), serde_json::json!("value"));
        assert_eq!(new_cookie_map.0, cookie_map.0);

        Ok(())
    }

    #[test]
    fn test_empty_cookie_map_be_modified_secure() -> Result<(), SignedPrivateCookieJarError> {
        let private_key = Key::generate();
        let config = create_secure_session_config();
        let headers = create_empty_header(&private_key, &config)?;
        let jar = SignedPrivateCookieJar::new(&headers, private_key.clone())?;

        // Turn into empty cookie map
        let mut cookie_map = jar.into_cookie_map(&config)?;
        assert!(cookie_map.is_empty());

        // Add stuff to cookie map
        cookie_map
            .0
            .insert("key".to_string(), serde_json::json!("value"));

        // Turn back into SignedPrivateCookieJar
        let new_jar =
            SignedPrivateCookieJar::from_cookie_map(&private_key, cookie_map.clone(), &config)?;
        assert!(new_jar.is_some());
        let new_jar = new_jar.unwrap();
        let cookie = new_jar.get(config.name.as_str());
        assert!(cookie.is_some());
        assert!(check_cookie_same_as_config(&cookie.unwrap(), &config));

        // Turn into headers
        let headers = signed_private_jar_to_headers(new_jar);
        // create new jar from headers
        let new_jar = SignedPrivateCookieJar::new(&headers, private_key)?;
        // Turn into cookie map
        let new_cookie_map = new_jar.into_cookie_map(&config)?;
        assert_ne!(new_cookie_map.0, HashMap::new());

        // Add the key to the cookie map
        cookie_map
            .0
            .insert("key".to_string(), serde_json::json!("value"));
        assert_eq!(new_cookie_map.0, cookie_map.0);
        Ok(())
    }

    // Scenario 2: Test if non-empty cookie map can be modified
    // Non-empty request ({ foo: "bar" }) -> Non-empty SignedPrivateCookieJar ->
    // Non-empty CookieMap -> Modified CookieMap ({ foo: "bar", "new_key":
    // "new_value" }) -> Non-empty SignedPrivateCookieJar -> Response -> Non-empty
    // SignedPrivateCookieJar -> Non-empty CookieMap -> Assert
    #[test]
    fn test_non_empty_cookie_map_be_modified_not_secure() -> Result<(), SignedPrivateCookieJarError>
    {
        let private_key = Key::generate();
        let config = create_not_secure_session_config();
        let mut map = HashMap::new();
        map.insert(
            "foo".to_string(),
            serde_json::Value::String("bar".to_string()),
        );
        let non_empty_header = create_non_empty_header(&private_key, map.clone(), &config)?;
        let jar = SignedPrivateCookieJar::new(&non_empty_header, private_key.clone())?;
        let cookie = jar.get(config.name.as_str());
        assert!(cookie.is_some());
        assert!(check_cookie_same_as_config(&cookie.unwrap(), &config));

        // Turn into non-empty cookie map
        let mut cookie_map = jar.into_cookie_map(&config)?;
        assert!(!cookie_map.is_empty());

        // Modify cookie map
        cookie_map
            .0
            .insert("new_key".to_string(), serde_json::json!("new_value"));

        // Turn back into SignedPrivateCookieJar
        let new_jar =
            SignedPrivateCookieJar::from_cookie_map(&private_key, cookie_map.clone(), &config)?;
        assert!(new_jar.is_some());
        let new_jar = new_jar.unwrap();
        let cookie = new_jar.get(config.name.as_str());
        assert!(cookie.is_some());
        assert!(check_cookie_same_as_config(&cookie.unwrap(), &config));

        // Turn into headers
        let headers = signed_private_jar_to_headers(new_jar);
        // create new jar from headers
        let new_jar = SignedPrivateCookieJar::new(&headers, private_key)?;
        // Turn into cookie map
        let new_cookie_map = new_jar.into_cookie_map(&config)?;
        assert_ne!(new_cookie_map.0, map);
        map.insert("new_key".to_string(), serde_json::json!("new_value"));
        assert_eq!(new_cookie_map.0, map);
        Ok(())
    }

    #[test]
    fn test_non_empty_cookie_map_be_modified_secure() -> Result<(), SignedPrivateCookieJarError> {
        let private_key = Key::generate();
        let config = create_secure_session_config();
        let mut map = HashMap::new();
        map.insert(
            "foo".to_string(),
            serde_json::Value::String("bar".to_string()),
        );
        let non_empty_header = create_non_empty_header(&private_key, map.clone(), &config)?;
        let jar = SignedPrivateCookieJar::new(&non_empty_header, private_key.clone())?;
        let cookie = jar.get(config.name.as_str());
        assert!(cookie.is_some());
        assert!(check_cookie_same_as_config(&cookie.unwrap(), &config));

        // Turn into non-empty cookie map
        let mut cookie_map = jar.into_cookie_map(&config)?;
        assert!(!cookie_map.is_empty());

        // Modify cookie map
        cookie_map
            .0
            .insert("new_key".to_string(), serde_json::json!("new_value"));

        // Turn back into SignedPrivateCookieJar
        let new_jar =
            SignedPrivateCookieJar::from_cookie_map(&private_key, cookie_map.clone(), &config)?;
        assert!(new_jar.is_some());
        let new_jar = new_jar.unwrap();
        let cookie = new_jar.get(config.name.as_str());
        assert!(cookie.is_some());
        assert!(check_cookie_same_as_config(&cookie.unwrap(), &config));

        // Turn into headers
        let headers = signed_private_jar_to_headers(new_jar);
        // create new jar from headers
        let new_jar = SignedPrivateCookieJar::new(&headers, private_key)?;
        // Turn into cookie map
        let new_cookie_map = new_jar.into_cookie_map(&config)?;
        assert_ne!(new_cookie_map.0, map);
        map.insert("new_key".to_string(), serde_json::json!("new_value"));
        assert_eq!(new_cookie_map.0, map);
        Ok(())
    }

    // Scenario 3: Test if empty cookie map can be unchanged
    // Empty request -> Empty SignedPrivateCookieJar -> Empty CookieMap -> Unchanged
    // CookieMap -> Empty SignedPrivateCookieJar -> Response -> Empty
    // SignedPrivateCookieJar -> Empty CookieMap -> Assert
    #[test]
    fn test_empty_cookie_map_unchanged_not_secure() -> Result<(), SignedPrivateCookieJarError> {
        let private_key = Key::generate();
        let config = create_not_secure_session_config();
        let headers = create_empty_header(&private_key, &config)?;
        let jar = SignedPrivateCookieJar::new(&headers, private_key.clone())?;

        // Turn into empty cookie map
        let cookie_map = jar.into_cookie_map(&config)?;
        assert!(cookie_map.is_empty());

        // Turn back into SignedPrivateCookieJar without changes
        let new_jar = SignedPrivateCookieJar::from_cookie_map(&private_key, cookie_map, &config)?;
        assert!(new_jar.is_none());
        Ok(())
    }

    #[test]
    fn test_empty_cookie_map_unchanged_secure() -> Result<(), SignedPrivateCookieJarError> {
        let private_key = Key::generate();
        let config = create_secure_session_config();
        let headers = create_empty_header(&private_key, &config)?;
        let jar = SignedPrivateCookieJar::new(&headers, private_key.clone())?;

        // Turn into empty cookie map
        let cookie_map = jar.into_cookie_map(&config)?;
        assert!(cookie_map.is_empty());

        // Turn back into SignedPrivateCookieJar without changes
        let new_jar = SignedPrivateCookieJar::from_cookie_map(&private_key, cookie_map, &config)?;
        assert!(new_jar.is_none());
        Ok(())
    }

    // Scenario 4: Test if non-empty cookie map can be unchanged
    // Non-empty request ({ foo: "bar" }) -> Non-empty SignedPrivateCookieJar ->
    // Non-empty CookieMap -> Unchanged CookieMap -> Non-empty
    // SignedPrivateCookieJar -> Response -> Non-empty SignedPrivateCookieJar ->
    // Non-empty CookieMap -> Assert
    #[test]
    fn test_scenario_4_non_empty_unchanged_not_secure() -> Result<(), SignedPrivateCookieJarError> {
        let private_key = Key::generate();
        let config = create_not_secure_session_config();
        let mut map = HashMap::new();
        map.insert(
            "foo".to_string(),
            serde_json::Value::String("bar".to_string()),
        );
        let non_empty_header = create_non_empty_header(&private_key, map.clone(), &config)?;
        let jar = SignedPrivateCookieJar::new(&non_empty_header, private_key.clone())?;
        let cookie = jar.get(config.name.as_str());
        assert!(cookie.is_some());
        assert!(check_cookie_same_as_config(&cookie.unwrap(), &config));

        // Turn into non-empty cookie map
        let cookie_map = jar.into_cookie_map(&config)?;
        assert!(!cookie_map.is_empty());

        // Turn back into SignedPrivateCookieJar without changes
        let new_jar = SignedPrivateCookieJar::from_cookie_map(&private_key, cookie_map, &config)?;
        assert!(new_jar.is_some());
        let new_jar = new_jar.unwrap();
        let cookie = new_jar.get(config.name.as_str());
        assert!(cookie.is_some());
        assert!(check_cookie_same_as_config(&cookie.unwrap(), &config));
        let headers = signed_private_jar_to_headers(new_jar);
        let new_jar = SignedPrivateCookieJar::new(&headers, private_key)?;
        let new_cookie_map = new_jar.into_cookie_map(&config)?;
        assert_eq!(new_cookie_map.0, map);

        Ok(())
    }

    #[test]
    fn test_scenario_4_non_empty_unchanged_secure() -> Result<(), SignedPrivateCookieJarError> {
        let private_key = Key::generate();
        let config = create_secure_session_config();
        let mut map = HashMap::new();
        map.insert(
            "foo".to_string(),
            serde_json::Value::String("bar".to_string()),
        );
        let non_empty_header =
            create_non_empty_header(&private_key, map.clone(), &create_secure_session_config())?;
        let jar = SignedPrivateCookieJar::new(&non_empty_header, private_key.clone())?;

        // Turn into non-empty cookie map
        let cookie_map = jar.into_cookie_map(&config)?;
        assert!(!cookie_map.is_empty());

        // Turn back into SignedPrivateCookieJar without changes
        let new_jar = SignedPrivateCookieJar::from_cookie_map(
            &private_key,
            cookie_map,
            &create_secure_session_config(),
        )?;
        assert!(new_jar.is_some());
        let new_jar = new_jar.unwrap();
        let headers = signed_private_jar_to_headers(new_jar);
        let new_jar = SignedPrivateCookieJar::new(&headers, private_key)?;
        let new_cookie_map = new_jar.into_cookie_map(&config)?;
        assert_eq!(new_cookie_map.0, map);

        Ok(())
    }
}
