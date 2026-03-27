//! Type-erased field values for cross-field comparison.
//!
//! The derive macro generates `FieldAccess` implementations that convert each
//! struct field into a [`FieldValue`]. This enables rules like `same`, `gt`,
//! `required_if`, etc. to compare fields at runtime despite Rust's static types.

/// Type-erased representation of a field value for cross-field comparison.
/// The derive macro generates conversions from actual field types into this enum.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    None,
    Bool(bool),
    Int(i64),
    Uint(u64),
    Float(f64),
    String(String),
    List(Vec<FieldValue>),
}

impl FieldValue {
    pub fn is_none(&self) -> bool {
        matches!(self, FieldValue::None)
    }

    pub fn is_empty(&self) -> bool {
        match self {
            FieldValue::None => true,
            FieldValue::String(s) => s.is_empty(),
            FieldValue::List(l) => l.is_empty(),
            _ => false,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            FieldValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            FieldValue::Int(i) => Some(*i as f64),
            FieldValue::Uint(u) => Some(*u as f64),
            FieldValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn len(&self) -> Option<usize> {
        match self {
            FieldValue::String(s) => Some(s.len()),
            FieldValue::List(l) => Some(l.len()),
            _ => None,
        }
    }
}

impl PartialOrd for FieldValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (FieldValue::Int(a), FieldValue::Int(b)) => a.partial_cmp(b),
            (FieldValue::Uint(a), FieldValue::Uint(b)) => a.partial_cmp(b),
            (FieldValue::Float(a), FieldValue::Float(b)) => a.partial_cmp(b),
            (FieldValue::Int(a), FieldValue::Float(b)) => (*a as f64).partial_cmp(b),
            (FieldValue::Float(a), FieldValue::Int(b)) => a.partial_cmp(&(*b as f64)),
            (FieldValue::Uint(a), FieldValue::Float(b)) => (*a as f64).partial_cmp(b),
            (FieldValue::Float(a), FieldValue::Uint(b)) => a.partial_cmp(&(*b as f64)),
            (FieldValue::Int(a), FieldValue::Uint(b)) => (*a as i128).partial_cmp(&(*b as i128)),
            (FieldValue::Uint(a), FieldValue::Int(b)) => (*a as i128).partial_cmp(&(*b as i128)),
            (FieldValue::String(a), FieldValue::String(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

// Conversion impls used by the derive macro

impl From<&str> for FieldValue {
    fn from(s: &str) -> Self {
        FieldValue::String(s.to_string())
    }
}

impl From<String> for FieldValue {
    fn from(s: String) -> Self {
        FieldValue::String(s)
    }
}

impl From<&String> for FieldValue {
    fn from(s: &String) -> Self {
        FieldValue::String(s.clone())
    }
}

impl From<bool> for FieldValue {
    fn from(b: bool) -> Self {
        FieldValue::Bool(b)
    }
}

impl From<&bool> for FieldValue {
    fn from(b: &bool) -> Self {
        FieldValue::Bool(*b)
    }
}

macro_rules! impl_from_int {
    ($($t:ty),*) => {
        $(
            impl From<$t> for FieldValue {
                fn from(v: $t) -> Self {
                    FieldValue::Int(v as i64)
                }
            }
            impl From<&$t> for FieldValue {
                fn from(v: &$t) -> Self {
                    FieldValue::Int(*v as i64)
                }
            }
        )*
    };
}

macro_rules! impl_from_uint {
    ($($t:ty),*) => {
        $(
            impl From<$t> for FieldValue {
                fn from(v: $t) -> Self {
                    FieldValue::Uint(v as u64)
                }
            }
            impl From<&$t> for FieldValue {
                fn from(v: &$t) -> Self {
                    FieldValue::Uint(*v as u64)
                }
            }
        )*
    };
}

macro_rules! impl_from_float {
    ($($t:ty),*) => {
        $(
            impl From<$t> for FieldValue {
                fn from(v: $t) -> Self {
                    FieldValue::Float(v as f64)
                }
            }
            impl From<&$t> for FieldValue {
                fn from(v: &$t) -> Self {
                    FieldValue::Float(*v as f64)
                }
            }
        )*
    };
}

impl_from_int!(i8, i16, i32, i64);
impl_from_uint!(u8, u16, u32, u64);
impl_from_float!(f32, f64);

impl<T: Into<FieldValue> + Clone> From<&Option<T>> for FieldValue {
    fn from(opt: &Option<T>) -> Self {
        match opt {
            Some(v) => v.clone().into(),
            None => FieldValue::None,
        }
    }
}

impl<T: Into<FieldValue> + Clone> From<&Vec<T>> for FieldValue {
    fn from(vec: &Vec<T>) -> Self {
        FieldValue::List(vec.iter().map(|v| v.clone().into()).collect())
    }
}

