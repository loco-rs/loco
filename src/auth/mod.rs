#[cfg(feature = "auth_jwt")]
pub mod jwt;

#[cfg(all(
    any(feature = "jwt_aws_lc_rs", feature = "jwt_rustcrypto"),
    not(feature = "auth_jwt")
))]
compile_error!("jwt_aws_lc_rs and jwt_rustcrypto features require auth_jwt feature");

