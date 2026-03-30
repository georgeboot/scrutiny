//! Built-in validation rule functions.
//!
//! These are pure functions called by the generated `Validate` impl.
//! You generally don't call these directly — the derive macro wires them up.
//!
//! - [`presence`] — `required`, `filled`, `accepted`, `declined`
//! - [`string`] — `alpha`, `uppercase`, `contains`, `starts_with`, length checks, etc.
//! - [`numeric`] — `min`/`max` value, `digits`, `multiple_of`, `decimal`
//! - [`mod@format`] — `email`, `url`, `uuid`, `ip`, `date`, `hex_color`, `regex`, etc.
//! - [`comparison`] — `same`, `different`, `gt`/`gte`/`lt`/`lte`, `in_list`, `distinct`

pub mod string;
pub mod numeric;
pub mod format;
pub mod comparison;
pub mod presence;
