use std::sync::OnceLock;
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify,
};

use crate::{app::AppContext, config::JWTLocation};

static JWT_LOCATION: OnceLock<JWTLocation> = OnceLock::new();

pub fn set_jwt_location(ctx: &AppContext) -> &'static JWTLocation {
    JWT_LOCATION.get_or_init(|| {
        ctx.config
            .auth
            .as_ref()
            .and_then(|auth| auth.jwt.as_ref())
            .and_then(|jwt| jwt.location.as_ref())
            .unwrap_or(&JWTLocation::Bearer)
            .clone()
    })
}

fn get_jwt_location() -> &'static JWTLocation {
    JWT_LOCATION.get().unwrap()
}

pub struct SecurityAddon;

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
