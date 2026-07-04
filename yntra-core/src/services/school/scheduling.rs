#[cfg(not(target_arch = "wasm32"))]
use crate::database;
use crate::observer::notify_observers;
use crate::{TimetableSlot, YntraError};

#[cfg(target_arch = "wasm32")]
use crate::wasm_store;

#[uniffi::export]
pub async fn get_timetable_slots() -> Result<Vec<TimetableSlot>, YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        let mut stmt = conn.prepare("SELECT id, workspace_id, course_id, day_of_week, start_time, end_time, classroom, updated_at, sync_status FROM timetable_slots").await?;
        let list = stmt.query_map((), |row| {
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

    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        Ok(store.timetable_slots.clone())
    }
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        if !super::check_permission(&conn, &requester_user_id, "can_manage_courses").await? {
            return Err(YntraError::AuthError("Access denied: cannot manage scheduling".to_string()));
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let store = wasm_store::get_store().lock().unwrap();
        let requester = store.users.iter().find(|u| u.id == requester_user_id)
            .ok_or_else(|| YntraError::AuthError("Requester user not found".to_string()))?;
        let is_authorized = requester.role == "admin" || requester.role == "platform_admin" || requester.role.contains("rektor") || requester.role.contains("principal");
        if !is_authorized {
            return Err(YntraError::AuthError("Access denied".to_string()));
        }
    }

    let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
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

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
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
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        store.timetable_slots.push(slot.clone());
    }

    notify_observers();
    Ok(slot)
}

#[uniffi::export]
pub async fn delete_timetable_slot(requester_user_id: String, id: String) -> Result<(), YntraError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
        if !super::check_permission(&conn, &requester_user_id, "can_manage_courses").await? {
            return Err(YntraError::AuthError("Access denied: cannot manage scheduling".to_string()));
        }
        conn.execute("DELETE FROM timetable_slots WHERE id = ?1", crate::params![&id]).await?;
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        let requester = store.users.iter().find(|u| u.id == requester_user_id)
            .ok_or_else(|| YntraError::AuthError("Requester user not found".to_string()))?;
        let is_authorized = requester.role == "admin" || requester.role == "platform_admin" || requester.role.contains("rektor") || requester.role.contains("principal");
        if !is_authorized {
            return Err(YntraError::AuthError("Access denied".to_string()));
        }
        store.timetable_slots.retain(|s| s.id != id);
    }

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn sync_timetable_to_calendar(workspace_id: String, user_id: String) -> Result<(), YntraError> {
    let slots = get_timetable_slots().await?;
    if slots.is_empty() {
        return Ok(());
    }

    let courses = super::academics::get_courses(user_id.clone()).await?;
    let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
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

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conn = database::native::acquire_connection().await?;
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
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut store = wasm_store::get_store().lock().unwrap();
        for ev in new_events {
            let exists = store.events.iter().any(|e| e.title == ev.title && e.start_time == ev.start_time);
            if !exists {
                store.events.push(ev);
            }
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
