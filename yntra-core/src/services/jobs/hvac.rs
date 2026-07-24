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
    pub high_side_psi: Option<f64>,
    pub low_side_psi: Option<f64>,
    pub water_pressure_bar: Option<f64>,
    pub temp_differential_c: Option<f64>,
    pub voltage_v: Option<f64>,
    pub amp_draw_a: Option<f64>,
    pub diagnostic_status: String,
    pub asset_id: Option<String>,
    pub notes: Option<String>,
    pub operating_mode: Option<String>,
    pub ambient_temp_c: Option<f64>,
    pub static_flow_pressure_bar: Option<f64>,
    pub dynamic_flow_pressure_bar: Option<f64>,
    pub pipe_material: Option<String>,
    pub backflow_preventer_status: Option<String>,
    pub water_heater_temp_c: Option<f64>,
    pub leak_test_duration_min: Option<f64>,
    pub leak_test_pressure_drop_bar: Option<f64>,
    pub refrigerant_added_kg: Option<f64>,
    pub refrigerant_recovered_kg: Option<f64>,
    pub reclaim_cylinder_id: Option<String>,
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
    high_side_psi: Option<f64>,
    low_side_psi: Option<f64>,
    water_pressure_bar: Option<f64>,
    temp_differential_c: Option<f64>,
    voltage_v: Option<f64>,
    amp_draw_a: Option<f64>,
    asset_id: Option<String>,
    notes: Option<String>,
    operating_mode: Option<String>,
    ambient_temp_c: Option<f64>,
    static_flow_pressure_bar: Option<f64>,
    dynamic_flow_pressure_bar: Option<f64>,
    pipe_material: Option<String>,
    backflow_preventer_status: Option<String>,
    water_heater_temp_c: Option<f64>,
    leak_test_duration_min: Option<f64>,
    leak_test_pressure_drop_bar: Option<f64>,
    refrigerant_added_kg: Option<f64>,
    refrigerant_recovered_kg: Option<f64>,
    reclaim_cylinder_id: Option<String>,
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
        "HYDRONIC_HEATING" | "HYDRONIC_PLUMBING" | "POTABLE_WATER" | "POTABLE_PLUMBING" => {
            let is_potable = sys_type.contains("POTABLE");
            let backflow_fail = backflow_preventer_status.as_deref() == Some("FAIL_LEAKING");
            let drop = leak_test_pressure_drop_bar.unwrap_or(0.0);
            let drop_hazard = drop > 0.5;
            let drop_warn = drop > 0.1;

            let wp_hazard = if let Some(wp) = water_pressure_bar {
                if is_potable { wp > 7.0 || wp < 0.5 } else { wp > 3.5 || wp < 0.3 }
            } else {
                false
            };

            let wp_warn = if let Some(wp) = water_pressure_bar {
                if is_potable { wp < 2.0 || wp > 5.5 } else { wp < 0.8 || wp > 2.5 }
            } else {
                false
            };

            let heater_temp_warn = if let Some(ht) = water_heater_temp_c {
                ht < 50.0 || ht > 65.0
            } else {
                false
            };

            let flow_drop_warn = match (static_flow_pressure_bar, dynamic_flow_pressure_bar) {
                (Some(s), Some(d)) => (s - d) > 1.5,
                _ => false,
            };

            if backflow_fail || drop_hazard || wp_hazard {
                "CRITICAL_HAZARD".to_string()
            } else if drop_warn || wp_warn || heater_temp_warn || flow_drop_warn {
                "MAINTENANCE_WARNING".to_string()
            } else {
                "SYSTEM_NORMAL".to_string()
            }
        }
        _ => {
            let h_psi = high_side_psi.unwrap_or(0.0);
            let l_psi = low_side_psi.unwrap_or(0.0);
            let volt = voltage_v.unwrap_or(230.0);
            let amps = amp_draw_a.unwrap_or(0.0);
            let t_diff = temp_differential_c.unwrap_or(10.0);

            if mode == "HEATING_MODE" {
                match refrigerant_type.as_str() {
                    "R-410A" => {
                        if (high_side_psi.is_some() && h_psi > 580.0) || (low_side_psi.is_some() && l_psi < 8.0) || (voltage_v.is_some() && volt < 195.0) {
                            "CRITICAL_HAZARD".to_string()
                        } else if (temp_differential_c.is_some() && t_diff < 5.0) || (amp_draw_a.is_some() && amps > 38.0) {
                            "MAINTENANCE_WARNING".to_string()
                        } else {
                            "SYSTEM_NORMAL".to_string()
                        }
                    }
                    "R-32" => {
                        if (high_side_psi.is_some() && h_psi > 600.0) || (low_side_psi.is_some() && l_psi < 12.0) || (voltage_v.is_some() && volt < 195.0) {
                            "CRITICAL_HAZARD".to_string()
                        } else if (temp_differential_c.is_some() && t_diff < 6.0) || (amp_draw_a.is_some() && amps > 42.0) {
                            "MAINTENANCE_WARNING".to_string()
                        } else {
                            "SYSTEM_NORMAL".to_string()
                        }
                    }
                    _ => {
                        if (high_side_psi.is_some() && h_psi > 550.0) || (low_side_psi.is_some() && l_psi < 10.0) || (voltage_v.is_some() && volt < 195.0) {
                            "CRITICAL_HAZARD".to_string()
                        } else if (temp_differential_c.is_some() && t_diff < 5.0) || (amp_draw_a.is_some() && amps > 38.0) {
                            "MAINTENANCE_WARNING".to_string()
                        } else {
                            "SYSTEM_NORMAL".to_string()
                        }
                    }
                }
            } else {
                match refrigerant_type.as_str() {
                    "R-134a" => {
                        if (high_side_psi.is_some() && h_psi > 260.0) || (low_side_psi.is_some() && l_psi < 10.0) || (voltage_v.is_some() && volt < 195.0) {
                            "CRITICAL_HAZARD".to_string()
                        } else if (temp_differential_c.is_some() && t_diff < 6.0) || (amp_draw_a.is_some() && amps > 30.0) {
                            "MAINTENANCE_WARNING".to_string()
                        } else {
                            "SYSTEM_NORMAL".to_string()
                        }
                    }
                    "R-22" => {
                        if (high_side_psi.is_some() && h_psi > 275.0) || (low_side_psi.is_some() && l_psi < 20.0) || (voltage_v.is_some() && volt < 195.0) {
                            "CRITICAL_HAZARD".to_string()
                        } else if (temp_differential_c.is_some() && t_diff < 5.0) || (amp_draw_a.is_some() && amps > 25.0) {
                            "MAINTENANCE_WARNING".to_string()
                        } else {
                            "SYSTEM_NORMAL".to_string()
                        }
                    }
                    "R-290" | "Propane" => {
                        if (high_side_psi.is_some() && h_psi > 290.0) || (low_side_psi.is_some() && l_psi < 15.0) || (voltage_v.is_some() && volt < 195.0) {
                            "CRITICAL_HAZARD".to_string()
                        } else if (temp_differential_c.is_some() && t_diff < 7.0) || (amp_draw_a.is_some() && amps > 20.0) {
                            "MAINTENANCE_WARNING".to_string()
                        } else {
                            "SYSTEM_NORMAL".to_string()
                        }
                    }
                    "R-32" => {
                        if (high_side_psi.is_some() && h_psi > 480.0) || (low_side_psi.is_some() && l_psi < 25.0) || (voltage_v.is_some() && volt < 195.0) {
                            "CRITICAL_HAZARD".to_string()
                        } else if (temp_differential_c.is_some() && t_diff < 8.0) || (amp_draw_a.is_some() && amps > 38.0) {
                            "MAINTENANCE_WARNING".to_string()
                        } else {
                            "SYSTEM_NORMAL".to_string()
                        }
                    }
                    _ => {
                        if (high_side_psi.is_some() && h_psi > 450.0) || (low_side_psi.is_some() && l_psi < 20.0) || (voltage_v.is_some() && volt < 195.0) {
                            "CRITICAL_HAZARD".to_string()
                        } else if (temp_differential_c.is_some() && t_diff < 8.0) || (amp_draw_a.is_some() && amps > 35.0) {
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
            voltage_v, amp_draw_a, diagnostic_status, asset_id, notes, operating_mode, ambient_temp_c,
            static_flow_pressure_bar, dynamic_flow_pressure_bar, pipe_material, backflow_preventer_status,
            water_heater_temp_c, leak_test_duration_min, leak_test_pressure_drop_bar,
            refrigerant_added_kg, refrigerant_recovered_kg, reclaim_cylinder_id, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29)",
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
            static_flow_pressure_bar,
            dynamic_flow_pressure_bar,
            pipe_material.clone(),
            backflow_preventer_status.clone(),
            water_heater_temp_c,
            leak_test_duration_min,
            leak_test_pressure_drop_bar,
            refrigerant_added_kg,
            refrigerant_recovered_kg,
            reclaim_cylinder_id.clone(),
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
        static_flow_pressure_bar,
        dynamic_flow_pressure_bar,
        pipe_material,
        backflow_preventer_status,
        water_heater_temp_c,
        leak_test_duration_min,
        leak_test_pressure_drop_bar,
        refrigerant_added_kg,
        refrigerant_recovered_kg,
        reclaim_cylinder_id,
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
                voltage_v, amp_draw_a, diagnostic_status, asset_id, notes, operating_mode, ambient_temp_c,
                static_flow_pressure_bar, dynamic_flow_pressure_bar, pipe_material, backflow_preventer_status,
                water_heater_temp_c, leak_test_duration_min, leak_test_pressure_drop_bar,
                refrigerant_added_kg, refrigerant_recovered_kg, reclaim_cylinder_id, created_at
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
                high_side_psi: row.get(7).ok(),
                low_side_psi: row.get(8).ok(),
                water_pressure_bar: row.get(9).ok(),
                temp_differential_c: row.get(10).ok(),
                voltage_v: row.get(11).ok(),
                amp_draw_a: row.get(12).ok(),
                diagnostic_status: row.get(13)?,
                asset_id: row.get(14)?,
                notes: row.get(15)?,
                operating_mode: row.get(16)?,
                ambient_temp_c: row.get(17)?,
                static_flow_pressure_bar: row.get(18).ok(),
                dynamic_flow_pressure_bar: row.get(19).ok(),
                pipe_material: row.get(20).ok(),
                backflow_preventer_status: row.get(21).ok(),
                water_heater_temp_c: row.get(22).ok(),
                leak_test_duration_min: row.get(23).ok(),
                leak_test_pressure_drop_bar: row.get(24).ok(),
                refrigerant_added_kg: row.get(25).ok(),
                refrigerant_recovered_kg: row.get(26).ok(),
                reclaim_cylinder_id: row.get(27).ok(),
                created_at: row.get(28)?,
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
    parts_cost_sek: f64,
    travel_fee_sek: f64,
) -> Result<RotInvoiceSplitBreakdown, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let (deduction_rate, annual_cap) = {
        let settings_str: String = conn
            .query_row(
                "SELECT settings FROM workspaces WHERE id = ?1",
                crate::params![auth.workspace_id],
                |r| r.get(0),
            )
            .await
            .unwrap_or_else(|_| "{}".to_string());
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&settings_str) {
            let rate = json.get("rot_deduction_rate").and_then(|v| v.as_f64()).unwrap_or(0.30);
            let cap = json.get("rot_annual_cap_sek").and_then(|v| v.as_f64()).unwrap_or(75000.0);
            (rate, cap)
        } else {
            (0.30, 75000.0)
        }
    };

    let eligible_labor_sek = labor_cost_sek.max(0.0);
    let non_eligible_parts_sek = parts_cost_sek.max(0.0) + travel_fee_sek.max(0.0);
    let total_gross_amount_sek = eligible_labor_sek + non_eligible_parts_sek;

    // ROT / Tax Deduction is configurable per workspace settings (defaulting to 30% rate and updated 75,000 SEK limit).
    // Materials, parts, and travel fees remain 100% non-eligible under standard tax authority rules.
    let rot_deduction_30_percent_sek = (eligible_labor_sek * deduction_rate).min(annual_cap);
    let net_customer_payable_sek = total_gross_amount_sek - rot_deduction_30_percent_sek;

    Ok(RotInvoiceSplitBreakdown {
        total_gross_amount_sek,
        eligible_labor_sek,
        eligible_parts_sek: 0.0,
        non_eligible_parts_sek,
        rot_deduction_30_percent_sek,
        net_customer_payable_sek,
        max_annual_rot_cap_remaining_sek: (annual_cap - rot_deduction_30_percent_sek).max(0.0),
        rot_eligible_flag: eligible_labor_sek > 0.0,
    })
}

#[uniffi::export]
pub async fn add_hvac_location_asset(
    requester_user_id: String,
    job_ticket_id: Option<String>,
    customer_id: Option<String>,
    asset_tag: String,
    model_name: String,
    serial_number: String,
    equipment_category: String,
    location_address: String,
) -> Result<LocationEquipmentAsset, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let id = Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO location_assets (
            id, workspace_id, job_ticket_id, customer_id, asset_tag, model_name, serial_number, equipment_category, location_address, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        crate::params![
            id.clone(),
            auth.workspace_id.clone(),
            job_ticket_id.clone(),
            customer_id.clone(),
            asset_tag.trim().to_string(),
            model_name.trim().to_string(),
            serial_number.trim().to_string(),
            equipment_category.trim().to_string(),
            location_address.trim().to_string(),
            now_ms,
        ],
    ).await?;

    notify_observers();

    Ok(LocationEquipmentAsset {
        id,
        asset_tag: asset_tag.trim().to_string(),
        model_name: model_name.trim().to_string(),
        serial_number: serial_number.trim().to_string(),
        equipment_category: equipment_category.trim().to_string(),
        location_address: location_address.trim().to_string(),
    })
}

