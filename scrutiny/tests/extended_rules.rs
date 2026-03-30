use scrutiny::Validate;
use scrutiny::traits::Validate as ValidateTrait;

// ── String rules ──

#[derive(Validate)]
struct StringRules {
    #[validate(nullable, uppercase)]
    upper: Option<String>,
    #[validate(nullable, lowercase)]
    lower: Option<String>,
    #[validate(nullable, contains = "hello")]
    has_hello: Option<String>,
    #[validate(nullable, doesnt_contain = "bad")]
    no_bad: Option<String>,
    #[validate(nullable, doesnt_start_with = "X")]
    no_x_start: Option<String>,
    #[validate(nullable, doesnt_end_with = ".exe")]
    no_exe: Option<String>,
    #[validate(nullable, size = 5)]
    exact_five: Option<String>,
    #[validate(nullable, integer)]
    int_str: Option<String>,
}

#[test]
fn test_uppercase() {
    let s = StringRules {
        upper: Some("hello".into()),
        lower: None,
        has_hello: None,
        no_bad: None,
        no_x_start: None,
        no_exe: None,
        exact_five: None,
        int_str: None,
    };
    assert!(s.validate().is_err());

    let s = StringRules {
        upper: Some("HELLO".into()),
        lower: None,
        has_hello: None,
        no_bad: None,
        no_x_start: None,
        no_exe: None,
        exact_five: None,
        int_str: None,
    };
    assert!(s.validate().is_ok());
}

#[test]
fn test_lowercase() {
    let s = StringRules {
        upper: None,
        lower: Some("HELLO".into()),
        has_hello: None,
        no_bad: None,
        no_x_start: None,
        no_exe: None,
        exact_five: None,
        int_str: None,
    };
    assert!(s.validate().is_err());

    let s = StringRules {
        upper: None,
        lower: Some("hello".into()),
        has_hello: None,
        no_bad: None,
        no_x_start: None,
        no_exe: None,
        exact_five: None,
        int_str: None,
    };
    assert!(s.validate().is_ok());
}

#[test]
fn test_contains() {
    let s = StringRules {
        upper: None,
        lower: None,
        has_hello: Some("say hello world".into()),
        no_bad: None,
        no_x_start: None,
        no_exe: None,
        exact_five: None,
        int_str: None,
    };
    assert!(s.validate().is_ok());

    let s = StringRules {
        upper: None,
        lower: None,
        has_hello: Some("goodbye".into()),
        no_bad: None,
        no_x_start: None,
        no_exe: None,
        exact_five: None,
        int_str: None,
    };
    assert!(s.validate().is_err());
}

#[test]
fn test_doesnt_contain() {
    let s = StringRules {
        upper: None,
        lower: None,
        has_hello: None,
        no_bad: Some("this is bad".into()),
        no_x_start: None,
        no_exe: None,
        exact_five: None,
        int_str: None,
    };
    assert!(s.validate().is_err());

    let s = StringRules {
        upper: None,
        lower: None,
        has_hello: None,
        no_bad: Some("this is good".into()),
        no_x_start: None,
        no_exe: None,
        exact_five: None,
        int_str: None,
    };
    assert!(s.validate().is_ok());
}

#[test]
fn test_doesnt_start_with() {
    let s = StringRules {
        upper: None,
        lower: None,
        has_hello: None,
        no_bad: None,
        no_x_start: Some("Xfoo".into()),
        no_exe: None,
        exact_five: None,
        int_str: None,
    };
    assert!(s.validate().is_err());

    let s = StringRules {
        upper: None,
        lower: None,
        has_hello: None,
        no_bad: None,
        no_x_start: Some("foo".into()),
        no_exe: None,
        exact_five: None,
        int_str: None,
    };
    assert!(s.validate().is_ok());
}

#[test]
fn test_doesnt_end_with() {
    let s = StringRules {
        upper: None,
        lower: None,
        has_hello: None,
        no_bad: None,
        no_x_start: None,
        no_exe: Some("virus.exe".into()),
        exact_five: None,
        int_str: None,
    };
    assert!(s.validate().is_err());
}

#[test]
fn test_size() {
    let s = StringRules {
        upper: None,
        lower: None,
        has_hello: None,
        no_bad: None,
        no_x_start: None,
        no_exe: None,
        exact_five: Some("12345".into()),
        int_str: None,
    };
    assert!(s.validate().is_ok());

    let s = StringRules {
        upper: None,
        lower: None,
        has_hello: None,
        no_bad: None,
        no_x_start: None,
        no_exe: None,
        exact_five: Some("1234".into()),
        int_str: None,
    };
    assert!(s.validate().is_err());
}

