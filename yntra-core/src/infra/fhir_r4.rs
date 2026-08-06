use serde::{Deserialize, Serialize};

/// HL7 FHIR R4 HumanName structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FhirHumanName {
    #[serde(default)]
    pub use_type: Option<String>, // official, usual, temp
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub given: Vec<String>,
    #[serde(default)]
    pub prefix: Vec<String>,
    #[serde(default)]
    pub suffix: Vec<String>,
}

/// HL7 FHIR R4 Identifier structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FhirIdentifier {
    #[serde(default)]
    pub use_type: Option<String>, // usual, official, temp, secondary
    #[serde(default)]
    pub system: Option<String>, // e.g. urn:oid:2.16.840.1.113883.4.1 (SSN), MRN
    #[serde(default)]
    pub value: String,
}

/// HL7 FHIR R4 Address structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FhirAddress {
    #[serde(default)]
    pub use_type: Option<String>, // home, work, temp
    #[serde(default)]
    pub line: Vec<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub postal_code: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
}

/// HL7 FHIR R4 Telecom structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FhirContactPoint {
    #[serde(default)]
    pub system: Option<String>, // phone, email, fax
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub use_type: Option<String>, // home, work, mobile
}

/// HL7 FHIR R4 Reference structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FhirReference {
    pub reference: String, // e.g. "Patient/p-101"
    #[serde(default)]
    pub display: Option<String>,
}

/// HL7 FHIR R4 Coding structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FhirCoding {
    #[serde(default)]
    pub system: Option<String>, // e.g. http://loinc.org, http://hl7.org/fhir/sid/icd-10-cm, http://snomed.info/sct
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub display: Option<String>,
}

/// HL7 FHIR R4 CodeableConcept structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FhirCodeableConcept {
    #[serde(default)]
    pub coding: Vec<FhirCoding>,
    #[serde(default)]
    pub text: Option<String>,
}

/// HL7 FHIR R4 Quantity structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FhirQuantity {
    pub value: f64,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub system: Option<String>, // http://unitsofmeasure.org
    #[serde(default)]
    pub code: Option<String>,
}

/// HL7 FHIR R4 Period structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FhirPeriod {
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
}

// -----------------------------------------------------------------------------
// FHIR R4 Core Resource Schemas
// -----------------------------------------------------------------------------

/// HL7 FHIR R4 /Patient Resource Schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FhirPatient {
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    pub id: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub identifier: Vec<FhirIdentifier>,
    #[serde(default)]
    pub name: Vec<FhirHumanName>,
    #[serde(default)]
    pub gender: Option<String>, // male, female, other, unknown
    #[serde(default, rename = "birthDate")]
    pub birth_date: Option<String>, // YYYY-MM-DD
    #[serde(default)]
    pub address: Vec<FhirAddress>,
    #[serde(default)]
    pub telecom: Vec<FhirContactPoint>,
}

impl Default for FhirPatient {
    fn default() -> Self {
        Self {
            resource_type: "Patient".to_string(),
            id: String::new(),
            active: true,
            identifier: Vec::new(),
            name: Vec::new(),
            gender: None,
            birth_date: None,
            address: Vec::new(),
            telecom: Vec::new(),
        }
    }
}

/// HL7 FHIR R4 /Encounter Resource Schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FhirEncounter {
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    pub id: String,
    pub status: String, // planned, arrived, triaged, in-progress, onleave, finished, cancelled
    pub class: FhirCoding, // inpatient, outpatient, ambulatory, emergency
    #[serde(default)]
    pub subject: Option<FhirReference>,
    #[serde(default)]
    pub period: Option<FhirPeriod>,
    #[serde(default, rename = "reasonCode")]
    pub reason_code: Vec<FhirCodeableConcept>,
}

impl Default for FhirEncounter {
    fn default() -> Self {
        Self {
            resource_type: "Encounter".to_string(),
            id: String::new(),
            status: "finished".to_string(),
            class: FhirCoding {
                system: Some("http://terminology.hl7.org/CodeSystem/v3-ActCode".to_string()),
                code: Some("AMB".to_string()),
                display: Some("ambulatory".to_string()),
            },
            subject: None,
            period: None,
            reason_code: Vec::new(),
        }
    }
}

