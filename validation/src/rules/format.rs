//! Format validation rules: email, URL, UUID, IP, dates, colors, etc.
//!
//! Rules delegate to dedicated, standards-compliant parsing crates.
//!
//! - `email_address` crate for RFC 5321 email validation (`email` feature)
//! - `url` crate for WHATWG URL standard (`url-parse` feature)
//! - `uuid` crate for RFC 4122 UUID validation (`uuid-parse` feature)
//! - `ulid` crate for ULID validation (`ulid-parse` feature)
//! - `chrono` crate for ISO 8601 date/datetime parsing (`chrono` feature)
//! - `chrono-tz` crate for IANA timezone database (`timezone` feature)
//! - `regex` crate for user-provided patterns (`regex` feature)
//! - `serde_json` for JSON validation (`serde_json` feature)
//! - `std::net` for IP address validation (always available)

// ── Regex (feature-gated) ──

#[cfg(feature = "regex")]
use regex::Regex;

/// Validate regex pattern match. Requires the `regex` feature.
#[cfg(feature = "regex")]
pub fn matches_regex(value: &str, pattern: &str) -> bool {
    Regex::new(pattern).map(|re| re.is_match(value)).unwrap_or(false)
}

/// Validate value does NOT match regex. Requires the `regex` feature.
#[cfg(feature = "regex")]
pub fn not_matches_regex(value: &str, pattern: &str) -> bool {
    !matches_regex(value, pattern)
}

// ── Email (feature-gated) ──

/// Validate email address per RFC 5321.
///
/// Uses the `email_address` crate for standards-compliant parsing.
/// Requires the `email` feature (enabled by default).
#[cfg(feature = "email")]
pub fn is_email(value: &str) -> bool {
    email_address::EmailAddress::is_valid(value)
}

// ── URL (feature-gated) ──

/// Validate URL per the WHATWG URL Standard.
///
/// Uses the `url` crate for standards-compliant parsing. Requires `http` or `https` scheme.
/// Requires the `url-parse` feature (enabled by default).
#[cfg(feature = "url-parse")]
pub fn is_url(value: &str) -> bool {
    match url::Url::parse(value) {
        Ok(u) => u.scheme() == "http" || u.scheme() == "https",
        Err(_) => false,
    }
}

// ── UUID (feature-gated) ──

/// Validate UUID per RFC 4122.
///
/// Uses the `uuid` crate for correct parsing. Accepts any valid UUID version.
/// Requires the `uuid-parse` feature (enabled by default).
#[cfg(feature = "uuid-parse")]
pub fn is_uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok()
}

// ── ULID (feature-gated) ──

/// Validate ULID per the [ULID spec](https://github.com/ulid/spec).
///
/// Uses the `ulid` crate for correct parsing.
/// Requires the `ulid-parse` feature (enable via `full` feature).
#[cfg(feature = "ulid-parse")]
pub fn is_ulid(value: &str) -> bool {
    value.parse::<ulid::Ulid>().is_ok()
}

// ── Date / DateTime (feature-gated) ──

/// Validate ISO 8601 date (`YYYY-MM-DD`) using the `chrono` crate.
///
/// Strict: rejects invalid dates like `2023-02-30` and non-leap-year Feb 29.
/// Requires the `chrono` feature (enabled by default).
#[cfg(feature = "chrono")]
pub fn is_iso_date(value: &str) -> bool {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}

/// Validate ISO 8601 datetime using the `chrono` crate.
///
/// Accepts `YYYY-MM-DDTHH:MM:SS`, with optional fractional seconds and timezone.
/// Requires the `chrono` feature (enabled by default).
#[cfg(feature = "chrono")]
pub fn is_iso_datetime(value: &str) -> bool {
    // Try full datetime with timezone (RFC 3339, a profile of ISO 8601)
    if chrono::DateTime::parse_from_rfc3339(value).is_ok() {
        return true;
    }
    // Try naive datetime (no timezone)
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").is_ok()
        || chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f").is_ok()
}

// ── Date comparison helpers (feature-gated) ──

