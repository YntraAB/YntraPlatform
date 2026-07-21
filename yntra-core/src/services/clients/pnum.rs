pub fn verify_luhn_checksum(pnum_10_digits: &str) -> bool {
    if pnum_10_digits.len() != 10 {
        return false;
    }
    let mut sum = 0;
    for (i, ch) in pnum_10_digits.chars().enumerate() {
        let mut digit = match ch.to_digit(10) {
            Some(d) => d as i32,
            None => return false,
        };
        if i % 2 == 0 {
            digit *= 2;
            if digit > 9 {
                digit -= 9;
            }
        }
        sum += digit;
    }
    sum % 10 == 0
}

pub fn normalize_swedish_pnum(pnum: &str, current_year: i32) -> Option<String> {
    let clean = pnum.trim();
    let digits_only: String = clean.chars().filter(|c| c.is_ascii_digit()).collect();

    let pnum_10 = if digits_only.len() == 12 {
        &digits_only[2..12]
    } else if digits_only.len() == 10 {
        &digits_only[..10]
    } else {
        return None;
    };

    if !verify_luhn_checksum(pnum_10) {
        return None;
    }

    if digits_only.len() == 12 {
        return Some(digits_only);
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
    use chrono::Datelike;
    let current_year = chrono::Utc::now().year();
    if let (Some(n1), Some(n2)) = (
        normalize_swedish_pnum(p1, current_year),
        normalize_swedish_pnum(p2, current_year),
    ) {
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
        assert!(personal_numbers_match("19811218-9876", "198112189876"));
        assert!(personal_numbers_match("19811218-9876", "811218-9876"));
        assert!(personal_numbers_match("8112189876", "19811218-9876"));
        assert!(!personal_numbers_match("19811218-9876", "19811218-9877"));
    }
}
