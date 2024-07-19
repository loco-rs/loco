use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use tracing::Span;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RequestId(uuid::Uuid);

impl RequestId {
    /// Create a new request id
    ///
    /// # Arguments
    /// * `uuid::Uuid` - The request id
    /// # Returns
    /// * `Self` - The request id
    #[must_use]
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }
    /// Get the request id
    #[must_use]
    pub fn get(&self) -> &Uuid {
        &self.0
    }
}

pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    let request_id = Uuid::new_v4();
    Span::current().record("request_id", request_id.to_string().as_str());
    request.extensions_mut().insert(RequestId::new(request_id));
    next.run(request).await
}
