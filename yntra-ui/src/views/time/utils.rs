pub fn format_month_year(date_str: &str) -> String {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() >= 2 {
        let year = parts[0];
        let month = match parts[1] {
            "01" => "Januari",
            "02" => "Februari",
            "03" => "Mars",
            "04" => "April",
            "05" => "Maj",
            "06" => "Juni",
            "07" => "Juli",
            "08" => "Augusti",
            "09" => "September",
            "10" => "Oktober",
            "11" => "November",
            "12" => "December",
            _ => "Okänd",
        };
        format!("{} {}", month, year)
    } else {
        date_str.to_string()
    }
}
