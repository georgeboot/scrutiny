/// Check minimum value.
pub fn check_min<T: PartialOrd>(value: T, min: T) -> bool {
    value >= min
}

/// Check maximum value.
pub fn check_max<T: PartialOrd>(value: T, max: T) -> bool {
    value <= max
}

/// Check value is between min and max (inclusive).
pub fn check_between<T: PartialOrd>(value: T, min: T, max: T) -> bool {
    value >= min && value <= max
}

/// Check exact digit count in a numeric string.
pub fn check_digits(value: &str, count: usize) -> bool {
    let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.len() == count
}

/// Check digit count is between min and max.
pub fn check_digits_between(value: &str, min: usize, max: usize) -> bool {
    let count = value.chars().filter(|c| c.is_ascii_digit()).count();
    count >= min && count <= max
}

/// Check if value is a multiple of N.
pub fn is_multiple_of(value: f64, n: f64) -> bool {
    if n == 0.0 {
        return false;
    }
    (value % n).abs() < f64::EPSILON
}

/// Check if a numeric string has exactly N decimal places.
pub fn check_decimal(value: &str, min_places: usize, max_places: Option<usize>) -> bool {
    if let Some(dot_pos) = value.find('.') {
        let decimal_part = &value[dot_pos + 1..];
        let places = decimal_part.len();
        match max_places {
            Some(max) => places >= min_places && places <= max,
            None => places == min_places,
        }
    } else {
        // No decimal point — 0 decimal places
        min_places == 0
    }
}