#[uniffi::export]
pub async fn get_hvac_location_assets(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Vec<LocationEquipmentAsset>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let mut stmt = conn.prepare(
        "SELECT id, asset_tag, model_name, serial_number, equipment_category, location_address
         FROM location_assets
         WHERE (job_ticket_id = ?1 OR job_ticket_id IS NULL) AND workspace_id = ?2
         ORDER BY created_at DESC"
    ).await?;

    let list = stmt
        .query_map(crate::params![job_ticket_id, auth.workspace_id], |row| {
            Ok(LocationEquipmentAsset {
                id: row.get(0)?,
                asset_tag: row.get(1)?,
                model_name: row.get(2)?,
                serial_number: row.get(3)?,
                equipment_category: row.get(4)?,
                location_address: row.get(5)?,
            })
        })
        .await?;

    Ok(list)
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
            Some(320.0),
            Some(115.0),
            None,
            Some(12.5),
            Some(230.0),
            Some(14.2),
            Some("NIBE-HEAT-PUMP-001".to_string()),
            Some("Heat pump operating normally".to_string()),
            Some("COOLING_MODE".to_string()),
            Some(22.0),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(0.5),
            None,
            Some("CYL-8821".to_string()),
        ).await.unwrap();

        assert_eq!(diag.diagnostic_status, "SYSTEM_NORMAL");
        assert_eq!(diag.asset_id.as_deref(), Some("NIBE-HEAT-PUMP-001"));
        assert_eq!(diag.operating_mode.as_deref(), Some("COOLING_MODE"));
        assert_eq!(diag.refrigerant_added_kg, Some(0.5));
        assert_eq!(diag.reclaim_cylinder_id.as_deref(), Some("CYL-8821"));

        let list = get_hvac_job_diagnostics("u-hvac-tech".to_string(), "ticket-101".to_string()).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].refrigerant_type, "R-410A");
        assert_eq!(list[0].refrigerant_added_kg, Some(0.5));

        // Log Hydronic & Potable Plumbing diagnostic with leak test and backflow check
        let hyd_diag = log_hvac_system_diagnostic(
            "u-hvac-tech".to_string(),
            "ticket-101".to_string(),
            "POTABLE_PLUMBING".to_string(),
            "Hydronic Water".to_string(),
            "N/A".to_string(),
            None,
            None,
            Some(4.2), // 4.2 bar normal pressure
            Some(10.0),
            Some(230.0),
            Some(5.0),
            None,
            Some("Potable water pressure & backflow checked".to_string()),
            Some("HEATING_MODE".to_string()),
            Some(-5.0),
            Some(4.5), // Static pressure bar
            Some(4.2), // Dynamic pressure bar
            Some("PEX".to_string()),
            Some("PASS_TESTED".to_string()),
            Some(58.0), // 58°C safe water heater temp
            Some(30.0), // 30 min test duration
            Some(0.0),  // 0 bar pressure drop
            None,
            None,
            None,
        ).await.unwrap();

        assert_eq!(hyd_diag.diagnostic_status, "SYSTEM_NORMAL");
        assert_eq!(hyd_diag.pipe_material.as_deref(), Some("PEX"));
        assert_eq!(hyd_diag.backflow_preventer_status.as_deref(), Some("PASS_TESTED"));

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

        // Calculate ROT split (parts & travel are 100% non-eligible under Skatteverket rules)
        let rot_split = calculate_hvac_rot_invoice_breakdown(
            "u-hvac-tech".to_string(),
            8000.0,
            3000.0,
            500.0,
        ).await.unwrap();

        assert_eq!(rot_split.total_gross_amount_sek, 11500.0);
        assert_eq!(rot_split.eligible_labor_sek, 8000.0);
        assert_eq!(rot_split.eligible_parts_sek, 0.0);
        assert_eq!(rot_split.non_eligible_parts_sek, 3500.0);
        assert_eq!(rot_split.rot_deduction_30_percent_sek, 2400.0); // 30% of 8,000 SEK labor = 2,400 SEK
        assert_eq!(rot_split.net_customer_payable_sek, 9100.0);

        // Test location asset creation and retrieval without mock data fallback
        let empty_assets = get_hvac_location_assets("u-hvac-tech".to_string(), "ticket-101".to_string()).await.unwrap();
        assert_eq!(empty_assets.len(), 0);

        let created_asset = add_hvac_location_asset(
            "u-hvac-tech".to_string(),
            Some("ticket-101".to_string()),
            None,
            "HP-202".to_string(),
            "Daikin Altherma 3".to_string(),
            "SN#998811".to_string(),
            "REFRIGERANT_HVAC".to_string(),
            "North Utility Room".to_string(),
        ).await.unwrap();
        assert_eq!(created_asset.asset_tag, "HP-202");

        let fetched_assets = get_hvac_location_assets("u-hvac-tech".to_string(), "ticket-101".to_string()).await.unwrap();
        assert_eq!(fetched_assets.len(), 1);
        assert_eq!(fetched_assets[0].model_name, "Daikin Altherma 3");

        conn.execute("DELETE FROM hvac_diagnostics WHERE workspace_id = 'ws-hvac-test'", ()).await.unwrap();
        conn.execute("DELETE FROM job_parts_used WHERE workspace_id = 'ws-hvac-test'", ()).await.unwrap();
        conn.execute("DELETE FROM location_assets WHERE workspace_id = 'ws-hvac-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-hvac-tech'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-hvac-test'", ()).await.unwrap();
    }
}
