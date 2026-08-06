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

use sqlparser::ast::{AssignmentTarget, BinaryOperator, Expr, SetExpr, Statement, Value as SqlValue};
use sqlparser::dialect::SQLiteDialect;
use sqlparser::parser::Parser;
use std::collections::HashMap;

fn expr_to_json_val(expr: &Expr, params: &[serde_json::Value], param_index: &mut usize) -> serde_json::Value {
    match expr {
        Expr::Value(SqlValue::SingleQuotedString(s))
        | Expr::Value(SqlValue::DoubleQuotedString(s)) => serde_json::Value::String(s.clone()),
        Expr::Value(SqlValue::DollarQuotedString(s)) => serde_json::Value::String(s.value.clone()),
        Expr::Value(SqlValue::Number(n, _)) => {
            if let Ok(i) = n.parse::<i64>() {
                serde_json::Value::Number(i.into())
            } else if let Ok(f) = n.parse::<f64>() {
                if let Some(num) = serde_json::Number::from_f64(f) {
                    serde_json::Value::Number(num)
                } else {
                    serde_json::Value::String(n.clone())
                }
            } else {
                serde_json::Value::String(n.clone())
            }
        }
        Expr::Value(SqlValue::Boolean(b)) => serde_json::Value::Bool(*b),
        Expr::Value(SqlValue::Null) => serde_json::Value::Null,
        Expr::Value(SqlValue::Placeholder(p)) => {
            let idx = if p.starts_with('?') || p.starts_with('$') {
                p[1..].parse::<usize>().ok().map(|i| i.saturating_sub(1)).unwrap_or(*param_index)
            } else {
                *param_index
            };
            *param_index += 1;
            params.get(idx).cloned().unwrap_or(serde_json::Value::Null)
        }
        _ => serde_json::Value::String(expr.to_string()),
    }
}

fn extract_eq_predicates(
    expr: &Expr,
    params: &[serde_json::Value],
    param_index: &mut usize,
    map: &mut HashMap<String, serde_json::Value>,
) {
    match expr {
        Expr::BinaryOp { left, op: BinaryOperator::Eq, right } => {
            if let Expr::Identifier(ident) = &**left {
                let name = ident.value.to_lowercase();
                let val = expr_to_json_val(right, params, param_index);
                map.entry(name).or_insert(val);
            } else if let Expr::CompoundIdentifier(idents) = &**left {
                if let Some(ident) = idents.last() {
                    let name = ident.value.to_lowercase();
                    let val = expr_to_json_val(right, params, param_index);
                    map.entry(name).or_insert(val);
                }
            }
        }
        Expr::BinaryOp { left, op: BinaryOperator::And, right } => {
            extract_eq_predicates(left, params, param_index, map);
            extract_eq_predicates(right, params, param_index, map);
        }
        Expr::Nested(inner) => {
            extract_eq_predicates(inner, params, param_index, map);
        }
        _ => {}
    }
}

fn strip_cte(sql: &str) -> &str {
    let trimmed = sql.trim_start();
    if trimmed.len() >= 4 && trimmed[..4].eq_ignore_ascii_case("WITH") {
        let lower = trimmed.to_lowercase();
        if let Some(pos) = lower.find("update ")
            .or_else(|| lower.find("insert "))
            .or_else(|| lower.find("delete "))
        {
            return &trimmed[pos..];
        }
    }
    trimmed
}

pub fn parse_insert_columns_and_values(
    sql: &str,
    params: &[serde_json::Value],
) -> std::collections::HashMap<String, serde_json::Value> {
    let mut map = HashMap::new();
    let dialect = SQLiteDialect {};
    let target_sql = strip_cte(sql);

    if let Ok(statements) = Parser::parse_sql(&dialect, target_sql) {

        for stmt in statements {
            match stmt {
                Statement::Insert(insert) => {
                    let col_names: Vec<String> = insert
                        .columns
                        .iter()
                        .map(|c| c.value.to_lowercase())
                        .collect();

                    let mut param_index = 0;
                    if let Some(source) = &insert.source {
                        if let SetExpr::Values(values) = &*source.body {
                            if let Some(first_row) = values.rows.first() {
                                for (idx, expr) in first_row.iter().enumerate() {
                                    let val = expr_to_json_val(expr, params, &mut param_index);
                                    if idx < col_names.len() {
                                        map.insert(col_names[idx].clone(), val);
                                    }
                                }
                            }
                        }
                    } else {
                        for (idx, col) in col_names.iter().enumerate() {
                            if idx < params.len() {
                                map.insert(col.clone(), params[idx].clone());
                            }
                        }
                    }
                    if !map.is_empty() {
                        return map;
                    }
                }
                Statement::Update { assignments, selection, .. } => {
                    let mut param_index = 0;
                    for assignment in assignments {
                        let col_name = match &assignment.target {
                            AssignmentTarget::ColumnName(obj_name) => {
                                obj_name.0.last().map(|id| id.value.to_lowercase())
                            }
                            _ => None,
                        };
                        if let Some(name) = col_name {
                            let val = expr_to_json_val(&assignment.value, params, &mut param_index);
                            map.insert(name, val);
                        }
                    }
                    if let Some(where_expr) = &selection {
                        extract_eq_predicates(where_expr, params, &mut param_index, &mut map);
                    }
                    if !map.is_empty() {
                        return map;
                    }
                }
                _ => {}
            }
        }
    }

    parse_insert_columns_and_values_legacy(sql, params)
}




fn parse_insert_columns_and_values_legacy(
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

