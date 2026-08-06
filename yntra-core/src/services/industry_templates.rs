use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::time::get_current_time_ms;
use crate::{BlockItem, YntraError};
use uuid::Uuid;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, uniffi::Record)]
pub struct IndustryTemplate {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub icon: String,
    pub fields_schema: String,
    pub ui_config: String,
    pub seed_data_json: String,
}

#[uniffi::export]
pub async fn get_industry_templates(
    requester_user_id: String,
) -> Result<Vec<IndustryTemplate>, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let templates = vec![
        IndustryTemplate {
            id: "hvac_work_orders".to_string(),
            name: "HVAC & Field Service Work Orders".to_string(),
            category: "HVAC & Field Service".to_string(),
            description: "Manage technician dispatches, customer assets, service inspection checklists, and job priority statuses.".to_string(),
            icon: "wrench".to_string(),
            fields_schema: r#"[
                {"name":"client_name","label":"Customer Name","type":"text","required":true,"placeholder":"Acme Corp / John Doe"},
                {"name":"asset_serial","label":"Asset / Unit Serial #","type":"text","required":true,"placeholder":"HVAC-2026-X99"},
                {"name":"service_date","label":"Scheduled Service Date","type":"date","required":true},
                {"name":"priority","label":"Job Priority","type":"select","required":true,"options":["Normal","High","Urgent"]},
                {"name":"technician","label":"Assigned Technician","type":"text","required":false,"placeholder":"Alex Rivera"},
                {"name":"status","label":"Work Order Status","type":"select","required":true,"options":["New","Assigned","In Progress","Completed"]},
                {"name":"notes","label":"Service Notes & Diagnosis","type":"textarea","required":false,"placeholder":"Replaced compressor capacitor..."}
            ]"#.to_string(),
            ui_config: r#"[
                {"name":"client_name","label":"Customer"},
                {"name":"asset_serial","label":"Unit Serial"},
                {"name":"priority","label":"Priority"},
                {"name":"status","label":"Status"}
            ]"#.to_string(),
            seed_data_json: r#"[
                {"client_name":"Nordic Logistics Center","asset_serial":"HVAC-COMM-8821","service_date":"2026-08-05","priority":"Urgent","technician":"Marcus Vance","status":"In Progress","notes":"Annual coolant pressure flush and sensor recalibration."},
                {"client_name":"Grand Plaza Residence #402","asset_serial":"SPLIT-AC-9011","service_date":"2026-08-06","priority":"Normal","technician":"Sarah Jenkins","status":"Assigned","notes":"Blower fan noise investigation."}
            ]"#.to_string(),
        },
        IndustryTemplate {
            id: "healthcare_patient_log".to_string(),
            name: "Healthcare & Patient Care Log".to_string(),
            category: "Healthcare & Care Services".to_string(),
            description: "Track resident care levels, medication administration logs, vitals observations, and shift nursing notes.".to_string(),
            icon: "activity".to_string(),
            fields_schema: r#"[
                {"name":"patient_name","label":"Patient / Resident Name","type":"text","required":true,"placeholder":"Elena Rostova"},
                {"name":"care_level","label":"Care Severity Level","type":"select","required":true,"options":["Low Care","Moderate Care","High Care","ICU / Monitor"]},
                {"name":"vitals_summary","label":"Latest Vitals (BP / HR)","type":"text","required":false,"placeholder":"120/80 mmHg, 72 bpm"},
                {"name":"medication_given","label":"Medication Administered","type":"boolean","required":true},
                {"name":"attending_nurse","label":"Attending Nurse","type":"text","required":true,"placeholder":"Nurse Clara Barton"},
                {"name":"status","label":"Care Status","type":"select","required":true,"options":["Stable","Observation","Critical","Discharged"]},
                {"name":"shift_notes","label":"Shift Observations","type":"textarea","required":false,"placeholder":"Patient slept comfortably through night shift."}
            ]"#.to_string(),
            ui_config: r#"[
                {"name":"patient_name","label":"Patient"},
                {"name":"care_level","label":"Level"},
                {"name":"attending_nurse","label":"Nurse"},
                {"name":"status","label":"Status"}
            ]"#.to_string(),
            seed_data_json: r#"[
                {"patient_name":"Karl Lindqvist","care_level":"Moderate Care","vitals_summary":"135/85 mmHg, 68 bpm","medication_given":true,"attending_nurse":"Nurse Clara Barton","status":"Stable","shift_notes":"Morning insulin dose administered after breakfast."},
                {"patient_name":"Sofia Berg","care_level":"High Care","vitals_summary":"110/70 mmHg, 78 bpm","medication_given":true,"attending_nurse":"Nurse David Miller","status":"Observation","shift_notes":"Physical therapy exercise completed at 10:30."}
            ]"#.to_string(),
        },
        IndustryTemplate {
            id: "logistics_fleet_manifest".to_string(),
            name: "Logistics & Transport Dispatch Manifest".to_string(),
            category: "Logistics & Transport".to_string(),
            description: "Dispatch tracking for freight vehicles, driver assignments, cargo weight metrics, and delivery status.".to_string(),
            icon: "truck".to_string(),
            fields_schema: r#"[
                {"name":"vehicle_reg","label":"Vehicle Plate / ID","type":"text","required":true,"placeholder":"TRK-9021-EU"},
                {"name":"driver_name","label":"Driver Name","type":"text","required":true,"placeholder":"Erik Olovsson"},
                {"name":"origin","label":"Origin Warehouse","type":"text","required":true,"placeholder":"Stockholm Hub Alpha"},
                {"name":"destination","label":"Destination Address","type":"text","required":true,"placeholder":"Gothenburg Port Gate 4"},
                {"name":"cargo_weight_kg","label":"Cargo Weight (kg)","type":"number","required":true,"placeholder":"14500"},
                {"name":"inspection_passed","label":"Pre-Trip Safety Passed","type":"boolean","required":true},
                {"name":"status","label":"Dispatch Status","type":"select","required":true,"options":["Draft","Dispatched","In Transit","Delivered"]}
            ]"#.to_string(),
            ui_config: r#"[
                {"name":"vehicle_reg","label":"Vehicle"},
                {"name":"driver_name","label":"Driver"},
                {"name":"destination","label":"Destination"},
                {"name":"status","label":"Status"}
            ]"#.to_string(),
            seed_data_json: r#"[
                {"vehicle_reg":"TRK-8801-SE","driver_name":"Lars Hedman","origin":"Malmö Freight Depot","destination":"Stockholm Cargo Central","cargo_weight_kg":18200,"inspection_passed":true,"status":"In Transit"},
                {"vehicle_reg":"VAN-4420-SE","driver_name":"Emma Wallin","origin":"Stockholm Central Hub","destination":"Uppsala Retail Warehouse","cargo_weight_kg":3400,"inspection_passed":true,"status":"Dispatched"}
            ]"#.to_string(),
        },
        IndustryTemplate {
            id: "property_maintenance".to_string(),
            name: "Property & Real Estate Maintenance".to_string(),
            category: "Property Management".to_string(),
            description: "Tenant maintenance request portal, building asset issues, contractor assignments, and resolution status.".to_string(),
            icon: "building".to_string(),
            fields_schema: r#"[
                {"name":"property_unit","label":"Building / Unit #","type":"text","required":true,"placeholder":"Tower B - Apt 1402"},
                {"name":"tenant_contact","label":"Tenant Contact","type":"text","required":true,"placeholder":"Maria Santos (555-0192)"},
                {"name":"category","label":"Issue Category","type":"select","required":true,"options":["Plumbing","Electrical","HVAC","Elevator","General"]},
                {"name":"reported_date","label":"Reported Date","type":"date","required":true},
                {"name":"urgency","label":"Urgency","type":"select","required":true,"options":["Low","Medium","High","Critical Emergency"]},
                {"name":"status","label":"Resolution Status","type":"select","required":true,"options":["Open","Assigned","In Progress","Closed"]},
                {"name":"description","label":"Issue Details","type":"textarea","required":false,"placeholder":"Main bathroom pipe slow leak under sink."}
            ]"#.to_string(),
            ui_config: r#"[
                {"name":"property_unit","label":"Unit"},
                {"name":"category","label":"Category"},
                {"name":"urgency","label":"Urgency"},
                {"name":"status","label":"Status"}
            ]"#.to_string(),
            seed_data_json: r#"[
                {"property_unit":"Sveavägen 44 - Unit 3B","tenant_contact":"Oscar Nygård","category":"Plumbing","reported_date":"2026-08-03","urgency":"High","status":"In Progress","description":"Radiator valve leak requiring gasket replacement."},
                {"property_unit":"Kungsgatan 12 - Commercial 1","tenant_contact":"Café Nordic","category":"Electrical","reported_date":"2026-08-04","urgency":"Medium","status":"Assigned","description":"Display fridge circuit breaker trip."}
            ]"#.to_string(),
        },
        IndustryTemplate {
            id: "retail_stock_audit".to_string(),
            name: "Retail & Asset Stock Audit".to_string(),
            category: "Retail & Asset Management".to_string(),
            description: "Inventory audit logs, reorder thresholds, barcode/SKU tracking, and location warehouse mapping.".to_string(),
            icon: "package".to_string(),
            fields_schema: r#"[
                {"name":"item_name","label":"Product / Asset Name","type":"text","required":true,"placeholder":"Industrial Lithium Battery Pack 48V"},
                {"name":"sku_barcode","label":"SKU / Barcode #","type":"text","required":true,"placeholder":"SKU-BATT-48V-01"},
                {"name":"stock_qty","label":"Current Quantity","type":"number","required":true,"placeholder":"42"},
                {"name":"reorder_level","label":"Reorder Threshold","type":"number","required":true,"placeholder":"15"},
                {"name":"warehouse_loc","label":"Warehouse Aisle / Shelf","type":"text","required":false,"placeholder":"Aisle 4, Shelf B-2"},
                {"name":"status","label":"Stock Status","type":"select","required":true,"options":["In Stock","Low Stock","Reordered","Discontinued"]}
            ]"#.to_string(),
            ui_config: r#"[
                {"name":"item_name","label":"Product Name"},
                {"name":"sku_barcode","label":"SKU"},
                {"name":"stock_qty","label":"Quantity"},
                {"name":"status","label":"Status"}
            ]"#.to_string(),
            seed_data_json: r#"[
                {"item_name":"Pro Wireless Scanner Module","sku_barcode":"SKU-SCAN-900X","stock_qty":8,"reorder_level":12,"warehouse_loc":"Aisle 1, Bin 4","status":"Low Stock"},
                {"item_name":"Heavy Duty Pallet Strapping 19mm","sku_barcode":"SKU-STRAP-19MM","stock_qty":140,"reorder_level":30,"warehouse_loc":"Aisle 8, Pallet Rack 2","status":"In Stock"}
            ]"#.to_string(),
        },
    ];

    Ok(templates)
}

