use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;
use crate::infra::time::get_current_time_ms;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, uniffi::Record)]
pub struct CsvImportPreview {
    pub delimiter: String,
    pub headers: Vec<String>,
    pub total_rows: u32,
    pub preview_rows_json: String,
    pub suggested_mappings_json: String,
    pub warnings: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, uniffi::Record)]
pub struct CsvImportResult {
    pub imported_count: u32,
    pub failed_count: u32,
    pub block_id: String,
    pub errors: Vec<String>,
}

fn detect_delimiter(csv_content: &str) -> char {
    let mut comma_count = 0;
    let mut semi_count = 0;
    let mut tab_count = 0;

    for line in csv_content.lines().take(5) {
        comma_count += line.matches(',').count();
        semi_count += line.matches(';').count();
        tab_count += line.matches('\t').count();
    }

    if semi_count > comma_count && semi_count > tab_count {
        ';'
    } else if tab_count > comma_count && tab_count > semi_count {
        '\t'
    } else {
        ','
    }
}

fn parse_csv_line(line: &str, delimiter: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let chars: Vec<char> = line.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            if in_quotes && i + 1 < chars.len() && chars[i + 1] == '"' {
                current.push('"');
                i += 1;
            } else {
                in_quotes = !in_quotes;
            }
        } else if c == delimiter && !in_quotes {
            fields.push(current.trim().to_string());
            current = String::new();
        } else {
            current.push(c);
        }
        i += 1;
    }
    fields.push(current.trim().to_string());
    fields
}

fn normalize_name(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

#[uniffi::export]
pub async fn parse_and_preview_csv(
    requester_user_id: String,
    csv_content: String,
    target_block_id: Option<String>,
) -> Result<CsvImportPreview, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let lines: Vec<&str> = csv_content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if lines.is_empty() {
        return Err(YntraError::ValidationError("CSV content is empty".to_string()));
    }

    let delimiter_char = detect_delimiter(&csv_content);
    let delimiter = delimiter_char.to_string();

    let headers = parse_csv_line(lines[0], delimiter_char);
    let total_rows = (lines.len().saturating_sub(1)) as u32;

    let mut preview_rows = Vec::new();
    let mut warnings = Vec::new();

    for (idx, line) in lines.iter().skip(1).take(5).enumerate() {
        let cols = parse_csv_line(line, delimiter_char);
        if cols.len() != headers.len() {
            warnings.push(format!("Row {} column count ({}) differs from header ({})", idx + 1, cols.len(), headers.len()));
        }
        let mut row_obj = serde_json::Map::new();
        for (h_idx, header) in headers.iter().enumerate() {
            let val = cols.get(h_idx).cloned().unwrap_or_default();
            row_obj.insert(header.clone(), serde_json::Value::String(val));
        }
        preview_rows.push(serde_json::Value::Object(row_obj));
    }

    // Attempt auto-mapping to block fields_schema
    let mut suggested_mappings = HashMap::new();
    if let Some(ref block_id) = target_block_id {
        let fields_schema_str: Option<String> = conn
            .query_row(
                "SELECT fields_schema FROM blocks WHERE id = ?1",
                crate::params![block_id],
                |r| r.get(0),
            )
            .await
            .ok()
            .flatten();

        if let Some(schema_json) = fields_schema_str {
            if let Ok(fields) = serde_json::from_str::<Vec<serde_json::Value>>(&schema_json) {
                for header in &headers {
                    let norm_h = normalize_name(header);
                    for field in &fields {
                        let name = field.get("name").and_then(|v| v.as_str()).unwrap_or_default();
                        let label = field.get("label").and_then(|v| v.as_str()).unwrap_or_default();
                        if normalize_name(name) == norm_h || normalize_name(label) == norm_h {
                            suggested_mappings.insert(header.clone(), name.to_string());
                            break;
                        }
                    }
                }
            }
        }
    }

    // Default 1:1 mapping if unmapped
    for header in &headers {
        if !suggested_mappings.contains_key(header) {
            suggested_mappings.insert(header.clone(), header.to_lowercase().replace(' ', "_"));
        }
    }

    Ok(CsvImportPreview {
        delimiter,
        headers,
        total_rows,
        preview_rows_json: serde_json::to_string(&preview_rows).unwrap_or_else(|_| "[]".to_string()),
        suggested_mappings_json: serde_json::to_string(&suggested_mappings).unwrap_or_else(|_| "{}".to_string()),
        warnings,
    })
}

