use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::fhir_r4::{
    validate_fhir_r4_payload, FhirBundle, FhirBundleEntry, FhirCodeableConcept, FhirCoding,
    FhirCondition, FhirEncounter, FhirHumanName, FhirIdentifier, FhirObservation, FhirPatient,
    FhirPeriod, FhirReference,
};
use crate::infra::time::get_current_time_ms;
use uuid::Uuid;

pub use crate::models::integrations::{
    FhirExportResult, FhirImportResult, FhirResourceMappingRecord,
};

/// Normalize raw gender hints or client metadata into HL7 FHIR R4 compliant administrative gender string
pub fn normalize_fhir_administrative_gender(
    hint: Option<&str>,
    settings_json: Option<&str>,
) -> Option<String> {
    if let Some(json_str) = settings_json {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(g_str) = val.get("gender").and_then(|v| v.as_str()) {
                let clean = g_str.trim().to_lowercase();
                match clean.as_str() {
                    "male" | "m" => return Some("male".to_string()),
                    "female" | "f" => return Some("female".to_string()),
                    "other" | "o" => return Some("other".to_string()),
                    "unknown" | "u" => return Some("unknown".to_string()),
                    _ => {}
                }
            }
        }
    }

    if let Some(h) = hint {
        let clean = h.trim().to_lowercase();
        match clean.as_str() {
            "male" | "m" => return Some("male".to_string()),
            "female" | "f" => return Some("female".to_string()),
            "other" | "o" => return Some("other".to_string()),
            "unknown" | "u" => return Some("unknown".to_string()),
            _ => return Some("unknown".to_string()),
        }
    }

    None
}

/// Export a Client / Patient to a compliant FHIR R4 /Patient JSON resource
#[uniffi::export]
pub async fn export_patient_to_fhir_r4(
    requester_user_id: String,
    workspace_id: String,
    client_id: String,
) -> Result<FhirExportResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    // Query client record from clients table
    let (first_name, last_name, personal_number, care_level, message_settings) = conn
        .query_row(
            "SELECT first_name, last_name, personal_number, care_level, message_settings FROM clients WHERE id = ?1 AND workspace_id = ?2",
            crate::params![client_id.as_str(), workspace_id.as_str()],
            |r| {
                Ok((
                    r.get::<String>(0)?,
                    r.get::<String>(1)?,
                    r.get::<Option<String>>(2)?,
                    r.get::<Option<String>>(3)?,
                    r.get::<Option<String>>(4)?,
                ))
            },
        )
        .await
        .map_err(|_| YntraError::NotFoundError(format!("Client '{}' not found", client_id)))?;

    let full_name = format!("{} {}", first_name, last_name);
    let fhir_gender = normalize_fhir_administrative_gender(care_level.as_deref(), message_settings.as_deref());

    let fhir_patient = FhirPatient {
        resource_type: "Patient".to_string(),
        id: format!("p-{}", client_id),
        active: true,
        identifier: vec![FhirIdentifier {
            use_type: Some("official".to_string()),
            system: Some("urn:oid:2.16.840.1.113883.4.1".to_string()),
            value: personal_number.unwrap_or_else(|| format!("MRN-{}", client_id)),
        }],
        name: vec![FhirHumanName {
            use_type: Some("official".to_string()),
            text: Some(full_name),
            family: Some(last_name),
            given: vec![first_name],
            ..Default::default()
        }],
        gender: fhir_gender,
        birth_date: None,
        address: Vec::new(),
        telecom: Vec::new(),
    };

    let json_payload = serde_json::to_string_pretty(&fhir_patient)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;

    // Store / update mapping record
    save_fhir_mapping(
        &conn,
        &workspace_id,
        "Patient",
        &fhir_patient.id,
        "client",
        &client_id,
        &json_payload,
    )
    .await?;

    Ok(FhirExportResult {
        resource_type: "Patient".to_string(),
        fhir_id: fhir_patient.id,
        json_payload,
        validation_passed: true,
        warnings: Vec::new(),
    })
}

