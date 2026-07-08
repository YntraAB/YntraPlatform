pub fn normalize_swedish_pnum(pnum: &str, current_year: i32) -> Option<String> {
    let clean = pnum.trim();
    let digits_only: String = clean.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits_only.len() == 12 {
        return Some(digits_only);
    }
    if digits_only.len() != 10 {
        return None;
    }
    let is_over_100 = clean.contains('+');
    let yy = digits_only[0..2].parse::<i32>().ok()?;
    let mm_dd_xxxx = &digits_only[2..10];
    
    let current_year_short = current_year % 100;
    let current_century = current_year / 100 * 100;
    
    let mut year = current_century + yy;
    if yy > current_year_short {
        year -= 100;
    }
    if is_over_100 {
        year -= 100;
    }
    Some(format!("{:04}{}", year, mm_dd_xxxx))
}

pub fn personal_numbers_match(p1: &str, p2: &str) -> bool {
    let current_year = chrono::Utc::now().format("%Y").to_string().parse::<i32>().unwrap_or(2026);
    if let (Some(n1), Some(n2)) = (normalize_swedish_pnum(p1, current_year), normalize_swedish_pnum(p2, current_year)) {
        return n1 == n2;
    }
    let d1: String = p1.chars().filter(|c| c.is_ascii_digit()).collect();
    let d2: String = p2.chars().filter(|c| c.is_ascii_digit()).collect();
    if d1.is_empty() || d2.is_empty() {
        return false;
    }
    if d1 == d2 {
        return true;
    }
    if d1.len() == 12 && d2.len() == 10 {
        return d1[2..] == d2;
    }
    if d1.len() == 10 && d2.len() == 12 {
        return d1 == d2[2..];
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_personal_numbers_match_helper() {
        assert!(personal_numbers_match("19900101-1234", "199001011234"));
        assert!(personal_numbers_match("19900101-1234", "900101-1234"));
        assert!(personal_numbers_match("9001011234", "19900101-1234"));
        assert!(!personal_numbers_match("19900101-1234", "19900101-1235"));
    }
}
