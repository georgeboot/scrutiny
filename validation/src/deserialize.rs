//! Deserialize-and-validate helpers.
//!
//! These functions deserialize JSON into a struct using `serde_path_to_error`,
//! then run validation. Deserialization errors are converted to field-level
//! [`ValidationErrors`](crate::error::ValidationErrors) — so a `uuid::Uuid` field
//! that receives `"not-a-uuid"` produces `{"id": ["expected UUID"]}` instead of
//! a generic parse error.
//!
//! # Usage
//!
//! ```rust,ignore
//! use validation::deserialize::from_json;
//!
//! let input = br#"{"name": "Jo", "id": "not-a-uuid"}"#;
//! match from_json::<CreateUser>(input) {
//!     Ok(user) => { /* deserialized AND validated */ }
//!     Err(errors) => {
//!         // errors.messages() → {"id": ["invalid type: ..."]}
//!     }
//! }
//! ```
//!
//! ## For axum users
//!
//! You don't need these functions directly — the `validation-axum` extractors
//! (`Valid<T>`, `ValidForm<T>`, etc.) call them automatically.
//!
//! Requires the `serde_json` and `serde_path_to_error` features (both default).

use crate::error::{ValidationError, ValidationErrors};
use crate::traits::Validate;

/// Convert a `serde_path_to_error` path to a dot-notation field name.
fn path_to_field_name(path: String) -> String {
    if path.is_empty() || path == "." {
        "_body".to_string()
    } else {
        path.strip_prefix('.').unwrap_or(&path).to_string()
    }
}

/// Deserialize JSON bytes into `T`, then validate.
///
/// Deserialization errors are returned as field-level `ValidationErrors` with
/// the field path from `serde_path_to_error`. If deserialization succeeds,
/// `T::validate()` is called and its errors (if any) are returned.
///
/// # Example
///
/// ```rust
/// use validation::Validate;
/// use validation::traits::Validate as _;
/// use serde::Deserialize;
///
/// #[derive(Validate, Deserialize, Debug)]
/// struct Input {
///     #[validate(required, min = 2)]
///     name: Option<String>,
///     count: u32,
/// }
///
/// // Deserialization error on `count` → field-level error
/// let result = validation::deserialize::from_json::<Input>(br#"{"name": "Jo", "count": "abc"}"#);
/// let err = result.unwrap_err();
/// assert!(err.messages().contains_key("count"));
///
/// // Validation error on `name` (too short is fine, but missing is caught)
/// let result = validation::deserialize::from_json::<Input>(br#"{"count": 5}"#);
/// let err = result.unwrap_err();
/// assert!(err.messages().contains_key("name"));
///
/// // Both valid
/// let result = validation::deserialize::from_json::<Input>(br#"{"name": "John", "count": 5}"#);
/// assert!(result.is_ok());
/// ```
#[cfg(all(feature = "serde_json", feature = "serde_path_to_error"))]
pub fn from_json<T>(bytes: &[u8]) -> Result<T, ValidationErrors>
where
    T: serde::de::DeserializeOwned + Validate,
{
    let value: T = deserialize_json(bytes)?;
    value.validate()?;
    Ok(value)
}

/// Deserialize JSON bytes with field-level error tracking (without validation).
///
/// Use this if you only want deserialization errors as `ValidationErrors`
/// without running the validation rules.
#[cfg(all(feature = "serde_json", feature = "serde_path_to_error"))]
pub fn deserialize_json<T>(bytes: &[u8]) -> Result<T, ValidationErrors>
where
    T: serde::de::DeserializeOwned,
{
    let deserializer = &mut serde_json::Deserializer::from_slice(bytes);
    serde_path_to_error::deserialize(deserializer).map_err(|err| {
        let mut errors = ValidationErrors::new();
        let path = err.path().to_string();
        let field = path_to_field_name(path);
        let message = err.inner().to_string();
        errors.add(&field, ValidationError::new("deserialization", message));
        errors
    })
}