#[test]
fn test_integer() {
    let s = StringRules {
        upper: None,
        lower: None,
        has_hello: None,
        no_bad: None,
        no_x_start: None,
        no_exe: None,
        exact_five: None,
        int_str: Some("42".into()),
    };
    assert!(s.validate().is_ok());

    let s = StringRules {
        upper: None,
        lower: None,
        has_hello: None,
        no_bad: None,
        no_x_start: None,
        no_exe: None,
        exact_five: None,
        int_str: Some("3.14".into()),
    };
    assert!(s.validate().is_err());
}

// ── Accepted / Declined ──

#[derive(Validate)]
struct AcceptDecline {
    #[validate(nullable, accepted)]
    terms: Option<String>,
    #[validate(nullable, declined)]
    opt_out: Option<String>,
}

#[test]
fn test_accepted() {
    let a = AcceptDecline {
        terms: Some("yes".into()),
        opt_out: None,
    };
    assert!(a.validate().is_ok());
    let a = AcceptDecline {
        terms: Some("true".into()),
        opt_out: None,
    };
    assert!(a.validate().is_ok());
    let a = AcceptDecline {
        terms: Some("no".into()),
        opt_out: None,
    };
    assert!(a.validate().is_err());
}

#[test]
fn test_declined() {
    let a = AcceptDecline {
        terms: None,
        opt_out: Some("no".into()),
    };
    assert!(a.validate().is_ok());
    let a = AcceptDecline {
        terms: None,
        opt_out: Some("off".into()),
    };
    assert!(a.validate().is_ok());
    let a = AcceptDecline {
        terms: None,
        opt_out: Some("yes".into()),
    };
    assert!(a.validate().is_err());
}

// ── Prohibited ──

#[derive(Validate)]
struct ProhibitedRules {
    #[validate(required, in_list("basic", "premium"))]
    plan: Option<String>,
    #[validate(prohibited_if(field = "plan", value = "basic"))]
    premium_feature: Option<String>,
    #[validate(prohibited)]
    never_set: Option<String>,
}

#[test]
fn test_prohibited() {
    let p = ProhibitedRules {
        plan: Some("basic".into()),
        premium_feature: None,
        never_set: None,
    };
    assert!(p.validate().is_ok());

    let p = ProhibitedRules {
        plan: Some("basic".into()),
        premium_feature: Some("enabled".into()),
        never_set: None,
    };
    assert!(p.validate().is_err());

    let p = ProhibitedRules {
        plan: Some("premium".into()),
        premium_feature: Some("enabled".into()),
        never_set: None,
    };
    assert!(p.validate().is_ok());
}

#[test]
fn test_prohibited_always() {
    let p = ProhibitedRules {
        plan: Some("basic".into()),
        premium_feature: None,
        never_set: Some("oops".into()),
    };
    assert!(p.validate().is_err());
}

// ── Date rules (ISO 8601 strict) ──

#[derive(Validate)]
struct DateRules {
    #[validate(nullable, date)]
    birth_date: Option<String>,
    #[validate(nullable, before = "2025-01-01")]
    before_2025: Option<String>,
    #[validate(nullable, after = "2020-01-01")]
    after_2020: Option<String>,
    #[validate(nullable, before_or_equal = "2025-12-31")]
    by_end_of_2025: Option<String>,
    #[validate(nullable, date_equals = "2024-06-15")]
    exact_date: Option<String>,
}

#[test]
fn test_date_valid() {
    let d = DateRules {
        birth_date: Some("2000-06-15".into()),
        before_2025: None,
        after_2020: None,
        by_end_of_2025: None,
        exact_date: None,
    };
    assert!(d.validate().is_ok());
}

#[test]
fn test_date_invalid_format() {
    let d = DateRules {
        birth_date: Some("06/15/2000".into()),
        before_2025: None,
        after_2020: None,
        by_end_of_2025: None,
        exact_date: None,
    };
    assert!(d.validate().is_err());
}

#[test]
fn test_date_invalid_day() {
    // Feb 30 doesn't exist
    let d = DateRules {
        birth_date: Some("2000-02-30".into()),
        before_2025: None,
        after_2020: None,
        by_end_of_2025: None,
        exact_date: None,
    };
    assert!(d.validate().is_err());
}