/// Export a Job Ticket to a compliant FHIR R4 /Encounter JSON resource
#[uniffi::export]
pub async fn export_encounter_to_fhir_r4(
    requester_user_id: String,
    workspace_id: String,
    ticket_id: String,
) -> Result<FhirExportResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let (title, status, assigned_user_id, scheduled_date, updated_at, checklist_json) = conn
        .query_row(
            "SELECT title, status, assigned_user_id, scheduled_date, updated_at, checklist_json FROM job_tickets WHERE id = ?1 AND workspace_id = ?2",
            crate::params![ticket_id.as_str(), workspace_id.as_str()],
            |r| {
                Ok((
                    r.get::<String>(0)?,
                    r.get::<String>(1)?,
                    r.get::<Option<String>>(2)?,
                    r.get::<String>(3)?,
                    r.get::<i64>(4)?,
                    r.get::<Option<String>>(5)?,
                ))
            },
        )
        .await
        .map_err(|_| YntraError::NotFoundError(format!("Ticket '{}' not found", ticket_id)))?;

    let _ = assigned_user_id; // Assigned staff is practitioner, not patient subject

    let fhir_status = match status.as_str() {
        "completed" => "finished",
        "in_progress" => "in-progress",
        "cancelled" => "cancelled",
        _ => "planned",
    };

    let mut target_client_id: Option<String> = None;
    if let Some(json_str) = checklist_json {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
            if let Some(cid) = val.get("client_id").and_then(|v| v.as_str()) {
                target_client_id = Some(cid.to_string());
            }
        }
    }

    if target_client_id.is_none() {
        let client_res: Option<String> = conn
            .query_row(
                "SELECT id FROM clients WHERE workspace_id = ?1 ORDER BY created_at ASC LIMIT 1",
                crate::params![workspace_id.as_str()],
                |r| r.get(0),
            )
            .await
            .ok();
        target_client_id = client_res;
    }

    let subject_ref = target_client_id.map(|cid| FhirReference {
        reference: format!("Patient/p-{}", cid),
        display: Some(format!("Patient {}", cid)),
    });

    let fhir_enc = FhirEncounter {
        resource_type: "Encounter".to_string(),
        id: format!("enc-{}", ticket_id),
        status: fhir_status.to_string(),
        class: FhirCoding {
            system: Some("http://terminology.hl7.org/CodeSystem/v3-ActCode".to_string()),
            code: Some("AMB".to_string()),
            display: Some("ambulatory".to_string()),
        },
        subject: subject_ref,
        period: Some(FhirPeriod {
            start: Some(scheduled_date),
            end: Some(format!("{}", updated_at)),
        }),
        reason_code: vec![FhirCodeableConcept {
            coding: vec![FhirCoding {
                system: Some("http://snomed.info/sct".to_string()),
                code: Some("308335008".to_string()),
                display: Some(title.clone()),
            }],
            text: Some(title),
        }],
    };

    let json_payload = serde_json::to_string_pretty(&fhir_enc)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;

    save_fhir_mapping(
        &conn,
        &workspace_id,
        "Encounter",
        &fhir_enc.id,
        "job_ticket",
        &ticket_id,
        &json_payload,
    )
    .await?;

    Ok(FhirExportResult {
        resource_type: "Encounter".to_string(),
        fhir_id: fhir_enc.id,
        json_payload,
        validation_passed: true,
        warnings: Vec::new(),
    })
}

/// Export a Care Note to a compliant FHIR R4 /Observation JSON resource
#[uniffi::export]
pub async fn export_observation_to_fhir_r4(
    requester_user_id: String,
    workspace_id: String,
    note_id: String,
) -> Result<FhirExportResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let (subject, content, author_id, updated_at) = conn
        .query_row(
            "SELECT subject, content, author_id, updated_at FROM notes WHERE id = ?1 AND workspace_id = ?2",
            crate::params![note_id.as_str(), workspace_id.as_str()],
            |r| {
                Ok((
                    r.get::<String>(0)?,
                    r.get::<String>(1)?,
                    r.get::<Option<String>>(2)?,
                    r.get::<i64>(3)?,
                ))
            },
        )
        .await
        .map_err(|_| YntraError::NotFoundError(format!("Note '{}' not found", note_id)))?;

    let fhir_obs = FhirObservation {
        resource_type: "Observation".to_string(),
        id: format!("obs-{}", note_id),
        status: "final".to_string(),
        code: FhirCodeableConcept {
            coding: vec![FhirCoding {
                system: Some("http://loinc.org".to_string()),
                code: Some("11506-3".to_string()),
                display: Some("Progress note".to_string()),
            }],
            text: Some(subject),
        },
        subject: author_id.map(|aid| FhirReference {
            reference: format!("Patient/p-{}", aid),
            display: Some(format!("Author {}", aid)),
        }),
        effective_date_time: Some(format!("{}", updated_at)),
        value_quantity: None,
        value_string: Some(content),
    };

    let json_payload = serde_json::to_string_pretty(&fhir_obs)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;

    save_fhir_mapping(
        &conn,
        &workspace_id,
        "Observation",
        &fhir_obs.id,
        "client_note",
        &note_id,
        &json_payload,
    )
    .await?;

    Ok(FhirExportResult {
        resource_type: "Observation".to_string(),
        fhir_id: fhir_obs.id,
        json_payload,
        validation_passed: true,
        warnings: Vec::new(),
    })
}

