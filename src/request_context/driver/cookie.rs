use crate::request_context::driver::PRIVATE_COOKIE_NAME;
use axum::http::HeaderMap;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::cookie::{Key, PrivateCookieJar, SignedCookieJar};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SignedPrivateCookieJar {
    private_jar: PrivateCookieJar,
    signed_jar: SignedCookieJar,
}

impl SignedPrivateCookieJar {
    #[must_use]
    pub fn new(private_key: &Key, signed_key: &Key) -> Self {
        Self {
            private_jar: PrivateCookieJar::new(private_key.clone()),
            signed_jar: SignedCookieJar::new(signed_key.clone()),
        }
    }

    #[must_use]
    pub fn from_headers(
        private_key: &Key,
        signed_key: &Key,
        headers: &HeaderMap,
    ) -> Result<Self, SignedPrivateCookieJarError> {
        // Create a new instance of the SignedPrivateCookieJar
        let signed_jar = SignedCookieJar::from_headers(headers, signed_key.clone());
        // Create a new instance of the PrivateCookieJar
        let private_jar = PrivateCookieJar::from_headers(headers, private_key.clone());
        let private_map = private_jar.get(PRIVATE_COOKIE_NAME);
        let signed_map = signed_jar.get(PRIVATE_COOKIE_NAME);
        match (private_map, signed_map) {
            (Some(_), None) => Err(SignedPrivateCookieJarError::FromHeaders(
                "Private cookie is present but signed cookie is missing".to_string(),
            )),
            (None, Some(_)) => Err(SignedPrivateCookieJarError::FromHeaders(
                "Signed cookie is present but private cookie is missing".to_string(),
            )),
            (None, None) => Ok(Self::new(private_key, signed_key)),
            (Some(private_cookie), Some(signed_cookie)) => {
                if private_cookie.value() == signed_cookie.value() {
                    Ok(Self {
                        private_jar,
                        signed_jar,
                    })
                } else {
                    Err(SignedPrivateCookieJarError::FromHeaders(
                        "Private cookie and signed cookie do not match".to_string(),
                    ))
                }
            }
        }
    }

    #[must_use]
    pub fn add(
        &mut self,
        name: &str,
        value: impl Serialize + Send,
    ) -> Result<(), SignedPrivateCookieJarError> {
        // Firstly, get the Hashmap from the private_jar
        let mut map: HashMap<String, serde_json::Value> =
            if let Some(cookie) = self.private_jar.get(PRIVATE_COOKIE_NAME) {
                let cookie_value = cookie.value().to_owned();
                serde_json::from_str(&cookie_value)?
            } else {
                HashMap::new()
            };
        // Insert the value into the Hashmap
        map.insert(name.to_owned(), serde_json::to_value(value)?);
        // Serialize the updated map back to a string
        let updated_cookie_value = serde_json::to_string(&map)?;
        // Create a new cookie with the updated value
        let new_cookie = Cookie::new(PRIVATE_COOKIE_NAME, updated_cookie_value);
        // Add the new cookie to the jar
        self.private_jar = self.private_jar.clone().add(new_cookie.clone());
        // Then, sign the encrypted data
        if let Some(encrypted_cookie) = self.private_jar.get(PRIVATE_COOKIE_NAME) {
            self.signed_jar = self.signed_jar.clone().add(encrypted_cookie.clone());
        }

        Ok(())
    }

    #[must_use]
    pub fn get<T: for<'de> Deserialize<'de>>(
        &self,
        name: &str,
    ) -> Result<Option<T>, SignedPrivateCookieJarError> {
        // Firstly, get the Hashmap from the private_jar
        if let Some(cookie) = self.private_jar.get(PRIVATE_COOKIE_NAME) {
            let cookie_value = cookie.value().to_owned();
            let map: HashMap<String, serde_json::Value> = serde_json::from_str(&cookie_value)?;
            // Deserialize the value from the Hashmap
            return Ok(map
                .get(name)
                .and_then(|value| serde_json::from_value(value.clone()).ok()));
        }
        Ok(None)
    }

    #[must_use]
    pub fn remove(&mut self, name: &str) -> Result<(), SignedPrivateCookieJarError> {
        // Firstly, get the Hashmap from the private_jar
        let mut map: HashMap<String, serde_json::Value> =
            if let Some(cookie) = self.private_jar.get(PRIVATE_COOKIE_NAME) {
                let cookie_value = cookie.value().to_owned();
                serde_json::from_str(&cookie_value)?
            } else {
                HashMap::new()
            };

        // Remove the value from the Hashmap
        map.remove(name);
        // Serialize the updated map back to a string
        let updated_cookie_value = serde_json::to_string(&map).unwrap();
        // Create a new cookie with the updated value
        let new_cookie = Cookie::new(PRIVATE_COOKIE_NAME, updated_cookie_value);
        // Add the new cookie to the jar
        self.private_jar = self.private_jar.clone().add(new_cookie.clone());
        // Then, sign the encrypted data
        if let Some(encrypted_cookie) = self.private_jar.get(PRIVATE_COOKIE_NAME) {
            self.signed_jar = self.signed_jar.clone().add(encrypted_cookie.clone());
        }
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum SignedPrivateCookieJarError {
    #[error("Unable to extract data from cookie")]
    ExtractData(#[from] serde_json::Error),
    #[error("From headers error")]
    FromHeaders(String),
}

#[cfg(test)]
mod test {
    use super::*;
    use axum_extra::extract::cookie::Cookie;
    use axum_extra::extract::cookie::Key;
    use std::collections::HashMap;

    #[test]
    fn test_signed_private_cookie_jar() -> Result<(), SignedPrivateCookieJarError> {
        let (private_key, signed_key) = (Key::generate(), Key::generate());
        let mut jar = SignedPrivateCookieJar::new(&private_key, &signed_key);
        let (name, value) = ("foo", "bar".to_string());
        jar.add(name, value.clone())?;
        assert_eq!(jar.get::<String>(name)?, Some(value.clone()));
        jar.remove(name)?;
        assert_eq!(jar.get::<String>(name)?, None);
        Ok(())
    }
}