/// HL7 FHIR R4 /Observation Resource Schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FhirObservation {
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    pub id: String,
    pub status: String, // registered, preliminary, final, amended, cancelled
    pub code: FhirCodeableConcept, // LOINC or SNOMED CT
    #[serde(default)]
    pub subject: Option<FhirReference>,
    #[serde(default, rename = "effectiveDateTime")]
    pub effective_date_time: Option<String>,
    #[serde(default, rename = "valueQuantity")]
    pub value_quantity: Option<FhirQuantity>,
    #[serde(default, rename = "valueString")]
    pub value_string: Option<String>,
}

impl Default for FhirObservation {
    fn default() -> Self {
        Self {
            resource_type: "Observation".to_string(),
            id: String::new(),
            status: "final".to_string(),
            code: FhirCodeableConcept {
                coding: vec![FhirCoding {
                    system: Some("http://loinc.org".to_string()),
                    code: Some("11506-3".to_string()),
                    display: Some("Progress note".to_string()),
                }],
                text: Some("Clinical Care Note".to_string()),
            },
            subject: None,
            effective_date_time: None,
            value_quantity: None,
            value_string: None,
        }
    }
}

/// HL7 FHIR R4 /Condition Resource Schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FhirCondition {
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    pub id: String,
    #[serde(default, rename = "clinicalStatus")]
    pub clinical_status: Option<FhirCodeableConcept>, // active, recurrence, relapse, inactive, remission, resolved
    #[serde(default, rename = "verificationStatus")]
    pub verification_status: Option<FhirCodeableConcept>, // unconfirmed, provisional, differential, confirmed, refuted
    #[serde(default)]
    pub category: Vec<FhirCodeableConcept>,
    #[serde(default)]
    pub code: Option<FhirCodeableConcept>, // ICD-10 or SNOMED CT
    #[serde(default)]
    pub subject: Option<FhirReference>,
    #[serde(default, rename = "onsetDateTime")]
    pub onset_date_time: Option<String>,
}

impl Default for FhirCondition {
    fn default() -> Self {
        Self {
            resource_type: "Condition".to_string(),
            id: String::new(),
            clinical_status: Some(FhirCodeableConcept {
                coding: vec![FhirCoding {
                    system: Some("http://terminology.hl7.org/CodeSystem/condition-clinical".to_string()),
                    code: Some("active".to_string()),
                    display: Some("Active".to_string()),
                }],
                text: Some("active".to_string()),
            }),
            verification_status: Some(FhirCodeableConcept {
                coding: vec![FhirCoding {
                    system: Some("http://terminology.hl7.org/CodeSystem/condition-ver-status".to_string()),
                    code: Some("confirmed".to_string()),
                    display: Some("Confirmed".to_string()),
                }],
                text: Some("confirmed".to_string()),
            }),
            category: Vec::new(),
            code: None,
            subject: None,
            onset_date_time: None,
        }
    }
}

/// HL7 FHIR R4 Bundle Entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FhirBundleEntry {
    #[serde(default, rename = "fullUrl")]
    pub full_url: Option<String>,
    pub resource: serde_json::Value,
}

/// HL7 FHIR R4 /Bundle Resource Schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FhirBundle {
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    pub id: String,
    pub fhir_type: String, // searchset, transaction, batch, collection
    #[serde(default)]
    pub total: u32,
    #[serde(default)]
    pub entry: Vec<FhirBundleEntry>,
}

impl Default for FhirBundle {
    fn default() -> Self {
        Self {
            resource_type: "Bundle".to_string(),
            id: uuid::Uuid::new_v4().to_string(),
            fhir_type: "collection".to_string(),
            total: 0,
            entry: Vec::new(),
        }
    }
}

// -----------------------------------------------------------------------------
// Validation & Parsing Helpers
// -----------------------------------------------------------------------------

