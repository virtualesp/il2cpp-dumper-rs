pub mod pattern_search;
pub mod string_utils;

pub use pattern_search::{search_pattern, boyer_moore_horspool};
pub use string_utils::escape_string;
