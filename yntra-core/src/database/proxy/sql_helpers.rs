use crate::database;

pub fn find_ignore_ascii_case(haystack: &str, needle: &str) -> Option<usize> {
    let h_bytes = haystack.as_bytes();
    let n_bytes = needle.as_bytes();
    if n_bytes.is_empty() || h_bytes.len() < n_bytes.len() {
        return None;
    }
    for i in 0..=(h_bytes.len() - n_bytes.len()) {
        if h_bytes[i..i + n_bytes.len()].eq_ignore_ascii_case(n_bytes) {
            return Some(i);
        }
    }
    None
}

pub fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    find_ignore_ascii_case(haystack, needle).is_some()
}

pub fn extract_public_key_from_metadata(meta: &str) -> String {
    if meta.is_empty() {
        return String::new();
    }
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(meta) {
        val.get("public_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                val.get("siths_public_key")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_default()
    } else {
        String::new()
    }
}

pub fn parse_insert_columns_and_values(
    sql: &str,
    params: &[serde_json::Value],
) -> std::collections::HashMap<String, serde_json::Value> {
    let mut map = std::collections::HashMap::new();
    let cleaned = database::parser::clean_sql(sql);
    if let Some(start_cols) = cleaned.find('(') {
        if let Some(end_cols) = cleaned[start_cols..].find(')') {
            let cols_str = &cleaned[start_cols + 1..start_cols + end_cols];
            for (idx, col) in cols_str.split(',').enumerate() {
                let col_clean = col
                    .trim()
                    .trim_matches(|c| c == '`' || c == '"' || c == '\'')
                    .to_lowercase();
                if idx < params.len() {
                    map.insert(col_clean, params[idx].clone());
                }
            }
        }
    }
    map
}
