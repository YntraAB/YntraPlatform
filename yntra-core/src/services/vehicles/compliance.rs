use crate::database;
use crate::infra::observer::notify_observers;
use crate::models::jobs::{
    DriverVehicleInspectionReport, EldHosLogRecord, GvwrWeightComplianceWarning,
    IftaStateFuelLogRecord, VehicleDotComplianceSummary,
};
use crate::YntraError;
use uuid::Uuid;

#[uniffi::export]
pub async fn log_eld_hos_status(
    requester_user_id: String,
    driver_id: String,
    driver_name: String,
    vehicle_id: String,
    status: String,
    driving_hours_today: f64,
    on_duty_hours_today: f64,
    cycle_hours_7day: f64,
) -> Result<EldHosLogRecord, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions to log ELD HOS status".to_string(),
        ));
    }

    let rest_break_required = driving_hours_today >= 8.0;
    let mut violation_flag = false;
    let mut violation_reason = None;

    if driving_hours_today > 11.0 {
        violation_flag = true;
        violation_reason = Some("Exceeded DOT 11-Hour Driving Limit Rule".to_string());
    } else if on_duty_hours_today > 14.0 {
        violation_flag = true;
        violation_reason = Some("Exceeded DOT 14-Hour On-Duty Window Limit Rule".to_string());
    } else if cycle_hours_7day > 70.0 {
        violation_flag = true;
        violation_reason = Some("Exceeded DOT 70-Hour 8-Day Cycle Limit Rule".to_string());
    }

    let id = Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO eld_hos_logs (
            id, workspace_id, driver_id, driver_name, vehicle_id, status,
            driving_hours_today, on_duty_hours_today, cycle_hours_7day,
            rest_break_required, violation_flag, violation_reason, timestamp_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        crate::params![
            id.clone(),
            auth.workspace_id.clone(),
            driver_id.clone(),
            driver_name.clone(),
            vehicle_id.clone(),
            status.clone(),
            driving_hours_today,
            on_duty_hours_today,
            cycle_hours_7day,
            if rest_break_required { 1 } else { 0 },
            if violation_flag { 1 } else { 0 },
            violation_reason.clone(),
            now_ms,
        ],
    ).await?;

    notify_observers();

    Ok(EldHosLogRecord {
        id,
        workspace_id: auth.workspace_id,
        driver_id,
        driver_name,
        vehicle_id,
        status,
        driving_hours_today,
        on_duty_hours_today,
        cycle_hours_7day,
        rest_break_required,
        violation_flag,
        violation_reason,
        timestamp_ms: now_ms,
    })
}

#[uniffi::export]
pub async fn submit_dvir_inspection(
    requester_user_id: String,
    vehicle_id: String,
    inspector_driver_id: String,
    inspection_type: String,
    brakes_ok: bool,
    tires_ok: bool,
    lights_ok: bool,
    steering_ok: bool,
    coupling_devices_ok: bool,
    defect_details: Option<String>,
) -> Result<DriverVehicleInspectionReport, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions to submit DVIR".to_string(),
        ));
    }

    let defects_found = !(brakes_ok && tires_ok && lights_ok && steering_ok && coupling_devices_ok)
        || defect_details.as_ref().map_or(false, |d| !d.trim().is_empty());

    let safety_status = if !brakes_ok || !steering_ok {
        "OUT_OF_SERVICE".to_string()
    } else if defects_found {
        "PASS_REPAIR_REQUIRED".to_string()
    } else {
        "PASS_SAFE".to_string()
    };

    let id = Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO dvir_inspections (
            id, workspace_id, vehicle_id, inspector_driver_id, inspection_type,
            brakes_ok, tires_ok, lights_ok, steering_ok, coupling_devices_ok,
            defects_found, defect_details, safety_status, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        crate::params![
            id.clone(),
            auth.workspace_id.clone(),
            vehicle_id.clone(),
            inspector_driver_id.clone(),
            inspection_type.clone(),
            if brakes_ok { 1 } else { 0 },
            if tires_ok { 1 } else { 0 },
            if lights_ok { 1 } else { 0 },
            if steering_ok { 1 } else { 0 },
            if coupling_devices_ok { 1 } else { 0 },
            if defects_found { 1 } else { 0 },
            defect_details.clone(),
            safety_status.clone(),
            now_ms,
        ],
    ).await?;

    notify_observers();

    Ok(DriverVehicleInspectionReport {
        id,
        workspace_id: auth.workspace_id,
        vehicle_id,
        inspector_driver_id,
        inspection_type,
        brakes_ok,
        tires_ok,
        lights_ok,
        steering_ok,
        coupling_devices_ok,
        defects_found,
        defect_details,
        safety_status,
        created_at: now_ms,
    })
}

