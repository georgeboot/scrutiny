//! # Validation
//!
//! A powerful, Laravel-inspired validation library for Rust. Uses derive macros and
//! field-level attributes to declaratively validate structs — no runtime string parsing.
//!
//! ## Correct by default
//!
//! Format rules delegate to dedicated, standards-compliant crates rather than
//! hand-rolled regexes:
//!
//! | Rule | Standard | Crate | Feature |
//! |------|----------|-------|---------|
//! | `email` | RFC 5321 | `email_address` | `email` |
//! | `url` | WHATWG URL | `url` | `url-parse` |
//! | `uuid` | RFC 4122 | `uuid` | `uuid-parse` |
//! | `ulid` | ULID spec | `ulid` | `ulid-parse` |
//! | `date` / `datetime` | ISO 8601 | `chrono` | `chrono` |
//! | `timezone` | IANA tz database | `chrono-tz` | `timezone` |
//! | `ip` / `ipv4` / `ipv6` | RFC 791 / 2460 | `std::net` | — |
//!
//! All enabled by default. Disable default features for a minimal build and
//! opt in to what you need.
//!
//! ## Quick Start
//!
//! ```rust
//! use validation::Validate;
//! use validation::traits::Validate as _;
//!
//! #[derive(Validate)]
//! struct CreateUser {
//!     #[validate(required, email, bail)]
//!     email: Option<String>,
//!
//!     #[validate(required, min = 2, max = 255)]
//!     name: Option<String>,
//!
//!     #[validate(required, min = 8, confirmed)]
//!     password: Option<String>,
//!
//!     password_confirmation: Option<String>,
//!
//!     #[validate(nullable, url)]
//!     website: Option<String>,
//! }
//!
//! let user = CreateUser {
//!     email: Some("test@example.com".into()),
//!     name: Some("Jane".into()),
//!     password: Some("secret123".into()),
//!     password_confirmation: Some("secret123".into()),
//!     website: None,
//! };
//! assert!(user.validate().is_ok());
//! ```
//!
//! ## Custom Error Messages
//!
//! Every rule has a sensible default message. Override per-rule with `message`:
//!
//! ```rust
//! use validation::Validate;
//! use validation::traits::Validate as _;
//!
//! #[derive(Validate)]
//! #[validate(attributes(name = "full name"))]
//! struct Profile {
//!     #[validate(required(message = "We need your name!"), min = 2)]
//!     name: Option<String>,
//!
//!     #[validate(required, email(message = "That doesn't look right."))]
//!     email: Option<String>,
//! }
//!
//! let p = Profile { name: None, email: Some("bad".into()) };
//! let err = p.validate().unwrap_err();
//! assert_eq!(err.messages()["name"][0], "We need your name!");
//! assert_eq!(err.messages()["email"][0], "That doesn't look right.");
//! ```
//!
//! ## Nested & Vec Validation
//!
//! Use `nested` to recursively validate nested structs and Vec elements.
//! Errors use dot-notation paths: `address.city`, `members.0.email`.
//!
//! ```rust
//! use validation::Validate;
//! use validation::traits::Validate as _;
//!
//! #[derive(Validate)]
//! struct Address {
//!     #[validate(required)]
//!     city: Option<String>,
//! }
//!
//! #[derive(Validate)]
//! struct Order {
//!     #[validate(nested)]
//!     address: Address,
//! }
//!
//! let order = Order { address: Address { city: None } };
//! let err = order.validate().unwrap_err();
//! assert!(err.messages().contains_key("address.city"));
//! ```
//!
//! ## Conditional Validation
//!
//! ```rust
//! use validation::Validate;
//! use validation::traits::Validate as _;
//!
//! #[derive(Validate)]
//! struct Registration {
//!     #[validate(required, in_list("user", "admin"))]
//!     role: Option<String>,
//!
//!     #[validate(required_if(field = "role", value = "admin"))]
//!     admin_code: Option<String>,
//! }
//!
//! // admin_code only required when role is "admin"
//! let reg = Registration { role: Some("user".into()), admin_code: None };
//! assert!(reg.validate().is_ok());
//! ```
//!
//! ## Available Rules
//!
//! ### Presence & Meta
//! `required`, `filled`, `nullable`, `sometimes`, `bail`, `prohibited`,
//! `prohibited_if`, `prohibited_unless`
//!
//! ### Type & Format
//! `string`, `integer`, `numeric`, `boolean`, `email`, `url`, `uuid`, `ulid`,
//! `ip`, `ipv4`, `ipv6`, `mac_address`, `json`, `ascii`, `hex_color`, `timezone`
//!
//! ### String
//! `alpha`, `alpha_num`, `alpha_dash`, `uppercase`, `lowercase`,
//! `starts_with`, `ends_with`, `doesnt_start_with`, `doesnt_end_with`,
//! `contains`, `doesnt_contain`, `regex`, `not_regex`
//!
//! ### Size & Length
//! `min`, `max`, `between`, `size`, `digits`, `digits_between`, `decimal`,
//! `multiple_of`
//!
//! ### Comparison
//! `same`, `different`, `confirmed`, `gt`, `gte`, `lt`, `lte`,
//! `in_list`, `not_in`, `in_array`, `distinct`
//!
//! ### Conditional
//! `required_if`, `required_unless`, `required_with`, `required_without`,
//! `required_with_all`, `required_without_all`, `accepted`, `accepted_if`,
//! `declined`, `declined_if`
//!
//! ### Date (ISO 8601 strict)
//! `date`, `datetime`, `date_equals`, `before`, `after`,
//! `before_or_equal`, `after_or_equal`
//!
//! ### Structural
//! `nested` (alias: `dive`), `custom`
//!
//! ## Error Serialization
//!
//! With the `serde` feature (default), `ValidationErrors` serializes to:
//!
//! ```json
//! {
//!   "email": ["The email field is required."],
//!   "name": ["The name field must be at least 2 characters."]
//! }
//! ```

pub use validation_derive::Validate;

pub mod error;
pub mod rules;
pub mod traits;
pub mod value;
