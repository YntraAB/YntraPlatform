use super::clock_skew::normalize_clock_skew;
use super::sql_helpers::{
    extract_public_key_from_metadata, parse_insert_columns_and_values,
};
use crate::database;
use crate::{YntraError, ZeroCopyStore, ZkCryptoTrust};
use std::sync::Arc;

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

        let public_key_hex = metadata_str
            .as_deref()
            .map(extract_public_key_from_metadata)
            .unwrap_or_default();

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
                YntraError::AuthError(
                    "Zero-Knowledge Role Proof is required for write operations".to_string(),
                )
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
                trust.verify_proof(
                    proof,
                    requester_user_id.clone(),
                    role.clone(),
                    public_key_hex.clone(),
                )
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
        let is_unprivileged = role_lower == "student"
            || role_lower == "role-school-student"
            || role_lower == "parent"
            || role_lower == "role-school-parent";

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
                            "Access denied: Students and Parents cannot set or modify grades"
                                .to_string(),
                        ));
                    }
                }
                if let Some(feedback) = col_vals.get("feedback") {
                    if !feedback.is_null() && feedback.as_str() != Some("") {
                        return Err(YntraError::AuthError(
                            "Access denied: Students and Parents cannot set or modify feedback"
                                .to_string(),
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
                                "Access denied: You are not authorized to submit for this student"
                                    .to_string(),
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
                    let log_id = parsed_params.iter().find(|v| {
                        v.is_string()
                            && (v.as_str().unwrap().starts_with("log-")
                                || v.as_str().unwrap().starts_with("lend-"))
                    });
                    if let Some(log_id_val) = log_id {
                        let log_id_str = log_id_val.as_str().unwrap();
                        let student_id: Option<String> = conn
                            .query_row(
                                "SELECT student_id FROM library_lending_logs WHERE id = ?1",
                                crate::params![log_id_str],
                                |r| r.get(0),
                            )
                            .await
                            .ok();

                        if let Some(sid) = student_id {
                            let mut authorized = false;
                            if role_lower == "student" || role_lower == "role-school-student" {
                                let profile_uid: Option<String> = conn
                                    .query_row(
                                        "SELECT user_id FROM student_profiles WHERE id = ?1",
                                        crate::params![&sid],
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
                            return Err(YntraError::NotFoundError(
                                "Lending log not found".to_string(),
                            ));
                        }
                    } else {
                        return Err(YntraError::ValidationError(
                            "Missing library log ID".to_string(),
                        ));
                    }
                } else {
                    return Err(YntraError::AuthError(
                        "Access denied: Students/Parents can only update library logs".to_string(),
                    ));
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

        let public_key_hex = metadata_str
            .as_deref()
            .map(extract_public_key_from_metadata)
            .unwrap_or_default();

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
                YntraError::AuthError(
                    "Zero-Knowledge Role Proof is required for write operations".to_string(),
                )
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
        store
            .apply_loro_update(update_bytes)
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

        let public_key_hex = metadata_str
            .as_deref()
            .map(extract_public_key_from_metadata)
            .unwrap_or_default();

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
                YntraError::AuthError(
                    "Zero-Knowledge Role Proof is required for sync operations".to_string(),
                )
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
                        student_ids
                            .iter()
                            .map(|id| format!("'{}'", id.replace('\'', "''")))
                            .collect::<Vec<_>>()
                            .join(",")
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
                    stmt.query(crate::params![&auth.workspace_id, &auth.user_id])
                        .await?
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
                        student_ids
                            .iter()
                            .map(|id| format!("'{}'", id.replace('\'', "''")))
                            .collect::<Vec<_>>()
                            .join(",")
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
                        student_ids
                            .iter()
                            .map(|id| format!("'{}'", id.replace('\'', "''")))
                            .collect::<Vec<_>>()
                            .join(",")
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
                        student_ids
                            .iter()
                            .map(|id| format!("'{}'", id.replace('\'', "''")))
                            .collect::<Vec<_>>()
                            .join(",")
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
                        student_ids
                            .iter()
                            .map(|id| format!("'{}'", id.replace('\'', "''")))
                            .collect::<Vec<_>>()
                            .join(",")
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
                        student_ids
                            .iter()
                            .map(|id| format!("'{}'", id.replace('\'', "''")))
                            .collect::<Vec<_>>()
                            .join(",")
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
                        student_ids
                            .iter()
                            .map(|id| format!("'{}'", id.replace('\'', "''")))
                            .collect::<Vec<_>>()
                            .join(",")
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
                        student_ids
                            .iter()
                            .map(|id| format!("'{}'", id.replace('\'', "''")))
                            .collect::<Vec<_>>()
                            .join(",")
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
                    let gpa: Option<f64> = row.get(4)?;
                    let principal_comments: Option<String> = row.get(5)?;
                    let item = serde_json::json!({
                        "id": row.get::<String>(0)?,
                        "workspace_id": row.get::<String>(1)?,
                        "student_id": row.get::<String>(2)?,
                        "term_name": row.get::<String>(3)?,
                        "gpa": gpa,
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
                        student_ids
                            .iter()
                            .map(|id| format!("'{}'", id.replace('\'', "''")))
                            .collect::<Vec<_>>()
                            .join(",")
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
                        student_ids
                            .iter()
                            .map(|id| format!("'{}'", id.replace('\'', "''")))
                            .collect::<Vec<_>>()
                            .join(",")
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
            "job_crew" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, job_ticket_id, user_id, role, assigned_at FROM job_crew").await?
                } else {
                    conn.prepare("SELECT id, job_ticket_id, user_id, role, assigned_at FROM job_crew WHERE job_ticket_id IN (SELECT id FROM job_tickets WHERE workspace_id = ?1)").await?
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
                        "job_ticket_id": row.get::<String>(1)?,
                        "user_id": row.get::<String>(2)?,
                        "role": row.get::<String>(3)?,
                        "assigned_at": row.get::<i64>(4)?,
                    });
                    list.push(item);
                }
                list
            }
            "move_inventory" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, COALESCE(handling_notes, ''), COALESCE(room_name, ''), updated_at FROM move_inventory").await?
                } else {
                    conn.prepare("SELECT id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, COALESCE(handling_notes, ''), COALESCE(room_name, ''), updated_at FROM move_inventory WHERE workspace_id = ?1").await?
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
                        "item_category": row.get::<String>(3)?,
                        "item_name": row.get::<String>(4)?,
                        "quantity": row.get::<i64>(5)?,
                        "estimated_volume_m3": row.get::<f64>(6)?,
                        "handling_notes": row.get::<String>(7)?,
                        "room_name": row.get::<String>(8)?,
                        "updated_at": row.get::<i64>(9)?,
                    });
                    list.push(item);
                }
                list
            }
            "move_quotes" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at FROM move_quotes").await?
                } else {
                    conn.prepare("SELECT id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at FROM move_quotes WHERE workspace_id = ?1").await?
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
                        "base_price": row.get::<f64>(3)?,
                        "distance_fee": row.get::<f64>(4)?,
                        "stairs_surcharge": row.get::<f64>(5)?,
                        "packing_supplies_fee": row.get::<f64>(6)?,
                        "total_price": row.get::<f64>(7)?,
                        "status": row.get::<String>(8)?,
                        "updated_at": row.get::<i64>(9)?,
                    });
                    list.push(item);
                }
                list
            }
            "move_invoices" => {
                let mut stmt = if auth.role == "platform_admin" {
                    conn.prepare("SELECT id, workspace_id, quote_id, customer_id, invoice_date, due_date, subtotal, rut_deduction, customer_amount, tax_authority_amount, status, updated_at FROM move_invoices").await?
                } else {
                    conn.prepare("SELECT id, workspace_id, quote_id, customer_id, invoice_date, due_date, subtotal, rut_deduction, customer_amount, tax_authority_amount, status, updated_at FROM move_invoices WHERE workspace_id = ?1").await?
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
                return Err(YntraError::ValidationError(format!(
                    "Table '{}' is not enabled for row-level sync partitioning",
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
        let mut rows = stmt
            .query(crate::params![&auth.user_id, &auth.workspace_id])
            .await?;
        while let Some(row) = rows.next().await? {
            let id: String = row.get(0)?;
            student_ids.push(id);
        }
    } else if role_lower == "parent" || role_lower == "role-school-parent" {
        let mut stmt = conn
            .prepare("SELECT student_id FROM student_parents WHERE parent_user_id = ?1 AND workspace_id = ?2")
            .await?;
        let mut rows = stmt
            .query(crate::params![&auth.user_id, &auth.workspace_id])
            .await?;
        while let Some(row) = rows.next().await? {
            let student_id: String = row.get(0)?;
            student_ids.push(student_id);
        }
    }
    Ok(student_ids)
}