#[uniffi::export]
pub async fn get_vehicle_dvir_reports(
    requester_user_id: String,
    vehicle_id: String,
) -> Result<Vec<DriverVehicleInspectionReport>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions to view DVIR reports".to_string(),
        ));
    }

    let mut stmt = conn
        .prepare(
            "SELECT id, workspace_id, vehicle_id, inspector_driver_id, inspection_type,
                    brakes_ok, tires_ok, lights_ok, steering_ok, coupling_devices_ok,
                    defects_found, defect_details, safety_status, created_at
             FROM dvir_inspections
             WHERE vehicle_id = ?1 AND workspace_id = ?2
             ORDER BY created_at DESC, rowid DESC"
        )
        .await?;

    let list = stmt
        .query_map(crate::params![vehicle_id, auth.workspace_id], |row| {
            Ok(DriverVehicleInspectionReport {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                vehicle_id: row.get(2)?,
                inspector_driver_id: row.get(3)?,
                inspection_type: row.get(4)?,
                brakes_ok: row.get::<i32>(5)? != 0,
                tires_ok: row.get::<i32>(6)? != 0,
                lights_ok: row.get::<i32>(7)? != 0,
                steering_ok: row.get::<i32>(8)? != 0,
                coupling_devices_ok: row.get::<i32>(9)? != 0,
                defects_found: row.get::<i32>(10)? != 0,
                defect_details: row.get(11)?,
                safety_status: row.get(12)?,
                created_at: row.get(13)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn check_gvwr_overload_status(
    requester_user_id: String,
    vehicle_id: String,
    cargo_weight_kg: f64,
    tare_weight_kg: f64,
    gvwr_kg: f64,
) -> Result<GvwrWeightComplianceWarning, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions to check GVWR overload status".to_string(),
        ));
    }

    let mut stmt = conn
        .prepare("SELECT name, license_plate FROM vehicles WHERE id = ?1 AND workspace_id = ?2")
        .await?;

    let mut rows = stmt
        .query(crate::params![vehicle_id.clone(), auth.workspace_id.clone()])
        .await?;

    let (vehicle_name, license_plate) = if let Some(row) = rows.next().await? {
        (row.get::<String>(0)?, row.get::<String>(1)?)
    } else {
        ("Move Vehicle".to_string(), "TRK-000".to_string())
    };

    let total_actual_gross_weight_kg = tare_weight_kg + cargo_weight_kg;
    let is_overloaded = total_actual_gross_weight_kg > gvwr_kg;
    let overload_margin_kg = if is_overloaded {
        total_actual_gross_weight_kg - gvwr_kg
    } else {
        0.0
    };

    let warning_severity = if !is_overloaded {
        "NORMAL".to_string()
    } else if overload_margin_kg <= 500.0 {
        "WARNING_OVERLOAD".to_string()
    } else {
        "CRITICAL_OVERLOAD".to_string()
    };

    Ok(GvwrWeightComplianceWarning {
        vehicle_id,
        vehicle_name,
        license_plate,
        gvwr_kg,
        current_cargo_weight_kg: cargo_weight_kg,
        tare_weight_kg,
        total_actual_gross_weight_kg,
        is_overloaded,
        overload_margin_kg,
        warning_severity,
    })
}