/// Compare two ISO 8601 date strings using `chrono`.
/// Requires the `chrono` feature.
#[cfg(feature = "chrono")]
fn compare_dates(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let a_str = a.split('T').next().unwrap_or(a);
    let b_str = b.split('T').next().unwrap_or(b);
    let a_date = chrono::NaiveDate::parse_from_str(a_str, "%Y-%m-%d").ok()?;
    let b_date = chrono::NaiveDate::parse_from_str(b_str, "%Y-%m-%d").ok()?;
    Some(a_date.cmp(&b_date))
}

/// Date is before another date (ISO 8601). Requires the `chrono` feature.
#[cfg(feature = "chrono")]
pub fn is_before(value: &str, other: &str) -> bool {
    compare_dates(value, other) == Some(std::cmp::Ordering::Less)
}

/// Date is after another date (ISO 8601). Requires the `chrono` feature.
#[cfg(feature = "chrono")]
pub fn is_after(value: &str, other: &str) -> bool {
    compare_dates(value, other) == Some(std::cmp::Ordering::Greater)
}

/// Date is before or equal to another date (ISO 8601). Requires the `chrono` feature.
#[cfg(feature = "chrono")]
pub fn is_before_or_equal(value: &str, other: &str) -> bool {
    matches!(compare_dates(value, other), Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal))
}

/// Date is after or equal to another date (ISO 8601). Requires the `chrono` feature.
#[cfg(feature = "chrono")]
pub fn is_after_or_equal(value: &str, other: &str) -> bool {
    matches!(compare_dates(value, other), Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal))
}

/// Date equals another date (ISO 8601). Requires the `chrono` feature.
#[cfg(feature = "chrono")]
pub fn is_date_equals(value: &str, other: &str) -> bool {
    compare_dates(value, other) == Some(std::cmp::Ordering::Equal)
}

// ── Standalone format rules (stdlib only, always available) ──

/// Validate IP address (v4 or v6) using `std::net::IpAddr`.
pub fn is_ip(value: &str) -> bool {
    value.parse::<std::net::IpAddr>().is_ok()
}

/// Validate IPv4 address using `std::net::Ipv4Addr`.
pub fn is_ipv4(value: &str) -> bool {
    value.parse::<std::net::Ipv4Addr>().is_ok()
}

/// Validate IPv6 address using `std::net::Ipv6Addr`.
pub fn is_ipv6(value: &str) -> bool {
    value.parse::<std::net::Ipv6Addr>().is_ok()
}

/// Validate MAC address (IEEE 802): `XX:XX:XX:XX:XX:XX` or `XX-XX-XX-XX-XX-XX`.
///
/// This is a simple format check — MAC addresses are just 6 hex octets separated
/// by `:` or `-`, no ambiguity to get wrong.
pub fn is_mac_address(value: &str) -> bool {
    let parts: Vec<&str> = if value.contains(':') {
        value.split(':').collect()
    } else if value.contains('-') {
        value.split('-').collect()
    } else {
        return false;
    };
    parts.len() == 6 && parts.iter().all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Validate JSON string using `serde_json`. Requires the `serde_json` feature.
#[cfg(feature = "serde_json")]
pub fn is_json(value: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(value).is_ok()
}

/// Validate that a string contains only ASCII characters.
///
/// Uses `str::is_ascii()` from stdlib — no ambiguity here.
pub fn is_ascii(value: &str) -> bool {
    value.is_ascii()
}

/// Valid hex color: `#RGB`, `#RRGGBB`, or `#RRGGBBAA`.
///
/// Simple format: `#` followed by exactly 3, 6, or 8 hex digits. No ambiguity.
pub fn is_hex_color(value: &str) -> bool {
    if !value.starts_with('#') {
        return false;
    }
    let hex = &value[1..];
    matches!(hex.len(), 3 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// Validate timezone against the IANA timezone database.
///
/// Uses the `chrono-tz` crate which embeds the full IANA tz database.
/// Requires the `timezone` feature (enabled by default).
#[cfg(feature = "timezone")]
pub fn is_timezone(value: &str) -> bool {
    value.parse::<chrono_tz::Tz>().is_ok()
}
