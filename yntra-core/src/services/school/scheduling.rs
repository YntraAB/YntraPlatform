use crate::database;
use crate::observer::notify_observers;
use crate::{TimetableSlot, YntraError};

#[uniffi::export]
pub async fn get_timetable_slots(requester_user_id: String) -> Result<Vec<TimetableSlot>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let mut stmt = conn.prepare("SELECT id, workspace_id, course_id, day_of_week, start_time, end_time, classroom, updated_at, sync_status FROM timetable_slots WHERE workspace_id = ?1").await?;
    let list = stmt.query_map(crate::params![auth.workspace_id], |row| {
        Ok(TimetableSlot {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            course_id: row.get(2)?,
            day_of_week: row.get(3)?,
            start_time: row.get(4)?,
            end_time: row.get(5)?,
            classroom: row.get(6)?,
            updated_at: row.get(7)?,
            sync_status: row.get(8)?,
        })
    }).await?;
    Ok(list)
}

#[uniffi::export]
pub async fn save_timetable_slot(
    requester_user_id: String,
    workspace_id: String,
    course_id: String,
    day_of_week: i32,
    start_time: String,
    end_time: String,
    classroom: Option<String>,
) -> Result<TimetableSlot, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let course_ws: String = conn.query_row(
        "SELECT workspace_id FROM courses WHERE id = ?1",
        crate::params![&course_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Course not found".to_string()))?;

    if course_ws != workspace_id {
        return Err(YntraError::ValidationError("Course does not belong to the specified workspace".to_string()));
    }

    if !super::check_permission(&conn, &requester_user_id, "can_manage_courses").await? {
        return Err(YntraError::AuthError("Access denied: cannot manage scheduling".to_string()));
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    let id = uuid::Uuid::new_v4().to_string();
    let slot = TimetableSlot {
        id: id.clone(),
        workspace_id: workspace_id.clone(),
        course_id: course_id.clone(),
        day_of_week,
        start_time: start_time.clone(),
        end_time: end_time.clone(),
        classroom: classroom.clone(),
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    conn.execute(
        "INSERT INTO timetable_slots (id, workspace_id, course_id, day_of_week, start_time, end_time, classroom, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        crate::params![
            &slot.id,
            &slot.workspace_id,
            &slot.course_id,
            &slot.day_of_week,
            &slot.start_time,
            &slot.end_time,
            &slot.classroom,
            &slot.updated_at,
            &slot.sync_status,
        ],
    ).await?;

    notify_observers();
    Ok(slot)
}

#[uniffi::export]
pub async fn delete_timetable_slot(requester_user_id: String, id: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let slot_ws: String = conn.query_row(
        "SELECT workspace_id FROM timetable_slots WHERE id = ?1",
        crate::params![&id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Timetable slot not found".to_string()))?;

    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != slot_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    if !super::check_permission(&conn, &requester_user_id, "can_manage_courses").await? {
        return Err(YntraError::AuthError("Access denied: cannot manage scheduling".to_string()));
    }
    conn.execute("DELETE FROM timetable_slots WHERE id = ?1", crate::params![&id]).await?;

    notify_observers();
    Ok(())
}

async fn get_student_active_courses(
    conn: &database::DbConnection,
    student_id: &str,
) -> Result<std::collections::HashSet<String>, YntraError> {
    let mut course_ids = std::collections::HashSet::new();

    // 1. Attendance records
    if let Ok(mut stmt) = conn.prepare("SELECT DISTINCT course_id FROM attendance_records WHERE student_id = ?1").await {
        if let Ok(mut rows) = stmt.query(crate::params![student_id]).await {
            while let Ok(Some(row)) = rows.next().await {
                if let Ok(cid) = row.get::<String>(0) {
                    course_ids.insert(cid);
                }
            }
        }
    }

    // 2. Term grades
    if let Ok(mut stmt) = conn.prepare("SELECT DISTINCT course_id FROM term_grades WHERE student_id = ?1").await {
        if let Ok(mut rows) = stmt.query(crate::params![student_id]).await {
            while let Ok(Some(row)) = rows.next().await {
                if let Ok(cid) = row.get::<String>(0) {
                    course_ids.insert(cid);
                }
            }
        }
    }

    // 3. Submissions
    if let Ok(mut stmt) = conn.prepare(
        "SELECT DISTINCT a.course_id FROM submissions s JOIN assignments a ON s.assignment_id = a.id WHERE s.student_id = ?1"
    ).await {
        if let Ok(mut rows) = stmt.query(crate::params![student_id]).await {
            while let Ok(Some(row)) = rows.next().await {
                if let Ok(cid) = row.get::<String>(0) {
                    course_ids.insert(cid);
                }
            }
        }
    }

    Ok(course_ids)
}

#[uniffi::export]
pub async fn sync_timetable_to_calendar(workspace_id: String, user_id: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &user_id).await?;

    let is_admin = auth.role == "platform_admin" || auth.role == "admin" || auth.role.contains("rektor") || auth.role.contains("principal");

    let mut allowed_course_ids = std::collections::HashSet::new();

    // 1. Check if user is a teacher
    let mut stmt = conn.prepare("SELECT id FROM courses WHERE teacher_id = ?1").await?;
    let mut rows = stmt.query(crate::params![&user_id]).await?;
    while let Some(row) = rows.next().await? {
        let cid: String = row.get(0)?;
        allowed_course_ids.insert(cid);
    }

    // 2. Check if user is a student
    if let Ok(student_id) = conn.query_row(
        "SELECT id FROM student_profiles WHERE user_id = ?1",
        crate::params![&user_id],
        |r| r.get::<String>(0)
    ).await {
        let active = get_student_active_courses(&conn, &student_id).await?;
        allowed_course_ids.extend(active);
    }

    // 3. Check if user is a parent
    let mut stmt = conn.prepare("SELECT student_id FROM student_parents WHERE parent_user_id = ?1").await?;
    let mut rows = stmt.query(crate::params![&user_id]).await?;
    while let Some(row) = rows.next().await? {
        let sid: String = row.get(0)?;
        let child_courses = get_student_active_courses(&conn, &sid).await?;
        allowed_course_ids.extend(child_courses);
    }

    let all_slots = get_timetable_slots(user_id.clone()).await?;
    let slots: Vec<TimetableSlot> = all_slots
        .into_iter()
        .filter(|s| s.workspace_id == workspace_id)
        .filter(|s| {
            if is_admin {
                true
            } else {
                allowed_course_ids.contains(&s.course_id)
            }
        })
        .collect();

    if slots.is_empty() {
        return Ok(());
    }

    let courses = super::academics::get_courses(user_id.clone()).await?;
    let now_ms = crate::infra::time::get_current_time_ms();
    let now_secs = now_ms / 1000;

    let mut new_events = Vec::new();
    for i in 0..14 {
        let day_secs = now_secs + i * 86400;
        let days_since = day_secs / 86400;
        let day_of_week = ((days_since + 3) % 7) + 1;

        for slot in slots.iter() {
            if slot.day_of_week == day_of_week as i32 {
                let course_name = courses
                    .iter()
                    .find(|c| c.id == slot.course_id)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "School Course".to_string());

                let room_str = slot.classroom.clone().unwrap_or_else(|| "TBD".to_string());
                let title = format!("Class: {} ({})", course_name, room_str);

                let date_str = format_date_from_secs(day_secs);
                let start_dt = format!("{} {}", date_str, slot.start_time);
                let end_dt = format!("{} {}", date_str, slot.end_time);

                let event_id = uuid::Uuid::new_v4().to_string();
                let event = crate::TeamEvent {
                    id: event_id,
                    workspace_id: workspace_id.clone(),
                    user_id: Some(user_id.clone()),
                    team_id: None,
                    assignee_id: None,
                    title,
                    start_time: start_dt,
                    end_time: end_dt,
                    metadata: "{\"category\":\"training\"}".to_string(),
                    updated_at: now_ms,
                    sync_status: "pending".to_string(),
                };
                new_events.push(event);
            }
        }
    }

    let conn = database::acquire_connection().await?;
    for ev in new_events {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM events WHERE title = ?1 AND start_time = ?2",
            crate::params![&ev.title, &ev.start_time],
            |row| row.get(0),
        ).await.unwrap_or(0);

        if count == 0 {
            conn.execute(
                "INSERT INTO events (id, workspace_id, user_id, team_id, assignee_id, title, start_time, end_time, metadata, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                crate::params![
                    &ev.id,
                    &ev.workspace_id,
                    &ev.user_id,
                    &ev.team_id,
                    &ev.assignee_id,
                    &ev.title,
                    &ev.start_time,
                    &ev.end_time,
                    &ev.metadata,
                    &ev.updated_at,
                    &ev.sync_status,
                ],
            ).await?;
        }
    }

    notify_observers();
    Ok(())
}

