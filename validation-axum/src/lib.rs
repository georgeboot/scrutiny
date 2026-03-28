//! Axum integration for the `validation` crate.
//!
//! Provides extractors that deserialize **and** validate request data in one step.
//! Deserialization errors are reported as field-level validation errors — not generic
//! error strings. This means typed fields like `uuid::Uuid` or `chrono::NaiveDate`
//! produce the same structured error format as validation failures.
//!
//! # Extractors
//!
//! | Extractor | Wraps | Description |
//! |-----------|-------|-------------|
//! | [`Valid<T>`] | `Json<T>` | JSON body with validation |
//! | [`ValidWith<T, E>`] | `Json<T>` | JSON body with custom error response |
//! | [`ValidForm<T>`] | `Form<T>` | Form-encoded body with validation |
//! | [`ValidQuery<T>`] | `Query<T>` | Query parameters with validation |
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use validation::Validate;
//! use validation_axum::Valid;
//! use axum::{Router, routing::post, response::IntoResponse, Json};
//! use serde::Deserialize;
//!
//! #[derive(Validate, Deserialize)]
//! struct CreateUser {
//!     #[validate(required, min = 2)]
//!     name: Option<String>,
//!     id: uuid::Uuid,  // deserialization error → field-level validation error
//! }
//!
//! async fn create_user(Valid(user): Valid<CreateUser>) -> impl IntoResponse {
//!     Json(serde_json::json!({ "name": user.name }))
//! }
//!
//! let app = Router::new().route("/users", post(create_user));
//! ```
//!
//! Both validation failures and deserialization errors produce the same format:
//!
//! ```json
//! {
//!   "message": "The given data was invalid.",
//!   "errors": {
//!     "name": ["The name field is required."],
//!     "id": ["invalid type: string \"not-a-uuid\", expected UUID"]
//!   }
//! }
//! ```
//!
//! # Custom Error Responses
//!
//! Implement [`ValidationErrorResponse`] to control the HTTP response format:
//!
//! ```rust,ignore
//! use validation_axum::{ValidWith, ValidationErrorResponse};
//!
//! struct MyApiError;
//! impl ValidationErrorResponse for MyApiError {
//!     fn from_validation_errors(errors: ValidationErrors) -> Response { /* ... */ }
//! }
//!
//! async fn handler(result: ValidWith<CreateUser, MyApiError>) -> impl IntoResponse {
//!     let user = result.into_inner();
//!     // ...
//! }
//! ```

use axum::extract::{FromRequest, Request};
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use std::marker::PhantomData;
use validation::error::{ValidationError, ValidationErrors};
use validation::traits::Validate;

/// Trait for customizing how validation errors become HTTP responses.
/// Implement this to use your own error format with `ValidWith<T, E>`.
///
/// Both deserialization errors and validation errors are unified into
/// `ValidationErrors` before being passed to this trait.
pub trait ValidationErrorResponse: Send + Sync + 'static {
    fn from_validation_errors(errors: ValidationErrors) -> Response;
}

/// Default error response: Laravel-compatible 422 JSON.
pub struct DefaultErrorResponse;

impl ValidationErrorResponse for DefaultErrorResponse {
    fn from_validation_errors(errors: ValidationErrors) -> Response {
        let body = serde_json::json!({
            "message": "The given data was invalid.",
            "errors": errors
        });
        (StatusCode::UNPROCESSABLE_ENTITY, axum::Json(body)).into_response()
    }
}

/// Convert a serde_path_to_error path string to a field name.
/// Top-level errors (empty path or ".") map to "_body".
fn path_to_field_name(path: String) -> String {
    if path.is_empty() || path == "." {
        "_body".to_string()
    } else {
        // Convert serde_path_to_error's dot notation to our field names
        // e.g. "address.city" stays as-is, ".foo" becomes "foo"
        path.strip_prefix('.').unwrap_or(&path).to_string()
    }
}

/// Deserialize JSON bytes using `serde_path_to_error` and convert any
/// deserialization error into a field-level `ValidationErrors`.
fn deserialize_json<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T, ValidationErrors> {
    let deserializer = &mut serde_json::Deserializer::from_slice(body);
    serde_path_to_error::deserialize(deserializer).map_err(|err| {
        let mut errors = ValidationErrors::new();
        let path = err.path().to_string();
        let field = path_to_field_name(path);
        let message = err.inner().to_string();
        errors.add(&field, ValidationError::new("deserialization", message));
        errors
    })
}

