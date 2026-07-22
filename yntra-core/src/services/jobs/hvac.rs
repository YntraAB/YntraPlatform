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
    pub refrigerant_type: String,
    pub refrigerant_charge_level: String,
    pub high_side_psi: f64,
    pub low_side_psi: f64,
    pub temp_differential_c: f64,
    pub voltage_v: f64,
    pub amp_draw_a: f64,
    pub diagnostic_status: String,
    pub notes: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, uniffi::Record)]
pub struct RotInvoiceSplitBreakdown {
    pub total_gross_amount_sek: f64,
    pub eligible_labor_sek: f64,
    pub non_eligible_parts_sek: f64,
    pub rot_deduction_30_percent_sek: f64,
    pub net_customer_payable_sek: f64,
    pub max_annual_rot_cap_remaining_sek: f64,
    pub rot_eligible_flag: bool,
}

#[uniffi::export]
pub async fn log_hvac_system_diagnostic(
    requester_user_id: String,
    job_ticket_id: String,
    refrigerant_type: String,
    refrigerant_charge_level: String,
    high_side_psi: f64,
    low_side_psi: f64,
    temp_differential_c: f64,
    voltage_v: f64,
    amp_draw_a: f64,
    notes: Option<String>,
) -> Result<HvacSystemDiagnostic, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let diagnostic_status = if high_side_psi > 450.0 || low_side_psi < 20.0 || voltage_v < 195.0 {
        "CRITICAL_HAZARD".to_string()
    } else if temp_differential_c < 8.0 || amp_draw_a > 35.0 {
        "MAINTENANCE_WARNING".to_string()
    } else {
        "SYSTEM_NORMAL".to_string()
    };

    let id = Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO hvac_diagnostics (
            id, workspace_id, job_ticket_id, technician_id, refrigerant_type,
            refrigerant_charge_level, high_side_psi, low_side_psi, temp_differential_c,
            voltage_v, amp_draw_a, diagnostic_status, notes, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        crate::params![
            id.clone(),
            auth.workspace_id.clone(),
            job_ticket_id.clone(),
            auth.user_id.clone(),
            refrigerant_type.clone(),
            refrigerant_charge_level.clone(),
            high_side_psi,
            low_side_psi,
            temp_differential_c,
            voltage_v,
            amp_draw_a,
            diagnostic_status.clone(),
            notes.clone(),
            now_ms,
        ],
    ).await?;

    notify_observers();

    Ok(HvacSystemDiagnostic {
        id,
        workspace_id: auth.workspace_id,
        job_ticket_id,
        technician_id: auth.user_id,
        refrigerant_type,
        refrigerant_charge_level,
        high_side_psi,
        low_side_psi,
        temp_differential_c,
        voltage_v,
        amp_draw_a,
        diagnostic_status,
        notes,
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
        "SELECT id, workspace_id, job_ticket_id, technician_id, refrigerant_type,
                refrigerant_charge_level, high_side_psi, low_side_psi, temp_differential_c,
                voltage_v, amp_draw_a, diagnostic_status, notes, created_at
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
                refrigerant_type: row.get(4)?,
                refrigerant_charge_level: row.get(5)?,
                high_side_psi: row.get(6)?,
                low_side_psi: row.get(7)?,
                temp_differential_c: row.get(8)?,
                voltage_v: row.get(9)?,
                amp_draw_a: row.get(10)?,
                diagnostic_status: row.get(11)?,
                notes: row.get(12)?,
                created_at: row.get(13)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn calculate_hvac_rot_invoice_breakdown(
    requester_user_id: String,
    labor_cost_sek: f64,
    parts_material_cost_sek: f64,
    travel_fee_sek: f64,
) -> Result<RotInvoiceSplitBreakdown, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let eligible_labor_sek = labor_cost_sek.max(0.0);
    let non_eligible_parts_sek = parts_material_cost_sek.max(0.0) + travel_fee_sek.max(0.0);
    let total_gross_amount_sek = eligible_labor_sek + non_eligible_parts_sek;

    // ROT Deduction is 30% of labor cost in Sweden (capped at 50,000 SEK per person per year)
    let rot_deduction_30_percent_sek = (eligible_labor_sek * 0.30).min(50000.0);
    let net_customer_payable_sek = total_gross_amount_sek - rot_deduction_30_percent_sek;

    Ok(RotInvoiceSplitBreakdown {
        total_gross_amount_sek,
        eligible_labor_sek,
        non_eligible_parts_sek,
        rot_deduction_30_percent_sek,
        net_customer_payable_sek,
        max_annual_rot_cap_remaining_sek: 50000.0 - rot_deduction_30_percent_sek,
        rot_eligible_flag: eligible_labor_sek > 0.0,
    })
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
            "R-410A".to_string(),
            "OPTIMAL_95%".to_string(),
            320.0,
            115.0,
            12.5,
            230.0,
            14.2,
            Some("Heat pump operating normally".to_string()),
        ).await.unwrap();

        assert_eq!(diag.diagnostic_status, "SYSTEM_NORMAL");

        let list = get_hvac_job_diagnostics("u-hvac-tech".to_string(), "ticket-101".to_string()).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].refrigerant_type, "R-410A");

        // Calculate ROT split (8000 SEK labor + 3000 SEK parts + 500 SEK travel = 11500 gross)
        let rot_split = calculate_hvac_rot_invoice_breakdown(
            "u-hvac-tech".to_string(),
            8000.0,
            3000.0,
            500.0,
        ).await.unwrap();

        assert_eq!(rot_split.total_gross_amount_sek, 11500.0);
        assert_eq!(rot_split.eligible_labor_sek, 8000.0);
        assert_eq!(rot_split.rot_deduction_30_percent_sek, 2400.0); // 30% of 8000
        assert_eq!(rot_split.net_customer_payable_sek, 9100.0);    // 11500 - 2400

        conn.execute("DELETE FROM hvac_diagnostics WHERE workspace_id = 'ws-hvac-test'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-hvac-tech'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-hvac-test'", ()).await.unwrap();
    }
}
