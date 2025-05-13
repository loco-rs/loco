use crate::{Error, Result};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
};
use rand::{distr::Alphanumeric, rng, Rng};

/// Hashes a plain text password and returns the hashed result.
///
/// # Errors
///
/// Return [`argon2::password_hash::Result`] when could not hash the given
/// password.
///
/// # Example
/// ```rust
/// use loco_rs::hash;
///
/// hash::hash_password("password-to-hash");
/// ```
pub fn hash_password(pass: &str) -> Result<String> {
    let arg2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::default(),
    );
    let salt = SaltString::generate(&mut OsRng);

    Ok(arg2
        .hash_password(pass.as_bytes(), &salt)
        .map_err(|err| Error::Hash(err.to_string()))?
        .to_string())
}

/// Verifies a plain text password against a hashed password.
///
/// # Errors
///
/// Return [`argon2::password_hash::Result`] when could verify the given data.
///
/// # Example
/// ```rust
/// use loco_rs::hash;
///
/// hash::verify_password("password", "hashed-password");
/// ```
#[must_use]
pub fn verify_password(pass: &str, hashed_password: &str) -> bool {
    let arg2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        Version::V0x13,
        Params::default(),
    );
    let Ok(hash) = PasswordHash::new(hashed_password) else {
        return false;
    };
    arg2.verify_password(pass.as_bytes(), &hash).is_ok()
}

/// Generates a random alphanumeric string of the specified length.
///
/// # Example
///
/// ```rust
/// use loco_rs::hash;
///
/// let rand_str = hash::random_string(10);
/// assert_eq!(rand_str.len(), 10);
/// assert_ne!(rand_str, hash::random_string(10));
///
/// ```
pub fn random_string(length: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn can_hah_password() {
        let pass = "password-1234";

        let hash_pass = hash_password(pass).unwrap();

        assert!(verify_password(pass, &hash_pass));
    }

    #[test]
    fn can_random_string() {
        let random_length = 32;
        let first = random_string(random_length);
        assert_eq!(first.len(), random_length);
        let second: String = random_string(random_length);
        assert_eq!(second.len(), random_length);
        assert_ne!(first, second);
    }
}