/// Deserialize a query/form string using `serde_path_to_error` and convert any
/// deserialization error into a field-level `ValidationErrors`.
fn deserialize_query<T: serde::de::DeserializeOwned>(query: &str) -> Result<T, ValidationErrors> {
    let deserializer = serde_urlencoded::Deserializer::new(form_urlencoded::parse(query.as_bytes()));
    serde_path_to_error::deserialize(deserializer).map_err(|err| {
        let mut errors = ValidationErrors::new();
        let path = err.path().to_string();
        let field = path_to_field_name(path);
        let message = err.inner().to_string();
        errors.add(&field, ValidationError::new("deserialization", message));
        errors
    })
}

// ── Valid<T> — the simple extractor with default error response ──

/// Extracts and validates JSON body. Drop-in replacement for `axum::Json<T>`.
///
/// Deserialization errors are converted to field-level validation errors using
/// `serde_path_to_error`, so typed fields like `uuid::Uuid` produce structured
/// errors on the correct field name.
///
/// ```ignore
/// async fn create_user(Valid(user): Valid<CreateUser>) -> impl IntoResponse {
///     // user is already deserialized and validated
/// }
/// ```
pub struct Valid<T>(pub T);

impl<T, S> FromRequest<S> for Valid<T>
where
    T: serde::de::DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let body = axum::body::to_bytes(req.into_body(), usize::MAX)
            .await
            .map_err(|e| {
                let mut errors = ValidationErrors::new();
                errors.add("_body", ValidationError::new("body", e.to_string()));
                DefaultErrorResponse::from_validation_errors(errors)
            })?;

        let value: T = deserialize_json(&body)
            .map_err(DefaultErrorResponse::from_validation_errors)?;

        value
            .validate()
            .map_err(DefaultErrorResponse::from_validation_errors)?;

        Ok(Valid(value))
    }
}

// ── ValidWith<T, E> — customizable error response ──

/// Extracts and validates JSON body with a custom error response type.
///
/// ```ignore
/// async fn handler(result: ValidWith<CreateUser, MyApiError>) -> impl IntoResponse {
///     let user = result.into_inner();
/// }
/// ```
pub struct ValidWith<T, E: ValidationErrorResponse = DefaultErrorResponse>(
    pub T,
    PhantomData<E>,
);

impl<T, E: ValidationErrorResponse> ValidWith<T, E> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T, E, S> FromRequest<S> for ValidWith<T, E>
where
    T: serde::de::DeserializeOwned + Validate + Send,
    E: ValidationErrorResponse,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let body = axum::body::to_bytes(req.into_body(), usize::MAX)
            .await
            .map_err(|e| {
                let mut errors = ValidationErrors::new();
                errors.add("_body", ValidationError::new("body", e.to_string()));
                E::from_validation_errors(errors)
            })?;

        let value: T = deserialize_json(&body)
            .map_err(E::from_validation_errors)?;

        value
            .validate()
            .map_err(E::from_validation_errors)?;

        Ok(ValidWith(value, PhantomData))
    }
}

// ── ValidForm<T> — form data extraction + validation ──

/// Extracts and validates form-encoded body.
pub struct ValidForm<T>(pub T);

impl<T, S> FromRequest<S> for ValidForm<T>
where
    T: serde::de::DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let body = axum::body::to_bytes(req.into_body(), usize::MAX)
            .await
            .map_err(|e| {
                let mut errors = ValidationErrors::new();
                errors.add("_body", ValidationError::new("body", e.to_string()));
                DefaultErrorResponse::from_validation_errors(errors)
            })?;

        let query_str = std::str::from_utf8(&body).map_err(|e| {
            let mut errors = ValidationErrors::new();
            errors.add("_body", ValidationError::new("encoding", e.to_string()));
            DefaultErrorResponse::from_validation_errors(errors)
        })?;

        let value: T = deserialize_query(query_str)
            .map_err(DefaultErrorResponse::from_validation_errors)?;

        value
            .validate()
            .map_err(DefaultErrorResponse::from_validation_errors)?;

        Ok(ValidForm(value))
    }
}

// ── ValidQuery<T> — query parameter extraction + validation ──

/// Extracts and validates query parameters.
pub struct ValidQuery<T>(pub T);

impl<T, S> FromRequest<S> for ValidQuery<T>
where
    T: serde::de::DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let query_str = req.uri().query().unwrap_or("");

        let value: T = deserialize_query(query_str)
            .map_err(DefaultErrorResponse::from_validation_errors)?;

        value
            .validate()
            .map_err(DefaultErrorResponse::from_validation_errors)?;

        Ok(ValidQuery(value))
    }
}
