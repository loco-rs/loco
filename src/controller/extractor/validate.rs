use crate::Error;
use axum::extract::{
    Form, FromRequest, FromRequestParts, Json, Path, Query, Request,
};
use serde::de::DeserializeOwned;
use validator::Validate;

/// Axum middleware for validating JSON request bodies
///
/// This module provides extractors for validating JSON request bodies, form data,
/// path parameters, and query parameters using the `validator` crate. Each extractor
/// supports both detailed validation error messages (`WithMessage` variants) and
/// simplified error responses.
///
/// # Example:
///
/// ```
/// use axum::{routing::post, Router};
/// use serde::{Deserialize, Serialize};
/// use validator::Validate;
///
/// #[derive(Serialize, Deserialize, Validate)]
/// struct User {
///     #[validate(length(min = 3, message = "username must be at least 3 characters"))]
///     username: String,
///     #[validate(email(message = "email must be valid"))]
///     email: String,
/// }
///
/// async fn create_user(validate::JsonValidateWithMessage(user): validate::JsonValidateWithMessage<User>) -> String {
///     format!("User created: {}, {}", user.username, user.email)
/// }
///
/// fn app() -> Router {
///     Router::new()
///         .route("/users", post(create_user))
/// }
/// ```

#[derive(Debug, Clone, Copy, Default)]
pub struct JsonValidateWithMessage<T>(pub T);

impl<T, S> FromRequest<S> for JsonValidateWithMessage<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(Self(value))
    }
}

/// Axum middleware for validating form data
///
/// # Example:
///
/// ```
/// use axum::{routing::post, Router};
/// use serde::{Deserialize, Serialize};
/// use validator::Validate;
///
/// #[derive(Serialize, Deserialize, Validate)]
/// struct User {
///     #[validate(length(min = 3, message = "username must be at least 3 characters"))]
///     username: String,
///     #[validate(email(message = "email must be valid"))]
///     email: String,
/// }
///
/// async fn create_user(validate::FormValidateWithMessage(user): validate::FormValidateWithMessage<User>) -> String {
///     format!("User created: {}, {}", user.username, user.email)
/// }
///
/// fn app() -> Router {
///     Router::new()
///         .route("/users", post(create_user))
/// }
/// ```

#[derive(Debug, Clone, Copy, Default)]
pub struct FormValidateWithMessage<T>(pub T);

impl<T, S> FromRequest<S> for FormValidateWithMessage<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Form(value) = Form::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(Self(value))
    }
}

/// Axum middleware for validating JSON request bodies with simplified error handling
///
/// # Example:
///
/// ```
/// use axum::{routing::post, Router};
/// use serde::{Deserialize, Serialize};
/// use validator::Validate;
///
/// #[derive(Serialize, Deserialize, Validate)]
/// struct User {
///     #[validate(length(min = 3, message = "username must be at least 3 characters"))]
///     username: String,
///     #[validate(email(message = "email must be valid"))]
///     email: String,
/// }
///
/// async fn create_user(validate::JsonValidate(user): validate::JsonValidate<User>) -> String {
///     format!("User created: {}, {}", user.username, user.email)
/// }
///
/// fn app() -> Router {
///     Router::new()
///         .route("/users", post(create_user))
/// }
/// ```

#[derive(Debug, Clone, Copy, Default)]
pub struct JsonValidate<T>(pub T);

impl<T, S> FromRequest<S> for JsonValidate<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await?;
        value.validate().map_err(|err| {
            tracing::debug!(err = ?err, "request validation error occurred");
            Error::BadRequest(String::new())
        })?;
        Ok(Self(value))
    }
}

/// Axum middleware for validating form data with simplified error handling
///
/// # Example:
///
/// ```
/// use axum::{routing::post, Router};
/// use serde::{Deserialize, Serialize};
/// use validator::Validate;
///
/// #[derive(Serialize, Deserialize, Validate)]
/// struct User {
///     #[validate(length(min = 3, message = "username must be at least 3 characters"))]
///     username: String,
///     #[validate(email(message = "email must be valid"))]
///     email: String,
/// }
///
/// async fn create_user(validate::FormValidate(user): validate::FormValidate<User>) -> String {
///     format!("User created: {}, {}", user.username, user.email)
/// }
///
/// fn app() -> Router {
///     Router::new()
///         .route("/users", post(create_user))
/// }
/// ```

