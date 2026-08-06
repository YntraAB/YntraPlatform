use super::sql_helpers::{contains_ignore_ascii_case, find_ignore_ascii_case};
use crate::database;

pub fn normalize_clock_skew(sql: &str, params: &mut [serde_json::Value]) {
    if params.is_empty() || !contains_ignore_ascii_case(sql, "updated_at") {
        return;
    }
    let cleaned = database::parser::clean_sql(sql);
    let now_ms = crate::infra::time::get_current_time_ms();

    // 1. Handle INSERT / REPLACE statements
    if contains_ignore_ascii_case(&cleaned, "insert")
        || contains_ignore_ascii_case(&cleaned, "replace")
    {
        if let Some(start_cols) = cleaned.find('(') {
            if let Some(end_cols) = cleaned[start_cols..].find(')') {
                let cols_str = &cleaned[start_cols + 1..start_cols + end_cols];
                for (idx, col) in cols_str.split(',').enumerate() {
                    let col_clean = col
                        .trim()
                        .trim_matches(|c| c == '`' || c == '"' || c == '\'');
                    if col_clean.eq_ignore_ascii_case("updated_at") && idx < params.len() {
                        if let Some(client_time) = params[idx].as_i64() {
                            // If client timestamp is in the future (plus a small 5-second tolerance for delays)
                            if client_time > now_ms + 5000 {
                                params[idx] =
                                    serde_json::Value::Number(serde_json::Number::from(now_ms));
                            }
                        }
                    }
                }
            }
        }
    }
    // 2. Handle UPDATE statements
    else if contains_ignore_ascii_case(&cleaned, "update") {
        if let Some(pos) = find_ignore_ascii_case(&cleaned, "updated_at") {
            let search_slice = &cleaned[pos..];
            if let Some(q_pos) = search_slice.find('?') {
                let start_digits = pos + q_pos + 1;
                let mut end_digits = start_digits;
                while end_digits < cleaned.len() && cleaned.as_bytes()[end_digits].is_ascii_digit()
                {
                    end_digits += 1;
                }
                if end_digits > start_digits {
                    if let Ok(param_idx_1based) = cleaned[start_digits..end_digits].parse::<usize>()
                    {
                        let param_idx = param_idx_1based - 1;
                        if param_idx < params.len() {
                            if let Some(client_time) = params[param_idx].as_i64() {
                                if client_time > now_ms + 5000 {
                                    params[param_idx] =
                                        serde_json::Value::Number(serde_json::Number::from(now_ms));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