#[uniffi::export]
pub async fn install_industry_template(
    requester_user_id: String,
    workspace_id: String,
    template_id: String,
) -> Result<BlockItem, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if auth.role != "platform_admin" && auth.workspace_id != workspace_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let templates = get_industry_templates(requester_user_id.clone()).await?;
    let tpl = templates
        .into_iter()
        .find(|t| t.id == template_id)
        .ok_or_else(|| YntraError::NotFoundError("Industry template not found".to_string()))?;

    let now_ms = get_current_time_ms();
    let block_id = format!("custom_{}_{}", tpl.id, Uuid::new_v4().simple());
    let now_str = "2026-08-04T12:00:00Z";

    // 1. Insert template as custom block into `blocks` table
    conn.execute(
        "INSERT INTO blocks (id, name, description, icon, category, dependencies, fields_schema, navigation_items, ui_config, created_at) VALUES (?1, ?2, ?3, ?4, ?5, '[]', ?6, '[]', ?7, ?8)",
        crate::params![
            &block_id,
            &tpl.name,
            &tpl.description,
            &tpl.icon,
            &tpl.category,
            &tpl.fields_schema,
            &tpl.ui_config,
            now_str
        ],
    )
    .await?;

    // 2. Add block_id to workspace modules_active list
    let modules_active_str: String = conn
        .query_row(
            "SELECT modules_active FROM workspaces WHERE id = ?1",
            crate::params![&workspace_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "[]".to_string());

    let mut active_ids: Vec<String> = serde_json::from_str(&modules_active_str).unwrap_or_default();
    if !active_ids.contains(&block_id) {
        active_ids.push(block_id.clone());
    }
    let updated_modules_str =
        serde_json::to_string(&active_ids).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "UPDATE workspaces SET modules_active = ?1 WHERE id = ?2",
        crate::params![&updated_modules_str, &workspace_id],
    )
    .await?;

    // 3. Seed initial records into `entities` table
    if let Ok(seed_records) = serde_json::from_str::<Vec<serde_json::Value>>(&tpl.seed_data_json) {
        for record in seed_records {
            let entity_id = Uuid::new_v4().to_string();
            let data_str = serde_json::to_string(&record).unwrap_or_else(|_| "{}".to_string());
            let _ = conn
                .execute(
                    "INSERT INTO entities (id, workspace_id, block_id, entity_type, data, created_at, updated_at, sync_status) VALUES (?1, ?2, ?3, 'custom', ?4, ?5, ?5, 'synced')",
                    crate::params![&entity_id, &workspace_id, &block_id, &data_str, now_ms],
                )
                .await;
        }
    }

    // 4. Auto-register pre-wired cross-block event rules for the suite bundle
    let (src_b, tgt_b, trig_e, act_t) = match template_id.as_str() {
        "hvac_work_orders" => ("time", "jobs", "TimeLogSubmitted", "UpdateJobCosting"),
        "healthcare_patient_log" => ("journals", "messaging", "CareNoteLogged", "PostEmergencyAlert"),
        _ => ("scheduling", "messaging", "DispatchScheduled", "NotifyCrewMessaging"),
    };
    let rule_id = format!("rule_auto_{}", Uuid::new_v4().simple());
    let _ = conn.execute(
        "INSERT INTO event_rules (id, workspace_id, source_block_id, target_block_id, trigger_event, action_type, config_json, is_active, created_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '{}', 1, ?7, 'synced')",
        crate::params![&rule_id, &workspace_id, src_b, tgt_b, trig_e, act_t, now_ms],
    ).await;

    notify_observers();

    Ok(BlockItem {
        id: block_id,
        name: tpl.name,
        description: Some(tpl.description),
        icon: tpl.icon,
        category: tpl.category,
        dependencies: "[]".to_string(),
        fields_schema: Some(tpl.fields_schema),
        navigation_items: Some("[]".to_string()),
        ui_config: Some(tpl.ui_config),
        tier: None,
        compliance_standards: None,
        supported_protocols: None,
        enterprise_connectors: None,
        jurisdiction: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_industry_templates_flow() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-tpl-test', 'Tpl WS', '[]', '{}')",
            (),
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, password_hash, role) VALUES ('u-tpl-test', 'ws-tpl-test', 'admin@tpl.io', 'hash', 'platform_admin')",
            (),
        )
        .await
        .unwrap();

        let templates = get_industry_templates("u-tpl-test".to_string())
            .await
            .unwrap();
        assert_eq!(templates.len(), 5);

        let installed = install_industry_template(
            "u-tpl-test".to_string(),
            "ws-tpl-test".to_string(),
            "hvac_work_orders".to_string(),
        )
        .await
        .unwrap();

        assert!(installed.id.starts_with("custom_hvac_work_orders_"));
        assert_eq!(installed.name, "HVAC & Field Service Work Orders");

        // Verify entities seeded
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE workspace_id = 'ws-tpl-test' AND block_id = ?1",
                crate::params![&installed.id],
                |r| r.get(0),
            )
            .await
            .unwrap();
        assert_eq!(count, 2);
    }
}