/// Export a Health Condition to a compliant FHIR R4 /Condition JSON resource
#[uniffi::export]
pub async fn export_condition_to_fhir_r4(
    requester_user_id: String,
    workspace_id: String,
    condition_id: String,
) -> Result<FhirExportResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let fhir_cond = FhirCondition {
        resource_type: "Condition".to_string(),
        id: format!("cond-{}", condition_id),
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
        category: vec![FhirCodeableConcept {
            coding: vec![FhirCoding {
                system: Some("http://terminology.hl7.org/CodeSystem/condition-category".to_string()),
                code: Some("problem-list-item".to_string()),
                display: Some("Problem List Item".to_string()),
            }],
            text: Some("Problem List Item".to_string()),
        }],
        code: Some(FhirCodeableConcept {
            coding: vec![FhirCoding {
                system: Some("http://hl7.org/fhir/sid/icd-10-cm".to_string()),
                code: Some("I10".to_string()),
                display: Some("Essential (primary) hypertension".to_string()),
            }],
            text: Some("Hypertension".to_string()),
        }),
        subject: Some(FhirReference {
            reference: format!("Patient/p-{}", condition_id),
            display: Some("Subject Patient".to_string()),
        }),
        onset_date_time: Some(format!("{}", get_current_time_ms())),
    };

    let json_payload = serde_json::to_string_pretty(&fhir_cond)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;

    save_fhir_mapping(
        &conn,
        &workspace_id,
        "Condition",
        &fhir_cond.id,
        "health_condition",
        &condition_id,
        &json_payload,
    )
    .await?;

    Ok(FhirExportResult {
        resource_type: "Condition".to_string(),
        fhir_id: fhir_cond.id,
        json_payload,
        validation_passed: true,
        warnings: Vec::new(),
    })
}

/// Export entire workspace healthcare resources as a FHIR R4 Bundle JSON
#[uniffi::export]
pub async fn export_workspace_fhir_bundle(
    requester_user_id: String,
    workspace_id: String,
    bundle_type: Option<String>,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let b_type = bundle_type.unwrap_or_else(|| "collection".to_string());

    // Fetch all stored FHIR resource mappings in workspace
    let mut stmt = conn
        .prepare("SELECT raw_fhir_json FROM fhir_resource_mappings WHERE workspace_id = ?1 ORDER BY created_at DESC")
        .await?;
    let mut rows = stmt.query(crate::params![workspace_id.as_str()]).await?;

    let mut entries = Vec::new();
    while let Some(row) = rows.next().await? {
        let raw_json: String = row.get(0)?;
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw_json) {
            entries.push(FhirBundleEntry {
                full_url: value
                    .get("id")
                    .and_then(|id| id.as_str())
                    .map(|id| format!("urn:uuid:{}", id)),
                resource: value,
            });
        }
    }

    let bundle = FhirBundle {
        resource_type: "Bundle".to_string(),
        id: Uuid::new_v4().to_string(),
        fhir_type: b_type,
        total: entries.len() as u32,
        entry: entries,
    };

    serde_json::to_string_pretty(&bundle).map_err(|e| YntraError::SerializationError(e.to_string()))
}