#[derive(Debug, Clone, Copy, Default)]
pub struct FormValidate<T>(pub T);

impl<T, S> FromRequest<S> for FormValidate<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Form(value) = Form::<T>::from_request(req, state).await?;
        value.validate().map_err(|err| {
            tracing::debug!(err = ?err, "request validation error occurred");
            Error::BadRequest(String::new())
        })?;
        Ok(Self(value))
    }
}

/// Axum middleware for validating path parameters
///
/// # Example:
///
/// ```
/// use axum::{routing::get, Router};
/// use serde::{Deserialize, Serialize};
/// use validator::Validate;
///
/// #[derive(Serialize, Deserialize, Validate)]
/// struct UserParams {
///     #[validate(range(min = 1, message = "id must be at least 1"))]
///     id: i32,
/// }
///
/// async fn get_user(validate::PathValidateWithMessage(params): validate::PathValidateWithMessage<UserParams>) -> String {
///     format!("User ID: {}", params.id)
/// }
///
/// fn app() -> Router {
///     Router::new()
///         .route("/users/:id", get(get_user))
/// }
/// ```

#[derive(Debug, Clone, Copy, Default)]
pub struct PathValidateWithMessage<T>(pub T);

impl<T, S> FromRequestParts<S> for PathValidateWithMessage<T>
where
    T: DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Path(value) =
            <Path<T> as axum::extract::FromRequestParts<S>>::from_request_parts(parts, state)
                .await
                .map_err(|_err| Error::BadRequest(String::new()))?;
        value.validate()?;
        Ok(Self(value))
    }
}

/// Axum middleware for validating path parameters with simplified error handling
///
/// # Example:
///
/// ```
/// use axum::{routing::get, Router};
/// use serde::{Deserialize, Serialize};
/// use validator::Validate;
///
/// #[derive(Serialize, Deserialize, Validate)]
/// struct UserParams {
///     #[validate(range(min = 1, message = "id must be at least 1"))]
///     id: i32,
/// }
///
/// async fn get_user(validate::PathValidate(params): validate::PathValidate<UserParams>) -> String {
///     format!("User ID: {}", params.id)
/// }
///
/// fn app() -> Router {
///     Router::new()
///         .route("/users/:id", get(get_user))
/// }
/// ```

#[derive(Debug, Clone, Copy, Default)]
pub struct PathValidate<T>(pub T);

impl<T, S> FromRequestParts<S> for PathValidate<T>
where
    T: DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Path(value) = <Path<T> as axum::extract::FromRequestParts<S>>::from_request_parts(parts, state)
            .await
            .map_err(|_err| Error::BadRequest(String::new()))?;
        value.validate().map_err(|err| {
            tracing::debug!(err = ?err, "path validation error occurred");
            Error::BadRequest(String::new())
        })?;
        Ok(Self(value))
    }
}

/// Axum middleware for validating query parameters
///
/// # Example:
///
/// ```
/// use axum::{routing::get, Router};
/// use serde::{Deserialize, Serialize};
/// use validator::Validate;
///
/// #[derive(Serialize, Deserialize, Validate)]
/// struct UserQuery {
///     #[validate(length(min = 3, message = "username must be at least 3 characters"))]
///     username: String,
///     #[validate(email(message = "email must be valid"))]
///     email: String,
/// }
///
/// async fn get_user(validate::QueryValidateWithMessage(params): validate::QueryValidateWithMessage<UserQuery>) -> String {
///     format!("User: {}, Email: {}", params.username, params.email)
/// }
///
/// fn app() -> Router {
///     Router::new()
///         .route("/users", get(get_user))
/// }
/// ```

#[derive(Debug, Clone, Copy, Default)]
pub struct QueryValidateWithMessage<T>(pub T);

impl<T, S> FromRequest<S> for QueryValidateWithMessage<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request(req, state)
            .await
            .map_err(|_err| Error::BadRequest(String::new()))?;
        value.validate()?;
        Ok(Self(value))
    }
}

