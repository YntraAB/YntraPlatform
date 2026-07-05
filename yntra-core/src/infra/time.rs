use chrono::{Utc};

pub fn get_current_time_ms() -> i64 {
    Utc::now().timestamp_millis()
}

pub fn get_current_datetime_str() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn get_current_time_str_hm() -> String {
    Utc::now().format("%H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime_format() {
        let dt = get_current_datetime_str();
        assert_eq!(dt.len(), 19);
        assert_eq!(&dt[4..5], "-");
        assert_eq!(&dt[7..8], "-");
        assert_eq!(&dt[10..11], " ");
        assert_eq!(&dt[13..14], ":");
        assert_eq!(&dt[16..17], ":");
    }

    #[test]
    fn test_hm_format() {
        let hm = get_current_time_str_hm();
        assert_eq!(hm.len(), 5);
        assert_eq!(&hm[2..3], ":");
    }
}
