use crate::database;
use crate::observer::notify_observers;
use crate::{TimeReport, YntraError};

#[uniffi::export]
pub async fn get_time_reports(requester_user_id: String, user_id: Option<String>) -> Result<Vec<TimeReport>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let ws_id = auth.workspace_id.clone();

    let (query, params) = if auth.role == "platform_admin" {
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
    } else if auth.role == "admin" {
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

    // 2. Fetch all logged hours, workspace settings, and user preferences to determine national limits
    let (settings_json, user_prefs_json, user_reports) = {
        let conn = database::acquire_connection().await?;
        
        let mut settings_json = "{}".to_string();
        // Fetch workspace settings
        let mut w_stmt = conn.prepare("SELECT settings FROM workspaces WHERE id = ?1").await?;
        let mut w_rows = w_stmt.query(crate::params![&workspace_id]).await?;
        if let Some(row) = w_rows.next().await? {
            settings_json = row.get(0)?;
        }

        let mut user_prefs_json = "{}".to_string();
        // Fetch user preferences
        let mut u_stmt = conn.prepare("SELECT preferences FROM users WHERE id = ?1").await?;
        let mut u_rows = u_stmt.query(crate::params![&user_id]).await?;
        if let Some(row) = u_rows.next().await? {
            user_prefs_json = row.get::<Option<String>>(0)?.unwrap_or_else(|| "{}".to_string());
        }

        // Fetch user reports
        let mut user_reports = Vec::new();
        let mut stmt = conn.prepare("SELECT date, hours, start_time, end_time FROM time_reports WHERE user_id = ?1").await?;
        let mut rows = stmt.query(crate::params![&user_id]).await?;
        while let Some(row) = rows.next().await? {
            let r_date: String = row.get(0)?;
            let r_hours: f64 = row.get(1)?;
            let r_start: Option<String> = row.get(2)?;
            let r_end: Option<String> = row.get(3)?;
            user_reports.push((r_date, r_hours, r_start, r_end));
        }
        (settings_json, user_prefs_json, user_reports)
    };

    // Parse configuration fields
    let settings: serde_json::Value = serde_json::from_str(&settings_json).unwrap_or(serde_json::Value::Null);
    let u_prefs: serde_json::Value = serde_json::from_str(&user_prefs_json).unwrap_or(serde_json::Value::Null);

    let target_region = u_prefs.get("target_region")
        .or_else(|| settings.get("target_region"))
        .and_then(|v| v.as_str())
        .unwrap_or("EU");

    let allow_overtime = u_prefs.get("allow_overtime")
        .or_else(|| settings.get("allow_overtime"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let allow_union_exempt = u_prefs.get("allow_union_exempt")
        .or_else(|| settings.get("allow_union_exempt"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Resolve rules from Compliance Registry
    let rule = crate::infra::compliance::ComplianceRegistry::get_rule(target_region);
    let daily_limit = if allow_overtime { rule.max_daily_limit_with_overtime } else { rule.standard_daily_limit };
    let weekly_limit = if allow_overtime || allow_union_exempt { rule.max_weekly_limit_with_exemption } else { rule.standard_weekly_limit };

    // 3. Resolve interval bounds for new report
    let (new_start_abs, new_end_abs) = match get_report_interval(&date, hours, start_time.as_deref(), end_time.as_deref()) {
        Some(interval) => interval,
        None => return Err(YntraError::ValidationError("Invalid start_time or end_time format (expected HH:MM)".to_string())),
    };

    let start_day_idx = new_start_abs / 1440;
    let end_day_idx = (new_end_abs - 1) / 1440;

    let dst_adj = adjust_duration_for_dst(new_start_abs, new_end_abs, target_region);
    let interval_duration_hrs = (new_end_abs - new_start_abs + dst_adj) as f64 / 60.0;
    if hours > interval_duration_hrs {
        return Err(YntraError::ValidationError(format!(
            "Logged hours ({:.2}h) cannot exceed the shift duration ({:.2}h from {} to {})",
            hours, interval_duration_hrs, start_time.as_deref().unwrap_or(""), end_time.as_deref().unwrap_or("")
        )));
    }

    // Convert existing user reports to absolute minute intervals
    let mut existing_intervals = Vec::new();
    for (r_date, r_hrs, r_start, r_end) in &user_reports {
        if let Some(interval) = get_report_interval(r_date, *r_hrs, r_start.as_deref(), r_end.as_deref()) {
            existing_intervals.push(interval);
        }
    }

    // 4. Validate daily limit on all affected days
    let mut all_intervals = existing_intervals.clone();
    all_intervals.push((new_start_abs, new_end_abs));

    for day_idx in start_day_idx..=end_day_idx {
        let daily_logged_on_day = get_hours_on_day(day_idx, &all_intervals);
        if daily_logged_on_day > daily_limit {
            let day_date_str = format_date_from_days(day_idx);
            let msg = match target_region {
                "NO" | "SE" | "DK" => format!(
                    "Daily working hours limit ({}h) exceeded under {} on {}. Logged on this day: {:.2}h.",
                    daily_limit, rule.law_name, day_date_str, daily_logged_on_day
                ),
                _ => format!(
                    "Daily working hours limit ({}h) exceeded on {}. Logged on this day: {:.2}h (Violates mandatory 11h daily rest period)",
                    daily_limit, day_date_str, daily_logged_on_day
                ),
            };
            return Err(YntraError::ValidationError(msg));
        }
    }

    // Check for overlaps
    for &(estart, eend) in &existing_intervals {
        if new_start_abs < eend && estart < new_end_abs {
            return Err(YntraError::ValidationError(format!(
                "Shift overlaps with an existing logged shift (existing: {} to {})",
                format_abs_minutes_to_datetime(estart),
                format_abs_minutes_to_datetime(eend)
            )));
        }
    }

    // Check consecutive daily rest hours if mandatory rest is set
    if rule.mandatory_daily_rest_hours > 0.0 {
        let mandatory_rest_min = (rule.mandatory_daily_rest_hours * 60.0) as i32;
        
        let mut sorted_intervals = all_intervals.clone();
        sorted_intervals.sort_by_key(|x| x.0);
        
        for i in 0..sorted_intervals.len() {
            let (s_start, _s_end) = sorted_intervals[i];
            let window_start = s_start;
            let dst_change_in_window = adjust_duration_for_dst(window_start, window_start + 1440, target_region);
            let window_end = s_start + 1440 - dst_change_in_window;
            
            // Collect all segments overlapping with W
            let mut segments = Vec::new();
            for &(start, end) in &sorted_intervals {
                let seg_start = start.max(window_start);
                let seg_end = end.min(window_end);
                if seg_start < seg_end {
                    segments.push((seg_start, seg_end));
                }
            }
            
            // Find max gap in W
            let mut max_rest = 0;
            let mut current_point = window_start;
            
            for &(seg_start, seg_end) in &segments {
                if seg_start > current_point {
                    let rest_gap = seg_start - current_point;
                    if rest_gap > max_rest {
                        max_rest = rest_gap;
                    }
                }
                current_point = current_point.max(seg_end);
            }
            
            if window_end > current_point {
                let rest_gap = window_end - current_point;
                if rest_gap > max_rest {
                    max_rest = rest_gap;
                }
            }
            
            if max_rest < mandatory_rest_min {
                return Err(YntraError::ValidationError(format!(
                    "Daily working hours violation under {}: does not satisfy mandatory {}h consecutive daily rest period in the 24h window starting at {}",
                    rule.law_name, rule.mandatory_daily_rest_hours, format_abs_minutes_to_datetime(s_start)
                )));
            }
        }
    }

    // 5. Validate weekly limit (fixed Monday-to-Sunday week containing target_days)
    let target_monday = target_days - ((target_days + 2) % 7);
    let weekly_logged: f64 = user_reports.iter()
        .filter_map(|(r_date, hrs, _, _)| {
            parse_date(r_date)
                .map(|(y, m, d)| date_to_days(y, m, d))
                .filter(|&days| days >= target_monday && days < target_monday + 7)
                .map(|_| *hrs)
        })
        .sum();

    let skip_calendar_weekly_cap = matches!(target_region, "SE" | "NO" | "DK" | "EU") && allow_overtime;

    if !skip_calendar_weekly_cap && weekly_logged + hours > weekly_limit {
        let msg = match target_region {
            "NO" | "SE" | "DK" => format!(
                "Weekly working hours limit ({}h) exceeded under {}. Currently logged in window: {}h, trying to log: {}h.",
                weekly_limit, rule.law_name, weekly_logged, hours
            ),
            _ => format!(
                "Weekly working hours limit ({}h in calendar week) exceeded under {}. Currently logged in window: {}h, trying to log: {}h.",
                weekly_limit, rule.law_name, weekly_logged, hours
            ),
        };
        return Err(YntraError::ValidationError(msg));
    }

    // Check rolling 16-week average weekly limit of 48h for Nordic/EU regions
    if matches!(target_region, "SE" | "NO" | "DK" | "EU") {
        let window_start_day = target_monday - 15 * 7;
        let window_end_day = target_monday + 7;

        let rolling_logged: f64 = user_reports.iter()
            .filter_map(|(r_date, hrs, _, _)| {
                parse_date(r_date)
                    .map(|(y, m, d)| date_to_days(y, m, d))
                    .filter(|&days| days >= window_start_day && days < window_end_day)
                    .map(|_| *hrs)
            })
            .sum();

        let rolling_average = (rolling_logged + hours) / 16.0;
        if rolling_average > 48.0 {
            return Err(YntraError::ValidationError(format!(
                "Rolling 16-week average weekly working hours ({:.2}h) exceeds the legal limit of 48.0h under {}.",
                rolling_average, rule.law_name
            )));
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let created_at = crate::infra::time::get_current_datetime_str();
    let now_ms = crate::infra::time::get_current_time_ms();
    
    // Log audit action first (releases its connection upon return)
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

    // Re-acquire connection to perform insert
    let conn = database::acquire_connection().await?;
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
pub async fn update_time_report_status(requester_user_id: String, id: String, status: String) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;

    let report_ws: String = conn.query_row(
        "SELECT workspace_id FROM time_reports WHERE id = ?1",
        crate::params![&id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError("Time report not found".to_string()))?;

    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if !auth.is_admin {
        return Err(YntraError::AuthError("Access denied: administrator privileges required".to_string()));
    }

    if auth.role != "platform_admin" && auth.workspace_id != report_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    conn.execute(
        "UPDATE time_reports SET status = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![&status, &now_ms, &id],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn delete_time_report(requester_user_id: String, id: String) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;

    let (report_ws, report_user): (String, String) = conn.query_row(
        "SELECT workspace_id, user_id FROM time_reports WHERE id = ?1",
        crate::params![&id],
        |r| Ok((r.get(0)?, r.get(1)?))
    ).await.map_err(|_| YntraError::NotFoundError("Time report not found".to_string()))?;

    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role != "platform_admin" && auth.workspace_id != report_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    if !auth.is_admin && report_user != requester_user_id {
        return Err(YntraError::AuthError("Access denied: you can only delete your own time reports".to_string()));
    }

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

fn parse_time_to_minutes(time_str: &str) -> Option<i32> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let h = parts[0].parse::<i32>().ok()?;
    let m = parts[1].parse::<i32>().ok()?;
    if h < 0 || h > 23 || m < 0 || m > 59 {
        return None;
    }
    Some(h * 60 + m)
}

fn get_report_interval(
    date_str: &str,
    hours: f64,
    start_time: Option<&str>,
    end_time: Option<&str>,
) -> Option<(i32, i32)> {
    let (y, m, d) = parse_date(date_str)?;
    let day_start_min = date_to_days(y, m, d) * 1440;
    
    if let (Some(s_str), Some(e_str)) = (start_time, end_time) {
        let s_min = parse_time_to_minutes(s_str)?;
        let e_min = parse_time_to_minutes(e_str)?;
        let duration = if e_min < s_min {
            // Spans midnight
            (e_min + 1440 - s_min) % 1440
        } else {
            e_min - s_min
        };
        Some((day_start_min + s_min, day_start_min + s_min + duration))
    } else {
        // Fallback: assume default shift starting at 08:00
        let s_min = 8 * 60; // 08:00
        let duration = (hours * 60.0) as i32;
        Some((day_start_min + s_min, day_start_min + s_min + duration))
    }
}

fn format_abs_minutes_to_time(abs_min: i32) -> String {
    let day_min = abs_min % 1440;
    let h = day_min / 60;
    let m = day_min % 60;
    format!("{:02}:{:02}", h, m)
}

fn format_date_from_days(days: i32) -> String {
    let mut d_count = days - 719468;
    let mut year = 1970;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if leap { 366 } else { 365 };
        if d_count >= days_in_year {
            d_count -= days_in_year;
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
        if d_count >= length {
            d_count -= length;
            month += 1;
        } else {
            break;
        }
    }
    let day = d_count + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn format_abs_minutes_to_datetime(abs_min: i32) -> String {
    let days = abs_min / 1440;
    let date_str = format_date_from_days(days);
    let time_str = format_abs_minutes_to_time(abs_min);
    format!("{} {}", date_str, time_str)
}

fn get_hours_on_day(day_index: i32, intervals: &[(i32, i32)]) -> f64 {
    let day_start = day_index * 1440;
    let day_end = day_start + 1440;
    let mut total_min = 0;
    for &(start, end) in intervals {
        let overlap_start = start.max(day_start);
        let overlap_end = end.min(day_end);
        if overlap_start < overlap_end {
            total_min += overlap_end - overlap_start;
        }
    }
    total_min as f64 / 60.0
}

fn get_dst_offset_change(y: i32, m: i32, d: i32, region: &str) -> i32 {
    let days = date_to_days(y, m, d);
    let is_sunday = (days + 3) % 7 == 0;
    if !is_sunday {
        return 0;
    }
    let is_eu = matches!(region, "SE" | "NO" | "DK" | "EU");
    let is_us = region.starts_with("US");

    if is_eu {
        if m == 3 && d >= 25 {
            // Last Sunday of March -> Spring forward (-60 minutes)
            return -60;
        }
        if m == 10 && d >= 25 {
            // Last Sunday of October -> Fall back (+60 minutes)
            return 60;
        }
    } else if is_us {
        if m == 3 && d >= 8 && d <= 14 {
            // Second Sunday of March -> Spring forward (-60 minutes)
            return -60;
        }
        if m == 11 && d >= 1 && d <= 7 {
            // First Sunday of November -> Fall back (+60 minutes)
            return 60;
        }
    }
    0
}

fn adjust_duration_for_dst(start_abs: i32, end_abs: i32, region: &str) -> i32 {
    let start_day = start_abs / 1440;
    let end_day = end_abs / 1440;
    let mut adjustment = 0;
    for day_idx in start_day..=end_day {
        let (y, m, d) = format_date_parts_from_days(day_idx);
        let change = get_dst_offset_change(y, m, d, region);
        if change != 0 {
            let transition_abs = day_idx * 1440 + 120; // 02:00
            if start_abs <= transition_abs && transition_abs <= end_abs {
                adjustment += change;
            }
        }
    }
    adjustment
}

fn format_date_parts_from_days(days: i32) -> (i32, i32, i32) {
    let mut d_count = days - 719468;
    let mut year = 1970;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if leap { 366 } else { 365 };
        if d_count >= days_in_year {
            d_count -= days_in_year;
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
        if d_count >= length {
            d_count -= length;
            month += 1;
        } else {
            break;
        }
    }
    let day = d_count + 1;
    (year, month, day)
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

    #[test]
    fn test_dst_offset_math() {
        // Last Sunday of March 2026: March 29
        assert_eq!(get_dst_offset_change(2026, 3, 29, "SE"), -60);
        // Last Sunday of October 2026: October 25
        assert_eq!(get_dst_offset_change(2026, 10, 25, "NO"), 60);
        // Non-Sundays or other months should be 0
        assert_eq!(get_dst_offset_change(2026, 3, 28, "SE"), 0);
        assert_eq!(get_dst_offset_change(2026, 6, 15, "SE"), 0);
    }

    #[test]
    fn test_daily_hours_on_day_partitioning() {
        // Shift spanning midnight: July 5 22:00 to July 6 06:00
        let j5_idx = date_to_days(2026, 7, 5);
        let j5_start = j5_idx * 1440;
        
        let shift = (j5_start + 1320, j5_start + 1800); // 22:00 (1320m) to 06:00 (1800m relative to July 5 start)
        let intervals = vec![shift];
        
        // July 5 portion: 22:00 to 24:00 (2 hours)
        assert_eq!(get_hours_on_day(j5_idx, &intervals), 2.0);
        // July 6 portion: 00:00 to 06:00 (6 hours)
        assert_eq!(get_hours_on_day(j5_idx + 1, &intervals), 6.0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_rolling_average_weekly_limit() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Clean any existing test workspace/users in dependency order
        let _ = conn.execute("DELETE FROM time_reports WHERE workspace_id = 'ws-time-test'", ()).await;
        let _ = conn.execute("DELETE FROM users WHERE workspace_id = 'ws-time-test'", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = 'ws-time-test'", ()).await;

        // 1. Setup workspace & user with SE (Sweden) region
        conn.execute("INSERT INTO workspaces (id, name, modules_active, settings) VALUES ('ws-time-test', 'Time Test WS', '[]', '{\"target_region\":\"SE\"}')", ()).await.unwrap();
        conn.execute("INSERT INTO users (id, workspace_id, email, role, preferences) VALUES ('u-time-test', 'ws-time-test', 'test@time.se', 'user', '{\"target_region\":\"SE\", \"allow_overtime\":true}')", ()).await.unwrap();

        // 2. Insert 15 weeks of 48-hour reports to fill the reference period
        // Swedish week is Monday-to-Sunday. Let's insert 48 hours for 15 consecutive weeks (Monday to Friday, 9.6h/day)
        let base_date = date_to_days(2026, 1, 5); // 2026-01-05 is a Monday
        for w in 0..15 {
            let week_monday = base_date + w * 7;
            for d in 0..5 {
                let day_idx = week_monday + d;
                let (y, m, day_val) = format_date_parts_from_days(day_idx);
                let date_str = format!("{:04}-{:02}-{:02}", y, m, day_val);
                conn.execute(
                    "INSERT INTO time_reports (id, workspace_id, user_id, date, hours, note, status, created_at, updated_at, sync_status)
                     VALUES (?1, 'ws-time-test', 'u-time-test', ?2, 9.6, 'Work', 'approved', '2026-07-06', 0, 'synced')",
                    crate::params![format!("tr-{}-{}", w, d), date_str]
                ).await.unwrap();
            }
        }

        // 3. Trying to log 49 hours in the 16th week should fail the rolling average (15 * 48 + 49) / 16 = 48.06 > 48.0
        // We log 9.6h on Mon, Tue, Wed, Thu, Fri, and try to log 1.0h on Sat
        let week16_monday = base_date + 15 * 7;
        for d in 0..5 {
            let day_idx = week16_monday + d;
            let (y, m, day_val) = format_date_parts_from_days(day_idx);
            let date_str = format!("{:04}-{:02}-{:02}", y, m, day_val);
            conn.execute(
                "INSERT INTO time_reports (id, workspace_id, user_id, date, hours, note, status, created_at, updated_at, sync_status)
                 VALUES (?1, 'ws-time-test', 'u-time-test', ?2, 9.6, 'Work', 'approved', '2026-07-06', 0, 'synced')",
                crate::params![format!("tr-15-{}", d), date_str]
            ).await.unwrap();
        }

        // Try logging 1.0h on Saturday of week 16
        let (y, m, day_val) = format_date_parts_from_days(week16_monday + 5);
        let sat_date = format!("{:04}-{:02}-{:02}", y, m, day_val);
        let res = add_time_report(
            "ws-time-test".to_string(),
            "u-time-test".to_string(),
            None,
            sat_date,
            1.0,
            "Extra hours".to_string(),
            None,
            None
        ).await;

        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Rolling 16-week average"));

        // Cleanup
        conn.execute("DELETE FROM time_reports WHERE workspace_id = 'ws-time-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-time-test'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-time-test'", ()).await.unwrap();
    }
}

