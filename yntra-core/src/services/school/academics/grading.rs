use crate::YntraError;

pub fn validate_grade_for_region(grade: &str, region: &str) -> Result<String, YntraError> {
    let mut clean = grade.trim().to_string();
    if clean.is_empty() {
        return Ok(clean);
    }
    match region {
        "SE" => {
            let valid_grades = ["A", "B", "C", "D", "E", "F", "a", "b", "c", "d", "e", "f", "-", "G", "VG", "MVG", "U", "g", "vg", "mvg", "u"];
            if !valid_grades.contains(&clean.as_str()) {
                return Err(YntraError::ValidationError(format!("Invalid grade '{}' for Swedish grading system (expected A-F, G, VG, MVG, U, or -).", clean)));
            }
        }
        "NO" => {
            let mut base = clean.clone();
            if base.ends_with('+') || base.ends_with('-') {
                base.pop();
            } else if base.contains('/') {
                let parts: Vec<&str> = base.split('/').collect();
                if parts.len() == 2 {
                    let valid_grades = ["1", "2", "3", "4", "5", "6"];
                    if valid_grades.contains(&parts[0]) && valid_grades.contains(&parts[1]) {
                        return Ok(clean);
                    }
                }
            }
            let valid_grades = ["1", "2", "3", "4", "5", "6", "B", "b", "IB", "ib", "G", "g", "U", "u"];
            if !valid_grades.contains(&base.as_str()) {
                return Err(YntraError::ValidationError(format!("Invalid grade '{}' for Norwegian grading system (expected 1-6 or B/IB with optional modifiers).", clean)));
            }
        }
        "DK" => {
            if clean == "2" {
                clean = "02".to_string();
            } else if clean == "0" {
                clean = "00".to_string();
            }
            let valid_grades = ["-3", "00", "02", "4", "7", "10", "12"];
            if !valid_grades.contains(&clean.as_str()) {
                return Err(YntraError::ValidationError(format!("Invalid grade '{}' for Danish grading system (expected -3, 00, 02, 4, 7, 10, or 12).", clean)));
            }
        }
        "FI" => {
            let mut base = clean.clone();
            if base.ends_with('+') || base.ends_with('-') || base.ends_with('½') {
                base.pop();
            } else if base.ends_with(".5") {
                base = base[..base.len()-2].to_string();
            }
            let valid_grades = [
                "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10",
                "S", "H", "s", "h", "HYV", "HYL", "hyv", "hyl"
            ];
            if !valid_grades.contains(&base.as_str()) {
                return Err(YntraError::ValidationError(format!("Invalid grade '{}' for Finnish grading system (expected 4-10, 0-5, S, H, HYV, or HYL with optional modifier).", clean)));
            }
        }
        r if r.starts_with("US") => {
            let upper_clean = clean.to_uppercase();
            let non_punitive = ["I", "INC", "P", "NP", "W", "AU"];
            if non_punitive.contains(&upper_clean.as_str()) {
                return Ok(clean);
            }
            
            if let Ok(pct) = upper_clean.parse::<f64>() {
                if pct >= 0.0 && pct <= 100.0 {
                    return Ok(clean);
                }
            }
            
            let first_char = upper_clean.chars().next().unwrap_or(' ');
            let valid_letters = ['A', 'B', 'C', 'D', 'E', 'F'];
            if !valid_letters.contains(&first_char) {
                return Err(YntraError::ValidationError(format!("Invalid grade '{}' for US grading system (expected A-F, percentage, or non-punitive codes like I, P, W).", clean)));
            }
            if upper_clean.len() > 2 {
                return Err(YntraError::ValidationError(format!("Invalid grade '{}' for US grading system (too long).", clean)));
            }
            if upper_clean.len() == 2 {
                let second_char = upper_clean.chars().nth(1).unwrap_or(' ');
                if second_char != '+' && second_char != '-' {
                    return Err(YntraError::ValidationError(format!("Invalid modifier in US grade '{}' (expected + or -).", clean)));
                }
            }
        }
        "EU" => {
            let upper_clean = clean.to_uppercase();
            let valid_grades = ["A", "B", "C", "D", "E", "FX", "F"];
            if !valid_grades.contains(&upper_clean.as_str()) {
                return Err(YntraError::ValidationError(format!("Invalid grade '{}' for ECTS grading system.", clean)));
            }
        }
        _ => {}
    }
    Ok(clean)
}

