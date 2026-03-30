use scrutiny::Validate;
use scrutiny::traits::Validate as ValidateTrait;

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

#[test]
fn test_valid_user() {
    let user = CreateUser {
        email: Some("test@example.com".to_string()),
        name: Some("John Doe".to_string()),
        password: Some("secretpassword".to_string()),
        password_confirmation: Some("secretpassword".to_string()),
        website: None,
        role: Some("admin".to_string()),
    };
    assert!(user.validate().is_ok());
}

#[test]
fn test_required_fields() {
    let user = CreateUser {
        email: None,
        name: None,
        password: None,
        password_confirmation: None,
        website: None,
        role: None,
    };
    let err = user.validate().unwrap_err();
    let msgs = err.messages();

    assert!(msgs.contains_key("email"));
    assert!(msgs.contains_key("name"));
    assert!(msgs.contains_key("password"));
    assert!(msgs.contains_key("role"));
    // website is nullable, so no error
    assert!(!msgs.contains_key("website"));
}

#[test]
fn test_email_validation() {
    let user = CreateUser {
        email: Some("not-an-email".to_string()),
        name: Some("John".to_string()),
        password: Some("secretpassword".to_string()),
        password_confirmation: Some("secretpassword".to_string()),
        website: None,
        role: Some("user".to_string()),
    };
    let err = user.validate().unwrap_err();
    let msgs = err.messages();
    assert!(msgs.contains_key("email"));
    assert!(msgs["email"][0].contains("email"));
}

#[test]
fn test_bail_stops_at_first_error() {
    // email has bail — if required fails, email check shouldn't fire
    let user = CreateUser {
        email: None, // required fails first, bail should stop here
        name: Some("John".to_string()),
        password: Some("secretpassword".to_string()),
        password_confirmation: Some("secretpassword".to_string()),
        website: None,
        role: Some("user".to_string()),
    };
    let err = user.validate().unwrap_err();
    let email_errs = &err.messages()["email"];
    assert_eq!(email_errs.len(), 1, "bail should stop at first error");
    assert!(email_errs[0].contains("required"));
}

#[test]
fn test_min_max_length() {
    let user = CreateUser {
        email: Some("test@example.com".to_string()),
        name: Some("A".to_string()), // too short (min 2)
        password: Some("short".to_string()), // too short (min 8)
        password_confirmation: Some("short".to_string()),
        website: None,
        role: Some("user".to_string()),
    };
    let err = user.validate().unwrap_err();
    let msgs = err.messages();
    assert!(msgs.contains_key("name"));
    assert!(msgs.contains_key("password"));
}

#[test]
fn test_confirmed() {
    let user = CreateUser {
        email: Some("test@example.com".to_string()),
        name: Some("John".to_string()),
        password: Some("secretpassword".to_string()),
        password_confirmation: Some("different".to_string()), // doesn't match
        website: None,
        role: Some("user".to_string()),
    };
    let err = user.validate().unwrap_err();
    let msgs = err.messages();
    assert!(msgs.contains_key("password"));
    assert!(msgs["password"].iter().any(|m| m.contains("confirmation")));
}

#[test]
fn test_in_list() {
    let user = CreateUser {
        email: Some("test@example.com".to_string()),
        name: Some("John".to_string()),
        password: Some("secretpassword".to_string()),
        password_confirmation: Some("secretpassword".to_string()),
        website: None,
        role: Some("superadmin".to_string()), // not in list
    };
    let err = user.validate().unwrap_err();
    assert!(err.messages().contains_key("role"));
}

#[test]
fn test_nullable_allows_none() {
    let user = CreateUser {
        email: Some("test@example.com".to_string()),
        name: Some("John".to_string()),
        password: Some("secretpassword".to_string()),
        password_confirmation: Some("secretpassword".to_string()),
        website: None, // nullable, should be fine
        role: Some("user".to_string()),
    };
    assert!(user.validate().is_ok());
}

#[test]
fn test_nullable_validates_when_present() {
    let user = CreateUser {
        email: Some("test@example.com".to_string()),
        name: Some("John".to_string()),
        password: Some("secretpassword".to_string()),
        password_confirmation: Some("secretpassword".to_string()),
        website: Some("not-a-url".to_string()), // nullable but present and invalid
        role: Some("user".to_string()),
    };
    let err = user.validate().unwrap_err();
    assert!(err.messages().contains_key("website"));
}

#[test]
fn test_url_validation() {
    let user = CreateUser {
        email: Some("test@example.com".to_string()),
        name: Some("John".to_string()),
        password: Some("secretpassword".to_string()),
        password_confirmation: Some("secretpassword".to_string()),
        website: Some("https://example.com".to_string()),
        role: Some("user".to_string()),
    };
    assert!(user.validate().is_ok());
}

// --- Nested validation ---

#[derive(Validate)]
struct Address {
    #[validate(required, max = 255)]
    line1: Option<String>,
    #[validate(required)]
    city: Option<String>,
}

#[derive(Validate)]
struct Order {
    #[validate(required)]
    customer_name: Option<String>,
    #[validate(nested)]
    address: Address,
}