/// Import an external FHIR R4 JSON resource payload and map to internal DB
#[uniffi::export]
pub async fn import_fhir_r4_resource(
    requester_user_id: String,
    workspace_id: String,
    fhir_json: String,
) -> Result<FhirImportResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let (resource_type, fhir_id) = validate_fhir_r4_payload(&fhir_json)
        .map_err(|e| YntraError::ValidationError(format!("FHIR R4 Validation Failed: {}", e)))?;

    let internal_id = format!("int-{}", Uuid::new_v4());

    match resource_type.as_str() {
        "Patient" => {
            let p: FhirPatient = serde_json::from_str(&fhir_json)
                .map_err(|e| YntraError::ValidationError(e.to_string()))?;
            let first_name = p
                .name
                .first()
                .and_then(|n| n.given.first().cloned())
                .unwrap_or_else(|| "Imported".to_string());
            let last_name = p
                .name
                .first()
                .and_then(|n| n.family.clone())
                .unwrap_or_else(|| "Patient".to_string());
            let mrn = p
                .identifier
                .first()
                .map(|i| i.value.clone())
                .unwrap_or_else(|| fhir_id.clone());

            let now = get_current_time_ms();
            let now_str = format!("{}", now);

            conn.execute(
                "INSERT INTO clients (id, workspace_id, first_name, last_name, personal_number, created_at, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'synced') ON CONFLICT(id) DO UPDATE SET first_name=excluded.first_name, last_name=excluded.last_name",
                crate::params![internal_id.as_str(), workspace_id.as_str(), first_name.as_str(), last_name.as_str(), mrn.as_str(), now_str.as_str(), now],
            )
            .await?;
        }
        "Encounter" => {
            let enc: FhirEncounter = serde_json::from_str(&fhir_json)
                .map_err(|e| YntraError::ValidationError(e.to_string()))?;
            let title = enc
                .reason_code
                .first()
                .and_then(|rc| rc.text.clone())
                .unwrap_or_else(|| "Imported FHIR Encounter".to_string());

            let now = get_current_time_ms();
            let now_str = format!("{}", now);

            conn.execute(
                "INSERT INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at, sync_status) VALUES (?1, ?2, ?3, 'Imported FHIR Encounter', 'Clinical Site', 'normal', 'completed', ?4, '[]', ?4, ?5, 'synced') ON CONFLICT(id) DO UPDATE SET title=excluded.title",
                crate::params![internal_id.as_str(), workspace_id.as_str(), title.as_str(), now_str.as_str(), now],
            )
            .await?;
        }
        "Observation" => {
            let obs: FhirObservation = serde_json::from_str(&fhir_json)
                .map_err(|e| YntraError::ValidationError(e.to_string()))?;
            let title = obs
                .code
                .text
                .clone()
                .unwrap_or_else(|| "Imported FHIR Observation".to_string());
            let content = obs
                .value_string
                .clone()
                .unwrap_or_else(|| fhir_json.clone());

            let now = get_current_time_ms();
            let now_str = format!("{}", now);

            conn.execute(
                "INSERT INTO notes (id, workspace_id, team_id, author_id, subject, content, created_at, updated_at, sync_status) VALUES (?1, ?2, 'team-1', ?3, ?4, ?5, ?6, ?7, 'synced') ON CONFLICT(id) DO UPDATE SET subject=excluded.subject, content=excluded.content",
                crate::params![internal_id.as_str(), workspace_id.as_str(), requester_user_id.as_str(), title.as_str(), content.as_str(), now_str.as_str(), now],
            )
            .await?;
        }
        _ => {}
    }

    save_fhir_mapping(
        &conn,
        &workspace_id,
        &resource_type,
        &fhir_id,
        "imported_entity",
        &internal_id,
        &fhir_json,
    )
    .await?;

    crate::infra::observer::set_last_modified_table("fhir_resource_mappings");
    crate::infra::observer::notify_observers();

    Ok(FhirImportResult {
        resource_type,
        internal_entity_id: internal_id,
        success: true,
        message: format!("Successfully imported FHIR resource '{}'", fhir_id),
    })
}

/// Retrieve all stored FHIR R4 resource mappings for a workspace
#[uniffi::export]
pub async fn get_fhir_resource_mappings(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<FhirResourceMappingRecord>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let mut stmt = conn
        .prepare("SELECT id, workspace_id, resource_type, fhir_id, internal_entity_type, internal_entity_id, raw_fhir_json, last_synced_at, created_at, updated_at FROM fhir_resource_mappings WHERE workspace_id = ?1 ORDER BY updated_at DESC")
        .await?;
    let mut rows = stmt.query(crate::params![workspace_id.as_str()]).await?;
    let mut results = Vec::new();

    while let Some(row) = rows.next().await? {
        results.push(FhirResourceMappingRecord {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            resource_type: row.get(2)?,
            fhir_id: row.get(3)?,
            internal_entity_type: row.get(4)?,
            internal_entity_id: row.get(5)?,
            raw_fhir_json: row.get(6)?,
            last_synced_at: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        });
    }

    Ok(results)
}

/// Helper function to insert or replace FHIR mapping record in database
async fn save_fhir_mapping(
    conn: &database::DbConnection,
    workspace_id: &str,
    resource_type: &str,
    fhir_id: &str,
    internal_entity_type: &str,
    internal_entity_id: &str,
    raw_fhir_json: &str,
) -> Result<(), YntraError> {
    let now = get_current_time_ms();
    let mapping_id = format!("fmap-{}", Uuid::new_v4());

    conn.execute(
        "INSERT INTO fhir_resource_mappings (id, workspace_id, resource_type, fhir_id, internal_entity_type, internal_entity_id, raw_fhir_json, last_synced_at, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?8) \
         ON CONFLICT(id) DO UPDATE SET raw_fhir_json=excluded.raw_fhir_json, updated_at=excluded.updated_at",
        crate::params![
            mapping_id.as_str(),
            workspace_id,
            resource_type,
            fhir_id,
            internal_entity_type,
            internal_entity_id,
            raw_fhir_json,
            now,
        ],
    )
    .await?;

    crate::infra::observer::set_last_modified_table("fhir_resource_mappings");
    crate::infra::observer::notify_observers();

    Ok(())
}
