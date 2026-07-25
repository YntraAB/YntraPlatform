use crate::database;
use crate::{YntraError, ZkCryptoTrust, ZeroCopyStore};
use std::sync::Arc;

fn find_ignore_ascii_case(haystack: &str, needle: &str) -> Option<usize> {
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

fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    find_ignore_ascii_case(haystack, needle).is_some()
}

fn parse_insert_columns_and_values(
    sql: &str,
    params: &[serde_json::Value],
) -> std::collections::HashMap<String, serde_json::Value> {
    let mut map = std::collections::HashMap::new();
    if let Some(start_cols) = sql.find('(') {
        if let Some(end_cols) = sql[start_cols..].find(')') {
            let cols_str = &sql[start_cols + 1..start_cols + end_cols];
            for (idx, col) in cols_str.split(',').enumerate() {
                let col_clean = col.trim().trim_matches(|c| c == '`' || c == '"' || c == '\'').to_lowercase();
                if idx < params.len() {
                    map.insert(col_clean, params[idx].clone());
                }
            }
        }
    }
    map
}

fn normalize_clock_skew(
    sql: &str,
    params: &mut [serde_json::Value],
) {
    let now_ms = crate::infra::time::get_current_time_ms();

    // 1. Handle INSERT / REPLACE statements
    if contains_ignore_ascii_case(sql, "insert") || contains_ignore_ascii_case(sql, "replace") {
        if let Some(start_cols) = sql.find('(') {
            if let Some(end_cols) = sql[start_cols..].find(')') {
                let cols_str = &sql[start_cols + 1..start_cols + end_cols];
                for (idx, col) in cols_str.split(',').enumerate() {
                    let col_clean = col.trim().trim_matches(|c| c == '`' || c == '"' || c == '\'');
                    if col_clean.eq_ignore_ascii_case("updated_at") && idx < params.len() {
                        if let Some(client_time) = params[idx].as_i64() {
                            // If client timestamp is in the future (plus a small 5-second tolerance for delays)
                            if client_time > now_ms + 5000 {
                                params[idx] = serde_json::Value::Number(serde_json::Number::from(now_ms));
                            }
                        }
                    }
                }
            }
        }
    }
    // 2. Handle UPDATE statements
    else if contains_ignore_ascii_case(sql, "update") {
        if let Some(pos) = find_ignore_ascii_case(sql, "updated_at") {
            let search_slice = &sql[pos..];
            if let Some(q_pos) = search_slice.find('?') {
                let start_digits = pos + q_pos + 1;
                let mut end_digits = start_digits;
                while end_digits < sql.len() && sql.as_bytes()[end_digits].is_ascii_digit() {
                    end_digits += 1;
                }
                if end_digits > start_digits {
                    if let Ok(param_idx_1based) = sql[start_digits..end_digits].parse::<usize>() {
                        let param_idx = param_idx_1based - 1;
                        if param_idx < params.len() {
                            if let Some(client_time) = params[param_idx].as_i64() {
                                if client_time > now_ms + 5000 {
                                    params[param_idx] = serde_json::Value::Number(serde_json::Number::from(now_ms));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, uniffi::Object)]
pub struct RemoteSyncCoordinator {}

#[uniffi::export]
impl RemoteSyncCoordinator {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {}
    }

    /// Intercepts and verifies a client's SQL write transaction payload with a ZK-proof before executing it against the primary database.
    pub async fn verify_and_execute_write(
        &self,
        requester_user_id: String,
        role: String,
        role_proof: Option<String>,
        sql: String,
        params_json: String,
    ) -> Result<u64, YntraError> {
        let conn = database::acquire_connection().await?;

        // 1. Fetch user's public key from the database metadata
        let metadata_str: Option<String> = conn
            .query_row(
                "SELECT metadata FROM users WHERE id = ?1",
                crate::params![&requester_user_id],
                |r| r.get(0),
            )
            .await
            .ok()
            .flatten();

        let public_key_hex = if let Some(ref meta) = metadata_str {
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
        } else {
            String::new()
        };

        if public_key_hex.is_empty() {
            return Err(YntraError::AuthError(
                "Cryptographic role verification failed: User public key not found".to_string(),
            ));
        }

        // 2. Validate the Zero-Knowledge proof payload
        let is_proof_required = if crate::infra::auth::is_production() {
            true
        } else {
            role_proof.is_some()
        };

        if is_proof_required {
            let proof = role_proof.ok_or_else(|| {
                YntraError::AuthError("Zero-Knowledge Role Proof is required for write operations".to_string())
            })?;

            #[cfg(not(target_arch = "wasm32"))]
            let is_valid = {
                let proof_c = proof.clone();
                let uid_c = requester_user_id.clone();
                let role_c = role.clone();
                let pk_c = public_key_hex.clone();
                tokio::task::spawn_blocking(move || {
                    let trust = ZkCryptoTrust::new();
                    trust.verify_proof(proof_c, uid_c, role_c, pk_c)
                })
                .await
                .unwrap_or(false)
            };

            #[cfg(target_arch = "wasm32")]
            let is_valid = {
                let trust = ZkCryptoTrust::new();
                trust.verify_proof(proof, requester_user_id.clone(), role.clone(), public_key_hex.clone())
            };

            if !is_valid {
                return Err(YntraError::CryptoError(
                    "Zero-Knowledge Role Proof verification failed: privilege escalation or local database tampering suspected".to_string(),
                ));
            }
        }

        // 3. Parse JSON params
        let mut parsed_params: Vec<serde_json::Value> = if params_json.is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&params_json)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?
        };

        normalize_clock_skew(&sql, &mut parsed_params);

        // 4. Enforce server-side Role-Based Access Control (RBAC) validations on SQL write payloads
        let role_lower = role.to_lowercase();
        let is_unprivileged = role_lower == "student" || role_lower == "role-school-student" || role_lower == "parent" || role_lower == "role-school-parent";

        if is_unprivileged {
            let table_name_opt = database::parser::extract_table_name(&sql);
            let table_name = match table_name_opt {
                Some(t) => t.to_lowercase(),
                None => {
                    return Err(YntraError::AuthError(
                        "Access denied: SQL command has invalid or untrackable table".to_string(),
                    ));
                }
            };

            if table_name == "submissions" {
                let col_vals = parse_insert_columns_and_values(&sql, &parsed_params);

                // Enforce that unprivileged users cannot write/modify grades or feedback
                if let Some(grade) = col_vals.get("grade") {
                    if !grade.is_null() && grade.as_str() != Some("") {
                        return Err(YntraError::AuthError(
                            "Access denied: Students and Parents cannot set or modify grades".to_string(),
                        ));
                    }
                }
                if let Some(feedback) = col_vals.get("feedback") {
                    if !feedback.is_null() && feedback.as_str() != Some("") {
                        return Err(YntraError::AuthError(
                            "Access denied: Students and Parents cannot set or modify feedback".to_string(),
                        ));
                    }
                }

                // Enforce that student_id belongs to the requester
                let student_id = col_vals.get("student_id").and_then(|v| v.as_str());
                match student_id {
                    Some(sid) => {
                        let mut authorized = false;
                        if role_lower == "student" || role_lower == "role-school-student" {
                            let profile_uid: Option<String> = conn
                                .query_row(
                                    "SELECT user_id FROM student_profiles WHERE id = ?1",
                                    crate::params![sid],
                                    |r| r.get(0),
                                )
                                .await
                                .ok()
                                .flatten();
                            if let Some(uid) = profile_uid {
                                if uid == requester_user_id {
                                    authorized = true;
                                }
                            }
                        } else if role_lower == "parent" || role_lower == "role-school-parent" {
                            let linked: Option<i64> = conn
                                .query_row(
                                    "SELECT 1 FROM student_parents WHERE student_id = ?1 AND parent_user_id = ?2",
                                    crate::params![sid, &requester_user_id],
                                    |r| r.get(0),
                                )
                                .await
                                .ok();
                            if linked.is_some() {
                                authorized = true;
                            }
                        }

                        if !authorized {
                            return Err(YntraError::AuthError(
                                "Access denied: You are not authorized to submit for this student".to_string(),
                            ));
                        }
                    }
                    None => {
                        return Err(YntraError::ValidationError(
                            "student_id is required for submissions".to_string(),
                        ));
                    }
                }
            } else if table_name == "library_lending_logs" {
                let lower_sql = sql.to_lowercase();
                if lower_sql.contains("update") {
                    let log_id = parsed_params.iter().find(|v| v.is_string() && (v.as_str().unwrap().starts_with("log-") || v.as_str().unwrap().starts_with("lend-")));
                    if let Some(log_id_val) = log_id {
                        let log_id_str = log_id_val.as_str().unwrap();
                        let student_id: Option<String> = conn.query_row(
                            "SELECT student_id FROM library_lending_logs WHERE id = ?1",
                            crate::params![log_id_str],
                            |r| r.get(0)
                        ).await.ok();

                        if let Some(sid) = student_id {
                            let mut authorized = false;
                            if role_lower == "student" || role_lower == "role-school-student" {
                                let profile_uid: Option<String> = conn.query_row(
                                    "SELECT user_id FROM student_profiles WHERE id = ?1",
                                    crate::params![&sid],
                                    |r| r.get(0)
                                ).await.ok().flatten();
                                if let Some(uid) = profile_uid {
                                    if uid == requester_user_id {
                                        authorized = true;
                                    }
                                }
                            } else if role_lower == "parent" || role_lower == "role-school-parent" {
                                let count: Option<i64> = conn.query_row(
                                    "SELECT 1 FROM student_parents WHERE student_id = ?1 AND parent_user_id = ?2",
                                    crate::params![&sid, &requester_user_id],
                                    |r| r.get(0)
                                ).await.ok();
                                if count.is_some() {
                                    authorized = true;
                                }
                            }
                            if !authorized {
                                return Err(YntraError::AuthError("Access denied: You are not authorized to modify this library log".to_string()));
                            }
                        } else {
                            return Err(YntraError::NotFoundError("Lending log not found".to_string()));
                        }
                    } else {
                        return Err(YntraError::ValidationError("Missing library log ID".to_string()));
                    }
                } else {
                    return Err(YntraError::AuthError("Access denied: Students/Parents can only update library logs".to_string()));
                }
            } else {
                return Err(YntraError::AuthError(format!(
                    "Access denied: role '{}' does not have write permissions to table '{}'",
                    role, table_name
                )));
            }
        }

        conn.begin_transaction().await?;

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut libsql_params = Vec::new();
            for val in parsed_params {
                let l_val = match val {
                    serde_json::Value::Null => libsql::Value::Null,
                    serde_json::Value::Bool(b) => libsql::Value::Integer(if b { 1 } else { 0 }),
                    serde_json::Value::Number(num) => {
                        if let Some(i) = num.as_i64() {
                            libsql::Value::Integer(i)
                        } else if let Some(f) = num.as_f64() {
                            libsql::Value::Real(f)
                        } else {
                            libsql::Value::Null
                        }
                    }
                    serde_json::Value::String(s) => libsql::Value::Text(s),
                    _ => libsql::Value::Null,
                };
                libsql_params.push(l_val);
            }

            let res = match conn.execute(&sql, libsql_params).await {
                Ok(r) => r,
                Err(e) => {
                    let _ = conn.rollback().await;
                    return Err(e);
                }
            };
            conn.commit().await?;
            Ok(res)
        }

        #[cfg(target_arch = "wasm32")]
        {
            let res = match conn.execute(&sql, parsed_params).await {
                Ok(r) => r,
                Err(e) => {
                    let _ = conn.rollback().await;
                    return Err(e);
                }
            };
            conn.commit().await?;
            Ok(res)
        }
    }

    /// Intercepts and verifies a client's Loro CRDT update sync payload with a ZK-proof before applying it to the primary store.
    pub async fn verify_and_apply_loro_sync(
        &self,
        requester_user_id: String,
        role: String,
        role_proof: Option<String>,
        compliance_proof: Option<String>,
        loro_update_hex: String,
        store: Arc<ZeroCopyStore>,
    ) -> Result<(), YntraError> {
        let conn = database::acquire_connection().await?;

        // 1. Fetch user's public key from the database metadata
        let metadata_str: Option<String> = conn
            .query_row(
                "SELECT metadata FROM users WHERE id = ?1",
                crate::params![&requester_user_id],
                |r| r.get(0),
            )
            .await
            .ok()
            .flatten();

        let public_key_hex = if let Some(ref meta) = metadata_str {
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
        } else {
            String::new()
        };

        if public_key_hex.is_empty() {
            return Err(YntraError::AuthError(
                "Cryptographic role verification failed: User public key not found".to_string(),
            ));
        }

        let trust = ZkCryptoTrust::new();

        // 2. Validate the Zero-Knowledge role proof payload (Authentication)
        let is_proof_required = if crate::infra::auth::is_production() {
            true
        } else {
            role_proof.is_some()
        };

        if is_proof_required {
            let proof = role_proof.ok_or_else(|| {
                YntraError::AuthError("Zero-Knowledge Role Proof is required for write operations".to_string())
            })?;

            if !trust.verify_proof(
                proof,
                requester_user_id.clone(),
                role.clone(),
                public_key_hex.clone(),
            ) {
                return Err(YntraError::CryptoError(
                    "Zero-Knowledge Role Proof verification failed: privilege escalation or local database tampering suspected".to_string(),
                ));
            }
        }

        // 3. Validate compliance proof payload if provided (Schema constraint verification)
        let update_bytes = const_hex::decode(&loro_update_hex)
            .map_err(|e| YntraError::CryptoError(e.to_string()))?;

        if let Some(comp_proof) = compliance_proof {
            let data_hash = blake3::hash(&update_bytes);
            let data_hash_hex = const_hex::encode(data_hash.as_bytes());

            let is_compliant = trust.verify_compliance_proof(
                comp_proof,
                requester_user_id,
                role,
                data_hash_hex,
                public_key_hex,
            )?;

            if !is_compliant {
                return Err(YntraError::CryptoError(
                    "Zero-Knowledge Compliance Proof verification failed: payload violates schema rules or is tampered".to_string(),
                ));
            }
        }

        // 4. Apply to the primary store
        store.apply_loro_update(update_bytes)
            .map_err(|e| YntraError::SyncError(e.to_string()))?;

        Ok(())
    }

    /// Generates a row-level partitioned sync payload for the client replica.
    /// Only returns rows authorized for the requester's workspace and role (enforces student/parent boundaries).
    pub async fn generate_partitioned_sync_payload(
        &self,
        requester_user_id: String,
        role: String,
        role_proof: Option<String>,
        table_name: String,
    ) -> Result<String, YntraError> {
        let conn = database::acquire_connection().await?;
        let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

        // 1. Fetch user's public key from the database metadata
        let metadata_str: Option<String> = conn
            .query_row(
                "SELECT metadata FROM users WHERE id = ?1",
                crate::params![&requester_user_id],
                |r| r.get(0),
            )
            .await
            .ok()
            .flatten();

        let public_key_hex = if let Some(ref meta) = metadata_str {
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
        } else {
            String::new()
        };

        if public_key_hex.is_empty() {
            return Err(YntraError::AuthError(
                "Cryptographic role verification failed: User public key not found".to_string(),
            ));
        }

        // 2. Validate the Zero-Knowledge role proof payload (Authentication)
        let is_proof_required = if crate::infra::auth::is_production() {
            true
        } else {
            role_proof.is_some()
        };

        if is_proof_required {
            let proof = role_proof.ok_or_else(|| {
                YntraError::AuthError("Zero-Knowledge Role Proof is required for sync operations".to_string())
            })?;

            let trust = ZkCryptoTrust::new();
            if !trust.verify_proof(
                proof,
                requester_user_id.clone(),
                role.clone(),
                public_key_hex.clone(),
            ) {
                return Err(YntraError::CryptoError(
                    "Zero-Knowledge Role Proof verification failed: privilege escalation or local database tampering suspected".to_string(),
                ));
            }
        }

        // 3. Determine if user has restricted access (student/parent) and fetch their authorized student IDs
        let is_privileged = auth.role == "platform_admin"
            || auth.role == "admin"
            || auth.role == "school-admin"
            || auth.role == "role-school-admin"
            || auth.role == "teacher"
            || auth.role == "principal"
            || auth.role == "role-school-principal"
            || auth.role == "staff";

        let student_ids = if !is_privileged {
            get_authorized_student_ids_helper(&conn, &auth).await?
        } else {
            Vec::new()
        };

        // 4. Construct SQL query based on table and authorization
        let json_rows = match table_name.as_str() {
            "student_profiles" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, user_id, first_name, last_name, grade_level, parent_contact, updated_at FROM student_profiles").await?
                } else if is_privileged {
                    conn.prepare("SELECT id, workspace_id, user_id, first_name, last_name, grade_level, parent_contact, updated_at FROM student_profiles WHERE workspace_id = ?1").await?
                } else {
                    let place_holders = if student_ids.is_empty() {
                        "''".to_string()
                    } else {
                        student_ids.iter().map(|id| format!("'{}'", id.replace('\'', "''"))).collect::<Vec<_>>().join(",")
                    };
                    let query = format!(
                        "SELECT id, workspace_id, user_id, first_name, last_name, grade_level, parent_contact, updated_at FROM student_profiles WHERE workspace_id = ?1 AND (user_id = ?2 OR id IN ({}))",
                        place_holders
                    );
                    conn.prepare(&query).await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else if is_privileged {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id, &auth.user_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let user_id: Option<String> = row.get(2)?;
                    let parent_contact: Option<String> = row.get(6)?;
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "user_id": user_id,
                        "first_name": row.get::<String>(3)?,
                        "last_name": row.get::<String>(4)?,
                        "grade_level": row.get::<String>(5)?,
                        "parent_contact": parent_contact,
                        "updated_at": row.get::<i64>(7)?,
                    });
                    list.push(item);
                }
                list
            }
            "attendance_records" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, student_id, course_id, date, status, notes, updated_at FROM attendance_records").await?
                } else if is_privileged {
                    conn.prepare("SELECT id, workspace_id, student_id, course_id, date, status, notes, updated_at FROM attendance_records WHERE workspace_id = ?1").await?
                } else {
                    let place_holders = if student_ids.is_empty() {
                        "''".to_string()
                    } else {
                        student_ids.iter().map(|id| format!("'{}'", id.replace('\'', "''"))).collect::<Vec<_>>().join(",")
                    };
                    let query = format!(
                        "SELECT id, workspace_id, student_id, course_id, date, status, notes, updated_at FROM attendance_records WHERE workspace_id = ?1 AND student_id IN ({})",
                        place_holders
                    );
                    conn.prepare(&query).await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let notes: Option<String> = row.get(6)?;
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "student_id": row.get::<String>(2)?,
                        "course_id": row.get::<String>(3)?,
                        "date": row.get::<String>(4)?,
                        "status": row.get::<String>(5)?,
                        "notes": notes,
                        "updated_at": row.get::<i64>(7)?,
                    });
                    list.push(item);
                }
                list
            }
            "health_incidents" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, student_id, visit_reason, treatment, checked_in_at, checked_out_at, notes, updated_at FROM health_incidents").await?
                } else if is_privileged {
                    conn.prepare("SELECT id, workspace_id, student_id, visit_reason, treatment, checked_in_at, checked_out_at, notes, updated_at FROM health_incidents WHERE workspace_id = ?1").await?
                } else {
                    let place_holders = if student_ids.is_empty() {
                        "''".to_string()
                    } else {
                        student_ids.iter().map(|id| format!("'{}'", id.replace('\'', "''"))).collect::<Vec<_>>().join(",")
                    };
                    let query = format!(
                        "SELECT id, workspace_id, student_id, visit_reason, treatment, checked_in_at, checked_out_at, notes, updated_at FROM health_incidents WHERE workspace_id = ?1 AND student_id IN ({})",
                        place_holders
                    );
                    conn.prepare(&query).await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let checked_out_at: Option<String> = row.get(6)?;
                    let notes: Option<String> = row.get(7)?;
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "student_id": row.get::<String>(2)?,
                        "visit_reason": row.get::<String>(3)?,
                        "treatment": row.get::<String>(4)?,
                        "checked_in_at": row.get::<String>(5)?,
                        "checked_out_at": checked_out_at,
                        "notes": notes,
                        "updated_at": row.get::<i64>(8)?,
                    });
                    list.push(item);
                }
                list
            }
            "term_grades" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, student_id, course_id, term_name, final_grade, final_points, teacher_comments, updated_at FROM term_grades").await?
                } else if is_privileged {
                    conn.prepare("SELECT id, workspace_id, student_id, course_id, term_name, final_grade, final_points, teacher_comments, updated_at FROM term_grades WHERE workspace_id = ?1").await?
                } else {
                    let place_holders = if student_ids.is_empty() {
                        "''".to_string()
                    } else {
                        student_ids.iter().map(|id| format!("'{}'", id.replace('\'', "''"))).collect::<Vec<_>>().join(",")
                    };
                    let query = format!(
                        "SELECT id, workspace_id, student_id, course_id, term_name, final_grade, final_points, teacher_comments, updated_at FROM term_grades WHERE workspace_id = ?1 AND student_id IN ({})",
                        place_holders
                    );
                    conn.prepare(&query).await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let final_grade: Option<String> = row.get(5)?;
                    let final_points: Option<i64> = row.get(6)?;
                    let teacher_comments: Option<String> = row.get(7)?;
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "student_id": row.get::<String>(2)?,
                        "course_id": row.get::<String>(3)?,
                        "term_name": row.get::<String>(4)?,
                        "final_grade": final_grade,
                        "final_points": final_points,
                        "teacher_comments": teacher_comments,
                        "updated_at": row.get::<i64>(8)?,
                    });
                    list.push(item);
                }
                list
            }
            "timetable_slots" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, course_id, day_of_week, start_time, end_time, classroom, updated_at FROM timetable_slots").await?
                } else {
                    conn.prepare("SELECT id, workspace_id, course_id, day_of_week, start_time, end_time, classroom, updated_at FROM timetable_slots WHERE workspace_id = ?1").await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let classroom: Option<String> = row.get(6)?;
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "course_id": row.get::<String>(2)?,
                        "day_of_week": row.get::<i64>(3)?,
                        "start_time": row.get::<String>(4)?,
                        "end_time": row.get::<String>(5)?,
                        "classroom": classroom,
                        "updated_at": row.get::<i64>(7)?,
                    });
                    list.push(item);
                }
                list
            }
            "courses" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, name, subject, teacher_id, classroom, updated_at FROM courses").await?
                } else {
                    conn.prepare("SELECT id, workspace_id, name, subject, teacher_id, classroom, updated_at FROM courses WHERE workspace_id = ?1").await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let teacher_id: Option<String> = row.get(4)?;
                    let classroom: Option<String> = row.get(5)?;
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "name": row.get::<String>(2)?,
                        "subject": row.get::<String>(3)?,
                        "teacher_id": teacher_id,
                        "classroom": classroom,
                        "updated_at": row.get::<i64>(6)?,
                    });
                    list.push(item);
                }
                list
            }
            "school_invoices" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, student_id, title, amount, due_date, status, paid_at, updated_at FROM school_invoices").await?
                } else if is_privileged {
                    conn.prepare("SELECT id, workspace_id, student_id, title, amount, due_date, status, paid_at, updated_at FROM school_invoices WHERE workspace_id = ?1").await?
                } else {
                    let place_holders = if student_ids.is_empty() {
                        "''".to_string()
                    } else {
                        student_ids.iter().map(|id| format!("'{}'", id.replace('\'', "''"))).collect::<Vec<_>>().join(",")
                    };
                    let query = format!(
                        "SELECT id, workspace_id, student_id, title, amount, due_date, status, paid_at, updated_at FROM school_invoices WHERE workspace_id = ?1 AND student_id IN ({})",
                        place_holders
                    );
                    conn.prepare(&query).await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let paid_at: Option<String> = row.get(7)?;
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "student_id": row.get::<String>(2)?,
                        "title": row.get::<String>(3)?,
                        "amount": row.get::<f64>(4)?,
                        "due_date": row.get::<String>(5)?,
                        "status": row.get::<String>(6)?,
                        "paid_at": paid_at,
                        "updated_at": row.get::<i64>(8)?,
                    });
                    list.push(item);
                }
                list
            }
            "submissions" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, assignment_id, student_id, content, grade, feedback, submitted_at, updated_at FROM submissions").await?
                } else if is_privileged {
                    conn.prepare("SELECT id, workspace_id, assignment_id, student_id, content, grade, feedback, submitted_at, updated_at FROM submissions WHERE workspace_id = ?1").await?
                } else {
                    let place_holders = if student_ids.is_empty() {
                        "''".to_string()
                    } else {
                        student_ids.iter().map(|id| format!("'{}'", id.replace('\'', "''"))).collect::<Vec<_>>().join(",")
                    };
                    let query = format!(
                        "SELECT id, workspace_id, assignment_id, student_id, content, grade, feedback, submitted_at, updated_at FROM submissions WHERE workspace_id = ?1 AND student_id IN ({})",
                        place_holders
                    );
                    conn.prepare(&query).await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let grade: Option<String> = row.get(5)?;
                    let feedback: Option<String> = row.get(6)?;
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "assignment_id": row.get::<String>(2)?,
                        "student_id": row.get::<String>(3)?,
                        "content": row.get::<String>(4)?,
                        "grade": grade,
                        "feedback": feedback,
                        "submitted_at": row.get::<String>(7)?,
                        "updated_at": row.get::<i64>(8)?,
                    });
                    list.push(item);
                }
                list
            }
            "health_records" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, student_id, vaccine_name, status, administered_at, updated_at FROM health_records").await?
                } else if is_privileged {
                    conn.prepare("SELECT id, workspace_id, student_id, vaccine_name, status, administered_at, updated_at FROM health_records WHERE workspace_id = ?1").await?
                } else {
                    let place_holders = if student_ids.is_empty() {
                        "''".to_string()
                    } else {
                        student_ids.iter().map(|id| format!("'{}'", id.replace('\'', "''"))).collect::<Vec<_>>().join(",")
                    };
                    let query = format!(
                        "SELECT id, workspace_id, student_id, vaccine_name, status, administered_at, updated_at FROM health_records WHERE workspace_id = ?1 AND student_id IN ({})",
                        place_holders
                    );
                    conn.prepare(&query).await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let administered_at: Option<String> = row.get(5)?;
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "student_id": row.get::<String>(2)?,
                        "vaccine_name": row.get::<String>(3)?,
                        "status": row.get::<String>(4)?,
                        "administered_at": administered_at,
                        "updated_at": row.get::<i64>(6)?,
                    });
                    list.push(item);
                }
                list
            }
            "report_cards" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, student_id, term_name, gpa, principal_comments, status, updated_at FROM report_cards").await?
                } else if is_privileged {
                    conn.prepare("SELECT id, workspace_id, student_id, term_name, gpa, principal_comments, status, updated_at FROM report_cards WHERE workspace_id = ?1").await?
                } else {
                    let place_holders = if student_ids.is_empty() {
                        "''".to_string()
                    } else {
                        student_ids.iter().map(|id| format!("'{}'", id.replace('\'', "''"))).collect::<Vec<_>>().join(",")
                    };
                    let query = format!(
                        "SELECT id, workspace_id, student_id, term_name, gpa, principal_comments, status, updated_at FROM report_cards WHERE workspace_id = ?1 AND student_id IN ({})",
                        place_holders
                    );
                    conn.prepare(&query).await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let principal_comments: Option<String> = row.get(5)?;
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "student_id": row.get::<String>(2)?,
                        "term_name": row.get::<String>(3)?,
                        "gpa": row.get::<f64>(4)?,
                        "principal_comments": principal_comments,
                        "status": row.get::<String>(6)?,
                        "updated_at": row.get::<i64>(7)?,
                    });
                    list.push(item);
                }
                list
            }
            "library_books" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, title, author, isbn, copies_available, total_copies, updated_at FROM library_books").await?
                } else {
                    conn.prepare("SELECT id, workspace_id, title, author, isbn, copies_available, total_copies, updated_at FROM library_books WHERE workspace_id = ?1").await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "title": row.get::<String>(2)?,
                        "author": row.get::<String>(3)?,
                        "isbn": row.get::<String>(4)?,
                        "copies_available": row.get::<i64>(5)?,
                        "total_copies": row.get::<i64>(6)?,
                        "updated_at": row.get::<i64>(7)?,
                    });
                    list.push(item);
                }
                list
            }
            "library_lending_logs" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, book_id, student_id, checked_out_at, due_date, returned_at, status, updated_at FROM library_lending_logs").await?
                } else if is_privileged {
                    conn.prepare("SELECT id, workspace_id, book_id, student_id, checked_out_at, due_date, returned_at, status, updated_at FROM library_lending_logs WHERE workspace_id = ?1").await?
                } else {
                    let place_holders = if student_ids.is_empty() {
                        "''".to_string()
                    } else {
                        student_ids.iter().map(|id| format!("'{}'", id.replace('\'', "''"))).collect::<Vec<_>>().join(",")
                    };
                    let query = format!(
                        "SELECT id, workspace_id, book_id, student_id, checked_out_at, due_date, returned_at, status, updated_at FROM library_lending_logs WHERE workspace_id = ?1 AND student_id IN ({})",
                        place_holders
                    );
                    conn.prepare(&query).await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let returned_at: Option<String> = row.get(6)?;
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "book_id": row.get::<String>(2)?,
                        "student_id": row.get::<String>(3)?,
                        "checked_out_at": row.get::<String>(4)?,
                        "due_date": row.get::<String>(5)?,
                        "returned_at": returned_at,
                        "status": row.get::<String>(7)?,
                        "updated_at": row.get::<i64>(8)?,
                    });
                    list.push(item);
                }
                list
            }
            "school_payments" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, invoice_id, amount, payment_method, paid_at, updated_at FROM school_payments").await?
                } else if is_privileged {
                    conn.prepare("SELECT id, workspace_id, invoice_id, amount, payment_method, paid_at, updated_at FROM school_payments WHERE workspace_id = ?1").await?
                } else {
                    let place_holders = if student_ids.is_empty() {
                        "''".to_string()
                    } else {
                        student_ids.iter().map(|id| format!("'{}'", id.replace('\'', "''"))).collect::<Vec<_>>().join(",")
                    };
                    let query = format!(
                        "SELECT id, workspace_id, invoice_id, amount, payment_method, paid_at, updated_at FROM school_payments WHERE workspace_id = ?1 AND invoice_id IN (SELECT id FROM school_invoices WHERE student_id IN ({}))",
                        place_holders
                    );
                    conn.prepare(&query).await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "invoice_id": row.get::<String>(2)?,
                        "amount": row.get::<f64>(3)?,
                        "payment_method": row.get::<String>(4)?,
                        "paid_at": row.get::<String>(5)?,
                        "updated_at": row.get::<i64>(6)?,
                    });
                    list.push(item);
                }
                list
            }
            "job_tickets" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, title, description, location_address, priority, status, assigned_user_id, scheduled_date, checklist_json, completion_report, created_at, updated_at, origin_address, destination_address, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, origin_parking_permit_needed, destination_parking_permit_needed, assigned_vehicle_id, route_stops_json, long_carry_meters, toll_fees FROM job_tickets").await?
                } else {
                    conn.prepare("SELECT id, workspace_id, title, description, location_address, priority, status, assigned_user_id, scheduled_date, checklist_json, completion_report, created_at, updated_at, origin_address, destination_address, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, origin_parking_permit_needed, destination_parking_permit_needed, assigned_vehicle_id, route_stops_json, long_carry_meters, toll_fees FROM job_tickets WHERE workspace_id = ?1").await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let assigned_user_id: Option<String> = row.get(7)?;
                    let completion_report: Option<String> = row.get(10)?;
                    let origin_address: Option<String> = row.get(13)?;
                    let destination_address: Option<String> = row.get(14)?;
                    let assigned_vehicle_id: Option<String> = row.get(21)?;
                    let route_stops_json: Option<String> = row.get(22)?;
                    
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "title": row.get::<String>(2)?,
                        "description": row.get::<String>(3)?,
                        "location_address": row.get::<String>(4)?,
                        "priority": row.get::<String>(5)?,
                        "status": row.get::<String>(6)?,
                        "assigned_user_id": assigned_user_id,
                        "scheduled_date": row.get::<String>(8)?,
                        "checklist_json": row.get::<String>(9)?,
                        "completion_report": completion_report,
                        "created_at": row.get::<String>(11)?,
                        "updated_at": row.get::<i64>(12)?,
                        "origin_address": origin_address,
                        "destination_address": destination_address,
                        "origin_floor": row.get::<i64>(15)?,
                        "destination_floor": row.get::<i64>(16)?,
                        "origin_has_elevator": row.get::<i64>(17)? != 0,
                        "destination_has_elevator": row.get::<i64>(18)? != 0,
                        "origin_parking_permit_needed": row.get::<i64>(19)? != 0,
                        "destination_parking_permit_needed": row.get::<i64>(20)? != 0,
                        "assigned_vehicle_id": assigned_vehicle_id,
                        "route_stops_json": route_stops_json,
                        "long_carry_meters": row.get::<i64>(23)?,
                        "toll_fees": row.get::<f64>(24)?,
                    });
                    list.push(item);
                }
                list
            }
            "move_inventory" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, handling_notes, updated_at FROM move_inventory").await?
                } else {
                    conn.prepare("SELECT id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, handling_notes, updated_at FROM move_inventory WHERE workspace_id = ?1").await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let handling_notes: Option<String> = row.get(7)?;
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "job_ticket_id": row.get::<String>(2)?,
                        "item_category": row.get::<String>(3)?,
                        "item_name": row.get::<String>(4)?,
                        "quantity": row.get::<i64>(5)?,
                        "estimated_volume_m3": row.get::<f64>(6)?,
                        "handling_notes": handling_notes,
                        "updated_at": row.get::<i64>(8)?,
                    });
                    list.push(item);
                }
                list
            }
            "move_quotes" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, accepted_at, updated_at, manual_price_override, price_discount FROM move_quotes").await?
                } else {
                    conn.prepare("SELECT id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, accepted_at, updated_at, manual_price_override, price_discount FROM move_quotes WHERE workspace_id = ?1").await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let accepted_at: Option<i64> = row.get(9)?;
                    let manual_price_override: Option<f64> = row.get(11)?;
                    let price_discount: Option<f64> = row.get(12)?;
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "job_ticket_id": row.get::<String>(2)?,
                        "base_price": row.get::<f64>(3)?,
                        "distance_fee": row.get::<f64>(4)?,
                        "stairs_surcharge": row.get::<f64>(5)?,
                        "packing_supplies_fee": row.get::<f64>(6)?,
                        "total_price": row.get::<f64>(7)?,
                        "status": row.get::<String>(8)?,
                        "accepted_at": accepted_at,
                        "updated_at": row.get::<i64>(10)?,
                        "manual_price_override": manual_price_override,
                        "price_discount": price_discount,
                    });
                    list.push(item);
                }
                list
            }
            "move_invoices" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, quote_id, customer_id, invoice_date, due_date, subtotal, rut_deduction, customer_amount, tax_authority_amount, status, updated_at, actual_hours, additional_charges, adjustment_notes FROM move_invoices").await?
                } else {
                    conn.prepare("SELECT id, workspace_id, quote_id, customer_id, invoice_date, due_date, subtotal, rut_deduction, customer_amount, tax_authority_amount, status, updated_at, actual_hours, additional_charges, adjustment_notes FROM move_invoices WHERE workspace_id = ?1").await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let actual_hours: Option<f64> = row.get(12)?;
                    let additional_charges: Option<f64> = row.get(13)?;
                    let adjustment_notes: Option<String> = row.get(14)?;
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "quote_id": row.get::<String>(2)?,
                        "customer_id": row.get::<String>(3)?,
                        "invoice_date": row.get::<String>(4)?,
                        "due_date": row.get::<String>(5)?,
                        "subtotal": row.get::<f64>(6)?,
                        "rut_deduction": row.get::<f64>(7)?,
                        "customer_amount": row.get::<f64>(8)?,
                        "tax_authority_amount": row.get::<f64>(9)?,
                        "status": row.get::<String>(10)?,
                        "updated_at": row.get::<i64>(11)?,
                        "actual_hours": actual_hours,
                        "additional_charges": additional_charges,
                        "adjustment_notes": adjustment_notes,
                    });
                    list.push(item);
                }
                list
            }
            "move_signatures" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, job_ticket_id, signer_name, signature_data_base64, signed_at FROM move_signatures").await?
                } else {
                    conn.prepare("SELECT id, workspace_id, job_ticket_id, signer_name, signature_data_base64, signed_at FROM move_signatures WHERE workspace_id = ?1").await?
                };

                let mut rows = if auth.role == "platform_admin" {
                    stmt.query(()).await?
                } else {
                    stmt.query(crate::params![&auth.workspace_id]).await?
                };

                let mut list = Vec::new();
                while let Some(row) = rows.next().await? {
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "job_ticket_id": row.get::<String>(2)?,
                        "signer_name": row.get::<String>(3)?,
                        "signature_data_base64": row.get::<String>(4)?,
                        "signed_at": row.get::<i64>(5)?,
                    });
                    list.push(item);
                }
                list
            }
            _ => {
                return Err(YntraError::DbError(format!(
                    "Sync partitioning is not supported for table '{}'",
                    table_name
                )));
            }
        };

        serde_json::to_string(&json_rows)
            .map_err(|e| YntraError::SerializationError(e.to_string()))
    }
}

