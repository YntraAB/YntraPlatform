use crate::database;
use crate::infra::observer::notify_observers;
use crate::YntraError;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, uniffi::Record)]
pub struct HvacSystemDiagnostic {
    pub id: String,
    pub workspace_id: String,
    pub job_ticket_id: String,
    pub technician_id: String,
    pub system_type: String,
    pub refrigerant_type: String,
    pub refrigerant_charge_level: String,
    pub high_side_psi: f64,
    pub low_side_psi: f64,
    pub water_pressure_bar: f64,
    pub temp_differential_c: f64,
    pub voltage_v: f64,
    pub amp_draw_a: f64,
    pub diagnostic_status: String,
    pub asset_id: Option<String>,
    pub notes: Option<String>,
    pub operating_mode: Option<String>,
    pub ambient_temp_c: Option<f64>,
    pub created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, uniffi::Record)]
pub struct JobPartItem {
    pub id: String,
    pub workspace_id: String,
    pub job_ticket_id: String,
    pub part_name: String,
    pub quantity: f64,
    pub unit_cost_sek: f64,
    pub rot_eligible: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
pub struct RotInvoiceSplitBreakdown {
    pub total_gross_amount_sek: f64,
    pub eligible_labor_sek: f64,
    pub eligible_parts_sek: f64,
    pub non_eligible_parts_sek: f64,
    pub rot_deduction_30_percent_sek: f64,
    pub net_customer_payable_sek: f64,
    pub max_annual_rot_cap_remaining_sek: f64,
    pub rot_eligible_flag: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, uniffi::Record)]
pub struct LocationEquipmentAsset {
    pub id: String,
    pub asset_tag: String,
    pub model_name: String,
    pub serial_number: String,
    pub equipment_category: String,
    pub location_address: String,
}