#[test]
fn test_date_leap_year() {
    // 2000 was a leap year
    let d = DateRules {
        birth_date: Some("2000-02-29".into()),
        before_2025: None,
        after_2020: None,
        by_end_of_2025: None,
        exact_date: None,
    };
    assert!(d.validate().is_ok());

    // 2001 was not
    let d = DateRules {
        birth_date: Some("2001-02-29".into()),
        before_2025: None,
        after_2020: None,
        by_end_of_2025: None,
        exact_date: None,
    };
    assert!(d.validate().is_err());
}

#[test]
fn test_before() {
    let d = DateRules {
        birth_date: None,
        before_2025: Some("2024-12-31".into()),
        after_2020: None,
        by_end_of_2025: None,
        exact_date: None,
    };
    assert!(d.validate().is_ok());

    let d = DateRules {
        birth_date: None,
        before_2025: Some("2025-06-01".into()),
        after_2020: None,
        by_end_of_2025: None,
        exact_date: None,
    };
    assert!(d.validate().is_err());
}

#[test]
fn test_after() {
    let d = DateRules {
        birth_date: None,
        before_2025: None,
        after_2020: Some("2021-01-01".into()),
        by_end_of_2025: None,
        exact_date: None,
    };
    assert!(d.validate().is_ok());

    let d = DateRules {
        birth_date: None,
        before_2025: None,
        after_2020: Some("2019-12-31".into()),
        by_end_of_2025: None,
        exact_date: None,
    };
    assert!(d.validate().is_err());
}

#[test]
fn test_date_equals() {
    let d = DateRules {
        birth_date: None,
        before_2025: None,
        after_2020: None,
        by_end_of_2025: None,
        exact_date: Some("2024-06-15".into()),
    };
    assert!(d.validate().is_ok());

    let d = DateRules {
        birth_date: None,
        before_2025: None,
        after_2020: None,
        by_end_of_2025: None,
        exact_date: Some("2024-06-16".into()),
    };
    assert!(d.validate().is_err());
}

// ── Format rules ──

#[derive(Validate)]
struct FormatRulesExtended {
    #[validate(nullable, hex_color)]
    color: Option<String>,
    #[validate(nullable, not_regex = r"^\d+$")]
    no_numbers_only: Option<String>,
}

#[test]
fn test_hex_color() {
    let f = FormatRulesExtended {
        color: Some("#ff0000".into()),
        no_numbers_only: None,
    };
    assert!(f.validate().is_ok());

    let f = FormatRulesExtended {
        color: Some("#f00".into()),
        no_numbers_only: None,
    };
    assert!(f.validate().is_ok());

    let f = FormatRulesExtended {
        color: Some("red".into()),
        no_numbers_only: None,
    };
    assert!(f.validate().is_err());
}

#[test]
fn test_not_regex() {
    let f = FormatRulesExtended {
        color: None,
        no_numbers_only: Some("123".into()),
    };
    assert!(f.validate().is_err()); // matches "only digits" pattern, should fail

    let f = FormatRulesExtended {
        color: None,
        no_numbers_only: Some("abc123".into()),
    };
    assert!(f.validate().is_ok());
}

// ── Numeric rules ──

#[derive(Validate)]
struct NumericRules {
    #[validate(nullable, digits = 4)]
    pin: Option<String>,
    #[validate(nullable, digits_between(min = 3, max = 5))]
    code: Option<String>,
    #[validate(nullable, multiple_of = "5")]
    multiple: Option<String>,
    #[validate(nullable, decimal = 2)]
    price: Option<String>,
    #[validate(nullable, decimal(min = 1, max = 3))]
    flex_decimal: Option<String>,
}

#[test]
fn test_digits() {
    let n = NumericRules {
        pin: Some("1234".into()),
        code: None,
        multiple: None,
        price: None,
        flex_decimal: None,
    };
    assert!(n.validate().is_ok());

    let n = NumericRules {
        pin: Some("123".into()),
        code: None,
        multiple: None,
        price: None,
        flex_decimal: None,
    };
    assert!(n.validate().is_err());
}

#[test]
fn test_digits_between() {
    let n = NumericRules {
        pin: None,
        code: Some("1234".into()),
        multiple: None,
        price: None,
        flex_decimal: None,
    };
    assert!(n.validate().is_ok());

    let n = NumericRules {
        pin: None,
        code: Some("12".into()),
        multiple: None,
        price: None,
        flex_decimal: None,
    };
    assert!(n.validate().is_err());
}

