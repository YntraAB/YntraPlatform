use crate::database;
use crate::{YntraError, ZkCryptoTrust, ZeroCopyStore};
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

            let trust = ZkCryptoTrust::new();
            if !trust.verify_proof(
                proof,
                requester_user_id.clone(),
                role.clone(),
                public_key_hex,
            ) {
                return Err(YntraError::CryptoError(
                    "Zero-Knowledge Role Proof verification failed: privilege escalation or local database tampering suspected".to_string(),
                ));
            }
        }

        // 3. Parse JSON params and execute inside a transaction for atomic safety
        let parsed_params: Vec<serde_json::Value> = if params_json.is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&params_json)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?
        };

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
}