#[uniffi::export]
pub async fn execute_csv_import(
    requester_user_id: String,
    workspace_id: String,
    block_id: String,
    column_mappings_json: String,
    csv_content: String,
) -> Result<CsvImportResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mappings: HashMap<String, String> = serde_json::from_str(&column_mappings_json)
        .map_err(|e| YntraError::ValidationError(format!("Invalid column mappings JSON: {}", e)))?;

    let lines: Vec<&str> = csv_content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if lines.len() <= 1 {
        return Err(YntraError::ValidationError("CSV content has no data rows".to_string()));
    }

    let delimiter_char = detect_delimiter(&csv_content);
    let headers = parse_csv_line(lines[0], delimiter_char);

    let mut imported_count = 0u32;
    let mut failed_count = 0u32;
    let mut errors = Vec::new();
    let now_ms = get_current_time_ms();

    for (row_idx, line) in lines.iter().skip(1).enumerate() {
        let cols = parse_csv_line(line, delimiter_char);
        let mut entity_data = serde_json::Map::new();

        for (h_idx, header) in headers.iter().enumerate() {
            if let Some(target_field) = mappings.get(header) {
                let raw_val = cols.get(h_idx).cloned().unwrap_or_default();
                entity_data.insert(target_field.clone(), serde_json::Value::String(raw_val));
            }
        }

        let entity_id = Uuid::new_v4().to_string();
        let data_str = serde_json::to_string(&entity_data).unwrap_or_else(|_| "{}".to_string());

        let res = conn.execute(
            "INSERT INTO entities (id, workspace_id, block_id, entity_type, data, created_at, updated_at, sync_status) VALUES (?1, ?2, ?3, 'custom', ?4, ?5, ?5, 'pending')",
            crate::params![&entity_id, &workspace_id, &block_id, &data_str, now_ms],
        ).await;

        match res {
            Ok(_) => imported_count += 1,
            Err(e) => {
                failed_count += 1;
                errors.push(format!("Row {}: {}", row_idx + 1, e));
            }
        }
    }

    notify_observers();

    Ok(CsvImportResult {
        imported_count,
        failed_count,
        block_id,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_csv_import_delimiter_and_execution_flow() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-csv-1', 'CSV WS', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-csv-1', 'ws-csv-1', 'csv@yntra.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO blocks (id, name, description, icon, category, created_at, fields_schema) VALUES ('blk-hvac', 'HVAC Work Orders', 'HVAC Block', 'wrench', 'HVAC', '2026-08-04', '[{\"name\":\"client_name\",\"label\":\"Customer Name\"},{\"name\":\"priority\",\"label\":\"Job Priority\"}]')", ()).await.unwrap();


        crate::infra::crypto::set_session_key("csv-test-key".to_string().into_bytes(), "ws-csv-1".to_string());

        // 1. Semicolon delimited CSV preview with fuzzy header mapping
        let csv_data = "Customer Name;Job Priority\nNordic Logistics;Urgent\nGrand Plaza;Normal";
        let preview = parse_and_preview_csv("u-csv-1".to_string(), csv_data.to_string(), Some("blk-hvac".to_string())).await.unwrap();

        assert_eq!(preview.delimiter, ";");
        assert_eq!(preview.total_rows, 2);
        assert!(preview.suggested_mappings_json.contains("client_name"));

        // 2. Execute CSV import
        let res = execute_csv_import(
            "u-csv-1".to_string(),
            "ws-csv-1".to_string(),
            "blk-hvac".to_string(),
            preview.suggested_mappings_json,
            csv_data.to_string()
        ).await.unwrap();

        assert_eq!(res.imported_count, 2);
        assert_eq!(res.failed_count, 0);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM entities WHERE workspace_id = 'ws-csv-1' AND block_id = 'blk-hvac'", (), |r| r.get(0))
            .await
            .unwrap();
        assert_eq!(count, 2);

        crate::infra::crypto::clear_session_key();
    }
}
