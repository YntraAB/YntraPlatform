#[cfg(target_arch = "wasm32")]
use js_sys::Date;

#[cfg(not(target_arch = "wasm32"))]
pub fn get_current_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(target_arch = "wasm32")]
pub fn get_current_time_ms() -> i64 {
    Date::now() as i64
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_current_datetime_str() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let days = secs / 86400;
    let sec_of_day = secs % 86400;
    
    let hour = sec_of_day / 3600;
    let minute = (sec_of_day % 3600) / 60;
    let second = sec_of_day % 60;
    
    let mut year = 1970;
    let mut days_left = days;
    
    loop {
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if is_leap { 366 } else { 365 };
        if days_left < days_in_year {
            break;
        }
        days_left -= days_in_year;
        year += 1;
    }
    
    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let month_days = if is_leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    
    let mut month = 1;
    for &days_in_month in &month_days {
        if days_left < days_in_month {
            break;
        }
        days_left -= days_in_month;
        month += 1;
    }
    
    let day = days_left + 1;
    
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hour, minute, second
    )
}

#[cfg(target_arch = "wasm32")]
pub fn get_current_datetime_str() -> String {
    let date = Date::new_0();
    let iso = date.to_iso_string().as_string().unwrap_or_default();
    if iso.len() >= 19 {
        iso[..19].replace('T', " ")
    } else {
        "2026-07-01 20:23:00".to_string()
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_current_time_str_hm() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let sec_of_day = secs % 86400;
    let hour = sec_of_day / 3600;
    let minute = (sec_of_day % 3600) / 60;
    format!("{:02}:{:02}", hour, minute)
}

#[cfg(target_arch = "wasm32")]
pub fn get_current_time_str_hm() -> String {
    let date = Date::new_0();
    format!("{:02}:{:02}", date.get_utc_hours(), date.get_utc_minutes())
}
