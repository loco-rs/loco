use axum::extract::{Form, FromRequest, Json, Request};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::Error;

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

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{self, Request as HttpRequest, StatusCode},
        response::IntoResponse,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::{Value, json};
    use validator::Validate;

    use super::*;

    // Define a test struct that implements Validate
    #[derive(Debug, Serialize, Deserialize, Validate)]
    struct TestUser {
        #[validate(length(min = 3, message = "username must be at least 3 characters"))]
        username: String,
        #[validate(email(message = "email must be valid"))]
        email: String,
    }

    // Helper function to create a mock JSON request
    fn create_json_request(json: &str) -> HttpRequest<Body> {
        HttpRequest::builder()
            .method(http::Method::POST)
            .uri("/test")
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(json.to_string()))
            .unwrap()
    }

    // Helper function to create a mock Form request
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

    // Helper function to check the status code and get JSON response
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
        let invalid_json = r#"{"username": "valid_user", "email": "test@example.com"#; // Missing closing brace
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
}
