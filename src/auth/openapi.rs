use std::sync::OnceLock;

use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify,
};

use crate::{app::AppContext, config::JWTLocation};

static JWT_LOCATION: OnceLock<JWTLocation> = OnceLock::new();

pub fn get_jwt_location_from_ctx(ctx: &AppContext) -> JWTLocation {
    ctx.config
        .auth
        .as_ref()
        .and_then(|auth| auth.jwt.as_ref())
        .and_then(|jwt| jwt.location.as_ref())
        .unwrap_or(&JWTLocation::Bearer)
        .clone()
}

pub fn set_jwt_location_ctx(ctx: &AppContext) -> &'static JWTLocation {
    set_jwt_location(get_jwt_location_from_ctx(ctx))
}

pub fn set_jwt_location(jwt_location: JWTLocation) -> &'static JWTLocation {
    JWT_LOCATION.get_or_init(|| jwt_location)
}

fn get_jwt_location() -> &'static JWTLocation {
    JWT_LOCATION.get().unwrap_or(&JWTLocation::Bearer)
}

pub struct SecurityAddon;

/// Adds security to the OpenAPI doc, using the JWT location in the config
impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_schemes_from_iter([
                (
                    "jwt_token",
                    match get_jwt_location() {
                        JWTLocation::Bearer => SecurityScheme::Http(
                            HttpBuilder::new()
                                .scheme(HttpAuthScheme::Bearer)
                                .bearer_format("JWT")
                                .build(),
                        ),
                        JWTLocation::Query { name } => {
                            SecurityScheme::ApiKey(ApiKey::Query(ApiKeyValue::new(name)))
                        }
                        JWTLocation::Cookie { name } => {
                            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(name)))
                        }
                    },
                ),
                (
                    "api_key",
                    SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("apikey"))),
                ),
            ]);
        }
    }
}
