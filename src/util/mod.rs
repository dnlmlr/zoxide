pub mod path;

use crate::db::Epoch;

use std::time::SystemTime;

// Returns the current UNIX timestamp (in seconds). Returns 0 upon error.
pub fn current_time() -> Epoch {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// Converts a string to lowercase. Provides a fast path for ASCII-only strings.
pub fn to_lowercase<S: AsRef<str>>(s: S) -> String {
    let s = s.as_ref();
    if s.is_ascii() {
        s.to_ascii_lowercase()
    } else {
        s.to_lowercase()
    }
}
