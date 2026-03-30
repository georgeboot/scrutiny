use crate::value::FieldValue;

/// Check if value is in a list of allowed values.
pub fn is_in(value: &str, list: &[&str]) -> bool {
    list.contains(&value)
}

/// Check if value is NOT in a list of values.
pub fn is_not_in(value: &str, list: &[&str]) -> bool {
    !list.contains(&value)
}

/// Check if two field values are equal.
pub fn is_same(a: &FieldValue, b: &FieldValue) -> bool {
    a == b
}

/// Check if two field values are different.
pub fn is_different(a: &FieldValue, b: &FieldValue) -> bool {
    a != b
}

/// Check if field value a > field value b.
pub fn is_gt(a: &FieldValue, b: &FieldValue) -> bool {
    a.partial_cmp(b)
        .is_some_and(|o| o == std::cmp::Ordering::Greater)
}

/// Check if field value a >= field value b.
pub fn is_gte(a: &FieldValue, b: &FieldValue) -> bool {
    a.partial_cmp(b)
        .is_some_and(|o| o != std::cmp::Ordering::Less)
}

/// Check if field value a < field value b.
pub fn is_lt(a: &FieldValue, b: &FieldValue) -> bool {
    a.partial_cmp(b)
        .is_some_and(|o| o == std::cmp::Ordering::Less)
}

/// Check if field value a <= field value b.
pub fn is_lte(a: &FieldValue, b: &FieldValue) -> bool {
    a.partial_cmp(b)
        .is_some_and(|o| o != std::cmp::Ordering::Greater)
}

/// Check if a value exists in another field's array.
pub fn is_in_array(value: &FieldValue, array: &FieldValue) -> bool {
    if let FieldValue::List(items) = array {
        items.contains(value)
    } else {
        false
    }
}

/// Check if all items in a list are distinct (no duplicates).
pub fn is_distinct(values: &FieldValue) -> bool {
    if let FieldValue::List(items) = values {
        for i in 0..items.len() {
            for j in (i + 1)..items.len() {
                if items[i] == items[j] {
                    return false;
                }
            }
        }
        true
    } else {
        true
    }
}
