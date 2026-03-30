use axum::{Router, routing::post, response::IntoResponse, Json};
use http::{Request, StatusCode};
use serde::{Deserialize, Serialize};
use tower::ServiceExt;
use scrutiny::Validate;
use scrutiny_axum::{Valid, ValidWith, ValidationErrorResponse};

#[derive(Validate, Deserialize, Serialize)]
struct CreateUser {
    #[validate(required, email)]
    email: Option<String>,
    #[validate(required, min = 2)]
    name: Option<String>,
}

async fn create_user_handler(Valid(user): Valid<CreateUser>) -> impl IntoResponse {
    Json(serde_json::json!({ "email": user.email, "name": user.name }))
}

fn app() -> Router {
    Router::new().route("/users", post(create_user_handler))
}

fn json_request(body: &str) -> Request<axum::body::Body> {
    Request::builder()
        .method("POST")
        .uri("/users")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body.to_string()))
        .unwrap()
}

async fn body_to_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn test_valid_request() {
    let response = app()
        .oneshot(json_request(r#"{"email": "test@example.com", "name": "John"}"#))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_validation_failure_returns_422() {
    let response = app()
        .oneshot(json_request(r#"{"email": null, "name": null}"#))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let json = body_to_json(response).await;
    assert_eq!(json["message"], "The given data was invalid.");
    assert!(json["errors"]["email"].is_array());
    assert!(json["errors"]["name"].is_array());
}

#[tokio::test]
async fn test_email_validation_error() {
    let response = app()
        .oneshot(json_request(r#"{"email": "not-valid", "name": "John"}"#))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let json = body_to_json(response).await;
    let email_errors = json["errors"]["email"].as_array().unwrap();
    assert_eq!(email_errors.len(), 1);
    assert!(email_errors[0].as_str().unwrap().contains("email"));
}

#[tokio::test]
async fn test_deserialization_error_returns_422() {
    let response = app()
        .oneshot(json_request(r#"not json at all"#))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let json = body_to_json(response).await;
    // Top-level deserialization errors land on "_body"
    assert!(json["errors"]["_body"].is_array());
}

// --- Custom error response ---

struct CustomApiError;

impl ValidationErrorResponse for CustomApiError {
    fn from_validation_errors(errors: scrutiny::error::ValidationErrors) -> axum::response::Response {
        let body = serde_json::json!({
            "success": false,
            "code": "VALIDATION_FAILED",
            "details": errors.messages(),
        });
        (StatusCode::BAD_REQUEST, Json(body)).into_response()
    }

}

async fn custom_handler(result: ValidWith<CreateUser, CustomApiError>) -> impl IntoResponse {
    let _user = result.into_inner();
    Json(serde_json::json!({ "ok": true }))
}

#[tokio::test]
async fn test_custom_error_response() {
    let app = Router::new().route("/users", post(custom_handler));

    let response = app
        .oneshot(json_request(r#"{"email": null, "name": null}"#))
        .await
        .unwrap();

    // Custom response uses 400 instead of 422
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let json = body_to_json(response).await;
    assert_eq!(json["success"], false);
    assert_eq!(json["code"], "VALIDATION_FAILED");
    assert!(json["details"].is_object());
}

// --- Typed field deserialization errors become field-level validation errors ---

#[derive(Validate, Deserialize)]
struct TypedRequest {
    #[validate(required)]
    name: Option<String>,
    id: u64,
}

async fn typed_handler(Valid(req): Valid<TypedRequest>) -> impl IntoResponse {
    Json(serde_json::json!({ "id": req.id }))
}

#[tokio::test]
async fn test_typed_field_deser_error_is_field_level() {
    let app = Router::new().route("/typed", post(typed_handler));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/typed")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(r#"{"name": "John", "id": "not-a-number"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let json = body_to_json(response).await;
    // The error should be on the "id" field, not a generic deserialization error
    assert!(json["errors"]["id"].is_array(), "expected field-level error on 'id', got: {json}");
    let id_errors = json["errors"]["id"].as_array().unwrap();
    assert_eq!(id_errors.len(), 1);
}

#[tokio::test]
async fn test_typed_field_valid_request_passes() {
    let app = Router::new().route("/typed", post(typed_handler));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/typed")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(r#"{"name": "John", "id": 42}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
