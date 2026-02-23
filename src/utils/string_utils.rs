pub fn escape_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\0' => result.push_str("\\0"),
            c if c.is_control() => {
                result.push_str(&format!("\\x{:02X}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}

pub fn sanitize_c_identifier(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            result.push(ch);
        } else if matches!(ch, '.' | '/' | '<' | '>' | '[' | ']') {
            result.push('_');
        }
    }
    if result.starts_with(|c: char| c.is_ascii_digit()) {
        result.insert(0, '_');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_string() {
        assert_eq!(escape_string("hello"), "hello");
        assert_eq!(escape_string("he\"llo"), "he\\\"llo");
        assert_eq!(escape_string("line\nnew"), "line\\nnew");
        assert_eq!(escape_string("tab\there"), "tab\\there");
        assert_eq!(escape_string("back\\slash"), "back\\\\slash");
    }

    #[test]
    fn test_sanitize() {
        assert_eq!(sanitize_c_identifier("System.Collections.Generic"), "System_Collections_Generic");
        assert_eq!(sanitize_c_identifier("List`1"), "List_1");
        assert_eq!(sanitize_c_identifier("1BadName"), "_1BadName");
    }
}
