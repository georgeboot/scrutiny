/// Check minimum length of a string.
pub fn check_min_length(value: &str, min: usize) -> bool {
    value.len() >= min
}

/// Check maximum length of a string.
pub fn check_max_length(value: &str, max: usize) -> bool {
    value.len() <= max
}

/// Check length is between min and max (inclusive).
pub fn check_between_length(value: &str, min: usize, max: usize) -> bool {
    let len = value.len();
    len >= min && len <= max
}

/// Check exact size (length for strings).
pub fn check_size(value: &str, size: usize) -> bool {
    value.len() == size
}

/// Check if string contains only alphabetic characters.
pub fn is_alpha(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_alphabetic())
}

/// Check if string contains only alphanumeric characters.
pub fn is_alpha_num(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_alphanumeric())
}

/// Check if string contains only alphanumeric characters, dashes, and underscores.
pub fn is_alpha_dash(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Check if string is numeric (can be parsed as a number).
pub fn is_numeric(value: &str) -> bool {
    value.parse::<f64>().is_ok()
}

/// Check if string is a valid integer.
pub fn is_integer(value: &str) -> bool {
    value.parse::<i64>().is_ok()
}

/// Check if string starts with given prefix.
pub fn starts_with(value: &str, prefix: &str) -> bool {
    value.starts_with(prefix)
}

/// Check if string ends with given suffix.
pub fn ends_with(value: &str, suffix: &str) -> bool {
    value.ends_with(suffix)
}

/// Check if string does NOT start with given prefix.
pub fn doesnt_start_with(value: &str, prefix: &str) -> bool {
    !value.starts_with(prefix)
}

/// Check if string does NOT end with given suffix.
pub fn doesnt_end_with(value: &str, suffix: &str) -> bool {
    !value.ends_with(suffix)
}

/// Check if string contains a substring.
pub fn contains(value: &str, needle: &str) -> bool {
    value.contains(needle)
}

/// Check if string does NOT contain a substring.
pub fn doesnt_contain(value: &str, needle: &str) -> bool {
    !value.contains(needle)
}

/// Check if string is entirely uppercase.
pub fn is_uppercase(value: &str) -> bool {
    !value.is_empty() && value == value.to_uppercase()
}

/// Check if string is entirely lowercase.
pub fn is_lowercase(value: &str) -> bool {
    !value.is_empty() && value == value.to_lowercase()
}
