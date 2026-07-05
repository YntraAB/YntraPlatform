use crate::database;
use crate::observer::notify_observers;
use crate::{TimeReport, YntraError};

#[uniffi::export]
pub async fn get_time_reports(requester_user_id: String, user_id: Option<String>) -> Result<Vec<TimeReport>, YntraError> {
    let conn = database::acquire_connection().await?;
    let (requester_role, requester_ws): (String, Option<String>) = conn.query_row(
        "SELECT role, workspace_id FROM users WHERE id = ?1",
        crate::params![&requester_user_id],
        |r| Ok((r.get(0)?, r.get(1)?))
    ).await.map_err(|e| YntraError::DbError(format!("Failed to retrieve user info: {}", e)))?;

    let ws_id = requester_ws.unwrap_or_else(|| "workspace-1".to_string());

    let (query, params) = if requester_role == "platform_admin" {
        match user_id {
            Some(uid) => (
                "SELECT id, workspace_id, user_id, team_id, date, start_time, end_time, hours, note, status, created_at, updated_at, sync_status FROM time_reports WHERE user_id = ?1 ORDER BY date DESC".to_string(),
                vec![uid],
            ),
            None => (
                "SELECT id, workspace_id, user_id, team_id, date, start_time, end_time, hours, note, status, created_at, updated_at, sync_status FROM time_reports ORDER BY date DESC".to_string(),
                vec![],
            ),
        }
    } else if requester_role == "admin" {
        match user_id {
            Some(uid) => (
                "SELECT id, workspace_id, user_id, team_id, date, start_time, end_time, hours, note, status, created_at, updated_at, sync_status FROM time_reports WHERE user_id = ?1 AND workspace_id = ?2 ORDER BY date DESC".to_string(),
                vec![uid, ws_id],
            ),
            None => (
                "SELECT id, workspace_id, user_id, team_id, date, start_time, end_time, hours, note, status, created_at, updated_at, sync_status FROM time_reports WHERE workspace_id = ?1 ORDER BY date DESC".to_string(),
                vec![ws_id],
            ),
        }
    } else {
        if let Some(ref uid) = user_id {
            if uid != &requester_user_id {
                return Err(YntraError::AuthError("Access denied: you can only view your own time reports".to_string()));
            }
        }
        (
            "SELECT id, workspace_id, user_id, team_id, date, start_time, end_time, hours, note, status, created_at, updated_at, sync_status FROM time_reports WHERE user_id = ?1 ORDER BY date DESC".to_string(),
            vec![requester_user_id.clone()],
        )
    };

    let mut stmt = conn.prepare(&query).await?;
    let list = stmt.query_map(crate::rusqlite::params_from_iter(params), |row| {
        Ok(TimeReport {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            user_id: row.get(2)?,
            team_id: row.get(3)?,
            date: row.get(4)?,
            start_time: row.get(5)?,
            end_time: row.get(6)?,
            hours: row.get(7)?,
            note: row.get(8)?,
            status: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
            sync_status: row.get(12)?,
        })
    }).await?;

    Ok(list)
}

