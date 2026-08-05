use serde::{Deserialize, Serialize};

/// Represents an HL7 FHIR R4 MedicationRequest Open Standard payload model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FhirMedicationRequest {
    pub resource_type: String,
    pub status: String,
    pub intent: String,
    pub rxnorm_code: Option<String>,
    pub prescriber_npi: Option<String>,
    pub dea_schedule: Option<String>,
    pub interaction_warnings: Vec<String>,
    pub dispenser_instructions: Option<String>,
}

impl FhirMedicationRequest {
    pub fn new_default() -> Self {
        Self {
            resource_type: "MedicationRequest".to_string(),
            status: "active".to_string(),
            intent: "order".to_string(),
            rxnorm_code: None,
            prescriber_npi: None,
            dea_schedule: None,
            interaction_warnings: Vec::new(),
            dispenser_instructions: None,
        }
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn from_json_str(json_str: &str) -> Result<Self, String> {
        if json_str.trim().is_empty() || json_str == "{}" {
            return Ok(Self::new_default());
        }
        serde_json::from_str(json_str).map_err(|e| format!("Invalid FHIR payload: {}", e))
    }
}

/// Represents an Ed-Fi v5 StudentAcademicRecord / ReportCard Open Standard payload model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EdFiReportCard {
    pub resource_type: String,
    pub grading_period: String,
    pub weighted_gpa: Option<f64>,
    pub class_rank: Option<i32>,
    pub total_credits_earned: Option<f64>,
    pub honors_distinction: Option<String>,
    pub state_compliance_flags: Vec<String>,
}

impl EdFiReportCard {
    pub fn new_default() -> Self {
        Self {
            resource_type: "StudentAcademicRecord".to_string(),
            grading_period: "Semester 1".to_string(),
            weighted_gpa: None,
            class_rank: None,
            total_credits_earned: None,
            honors_distinction: None,
            state_compliance_flags: Vec::new(),
        }
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn from_json_str(json_str: &str) -> Result<Self, String> {
        if json_str.trim().is_empty() || json_str == "{}" {
            return Ok(Self::new_default());
        }
        serde_json::from_str(json_str).map_err(|e| format!("Invalid Ed-Fi payload: {}", e))
    }
}

pub fn validate_fhir_payload(payload_str: &str) -> bool {
    FhirMedicationRequest::from_json_str(payload_str).is_ok()
}

pub fn validate_edfi_payload(payload_str: &str) -> bool {
    EdFiReportCard::from_json_str(payload_str).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fhir_payload_serialization_and_validation() {
        let fhir = FhirMedicationRequest {
            resource_type: "MedicationRequest".to_string(),
            status: "active".to_string(),
            intent: "order".to_string(),
            rxnorm_code: Some("313782".to_string()),
            prescriber_npi: Some("1234567890".to_string()),
            dea_schedule: Some("Schedule II".to_string()),
            interaction_warnings: vec!["Do not mix with alcohol".to_string()],
            dispenser_instructions: Some("Take 1 tablet daily with water".to_string()),
        };

        let json_str = fhir.to_json_string();
        assert!(validate_fhir_payload(&json_str));

        let parsed = FhirMedicationRequest::from_json_str(&json_str).unwrap();
        assert_eq!(parsed.rxnorm_code, Some("313782".to_string()));
        assert_eq!(parsed.prescriber_npi, Some("1234567890".to_string()));
        assert_eq!(parsed.interaction_warnings.len(), 1);
    }

    #[test]
    fn test_edfi_payload_serialization_and_validation() {
        let edfi = EdFiReportCard {
            resource_type: "StudentAcademicRecord".to_string(),
            grading_period: "Spring 2026".to_string(),
            weighted_gpa: Some(3.95),
            class_rank: Some(3),
            total_credits_earned: Some(120.0),
            honors_distinction: Some("Summa Cum Laude".to_string()),
            state_compliance_flags: vec!["State FERPA Compliant".to_string()],
        };

        let json_str = edfi.to_json_string();
        assert!(validate_edfi_payload(&json_str));

        let parsed = EdFiReportCard::from_json_str(&json_str).unwrap();
        assert_eq!(parsed.weighted_gpa, Some(3.95));
        assert_eq!(parsed.class_rank, Some(3));
        assert_eq!(parsed.honors_distinction, Some("Summa Cum Laude".to_string()));
    }
}