fn format_date_from_secs(secs: i64) -> String {
    let mut days = secs / 86400;
    let mut year = 1970;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if leap { 366 } else { 365 };
        if days >= days_in_year {
            days -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let month_lengths = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1;
    for &length in month_lengths.iter() {
        if days >= length {
            days -= length;
            month += 1;
        } else {
            break;
        }
    }
    let day = days + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    #[test]
    fn test_format_date_from_secs_scenarios() {
        // Test epoch start
        assert_eq!(format_date_from_secs(0), "1970-01-01");
        // Test 1 day after epoch
        assert_eq!(format_date_from_secs(86400), "1970-01-02");
        // Test leap years
        assert_eq!(format_date_from_secs(1582934400), "2020-02-29"); // 2020-02-29
        assert_eq!(format_date_from_secs(1583020800), "2020-03-01"); // 2020-03-01
        // Test standard modern dates (1783296000 is 2026-07-06 UTC)
        assert_eq!(format_date_from_secs(1783296000), "2026-07-06");
    }

    #[tokio::test]
    async fn test_sync_timetable_to_calendar_generation() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let conn = database::acquire_connection().await.unwrap();

        // Cleanup first in case of dirty state
        let _ = conn.execute("PRAGMA foreign_keys = OFF;", ()).await;
        let _ = conn.execute("DELETE FROM events WHERE workspace_id = 'ws-sched-1'", ()).await;
        let _ = conn.execute("DELETE FROM timetable_slots WHERE workspace_id = 'ws-sched-1'", ()).await;
        let _ = conn.execute("DELETE FROM courses WHERE workspace_id = 'ws-sched-1'", ()).await;
        let _ = conn.execute("DELETE FROM users WHERE workspace_id = 'ws-sched-1'", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = 'ws-sched-1'", ()).await;
        let _ = conn.execute("PRAGMA foreign_keys = ON;", ()).await;

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-sched-1', 'Sched WS', '[]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-sched-admin', 'ws-sched-1', 'admin@sched.io', 'admin')", ()).await.unwrap();

        // 1. Save course
        conn.execute(
            "INSERT OR REPLACE INTO courses (id, workspace_id, name, subject, teacher_id, classroom, updated_at, sync_status) VALUES ('course-1', 'ws-sched-1', 'Math 101', 'Math', 'teacher-1', 'Room 101', 0, 'synced')",
            ()
        ).await.unwrap();

        // 2. Setup timetable slot (day_of_week: 1)
        let slot = save_timetable_slot(
            "u-sched-admin".to_string(),
            "ws-sched-1".to_string(),
            "course-1".to_string(),
            1, // day_of_week
            "09:00".to_string(),
            "10:30".to_string(),
            Some("Room 101".to_string()),
        ).await.unwrap();

        assert_eq!(slot.classroom, Some("Room 101".to_string()));

        let sync_res = sync_timetable_to_calendar("ws-sched-1".to_string(), "u-sched-admin".to_string()).await;
        assert!(sync_res.is_ok());

        // Verify that events were created in the database
        let events_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM events WHERE workspace_id = 'ws-sched-1' AND user_id = 'u-sched-admin'",
            (),
            |r| r.get(0)
        ).await.unwrap();
        
        // Since it checks 14 days, it should find 2 days matching day_of_week = 1
        assert_eq!(events_count, 2);

        // Cleanup
        let _ = conn.execute("PRAGMA foreign_keys = OFF;", ()).await;
        conn.execute("DELETE FROM events WHERE workspace_id = 'ws-sched-1'", ()).await.unwrap();
        conn.execute("DELETE FROM timetable_slots WHERE workspace_id = 'ws-sched-1'", ()).await.unwrap();
        conn.execute("DELETE FROM courses WHERE workspace_id = 'ws-sched-1'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-sched-1'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-sched-1'", ()).await.unwrap();
        let _ = conn.execute("PRAGMA foreign_keys = ON;", ()).await;
    }
}

