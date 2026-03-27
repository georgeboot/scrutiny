//! Validation error types.
//!
//! [`ValidationErrors`] collects all field errors with dot-notation paths.
//! Serializes to `{"field": ["message1", "message2"]}` with the `serde` feature.

use std::collections::HashMap;
use std::fmt;

/// A single validation error on a field.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Rule name, e.g. "required", "min", "email"
    pub rule: String,
    /// The rendered human-readable message
    pub message: String,
    /// Parameters for message interpolation (e.g. "min" => "8")
    pub params: HashMap<String, String>,
}

impl ValidationError {
    pub fn new(rule: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            rule: rule.into(),
            message: message.into(),
            params: HashMap::new(),
        }
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }
}

/// All validation errors, keyed by field path (dot notation).
#[derive(Debug, Clone, Default)]
pub struct ValidationErrors {
    errors: HashMap<String, Vec<ValidationError>>,
}

impl ValidationErrors {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, field: &str, error: ValidationError) {
        self.errors.entry(field.to_string()).or_default().push(error);
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn field_errors(&self) -> &HashMap<String, Vec<ValidationError>> {
        &self.errors
    }

    /// Merge another ValidationErrors, prepending `prefix` to all field names.
    /// Used for nested validation: prefix = "address" produces "address.city".
    pub fn merge_with_prefix(&mut self, prefix: &str, other: ValidationErrors) {
        for (field, errs) in other.errors {
            let key = if prefix.is_empty() {
                field
            } else {
                format!("{}.{}", prefix, field)
            };
            self.errors.entry(key).or_default().extend(errs);
        }
    }

    /// Get just the messages grouped by field (the common serialization format).
    pub fn messages(&self) -> HashMap<String, Vec<String>> {
        self.errors
            .iter()
            .map(|(field, errs)| {
                (
                    field.clone(),
                    errs.iter().map(|e| e.message.clone()).collect(),
                )
            })
            .collect()
    }

    /// Get the first error message per field.
    pub fn first_messages(&self) -> HashMap<String, String> {
        self.errors
            .iter()
            .filter_map(|(field, errs)| {
                errs.first().map(|e| (field.clone(), e.message.clone()))
            })
            .collect()
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for (field, errs) in &self.errors {
            for err in errs {
                if !first {
                    write!(f, "; ")?;
                }
                write!(f, "{}: {}", field, err.message)?;
                first = false;
            }
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

#[cfg(feature = "serde")]
impl serde::Serialize for ValidationErrors {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.messages().serialize(serializer)
    }
}