#[test]
fn test_multiple_of() {
    let n = NumericRules {
        pin: None,
        code: None,
        multiple: Some("15".into()),
        price: None,
        flex_decimal: None,
    };
    assert!(n.validate().is_ok());

    let n = NumericRules {
        pin: None,
        code: None,
        multiple: Some("7".into()),
        price: None,
        flex_decimal: None,
    };
    assert!(n.validate().is_err());
}

#[test]
fn test_decimal() {
    let n = NumericRules {
        pin: None,
        code: None,
        multiple: None,
        price: Some("19.99".into()),
        flex_decimal: None,
    };
    assert!(n.validate().is_ok());

    let n = NumericRules {
        pin: None,
        code: None,
        multiple: None,
        price: Some("19.9".into()),
        flex_decimal: None,
    };
    assert!(n.validate().is_err()); // only 1 decimal place, needs exactly 2

    let n = NumericRules {
        pin: None,
        code: None,
        multiple: None,
        price: None,
        flex_decimal: Some("3.14".into()),
    };
    assert!(n.validate().is_ok()); // 2 places, between 1-3

    let n = NumericRules {
        pin: None,
        code: None,
        multiple: None,
        price: None,
        flex_decimal: Some("3.1415".into()),
    };
    assert!(n.validate().is_err()); // 4 places, > max 3
}

// ── Cross-field comparison ──

#[derive(Validate)]
struct Comparison {
    start: Option<i32>,
    end: Option<i32>,
    #[validate(gt = "start")]
    must_be_greater: Option<i32>,
    #[validate(lte = "end")]
    must_be_lte: Option<i32>,
}

#[test]
fn test_gt_cross_field() {
    let c = Comparison {
        start: Some(10),
        end: Some(100),
        must_be_greater: Some(20),
        must_be_lte: Some(50),
    };
    assert!(c.validate().is_ok());
}

#[test]
fn test_gt_cross_field_fails() {
    let c = Comparison {
        start: Some(10),
        end: Some(100),
        must_be_greater: Some(5), // not > 10
        must_be_lte: Some(50),
    };
    assert!(c.validate().is_err());
}

#[test]
fn test_lte_cross_field_fails() {
    let c = Comparison {
        start: Some(10),
        end: Some(100),
        must_be_greater: Some(20),
        must_be_lte: Some(200), // not <= 100
    };
    assert!(c.validate().is_err());
}

// ── required_with_all / required_without_all ──

#[derive(Validate)]
struct MultiConditional {
    first_name: Option<String>,
    last_name: Option<String>,
    #[validate(required_with_all("first_name", "last_name"))]
    full_name_greeting: Option<String>,
}

#[test]
fn test_required_with_all_triggers() {
    let m = MultiConditional {
        first_name: Some("John".into()),
        last_name: Some("Doe".into()),
        full_name_greeting: None, // should fail: both first+last present
    };
    assert!(m.validate().is_err());
}

#[test]
fn test_required_with_all_skips_partial() {
    let m = MultiConditional {
        first_name: Some("John".into()),
        last_name: None,          // only one present, not all
        full_name_greeting: None, // should pass
    };
    assert!(m.validate().is_ok());
}

// ── Filled ──

#[derive(Validate)]
struct FilledTest {
    #[validate(nullable, filled)]
    bio: Option<String>,
}

#[test]
fn test_filled_none_ok() {
    let f = FilledTest { bio: None };
    assert!(f.validate().is_ok());
}

#[test]
fn test_filled_empty_fails() {
    let f = FilledTest {
        bio: Some("  ".into()),
    };
    assert!(f.validate().is_err());
}

#[test]
fn test_filled_with_content_ok() {
    let f = FilledTest {
        bio: Some("Hello".into()),
    };
    assert!(f.validate().is_ok());
}

// ── Distinct ──

#[derive(Validate)]
struct DistinctTest {
    #[validate(distinct)]
    tags: Vec<String>,
}

#[test]
fn test_distinct_ok() {
    let d = DistinctTest {
        tags: vec!["a".into(), "b".into(), "c".into()],
    };
    assert!(d.validate().is_ok());
}

#[test]
fn test_distinct_fails() {
    let d = DistinctTest {
        tags: vec!["a".into(), "b".into(), "a".into()],
    };
    assert!(d.validate().is_err());
}

// ── Numeric min/max/between ──

#[derive(Validate)]
struct Pagination {
    #[validate(min = 1, max = 10000)]
    per_page: f64,
    #[validate(min = 1)]
    page: f64,
    #[validate(nullable, min = 0)]
    offset: Option<f64>,
}

