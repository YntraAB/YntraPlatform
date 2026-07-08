use crate::YntraError;

pub fn get_week_start_days(target_days: i32, week_start_day: i32) -> i32 {
    let day_of_week = (target_days + 2) % 7; // 0 = Monday, 1 = Tuesday ... 6 = Sunday
    let target_start = if week_start_day == 0 || week_start_day == 7 {
        6 // Sunday
    } else {
        week_start_day - 1 // Monday = 0, Tuesday = 1, etc.
    };
    let diff = (day_of_week - target_start + 7) % 7;
    target_days - diff
}

pub fn parse_date(date_str: &str) -> Option<(i32, i32, i32)> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year = parts[0].parse::<i32>().ok()?;
    let month = parts[1].parse::<i32>().ok()?;
    let day = parts[2].parse::<i32>().ok()?;
    if year < 1970 || month < 1 || month > 12 || day < 1 || day > 31 {
        return None;
    }
    Some((year, month, day))
}

pub fn date_to_days(year: i32, month: i32, day: i32) -> i32 {
    let m = (month + 9) % 12;
    let y = year - m / 10;
    365 * y + y / 4 - y / 100 + y / 400 + (m * 306 + 5) / 10 + (day - 1)
}

pub fn parse_time_to_minutes(time_str: &str) -> Option<i32> {
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

pub fn get_report_interval(
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

pub fn resolve_fallback_interval_for_day(
    day_start_min: i32,
    hours: f64,
    all_existing_intervals: &[(i32, i32)],
    mandatory_rest_min: i32,
) -> (i32, i32) {
    let duration_min = (hours * 60.0) as i32;
    let mut candidate_start = day_start_min + 8 * 60; // Start at 08:00
    
    loop {
        let candidate_end = candidate_start + duration_min;
        let mut conflict = false;
        
        // 1. Check for overlaps with ANY shift
        for &(estart, eend) in all_existing_intervals {
            if candidate_start < eend && estart < candidate_end {
                conflict = true;
                candidate_start = eend + 30; // Move start time past overlap
                break;
            }
        }
        
        // 2. Check if it satisfies mandatory daily rest relative to adjacent shifts
        if !conflict && mandatory_rest_min > 0 {
            for &(estart, eend) in all_existing_intervals {
                if eend <= candidate_start && (candidate_start - eend) < mandatory_rest_min {
                    conflict = true;
                    candidate_start = eend + mandatory_rest_min;
                    break;
                }
                if estart >= candidate_end && (estart - candidate_end) < mandatory_rest_min {
                    conflict = true;
                    candidate_start = estart + 30; // Jump past it
                    break;
                }
            }
        }
        
        if !conflict {
            return (candidate_start, candidate_end);
        }
    }
}

pub fn format_abs_minutes_to_time(abs_min: i32) -> String {
    let day_min = abs_min % 1440;
    let h = day_min / 60;
    let m = day_min % 60;
    format!("{:02}:{:02}", h, m)
}

pub fn format_date_from_days(days: i32) -> String {
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

pub fn format_abs_minutes_to_datetime(abs_min: i32) -> String {
    let days = abs_min / 1440;
    let date_str = format_date_from_days(days);
    let time_str = format_abs_minutes_to_time(abs_min);
    format!("{} {}", date_str, time_str)
}

pub fn get_hours_on_day(day_index: i32, intervals: &[(i32, i32)]) -> f64 {
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

pub fn get_dst_offset_change(y: i32, m: i32, d: i32, region: &str) -> i32 {
    if region == "US-AZ" || region == "US-HI" || region == "CA-SK" {
        return 0;
    }
    let days = date_to_days(y, m, d);
    let is_sunday = (days + 3) % 7 == 0;
    if !is_sunday {
        return 0;
    }
    let is_eu = matches!(region, "SE" | "NO" | "DK" | "FI" | "EU" | "GB" | "IE");
    let is_us = region.starts_with("US") || region.starts_with("CA");

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

pub fn format_date_parts_from_days(days: i32) -> (i32, i32, i32) {
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

pub fn adjust_duration_for_dst(start_abs: i32, end_abs: i32, region: &str) -> i32 {
    let start_day = start_abs / 1440;
    let end_day = end_abs / 1440;
    let mut adjustment = 0;
    for day_idx in start_day..=end_day {
        let is_sunday = (day_idx + 3) % 7 == 0;
        if !is_sunday {
            continue;
        }
        let (y, m, d) = format_date_parts_from_days(day_idx);
        let change = get_dst_offset_change(y, m, d, region);
        if change != 0 {
            let transition_hour = if region.starts_with("US") || region.starts_with("CA") {
                120
            } else if change < 0 {
                // Spring forward transitions
                if region == "FI" {
                    180
                } else if region == "GB" || region == "IE" {
                    60
                } else {
                    120
                }
            } else {
                // Autumn fallback transitions (local time moves back)
                if region == "FI" {
                    240
                } else if region == "GB" || region == "IE" {
                    120
                } else {
                    180
                }
            };
            let transition_abs = day_idx * 1440 + transition_hour;
            if start_abs <= transition_abs && transition_abs <= end_abs {
                adjustment += change;
            }
        }
    }
    adjustment
}

pub fn check_weekly_rest_for_week(
    week_start_days: i32,
    sorted_intervals: &[(i32, i32)],
    target_region: &str,
    weekly_rest_limit_min: i32,
    law_name: &str,
) -> Result<(), YntraError> {
    let week_start_min = week_start_days * 1440;
    let week_end_min = (week_start_days + 7) * 1440;
    
    // Early exit: if the worker has no shifts in the target week, they are automatically compliant.
    let has_shifts_in_target_week = sorted_intervals.iter().any(|&(s, e)| {
        s < week_end_min && e > week_start_min
    });
    if !has_shifts_in_target_week {
        return Ok(());
    }
    
    let start_days = week_start_days - 15 * 7;
    let end_days = week_start_days + 16 * 7;
    
    let window_start_min = start_days * 1440;
    let window_end_min = end_days * 1440;
    
    let mut intervals = Vec::new();
    intervals.push((window_start_min - 1, window_start_min - 1));
    for &(s, e) in sorted_intervals {
        if s < window_end_min && e > window_start_min {
            intervals.push((s, e));
        }
    }
    intervals.push((window_end_min + 1, window_end_min + 1));
    intervals.sort_by_key(|x| x.0);
    
    let mut has_compliant_rest = false;
    
    for idx in 0..intervals.len().saturating_sub(1) {
        let gap_start = intervals[idx].1;
        let gap_end = intervals[idx + 1].0;
        
        let overlap_start = gap_start.max(week_start_min);
        let overlap_end = gap_end.min(week_end_min);
        
        if overlap_start < overlap_end {
            let gap_dst = adjust_duration_for_dst(gap_start, gap_end, target_region);
            let gap_duration = gap_end - gap_start + gap_dst;
            
            let overlap_dst = adjust_duration_for_dst(overlap_start, overlap_end, target_region);
            let overlap_duration = overlap_end - overlap_start + overlap_dst;
            
            // Overlap with the calendar week must be at least 12 hours (720 mins) OR it must span across the week transition (gap starts in this week and ends in the next)
            // and the total consecutive rest period must satisfy the weekly rest limit
            let is_transition_span = gap_start < week_end_min && gap_end >= week_end_min;
            if (overlap_duration >= 12 * 60 || is_transition_span) && gap_duration >= weekly_rest_limit_min {
                has_compliant_rest = true;
                break;
            }
        }
    }
    
    if !has_compliant_rest {
        return Err(YntraError::ValidationError(format!(
            "Weekly rest period violation under {}: does not satisfy mandatory {:.1}h consecutive weekly rest period in the 7-day period starting at {}.",
            law_name, (weekly_rest_limit_min as f64 / 60.0), format_date_from_days(week_start_days)
        )));
    }
    
    Ok(())
}
