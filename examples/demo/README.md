# Demo

This app is a kitchensink for various capabilities and examples of the [Loco](https://loco.rs) project.

# Example Listings

## OpenAPI with `utoipa`

### Implementing OpenAPI path see
- [src/controllers/auth.rs](src/controllers/auth.rs)
- `Album` in [src/controllers/responses.rs](src/controllers/responses.rs)

### How to serve the OpenAPI doc see
- `after_routes` in [src/app.rs](src/app.rs)
- `api_routes` in [src/controllers/auth.rs](src/controllers/auth.rs)

### View the served OpenAPI doc at
- [http://localhost:5150/swagger-ui/](http://localhost:5150/swagger-ui/)
- [http://localhost:5150/redoc](http://localhost:5150/redoc)
- [http://localhost:5150/scalar](http://localhost:5150/scalar)