#[test]
fn test_numeric_min_max_valid() {
    let p = Pagination {
        per_page: 25.0,
        page: 1.0,
        offset: None,
    };
    assert!(p.validate().is_ok());
}

#[test]
fn test_numeric_min_fails() {
    let p = Pagination {
        per_page: 0.0,
        page: 1.0,
        offset: None,
    };
    assert!(p.validate().is_err());
}

#[test]
fn test_numeric_max_fails() {
    let p = Pagination {
        per_page: 99999.0,
        page: 1.0,
        offset: None,
    };
    assert!(p.validate().is_err());
}

#[test]
fn test_numeric_option_none_ok() {
    let p = Pagination {
        per_page: 10.0,
        page: 1.0,
        offset: None,
    };
    assert!(p.validate().is_ok());
}

#[test]
fn test_numeric_option_valid() {
    let p = Pagination {
        per_page: 10.0,
        page: 1.0,
        offset: Some(5.0),
    };
    assert!(p.validate().is_ok());
}

#[test]
fn test_numeric_option_invalid() {
    let p = Pagination {
        per_page: 10.0,
        page: 1.0,
        offset: Some(-1.0),
    };
    assert!(p.validate().is_err());
}

#[derive(Validate)]
struct IntRanges {
    #[validate(between(min = 1, max = 100))]
    score: i32,
    #[validate(min = 0, max = 255)]
    level: u32,
}

#[test]
fn test_int_between_valid() {
    let r = IntRanges {
        score: 50,
        level: 100,
    };
    assert!(r.validate().is_ok());
}

#[test]
fn test_int_between_too_low() {
    let r = IntRanges {
        score: 0,
        level: 100,
    };
    assert!(r.validate().is_err());
}

#[test]
fn test_int_between_too_high() {
    let r = IntRanges {
        score: 101,
        level: 100,
    };
    assert!(r.validate().is_err());
}

#[test]
fn test_int_max_fails() {
    let r = IntRanges {
        score: 50,
        level: 256,
    };
    assert!(r.validate().is_err());
}

// ── Vec min/max/size ──

#[derive(Validate)]
struct VecRules {
    #[validate(min = 1, max = 5)]
    tags: Vec<String>,
    #[validate(size = 3)]
    coordinates: Vec<f64>,
}

#[test]
fn test_vec_min_max_valid() {
    let v = VecRules {
        tags: vec!["a".into(), "b".into()],
        coordinates: vec![1.0, 2.0, 3.0],
    };
    assert!(v.validate().is_ok());
}

#[test]
fn test_vec_empty_fails_min() {
    let v = VecRules {
        tags: vec![],
        coordinates: vec![1.0, 2.0, 3.0],
    };
    assert!(v.validate().is_err());
}

#[test]
fn test_vec_too_many_fails_max() {
    let v = VecRules {
        tags: vec![
            "a".into(),
            "b".into(),
            "c".into(),
            "d".into(),
            "e".into(),
            "f".into(),
        ],
        coordinates: vec![1.0, 2.0, 3.0],
    };
    assert!(v.validate().is_err());
}

#[test]
fn test_vec_wrong_size() {
    let v = VecRules {
        tags: vec!["a".into()],
        coordinates: vec![1.0, 2.0], // needs exactly 3
    };
    assert!(v.validate().is_err());
}

// ── Numeric size (exact value) ──

#[derive(Validate)]
struct ExactValue {
    #[validate(size = 42)]
    answer: i32,
}

#[test]
fn test_numeric_size_exact() {
    let e = ExactValue { answer: 42 };
    assert!(e.validate().is_ok());
    let e = ExactValue { answer: 43 };
    assert!(e.validate().is_err());
}

// ── Default messages reflect type ──

#[test]
fn test_numeric_error_message() {
    let p = Pagination {
        per_page: 0.0,
        page: 1.0,
        offset: None,
    };
    let err = p.validate().unwrap_err();
    let msgs = err.messages();
    // Should say "at least 1" not "at least 1 characters"
    assert!(msgs["per_page"][0].contains("at least"));
    assert!(!msgs["per_page"][0].contains("characters"));
}

#[test]
fn test_vec_error_message() {
    let v = VecRules {
        tags: vec![],
        coordinates: vec![1.0, 2.0, 3.0],
    };
    let err = v.validate().unwrap_err();
    let msgs = err.messages();
    assert!(msgs["tags"][0].contains("items"));
}
