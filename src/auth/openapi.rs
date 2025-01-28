use std::sync::OnceLock;

use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify,
};

use crate::{app::AppContext, config::JWTLocation};

static JWT_LOCATION: OnceLock<Option<JWTLocation>> = OnceLock::new();

#[must_use]
pub fn get_jwt_location_from_ctx(ctx: &AppContext) -> JWTLocation {
    ctx.config
        .auth
        .as_ref()
        .and_then(|auth| auth.jwt.as_ref())
        .and_then(|jwt| jwt.location.as_ref())
        .unwrap_or(&JWTLocation::Bearer)
        .clone()
}

pub fn set_jwt_location_ctx(ctx: &AppContext) {
    set_jwt_location(get_jwt_location_from_ctx(ctx));
}

pub fn set_jwt_location(jwt_location: JWTLocation) -> &'static Option<JWTLocation> {
    JWT_LOCATION.get_or_init(|| Some(jwt_location))
}

fn get_jwt_location() -> Option<&'static JWTLocation> {
    JWT_LOCATION.get().unwrap_or(&None).as_ref()
}

pub struct SecurityAddon;

/// Adds security to the `OpenAPI` doc, using the JWT location in the config
impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(jwt_location) = get_jwt_location() {
            if let Some(components) = openapi.components.as_mut() {
                components.add_security_schemes_from_iter([
                    (
                        "jwt_token",
                        match jwt_location {
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
}