#[uniffi::export]
pub async fn log_ifta_jurisdiction_crossing(
    requester_user_id: String,
    vehicle_id: String,
    driver_id: String,
    from_jurisdiction: String,
    to_jurisdiction: String,
    odometer_km: f64,
    fuel_purchased_liters: f64,
) -> Result<IftaStateFuelLogRecord, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions to log IFTA crossing".to_string(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO ifta_fuel_logs (
            id, workspace_id, vehicle_id, driver_id, from_jurisdiction,
            to_jurisdiction, odometer_km, fuel_purchased_liters, timestamp_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        crate::params![
            id.clone(),
            auth.workspace_id.clone(),
            vehicle_id.clone(),
            driver_id.clone(),
            from_jurisdiction.clone(),
            to_jurisdiction.clone(),
            odometer_km,
            fuel_purchased_liters,
            now_ms,
        ],
    ).await?;

    notify_observers();

    Ok(IftaStateFuelLogRecord {
        id,
        workspace_id: auth.workspace_id,
        vehicle_id,
        driver_id,
        from_jurisdiction,
        to_jurisdiction,
        odometer_km,
        fuel_purchased_liters,
        timestamp_ms: now_ms,
    })
}

#[uniffi::export]
pub async fn get_vehicle_dot_compliance_summary(
    requester_user_id: String,
    vehicle_id: String,
) -> Result<VehicleDotComplianceSummary, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions to view DOT compliance summary".to_string(),
        ));
    }

    let mut hos_stmt = conn
        .prepare("SELECT status, violation_flag FROM eld_hos_logs WHERE vehicle_id = ?1 AND workspace_id = ?2 ORDER BY timestamp_ms DESC LIMIT 1")
        .await?;
    let mut hos_rows = hos_stmt.query(crate::params![vehicle_id.clone(), auth.workspace_id.clone()]).await?;
    let (active_hos_status, latest_hos_violation) = if let Some(row) = hos_rows.next().await? {
        (row.get::<String>(0)?, row.get::<i32>(1)? == 1)
    } else {
        ("OFF_DUTY".to_string(), false)
    };

    let mut count_stmt = conn
        .prepare("SELECT COUNT(*) FROM eld_hos_logs WHERE vehicle_id = ?1 AND workspace_id = ?2 AND violation_flag = 1")
        .await?;
    let mut count_rows = count_stmt.query(crate::params![vehicle_id.clone(), auth.workspace_id.clone()]).await?;
    let hos_violation_count = if let Some(row) = count_rows.next().await? {
        row.get::<i32>(0)?
    } else {
        0
    };

    let mut dvir_stmt = conn
        .prepare("SELECT safety_status FROM dvir_inspections WHERE vehicle_id = ?1 AND workspace_id = ?2 ORDER BY created_at DESC, rowid DESC LIMIT 1")
        .await?;
    let mut dvir_rows = dvir_stmt.query(crate::params![vehicle_id.clone(), auth.workspace_id.clone()]).await?;
    let latest_dvir_status = if let Some(row) = dvir_rows.next().await? {
        row.get::<String>(0)?
    } else {
        "PASS_SAFE".to_string()
    };

    let mut ifta_stmt = conn
        .prepare("SELECT COUNT(*) FROM ifta_fuel_logs WHERE vehicle_id = ?1 AND workspace_id = ?2")
        .await?;
    let mut ifta_rows = ifta_stmt.query(crate::params![vehicle_id.clone(), auth.workspace_id.clone()]).await?;
    let total_ifta_jurisdictions_logged = if let Some(row) = ifta_rows.next().await? {
        row.get::<i32>(0)?
    } else {
        0
    };

    let is_dot_compliant = !latest_hos_violation && latest_dvir_status != "OUT_OF_SERVICE";

    Ok(VehicleDotComplianceSummary {
        vehicle_id,
        active_hos_status,
        hos_violation_count,
        latest_dvir_status,
        gvwr_status: if is_dot_compliant { "NORMAL".to_string() } else { "ATTENTION_REQUIRED".to_string() },
        total_ifta_jurisdictions_logged,
        is_dot_compliant,
    })
}
