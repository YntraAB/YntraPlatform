#[derive(Clone, Copy, PartialEq)]
pub struct CalendarCell {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub is_current: bool,
    pub is_today: bool,
    pub is_selected: bool,
}

pub fn get_days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

pub fn get_first_day_of_week(year: i32, month: u32) -> u32 {
    let mut y = year;
    let mut m = month;
    if m < 3 {
        m += 12;
        y -= 1;
    }
    let k = y % 100;
    let j = y / 100;
    let h = (1 + (13 * (m as i32 + 1)) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;
    match h {
        0 => 6, // Saturday
        1 => 0, // Sunday
        2 => 1, // Monday
        3 => 2, // Tuesday
        4 => 3, // Wednesday
        5 => 4, // Thursday
        6 => 5, // Friday
        _ => 0,
    }
}

pub fn get_month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}

pub fn get_event_category(title: &str) -> &'static str {
    let lower = title.to_lowercase();
    if lower.contains("medicin") || lower.contains("medication") || lower.contains("assistans") || lower.contains("assistance") || lower.contains("vård") || lower.contains("care") {
        "assistance_time"
    } else if lower.contains("ledig") || lower.contains("leave") || lower.contains("vab") || lower.contains("sjuk") || lower.contains("sick") {
        "unauthorized_absence"
    } else if lower.contains("möte") || lower.contains("meeting") {
        "meeting"
    } else if lower.contains("intro") {
        "introduction"
    } else if lower.contains("utbildning") || lower.contains("training") {
        "training"
    } else {
        "assistance_time"
    }
}

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct TimeRange {
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct BreakInfo {
    pub from: String,
    pub to: String,
    #[serde(rename = "isPaid")]
    pub is_paid: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct EventMetadata {
    pub category: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "waitingTime")]
    pub waiting_time: Option<TimeRange>,
    #[serde(rename = "activeTimes")]
    pub active_times: Option<Vec<TimeRange>>,
    #[serde(rename = "break")]
    pub r#break: Option<BreakInfo>,
    
    // School Extensions
    pub course_id: Option<String>,
    pub classroom: Option<String>,
    
    // Moving Extensions
    pub vehicle_id: Option<String>,
    pub cargo_volume: Option<String>,
    pub destination: Option<String>,
}

pub fn parse_metadata(metadata_str: &str) -> EventMetadata {
    serde_json::from_str(metadata_str).unwrap_or_default()
}

pub struct CategoryConfig {
    pub id: &'static str,
    pub label_key: &'static str,
    pub color: &'static str,
    pub bg_color: &'static str,
    pub icon: &'static str,
}

pub const CATEGORIES: &[CategoryConfig] = &[
    CategoryConfig { id: "assistance_time", label_key: "scheduler-categories-assistance-time", color: "#3B82F6", bg_color: "rgba(59, 130, 246, 0.15)", icon: "user" },
    CategoryConfig { id: "on_call", label_key: "scheduler-categories-on-call", color: "#F59E0B", bg_color: "rgba(245, 158, 11, 0.15)", icon: "radio" },
    CategoryConfig { id: "travel_time", label_key: "scheduler-categories-travel-time", color: "#10B981", bg_color: "rgba(16, 185, 129, 0.15)", icon: "clock" },
    CategoryConfig { id: "introduction", label_key: "scheduler-categories-introduction", color: "#8B5CF6", bg_color: "rgba(139, 92, 246, 0.15)", icon: "book-open" },
    CategoryConfig { id: "meeting", label_key: "scheduler-categories-meeting", color: "#EC4899", bg_color: "rgba(236, 72, 153, 0.15)", icon: "users" },
    CategoryConfig { id: "administrative_hours", label_key: "scheduler-categories-administrative-hours", color: "#6366F1", bg_color: "rgba(99, 102, 241, 0.15)", icon: "file-text" },
    CategoryConfig { id: "training", label_key: "scheduler-categories-training", color: "#F97316", bg_color: "rgba(249, 115, 22, 0.15)", icon: "graduation-cap" },
    CategoryConfig { id: "escort_service", label_key: "scheduler-categories-escort-service", color: "#06B6D4", bg_color: "rgba(6, 182, 212, 0.15)", icon: "accessibility" },
    CategoryConfig { id: "respite_care", label_key: "scheduler-categories-respite-care", color: "#14B8A6", bg_color: "rgba(20, 184, 166, 0.15)", icon: "home" },
    CategoryConfig { id: "unauthorized_absence", label_key: "scheduler-categories-unauthorized-absence", color: "#EF4444", bg_color: "rgba(239, 68, 68, 0.15)", icon: "x-circle" },
    CategoryConfig { id: "involuntary_leave", label_key: "scheduler-categories-involuntary-leave", color: "#F43F5E", bg_color: "rgba(244, 63, 94, 0.15)", icon: "alert-circle" },
    CategoryConfig { id: "other_time", label_key: "scheduler-categories-other-time", color: "#84CC16", bg_color: "rgba(132, 204, 22, 0.15)", icon: "plus" },
    CategoryConfig { id: "customer_staff_note", label_key: "scheduler-categories-customer-staff-note", color: "#D946EF", bg_color: "rgba(217, 70, 239, 0.15)", icon: "message-square" },
    CategoryConfig { id: "severance_pay", label_key: "scheduler-categories-severance-pay", color: "#6B7280", bg_color: "rgba(107, 114, 128, 0.15)", icon: "banknote" },
    CategoryConfig { id: "other", label_key: "scheduler-categories-other", color: "#9CA3AF", bg_color: "rgba(156, 163, 175, 0.15)", icon: "more-horizontal" },
];

