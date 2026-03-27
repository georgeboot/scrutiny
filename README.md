# validation

A powerful, Laravel-inspired validation library for Rust. Brings Laravel's validation DX to the Rust ecosystem using derive macros and the type system — no runtime string parsing.

**Correct by default.** Format rules delegate to dedicated, standards-compliant parsing crates — not hand-rolled regexes. Email is validated per RFC 5321, URLs per the WHATWG URL Standard, UUIDs per RFC 4122, dates per ISO 8601, and IP addresses via Rust's stdlib. Where a standard exists, we follow it.

## Why?

The existing Rust validation crates (`validator`, `garde`) are limited: few rules, no conditional validation, no bail, no per-rule custom messages, no framework-aware error responses. They also tend to use simplistic regexes for format validation rather than proper parsers, leading to false positives and negatives.

This library provides 50+ validation rules, conditional logic, nested validation, first-class axum integration, and standards-compliant format validation out of the box.

### Standards used

| Rule | Standard | Crate |
|------|----------|-------|
| `email` | RFC 5321 | [`email_address`](https://crates.io/crates/email_address) |
| `url` | WHATWG URL | [`url`](https://crates.io/crates/url) |
| `uuid` | RFC 4122 | [`uuid`](https://crates.io/crates/uuid) |
| `ulid` | [ULID spec](https://github.com/ulid/spec) | [`ulid`](https://crates.io/crates/ulid) |
| `date` / `datetime` | ISO 8601 | [`chrono`](https://crates.io/crates/chrono) |
| `timezone` | IANA tz database | [`chrono-tz`](https://crates.io/crates/chrono-tz) |
| `ip` / `ipv4` / `ipv6` | RFC 791 / 2460 | `std::net` |
| `mac_address` | IEEE 802 | trivial format check |

Each is behind a feature flag (all on by default). Disable default features for a minimal build and opt in to what you need.

## Getting Started

Add to your `Cargo.toml`:

```toml
[dependencies]
validation = { path = "validation" }

# For axum integration:
validation-axum = { path = "validation-axum" }
```

### Basic Usage

```rust
use validation::Validate;
use validation::traits::Validate as _;

#[derive(Validate)]
struct CreateUser {
    #[validate(required, email, bail)]
    email: Option<String>,

    #[validate(required, min = 2, max = 255)]
    name: Option<String>,

    #[validate(required, min = 8, confirmed)]
    password: Option<String>,

    password_confirmation: Option<String>,

    #[validate(nullable, url)]
    website: Option<String>,

    #[validate(required, in_list("user", "admin", "editor"))]
    role: Option<String>,
}

let user = CreateUser {
    email: Some("test@example.com".into()),
    name: Some("Jane Doe".into()),
    password: Some("secretpassword".into()),
    password_confirmation: Some("secretpassword".into()),
    website: None,
    role: Some("admin".into()),
};

assert!(user.validate().is_ok());
```

### Custom Error Messages

Every rule has a sensible default message with field name interpolation. Override any rule's message inline:

```rust
#[derive(Validate)]
#[validate(attributes(name = "full name"))]
struct Profile {
    #[validate(
        required(message = "We need your name!"),
        min(value = 2, message = "Name must be at least :min characters."),
    )]
    name: Option<String>,

    #[validate(
        required,
        email(message = "That doesn't look like a valid email."),
    )]
    email: Option<String>,
}
```

Default messages use `:attribute` (friendly field name), `:min`, `:max`, etc. The `attributes()` macro maps field names to display names.

### Type-Aware Rules

`min`, `max`, `between`, and `size` automatically detect the field type and do the right thing:

```rust
#[derive(Validate)]
struct Query {
    #[validate(min = 1, max = 10000)]   // numeric: compares value
    per_page: f64,

    #[validate(min = 2, max = 255)]     // string: compares length
    search: String,

    #[validate(min = 1, max = 10)]      // vec: compares item count
    tags: Vec<String>,

    #[validate(size = 4)]               // vec: exactly 4 items
    bounding_box: Vec<f64>,

    #[validate(between(min = 0, max = 100))]  // numeric: value in range
    score: i32,
}
```

### Tuple Structs

Newtypes get validation for free — encode your invariants in the type system:

```rust
#[derive(Validate)]
struct Email(#[validate(email)] String);

#[derive(Validate)]
struct Score(#[validate(min = 0, max = 100)] i32);
```

Use them in other structs with `#[validate(nested)]`:

```rust
#[derive(Validate)]
struct UserProfile {
    #[validate(required)]
    name: Option<String>,
    #[validate(nested)]
    email: Email,
}
```

### Enums

Validate fields per variant. Unit variants always pass.

```rust
#[derive(Validate)]
enum ContactMethod {
    Email {
        #[validate(required, email)]
        address: Option<String>,
    },
    Phone {
        #[validate(required, min = 5)]
        number: Option<String>,
    },
    None,
}
```

Tuple variants work too:

```rust
#[derive(Validate)]
enum Wrapper {
    Text(#[validate(required, min = 1)] Option<String>),
    Number(#[validate(min = 0, max = 999)] i32),
    Empty,
}
```

### Conditional Validation

```rust
#[derive(Validate)]
struct Registration {
    #[validate(required, in_list("user", "admin"))]
    role: Option<String>,

    // Only required when role is "admin"
    #[validate(required_if(field = "role", value = "admin", message = "Admins need a code."))]
    admin_code: Option<String>,

    // Prohibited for basic users
    #[validate(prohibited_if(field = "role", value = "user"))]
    admin_feature: Option<String>,
}
```

### Nested & Array Validation

Use `nested` to recursively validate nested structs and `Vec` elements. Errors use dot-notation paths.

```rust
#[derive(Validate)]
struct Address {
    #[validate(required, max = 255)]
    line1: Option<String>,
    #[validate(required)]
    city: Option<String>,
    #[validate(required, regex(pattern = r"^\d{5}(-\d{4})?$", message = "Invalid ZIP."))]
    zip: Option<String>,
}

#[derive(Validate)]
struct Team {
    #[validate(required)]
    name: Option<String>,
    #[validate(nested)]
    members: Vec<Member>,
    #[validate(nested)]
    address: Address,
}

// Errors: "address.city", "members.0.email", "members.2.name"
```

### Axum Integration

Drop-in replacement for `axum::Json<T>` that validates before your handler runs:

```rust
use validation_axum::Valid;

async fn create_user(Valid(user): Valid<CreateUser>) -> impl IntoResponse {
    // `user` is already validated.
    // Invalid requests get a 422 JSON response automatically.
}
```

**Custom error responses** via trait:

```rust
use validation_axum::{ValidWith, ValidationErrorResponse};

struct MyApiError;

impl ValidationErrorResponse for MyApiError {
    fn from_validation_errors(errors: ValidationErrors) -> Response {
        let body = json!({
            "success": false,
            "code": "VALIDATION_FAILED",
            "details": errors.messages(),
        });
        (StatusCode::BAD_REQUEST, Json(body)).into_response()
    }

    fn from_deserialization_error(error: String) -> Response {
        // ...
    }
}

async fn handler(result: ValidWith<CreateUser, MyApiError>) -> impl IntoResponse {
    let user = result.into_inner();
    // ...
}
```

Also available: `ValidForm<T>` and `ValidQuery<T>` for form-encoded and query parameter validation.

## Available Rules (50+)

### Presence & Meta
| Rule | Attribute | Description |
|------|-----------|-------------|
| required | `required` | Field must be present and non-empty |
| filled | `filled` | If present, must not be empty |
| nullable | `nullable` | Skip rules if None |
| sometimes | `sometimes` | Skip rules if field absent |
| bail | `bail` | Stop on first error for this field |
| prohibited | `prohibited` | Field must NOT be present |
| prohibited_if | `prohibited_if(field, value)` | Prohibited when condition met |
| prohibited_unless | `prohibited_unless(field, value)` | Prohibited unless condition met |

### Type & Format
| Rule | Attribute | Description |
|------|-----------|-------------|
| string | `string` | Must be a string (compile-time assertion) |
| integer | `integer` | Must be a valid integer |
| numeric | `numeric` | Must be a valid number |
| boolean | `boolean` | Must be true/false/1/0 |
| email | `email` | Valid email (HTML5 spec) |
| url | `url` | Valid URL |
| uuid | `uuid` | Valid UUID (8-4-4-4-12 hex) |
| ulid | `ulid` | Valid ULID (26 char Crockford base32) |
| ip | `ip` | Valid IP address |
| ipv4 | `ipv4` | Valid IPv4 address |
| ipv6 | `ipv6` | Valid IPv6 address |
| mac_address | `mac_address` | Valid MAC address |
| json | `json` | Valid JSON string |
| ascii | `ascii` | Only ASCII characters |
| hex_color | `hex_color` | Valid hex color (#RGB, #RRGGBB, #RRGGBBAA) |
| timezone | `timezone` | Valid timezone (IANA format) |

### String
| Rule | Attribute | Description |
|------|-----------|-------------|
| alpha | `alpha` | Only alphabetic characters |
| alpha_num | `alpha_num` | Only alphanumeric |
| alpha_dash | `alpha_dash` | Alphanumeric + dashes + underscores |
| uppercase | `uppercase` | Must be entirely uppercase |
| lowercase | `lowercase` | Must be entirely lowercase |
| starts_with | `starts_with = "X"` | Must start with prefix |
| ends_with | `ends_with = "X"` | Must end with suffix |
| doesnt_start_with | `doesnt_start_with = "X"` | Must NOT start with prefix |
| doesnt_end_with | `doesnt_end_with = "X"` | Must NOT end with suffix |
| contains | `contains = "X"` | Must contain substring |
| doesnt_contain | `doesnt_contain = "X"` | Must NOT contain substring |
| regex | `regex = "pattern"` | Must match regex |
| not_regex | `not_regex = "pattern"` | Must NOT match regex |

### Size & Length
| Rule | Attribute | Description |
|------|-----------|-------------|
| min | `min = N` | Type-aware: numeric value, string length, or Vec item count |
| max | `max = N` | Type-aware: numeric value, string length, or Vec item count |
| between | `between(min, max)` | Type-aware: value/length/count between min and max |
| size | `size = N` | Type-aware: exact value, length, or count |
| digits | `digits = N` | Exact digit count |
| digits_between | `digits_between(min, max)` | Digit count between min and max |
| decimal | `decimal = N` or `decimal(min, max)` | Exact or range of decimal places |
| multiple_of | `multiple_of = "N"` | Must be a multiple of N |

### Comparison
| Rule | Attribute | Description |
|------|-----------|-------------|
| same | `same = "field"` | Must equal another field |
| different | `different = "field"` | Must differ from another field |
| confirmed | `confirmed` | Must match `{field}_confirmation` |
| gt | `gt = "field"` | Greater than another field |
| gte | `gte = "field"` | Greater than or equal |
| lt | `lt = "field"` | Less than another field |
| lte | `lte = "field"` | Less than or equal |
| in_list | `in_list("a", "b", "c")` | Must be one of the values |
| not_in | `not_in("a", "b")` | Must NOT be one of the values |
| in_array | `in_array = "field"` | Must exist in another field's array |
| distinct | `distinct` | Array items must be unique |

### Conditional
| Rule | Attribute | Description |
|------|-----------|-------------|
| required_if | `required_if(field, value)` | Required when field equals value |
| required_unless | `required_unless(field, value)` | Required unless field equals value |
| required_with | `required_with = "field"` | Required when field is present |
| required_without | `required_without = "field"` | Required when field is absent |
| required_with_all | `required_with_all("a", "b")` | Required when ALL fields present |
| required_without_all | `required_without_all("a", "b")` | Required when ALL fields absent |
| accepted | `accepted` | Must be yes/on/1/true |
| accepted_if | `accepted_if(field, value)` | Must be accepted when condition met |
| declined | `declined` | Must be no/off/0/false |
| declined_if | `declined_if(field, value)` | Must be declined when condition met |

### Date (ISO 8601 strict)
| Rule | Attribute | Description |
|------|-----------|-------------|
| date | `date` | Valid ISO 8601 date (YYYY-MM-DD) |
| datetime | `datetime` | Valid ISO 8601 datetime |
| date_equals | `date_equals = "YYYY-MM-DD"` | Must equal the date |
| before | `before = "YYYY-MM-DD"` | Must be before the date |
| after | `after = "YYYY-MM-DD"` | Must be after the date |
| before_or_equal | `before_or_equal = "YYYY-MM-DD"` | Before or equal |
| after_or_equal | `after_or_equal = "YYYY-MM-DD"` | After or equal |

### Structural
| Rule | Attribute | Description |
|------|-----------|-------------|
| nested | `nested` | Recursively validate nested struct/Vec (alias: `dive`) |
| custom | `custom = fn_name` | Custom validation function |

## Architecture

```
validation/          Core: traits, errors, rule functions
validation-derive/   Proc macro: #[derive(Validate)]
validation-axum/     Axum extractors + error response customization
```

The core is framework-agnostic. `validation-axum` adds axum extractors behind a separate crate. The error system uses `ValidationErrors` with dot-notation field paths and is serde-serializable.

## License

MIT