#[test]
fn test_nested_validation() {
    let order = Order {
        customer_name: Some("John".to_string()),
        address: Address {
            line1: None,
            city: None,
        },
    };
    let err = order.validate().unwrap_err();
    let msgs = err.messages();
    assert!(msgs.contains_key("address.line1"));
    assert!(msgs.contains_key("address.city"));
}

#[test]
fn test_nested_valid() {
    let order = Order {
        customer_name: Some("John".to_string()),
        address: Address {
            line1: Some("123 Main St".to_string()),
            city: Some("Springfield".to_string()),
        },
    };
    assert!(order.validate().is_ok());
}

// --- Vec nested validation ---

#[derive(Validate)]
struct Team {
    #[validate(required)]
    name: Option<String>,
    #[validate(nested)]
    members: Vec<Member>,
}

#[derive(Validate)]
struct Member {
    #[validate(required, email)]
    email: Option<String>,
}

#[test]
fn test_vec_nested_validation() {
    let team = Team {
        name: Some("Avengers".to_string()),
        members: vec![
            Member { email: Some("valid@example.com".to_string()) },
            Member { email: None },
            Member { email: Some("invalid".to_string()) },
        ],
    };
    let err = team.validate().unwrap_err();
    let msgs = err.messages();
    assert!(msgs.contains_key("members.1.email"));
    assert!(msgs.contains_key("members.2.email"));
    assert!(!msgs.contains_key("members.0.email"));
}

// --- Custom messages ---

#[derive(Validate)]
#[validate(attributes(name = "full name"))]
struct Profile {
    #[validate(required(message = "We need your name!"), min = 2)]
    name: Option<String>,
    #[validate(required, email(message = "That doesn't look like a valid email."))]
    email: Option<String>,
}

#[test]
fn test_custom_messages() {
    let profile = Profile {
        name: None,
        email: Some("bad".to_string()),
    };
    let err = profile.validate().unwrap_err();
    let msgs = err.messages();
    assert_eq!(msgs["name"][0], "We need your name!");
    assert_eq!(msgs["email"][0], "That doesn't look like a valid email.");
}

#[test]
fn test_attribute_names_in_default_messages() {
    let profile = Profile {
        name: Some("A".to_string()), // too short
        email: None,
    };
    let err = profile.validate().unwrap_err();
    let msgs = err.messages();
    // The min error should use "full name" not "name"
    assert!(msgs["name"][0].contains("full name"));
}

// --- Conditional validation ---

#[derive(Validate)]
struct Registration {
    #[validate(required, in_list("user", "admin"))]
    role: Option<String>,

    #[validate(required_if(field = "role", value = "admin"))]
    admin_code: Option<String>,
}

#[test]
fn test_required_if_triggers() {
    let reg = Registration {
        role: Some("admin".to_string()),
        admin_code: None, // required because role is admin
    };
    let err = reg.validate().unwrap_err();
    assert!(err.messages().contains_key("admin_code"));
}

#[test]
fn test_required_if_skips() {
    let reg = Registration {
        role: Some("user".to_string()),
        admin_code: None, // not required because role is user
    };
    assert!(reg.validate().is_ok());
}

// --- Serialization ---

#[test]
fn test_error_serialization() {
    let user = CreateUser {
        email: None,
        name: None,
        password: None,
        password_confirmation: None,
        website: None,
        role: None,
    };
    let err = user.validate().unwrap_err();
    let json = serde_json::to_value(&err).unwrap();
    assert!(json.is_object());
    assert!(json.get("email").is_some());
    assert!(json["email"].is_array());
}

// --- Format rules ---

#[derive(Validate)]
struct FormatTests {
    #[validate(nullable, alpha)]
    alpha_field: Option<String>,
    #[validate(nullable, alpha_num)]
    alpha_num_field: Option<String>,
    #[validate(nullable, alpha_dash)]
    alpha_dash_field: Option<String>,
    #[validate(nullable, numeric)]
    numeric_field: Option<String>,
    #[validate(nullable, uuid)]
    uuid_field: Option<String>,
    #[validate(nullable, ip)]
    ip_field: Option<String>,
    #[validate(nullable, ascii)]
    ascii_field: Option<String>,
}

#[test]
fn test_alpha() {
    let t = FormatTests {
        alpha_field: Some("abc123".to_string()), // invalid: has digits
        alpha_num_field: None, alpha_dash_field: None, numeric_field: None,
        uuid_field: None, ip_field: None, ascii_field: None,
    };
    let err = t.validate().unwrap_err();
    assert!(err.messages().contains_key("alpha_field"));
}

#[test]
fn test_uuid_valid() {
    let t = FormatTests {
        alpha_field: None, alpha_num_field: None, alpha_dash_field: None,
        numeric_field: None, ip_field: None, ascii_field: None,
        uuid_field: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
    };
    assert!(t.validate().is_ok());
}

#[test]
fn test_uuid_invalid() {
    let t = FormatTests {
        alpha_field: None, alpha_num_field: None, alpha_dash_field: None,
        numeric_field: None, ip_field: None, ascii_field: None,
        uuid_field: Some("not-a-uuid".to_string()),
    };
    assert!(t.validate().is_err());
}