async fn get_authorized_student_ids_helper(
    conn: &database::DbConnection,
    auth: &crate::AuthContext,
) -> Result<Vec<String>, YntraError> {
    let mut student_ids = Vec::new();
    let role_lower = auth.role.to_lowercase();
    if role_lower == "student" || role_lower == "role-school-student" {
        let mut stmt = conn
            .prepare("SELECT id FROM student_profiles WHERE user_id = ?1 AND workspace_id = ?2")
            .await?;
        let mut rows = stmt.query(crate::params![&auth.user_id, &auth.workspace_id]).await?;
        while let Some(row) = rows.next().await? {
            let id: String = row.get(0)?;
            student_ids.push(id);
        }
    } else if role_lower == "parent" || role_lower == "role-school-parent" {
        let mut stmt = conn
            .prepare("SELECT student_id FROM student_parents WHERE parent_user_id = ?1 AND workspace_id = ?2")
            .await?;
        let mut rows = stmt.query(crate::params![&auth.user_id, &auth.workspace_id]).await?;
        while let Some(row) = rows.next().await? {
            let student_id: String = row.get(0)?;
            student_ids.push(student_id);
        }
    }
    Ok(student_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;
    use crate::database::zero_copy::create_peer_store;

    #[tokio::test]
    async fn test_remote_sync_coordinator_verifications() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // 1. Setup mock workspace and user metadata containing public key
        let passkey_seed = "test-proxy-coordinator-seed".to_string();
        let trust = ZkCryptoTrust::new();
        let public_key_hex = trust.derive_public_key(passkey_seed.clone()).unwrap();

        let metadata = serde_json::json!({
            "public_key": public_key_hex
        }).to_string();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-proxy', 'Proxy WS', '[]', '{}')", ()).await.unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-proxy-tester', 'ws-proxy', 'proxy@test.com', 'Admin', ?1)",
            crate::params![&metadata],
        ).await.unwrap();

        let coordinator = RemoteSyncCoordinator::new();

        // 2. Generate a valid proof
        let valid_proof = trust.generate_role_proof(passkey_seed.clone(), "u-proxy-tester".to_string(), "Admin".to_string()).unwrap();

        // 3. Verify valid SQL write transaction executes successfully
        let sql = "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-proxy-updated', 'Updated Proxy WS', '[]', '{}')";
        let res = coordinator.verify_and_execute_write(
            "u-proxy-tester".to_string(),
            "Admin".to_string(),
            Some(valid_proof.clone()),
            sql.to_string(),
            "[]".to_string(),
        ).await;

        assert!(res.is_ok(), "Valid ZKP write transaction was rejected: {:?}", res.err());

        // 4. Verify invalid/tampered proof is rejected
        let invalid_proof = valid_proof.clone() + "tampered";
        let res_invalid = coordinator.verify_and_execute_write(
            "u-proxy-tester".to_string(),
            "Admin".to_string(),
            Some(invalid_proof),
            sql.to_string(),
            "[]".to_string(),
        ).await;

        assert!(res_invalid.is_err(), "Invalid/tampered proof was accepted");
        if let Err(e) = res_invalid {
            assert!(matches!(e, YntraError::CryptoError(_)));
        }

        // 5. Verify mismatched role is rejected
        let res_mismatched_role = coordinator.verify_and_execute_write(
            "u-proxy-tester".to_string(),
            "Moderator".to_string(), // Mismatched role
            Some(valid_proof),
            sql.to_string(),
            "[]".to_string(),
        ).await;

        assert!(res_mismatched_role.is_err(), "Mismatched role proof was accepted");

        // Cleanup
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-proxy'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-proxy'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-proxy-updated'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_remote_sync_coordinator_loro_verifications() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // 1. Setup mock workspace and user metadata containing public key
        let passkey_seed = "test-proxy-loro-seed".to_string();
        let trust = ZkCryptoTrust::new();
        let public_key_hex = trust.derive_public_key(passkey_seed.clone()).unwrap();

        let metadata = serde_json::json!({
            "public_key": public_key_hex
        }).to_string();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-proxy', 'Proxy WS', '[]', '{}')", ()).await.unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-proxy-tester', 'ws-proxy', 'proxy@test.com', 'Admin', ?1)",
            crate::params![&metadata],
        ).await.unwrap();

        let coordinator = RemoteSyncCoordinator::new();
        let store = Arc::new(create_peer_store("test-proxy-loro-store".to_string()).unwrap());

        // 2. Generate a valid Loro update (compliant with TodoItem schema)
        let doc_a = loro::LoroDoc::new();
        let db_map = doc_a.get_map("db");
        let m = db_map.insert_container("todo-1", loro::LoroMap::new()).unwrap();
        m.insert("id", uuid::Uuid::new_v4().to_string()).unwrap();
        m.insert("workspace_id", "ws-proxy".to_string()).unwrap();
        m.insert("text", "Valid Loro Todo Item".to_string()).unwrap();
        m.insert("completed", false).unwrap();
        m.insert("updated_at", 12345i64).unwrap();
        m.insert("sync_status", "pending".to_string()).unwrap();
        let update_bytes = doc_a.export(loro::ExportMode::Snapshot).unwrap();
        let update_hex = const_hex::encode(&update_bytes);

        // Generate valid compliance & role proofs
        let comp_proof = trust.generate_compliance_proof(
            passkey_seed.clone(),
            update_hex.clone(),
            "u-proxy-tester".to_string(),
            "Admin".to_string(),
        ).unwrap();

        let role_proof = trust.generate_role_proof(
            passkey_seed.clone(),
            "u-proxy-tester".to_string(),
            "Admin".to_string(),
        ).unwrap();

        // 3. Verify valid Loro sync passes verification
        let res = coordinator.verify_and_apply_loro_sync(
            "u-proxy-tester".to_string(),
            "Admin".to_string(),
            Some(role_proof.clone()),
            Some(comp_proof.clone()),
            update_hex.clone(),
            store.clone(),
        ).await;

        assert!(res.is_ok(), "Valid Loro sync was rejected: {:?}", res.err());

        // 4. Verify invalid Loro sync (violating schema constraints) is rejected
        let doc_b = loro::LoroDoc::new();
        let db_map_b = doc_b.get_map("db");
        let m = db_map_b.insert_container("todo-2", loro::LoroMap::new()).unwrap();
        m.insert("id", "invalid-uuid-format".to_string()).unwrap(); // Violates UUID schema
        m.insert("workspace_id", "ws-proxy".to_string()).unwrap();
        m.insert("text", "Invalid Loro Todo Item".to_string()).unwrap();
        m.insert("completed", false).unwrap();
        m.insert("updated_at", 12345i64).unwrap();
        m.insert("sync_status", "pending".to_string()).unwrap();
        let bad_update_bytes = doc_b.export(loro::ExportMode::Snapshot).unwrap();
        let bad_update_hex = const_hex::encode(&bad_update_bytes);

        let bad_comp_proof = trust.generate_compliance_proof(
            passkey_seed.clone(),
            bad_update_hex.clone(),
            "u-proxy-tester".to_string(),
            "Admin".to_string(),
        ).unwrap();

        let res_bad = coordinator.verify_and_apply_loro_sync(
            "u-proxy-tester".to_string(),
            "Admin".to_string(),
            Some(role_proof.clone()),
            Some(bad_comp_proof.clone()),
            bad_update_hex.clone(),
            store.clone(),
        ).await;

        assert!(res_bad.is_err(), "Invalid schema sync was accepted");

        // Cleanup
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-proxy'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-proxy'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_remote_sync_coordinator_row_level_partitioning() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let trust = ZkCryptoTrust::new();

        // 1. Setup passkey seeds, public keys and user metadata
        let passkey_admin = "seed-partition-admin".to_string();
        let passkey_student = "seed-partition-student".to_string();
        let passkey_parent = "seed-partition-parent".to_string();

        let pk_admin = trust.derive_public_key(passkey_admin.clone()).unwrap();
        let pk_student = trust.derive_public_key(passkey_student.clone()).unwrap();
        let pk_parent = trust.derive_public_key(passkey_parent.clone()).unwrap();

        let meta_admin = serde_json::json!({ "public_key": pk_admin }).to_string();
        let meta_student = serde_json::json!({ "public_key": pk_student }).to_string();
        let meta_parent = serde_json::json!({ "public_key": pk_parent }).to_string();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-partition-test', 'Partition WS', '[]', '{}')", ()).await.unwrap();
        
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-admin-p', 'ws-partition-test', 'admin@part.com', 'admin', ?1)", crate::params![&meta_admin]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-student-p', 'ws-partition-test', 'stud@part.com', 'student', ?1)", crate::params![&meta_student]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-parent-p', 'ws-partition-test', 'parent@part.com', 'parent', ?1)", crate::params![&meta_parent]).await.unwrap();

        // 2. Insert student profiles
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, parent_contact, updated_at) VALUES ('stud-p-1', 'ws-partition-test', 'u-student-p', 'Jane', 'Doe', '10A', 'parent@doe.com', 0)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, parent_contact, updated_at) VALUES ('stud-p-2', 'ws-partition-test', NULL, 'Alex', 'Smith', '10B', 'parent@smith.com', 0)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, parent_contact, updated_at) VALUES ('stud-p-3', 'ws-partition-test', NULL, 'Charlie', 'Brown', '10C', 'parent@brown.com', 0)", ()).await.unwrap();

        // Link parent u-parent-p to student stud-p-2
        conn.execute("INSERT OR REPLACE INTO student_parents (student_id, parent_user_id, workspace_id) VALUES ('stud-p-2', 'u-parent-p', 'ws-partition-test')", ()).await.unwrap();

        // Insert required course to satisfy FOREIGN KEY checks
        conn.execute("INSERT OR REPLACE INTO courses (id, name, subject, classroom, workspace_id, updated_at) VALUES ('crs-p', 'Math', 'Math', 'Room A', 'ws-partition-test', 0)", ()).await.unwrap();

        // Insert term grades
        conn.execute("INSERT OR REPLACE INTO term_grades (id, workspace_id, student_id, course_id, term_name, final_grade, final_points, teacher_comments, updated_at) VALUES ('tg-p-1', 'ws-partition-test', 'stud-p-1', 'crs-p', 'Fall 2026', 'A', 95, '', 0)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO term_grades (id, workspace_id, student_id, course_id, term_name, final_grade, final_points, teacher_comments, updated_at) VALUES ('tg-p-2', 'ws-partition-test', 'stud-p-2', 'crs-p', 'Fall 2026', 'B', 85, '', 0)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO term_grades (id, workspace_id, student_id, course_id, term_name, final_grade, final_points, teacher_comments, updated_at) VALUES ('tg-p-3', 'ws-partition-test', 'stud-p-3', 'crs-p', 'Fall 2026', 'C', 75, '', 0)", ()).await.unwrap();

        let coordinator = RemoteSyncCoordinator::new();

        // 3. Generate ZK proofs
        let proof_admin = trust.generate_role_proof(passkey_admin.clone(), "u-admin-p".to_string(), "admin".to_string()).unwrap();
        let proof_student = trust.generate_role_proof(passkey_student.clone(), "u-student-p".to_string(), "student".to_string()).unwrap();
        let proof_parent = trust.generate_role_proof(passkey_parent.clone(), "u-parent-p".to_string(), "parent".to_string()).unwrap();

        // 4. Verify Admin receives all rows
        let payload_admin = coordinator.generate_partitioned_sync_payload("u-admin-p".to_string(), "admin".to_string(), Some(proof_admin), "term_grades".to_string()).await.unwrap();
        let json_admin: serde_json::Value = serde_json::from_str(&payload_admin).unwrap();
        let arr_admin = json_admin.as_array().unwrap();
        assert_eq!(arr_admin.len(), 3);

        // 5. Verify Student receives only their own row
        let payload_student = coordinator.generate_partitioned_sync_payload("u-student-p".to_string(), "student".to_string(), Some(proof_student), "term_grades".to_string()).await.unwrap();
        let json_student: serde_json::Value = serde_json::from_str(&payload_student).unwrap();
        let arr_student = json_student.as_array().unwrap();
        assert_eq!(arr_student.len(), 1);
        assert_eq!(arr_student[0]["student_id"].as_str().unwrap(), "stud-p-1");
        assert_eq!(arr_student[0]["final_grade"].as_str().unwrap(), "A");

        // 6. Verify Parent receives only their child's row
        let payload_parent = coordinator.generate_partitioned_sync_payload("u-parent-p".to_string(), "parent".to_string(), Some(proof_parent.clone()), "term_grades".to_string()).await.unwrap();
        let json_parent: serde_json::Value = serde_json::from_str(&payload_parent).unwrap();
        let arr_parent = json_parent.as_array().unwrap();
        assert_eq!(arr_parent.len(), 1);
        assert_eq!(arr_parent[0]["student_id"].as_str().unwrap(), "stud-p-2");
        assert_eq!(arr_parent[0]["final_grade"].as_str().unwrap(), "B");

        // 7. Verify invalid ZK proof is rejected
        let bad_proof = proof_parent + "invalid";
        let res_bad = coordinator.generate_partitioned_sync_payload("u-parent-p".to_string(), "parent".to_string(), Some(bad_proof), "term_grades".to_string()).await;
        assert!(res_bad.is_err(), "Access should be blocked under invalid ZK proof");

        // Cleanup
        conn.execute("DELETE FROM term_grades WHERE workspace_id = 'ws-partition-test'", ()).await.unwrap();
        conn.execute("DELETE FROM courses WHERE workspace_id = 'ws-partition-test'", ()).await.unwrap();
        conn.execute("DELETE FROM student_parents WHERE workspace_id = 'ws-partition-test'", ()).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-partition-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-partition-test'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-partition-test'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_remote_sync_coordinator_write_rbac() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let trust = ZkCryptoTrust::new();

        // 1. Setup passkey seeds, public keys and user metadata
        let passkey_student = "seed-write-rbac-student".to_string();
        let passkey_parent = "seed-write-rbac-parent".to_string();

        let pk_student = trust.derive_public_key(passkey_student.clone()).unwrap();
        let pk_parent = trust.derive_public_key(passkey_parent.clone()).unwrap();

        let meta_student = serde_json::json!({ "public_key": pk_student }).to_string();
        let meta_parent = serde_json::json!({ "public_key": pk_parent }).to_string();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-rbac-test', 'RBAC WS', '[]', '{}')", ()).await.unwrap();
        
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-rbac-student', 'ws-rbac-test', 'stud@rbac.com', 'student', ?1)", crate::params![&meta_student]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-rbac-parent', 'ws-rbac-test', 'parent@rbac.com', 'parent', ?1)", crate::params![&meta_parent]).await.unwrap();

        // Setup student profiles
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, updated_at) VALUES ('stud-rbac-1', 'ws-rbac-test', 'u-rbac-student', 'Alice', 'Smith', '10A', 0)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, updated_at) VALUES ('stud-rbac-2', 'ws-rbac-test', NULL, 'Bob', 'Jones', '10B', 0)", ()).await.unwrap();

        // Link parent to stud-rbac-1
        conn.execute("INSERT OR REPLACE INTO student_parents (student_id, parent_user_id, workspace_id) VALUES ('stud-rbac-1', 'u-rbac-parent', 'ws-rbac-test')", ()).await.unwrap();

        // Insert library books
        conn.execute("INSERT OR REPLACE INTO library_books (id, workspace_id, title, author, isbn, copies_available, total_copies, updated_at) VALUES ('book-1', 'ws-rbac-test', 'Book A', 'Author A', '123456', 1, 1, 0)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO library_books (id, workspace_id, title, author, isbn, copies_available, total_copies, updated_at) VALUES ('book-2', 'ws-rbac-test', 'Book B', 'Author B', '789012', 1, 1, 0)", ()).await.unwrap();

        // Insert course and assignment
        conn.execute("INSERT OR REPLACE INTO courses (id, name, subject, classroom, workspace_id, updated_at) VALUES ('crs-rbac-1', 'Course A', 'Subj A', 'Room A', 'ws-rbac-test', 0)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO assignments (id, workspace_id, course_id, title, description, max_points, due_date, updated_at) VALUES ('assign-1', 'ws-rbac-test', 'crs-rbac-1', 'Assignment 1', 'Desc', 100, '2026-07-31', 0)", ()).await.unwrap();

        // Insert library log
        conn.execute("INSERT OR REPLACE INTO library_lending_logs (id, workspace_id, book_id, student_id, checked_out_at, due_date, status, updated_at) VALUES ('log-rbac-1', 'ws-rbac-test', 'book-1', 'stud-rbac-1', '2026-07-01', '2026-07-15', 'borrowed', 0)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO library_lending_logs (id, workspace_id, book_id, student_id, checked_out_at, due_date, status, updated_at) VALUES ('log-rbac-2', 'ws-rbac-test', 'book-2', 'stud-rbac-2', '2026-07-01', '2026-07-15', 'borrowed', 0)", ()).await.unwrap();

        let coordinator = RemoteSyncCoordinator::new();
        let proof_student = trust.generate_role_proof(passkey_student.clone(), "u-rbac-student".to_string(), "student".to_string()).unwrap();
        let proof_parent = trust.generate_role_proof(passkey_parent.clone(), "u-rbac-parent".to_string(), "parent".to_string()).unwrap();

        // A. Verify student writing to courses is rejected
        let res_course = coordinator.verify_and_execute_write(
            "u-rbac-student".to_string(),
            "student".to_string(),
            Some(proof_student.clone()),
            "INSERT INTO courses (id, name, subject, classroom, workspace_id, updated_at) VALUES ('c1', 'A', 'B', 'C', 'ws-rbac-test', 0)".to_string(),
            "[]".to_string()
        ).await;
        assert!(res_course.is_err(), "Student allowed to write to courses");
        assert!(res_course.err().unwrap().to_string().contains("does not have write permissions"), "Mismatched error message");

        // B. Verify student submitting with mismatched student_id is rejected
        let res_sub_mismatched = coordinator.verify_and_execute_write(
            "u-rbac-student".to_string(),
            "student".to_string(),
            Some(proof_student.clone()),
            "INSERT OR REPLACE INTO submissions (id, workspace_id, assignment_id, student_id, content, grade, feedback, submitted_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)".to_string(),
            "[\"sub-1\", \"ws-rbac-test\", \"assign-1\", \"stud-rbac-2\", \"content\", null, null, \"2026-07-20\", 0]".to_string()
        ).await;
        assert!(res_sub_mismatched.is_err(), "Student allowed to write other student's submission");

        // C. Verify student submitting with grade set is rejected
        let res_sub_grade = coordinator.verify_and_execute_write(
            "u-rbac-student".to_string(),
            "student".to_string(),
            Some(proof_student.clone()),
            "INSERT OR REPLACE INTO submissions (id, workspace_id, assignment_id, student_id, content, grade, feedback, submitted_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)".to_string(),
            "[\"sub-2\", \"ws-rbac-test\", \"assign-1\", \"stud-rbac-1\", \"content\", \"A\", null, \"2026-07-20\", 0]".to_string()
        ).await;
        assert!(res_sub_grade.is_err(), "Student allowed to write submission with grade");

        // D. Verify student submitting with valid owned profile & null grade/feedback is allowed
        let res_sub_ok = coordinator.verify_and_execute_write(
            "u-rbac-student".to_string(),
            "student".to_string(),
            Some(proof_student.clone()),
            "INSERT OR REPLACE INTO submissions (id, workspace_id, assignment_id, student_id, content, grade, feedback, submitted_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)".to_string(),
            "[\"sub-ok-1\", \"ws-rbac-test\", \"assign-1\", \"stud-rbac-1\", \"my answers\", null, null, \"2026-07-20\", 0]".to_string()
        ).await;
        assert!(res_sub_ok.is_ok(), "Student valid submission rejected: {:?}", res_sub_ok.err());

        // E. Verify student renewing their own book log is allowed
        let res_renew_student = coordinator.verify_and_execute_write(
            "u-rbac-student".to_string(),
            "student".to_string(),
            Some(proof_student.clone()),
            "UPDATE library_lending_logs SET due_date = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3".to_string(),
            "[\"2026-08-01\", 12345, \"log-rbac-1\"]".to_string()
        ).await;
        assert!(res_renew_student.is_ok(), "Student renewing own book rejected: {:?}", res_renew_student.err());

        // F. Verify student renewing someone else's book log is rejected
        let res_renew_bad = coordinator.verify_and_execute_write(
            "u-rbac-student".to_string(),
            "student".to_string(),
            Some(proof_student.clone()),
            "UPDATE library_lending_logs SET due_date = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3".to_string(),
            "[\"2026-08-01\", 12345, \"log-rbac-2\"]".to_string()
        ).await;
        assert!(res_renew_bad.is_err(), "Student allowed to renew other's book");

        // G. Verify parent renewing linked student's book is allowed
        let res_renew_parent_ok = coordinator.verify_and_execute_write(
            "u-rbac-parent".to_string(),
            "parent".to_string(),
            Some(proof_parent.clone()),
            "UPDATE library_lending_logs SET due_date = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3".to_string(),
            "[\"2026-08-01\", 12345, \"log-rbac-1\"]".to_string()
        ).await;
        assert!(res_renew_parent_ok.is_ok(), "Parent renewing child's book rejected: {:?}", res_renew_parent_ok.err());

        // H. Verify parent renewing mismatched student's book is rejected
        let res_renew_parent_bad = coordinator.verify_and_execute_write(
            "u-rbac-parent".to_string(),
            "parent".to_string(),
            Some(proof_parent.clone()),
            "UPDATE library_lending_logs SET due_date = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3".to_string(),
            "[\"2026-08-01\", 12345, \"log-rbac-2\"]".to_string()
        ).await;
        assert!(res_renew_parent_bad.is_err(), "Parent allowed to renew unlinked book");

        // Cleanup
        conn.execute("DELETE FROM library_lending_logs WHERE workspace_id = 'ws-rbac-test'", ()).await.unwrap();
        conn.execute("DELETE FROM submissions WHERE workspace_id = 'ws-rbac-test'", ()).await.unwrap();
        conn.execute("DELETE FROM student_parents WHERE workspace_id = 'ws-rbac-test'", ()).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-rbac-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-rbac-test'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-rbac-test'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_remote_sync_coordinator_clock_skew() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let trust = ZkCryptoTrust::new();

        // 1. Setup passkey seed, public key and user metadata
        let passkey_student = "seed-skew-student".to_string();
        let pk_student = trust.derive_public_key(passkey_student.clone()).unwrap();
        let meta_student = serde_json::json!({ "public_key": pk_student }).to_string();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-skew-test', 'Skew WS', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-skew-student', 'ws-skew-test', 'stud@skew.com', 'student', ?1)", crate::params![&meta_student]).await.unwrap();

        // Setup student profile
        conn.execute("INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, updated_at) VALUES ('stud-skew-1', 'ws-skew-test', 'u-skew-student', 'Alice', 'Smith', '10A', 0)", ()).await.unwrap();

        // Insert course and assignment
        conn.execute("INSERT OR REPLACE INTO courses (id, name, subject, classroom, workspace_id, updated_at) VALUES ('crs-skew-1', 'Course A', 'Subj A', 'Room A', 'ws-skew-test', 0)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO assignments (id, workspace_id, course_id, title, description, max_points, due_date, updated_at) VALUES ('assign-1', 'ws-skew-test', 'crs-skew-1', 'Assignment 1', 'Desc', 100, '2026-07-31', 0)", ()).await.unwrap();

        let coordinator = RemoteSyncCoordinator::new();
        let proof_student = trust.generate_role_proof(passkey_student.clone(), "u-skew-student".to_string(), "student".to_string()).unwrap();

        // 2. Perform write with future updated_at timestamp (year 2030, ~1893456000000)
        let future_time = 1893456000000i64;
        let res = coordinator.verify_and_execute_write(
            "u-skew-student".to_string(),
            "student".to_string(),
            Some(proof_student.clone()),
            "INSERT OR REPLACE INTO submissions (id, workspace_id, assignment_id, student_id, content, grade, feedback, submitted_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)".to_string(),
            format!("[\"sub-skew-1\", \"ws-skew-test\", \"assign-1\", \"stud-skew-1\", \"some answers\", null, null, \"2026-07-20\", {}]", future_time)
        ).await;
        assert!(res.is_ok(), "Write with clock skew rejected: {:?}", res.err());

        // 3. Query the inserted record to verify that updated_at was normalized (i.e. is not equal to future_time)
        let inserted_updated_at: i64 = conn.query_row(
            "SELECT updated_at FROM submissions WHERE id = 'sub-skew-1'",
            (),
            |r| r.get(0)
        ).await.unwrap();

        assert!(inserted_updated_at < future_time, "Clock skew was not normalized on the server side: {} vs {}", inserted_updated_at, future_time);
        
        let now_ms = crate::infra::time::get_current_time_ms();
        assert!(inserted_updated_at <= now_ms + 1000 && inserted_updated_at >= now_ms - 5000, "Clock skew was not normalized to current server time: {}", inserted_updated_at);

        // Cleanup
        conn.execute("DELETE FROM submissions WHERE workspace_id = 'ws-skew-test'", ()).await.unwrap();
        conn.execute("DELETE FROM assignments WHERE workspace_id = 'ws-skew-test'", ()).await.unwrap();
        conn.execute("DELETE FROM courses WHERE workspace_id = 'ws-skew-test'", ()).await.unwrap();
        conn.execute("DELETE FROM student_profiles WHERE workspace_id = 'ws-skew-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-skew-test'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-skew-test'", ()).await.unwrap();
    }

    #[tokio::test]
    async fn test_remote_sync_coordinator_partitioning_moving() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let trust = ZkCryptoTrust::new();
        let passkey_admin = "seed-partition-moving-admin".to_string();
        let pk_admin = trust.derive_public_key(passkey_admin.clone()).unwrap();
        let meta_admin = serde_json::json!({ "public_key": pk_admin }).to_string();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-moving-test', 'Moving WS', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-admin-m', 'ws-moving-test', 'admin@moving.com', 'admin', ?1)", crate::params![&meta_admin]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('cust-1', 'ws-moving-test', 'cust@moving.com', 'client')", ()).await.unwrap();

        // Insert job tickets
        conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at) VALUES ('job-1', 'ws-moving-test', 'Title 1', 'Desc 1', 'Addr 1', 'high', 'pending', '2026-07-21', '{}', '2026-07-21', 0)", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at) VALUES ('job-2', 'ws-moving-test', 'Title 2', 'Desc 2', 'Addr 2', 'medium', 'pending', '2026-07-21', '{}', '2026-07-21', 0)", ()).await.unwrap();

        // Insert move inventory
        conn.execute("INSERT OR REPLACE INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, updated_at) VALUES ('inv-1', 'ws-moving-test', 'job-1', 'Boxes', 'Small Box', 5, 0.5, 0)", ()).await.unwrap();

        // Insert move quotes
        conn.execute("INSERT OR REPLACE INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at) VALUES ('quote-1', 'ws-moving-test', 'job-1', 1000.0, 100.0, 50.0, 20.0, 1170.0, 'pending', 0)", ()).await.unwrap();

        // Insert move invoices
        conn.execute("INSERT OR REPLACE INTO move_invoices (id, workspace_id, quote_id, customer_id, invoice_date, due_date, subtotal, rut_deduction, customer_amount, tax_authority_amount, status, updated_at) VALUES ('invc-1', 'ws-moving-test', 'quote-1', 'cust-1', '2026-07-21', '2026-08-21', 1170.0, 0.0, 1170.0, 0.0, 'unpaid', 0)", ()).await.unwrap();

        // Insert move signatures
        conn.execute("INSERT OR REPLACE INTO move_signatures (id, workspace_id, job_ticket_id, signer_name, signature_data_base64, signed_at) VALUES ('sig-1', 'ws-moving-test', 'job-1', 'John Doe', 'base64-data', 0)", ()).await.unwrap();

        let coordinator = RemoteSyncCoordinator::new();
        let proof_admin = trust.generate_role_proof(passkey_admin.clone(), "u-admin-m".to_string(), "admin".to_string()).unwrap();

        // Verify payload lengths
        let payload_jobs = coordinator.generate_partitioned_sync_payload("u-admin-m".to_string(), "admin".to_string(), Some(proof_admin.clone()), "job_tickets".to_string()).await.unwrap();
        let arr_jobs: serde_json::Value = serde_json::from_str(&payload_jobs).unwrap();
        assert_eq!(arr_jobs.as_array().unwrap().len(), 2);

        let payload_inv = coordinator.generate_partitioned_sync_payload("u-admin-m".to_string(), "admin".to_string(), Some(proof_admin.clone()), "move_inventory".to_string()).await.unwrap();
        let arr_inv: serde_json::Value = serde_json::from_str(&payload_inv).unwrap();
        assert_eq!(arr_inv.as_array().unwrap().len(), 1);

        let payload_quotes = coordinator.generate_partitioned_sync_payload("u-admin-m".to_string(), "admin".to_string(), Some(proof_admin.clone()), "move_quotes".to_string()).await.unwrap();
        let arr_quotes: serde_json::Value = serde_json::from_str(&payload_quotes).unwrap();
        assert_eq!(arr_quotes.as_array().unwrap().len(), 1);

        let payload_invoices = coordinator.generate_partitioned_sync_payload("u-admin-m".to_string(), "admin".to_string(), Some(proof_admin.clone()), "move_invoices".to_string()).await.unwrap();
        let arr_invoices: serde_json::Value = serde_json::from_str(&payload_invoices).unwrap();
        assert_eq!(arr_invoices.as_array().unwrap().len(), 1);

        let payload_signatures = coordinator.generate_partitioned_sync_payload("u-admin-m".to_string(), "admin".to_string(), Some(proof_admin.clone()), "move_signatures".to_string()).await.unwrap();
        let arr_signatures: serde_json::Value = serde_json::from_str(&payload_signatures).unwrap();
        assert_eq!(arr_signatures.as_array().unwrap().len(), 1);

        // Cleanup
        conn.execute("DELETE FROM move_signatures WHERE workspace_id = 'ws-moving-test'", ()).await.ok();
        conn.execute("DELETE FROM move_invoices WHERE workspace_id = 'ws-moving-test'", ()).await.ok();
        conn.execute("DELETE FROM move_quotes WHERE workspace_id = 'ws-moving-test'", ()).await.ok();
        conn.execute("DELETE FROM move_inventory WHERE workspace_id = 'ws-moving-test'", ()).await.ok();
        conn.execute("DELETE FROM job_packaging_items WHERE workspace_id = 'ws-moving-test'", ()).await.ok();
        conn.execute("DELETE FROM job_crew WHERE job_ticket_id IN ('job-1', 'job-2')", ()).await.ok();
        conn.execute("DELETE FROM time_reports WHERE workspace_id = 'ws-moving-test'", ()).await.ok();
        conn.execute("DELETE FROM job_tickets WHERE workspace_id = 'ws-moving-test'", ()).await.ok();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-moving-test'", ()).await.ok();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-moving-test'", ()).await.ok();
    }
}