#[uniffi::export]
pub async fn log_hvac_system_diagnostic(
    requester_user_id: String,
    job_ticket_id: String,
    system_type: String,
    refrigerant_type: String,
    refrigerant_charge_level: String,
    high_side_psi: f64,
    low_side_psi: f64,
    water_pressure_bar: f64,
    temp_differential_c: f64,
    voltage_v: f64,
    amp_draw_a: f64,
    asset_id: Option<String>,
    notes: Option<String>,
    operating_mode: Option<String>,
    ambient_temp_c: Option<f64>,
) -> Result<HvacSystemDiagnostic, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let sys_type = if system_type.trim().is_empty() {
        "REFRIGERANT_HVAC".to_string()
    } else {
        system_type
    };

    let mode = operating_mode.clone().unwrap_or_else(|| "COOLING_MODE".to_string());

    let diagnostic_status = match sys_type.as_str() {
        "HYDRONIC_HEATING" | "HYDRONIC_PLUMBING" => {
            // Closed radiator heating circuit target pressure: 1.0 - 2.0 bar
            if water_pressure_bar > 3.5 || water_pressure_bar < 0.3 {
                "CRITICAL_HAZARD".to_string()
            } else if water_pressure_bar < 0.8 || water_pressure_bar > 2.5 {
                "MAINTENANCE_WARNING".to_string()
            } else {
                "SYSTEM_NORMAL".to_string()
            }
        }
        "POTABLE_WATER" | "POTABLE_PLUMBING" => {
            // Domestic potable drinking water supply target pressure: 3.0 - 5.0 bar
            if water_pressure_bar > 7.0 || water_pressure_bar < 0.5 {
                "CRITICAL_HAZARD".to_string()
            } else if water_pressure_bar < 2.0 || water_pressure_bar > 5.5 {
                "MAINTENANCE_WARNING".to_string()
            } else {
                "SYSTEM_NORMAL".to_string()
            }
        }
        _ => {
            // Refrigerant HVAC heat pump / AC safety thresholds normalized by operating_mode
            if mode == "HEATING_MODE" {
                match refrigerant_type.as_str() {
                    "R-410A" => {
                        if high_side_psi > 580.0 || low_side_psi < 8.0 || voltage_v < 195.0 {
                            "CRITICAL_HAZARD".to_string()
                        } else if temp_differential_c < 5.0 || amp_draw_a > 38.0 {
                            "MAINTENANCE_WARNING".to_string()
                        } else {
                            "SYSTEM_NORMAL".to_string()
                        }
                    }
                    "R-32" => {
                        if high_side_psi > 600.0 || low_side_psi < 12.0 || voltage_v < 195.0 {
                            "CRITICAL_HAZARD".to_string()
                        } else if temp_differential_c < 6.0 || amp_draw_a > 42.0 {
                            "MAINTENANCE_WARNING".to_string()
                        } else {
                            "SYSTEM_NORMAL".to_string()
                        }
                    }
                    _ => {
                        if high_side_psi > 550.0 || low_side_psi < 10.0 || voltage_v < 195.0 {
                            "CRITICAL_HAZARD".to_string()
                        } else if temp_differential_c < 5.0 || amp_draw_a > 38.0 {
                            "MAINTENANCE_WARNING".to_string()
                        } else {
                            "SYSTEM_NORMAL".to_string()
                        }
                    }
                }
            } else {
                // Standard cooling mode safety threshold rules
                match refrigerant_type.as_str() {
                    "R-134a" => {
                        if high_side_psi > 260.0 || low_side_psi < 10.0 || voltage_v < 195.0 {
                            "CRITICAL_HAZARD".to_string()
                        } else if temp_differential_c < 6.0 || amp_draw_a > 30.0 {
                            "MAINTENANCE_WARNING".to_string()
                        } else {
                            "SYSTEM_NORMAL".to_string()
                        }
                    }
                    "R-22" => {
                        if high_side_psi > 275.0 || low_side_psi < 20.0 || voltage_v < 195.0 {
                            "CRITICAL_HAZARD".to_string()
                        } else if temp_differential_c < 5.0 || amp_draw_a > 25.0 {
                            "MAINTENANCE_WARNING".to_string()
                        } else {
                            "SYSTEM_NORMAL".to_string()
                        }
                    }
                    "R-290" | "Propane" => {
                        if high_side_psi > 290.0 || low_side_psi < 15.0 || voltage_v < 195.0 {
                            "CRITICAL_HAZARD".to_string()
                        } else if temp_differential_c < 7.0 || amp_draw_a > 20.0 {
                            "MAINTENANCE_WARNING".to_string()
                        } else {
                            "SYSTEM_NORMAL".to_string()
                        }
                    }
                    "R-32" => {
                        if high_side_psi > 480.0 || low_side_psi < 25.0 || voltage_v < 195.0 {
                            "CRITICAL_HAZARD".to_string()
                        } else if temp_differential_c < 8.0 || amp_draw_a > 38.0 {
                            "MAINTENANCE_WARNING".to_string()
                        } else {
                            "SYSTEM_NORMAL".to_string()
                        }
                    }
                    _ => {
                        // R-410A, R-454B & default cooling systems
                        if high_side_psi > 450.0 || low_side_psi < 20.0 || voltage_v < 195.0 {
                            "CRITICAL_HAZARD".to_string()
                        } else if temp_differential_c < 8.0 || amp_draw_a > 35.0 {
                            "MAINTENANCE_WARNING".to_string()
                        } else {
                            "SYSTEM_NORMAL".to_string()
                        }
                    }
                }
            }
        }
    };

    let id = Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO hvac_diagnostics (
            id, workspace_id, job_ticket_id, technician_id, system_type, refrigerant_type,
            refrigerant_charge_level, high_side_psi, low_side_psi, water_pressure_bar, temp_differential_c,
            voltage_v, amp_draw_a, diagnostic_status, asset_id, notes, operating_mode, ambient_temp_c, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
        crate::params![
            id.clone(),
            auth.workspace_id.clone(),
            job_ticket_id.clone(),
            auth.user_id.clone(),
            sys_type.clone(),
            refrigerant_type.clone(),
            refrigerant_charge_level.clone(),
            high_side_psi,
            low_side_psi,
            water_pressure_bar,
            temp_differential_c,
            voltage_v,
            amp_draw_a,
            diagnostic_status.clone(),
            asset_id.clone(),
            notes.clone(),
            mode.clone(),
            ambient_temp_c,
            now_ms,
        ],
    ).await?;

    notify_observers();

    Ok(HvacSystemDiagnostic {
        id,
        workspace_id: auth.workspace_id,
        job_ticket_id,
        technician_id: auth.user_id,
        system_type: sys_type,
        refrigerant_type,
        refrigerant_charge_level,
        high_side_psi,
        low_side_psi,
        water_pressure_bar,
        temp_differential_c,
        voltage_v,
        amp_draw_a,
        diagnostic_status,
        asset_id,
        notes,
        operating_mode: Some(mode),
        ambient_temp_c,
        created_at: now_ms,
    })
}

