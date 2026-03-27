/// Check if an Option value is present (Some and non-empty for strings).
pub fn is_present_option<T: Presentable>(value: &Option<T>) -> bool {
    match value {
        None => false,
        Some(v) => v.is_present(),
    }
}

/// Check if a value is "present" (non-empty).
pub trait Presentable {
    fn is_present(&self) -> bool;
}

impl Presentable for String {
    fn is_present(&self) -> bool {
        !self.trim().is_empty()
    }
}

impl Presentable for bool {
    fn is_present(&self) -> bool {
        true
    }
}

impl<T> Presentable for Vec<T> {
    fn is_present(&self) -> bool {
        !self.is_empty()
    }
}

macro_rules! impl_presentable_numeric {
    ($($t:ty),*) => {
        $(
            impl Presentable for $t {
                fn is_present(&self) -> bool {
                    true
                }
            }
        )*
    };
}

impl_presentable_numeric!(i8, i16, i32, i64, u8, u16, u32, u64, f32, f64);

/// Check if a value is "accepted" (true, "yes", "on", "1", 1).
pub fn is_accepted(value: &str) -> bool {
    matches!(value.to_lowercase().as_str(), "yes" | "on" | "1" | "true")
}

pub fn is_accepted_bool(value: bool) -> bool {
    value
}

/// Check if a value is "declined" (false, "no", "off", "0", 0).
pub fn is_declined(value: &str) -> bool {
    matches!(value.to_lowercase().as_str(), "no" | "off" | "0" | "false")
}

pub fn is_declined_bool(value: bool) -> bool {
    !value
}

/// Filled: if the field is present, it must not be empty.
/// Unlike required, this doesn't demand the field's presence.
pub fn is_filled(value: &str) -> bool {
    !value.trim().is_empty()
}