pub fn calculate_gpa(grades: &[Option<String>], region: &str) -> f64 {
    let mut total_points = 0.0;
    let mut count = 0;
    let upper_region = region.to_uppercase();
    
    let mut use_fi_university = false;
    if upper_region == "FI" {
        let mut has_uni_indicators = false;
        let mut has_comp_indicators = false;
        for g_opt in grades {
            if let Some(g) = g_opt {
                let clean = g.trim().to_uppercase();
                if ["0", "1", "2", "3"].contains(&clean.as_str()) {
                    has_uni_indicators = true;
                }
                if ["6", "7", "8", "9", "10"].contains(&clean.as_str()) {
                    has_comp_indicators = true;
                }
            }
        }
        if has_uni_indicators && !has_comp_indicators {
            use_fi_university = true;
        }
    }
    
    for g_opt in grades {
        if let Some(g) = g_opt {
            let clean = g.trim().to_uppercase();
            if clean.is_empty() {
                continue;
            }
            let pts = match upper_region.as_str() {
                "SE" => match clean.as_str() {
                    "A" => Some(4.0),
                    "B" => Some(3.0),
                    "C" => Some(2.0),
                    "D" => Some(1.5),
                    "E" => Some(1.0),
                    "F" | "U" | "IG" => Some(0.0),
                    "G" => Some(2.0),
                    "VG" => Some(3.5),
                    "MVG" => Some(4.0),
                    _ => None,
                },
                "NO" => match clean.as_str() {
                    "6" => Some(4.0),
                    "5" => Some(3.0),
                    "4" => Some(2.0),
                    "3" => Some(1.5),
                    "2" => Some(1.0),
                    "1" | "U" | "IB" => Some(0.0),
                    "G" | "B" => Some(3.0),
                    _ => None,
                },
                "DK" => match clean.as_str() {
                    "12" => Some(4.0),
                    "10" => Some(3.5),
                    "7" => Some(3.0),
                    "4" => Some(2.0),
                    "02" => Some(1.0),
                    "00" | "0" | "-3" => Some(0.0),
                    _ => None,
                },
                "FI" => {
                    if use_fi_university {
                        match clean.as_str() {
                            "5" => Some(4.0),
                            "4" => Some(3.5),
                            "3" => Some(3.0),
                            "2" => Some(2.0),
                            "1" => Some(1.0),
                            "0" | "H" | "HYL" | "I" => Some(0.0),
                            "S" | "HYV" => Some(3.0),
                            "L" => Some(4.0),
                            "E" => Some(3.5),
                            "M" => Some(3.0),
                            "C" => Some(2.5),
                            "B" => Some(2.0),
                            "A" => Some(1.0),
                            _ => None,
                        }
                    } else {
                        match clean.as_str() {
                            "10" => Some(4.0),
                            "9" => Some(3.5),
                            "8" => Some(3.0),
                            "7" => Some(2.0),
                            "6" => Some(1.5),
                            "5" => Some(1.0),
                            "4" | "H" | "HYL" | "I" => Some(0.0),
                            "S" | "HYV" => Some(3.0),
                            "L" => Some(4.0),
                            "E" => Some(3.5),
                            "M" => Some(3.0),
                            "C" => Some(2.5),
                            "B" => Some(2.0),
                            "A" => Some(1.0),
                            _ => None,
                        }
                    }
                }
                r if r.starts_with("US") => {
                    match clean.as_str() {
                        "A" | "A+" => Some(4.0),
                        "A-" => Some(3.7),
                        "B+" => Some(3.3),
                        "B" => Some(3.0),
                        "B-" => Some(2.7),
                        "C+" => Some(2.3),
                        "C" => Some(2.0),
                        "C-" => Some(1.7),
                        "D+" => Some(1.3),
                        "D" | "E" | "E+" => Some(1.0),
                        "D-" | "E-" => Some(0.7),
                        "F" => Some(0.0),
                        _ => {
                            let mut clean_num = clean.clone();
                            while clean_num.ends_with('%') || clean_num.ends_with('+') || clean_num.ends_with('-') {
                                clean_num.pop();
                            }
                            if let Ok(pct) = clean_num.parse::<f64>() {
                                if pct >= 90.0 { Some(4.0) }
                                else if pct >= 80.0 { Some(3.0) }
                                else if pct >= 70.0 { Some(2.0) }
                                else if pct >= 60.0 { Some(1.0) }
                                else { Some(0.0) }
                            } else {
                                None
                            }
                        }
                    }
                },
                _ => {
                    match clean.as_str() {
                        "A" => Some(4.0),
                        "B" => Some(3.0),
                        "C" => Some(2.0),
                        "D" => Some(1.0),
                        "E" => Some(1.0),
                        "FX" | "F" => Some(0.0),
                        _ => None,
                    }
                }
            };
            if let Some(p) = pts {
                total_points += p;
                count += 1;
            }
        }
    }
    if count > 0 { total_points / (count as f64) } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_gpa_all_cases() {
        assert_eq!(calculate_gpa(&[Some("A".to_string())], "US"), 4.0);
        assert_eq!(calculate_gpa(&[Some("B".to_string())], "US"), 3.0);
        assert_eq!(calculate_gpa(&[Some("C".to_string())], "US"), 2.0);
        assert_eq!(calculate_gpa(&[Some("D".to_string())], "US"), 1.0);
        assert_eq!(calculate_gpa(&[Some("E".to_string())], "US"), 1.0);
        assert_eq!(calculate_gpa(&[Some("F".to_string())], "US"), 0.0);

        assert_eq!(calculate_gpa(&[Some("6".to_string()), Some("5".to_string())], "NO"), 3.5);
        assert_eq!(calculate_gpa(&[Some("2".to_string()), Some("1".to_string())], "NO"), 0.5);

        assert_eq!(calculate_gpa(&[Some("12".to_string()), Some("4".to_string())], "DK"), 3.0);
        assert_eq!(calculate_gpa(&[Some("02".to_string()), Some("-3".to_string())], "DK"), 0.5);

        assert_eq!(calculate_gpa(&[Some("10".to_string()), Some("8".to_string())], "FI"), 3.5);
        assert_eq!(calculate_gpa(&[Some("3".to_string()), Some("1".to_string())], "FI"), 2.0);
        assert_eq!(calculate_gpa(&[Some("5".to_string()), Some("4".to_string())], "FI"), 0.5);
        assert_eq!(calculate_gpa(&[Some("10".to_string()), Some("5".to_string()), Some("4".to_string())], "FI"), 5.0 / 3.0);

        assert_eq!(calculate_gpa(&[Some("MVG".to_string()), Some("IG".to_string())], "SE"), 2.0);

        assert_eq!(calculate_gpa(&[Some("95".to_string()), Some("85".to_string())], "US-CA"), 3.5);

        assert_eq!(calculate_gpa(&[Some("A".to_string()), Some("B".to_string())], "US"), 3.5);
        assert_eq!(calculate_gpa(&[Some("A".to_string()), Some("F".to_string())], "US"), 2.0);

        assert_eq!(calculate_gpa(&[Some("  a  ".to_string()), Some("b\n".to_string())], "US"), 3.5);

        assert_eq!(
            calculate_gpa(&[
                Some("A".to_string()),
                None,
                Some("INVALID".to_string()),
                Some("B".to_string())
            ], "US"),
            3.5
        );

        assert_eq!(calculate_gpa(&[], "US"), 0.0);
        assert_eq!(calculate_gpa(&[None, Some("X".to_string())], "US"), 0.0);
    }

    #[test]
    fn test_validate_grade_for_region_all_cases() {
        assert!(validate_grade_for_region("A", "SE").is_ok());
        assert!(validate_grade_for_region("F", "SE").is_ok());
        assert!(validate_grade_for_region("a", "SE").is_ok());
        assert!(validate_grade_for_region("-", "SE").is_ok());
        assert!(validate_grade_for_region("G", "SE").is_ok());
        assert!(validate_grade_for_region("VG", "SE").is_ok());
        assert!(validate_grade_for_region("X", "SE").is_err());

        assert!(validate_grade_for_region("6", "NO").is_ok());
        assert!(validate_grade_for_region("1", "NO").is_ok());
        assert!(validate_grade_for_region("B", "NO").is_ok());
        assert!(validate_grade_for_region("X", "NO").is_err());

        assert!(validate_grade_for_region("12", "DK").is_ok());
        assert!(validate_grade_for_region("-3", "DK").is_ok());
        assert!(validate_grade_for_region("02", "DK").is_ok());
        assert!(validate_grade_for_region("2", "DK").is_ok());
        assert!(validate_grade_for_region("A", "DK").is_err());

        assert!(validate_grade_for_region("10", "FI").is_ok());
        assert!(validate_grade_for_region("4", "FI").is_ok());
        assert!(validate_grade_for_region("5", "FI").is_ok());
        assert!(validate_grade_for_region("S", "FI").is_ok());
        assert!(validate_grade_for_region("HYV", "FI").is_ok());
        assert!(validate_grade_for_region("A", "FI").is_err());

        assert!(validate_grade_for_region("A+", "US-CA").is_ok());
        assert!(validate_grade_for_region("B-", "US-FED").is_ok());
        assert!(validate_grade_for_region("F", "US-NY").is_ok());
        assert!(validate_grade_for_region("E", "US-NY").is_ok());
        assert!(validate_grade_for_region("I", "US-CA").is_ok());
        assert!(validate_grade_for_region("INC", "US-CA").is_ok());
        assert!(validate_grade_for_region("W", "US-CA").is_ok());
        assert!(validate_grade_for_region("A++", "US-CA").is_err());
        assert!(validate_grade_for_region("X", "US-CA").is_err());

        assert!(validate_grade_for_region("A", "EU").is_ok());
        assert!(validate_grade_for_region("FX", "EU").is_ok());
        assert!(validate_grade_for_region("X", "EU").is_err());
    }
}
