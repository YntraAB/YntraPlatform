use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;
use crate::services::school::auth::{verify_school_write_zkp, verify_school_permission};
use crate::services::school::conflicts::record_school_conflict;
use crate::TimetableSlot;

#[uniffi::export]
pub async fn get_timetable_slots(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<TimetableSlot>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, course_id, day_of_week, start_time, end_time, classroom, updated_at FROM timetable_slots WHERE workspace_id = ?1")
        .await?;

    let list = stmt
        .query_map(crate::params![workspace_id], |row| {
            Ok(TimetableSlot {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                course_id: row.get(2)?,
                day_of_week: row.get::<i64>(3)? as i32,
                start_time: row.get(4)?,
                end_time: row.get(5)?,
                classroom: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn save_timetable_slot(
    requester_user_id: String,
    slot: TimetableSlot,
    role_proof: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != slot.workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    verify_school_write_zkp(&conn, &requester_user_id, &auth.role, role_proof).await?;
    verify_school_permission(&auth, "can_manage_schedule")?;

    let now_ms = crate::infra::time::get_current_time_ms();

    // Query existing record to check for offline concurrent modifications
    let existing: Option<(String, i64, String, String, Option<String>, i64)> = {
        let mut stmt = conn
            .prepare("SELECT course_id, day_of_week, start_time, end_time, classroom, updated_at FROM timetable_slots WHERE id = ?1")
            .await?;
        let mut rows = stmt.query(crate::params![&slot.id]).await?;
        if let Some(row) = rows.next().await? {
            Some((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
        } else {
            None
        }
    };

    let mut course_id = slot.course_id.clone();
    let mut day_of_week = slot.day_of_week as i64;
    let mut start_time = slot.start_time.clone();
    let mut end_time = slot.end_time.clone();
    let mut classroom = slot.classroom.clone();

    let incoming_updated_at = if slot.updated_at > now_ms + 5000 { now_ms } else { slot.updated_at };
    if let Some((old_course_id, old_day_of_week, old_start_time, old_end_time, old_classroom, old_updated_at)) = existing {
        if old_updated_at > incoming_updated_at {
            let course_diff = old_course_id != slot.course_id;
            let day_diff = old_day_of_week != slot.day_of_week as i64;
            let start_diff = old_start_time != slot.start_time;
            let end_diff = old_end_time != slot.end_time;
            let class_diff = old_classroom != slot.classroom;

            if course_diff || day_diff || start_diff || end_diff || class_diff {
                let mvr = serde_json::json!({
                    "conflict": true,
                    "versions": [
                        {
                            "course_id": old_course_id.clone(),
                            "day_of_week": old_day_of_week,
                            "start_time": old_start_time.clone(),
                            "end_time": old_end_time.clone(),
                            "classroom": old_classroom.clone(),
                            "by": "Concurrent Editor",
                            "updated_at": old_updated_at
                        },
                        {
                            "course_id": slot.course_id.clone(),
                            "day_of_week": slot.day_of_week as i64,
                            "start_time": slot.start_time.clone(),
                            "end_time": slot.end_time.clone(),
                            "classroom": slot.classroom.clone(),
                            "by": requester_user_id.clone(),
                            "updated_at": now_ms
                        }
                    ]
                });
                
                record_school_conflict(&conn, &slot.workspace_id, "timetable_slots", &slot.id, mvr).await?;

                // Keep database clean using Last-Write-Wins (which is the database version, since old_updated_at > slot.updated_at)
                course_id = old_course_id;
                day_of_week = old_day_of_week;
                start_time = old_start_time;
                end_time = old_end_time;
                classroom = old_classroom;
            }
        }
    }

    conn.execute(
        "INSERT OR REPLACE INTO timetable_slots (id, workspace_id, course_id, day_of_week, start_time, end_time, classroom, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')",
        crate::params![
            &slot.id,
            &slot.workspace_id,
            &course_id,
            &day_of_week,
            &start_time,
            &end_time,
            &classroom,
            &now_ms,
        ]
    ).await?;

    notify_observers();
    Ok(())
}