/// Axum middleware for validating query parameters with simplified error handling
///
/// # Example:
///
/// ```
/// use axum::{routing::get, Router};
/// use serde::{Deserialize, Serialize};
/// use validator::Validate;
///
/// #[derive(Serialize, Deserialize, Validate)]
/// struct UserQuery {
///     #[validate(length(min = 3, message = "username must be at least 3 characters"))]
///     username: String,
///     #[validate(email(message = "email must be valid"))]
///     email: String,
/// }
///
/// async fn get_user(validate::QueryValidate(params): validate::QueryValidate<UserQuery>) -> String {
///     format!("User: {}, Email: {}", params.username, params.email)
/// }
///
/// fn app() -> Router {
///     Router::new()
///         .route("/users", get(get_user))
/// }
/// ```

#[derive(Debug, Clone, Copy, Default)]
pub struct QueryValidate<T>(pub T);

impl<T, S> FromRequest<S> for QueryValidate<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request(req, state)
            .await
            .map_err(|_err| Error::BadRequest(String::new()))?;
        value.validate().map_err(|err| {
            tracing::debug!(err = ?err, "query validation error occurred");
            Error::BadRequest(String::new())
        })?;
        Ok(Self(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{self, Request as HttpRequest, StatusCode},
        response::IntoResponse,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};
    use validator::Validate;

    #[derive(Debug, Serialize, Deserialize, Validate)]
    struct TestUser {
        #[validate(length(min = 3, message = "username must be at least 3 characters"))]
        username: String,
        #[validate(email(message = "email must be valid"))]
        email: String,
    }

    #[derive(Debug, Serialize, Deserialize, Validate)]
    struct TestPathParams {
        #[validate(range(min = 1, message = "id must be at least 1"))]
        id: i32,
    }

    #[derive(Debug, Serialize, Deserialize, Validate)]
    struct TestQueryParams {
        #[validate(length(min = 3, message = "username must be at least 3 characters"))]
        username: String,
        #[validate(email(message = "email must be valid"))]
        email: String,
    }

    fn create_json_request(json: &str) -> HttpRequest<Body> {
        HttpRequest::builder()
            .method(http::Method::POST)
            .uri("/test")
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(json.to_string()))
            .unwrap()
    }

    fn create_form_request(form_data: &str) -> HttpRequest<Body> {
        HttpRequest::builder()
            .method(http::Method::POST)
            .uri("/test")
            .header(
                http::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(Body::from(form_data.to_string()))
            .unwrap()
    }

    fn create_path_request(path: &str) -> HttpRequest<Body> {
        HttpRequest::builder()
            .method(http::Method::GET)
            .uri(path)
            .body(Body::empty())
            .unwrap()
    }

    fn create_query_request(query: &str) -> HttpRequest<Body> {
        HttpRequest::builder()
            .method(http::Method::GET)
            .uri(format!("/test?{}", query))
            .body(Body::empty())
            .unwrap()
    }

    async fn assert_response_status_and_body(
        err: Error,
        expected_status: StatusCode,
        expected_json: Value,
    ) {
        let response = err.into_response();
        assert_eq!(response.status(), expected_status);

        let body = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("Failed to read response body");

        let body_str = String::from_utf8(body.to_vec()).expect("Response body is not valid UTF-8");

        let actual_json =
            serde_json::from_str::<Value>(&body_str).expect("Response body is not valid JSON");

        assert_eq!(actual_json, expected_json);
    }

    #[tokio::test]
    async fn test_json_validate_with_message_valid() {
        let valid_json = r#"{"username": "valid_user", "email": "test@example.com"}"#;
        let request = create_json_request(valid_json);

        let result = JsonValidateWithMessage::<TestUser>::from_request(request, &()).await;
        assert!(result.is_ok());

        let user = result.unwrap().0;
        assert_eq!(user.username, "valid_user");
        assert_eq!(user.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_json_validate_with_message_invalid() {
        let invalid_json = r#"{"username": "ab", "email": "invalid-email"}"#;
        let request = create_json_request(invalid_json);

        let result = JsonValidateWithMessage::<TestUser>::from_request(request, &()).await;
        assert!(result.is_err());

        let expected = json!({
            "errors": {
                "username": [
                    {
                        "code": "length",
                        "message": "username must be at least 3 characters",
                        "params": {
                            "min": 3,
                            "value": "ab"
                        }
                    }
                ],
                "email": [
                    {
                        "code": "email",
                        "message": "email must be valid",
                        "params": {
                            "value": "invalid-email"
                        }
                    }
                ]
            }
        });

        assert_response_status_and_body(result.unwrap_err(), StatusCode::BAD_REQUEST, expected)
            .await;
    }

    #[tokio::test]
    async fn test_form_validate_with_message_valid() {
        let valid_form = "username=valid_user&email=test@example.com";
        let request = create_form_request(valid_form);

        let result = FormValidateWithMessage::<TestUser>::from_request(request, &()).await;
        assert!(result.is_ok());

        let user = result.unwrap().0;
        assert_eq!(user.username, "valid_user");
        assert_eq!(user.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_form_validate_with_message_invalid() {
        let invalid_form = "username=ab&email=invalid-email";
        let request = create_form_request(invalid_form);

        let result = FormValidateWithMessage::<TestUser>::from_request(request, &()).await;
        assert!(result.is_err());

        let expected = json!({
            "errors": {
                "username": [
                    {
                        "code": "length",
                        "message": "username must be at least 3 characters",
                        "params": {
                            "min": 3,
                            "value": "ab"
                        }
                    }
                ],
                "email": [
                    {
                        "code": "email",
                        "message": "email must be valid",
                        "params": {
                            "value": "invalid-email"
                        }
                    }
                ]
            }
        });

        assert_response_status_and_body(result.unwrap_err(), StatusCode::BAD_REQUEST, expected)
            .await;
    }

    #[tokio::test]
    async fn test_json_validate_valid() {
        let valid_json = r#"{"username": "valid_user", "email": "test@example.com"}"#;
        let request = create_json_request(valid_json);

        let result = JsonValidate::<TestUser>::from_request(request, &()).await;
        assert!(result.is_ok());

        let user = result.unwrap().0;
        assert_eq!(user.username, "valid_user");
        assert_eq!(user.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_json_validate_invalid() {
        let invalid_json = r#"{"username": "ab", "email": "invalid-email"}"#;
        let request = create_json_request(invalid_json);

        let result = JsonValidate::<TestUser>::from_request(request, &()).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        if let Error::BadRequest(msg) = &err {
            assert_eq!(msg, &String::new());
        } else {
            panic!("Expected BadRequest error");
        }

        let expected = json!({
            "error": "Bad Request"
        });

        assert_response_status_and_body(err, StatusCode::BAD_REQUEST, expected).await;
    }

    #[tokio::test]
    async fn test_form_validate_valid() {
        let valid_form = "username=valid_user&email=test@example.com";
        let request = create_form_request(valid_form);

        let result = FormValidate::<TestUser>::from_request(request, &()).await;
        assert!(result.is_ok());

        let user = result.unwrap().0;
        assert_eq!(user.username, "valid_user");
        assert_eq!(user.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_form_validate_invalid() {
        let invalid_form = "username=ab&email=invalid-email";
        let request = create_form_request(invalid_form);

        let result = FormValidate::<TestUser>::from_request(request, &()).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        if let Error::BadRequest(msg) = &err {
            assert_eq!(msg, &String::new());
        } else {
            panic!("Expected BadRequest error");
        }

        let expected = json!({
            "error": "Bad Request"
        });

        assert_response_status_and_body(err, StatusCode::BAD_REQUEST, expected).await;
    }

    #[tokio::test]
    async fn test_malformed_json() {
        let invalid_json = r#"{"username": "valid_user", "email": "test@example.com"#;
        let request = create_json_request(invalid_json);

        let result = JsonValidate::<TestUser>::from_request(request, &()).await;
        assert!(result.is_err());

        let expected = json!({
            "error": "Bad Request"
        });

        assert_response_status_and_body(result.unwrap_err(), StatusCode::BAD_REQUEST, expected)
            .await;
    }

    #[tokio::test]
    async fn test_malformed_form() {
        let invalid_form = "username=valid_user&email%invalid_format";
        let request = create_form_request(invalid_form);

        let result = FormValidate::<TestUser>::from_request(request, &()).await;
        assert!(result.is_err());

        let expected = json!({
            "error": "internal_server_error",
            "description": "Internal Server Error"
        });

        assert_response_status_and_body(
            result.unwrap_err(),
            StatusCode::INTERNAL_SERVER_ERROR,
            expected,
        )
        .await;
    }

    #[tokio::test]
    async fn test_path_validate_with_message_valid() {
        let request = create_path_request("/users/42");

        let result = PathValidateWithMessage::<TestPathParams>::from_request_parts(
            &mut request.into_parts().0,
            &(),
        )
        .await;
        assert!(result.is_ok());

        let params = result.unwrap().0;
        assert_eq!(params.id, 42);
    }

    #[tokio::test]
    async fn test_path_validate_with_message_invalid() {
        let request = create_path_request("/users/0");

        let result = PathValidateWithMessage::<TestPathParams>::from_request_parts(
            &mut request.into_parts().0,
            &(),
        )
        .await;
        assert!(result.is_err());

        let expected = json!({
            "errors": {
                "id": [
                    {
                        "code": "range",
                        "message": "id must be at least 1",
                        "params": {
                            "min": 1,
                            "value": 0
                        }
                    }
                ]
            }
        });

        assert_response_status_and_body(result.unwrap_err(), StatusCode::BAD_REQUEST, expected)
            .await;
    }

    #[tokio::test]
    async fn test_path_validate_valid() {
        let request = create_path_request("/users/42");

        let result =
            PathValidate::<TestPathParams>::from_request_parts(&mut request.into_parts().0, &())
                .await;
        assert!(result.is_ok());

        let params = result.unwrap().0;
        assert_eq!(params.id, 42);
    }

    #[tokio::test]
    async fn test_path_validate_invalid() {
        let request = create_path_request("/users/0");

        let result =
            PathValidate::<TestPathParams>::from_request_parts(&mut request.into_parts().0, &())
                .await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        if let Error::BadRequest(msg) = &err {
            assert_eq!(msg, &String::new());
        } else {
            panic!("Expected BadRequest error");
        }

        let expected = json!({
            "error": "Bad Request"
        });

        assert_response_status_and_body(err, StatusCode::BAD_REQUEST, expected).await;
    }

    #[tokio::test]
    async fn test_query_validate_with_message_valid() {
        let valid_query = "username=valid_user&email=test@example.com";
        let request = create_query_request(valid_query);

        let result = QueryValidateWithMessage::<TestQueryParams>::from_request(request, &()).await;
        assert!(result.is_ok());

        let params = result.unwrap().0;
        assert_eq!(params.username, "valid_user");
        assert_eq!(params.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_query_validate_with_message_invalid() {
        let invalid_query = "username=ab&email=invalid-email";
        let request = create_query_request(invalid_query);

        let result = QueryValidateWithMessage::<TestQueryParams>::from_request(request, &()).await;
        assert!(result.is_err());

        let expected = json!({
            "errors": {
                "username": [
                    {
                        "code": "length",
                        "message": "username must be at least 3 characters",
                        "params": {
                            "min": 3,
                            "value": "ab"
                        }
                    }
                ],
                "email": [
                    {
                        "code": "email",
                        "message": "email must be valid",
                        "params": {
                            "value": "invalid-email"
                        }
                    }
                ]
            }
        });

        assert_response_status_and_body(result.unwrap_err(), StatusCode::BAD_REQUEST, expected)
            .await;
    }

    #[tokio::test]
    async fn test_query_validate_valid() {
        let valid_query = "username=valid_user&email=test@example.com";
        let request = create_query_request(valid_query);

        let result = QueryValidate::<TestQueryParams>::from_request(request, &()).await;
        assert!(result.is_ok());

        let params = result.unwrap().0;
        assert_eq!(params.username, "valid_user");
        assert_eq!(params.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_query_validate_invalid() {
        let invalid_query = "username=ab&email=invalid-email";
        let request = create_query_request(invalid_query);

        let result = QueryValidate::<TestQueryParams>::from_request(request, &()).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        if let Error::BadRequest(msg) = &err {
            assert_eq!(msg, &String::new());
        } else {
            panic!("Expected BadRequest error");
        }

        let expected = json!({
            "error": "Bad Request"
        });

        assert_response_status_and_body(err, StatusCode::BAD_REQUEST, expected).await;
    }

    #[tokio::test]
    async fn test_malformed_query() {
        let invalid_query = "username=valid_user&email%invalid_format";
        let request = create_query_request(invalid_query);

        let result = QueryValidate::<TestQueryParams>::from_request(request, &()).await;
        assert!(result.is_err());

        let expected = json!({
            "error": "Bad Request"
        });

        assert_response_status_and_body(result.unwrap_err(), StatusCode::BAD_REQUEST, expected)
            .await;
    }
}