pub const CARE_CATEGORIES: &[CategoryConfig] = CATEGORIES;

pub const SCHOOL_CATEGORIES: &[CategoryConfig] = &[
    CategoryConfig { id: "lectures", label_key: "scheduler-categories-lectures", color: "#3B82F6", bg_color: "rgba(59, 130, 246, 0.15)", icon: "graduation-cap" },
    CategoryConfig { id: "lab_slots", label_key: "scheduler-categories-lab-slots", color: "#10B981", bg_color: "rgba(16, 185, 129, 0.15)", icon: "flask" },
    CategoryConfig { id: "grading_hours", label_key: "scheduler-categories-grading-hours", color: "#F59E0B", bg_color: "rgba(245, 158, 11, 0.15)", icon: "file-text" },
    CategoryConfig { id: "exam_invigilation", label_key: "scheduler-categories-exam-invigilation", color: "#8B5CF6", bg_color: "rgba(139, 92, 246, 0.15)", icon: "eye" },
    CategoryConfig { id: "meeting", label_key: "scheduler-categories-meeting", color: "#EC4899", bg_color: "rgba(236, 72, 153, 0.15)", icon: "users" },
    CategoryConfig { id: "other", label_key: "scheduler-categories-other", color: "#9CA3AF", bg_color: "rgba(156, 163, 175, 0.15)", icon: "more-horizontal" },
];

pub const MOVING_CATEGORIES: &[CategoryConfig] = &[
    CategoryConfig { id: "packing", label_key: "scheduler-categories-packing", color: "#8B5CF6", bg_color: "rgba(139, 92, 246, 0.15)", icon: "package" },
    CategoryConfig { id: "loading", label_key: "scheduler-categories-loading", color: "#3B82F6", bg_color: "rgba(59, 130, 246, 0.15)", icon: "truck" },
    CategoryConfig { id: "transport", label_key: "scheduler-categories-transport", color: "#10B981", bg_color: "rgba(16, 185, 129, 0.15)", icon: "navigation" },
    CategoryConfig { id: "unloading", label_key: "scheduler-categories-unloading", color: "#EC4899", bg_color: "rgba(236, 72, 153, 0.15)", icon: "arrow-down-circle" },
    CategoryConfig { id: "vehicle_maintenance", label_key: "scheduler-categories-vehicle-maintenance", color: "#F59E0B", bg_color: "rgba(245, 158, 11, 0.15)", icon: "tool" },
    CategoryConfig { id: "other", label_key: "scheduler-categories-other", color: "#9CA3AF", bg_color: "rgba(156, 163, 175, 0.15)", icon: "more-horizontal" },
];

