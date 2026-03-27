use validation::Validate;
use validation::traits::Validate as ValidateTrait;

// ── Tuple struct (newtype) ──

#[derive(Validate)]
struct Email(#[validate(required, email)] Option<String>);

#[test]
fn test_tuple_struct_valid() {
    let e = Email(Some("user@example.com".into()));
    assert!(e.validate().is_ok());
}

#[test]
fn test_tuple_struct_invalid_email() {
    let e = Email(Some("not-email".into()));
    assert!(e.validate().is_err());
}

#[test]
fn test_tuple_struct_required() {
    let e = Email(None);
    let err = e.validate().unwrap_err();
    assert!(err.messages().contains_key("0"));
}

#[derive(Validate)]
struct Score(#[validate(min = 0, max = 100)] i32);

#[test]
fn test_tuple_struct_numeric_valid() {
    let s = Score(50);
    assert!(s.validate().is_ok());
}

#[test]
fn test_tuple_struct_numeric_invalid() {
    let s = Score(101);
    assert!(s.validate().is_err());
}

// ── Enum with named fields ──

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

#[test]
fn test_enum_email_valid() {
    let c = ContactMethod::Email {
        address: Some("user@example.com".into()),
    };
    assert!(c.validate().is_ok());
}

#[test]
fn test_enum_email_invalid() {
    let c = ContactMethod::Email {
        address: Some("bad".into()),
    };
    assert!(c.validate().is_err());
}

#[test]
fn test_enum_email_required() {
    let c = ContactMethod::Email { address: None };
    let err = c.validate().unwrap_err();
    assert!(err.messages().contains_key("address"));
}

#[test]
fn test_enum_phone_valid() {
    let c = ContactMethod::Phone {
        number: Some("12345".into()),
    };
    assert!(c.validate().is_ok());
}

#[test]
fn test_enum_phone_too_short() {
    let c = ContactMethod::Phone {
        number: Some("123".into()),
    };
    assert!(c.validate().is_err());
}

#[test]
fn test_enum_unit_variant_always_valid() {
    let c = ContactMethod::None;
    assert!(c.validate().is_ok());
}

// ── Enum with tuple variants ──

#[derive(Validate)]
enum Wrapper {
    Text(#[validate(required, min = 1)] Option<String>),
    Number(#[validate(min = 0, max = 999)] i32),
    Empty,
}

#[test]
fn test_enum_tuple_variant_valid() {
    let w = Wrapper::Text(Some("hello".into()));
    assert!(w.validate().is_ok());
}

#[test]
fn test_enum_tuple_variant_required() {
    let w = Wrapper::Text(None);
    assert!(w.validate().is_err());
}

#[test]
fn test_enum_tuple_number_valid() {
    let w = Wrapper::Number(42);
    assert!(w.validate().is_ok());
}

#[test]
fn test_enum_tuple_number_invalid() {
    let w = Wrapper::Number(1000);
    assert!(w.validate().is_err());
}

#[test]
fn test_enum_empty_variant() {
    let w = Wrapper::Empty;
    assert!(w.validate().is_ok());
}

// ── Nested: tuple struct used in another struct ──

#[derive(Validate)]
struct EmailAddress(#[validate(email)] String);

#[derive(Validate)]
struct UserProfile {
    #[validate(required)]
    name: Option<String>,
    #[validate(nested)]
    email: EmailAddress,
}

#[test]
fn test_nested_tuple_struct_valid() {
    let u = UserProfile {
        name: Some("John".into()),
        email: EmailAddress("user@example.com".into()),
    };
    assert!(u.validate().is_ok());
}

#[test]
fn test_nested_tuple_struct_invalid() {
    let u = UserProfile {
        name: Some("John".into()),
        email: EmailAddress("bad".into()),
    };
    let err = u.validate().unwrap_err();
    assert!(err.messages().contains_key("email.0"));
}

// ── Enum variant validation with strum ──

#[derive(strum::AsRefStr, Debug)]
enum UserStatus {
    Active,
    Inactive,
    Banned,
    Suspended,
}

#[derive(Validate)]
struct CreateUser2 {
    #[validate(in_list("Active", "Inactive"))]
    status: UserStatus,
}

#[derive(Validate)]
struct AdminUpdate {
    #[validate(not_in("Banned"))]
    status: UserStatus,
}

#[test]
fn test_enum_in_list_via_strum_valid() {
    let u = CreateUser2 { status: UserStatus::Active };
    assert!(u.validate().is_ok());
}

#[test]
fn test_enum_in_list_via_strum_invalid() {
    let u = CreateUser2 { status: UserStatus::Banned };
    assert!(u.validate().is_err());
}

#[test]
fn test_enum_not_in_via_strum_valid() {
    let a = AdminUpdate { status: UserStatus::Active };
    assert!(a.validate().is_ok());
}

#[test]
fn test_enum_not_in_via_strum_invalid() {
    let a = AdminUpdate { status: UserStatus::Banned };
    assert!(a.validate().is_err());
}
