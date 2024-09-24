//! # Utility Function to Extract and Modify HTTP Headers
//!
//! This module provides a helper function to retrieve headers from an HTTP
//! response while removing certain sensitive or irrelevant headers.

/// Extracts the headers from a [`reqwest::Response`] and removes dynamic headers.
pub fn get_headers_from_response(response: reqwest::Response) -> hyper::HeaderMap {
    let mut headers = response.headers().clone();
    headers.remove("x-request-id");
    headers.remove("date");
    headers
}
