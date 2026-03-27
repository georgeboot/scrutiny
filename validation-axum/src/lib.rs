//! Axum integration for the `validation` crate.
//!
//! Provides extractors that deserialize **and** validate request data in one step.
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
//!     #[validate(required, email)]
//!     email: Option<String>,
//!     #[validate(required, min = 2)]
//!     name: Option<String>,
//! }
//!
//! async fn create_user(Valid(user): Valid<CreateUser>) -> impl IntoResponse {
//!     Json(serde_json::json!({ "email": user.email }))
//! }
//!
//! let app = Router::new().route("/users", post(create_user));
//! ```
//!
//! Invalid requests get a **422 Unprocessable Entity** JSON response:
//!
//! ```json
//! {
//!   "message": "The given data was invalid.",
//!   "errors": {
//!     "email": ["The email field is required."],
//!     "name": ["The name field must be at least 2 characters."]
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
//!     fn from_deserialization_error(error: String) -> Response { /* ... */ }
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
use validation::error::ValidationErrors;
use validation::traits::Validate;

/// Trait for customizing how validation errors become HTTP responses.
/// Implement this to use your own error format with `ValidWith<T, E>`.
pub trait ValidationErrorResponse: Send + Sync + 'static {
    fn from_validation_errors(errors: ValidationErrors) -> Response;
    fn from_deserialization_error(error: String) -> Response;
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

    fn from_deserialization_error(error: String) -> Response {
        let body = serde_json::json!({
            "message": "The given data was invalid.",
            "errors": { "_deserialization": [error] }
        });
        (StatusCode::UNPROCESSABLE_ENTITY, axum::Json(body)).into_response()
    }
}

// ── Valid<T> — the simple extractor with default error response ──

/// Extracts and validates JSON body. Drop-in replacement for `axum::Json<T>`.
///
/// Returns a 422 JSON response on validation failure (Laravel-compatible format).
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

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::Json(value) = axum::Json::<T>::from_request(req, state)
            .await
            .map_err(|e| DefaultErrorResponse::from_deserialization_error(e.to_string()))?;

        value
            .validate()
            .map_err(|e| DefaultErrorResponse::from_validation_errors(e))?;

        Ok(Valid(value))
    }
}

// ── ValidWith<T, E> — customizable error response ──

/// Extracts and validates JSON body with a custom error response type.
///
/// ```ignore
/// type V<T> = ValidWith<T, MyApiError>;
///
/// async fn create_user(V(user): V<CreateUser>) -> impl IntoResponse {
///     // user is validated, errors use MyApiError format
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

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::Json(value) = axum::Json::<T>::from_request(req, state)
            .await
            .map_err(|e| E::from_deserialization_error(e.to_string()))?;

        value
            .validate()
            .map_err(|e| E::from_validation_errors(e))?;

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

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::Form(value) = axum::Form::<T>::from_request(req, state)
            .await
            .map_err(|e| DefaultErrorResponse::from_deserialization_error(e.to_string()))?;

        value
            .validate()
            .map_err(|e| DefaultErrorResponse::from_validation_errors(e))?;

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

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Query(value) =
            axum::extract::Query::<T>::from_request(req, state)
                .await
                .map_err(|e| DefaultErrorResponse::from_deserialization_error(e.to_string()))?;

        value
            .validate()
            .map_err(|e| DefaultErrorResponse::from_validation_errors(e))?;

        Ok(ValidQuery(value))
    }
}