#[uniffi::export]
#[allow(clippy::too_many_arguments)]
pub async fn add_time_report(
    workspace_id: String,
    user_id: String,
    team_id: Option<String>,
    date: String,
    hours: f64,
    note: String,
    start_time: Option<String>,
    end_time: Option<String>,
) -> Result<TimeReport, YntraError> {
    // 1. Calculate dates and convert target date to days
    let target_days = match parse_date(&date) {
        Some((y, m, d)) => date_to_days(y, m, d),
        None => return Err(YntraError::ValidationError("Invalid date format, expected YYYY-MM-DD".to_string())),
    };

    // 2. Fetch all logged hours and workspace settings to determine national limits
    let mut user_reports = Vec::new();
    let mut settings_json = "{}".to_string();

    let conn = database::acquire_connection().await?;
    
    // Fetch workspace settings
    let mut w_stmt = conn.prepare("SELECT settings FROM workspaces WHERE id = ?1").await?;
    let mut w_rows = w_stmt.query(crate::params![&workspace_id]).await?;
    if let Some(row) = w_rows.next().await? {
        settings_json = row.get(0)?;
    }

    // Fetch user reports
    let mut stmt = conn.prepare("SELECT date, hours FROM time_reports WHERE user_id = ?1").await?;
    let mut rows = stmt.query(crate::params![&user_id]).await?;
    while let Some(row) = rows.next().await? {
        let r_date: String = row.get(0)?;
        let r_hours: f64 = row.get(1)?;
        user_reports.push((r_date, r_hours));
    }

    // Parse configuration fields
    let settings: serde_json::Value = serde_json::from_str(&settings_json).unwrap_or(serde_json::Value::Null);
    let target_region = settings.get("target_region")
        .and_then(|v| v.as_str())
        .unwrap_or("EU");

    let allow_overtime = settings.get("allow_overtime")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let allow_union_exempt = settings.get("allow_union_exempt")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Resolve rules from Compliance Registry
    let rule = crate::infra::compliance::ComplianceRegistry::get_rule(target_region);
    let daily_limit = if allow_overtime { rule.max_daily_limit_with_overtime } else { rule.standard_daily_limit };
    let weekly_limit = if allow_union_exempt { rule.max_weekly_limit_with_exemption } else { rule.standard_weekly_limit };

    // 3. Validate daily limit
    let daily_logged: f64 = user_reports.iter()
        .filter(|(r_date, _)| r_date == &date)
        .map(|(_, hrs)| *hrs)
        .sum();

    if daily_logged + hours > daily_limit {
        let msg = match target_region {
            "NO" | "SE" | "DK" => format!(
                "Daily working hours limit ({}h) exceeded under {}. Currently logged: {}h, trying to log: {}h.",
                daily_limit, rule.law_name, daily_logged, hours
            ),
            _ => format!(
                "Daily working hours limit ({}h) exceeded. Currently logged: {}h, trying to log: {}h (Violates mandatory 11h daily rest period)",
                daily_limit, daily_logged, hours
            ),
        };
        return Err(YntraError::ValidationError(msg));
    }

    // 4. Validate weekly limit
    let weekly_logged: f64 = user_reports.iter()
        .filter_map(|(r_date, hrs)| {
            parse_date(r_date)
                .map(|(y, m, d)| date_to_days(y, m, d))
                .filter(|&days| days >= target_days - 6 && days <= target_days)
                .map(|_| *hrs)
        })
        .sum();

    if weekly_logged + hours > weekly_limit {
        let msg = match target_region {
            "NO" | "SE" | "DK" => format!(
                "Weekly working hours limit ({}h) exceeded under {}. Currently logged in window: {}h, trying to log: {}h.",
                weekly_limit, rule.law_name, weekly_logged, hours
            ),
            _ => format!(
                "Weekly working hours limit ({}h in rolling 7 days) exceeded under {}. Currently logged in window: {}h, trying to log: {}h.",
                weekly_limit, rule.law_name, weekly_logged, hours
            ),
        };
        return Err(YntraError::ValidationError(msg));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let created_at = crate::infra::time::get_current_datetime_str();
    let now_ms = crate::infra::time::get_current_time_ms();
    crate::log_action(user_id.clone(), None, "add_time_report".to_string()).await?;
    let item = TimeReport {
        id: id.clone(),
        workspace_id,
        user_id,
        team_id,
        date,
        start_time,
        end_time,
        hours,
        note: Some(note),
        status: "pending_attest".to_string(),
        created_at,
        updated_at: now_ms,
        sync_status: "pending".to_string(),
    };

    conn.execute(
        "INSERT INTO time_reports (id, workspace_id, user_id, team_id, date, start_time, end_time, hours, note, status, created_at, updated_at, sync_status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending_attest', ?10, ?11, 'pending')",
        crate::params![
            &item.id,
            &item.workspace_id,
            &item.user_id,
            &item.team_id,
            &item.date,
            &item.start_time,
            &item.end_time,
            &item.hours,
            &item.note,
            &item.created_at,
            &item.updated_at
        ],
    ).await?;

    notify_observers();

    Ok(item)
}

#[uniffi::export]
pub async fn update_time_report_status(id: String, status: String) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;

    conn.execute(
        "UPDATE time_reports SET status = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![&status, &now_ms, &id],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn delete_time_report(id: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;

    conn.execute(
        "DELETE FROM time_reports WHERE id = ?1",
        crate::params![id],
    ).await?;

    notify_observers();
    Ok(())
}

fn parse_date(date_str: &str) -> Option<(i32, i32, i32)> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year = parts[0].parse::<i32>().ok()?;
    let month = parts[1].parse::<i32>().ok()?;
    let day = parts[2].parse::<i32>().ok()?;
    Some((year, month, day))
}

fn date_to_days(year: i32, month: i32, day: i32) -> i32 {
    let m = (month + 9) % 12;
    let y = year - m / 10;
    365 * y + y / 4 - y / 100 + y / 400 + (m * 306 + 5) / 10 + (day - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date_valid() {
        assert_eq!(parse_date("2026-07-05"), Some((2026, 7, 5)));
        assert_eq!(parse_date("2000-01-01"), Some((2000, 1, 1)));
    }

    #[test]
    fn test_parse_date_invalid() {
        assert_eq!(parse_date("2026-07"), None);
        assert_eq!(parse_date("2026/07/05"), None);
        assert_eq!(parse_date("abc-def-ghi"), None);
    }

    #[test]
    fn test_date_to_days_sequence() {
        let day1 = date_to_days(2026, 7, 1);
        let day2 = date_to_days(2026, 7, 2);
        let day3 = date_to_days(2026, 7, 8);

        assert_eq!(day2 - day1, 1);
        assert_eq!(day3 - day1, 7);
    }

    #[test]
    fn test_date_to_days_leap_year() {
        // 2024 is a leap year (Feb 29 exists)
        let pre_leap = date_to_days(2024, 2, 28);
        let leap_day = date_to_days(2024, 2, 29);
        let post_leap = date_to_days(2024, 3, 1);

        assert_eq!(leap_day - pre_leap, 1);
        assert_eq!(post_leap - leap_day, 1);

        // 2025 is not a leap year
        let normal_feb28 = date_to_days(2025, 2, 28);
        let normal_mar01 = date_to_days(2025, 3, 1);
        assert_eq!(normal_mar01 - normal_feb28, 1);
    }
}

