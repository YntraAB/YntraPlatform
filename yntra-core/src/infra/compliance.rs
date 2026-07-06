pub struct LaborRule {
    pub country_code: &'static str,
    pub law_name: &'static str,
    pub standard_daily_limit: f64,
    pub max_daily_limit_with_overtime: f64,
    pub standard_weekly_limit: f64,
    pub max_weekly_limit_with_exemption: f64,
    pub mandatory_daily_rest_hours: f64,
}

pub struct ComplianceRegistry;

impl ComplianceRegistry {
    pub fn get_rule(country_code: &str) -> LaborRule {
        match country_code {
            "NO" => LaborRule {
                country_code: "NO",
                law_name: "Norwegian Arbeidsmiljøloven § 10-4",
                standard_daily_limit: 9.0,
                max_daily_limit_with_overtime: 13.0,
                standard_weekly_limit: 40.0,
                max_weekly_limit_with_exemption: 48.0,
                mandatory_daily_rest_hours: 11.0,
            },
            "SE" => LaborRule {
                country_code: "SE",
                law_name: "Swedish Arbetstidslagen",
                standard_daily_limit: 8.0,
                max_daily_limit_with_overtime: 13.0,
                standard_weekly_limit: 40.0,
                max_weekly_limit_with_exemption: 48.0,
                mandatory_daily_rest_hours: 11.0,
            },
            "FI" => LaborRule {
                country_code: "FI",
                law_name: "Finnish Työaikalaki",
                standard_daily_limit: 8.0,
                max_daily_limit_with_overtime: 13.0,
                standard_weekly_limit: 40.0,
                max_weekly_limit_with_exemption: 48.0,
                mandatory_daily_rest_hours: 11.0,
            },
            "DK" => LaborRule {
                country_code: "DK",
                law_name: "Danish Lov om arbejdstid",
                standard_daily_limit: 8.0,
                max_daily_limit_with_overtime: 13.0,
                standard_weekly_limit: 48.0,
                max_weekly_limit_with_exemption: 48.0,
                mandatory_daily_rest_hours: 11.0,
            },
            "US-FED" => LaborRule {
                country_code: "US-FED",
                law_name: "US Federal FLSA (Fair Labor Standards Act)",
                standard_daily_limit: 24.0,
                max_daily_limit_with_overtime: 24.0,
                standard_weekly_limit: 40.0,
                max_weekly_limit_with_exemption: 168.0,
                mandatory_daily_rest_hours: 0.0,
            },
            "US-CA" => LaborRule {
                country_code: "US-CA",
                law_name: "California Labor Code § 510",
                standard_daily_limit: 8.0,
                max_daily_limit_with_overtime: 24.0,
                standard_weekly_limit: 40.0,
                max_weekly_limit_with_exemption: 168.0,
                mandatory_daily_rest_hours: 0.0,
            },
            "US-CO" => LaborRule {
                country_code: "US-CO",
                law_name: "Colorado COMPS Order #38",
                standard_daily_limit: 12.0,
                max_daily_limit_with_overtime: 24.0,
                standard_weekly_limit: 40.0,
                max_weekly_limit_with_exemption: 168.0,
                mandatory_daily_rest_hours: 0.0,
            },
            "US-NV" => LaborRule {
                country_code: "US-NV",
                law_name: "Nevada NRS 608.018",
                standard_daily_limit: 8.0,
                max_daily_limit_with_overtime: 24.0,
                standard_weekly_limit: 40.0,
                max_weekly_limit_with_exemption: 168.0,
                mandatory_daily_rest_hours: 0.0,
            },
            "US-AK" => LaborRule {
                country_code: "US-AK",
                law_name: "Alaska AS 23.10.060",
                standard_daily_limit: 8.0,
                max_daily_limit_with_overtime: 24.0,
                standard_weekly_limit: 40.0,
                max_weekly_limit_with_exemption: 168.0,
                mandatory_daily_rest_hours: 0.0,
            },
            _ => LaborRule {
                country_code: "EU",
                law_name: "EU Working Time Directive Baseline",
                standard_daily_limit: 13.0,
                max_daily_limit_with_overtime: 13.0,
                standard_weekly_limit: 48.0,
                max_weekly_limit_with_exemption: 48.0,
                mandatory_daily_rest_hours: 11.0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_rules_retrieval() {
        // test Sweden
        let se = ComplianceRegistry::get_rule("SE");
        assert_eq!(se.country_code, "SE");
        assert_eq!(se.standard_daily_limit, 8.0);
        assert_eq!(se.mandatory_daily_rest_hours, 11.0);

        // test Norway
        let no = ComplianceRegistry::get_rule("NO");
        assert_eq!(no.country_code, "NO");
        assert_eq!(no.standard_daily_limit, 9.0);

        // test Finland
        let fi = ComplianceRegistry::get_rule("FI");
        assert_eq!(fi.country_code, "FI");
        assert_eq!(fi.standard_daily_limit, 8.0);

        // test US California
        let ca = ComplianceRegistry::get_rule("US-CA");
        assert_eq!(ca.country_code, "US-CA");
        assert_eq!(ca.standard_daily_limit, 8.0);

        // test fallback to EU Working Time Directive
        let fallback = ComplianceRegistry::get_rule("INVALID-CODE");
        assert_eq!(fallback.country_code, "EU");
        assert_eq!(fallback.standard_weekly_limit, 48.0);
    }
}
