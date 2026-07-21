use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::errors::YntraError;
use crate::services::jobs::tickets::is_staff;
use crate::WorkspaceUser;

#[uniffi::export]
pub async fn assign_vehicle_to_job(
    requester_user_id: String,
    job_id: String,
    assigned_vehicle_id: Option<String>,
) -> Result<(), YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if !is_staff(&auth) {
        return Err(YntraError::AuthError(
            "Access denied: only staff can assign vehicles to jobs".to_string(),
        ));
    }

    // Verify vehicle belongs to workspace if assigned and check capacity constraints
    if let Some(ref vehicle_id) = assigned_vehicle_id {
        let (vehicle_ws, vehicle_name, capacity_m3): (String, String, f64) = conn
            .query_row(
                "SELECT workspace_id, name, capacity_m3 FROM vehicles WHERE id = ?1",
                crate::params![vehicle_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .await
            .map_err(|_| YntraError::NotFoundError("Vehicle not found".to_string()))?;

        if auth.workspace_id != vehicle_ws {
            return Err(YntraError::AuthError(
                "Access denied: vehicle belongs to a different workspace".to_string(),
            ));
        }

        // Calculate total volume of the job inventory
        let mut inv_stmt = conn.prepare(
            "SELECT quantity, estimated_volume_m3 FROM move_inventory WHERE job_ticket_id = ?1",
        ).await?;
        let mut inv_rows = inv_stmt.query(crate::params![&job_id]).await?;
        let mut total_volume = 0.0;
        while let Some(row) = inv_rows.next().await? {
            let quantity: i64 = row.get(0)?;
            let vol: f64 = row.get(1)?;
            total_volume += (quantity as f64) * vol;
        }

        if total_volume > capacity_m3 {
            let settings_str: String = conn
                .query_row(
                    "SELECT settings FROM workspaces WHERE id = ?1",
                    crate::params![&auth.workspace_id],
                    |r| r.get(0),
                )
                .await
                .unwrap_or_else(|_| "{}".to_string());
            let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();

            let enforce_single_trip = settings_json
                .get("enforce_single_trip_capacity")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if enforce_single_trip {
                return Err(YntraError::ValidationError(format!(
                    "Cannot assign vehicle {}: total cargo volume ({:.2} m³) exceeds vehicle capacity ({:.2} m³)",
                    vehicle_name, total_volume, capacity_m3
                )));
            } else {
                let trips_needed = (total_volume / capacity_m3).ceil() as i64;
                tracing::info!(
                    "Vehicle {} assigned to job {} requires {} trips. Cargo volume ({:.2} m³) exceeds vehicle capacity ({:.2} m³)",
                    vehicle_name, job_id, trips_needed, total_volume, capacity_m3
                );
            }
        }
    }

    conn.execute(
        "UPDATE job_tickets SET assigned_vehicle_id = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![assigned_vehicle_id, now_ms, job_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn add_crew_member(
    requester_user_id: String,
    job_id: String,
    user_id: String,
    role: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch on job".to_string(),
        ));
    }

    if !is_staff(&auth) {
        return Err(YntraError::AuthError(
            "Access denied: only staff can assign crew members".to_string(),
        ));
    }

    // Verify user belongs to same workspace
    let (user_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM users WHERE id = ?1",
            crate::params![&user_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("User not found".to_string()))?;

    if auth.workspace_id != user_ws {
        return Err(YntraError::AuthError(
            "Access denied: user belongs to a different workspace".to_string(),
        ));
    }

    // Validate driver license class (C/CE vs B) and EU Tachograph compliance if role is driver
    let role_lower = role.to_lowercase();
    if role_lower.contains("driver") || role_lower.contains("förare") || role_lower.contains("forare") {
        let compliance = validate_driver_tachograph_compliance(
            requester_user_id.clone(),
            user_id.clone(),
            job_id.clone(),
        ).await?;

        if !compliance.is_compliant {
            return Err(YntraError::ValidationError(
                compliance.compliance_warnings.join(" ")
            ));
        }
    }

    conn.execute(
        "INSERT OR REPLACE INTO job_crew (job_ticket_id, user_id, role) VALUES (?1, ?2, ?3)",
        crate::params![job_id, user_id, role],
    ).await?;

    let quote_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM move_quotes WHERE job_ticket_id = ?1",
            crate::params![&job_id],
            |r| r.get::<i64>(0),
        )
        .await
        .unwrap_or(0) > 0;

    if quote_exists {
        drop(conn);
        let _ = crate::services::jobs::moves::calculate_and_save_move_quote(requester_user_id, job_id).await;
    }

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn remove_crew_member(
    requester_user_id: String,
    job_id: String,
    user_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch on job".to_string(),
        ));
    }

    if !is_staff(&auth) {
        return Err(YntraError::AuthError(
            "Access denied: only staff can assign crew members".to_string(),
        ));
    }

    conn.execute(
        "DELETE FROM job_crew WHERE job_ticket_id = ?1 AND user_id = ?2",
        crate::params![job_id, user_id],
    ).await?;

    let quote_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM move_quotes WHERE job_ticket_id = ?1",
            crate::params![&job_id],
            |r| r.get::<i64>(0),
        )
        .await
        .unwrap_or(0) > 0;

    if quote_exists {
        drop(conn);
        let _ = crate::services::jobs::moves::calculate_and_save_move_quote(requester_user_id, job_id).await;
    }

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn get_job_crew(
    requester_user_id: String,
    job_id: String,
) -> Result<Vec<WorkspaceUser>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let cipher = crate::infra::crypto::WorkspaceCipher::new(&auth.workspace_id)?;

    let mut stmt = conn.prepare(
        "SELECT u.id, u.workspace_id, u.email, u.full_name, u.phone, u.role, u.preferences, u.metadata, u.updated_at, u.sync_status 
         FROM users u 
         JOIN job_crew jc ON u.id = jc.user_id 
         WHERE jc.job_ticket_id = ?1",
    ).await?;

    let list = stmt
        .query_map(crate::params![&job_id], |row| {
            let id: String = row.get(0)?;
            let ws_id: Option<String> = row.get(1)?;
            let metadata_str: Option<String> = row.get(7)?;

            let is_self = id == requester_user_id;

            let mut siths_card_id = None;
            let mut nfc_badge_uid = None;
            let mut personal_number = None;
            let mut public_key = None;

            if let Some(ref m_str) = metadata_str {
                if let Ok(meta_val) = serde_json::from_str::<serde_json::Value>(m_str) {
                    public_key = meta_val
                        .get("public_key")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            meta_val
                                .get("siths_public_key")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        });

                    if auth.is_admin || is_self {
                        siths_card_id = meta_val
                            .get("siths_card_id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        nfc_badge_uid = meta_val
                            .get("nfc_badge_uid")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let raw_pnum = meta_val
                            .get("personal_number")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        if raw_pnum.is_some() {
                            personal_number = cipher.decrypt_opt(raw_pnum);
                        }
                    }
                }
            }

            Ok(WorkspaceUser {
                id,
                workspace_id: ws_id,
                email: row.get(2)?,
                full_name: row.get(3)?,
                phone: row.get(4)?,
                role: row.get(5)?,
                preferences: row.get(6)?,
                siths_card_id,
                nfc_badge_uid,
                updated_at: row.get(8)?,
                sync_status: row.get(9)?,
                personal_number,
                public_key,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn validate_driver_tachograph_compliance(
    requester_user_id: String,
    driver_user_id: String,
    job_id: String,
) -> Result<crate::DriverComplianceStatus, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws, assigned_vehicle_id, scheduled_date): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT workspace_id, assigned_vehicle_id, scheduled_date FROM job_tickets WHERE id = ?1",
            crate::params![&job_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
    }

    let user_meta_str: Option<String> = conn
        .query_row(
            "SELECT metadata FROM users WHERE id = ?1",
            crate::params![&driver_user_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(None);

    let user_license = user_meta_str
        .as_deref()
        .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
        .and_then(|v| v.get("driver_license_class").and_then(|l| l.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| "B".to_string());

    let (vehicle_capacity, vehicle_name) = if let Some(ref v_id) = assigned_vehicle_id {
        conn.query_row(
            "SELECT capacity_m3, name FROM vehicles WHERE id = ?1",
            crate::params![v_id],
            |r| Ok((r.get::<f64>(0)?, r.get::<String>(1)?)),
        )
        .await
        .unwrap_or((0.0, "".to_string()))
    } else {
        (0.0, "".to_string())
    };

    let required_license_class = if vehicle_capacity > 20.0 {
        "C/CE".to_string()
    } else {
        "B".to_string()
    };

    let mut warnings = Vec::new();
    let license_upper = user_license.to_uppercase();
    let has_c_license = license_upper.contains('C') || license_upper.contains("CE");

    let mut is_compliant = true;

    if vehicle_capacity > 20.0 && !has_c_license {
        is_compliant = false;
        warnings.push(format!(
            "Driver license violation: User holds class '{}', but assigned vehicle '{}' ({:.1} m³) requires heavy truck license C/CE.",
            user_license, vehicle_name, vehicle_capacity
        ));
    }

    let date_str = scheduled_date.unwrap_or_default();
    let same_day_jobs_count: i64 = if !date_str.trim().is_empty() {
        conn.query_row(
            "SELECT COUNT(*) FROM job_crew jc JOIN job_tickets jt ON jc.job_ticket_id = jt.id WHERE jc.user_id = ?1 AND jt.scheduled_date = ?2 AND jc.job_ticket_id != ?3",
            crate::params![&driver_user_id, &date_str, &job_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(0)
    } else {
        0
    };

    let total_hours = (same_day_jobs_count + 1) as f64 * 4.5;
    let max_hours = 9.0;
    let rest_compliant = total_hours <= max_hours;

    if !rest_compliant {
        is_compliant = false;
        warnings.push(format!(
            "EU Tachograph Regulation 561/2006 violation: Estimated driving time ({:.1}h) on {} exceeds 9.0h daily limit.",
            total_hours, date_str
        ));
    }

    Ok(crate::DriverComplianceStatus {
        is_compliant,
        license_class: user_license,
        required_license_class,
        total_driving_hours_today: total_hours,
        max_allowed_daily_hours: max_hours,
        rest_period_compliant: rest_compliant,
        compliance_warnings: warnings,
    })
}
