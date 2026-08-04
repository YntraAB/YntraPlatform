use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;
use crate::services::notes::verify_zkp_if_encrypted;
use crate::services::school::auth::{
    verify_school_permission, verify_school_write_zkp, verify_student_access,
};
use crate::services::school::conflicts::record_school_conflict;
use crate::{StudentProfile, WorkspaceUser};

#[uniffi::export]
pub async fn get_student_profiles(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<StudentProfile>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let role_lower = auth.role.to_lowercase();
    let query_str = if role_lower == "student" || role_lower == "role-school-student" {
        "SELECT id, workspace_id, user_id, first_name, last_name, grade_level, parent_contact, updated_at FROM student_profiles WHERE workspace_id = ?1 AND user_id = ?2"
    } else if role_lower == "parent" || role_lower == "role-school-parent" {
        "SELECT s.id, s.workspace_id, s.user_id, s.first_name, s.last_name, s.grade_level, s.parent_contact, s.updated_at FROM student_profiles s JOIN student_parents sp ON s.id = sp.student_id WHERE s.workspace_id = ?1 AND sp.parent_user_id = ?2"
    } else {
        "SELECT id, workspace_id, user_id, first_name, last_name, grade_level, parent_contact, updated_at FROM student_profiles WHERE workspace_id = ?1"
    };

    let mut stmt = conn.prepare(query_str).await?;

    let list = if role_lower == "student" || role_lower == "role-school-student" {
        stmt.query_map(crate::params![workspace_id, &auth.user_id], |row| {
            Ok(StudentProfile {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                user_id: row.get(2)?,
                first_name: row.get(3)?,
                last_name: row.get(4)?,
                grade_level: row.get(5)?,
                parent_contact: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .await?
    } else if role_lower == "parent" || role_lower == "role-school-parent" {
        stmt.query_map(crate::params![workspace_id, &auth.user_id], |row| {
            Ok(StudentProfile {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                user_id: row.get(2)?,
                first_name: row.get(3)?,
                last_name: row.get(4)?,
                grade_level: row.get(5)?,
                parent_contact: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .await?
    } else {
        stmt.query_map(crate::params![workspace_id], |row| {
            Ok(StudentProfile {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                user_id: row.get(2)?,
                first_name: row.get(3)?,
                last_name: row.get(4)?,
                grade_level: row.get(5)?,
                parent_contact: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .await?
    };

    Ok(list)
}

#[uniffi::export]
pub async fn save_student_profile(
    requester_user_id: String,
    profile: StudentProfile,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != profile.workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_manage_students")?;

    let team_id = "";
    verify_zkp_if_encrypted(
        &conn,
        &profile.first_name,
        &auth.user_id,
        &auth.role,
        &profile.workspace_id,
        team_id,
    )
    .await?;
    verify_zkp_if_encrypted(
        &conn,
        &profile.last_name,
        &auth.user_id,
        &auth.role,
        &profile.workspace_id,
        team_id,
    )
    .await?;
    verify_zkp_if_encrypted(
        &conn,
        &profile.grade_level,
        &auth.user_id,
        &auth.role,
        &profile.workspace_id,
        team_id,
    )
    .await?;
    if let Some(ref contact) = profile.parent_contact {
        verify_zkp_if_encrypted(
            &conn,
            contact,
            &auth.user_id,
            &auth.role,
            &profile.workspace_id,
            team_id,
        )
        .await?;
    }

    let now_ms = crate::infra::time::get_current_time_ms();

    // Query existing record to check for offline concurrent modifications
    let existing: Option<(String, String, String, Option<String>, i64)> = {
        let mut stmt = conn
            .prepare("SELECT first_name, last_name, grade_level, parent_contact, updated_at FROM student_profiles WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![&profile.id]).await?;
        if let Some(row) = rows.next().await? {
            Some((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        } else {
            None
        }
    };

    let mut first_name = profile.first_name.clone();
    let mut last_name = profile.last_name.clone();
    let mut grade_level = profile.grade_level.clone();
    let mut parent_contact = profile.parent_contact.clone();

    let incoming_updated_at = if profile.updated_at > now_ms + 5000 {
        now_ms
    } else {
        profile.updated_at
    };
    if let Some((old_first_name, old_last_name, old_grade, old_parent_contact, old_updated_at)) =
        existing
    {
        if old_updated_at > incoming_updated_at {
            let first_diff = old_first_name != profile.first_name;
            let last_diff = old_last_name != profile.last_name;
            let grade_diff = old_grade != profile.grade_level;
            let parent_diff = old_parent_contact != profile.parent_contact;

            if first_diff || last_diff || grade_diff || parent_diff {
                let mvr = serde_json::json!({
                    "conflict": true,
                    "versions": [
                        {
                            "first_name": old_first_name.clone(),
                            "last_name": old_last_name.clone(),
                            "grade_level": old_grade.clone(),
                            "parent_contact": old_parent_contact.clone(),
                            "by": "Concurrent Editor",
                            "updated_at": old_updated_at
                        },
                        {
                            "first_name": profile.first_name.clone(),
                            "last_name": profile.last_name.clone(),
                            "grade_level": profile.grade_level.clone(),
                            "parent_contact": profile.parent_contact.clone(),
                            "by": requester_user_id.clone(),
                            "updated_at": now_ms
                        }
                    ]
                });

                record_school_conflict(
                    &conn,
                    &profile.workspace_id,
                    "student_profiles",
                    &profile.id,
                    mvr,
                )
                .await?;

                // Keep database clean using Last-Write-Wins (which is the database version, since old_updated_at > profile.updated_at)
                first_name = old_first_name;
                last_name = old_last_name;
                grade_level = old_grade;
                parent_contact = old_parent_contact;
            }
        }
    }

    conn.execute(
        "INSERT OR REPLACE INTO student_profiles (id, workspace_id, user_id, first_name, last_name, grade_level, parent_contact, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
        crate::params![
            &profile.id,
            &profile.workspace_id,
            &profile.user_id,
            &first_name,
            &last_name,
            &grade_level,
            &parent_contact,
            &now_ms,
        ]
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn delete_student_profile(
    requester_user_id: String,
    id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let ws_id: String = conn
        .query_row(
            "SELECT workspace_id FROM student_profiles WHERE id = ?1",
            crate::params![&id],
            |r| r.get(0),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Student profile not found".to_string()))?;

    if auth.role != "platform_admin" && auth.workspace_id != ws_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    conn.execute(
        "DELETE FROM student_profiles WHERE id = ?1",
        crate::params![&id],
    )
    .await?;
    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn link_parent_to_student(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
    parent_user_id: String,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_manage_students")?;

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "INSERT OR REPLACE INTO student_parents (student_id, parent_user_id, workspace_id, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, 'pending')",
        crate::params![&student_id, &parent_user_id, &workspace_id, &now_ms]
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn link_student_self_service(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let parent_email: String = conn
        .query_row(
            "SELECT email FROM users WHERE id = ?1 AND workspace_id = ?2",
            crate::params![&auth.user_id, &workspace_id],
            |r| r.get(0),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Parent user not found".to_string()))?;

    let student_parent_contact: Option<String> = conn
        .query_row(
            "SELECT parent_contact FROM student_profiles WHERE id = ?1 AND workspace_id = ?2",
            crate::params![&student_id, &workspace_id],
            |r| r.get(0),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Student profile not found".to_string()))?;

    if let Some(contact) = student_parent_contact {
        if contact.trim().to_lowercase() == parent_email.trim().to_lowercase() {
            let now_ms = crate::infra::time::get_current_time_ms();
            conn.execute(
                "INSERT OR REPLACE INTO student_parents (student_id, parent_user_id, workspace_id, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, 'pending')",
                crate::params![&student_id, &auth.user_id, &workspace_id, &now_ms]
            ).await?;

            notify_observers();
            return Ok(());
        }
    }

    Err(YntraError::AuthError(
        "Verification failed: parent contact info does not match student profile".to_string(),
    ))
}

#[uniffi::export]
pub async fn get_student_parents(
    requester_user_id: String,
    workspace_id: String,
    student_id: String,
) -> Result<Vec<WorkspaceUser>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    verify_student_access(&conn, &auth, &student_id).await?;

    let mut stmt = conn
        .prepare("
            SELECT u.id, u.workspace_id, u.email, u.full_name, u.phone, u.role, u.preferences, u.metadata, u.updated_at, u.sync_status
            FROM users u
            JOIN student_parents sp ON u.id = sp.parent_user_id
            WHERE sp.student_id = ?1 AND sp.workspace_id = ?2
        ")
        .await?;

    let list = stmt
        .query_map(crate::params![student_id, workspace_id], |row| {
            let id: String = row.get(0)?;
            let metadata_str: Option<String> = row.get(7)?;

            let mut siths_card_id = None;
            let mut nfc_badge_uid = None;
            let mut personal_number = None;
            let mut public_key = None;

            if let Some(ref m_str) = metadata_str {
                if let Ok(meta_val) = serde_json::from_str::<serde_json::Value>(m_str) {
                    public_key = meta_val
                        .get("public_key")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            meta_val
                                .get("siths_public_key")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        });

                    siths_card_id = meta_val
                        .get("siths_card_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    nfc_badge_uid = meta_val
                        .get("nfc_badge_uid")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    personal_number = meta_val
                        .get("personal_number")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            }

            Ok(WorkspaceUser {
                id,
                workspace_id: row.get(1)?,
                email: row.get(2)?,
                full_name: row.get(3)?,
                phone: row.get(4)?,
                role: row.get(5)?,
                preferences: row.get(6)?,
                siths_card_id,
                nfc_badge_uid,
                updated_at: row.get(8)?,
                sync_status: row.get(9)?,
                personal_number,
                public_key,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn get_parent_students(
    requester_user_id: String,
    workspace_id: String,
    parent_user_id: String,
) -> Result<Vec<StudentProfile>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let mut stmt = conn
        .prepare("
            SELECT s.id, s.workspace_id, s.user_id, s.first_name, s.last_name, s.grade_level, s.parent_contact, s.updated_at
            FROM student_profiles s
            JOIN student_parents sp ON s.id = sp.student_id
            WHERE sp.parent_user_id = ?1 AND sp.workspace_id = ?2
        ")
        .await?;

    let list = stmt
        .query_map(crate::params![parent_user_id, workspace_id], |row| {
            Ok(StudentProfile {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                user_id: row.get(2)?,
                first_name: row.get(3)?,
                last_name: row.get(4)?,
                grade_level: row.get(5)?,
                parent_contact: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn get_student_profiles_rkyv(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<u8>, YntraError> {
    let profiles = get_student_profiles(requester_user_id, workspace_id).await?;
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&profiles)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    Ok(bytes.into_vec())
}
