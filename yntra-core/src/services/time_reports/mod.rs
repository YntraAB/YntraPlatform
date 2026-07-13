pub mod validation;

use crate::database;
use crate::observer::notify_observers;
use crate::{TimeReport, YntraError};
use validation::*;

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
    requester_user_id: String,
    workspace_id: String,
    user_id: String,
    team_id: Option<String>,
    date: String,
    hours: f64,
    note: String,
    start_time: Option<String>,
    end_time: Option<String>,
) -> Result<TimeReport, YntraError> {
    if hours <= 0.0 {
        return Err(YntraError::ValidationError("Logged hours must be greater than zero".to_string()));
    }

    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }
    if auth.role != "platform_admin" && auth.role != "admin" && requester_user_id != user_id {
        return Err(YntraError::AuthError("Access denied: cannot add time report for another user".to_string()));
    }

    let (target_user_ws, user_prefs_json_raw, settings_json_raw): (String, Option<String>, Option<String>) = conn.query_row(
        "SELECT u.workspace_id, u.preferences, w.settings \
         FROM users u \
         LEFT JOIN workspaces w ON u.workspace_id = w.id \
         WHERE u.id = ?1",
        crate::params![&user_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    ).await.map_err(|_| YntraError::NotFoundError("User not found".to_string()))?;

    if target_user_ws != workspace_id {
        return Err(YntraError::ValidationError("User does not belong to the specified workspace".to_string()));
    }

    // 1. Calculate dates and convert target date to days
    let target_days = match parse_date(&date) {
        Some((y, m, d)) => date_to_days(y, m, d),
        None => return Err(YntraError::ValidationError("Invalid date format, expected YYYY-MM-DD".to_string())),
    };

    // 2. Fetch logged hours, workspace settings, and user preferences to determine national limits
    let (settings, user_prefs_json, user_reports) = {
        let settings_json = if auth.role != "platform_admin" && auth.workspace_id == workspace_id {
            auth.workspace_settings.clone().unwrap_or_else(|| "{}".to_string())
        } else {
            settings_json_raw.unwrap_or_else(|| "{}".to_string())
        };

        let user_prefs_json = user_prefs_json_raw.unwrap_or_else(|| "{}".to_string());

        let settings: serde_json::Value = serde_json::from_str(&settings_json).unwrap_or(serde_json::Value::Null);
        let week_start_day = settings.get("week_start")
            .and_then(|v| v.as_i64())
            .unwrap_or(1) as i32;

        let target_week_start = get_week_start_days(target_days, week_start_day);

        // Fetch user reports in a sliding window to optimize performance and prevent scaling issues
        let start_days = target_week_start - 15 * 7 - 1;
        let end_days = target_week_start + 16 * 7;
        let start_date_str = format_date_from_days(start_days);
        let end_date_str = format_date_from_days(end_days);

        let mut user_reports = Vec::new();
        let mut stmt = conn.prepare("SELECT date, hours, start_time, end_time FROM time_reports WHERE user_id = ?1 AND date >= ?2 AND date <= ?3").await?;
        let mut rows = stmt.query(crate::params![&user_id, start_date_str, end_date_str]).await?;
        while let Some(row) = rows.next().await? {
            let r_date: String = row.get(0)?;
            let r_hours: f64 = row.get(1)?;
            let r_start: Option<String> = row.get(2)?;
            let r_end: Option<String> = row.get(3)?;
            user_reports.push((r_date, r_hours, r_start, r_end));
        }
        (settings, user_prefs_json, user_reports)
    };

    // Parse configuration fields
    let u_prefs: serde_json::Value = serde_json::from_str(&user_prefs_json).unwrap_or(serde_json::Value::Null);

    let target_region_raw = u_prefs.get("target_region")
        .or_else(|| settings.get("target_region"))
        .and_then(|v| v.as_str())
        .unwrap_or("EU");
    let target_region = target_region_raw.to_uppercase();

    let week_start_day = settings.get("week_start")
        .and_then(|v| v.as_i64())
        .unwrap_or(1) as i32;
    let target_week_start = get_week_start_days(target_days, week_start_day);

    let allow_overtime = u_prefs.get("allow_overtime")
        .or_else(|| settings.get("allow_overtime"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let allow_union_exempt = u_prefs.get("allow_union_exempt")
        .or_else(|| settings.get("allow_union_exempt"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Resolve rules from Compliance Registry
    let rule = crate::infra::compliance::ComplianceRegistry::get_rule(&target_region);
    let daily_limit = if allow_overtime { rule.max_daily_limit_with_overtime } else { rule.standard_daily_limit };
    let weekly_limit = if allow_overtime || allow_union_exempt { rule.max_weekly_limit_with_exemption } else { rule.standard_weekly_limit };

    let mandatory_rest_hours_limit = if rule.mandatory_daily_rest_hours > 0.0 {
        if allow_union_exempt { 8.0 } else { rule.mandatory_daily_rest_hours }
    } else {
        0.0
    };
    let mandatory_rest_min = (mandatory_rest_hours_limit * 60.0) as i32;

    // Convert existing user reports to absolute minute intervals
    let mut existing_intervals = Vec::new();
    for (r_date, r_hrs, r_start, r_end) in &user_reports {
        if let Some(interval) = get_report_interval(r_date, *r_hrs, r_start.as_deref(), r_end.as_deref()) {
            existing_intervals.push(interval);
        }
    }

    // 3. Resolve interval bounds for new report
    let (new_start_abs, new_end_abs) = if let (Some(s_str), Some(e_str)) = (start_time.as_deref(), end_time.as_deref()) {
        match get_report_interval(&date, hours, Some(s_str), Some(e_str)) {
            Some(interval) => interval,
            None => return Err(YntraError::ValidationError("Invalid start_time or end_time format (expected HH:MM)".to_string())),
        }
    } else {
        // Omitted shift times: resolve a non-overlapping fallback interval
        let (y, m, d) = match parse_date(&date) {
            Some(parts) => parts,
            None => return Err(YntraError::ValidationError("Invalid date format, expected YYYY-MM-DD".to_string())),
        };
        let day_start_min = date_to_days(y, m, d) * 1440;
        resolve_fallback_interval_for_day(day_start_min, hours, &existing_intervals, mandatory_rest_min)
    };

    let start_day_idx = new_start_abs / 1440;
    let end_day_idx = (new_end_abs - 1) / 1440;

    let dst_adj = adjust_duration_for_dst(new_start_abs, new_end_abs, &target_region);
    let interval_duration_hrs = (new_end_abs - new_start_abs + dst_adj) as f64 / 60.0;
    if hours > interval_duration_hrs {
        return Err(YntraError::ValidationError(format!(
            "Logged hours ({:.2}h) cannot exceed the shift duration ({:.2}h from {} to {})",
            hours, interval_duration_hrs, start_time.as_deref().unwrap_or(""), end_time.as_deref().unwrap_or("")
        )));
    }

    // 4. Validate daily limit on all affected days
    let mut all_intervals = existing_intervals.clone();
    all_intervals.push((new_start_abs, new_end_abs));

    for day_idx in start_day_idx..=end_day_idx {
        let daily_logged_on_day = get_hours_on_day(day_idx, &all_intervals);
        if daily_logged_on_day > daily_limit {
            let day_date_str = format_date_from_days(day_idx);
            let msg = match target_region.as_str() {
                "NO" | "SE" | "DK" | "FI" => format!(
                    "Daily working hours limit ({}h) exceeded under {} on {}. Logged on this day: {:.2}h.",
                    daily_limit, rule.law_name, day_date_str, daily_logged_on_day
                ),
                r if r.starts_with("US") => format!(
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
        let mandatory_rest_hours_limit = if allow_union_exempt {
            8.0 // Reduced to 8 hours under collective agreements / union exemptions in Nordics/EU
        } else {
            rule.mandatory_daily_rest_hours
        };
        let mandatory_rest_min = (mandatory_rest_hours_limit * 60.0) as i32;
        
        let mut sorted_intervals = all_intervals.clone();
        sorted_intervals.sort_by_key(|x| x.0);
        
        for i in 0..sorted_intervals.len() {
            let (s_start, _s_end) = sorted_intervals[i];
            let window_start = s_start;
            let dst_change_in_window = adjust_duration_for_dst(window_start, window_start + 1440, &target_region);
            let window_end = s_start + 1440 + dst_change_in_window;
            
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
                    let gap_dst = adjust_duration_for_dst(current_point, seg_start, &target_region);
                    let rest_gap = seg_start - current_point + gap_dst;
                    if rest_gap > max_rest {
                        max_rest = rest_gap;
                    }
                }
                current_point = current_point.max(seg_end);
            }
            
            if window_end > current_point {
                let gap_dst = adjust_duration_for_dst(current_point, window_end, &target_region);
                let rest_gap = window_end - current_point + gap_dst;
                if rest_gap > max_rest {
                    max_rest = rest_gap;
                }
            }
            
            if max_rest < mandatory_rest_min {
                return Err(YntraError::ValidationError(format!(
                    "Daily working hours violation under {}: does not satisfy mandatory {}h consecutive daily rest period in the 24h window starting at {}",
                    rule.law_name, mandatory_rest_hours_limit, format_abs_minutes_to_datetime(s_start)
                )));
            }
        }
    }

    // 5. Validate weekly limit (fixed calendar week containing target_days)
    let week_start_min = target_week_start * 1440;
    let week_end_min = week_start_min + 7 * 1440;
    let mut weekly_logged = 0.0;
    for &(start, end) in &existing_intervals {
        let overlap_start = start.max(week_start_min);
        let overlap_end = end.min(week_end_min);
        if overlap_start < overlap_end {
            weekly_logged += (overlap_end - overlap_start) as f64 / 60.0;
        }
    }

    let skip_calendar_weekly_cap = allow_overtime;

    if !skip_calendar_weekly_cap && weekly_logged + hours > weekly_limit {
        let msg = match target_region.as_str() {
            "NO" | "SE" | "DK" | "FI" => format!(
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

    // Check rolling 16-week average weekly limit of 48h for Nordic/EU regions across all 16 windows containing the target week
    if matches!(target_region.as_str(), "SE" | "NO" | "DK" | "FI" | "EU") {
        for offset in 0..16 {
            let w_start = target_week_start + (offset - 15) * 7;
            let w_start_min = w_start * 1440;
            let w_end_min = w_start_min + 16 * 7 * 1440;
            let mut rolling_logged = 0.0;
            for &(start, end) in &all_intervals {
                let overlap_start = start.max(w_start_min);
                let overlap_end = end.min(w_end_min);
                if overlap_start < overlap_end {
                    rolling_logged += (overlap_end - overlap_start) as f64 / 60.0;
                }
            }

            let rolling_average = rolling_logged / 16.0;
            if rolling_average > 48.0 {
                return Err(YntraError::ValidationError(format!(
                    "Rolling 16-week average weekly working hours ({:.2}h) exceeds the legal limit of 48.0h under {} in the 16-week window starting at {}.",
                    rolling_average, rule.law_name, format_date_from_days(w_start)
                )));
            }
        }
    }

    // Weekly rest period check for EU/Nordic regions (consecutive 35h or 36h in any rolling 7-day period)
    if matches!(target_region.as_str(), "SE" | "NO" | "DK" | "FI" | "EU") {
        let weekly_rest_limit_hrs = if target_region == "SE" { 36.0 } else { 35.0 };
        let weekly_rest_limit_min = (weekly_rest_limit_hrs * 60.0) as i32;
        
        let mut sorted_intervals = all_intervals.clone();
        sorted_intervals.sort_by_key(|x| x.0);
        
        // EU/Nordic laws mandate weekly rest in each period of seven days (rolling window)
        // We verify all rolling 7-day windows containing any day of the new shift.
        for win_start in (start_day_idx - 6)..=end_day_idx {
            check_weekly_rest_for_week(win_start, &sorted_intervals, &target_region, weekly_rest_limit_min, rule.law_name)?;
        }
    }

    // California 7-day rule check
    if target_region.as_str() == "US-CA" && !allow_overtime {
        let mut active_days = std::collections::HashSet::new();
        let mut weekly_total_hours = 0.0;
        let mut max_daily_hours = 0.0;
        let mut daily_hours_map = std::collections::HashMap::new();
        
        for &(start, end) in &all_intervals {
            let start_day = start / 1440;
            let end_day = (end - 1) / 1440;
            for d in start_day..=end_day {
                if d >= target_week_start && d < target_week_start + 7 {
                    active_days.insert(d);
                    
                    let day_start = d * 1440;
                    let day_end = day_start + 1440;
                    let overlap_start = start.max(day_start);
                    let overlap_end = end.min(day_end);
                    if overlap_start < overlap_end {
                        let hrs = (overlap_end - overlap_start) as f64 / 60.0;
                        *daily_hours_map.entry(d).or_insert(0.0) += hrs;
                    }
                }
            }
        }
        
        for &hrs in daily_hours_map.values() {
            weekly_total_hours += hrs;
            if hrs > max_daily_hours {
                max_daily_hours = hrs;
            }
        }
        
        if active_days.len() >= 7 {
            // Apply Section 554 exceptions: exempt if weekly total <= 30h and daily <= 6h on all days
            let is_exempt = weekly_total_hours <= 30.0 && max_daily_hours <= 6.0;
            if !is_exempt {
                return Err(YntraError::ValidationError(format!(
                    "California Labor Code violation under {}: working 7 consecutive days in a workweek is prohibited without overtime permission (Exceeds part-time limits: weekly hours: {:.2}h, max daily hours: {:.2}h).",
                    rule.law_name, weekly_total_hours, max_daily_hours
                )));
            }
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let created_at = crate::infra::time::get_current_datetime_str();
    let now_ms = crate::infra::time::get_current_time_ms();
    
    let item = TimeReport {
        id: id.clone(),
        workspace_id,
        user_id: user_id.clone(),
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

    conn.begin_transaction().await?;
    let res = async {
        crate::services::audit::log_action_with_conn(&conn, user_id, None, "add_time_report".to_string()).await?;
        
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
        Ok::<(), YntraError>(())
    }.await;

    match res {
        Ok(_) => {
            conn.commit().await?;
            notify_observers();
            Ok(item)
        }
        Err(e) => {
            let _ = conn.rollback().await;
            Err(e)
        }
    }
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

#[uniffi::export]
pub async fn get_time_reports_rkyv(requester_user_id: String, user_id: Option<String>) -> Result<Vec<u8>, YntraError> {
    let reports = get_time_reports(requester_user_id, user_id).await?;
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&reports)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    Ok(bytes.into_vec())
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
        assert_eq!(parse_date("1969-12-31"), None);
        assert_eq!(parse_date("2026-13-01"), None);
        assert_eq!(parse_date("2026-07-32"), None);
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
        let pre_leap = date_to_days(2024, 2, 28);
        let leap_day = date_to_days(2024, 2, 29);
        let post_leap = date_to_days(2024, 3, 1);

        assert_eq!(leap_day - pre_leap, 1);
        assert_eq!(post_leap - leap_day, 1);

        let normal_feb28 = date_to_days(2025, 2, 28);
        let normal_mar01 = date_to_days(2025, 3, 1);
        assert_eq!(normal_mar01 - normal_feb28, 1);
    }

    #[test]
    fn test_dst_offset_math() {
        assert_eq!(get_dst_offset_change(2026, 3, 29, "SE"), -60);
        assert_eq!(get_dst_offset_change(2026, 10, 25, "NO"), 60);
        assert_eq!(get_dst_offset_change(2026, 3, 28, "SE"), 0);
        assert_eq!(get_dst_offset_change(2026, 6, 15, "SE"), 0);
    }

    #[test]
    fn test_negative_days_date_roundtrip() {
        let y = 1969;
        let m = 12;
        let d = 31;
        let days = date_to_days(y, m, d);
        let date_str = format_date_from_days(days);
        assert_eq!(date_str, "1969-12-31");

        let (ry, rm, rd) = format_date_parts_from_days(days);
        assert_eq!((ry, rm, rd), (1969, 12, 31));

        let leap_days = date_to_days(1968, 2, 29);
        assert_eq!(format_date_from_days(leap_days), "1968-02-29");
    }

    #[test]
    fn test_daily_hours_on_day_partitioning() {
        let j5_idx = date_to_days(2026, 7, 5);
        let j5_start = j5_idx * 1440;
        
        let shift = (j5_start + 1320, j5_start + 1800);
        let intervals = vec![shift];
        
        assert_eq!(get_hours_on_day(j5_idx, &intervals), 2.0);
        assert_eq!(get_hours_on_day(j5_idx + 1, &intervals), 6.0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_rolling_average_weekly_limit() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let _ = conn.execute("DELETE FROM time_reports WHERE workspace_id = 'ws-time-test'", ()).await;
        let _ = conn.execute("DELETE FROM users WHERE workspace_id = 'ws-time-test'", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = 'ws-time-test'", ()).await;

        conn.execute("INSERT INTO workspaces (id, name, modules_active, settings) VALUES ('ws-time-test', 'Time Test WS', '[]', '{\"target_region\":\"SE\"}')", ()).await.unwrap();
        conn.execute("INSERT INTO users (id, workspace_id, email, role, preferences) VALUES ('u-time-test', 'ws-time-test', 'test@time.se', 'user', '{\"target_region\":\"SE\", \"allow_overtime\":true}')", ()).await.unwrap();

        let base_date = date_to_days(2026, 1, 5);
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

        let (y, m, day_val) = format_date_parts_from_days(week16_monday + 5);
        let sat_date = format!("{:04}-{:02}-{:02}", y, m, day_val);
        let res = add_time_report(
            "u-time-test".to_string(),
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

        conn.execute("DELETE FROM time_reports WHERE workspace_id = 'ws-time-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-time-test'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-time-test'", ()).await.unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_weekend_spanning_rest() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let _ = conn.execute("DELETE FROM time_reports WHERE workspace_id = 'ws-rest-test'", ()).await;
        let _ = conn.execute("DELETE FROM users WHERE workspace_id = 'ws-rest-test'", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = 'ws-rest-test'", ()).await;

        conn.execute("INSERT INTO workspaces (id, name, modules_active, settings) VALUES ('ws-rest-test', 'Rest Test WS', '[]', '{\"target_region\":\"SE\"}')", ()).await.unwrap();
        conn.execute("INSERT INTO users (id, workspace_id, email, role, preferences) VALUES ('u-rest-test', 'ws-rest-test', 'test@rest.se', 'user', '{\"target_region\":\"SE\", \"allow_overtime\":false}')", ()).await.unwrap();

        add_time_report(
            "u-rest-test".to_string(),
            "ws-rest-test".to_string(),
            "u-rest-test".to_string(),
            None,
            "2026-07-10".to_string(),
            8.0,
            "Friday Shift".to_string(),
            Some("08:00".to_string()),
            Some("16:00".to_string()),
        ).await.unwrap();

        let res = add_time_report(
            "u-rest-test".to_string(),
            "ws-rest-test".to_string(),
            "u-rest-test".to_string(),
            None,
            "2026-07-13".to_string(),
            8.0,
            "Monday Shift".to_string(),
            Some("09:00".to_string()),
            Some("17:00".to_string()),
        ).await;

        assert!(res.is_ok(), "Weekend-spanning rest should not violate weekly rest limit: {:?}", res.err());

        conn.execute("DELETE FROM time_reports WHERE workspace_id = 'ws-rest-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-rest-test'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-rest-test'", ()).await.unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_california_7day_part_time_exemption() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let _ = conn.execute("DELETE FROM time_reports WHERE workspace_id = 'ws-ca-test'", ()).await;
        let _ = conn.execute("DELETE FROM users WHERE workspace_id = 'ws-ca-test'", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id = 'ws-ca-test'", ()).await;

        conn.execute("INSERT INTO workspaces (id, name, modules_active, settings) VALUES ('ws-ca-test', 'CA Test WS', '[]', '{\"target_region\":\"US-CA\"}')", ()).await.unwrap();
        conn.execute("INSERT INTO users (id, workspace_id, email, role, preferences) VALUES ('u-ca-test', 'ws-ca-test', 'test@ca.us', 'user', '{\"target_region\":\"US-CA\", \"allow_overtime\":false}')", ()).await.unwrap();

        let base_date = date_to_days(2026, 7, 6);
        for d in 0..6 {
            let day_idx = base_date + d;
            let (y, m, day_val) = format_date_parts_from_days(day_idx);
            let date_str = format!("{:04}-{:02}-{:02}", y, m, day_val);
            add_time_report(
                "u-ca-test".to_string(),
                "ws-ca-test".to_string(),
                "u-ca-test".to_string(),
                None,
                date_str,
                3.0,
                "Part-time Shift".to_string(),
                Some("09:00".to_string()),
                Some("12:00".to_string()),
            ).await.unwrap();
        }

        let sunday_idx = base_date + 6;
        let (y, m, day_val) = format_date_parts_from_days(sunday_idx);
        let sunday_date = format!("{:04}-{:02}-{:02}", y, m, day_val);
        let res = add_time_report(
            "u-ca-test".to_string(),
            "ws-ca-test".to_string(),
            "u-ca-test".to_string(),
            None,
            sunday_date,
            3.0,
            "Sunday Shift".to_string(),
            Some("09:00".to_string()),
            Some("12:00".to_string()),
        ).await;

        assert!(res.is_ok(), "Part-time worker should be exempt from the 7-day rule: {:?}", res.err());

        let _ = conn.execute("DELETE FROM time_reports WHERE date = ?1 AND user_id = 'u-ca-test'", crate::params![format_date_from_days(sunday_idx)]).await;
        
        let res_fail = add_time_report(
            "u-ca-test".to_string(),
            "ws-ca-test".to_string(),
            "u-ca-test".to_string(),
            None,
            format_date_from_days(sunday_idx),
            7.0,
            "Long Sunday Shift".to_string(),
            Some("09:00".to_string()),
            Some("16:00".to_string()),
        ).await;

        assert!(res_fail.is_err(), "Exceeding daily 6h limit on 7th day should trigger violation");
        assert!(res_fail.unwrap_err().to_string().contains("California Labor Code violation"));

        conn.execute("DELETE FROM time_reports WHERE workspace_id = 'ws-ca-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-ca-test'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-ca-test'", ()).await.unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_add_time_report_workspace_mismatch() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let _ = conn.execute("DELETE FROM users WHERE workspace_id IN ('ws-time-a', 'ws-time-b')", ()).await;
        let _ = conn.execute("DELETE FROM workspaces WHERE id IN ('ws-time-a', 'ws-time-b')", ()).await;

        conn.execute("INSERT INTO workspaces (id, name, modules_active, settings) VALUES ('ws-time-a', 'WS A', '[]', '{\"target_region\":\"SE\"}')", ()).await.unwrap();
        conn.execute("INSERT INTO workspaces (id, name, modules_active, settings) VALUES ('ws-time-b', 'WS B', '[]', '{\"target_region\":\"SE\"}')", ()).await.unwrap();

        // Admin of WS A
        conn.execute("INSERT INTO users (id, workspace_id, email, role, preferences) VALUES ('u-admin-a', 'ws-time-a', 'admina@time.se', 'admin', '{}')", ()).await.unwrap();
        
        // User of WS B
        conn.execute("INSERT INTO users (id, workspace_id, email, role, preferences) VALUES ('u-user-b', 'ws-time-b', 'userb@time.se', 'user', '{}')", ()).await.unwrap();

        // Admin of WS A tries to log time report for User of WS B -> should fail with AuthError
        let res = add_time_report(
            "u-admin-a".to_string(),
            "ws-time-a".to_string(),
            "u-user-b".to_string(),
            None,
            "2026-07-10".to_string(),
            8.0,
            "Friday Shift".to_string(),
            Some("08:00".to_string()),
            Some("16:00".to_string()),
        ).await;

        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), YntraError::ValidationError(_)));

        conn.execute("DELETE FROM users WHERE workspace_id IN ('ws-time-a', 'ws-time-b')", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id IN ('ws-time-a', 'ws-time-b')", ()).await.unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_get_time_reports_rkyv_serialization() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-time-rkyv', 'Rkyv WS', '[]', '{\"target_region\":\"SE\"}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, preferences) VALUES ('u-time-rkyv', 'ws-time-rkyv', 'rkyv@time.se', 'user', '{}')", ()).await.unwrap();

        conn.execute(
            "INSERT INTO time_reports (id, workspace_id, user_id, date, hours, note, status, created_at, updated_at, sync_status)
             VALUES ('tr-rkyv-1', 'ws-time-rkyv', 'u-time-rkyv', '2026-07-06', 8.0, 'Work', 'approved', '2026-07-06', 0, 'synced')",
            ()
        ).await.unwrap();

        let bytes = get_time_reports_rkyv("u-time-rkyv".to_string(), Some("u-time-rkyv".to_string())).await.unwrap();
        let rkyv_reports: Vec<TimeReport> = rkyv::from_bytes::<Vec<TimeReport>, rkyv::rancor::Error>(&bytes).unwrap();
        assert_eq!(rkyv_reports.len(), 1);
        assert_eq!(rkyv_reports[0].id, "tr-rkyv-1");

        conn.execute("DELETE FROM time_reports WHERE id = 'tr-rkyv-1'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-time-rkyv'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-time-rkyv'", ()).await.unwrap();
    }
}