pub const GENERAL_CATEGORIES: &[CategoryConfig] = &[
    CategoryConfig { id: "meeting", label_key: "scheduler-categories-meeting", color: "#EC4899", bg_color: "rgba(236, 72, 153, 0.15)", icon: "users" },
    CategoryConfig { id: "administrative_hours", label_key: "scheduler-categories-administrative-hours", color: "#6366F1", bg_color: "rgba(99, 102, 241, 0.15)", icon: "file-text" },
    CategoryConfig { id: "training", label_key: "scheduler-categories-training", color: "#F97316", bg_color: "rgba(249, 115, 22, 0.15)", icon: "graduation-cap" },
    CategoryConfig { id: "other", label_key: "scheduler-categories-other", color: "#9CA3AF", bg_color: "rgba(156, 163, 175, 0.15)", icon: "more-horizontal" },
];

pub fn get_categories_for_template(template: yntra_core::WorkspaceTemplateType) -> &'static [CategoryConfig] {
    match template {
        yntra_core::WorkspaceTemplateType::Care => CARE_CATEGORIES,
        yntra_core::WorkspaceTemplateType::School => SCHOOL_CATEGORIES,
        yntra_core::WorkspaceTemplateType::MovingCompany => MOVING_CATEGORIES,
        yntra_core::WorkspaceTemplateType::General => GENERAL_CATEGORIES,
    }
}

pub fn get_category_config(category_id: &str) -> &'static CategoryConfig {
    if let Some(c) = CATEGORIES.iter().find(|c| c.id == category_id) {
        return c;
    }
    if let Some(c) = SCHOOL_CATEGORIES.iter().find(|c| c.id == category_id) {
        return c;
    }
    if let Some(c) = MOVING_CATEGORIES.iter().find(|c| c.id == category_id) {
        return c;
    }
    if let Some(c) = GENERAL_CATEGORIES.iter().find(|c| c.id == category_id) {
        return c;
    }
    &CATEGORIES[14]
}

pub fn get_event_category_config(title: &str, metadata_str: &str) -> &'static CategoryConfig {
    let meta = parse_metadata(metadata_str);
    if let Some(ref cat) = meta.category {
        get_category_config(cat)
    } else {
        get_category_config(get_event_category(title))
    }
}

pub fn parse_time_from_str(time_str: &str) -> Option<(u32, u32)> {
    let time_part = if time_str.contains('T') {
        time_str.split('T').next_back()
    } else {
        time_str.split(' ').next_back()
    }?;
    let parts: Vec<&str> = time_part.split(':').collect();
    if parts.len() >= 2 {
        let hour = parts[0].parse::<u32>().ok()?;
        let minute = parts[1].parse::<u32>().ok()?;
        Some((hour, minute))
    } else {
        None
    }
}

pub fn calculate_event_top(start_time: &str, hour_height: f64, start_hour: u32) -> f64 {
    if let Some((hour, minute)) = parse_time_from_str(start_time) {
        let h = hour as f64;
        let m = minute as f64;
        (h - start_hour as f64 + m / 60.0) * hour_height
    } else {
        0.0
    }
}

pub fn calculate_event_height(start_time: &str, end_time: &str, hour_height: f64) -> f64 {
    let start = parse_time_from_str(start_time);
    let end = parse_time_from_str(end_time);
    if let (Some((sh, sm)), Some((eh, em))) = (start, end) {
        let duration_hours = (eh as f64 + em as f64 / 60.0) - (sh as f64 + sm as f64 / 60.0);
        duration_hours * hour_height
    } else {
        hour_height
    }
}

pub fn format_time_range(start_time: &str, end_time: &str) -> String {
    let start = parse_time_from_str(start_time);
    let end = parse_time_from_str(end_time);
    if let (Some((sh, sm)), Some((eh, em))) = (start, end) {
        format!("{:02}:{:02} - {:02}:{:02}", sh, sm, eh, em)
    } else {
        "N/A".to_string()
    }
}

pub fn update_date_in_time_str(time_str: &str, new_date: &str) -> String {
    let parts: Vec<&str> = time_str.split(' ').collect();
    if parts.len() == 2 {
        format!("{} {}", new_date, parts[1])
    } else {
        format!("{} {}", new_date, time_str)
    }
}