#[uniffi::export]
pub async fn get_hvac_job_diagnostics(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Vec<HvacSystemDiagnostic>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, job_ticket_id, technician_id, system_type, refrigerant_type,
                refrigerant_charge_level, high_side_psi, low_side_psi, water_pressure_bar, temp_differential_c,
                voltage_v, amp_draw_a, diagnostic_status, asset_id, notes, operating_mode, ambient_temp_c, created_at
         FROM hvac_diagnostics
         WHERE job_ticket_id = ?1 AND workspace_id = ?2
         ORDER BY created_at DESC"
    ).await?;

    let list = stmt
        .query_map(crate::params![job_ticket_id, auth.workspace_id], |row| {
            Ok(HvacSystemDiagnostic {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                job_ticket_id: row.get(2)?,
                technician_id: row.get(3)?,
                system_type: row.get(4).unwrap_or_else(|_| "REFRIGERANT_HVAC".to_string()),
                refrigerant_type: row.get(5)?,
                refrigerant_charge_level: row.get(6)?,
                high_side_psi: row.get(7)?,
                low_side_psi: row.get(8)?,
                water_pressure_bar: row.get(9).unwrap_or(0.0),
                temp_differential_c: row.get(10)?,
                voltage_v: row.get(11)?,
                amp_draw_a: row.get(12)?,
                diagnostic_status: row.get(13)?,
                asset_id: row.get(14)?,
                notes: row.get(15)?,
                operating_mode: row.get(16)?,
                ambient_temp_c: row.get(17)?,
                created_at: row.get(18)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn add_job_part_used(
    requester_user_id: String,
    job_ticket_id: String,
    part_name: String,
    quantity: f64,
    unit_cost_sek: f64,
    rot_eligible: bool,
) -> Result<JobPartItem, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let id = Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();
    let rot_int = if rot_eligible { 1 } else { 0 };

    conn.execute(
        "INSERT INTO job_parts_used (
            id, workspace_id, job_ticket_id, part_name, quantity, unit_cost_sek, rot_eligible, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        crate::params![
            id.clone(),
            auth.workspace_id.clone(),
            job_ticket_id.clone(),
            part_name.trim().to_string(),
            quantity.max(0.01),
            unit_cost_sek.max(0.0),
            rot_int,
            now_ms,
        ],
    ).await?;

    notify_observers();

    Ok(JobPartItem {
        id,
        workspace_id: auth.workspace_id,
        job_ticket_id,
        part_name: part_name.trim().to_string(),
        quantity: quantity.max(0.01),
        unit_cost_sek: unit_cost_sek.max(0.0),
        rot_eligible,
        created_at: now_ms,
    })
}

#[uniffi::export]
pub async fn get_job_parts_used(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Vec<JobPartItem>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, job_ticket_id, part_name, quantity, unit_cost_sek, rot_eligible, created_at
         FROM job_parts_used
         WHERE job_ticket_id = ?1 AND workspace_id = ?2
         ORDER BY created_at ASC"
    ).await?;

    let list = stmt
        .query_map(crate::params![job_ticket_id, auth.workspace_id], |row| {
            let rot_int: i64 = row.get(6)?;
            Ok(JobPartItem {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                job_ticket_id: row.get(2)?,
                part_name: row.get(3)?,
                quantity: row.get(4)?,
                unit_cost_sek: row.get(5)?,
                rot_eligible: rot_int != 0,
                created_at: row.get(7)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn delete_job_part_used(
    requester_user_id: String,
    part_id: String,
) -> Result<bool, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let affected = conn.execute(
        "DELETE FROM job_parts_used WHERE id = ?1 AND workspace_id = ?2",
        crate::params![part_id, auth.workspace_id],
    ).await?;

    notify_observers();

    Ok(affected > 0)
}

#[uniffi::export]
pub async fn calculate_hvac_rot_invoice_breakdown(
    requester_user_id: String,
    labor_cost_sek: f64,
    eligible_parts_cost_sek: f64,
    non_eligible_parts_cost_sek: f64,
    travel_fee_sek: f64,
) -> Result<RotInvoiceSplitBreakdown, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let eligible_labor_sek = labor_cost_sek.max(0.0);
    let eligible_parts_sek = eligible_parts_cost_sek.max(0.0);
    let non_eligible_parts_sek = non_eligible_parts_cost_sek.max(0.0) + travel_fee_sek.max(0.0);
    let rot_eligible_base_sek = eligible_labor_sek + eligible_parts_sek;
    let total_gross_amount_sek = rot_eligible_base_sek + non_eligible_parts_sek;

    // ROT Deduction is 30% of eligible labor and materials in Sweden (capped at 50,000 SEK per person per year)
    let rot_deduction_30_percent_sek = (rot_eligible_base_sek * 0.30).min(50000.0);
    let net_customer_payable_sek = total_gross_amount_sek - rot_deduction_30_percent_sek;

    Ok(RotInvoiceSplitBreakdown {
        total_gross_amount_sek,
        eligible_labor_sek,
        eligible_parts_sek,
        non_eligible_parts_sek,
        rot_deduction_30_percent_sek,
        net_customer_payable_sek,
        max_annual_rot_cap_remaining_sek: (50000.0 - rot_deduction_30_percent_sek).max(0.0),
        rot_eligible_flag: rot_eligible_base_sek > 0.0,
    })
}

#[uniffi::export]
pub async fn get_hvac_location_assets(
    requester_user_id: String,
    _job_ticket_id: String,
) -> Result<Vec<LocationEquipmentAsset>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let assets = vec![
        LocationEquipmentAsset {
            id: "asset-101".to_string(),
            asset_tag: "HP-01".to_string(),
            model_name: "NIBE F2120-12 Air-Water Heat Pump".to_string(),
            serial_number: "SN#84920".to_string(),
            equipment_category: "REFRIGERANT_HVAC".to_string(),
            location_address: "Main Utility Room / Outdoor Unit".to_string(),
        },
        LocationEquipmentAsset {
            id: "asset-102".to_string(),
            asset_tag: "BOILER-01".to_string(),
            model_name: "IVT Greenline HE Hydronic Heating Unit".to_string(),
            serial_number: "SN#39201".to_string(),
            equipment_category: "HYDRONIC_HEATING".to_string(),
            location_address: "Basement Technical Utility Room".to_string(),
        },
        LocationEquipmentAsset {
            id: "asset-103".to_string(),
            asset_tag: "PUMP-01".to_string(),
            model_name: "Grundfos Scala2 Domestic Potable Booster".to_string(),
            serial_number: "SN#10293".to_string(),
            equipment_category: "POTABLE_WATER".to_string(),
            location_address: "Main Water Intake Manifold".to_string(),
        },
    ];

    Ok(assets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    #[tokio::test]
    async fn test_hvac_diagnostics_and_rot_split() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-hvac-test', 'HVAC WS', '{\"hvac_plumbing\":true}', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-hvac-tech', 'ws-hvac-test', 'tech@hvac.se', 'admin')", ()).await.unwrap();

        // Log diagnostic
        let diag = log_hvac_system_diagnostic(
            "u-hvac-tech".to_string(),
            "ticket-101".to_string(),
            "REFRIGERANT_HVAC".to_string(),
            "R-410A".to_string(),
            "OPTIMAL_95%".to_string(),
            320.0,
            115.0,
            0.0,
            12.5,
            230.0,
            14.2,
            Some("NIBE-HEAT-PUMP-001".to_string()),
            Some("Heat pump operating normally".to_string()),
            Some("COOLING_MODE".to_string()),
            Some(22.0),
        ).await.unwrap();

        assert_eq!(diag.diagnostic_status, "SYSTEM_NORMAL");
        assert_eq!(diag.asset_id.as_deref(), Some("NIBE-HEAT-PUMP-001"));
        assert_eq!(diag.operating_mode.as_deref(), Some("COOLING_MODE"));

        let list = get_hvac_job_diagnostics("u-hvac-tech".to_string(), "ticket-101".to_string()).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].refrigerant_type, "R-410A");

        // Log Hydronic Heating diagnostic
        let hyd_diag = log_hvac_system_diagnostic(
            "u-hvac-tech".to_string(),
            "ticket-101".to_string(),
            "HYDRONIC_HEATING".to_string(),
            "Hydronic Water".to_string(),
            "N/A".to_string(),
            0.0,
            0.0,
            1.8, // 1.8 bar normal pressure
            10.0,
            230.0,
            5.0,
            None,
            Some("Radiator circuit pressure checked".to_string()),
            Some("HEATING_MODE".to_string()),
            Some(-5.0),
        ).await.unwrap();

        assert_eq!(hyd_diag.diagnostic_status, "SYSTEM_NORMAL");

        // Test parts tracking
        let part = add_job_part_used(
            "u-hvac-tech".to_string(),
            "ticket-101".to_string(),
            "Copper Pipe 3/4in".to_string(),
            2.0,
            450.0,
            false,
        ).await.unwrap();
        assert_eq!(part.part_name, "Copper Pipe 3/4in");

        let parts = get_job_parts_used("u-hvac-tech".to_string(), "ticket-101".to_string()).await.unwrap();
        assert_eq!(parts.len(), 1);

        let deleted = delete_job_part_used("u-hvac-tech".to_string(), part.id).await.unwrap();
        assert!(deleted);

        // Calculate ROT split with eligible parts
        let rot_split = calculate_hvac_rot_invoice_breakdown(
            "u-hvac-tech".to_string(),
            8000.0,
            1000.0,
            2000.0,
            500.0,
        ).await.unwrap();

        assert_eq!(rot_split.total_gross_amount_sek, 11500.0);
        assert_eq!(rot_split.eligible_labor_sek, 8000.0);
        assert_eq!(rot_split.eligible_parts_sek, 1000.0);
        assert_eq!(rot_split.non_eligible_parts_sek, 2500.0);
        assert_eq!(rot_split.rot_deduction_30_percent_sek, 2700.0);
        assert_eq!(rot_split.net_customer_payable_sek, 8800.0);

        conn.execute("DELETE FROM hvac_diagnostics WHERE workspace_id = 'ws-hvac-test'", ()).await.unwrap();
        conn.execute("DELETE FROM job_parts_used WHERE workspace_id = 'ws-hvac-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-hvac-tech'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-hvac-test'", ()).await.unwrap();
    }
}