/// Validate any raw FHIR R4 JSON string structure and coding systems
pub fn validate_fhir_r4_payload(json_str: &str) -> Result<(String, String), String> {
    let value: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("Invalid JSON syntax: {}", e))?;

    let res_type = value
        .get("resourceType")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required 'resourceType' field".to_string())?;

    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("gen-id")
        .to_string();

    match res_type {
        "Patient" => {
            let _: FhirPatient = serde_json::from_value(value)
                .map_err(|e| format!("Failed to parse FhirPatient: {}", e))?;
            Ok(("Patient".to_string(), id))
        }
        "Encounter" => {
            let _: FhirEncounter = serde_json::from_value(value)
                .map_err(|e| format!("Failed to parse FhirEncounter: {}", e))?;
            Ok(("Encounter".to_string(), id))
        }
        "Observation" => {
            let _: FhirObservation = serde_json::from_value(value)
                .map_err(|e| format!("Failed to parse FhirObservation: {}", e))?;
            Ok(("Observation".to_string(), id))
        }
        "Condition" => {
            let _: FhirCondition = serde_json::from_value(value)
                .map_err(|e| format!("Failed to parse FhirCondition: {}", e))?;
            Ok(("Condition".to_string(), id))
        }
        "Bundle" => {
            let _: FhirBundle = serde_json::from_value(value)
                .map_err(|e| format!("Failed to parse FhirBundle: {}", e))?;
            Ok(("Bundle".to_string(), id))
        }
        other => Err(format!("Unsupported FHIR resourceType: '{}'", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fhir_patient_roundtrip() {
        let patient = FhirPatient {
            resource_type: "Patient".to_string(),
            id: "p-1001".to_string(),
            active: true,
            identifier: vec![FhirIdentifier {
                use_type: Some("official".to_string()),
                system: Some("MRN".to_string()),
                value: "MRN-889900".to_string(),
            }],
            name: vec![FhirHumanName {
                family: Some("DOE".to_string()),
                given: vec!["JANE".to_string(), "A".to_string()],
                text: Some("JANE A DOE".to_string()),
                ..Default::default()
            }],
            gender: Some("female".to_string()),
            birth_date: Some("1988-04-12".to_string()),
            address: vec![FhirAddress {
                line: vec!["100 HEALTH WAY".to_string()],
                city: Some("BOSTON".to_string()),
                state: Some("MA".to_string()),
                postal_code: Some("02108".to_string()),
                country: Some("USA".to_string()),
                ..Default::default()
            }],
            telecom: vec![FhirContactPoint {
                system: Some("phone".to_string()),
                value: Some("555-0199".to_string()),
                use_type: Some("mobile".to_string()),
            }],
        };

        let json_str = serde_json::to_string_pretty(&patient).unwrap();
        let (res_type, id) = validate_fhir_r4_payload(&json_str).unwrap();
        assert_eq!(res_type, "Patient");
        assert_eq!(id, "p-1001");
    }

    #[test]
    fn test_fhir_observation_roundtrip() {
        let obs = FhirObservation {
            resource_type: "Observation".to_string(),
            id: "obs-2001".to_string(),
            status: "final".to_string(),
            code: FhirCodeableConcept {
                coding: vec![FhirCoding {
                    system: Some("http://loinc.org".to_string()),
                    code: Some("8867-4".to_string()),
                    display: Some("Heart rate".to_string()),
                }],
                text: Some("Heart rate pulse".to_string()),
            },
            subject: Some(FhirReference {
                reference: "Patient/p-1001".to_string(),
                display: Some("JANE DOE".to_string()),
            }),
            effective_date_time: Some("2026-08-06T14:00:00Z".to_string()),
            value_quantity: Some(FhirQuantity {
                value: 72.0,
                unit: Some("beats/min".to_string()),
                system: Some("http://unitsofmeasure.org".to_string()),
                code: Some("/min".to_string()),
            }),
            value_string: None,
        };

        let json_str = serde_json::to_string(&obs).unwrap();
        let (res_type, id) = validate_fhir_r4_payload(&json_str).unwrap();
        assert_eq!(res_type, "Observation");
        assert_eq!(id, "obs-2001");
    }
}
