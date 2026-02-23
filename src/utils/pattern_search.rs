pub fn search_pattern(data: &[u8], pattern: &str) -> Vec<usize> {
    let parts: Vec<&str> = pattern.split_whitespace().collect();
    let pattern_len = parts.len();
    if pattern_len == 0 || data.len() < pattern_len {
        return Vec::new();
    }

    let mut bytes: Vec<Option<u8>> = Vec::with_capacity(pattern_len);
    for part in &parts {
        if *part == "?" || *part == "??" {
            bytes.push(None);
        } else {
            match u8::from_str_radix(part, 16) {
                Ok(b) => bytes.push(Some(b)),
                Err(_) => return Vec::new(),
            }
        }
    }

    let mut results = Vec::new();
    let end = data.len() - pattern_len;

    for i in 0..=end {
        let mut matched = true;
        for (j, byte) in bytes.iter().enumerate() {
            if let Some(expected) = byte {
                if data[i + j] != *expected {
                    matched = false;
                    break;
                }
            }
        }
        if matched {
            results.push(i);
        }
    }

    results
}

pub fn boyer_moore_horspool(data: &[u8], pattern: &[u8]) -> Vec<usize> {
    let n = data.len();
    let m = pattern.len();
    if m == 0 || n < m {
        return Vec::new();
    }

    let mut skip = [m; 256];
    for i in 0..m - 1 {
        skip[pattern[i] as usize] = m - 1 - i;
    }

    let mut results = Vec::new();
    let mut i = 0;

    while i <= n - m {
        let mut j = m - 1;
        loop {
            if data[i + j] != pattern[j] {
                break;
            }
            if j == 0 {
                results.push(i);
                break;
            }
            j -= 1;
        }
        i += skip[data[i + m - 1] as usize];
    }

    results
}

pub fn find_bytes(data: &[u8], pattern: &[u8]) -> Option<usize> {
    let n = data.len();
    let m = pattern.len();
    if m == 0 || n < m {
        return None;
    }

    let mut skip = [m; 256];
    for i in 0..m - 1 {
        skip[pattern[i] as usize] = m - 1 - i;
    }

    let mut i = 0;
    while i <= n - m {
        let mut j = m - 1;
        loop {
            if data[i + j] != pattern[j] {
                break;
            }
            if j == 0 {
                return Some(i);
            }
            j -= 1;
        }
        i += skip[data[i + m - 1] as usize];
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_pattern_exact() {
        let data = vec![0x10, 0x20, 0x30, 0x40, 0x50];
        let results = search_pattern(&data, "20 30");
        assert_eq!(results, vec![1]);
    }

    #[test]
    fn test_search_pattern_wildcard() {
        let data = vec![0x10, 0x20, 0x30, 0x40, 0x50];
        let results = search_pattern(&data, "? 20 ? 40");
        assert_eq!(results, vec![0]);
    }

    #[test]
    fn test_search_pattern_multiple() {
        let data = vec![0xAA, 0xBB, 0xAA, 0xBB, 0xCC];
        let results = search_pattern(&data, "AA BB");
        assert_eq!(results, vec![0, 2]);
    }

    #[test]
    fn test_bmh() {
        let data = b"hello world hello rust";
        let results = boyer_moore_horspool(data, b"hello");
        assert_eq!(results, vec![0, 12]);
    }

    #[test]
    fn test_find_bytes() {
        let data = b"mscorlib.dll\x00";
        assert_eq!(find_bytes(data, b"mscorlib.dll"), Some(0));
    }
}
