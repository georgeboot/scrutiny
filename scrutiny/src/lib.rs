//! # Scrutiny
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
//! use scrutiny::Validate;
//! use scrutiny::traits::Validate as _;
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
//! use scrutiny::Validate;
//! use scrutiny::traits::Validate as _;
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
//! use scrutiny::Validate;
//! use scrutiny::traits::Validate as _;
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
//! ## Tuple Structs
//!
//! Newtypes get validation for free — encode your invariants in the type system:
//!
//! ```rust
//! use scrutiny::Validate;
//! use scrutiny::traits::Validate as _;
//!
//! #[derive(Validate)]
//! struct Email(#[validate(email)] String);
//!
//! #[derive(Validate)]
//! struct Score(#[validate(min = 0, max = 100)] i32);
//!
//! assert!(Email("user@example.com".into()).validate().is_ok());
//! assert!(Score(101).validate().is_err());
//! ```
//!
//! ## Enums
//!
//! Validate fields per variant. Unit variants always pass.
//!
//! ```rust
//! use scrutiny::Validate;
//! use scrutiny::traits::Validate as _;
//!
//! #[derive(Validate)]
//! enum ContactMethod {
//!     Email {
//!         #[validate(required, email)]
//!         address: Option<String>,
//!     },
//!     Phone {
//!         #[validate(required, min = 5)]
//!         number: Option<String>,
//!     },
//!     None,
//! }
//!
//! let c = ContactMethod::Email { address: Some("bad".into()) };
//! assert!(c.validate().is_err());
//!
//! let c = ContactMethod::None;
//! assert!(c.validate().is_ok());
//! ```
//!
//! ## Restricting Enum Variants
//!
//! Use `in_list`/`not_in` with [strum](https://crates.io/crates/strum)'s `AsRefStr`
//! to restrict which variants are accepted:
//!
//! ```rust,ignore
//! #[derive(Deserialize, strum::AsRefStr)]
//! enum UserStatus { Active, Inactive, Banned }
//!
//! #[derive(Validate, Deserialize)]
//! struct CreateUser {
//!     #[validate(in_list("Active", "Inactive"))]
//!     status: UserStatus,
//! }
//! ```
//!
//! Works on any type implementing `AsRef<str>`.
//!
//! ## Conditional Validation
//!
//! ```rust
//! use scrutiny::Validate;
//! use scrutiny::traits::Validate as _;
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
//! ### Size, Length & Range
//! `min`, `max`, `between`, `size` — **type-aware**: compares numeric values
//! for number fields, string length for `String`, and item count for `Vec`.
//!
//! `digits`, `digits_between`, `decimal`, `multiple_of`
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
//! ## Typed Fields & Deserialization Errors
//!
//! Use actual types like `uuid::Uuid` or `chrono::NaiveDate` instead of validating
//! strings manually. Deserialization errors become field-level validation errors
//! automatically.
//!
//! **Axum users** — `Valid<T>` handles this out of the box. Just use typed fields.
//!
//! **Everyone else** — use [`deserialize::from_json`] to get unified errors:
//!
//! ```rust,ignore
//! use scrutiny::deserialize::from_json;
//!
//! // id: uuid::Uuid — if "not-a-uuid" is sent, you get:
//! // {"id": ["invalid type: expected UUID"]}
//! match from_json::<CreateUser>(body_bytes) {
//!     Ok(user) => { /* deserialized AND validated */ }
//!     Err(errors) => { /* same ValidationErrors type for both */ }
//! }
//! ```
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

pub use scrutiny_derive::Validate;

#[cfg(all(feature = "serde_json", feature = "serde_path_to_error"))]
pub mod deserialize;
pub mod error;
pub mod rules;
pub mod traits;
pub mod value;
