use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;
use crate::{
    FurniturePreset, InventoryScanManifest, JobPackagingItem, MoveInventoryItem,
    MoveInventorySummary, MoveQuote,
};

#[uniffi::export]
pub async fn get_move_inventory(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Vec<MoveInventoryItem>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    // Verify workspace scoping
    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, handling_notes, updated_at, sync_status, room_name, estimated_weight_kg, preset_id, barcode_tag, scan_status, last_scanned_at, last_scanned_by FROM move_inventory WHERE job_ticket_id = ?1",
    ).await?;

    let list = stmt
        .query_map(crate::params![job_ticket_id], |row| {
            Ok(MoveInventoryItem {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                job_ticket_id: row.get(2)?,
                item_category: row.get(3)?,
                item_name: row.get(4)?,
                quantity: row.get::<i64>(5)? as i32,
                estimated_volume_m3: row.get(6)?,
                handling_notes: row.get(7)?,
                updated_at: row.get(8)?,
                sync_status: row.get(9)?,
                room_name: row.get(10)?,
                estimated_weight_kg: row.get::<Option<f64>>(11)?.unwrap_or(0.0),
                preset_id: row.get(12)?,
                barcode_tag: row.get(13)?,
                scan_status: row
                    .get::<Option<String>>(14)?
                    .unwrap_or_else(|| "unscanned".to_string()),
                last_scanned_at: row.get(15)?,
                last_scanned_by: row.get(16)?,
            })
        })
        .await?;

    Ok(list)
}

#[uniffi::export]
pub async fn get_move_quote(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Option<MoveQuote>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    // Verify workspace scoping
    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if crate::services::jobs::tickets::is_field_mover_or_driver(&auth) {
        return Err(YntraError::AuthError(
            "Access denied: mover role cannot view financial quotes".to_string(),
        ));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, accepted_at, updated_at, sync_status, manual_price_override, price_discount FROM move_quotes WHERE job_ticket_id = ?1 LIMIT 1",
    ).await?;

    let mut rows = stmt.query(crate::params![job_ticket_id]).await?;
    if let Some(row) = rows.next().await? {
        Ok(Some(MoveQuote {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            job_ticket_id: row.get(2)?,
            base_price: row.get::<f64>(3)?,
            distance_fee: row.get::<f64>(4)?,
            stairs_surcharge: row.get::<f64>(5)?,
            packing_supplies_fee: row.get::<f64>(6)?,
            total_price: row.get::<f64>(7)?,
            status: row.get(8)?,
            accepted_at: row.get(9)?,
            updated_at: row.get(10)?,
            sync_status: row.get(11)?,
            manual_price_override: row.get(12)?,
            price_discount: row.get(13)?,
        }))
    } else {
        Ok(None)
    }
}

#[uniffi::export]
pub async fn accept_move_quote(
    requester_user_id: String,
    quote_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    // Verify workspace scoping
    let (quote_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM move_quotes WHERE id = ?1",
            crate::params![&quote_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Quote not found".to_string()))?;

    if auth.workspace_id != quote_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE move_quotes SET status = 'accepted', accepted_at = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![now_ms, now_ms, quote_id],
    ).await?;

    let customer_id = conn
        .query_row(
            "SELECT id FROM users WHERE workspace_id = ?1 AND role = 'client' LIMIT 1",
            crate::params![&quote_ws],
            |r| r.get::<String>(0),
        )
        .await
        .unwrap_or_else(|_| "client-1".to_string());

    let _ = crate::services::jobs::notifications::send_external_notification(
        requester_user_id.clone(),
        quote_ws,
        customer_id,
        "booking_confirmation".to_string(),
        None,
    )
    .await;

    let existing_invoice_rut: Option<bool> = conn
        .query_row(
            "SELECT rut_deduction > 0.0 FROM move_invoices WHERE quote_id = ?1",
            crate::params![&quote_id],
            |r| r.get(0),
        )
        .await
        .ok();

    if let Some(use_rut) = existing_invoice_rut {
        drop(conn);
        let _ = crate::services::jobs::billing::generate_move_invoice(
            requester_user_id.clone(),
            quote_id.clone(),
            use_rut,
        )
        .await;
    } else {
        notify_observers();
    }

    Ok(())
}

#[uniffi::export]
pub async fn update_customer_personal_number(
    requester_user_id: String,
    customer_id: String,
    personal_number: String,
) -> Result<String, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    use chrono::Datelike;
    let current_year = chrono::Utc::now().year();
    let normalized = crate::services::clients::normalize_swedish_pnum(&personal_number, current_year)
        .ok_or_else(|| YntraError::ValidationError(format!("Invalid Swedish personal number '{}': must be a valid 10 or 12 digit personal number with valid Luhn checksum.", personal_number)))?;

    let (target_ws, raw_meta): (String, Option<String>) = conn
        .query_row(
            "SELECT workspace_id, metadata FROM users WHERE id = ?1",
            crate::params![&customer_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Customer user not found".to_string()))?;

    if auth.workspace_id != target_ws && !auth.is_admin {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let mut meta_json: serde_json::Value = raw_meta
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::json!({}));

    let cipher = crate::infra::crypto::WorkspaceCipher::new(&target_ws)?;
    let enc_pnum = cipher.encrypt(&normalized)?;
    meta_json["personal_number"] = serde_json::Value::String(enc_pnum);

    conn.execute(
        "UPDATE users SET metadata = ?1 WHERE id = ?2",
        crate::params![meta_json.to_string(), customer_id],
    )
    .await?;

    notify_observers();
    Ok(normalized)
}

#[uniffi::export]
pub async fn accept_move_quote_with_rut(
    requester_user_id: String,
    quote_id: String,
    use_rut: bool,
    personal_number: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    let (quote_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM move_quotes WHERE id = ?1",
            crate::params![&quote_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Quote not found".to_string()))?;

    if auth.workspace_id != quote_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let customer_id = conn
        .query_row(
            "SELECT id FROM users WHERE workspace_id = ?1 AND role = 'client' LIMIT 1",
            crate::params![&quote_ws],
            |r| r.get::<String>(0),
        )
        .await
        .unwrap_or_else(|_| requester_user_id.clone());

    if use_rut {
        if let Some(pn) = personal_number {
            update_customer_personal_number(requester_user_id.clone(), customer_id.clone(), pn)
                .await?;
        } else {
            let user_pnum: Option<String> = conn
                .query_row(
                    "SELECT metadata ->> 'personal_number' FROM users WHERE id = ?1",
                    crate::params![&customer_id],
                    |r| r.get(0),
                )
                .await
                .ok()
                .flatten();

            let cipher = crate::infra::crypto::WorkspaceCipher::new(&quote_ws).ok();
            let decrypted_pnum = user_pnum.as_deref().and_then(|pn| {
                cipher
                    .as_ref()
                    .and_then(|c| c.decrypt_opt(Some(pn.to_string())))
            });

            use chrono::Datelike;
            let current_year = chrono::Utc::now().year();
            let valid = decrypted_pnum
                .as_deref()
                .and_then(|pn| crate::services::clients::normalize_swedish_pnum(pn, current_year));

            if valid.is_none() {
                return Err(YntraError::ValidationError(
                    "Missing valid Swedish personal number for RUT deduction. Please provide your personal number (Personnummer/Samordningsnummer) to proceed with RUT tax deduction.".to_string(),
                ));
            }
        }
    }

    drop(conn);

    // 1. Generate move invoice first (validates RUT, calculates pricing, creates invoice record)
    crate::services::jobs::billing::generate_move_invoice(
        requester_user_id.clone(),
        quote_id.clone(),
        use_rut,
    )
    .await?;

    // 2. Only after invoice generation succeeds, update quote status to 'accepted' and send booking confirmation
    let conn = database::acquire_connection().await?;
    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE move_quotes SET status = 'accepted', accepted_at = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3",
        crate::params![now_ms, now_ms, &quote_id],
    ).await?;

    let _ = crate::services::jobs::notifications::send_external_notification(
        requester_user_id,
        quote_ws,
        customer_id,
        "booking_confirmation".to_string(),
        None,
    )
    .await;

    notify_observers();

    Ok(())
}

#[uniffi::export]
pub fn get_furniture_catalog() -> Vec<FurniturePreset> {
    vec![
        // Living Room
        FurniturePreset {
            id: "sofa-3p".to_string(),
            category: "Living Room".to_string(),
            name: "3-Seater Sofa".to_string(),
            default_volume_m3: 1.8,
            default_weight_kg: 75.0,
            default_handling_notes: Some("Requires blanket wrap".to_string()),
        },
        FurniturePreset {
            id: "armchair".to_string(),
            category: "Living Room".to_string(),
            name: "Armchair".to_string(),
            default_volume_m3: 0.6,
            default_weight_kg: 25.0,
            default_handling_notes: None,
        },
        FurniturePreset {
            id: "tv-unit".to_string(),
            category: "Living Room".to_string(),
            name: "TV Console / Stand".to_string(),
            default_volume_m3: 0.8,
            default_weight_kg: 35.0,
            default_handling_notes: Some("Fragile glass/electronics".to_string()),
        },
        FurniturePreset {
            id: "coffee-table".to_string(),
            category: "Living Room".to_string(),
            name: "Coffee Table".to_string(),
            default_volume_m3: 0.4,
            default_weight_kg: 18.0,
            default_handling_notes: None,
        },
        // Bedroom
        FurniturePreset {
            id: "bed-king".to_string(),
            category: "Bedroom".to_string(),
            name: "King Bed & Mattress".to_string(),
            default_volume_m3: 2.4,
            default_weight_kg: 90.0,
            default_handling_notes: Some("Needs frame disassembly".to_string()),
        },
        FurniturePreset {
            id: "bed-single".to_string(),
            category: "Bedroom".to_string(),
            name: "Single Bed & Mattress".to_string(),
            default_volume_m3: 1.2,
            default_weight_kg: 45.0,
            default_handling_notes: None,
        },
        FurniturePreset {
            id: "wardrobe-3d".to_string(),
            category: "Bedroom".to_string(),
            name: "3-Door Wardrobe".to_string(),
            default_volume_m3: 2.0,
            default_weight_kg: 110.0,
            default_handling_notes: Some("2-person lift, disassemble doors".to_string()),
        },
        FurniturePreset {
            id: "dresser".to_string(),
            category: "Bedroom".to_string(),
            name: "Dresser / Chest of Drawers".to_string(),
            default_volume_m3: 0.7,
            default_weight_kg: 40.0,
            default_handling_notes: None,
        },
        // Kitchen & Dining
        FurniturePreset {
            id: "fridge-double".to_string(),
            category: "Kitchen".to_string(),
            name: "Double Door Refrigerator".to_string(),
            default_volume_m3: 1.5,
            default_weight_kg: 95.0,
            default_handling_notes: Some("Heavy appliance trolley required".to_string()),
        },
        FurniturePreset {
            id: "washer".to_string(),
            category: "Kitchen".to_string(),
            name: "Washing Machine / Dryer".to_string(),
            default_volume_m3: 0.6,
            default_weight_kg: 70.0,
            default_handling_notes: Some("Drain water before moving".to_string()),
        },
        FurniturePreset {
            id: "dining-set".to_string(),
            category: "Dining".to_string(),
            name: "Dining Table & 4 Chairs".to_string(),
            default_volume_m3: 1.8,
            default_weight_kg: 65.0,
            default_handling_notes: None,
        },
        // Office & Workspace
        FurniturePreset {
            id: "desk-office".to_string(),
            category: "Office".to_string(),
            name: "Work Desk & Ergonomic Chair".to_string(),
            default_volume_m3: 1.0,
            default_weight_kg: 40.0,
            default_handling_notes: None,
        },
        // Boxes & Supplies
        FurniturePreset {
            id: "box-std".to_string(),
            category: "Boxes".to_string(),
            name: "Standard Moving Box (Medium)".to_string(),
            default_volume_m3: 0.1,
            default_weight_kg: 15.0,
            default_handling_notes: None,
        },
        FurniturePreset {
            id: "box-wardrobe".to_string(),
            category: "Boxes".to_string(),
            name: "Tall Wardrobe Box with Hanger".to_string(),
            default_volume_m3: 0.4,
            default_weight_kg: 12.0,
            default_handling_notes: None,
        },
        // Garage / Outdoor
        FurniturePreset {
            id: "bicycle".to_string(),
            category: "Outdoor & Garage".to_string(),
            name: "Bicycle / E-bike".to_string(),
            default_volume_m3: 0.5,
            default_weight_kg: 18.0,
            default_handling_notes: None,
        },
    ]
}

#[uniffi::export]
pub async fn create_move_inventory_item_with_details(
    requester_user_id: String,
    job_ticket_id: String,
    item_category: String,
    item_name: String,
    quantity: i32,
    estimated_volume_m3: f64,
    handling_notes: Option<String>,
    room_name: Option<String>,
    estimated_weight_kg: Option<f64>,
    preset_id: Option<String>,
) -> Result<(), YntraError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();

    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    // Verify workspace scoping of the job ticket
    let (job_ws, assigned_user_id): (String, Option<String>) = conn
        .query_row(
            "SELECT workspace_id, assigned_user_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    if auth.role == "client" {
        if let Some(ref assigned) = assigned_user_id {
            if assigned != &requester_user_id {
                return Err(YntraError::ValidationError(
                    "Access denied: client does not own this job ticket".to_string(),
                ));
            }
        }
    }

    let job_status: Option<String> = conn
        .query_row(
            "SELECT status FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| r.get(0),
        )
        .await
        .ok();

    let is_staff = crate::services::jobs::tickets::is_staff(&auth);

    if !is_staff {
        if let Some(ref status) = job_status {
            if status == "completed" || status == "in_transit" {
                return Err(YntraError::ValidationError(
                    "Cannot modify inventory after job ticket has been completed or is in transit."
                        .to_string(),
                ));
            }
        }
    }

    let weight = match estimated_weight_kg {
        Some(w) if w > 0.0 => w,
        _ => {
            if let Some(ref pid) = preset_id {
                if let Some(preset) = get_furniture_catalog().into_iter().find(|p| &p.id == pid) {
                    preset.default_weight_kg
                } else {
                    (estimated_volume_m3 * 150.0).max(0.0)
                }
            } else {
                (estimated_volume_m3 * 150.0).max(0.0)
            }
        }
    };
    let job_clean = job_ticket_id.replace('-', "").to_uppercase();
    let id_clean = id.replace('-', "").to_uppercase();
    let job_part = if job_clean.len() >= 12 {
        &job_clean[..12]
    } else {
        &job_clean
    };
    let id_part = if id_clean.len() >= 12 {
        &id_clean[..12]
    } else {
        &id_clean
    };
    let base_barcode = format!("YNT-{}-{}", job_part, id_part);

    conn.execute(
        "INSERT INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, handling_notes, updated_at, sync_status, room_name, estimated_weight_kg, preset_id, barcode_tag, scan_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending', ?10, ?11, ?12, ?13, 'unscanned')",
        crate::params![
            id,
            job_ws,
            job_ticket_id,
            item_category,
            item_name,
            quantity as i64,
            estimated_volume_m3,
            handling_notes,
            now_ms,
            room_name,
            weight,
            preset_id,
            base_barcode
        ],
    ).await?;

    drop(conn);
    let _ = calculate_and_save_move_quote(requester_user_id, job_ticket_id).await;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn scan_inventory_item_by_barcode(
    requester_user_id: String,
    job_ticket_id: String,
    barcode_tag: String,
    target_status: String,
) -> Result<MoveInventoryItem, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError("Access denied".to_string()));
    }

    let clean_barcode = barcode_tag.trim().to_uppercase();
    let now_ms = chrono::Utc::now().timestamp_millis();

    let mut stmt = conn
        .prepare(
            "SELECT id FROM move_inventory
         WHERE job_ticket_id = ?1
           AND (
             UPPER(barcode_tag) = ?2
             OR UPPER(id) = ?2
             OR ?2 LIKE UPPER(barcode_tag) || '#%'
             OR ?2 LIKE UPPER(id) || '#%'
             OR ?2 LIKE UPPER(barcode_tag) || '-%'
             OR ?2 LIKE UPPER(id) || '-%'
           )
         ORDER BY
           CASE
             WHEN UPPER(barcode_tag) = ?2 OR UPPER(id) = ?2 THEN 0
             WHEN scan_status = 'unscanned' THEN 1
             ELSE 2
           END,
           LENGTH(barcode_tag) DESC,
           id ASC",
        )
        .await?;
    let mut rows = stmt
        .query(crate::params![&job_ticket_id, &clean_barcode])
        .await?;
    let mut matching_ids = Vec::new();
    while let Some(row) = rows.next().await? {
        let item_id: String = row.get(0)?;
        matching_ids.push(item_id);
    }

    if let Some(item_id) = matching_ids.into_iter().next() {
        let valid_status = match target_status.to_lowercase().as_str() {
            "packed" => "packed",
            "loaded" => "loaded",
            "unloaded" => "unloaded",
            "missing" => "missing",
            _ => "packed",
        };

        conn.execute(
            "UPDATE move_inventory SET scan_status = ?1, last_scanned_at = ?2, last_scanned_by = ?3, updated_at = ?2, sync_status = 'pending' WHERE id = ?4",
            crate::params![valid_status, now_ms, auth.user_id, item_id]
        ).await?;

        notify_observers();

        let items = get_move_inventory(requester_user_id, job_ticket_id).await?;
        let updated = items
            .into_iter()
            .find(|i| i.id == item_id)
            .ok_or_else(|| YntraError::NotFoundError("Item not found after update".to_string()))?;
        Ok(updated)
    } else {
        Err(YntraError::NotFoundError(format!(
            "No item matching barcode '{}' found on this job",
            barcode_tag
        )))
    }
}

#[uniffi::export]
pub async fn get_inventory_scan_manifest(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<InventoryScanManifest, YntraError> {
    let items = get_move_inventory(requester_user_id, job_ticket_id.clone()).await?;

    let mut total_items = 0;
    let mut packed_count = 0;
    let mut loaded_count = 0;
    let mut unloaded_count = 0;
    let mut missing_count = 0;

    for item in items {
        let q = item.quantity;
        total_items += q;
        match item.scan_status.as_str() {
            "packed" => packed_count += q,
            "loaded" => loaded_count += q,
            "unloaded" => unloaded_count += q,
            "missing" => missing_count += q,
            _ => {}
        }
    }

    Ok(InventoryScanManifest {
        job_ticket_id,
        total_items,
        packed_count,
        loaded_count,
        unloaded_count,
        missing_count,
    })
}

#[uniffi::export]
pub async fn create_move_inventory_item(
    requester_user_id: String,
    job_ticket_id: String,
    item_category: String,
    item_name: String,
    quantity: i32,
    estimated_volume_m3: f64,
    handling_notes: Option<String>,
) -> Result<(), YntraError> {
    create_move_inventory_item_with_details(
        requester_user_id,
        job_ticket_id,
        item_category,
        item_name,
        quantity,
        estimated_volume_m3,
        handling_notes,
        None,
        None,
        None,
    )
    .await
}

#[uniffi::export]
pub fn convert_m3_to_cu_ft(m3: f64) -> f64 {
    (m3 * 35.3146667 * 100.0).round() / 100.0
}

#[uniffi::export]
pub fn convert_cu_ft_to_m3(cu_ft: f64) -> f64 {
    (cu_ft / 35.3146667 * 100.0).round() / 100.0
}

#[uniffi::export]
pub fn convert_kg_to_lbs(kg: f64) -> f64 {
    (kg * 2.20462262 * 10.0).round() / 10.0
}

#[uniffi::export]
pub fn convert_lbs_to_kg(lbs: f64) -> f64 {
    (lbs / 2.20462262 * 10.0).round() / 10.0
}

#[uniffi::export]
pub fn calculate_volume_from_dimensions_cm(length_cm: f64, width_cm: f64, height_cm: f64) -> f64 {
    let m3 = (length_cm / 100.0) * (width_cm / 100.0) * (height_cm / 100.0);
    (m3 * 1000.0).round() / 1000.0
}

#[uniffi::export]
pub fn calculate_volume_from_dimensions_inches(
    length_in: f64,
    width_in: f64,
    height_in: f64,
) -> f64 {
    let cu_in = length_in * width_in * height_in;
    let cu_ft = cu_in / 1728.0;
    (cu_ft * 100.0).round() / 100.0
}

#[uniffi::export]
pub async fn get_move_inventory_summary(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<MoveInventorySummary, YntraError> {
    let items = get_move_inventory(requester_user_id, job_ticket_id.clone()).await?;

    let mut total_vol = 0.0;
    let mut total_weight = 0.0;
    let mut total_count = 0;

    for item in &items {
        let q = item.quantity as f64;
        let item_weight = if item.estimated_weight_kg > 0.0 {
            item.estimated_weight_kg
        } else if item.estimated_volume_m3 > 0.0 {
            item.estimated_volume_m3 * 150.0
        } else {
            0.0
        };
        total_vol += item.estimated_volume_m3 * q;
        total_weight += item_weight * q;
        total_count += item.quantity;
    }

    // Buffer of +20% for truck volume loading efficiency
    let recommended_truck_m3 = (total_vol * 1.2 * 10.0).round() / 10.0;
    let recommended_truck_cu_ft = convert_m3_to_cu_ft(recommended_truck_m3);

    // Determine base crew recommendation from volume and weight
    let base_crew = if total_vol > 35.0 || total_weight > 800.0 {
        4
    } else if total_vol > 15.0 || total_weight > 350.0 {
        3
    } else {
        2
    };

    // Query architectural access factors (floors, elevators, long carry meters) from job_tickets
    let mut access_crew_boost = 0;
    if let Ok(conn) = database::acquire_connection().await {
        if let Ok((orig_fl, dest_fl, orig_ele, dest_ele, carry_m)) = conn
            .query_row(
                "SELECT origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, long_carry_meters FROM job_tickets WHERE id = ?1",
                crate::params![&job_ticket_id],
                |r| Ok((
                    r.get::<i32>(0).unwrap_or(0),
                    r.get::<i32>(1).unwrap_or(0),
                    r.get::<i32>(2).unwrap_or(0),
                    r.get::<i32>(3).unwrap_or(0),
                    r.get::<i32>(4).unwrap_or(0),
                )),
            )
            .await
        {
            // Stair carrying boost when elevator is not present
            let orig_stairs = if orig_ele == 0 { orig_fl } else { 0 };
            let dest_stairs = if dest_ele == 0 { dest_fl } else { 0 };
            let max_stair_climb = orig_stairs.max(dest_stairs);

            if max_stair_climb >= 5 {
                access_crew_boost += 2;
            } else if max_stair_climb >= 3 {
                access_crew_boost += 1;
            }

            // Long carry distance boost (> 50m)
            if carry_m > 50 {
                access_crew_boost += 1;
            }
        }
    }

    // Heavy / Specialty item adjustment (> 150kg single item, piano, safe, or jacuzzi)
    let has_heavy_item = items.iter().any(|i| {
        i.estimated_weight_kg >= 150.0
            || i.item_category.to_lowercase().contains("piano")
            || i.item_name.to_lowercase().contains("piano")
            || i.item_name.to_lowercase().contains("kassaskåp")
            || i.item_name.to_lowercase().contains("safe")
            || i.item_name.to_lowercase().contains("jacuzzi")
    });

    if has_heavy_item && (base_crew + access_crew_boost) < 3 {
        access_crew_boost += 1;
    }

    let recommended_crew = (base_crew + access_crew_boost).min(6);

    let total_vol_m3_rounded = (total_vol * 100.0).round() / 100.0;
    let total_vol_cu_ft_rounded = convert_m3_to_cu_ft(total_vol);
    let total_weight_kg_rounded = (total_weight * 10.0).round() / 10.0;
    let total_weight_lbs_rounded = convert_kg_to_lbs(total_weight);

    // Optional Vehicle Capacity Check if assigned to job
    let mut truck_capacity_exceeded = false;
    let mut truck_capacity_warning = None;

    if let Ok(conn) = database::acquire_connection().await {
        if let Ok(assigned_vehicle_id) = conn
            .query_row(
                "SELECT assigned_vehicle_id FROM job_tickets WHERE id = ?1",
                crate::params![&job_ticket_id],
                |r| r.get::<Option<String>>(0),
            )
            .await
        {
            if let Some(v_id) = assigned_vehicle_id {
                if let Ok((capacity_m3, max_payload_kg)) = conn
                    .query_row(
                        "SELECT capacity_m3, max_payload_kg FROM vehicles WHERE id = ?1",
                        crate::params![&v_id],
                        |r| Ok((r.get::<Option<f64>>(0)?, r.get::<Option<f64>>(1)?)),
                    )
                    .await
                {
                    if let Some(cap) = capacity_m3 {
                        let required_loading_vol = (total_vol * 1.2 * 10.0).round() / 10.0;
                        if required_loading_vol > cap {
                            truck_capacity_exceeded = true;
                            truck_capacity_warning = Some(format!(
                                "Required loading volume with 20% packing buffer ({:.1} m³) exceeds assigned vehicle capacity ({:.1} m³)",
                                required_loading_vol, cap
                            ));
                        }
                    }
                    if let Some(payload) = max_payload_kg {
                        let settings_str: String = conn
                            .query_row(
                                "SELECT settings FROM workspaces WHERE id = (SELECT workspace_id FROM job_tickets WHERE id = ?1)",
                                crate::params![&job_ticket_id],
                                |r| r.get(0),
                            )
                            .await
                            .unwrap_or_else(|_| "{}".to_string());
                        let settings_json: serde_json::Value =
                            serde_json::from_str(&settings_str).unwrap_or_default();

                        let crew_size = settings_json
                            .get("moving_default_crew_size")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(2.0);
                        let crew_weight_kg = crew_size * 85.0; // 85 kg per crew member
                        let equipment_and_fuel_buffer_kg = settings_json
                            .get("vehicle_tare_equipment_buffer_kg")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(300.0); // Fuel, tailgate, ramps, dollies, blankets
                        let operational_tare_buffer = crew_weight_kg + equipment_and_fuel_buffer_kg;
                        let total_operational_payload = total_weight + operational_tare_buffer;

                        if total_operational_payload > payload {
                            truck_capacity_exceeded = true;
                            let msg = format!(
                                "Total operational payload ({:.1} kg cargo + {:.1} kg crew/equipment/fuel = {:.1} kg) exceeds vehicle max payload limit ({:.1} kg)",
                                total_weight,
                                operational_tare_buffer,
                                total_operational_payload,
                                payload
                            );
                            truck_capacity_warning = match truck_capacity_warning {
                                Some(prev) => Some(format!("{}; {}", prev, msg)),
                                None => Some(msg),
                            };
                        }
                    }
                }
            }
        }
    }

    Ok(MoveInventorySummary {
        total_volume_m3: total_vol_m3_rounded,
        total_weight_kg: total_weight_kg_rounded,
        total_volume_cu_ft: total_vol_cu_ft_rounded,
        total_weight_lbs: total_weight_lbs_rounded,
        total_item_count: total_count,
        recommended_truck_m3,
        recommended_truck_cu_ft,
        recommended_crew_size: recommended_crew,
        truck_capacity_exceeded,
        truck_capacity_warning,
    })
}

#[uniffi::export]
pub async fn delete_move_inventory_item(
    requester_user_id: String,
    item_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (item_ws, job_ticket_id): (String, String) = conn
        .query_row(
            "SELECT workspace_id, job_ticket_id FROM move_inventory WHERE id = ?1",
            crate::params![&item_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Inventory item not found".to_string()))?;

    if auth.workspace_id != item_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    let job_status: Option<String> = conn
        .query_row(
            "SELECT status FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| r.get(0),
        )
        .await
        .ok();

    let is_staff = crate::services::jobs::tickets::is_staff(&auth);

    if !is_staff {
        if let Some(ref status) = job_status {
            if status == "completed" || status == "in_transit" {
                return Err(YntraError::ValidationError(
                    "Cannot delete inventory items after job ticket has been completed or is in transit.".to_string(),
                ));
            }
        }
    }

    conn.execute(
        "DELETE FROM move_inventory WHERE id = ?1",
        crate::params![item_id],
    )
    .await?;

    drop(conn);
    let _ = calculate_and_save_move_quote(requester_user_id, job_ticket_id).await;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub fn calculate_item_specialty_surcharge(
    item_category: String,
    item_name: String,
    handling_notes: Option<String>,
    surcharge_piano: f64,
    surcharge_safe: f64,
    surcharge_jacuzzi: f64,
    surcharge_fragile: f64,
) -> f64 {
    let notes_str = handling_notes.as_deref().unwrap_or("");
    let text = format!("{} {} {}", item_category, item_name, notes_str).to_lowercase();

    // 1. Piano / Keyboard / Heavy Instruments
    let piano_keywords = [
        "piano",
        "flygel",
        "grand piano",
        "upright piano",
        "organ",
        "orgel",
        "pianostol",
        "fortepiano",
        "cembalo",
        "harpsichord",
        "spinet",
        "spinetta",
        "pianolyft",
        "klaver",
        "pianino",
    ];
    for kw in piano_keywords {
        if text.contains(kw) {
            return surcharge_piano;
        }
    }

    // 2. Safe / Vault / Heavy Metallic / Machinery
    let safe_keywords = [
        "safe",
        "kassaskåp",
        "kassaskap",
        "värdeskåp",
        "vardeskap",
        "gun safe",
        "vapenskåp",
        "vapenskap",
        "heavy safe",
        "fireproof safe",
        "skåp tungt",
        "skap tungt",
        "tunglyft",
        "valv",
        "penning-skåp",
        "penningskåp",
        "server rack",
        "racksling",
        "heavy machinery",
        "säkerhetsskåp",
        "sakerhetsskap",
    ];
    for kw in safe_keywords {
        if text.contains(kw) {
            return surcharge_safe;
        }
    }

    // 3. Jacuzzi / Bath / Spa / Sauna
    let jacuzzi_keywords = [
        "jacuzzi",
        "badkar",
        "bathtub",
        "spa",
        "spabad",
        "hot tub",
        "hottub",
        "whirlpool",
        "bastu",
        "sauna",
        "bubbelpool",
        "bubbelbad",
        "isbad",
        "massagebadkar",
    ];
    for kw in jacuzzi_keywords {
        if text.contains(kw) {
            return surcharge_jacuzzi;
        }
    }

    // 4. Fragile / Fine Art / Glass / Mirrors / Antiques
    let fragile_keywords = [
        "konst",
        "tavla",
        "painting",
        "fragile",
        "skör",
        "skor",
        "bräcklig",
        "bracklig",
        "glass",
        "glas",
        "mirror",
        "spegel",
        "kristall",
        "crystal",
        "sculpture",
        "skulptur",
        "målning",
        "malning",
        "antikt",
        "antique",
        "porslin",
        "porcelain",
        "akvarium",
        "aquarium",
        "ljuskrona",
        "chandelier",
        "marmor",
        "marble",
        "stenskiva",
        "stone slab",
        "vitrinskåp",
        "vitrinskap",
    ];
    for kw in fragile_keywords {
        if text.contains(kw) {
            return surcharge_fragile;
        }
    }

    0.0
}

#[uniffi::export]
pub fn calculate_item_specialty_surcharge_extended(
    item_category: String,
    item_name: String,
    handling_notes: Option<String>,
    surcharge_piano: f64,
    surcharge_safe: f64,
    surcharge_jacuzzi: f64,
    surcharge_fragile: f64,
    surcharge_server_rack: f64,
    surcharge_fitness_equipment: f64,
    surcharge_marble_glass: f64,
) -> f64 {
    let notes_str = handling_notes.as_deref().unwrap_or("");
    let text = format!("{} {} {}", item_category, item_name, notes_str).to_lowercase();

    // 1. Server Rack / IT Server Equipment
    if text.contains("server rack")
        || text.contains("racksling")
        || text.contains("ups batteri")
        || text.contains("serverstativ")
    {
        return surcharge_server_rack;
    }

    // 2. Heavy Fitness Equipment
    if text.contains("treadmill")
        || text.contains("löpband")
        || text.contains("lopband")
        || text.contains("roddmaskin")
        || text.contains("crosstrainer")
        || text.contains("gym")
    {
        return surcharge_fitness_equipment;
    }

    // 3. Marble / Glass Tabletops
    if text.contains("marmorbord")
        || text.contains("glasbord")
        || text.contains("glass table")
        || text.contains("marble table")
        || text.contains("stenskiva")
    {
        return surcharge_marble_glass;
    }

    calculate_item_specialty_surcharge(
        item_category,
        item_name,
        handling_notes,
        surcharge_piano,
        surcharge_safe,
        surcharge_jacuzzi,
        surcharge_fragile,
    )
}

#[uniffi::export]
pub fn calculate_eligible_stair_labor_surcharge(
    origin_floor: i32,
    destination_floor: i32,
    origin_has_elevator: bool,
    destination_has_elevator: bool,
    origin_staircase_type: Option<String>,
    destination_staircase_type: Option<String>,
    origin_elevator_size: Option<String>,
    destination_elevator_size: Option<String>,
    stairs_surcharge_per_floor: f64,
    surcharge_small_elevator: f64,
    mult_spiral: f64,
    mult_narrow: f64,
    mult_outdoor: f64,
) -> f64 {
    let get_stair_multiplier =
        |stype: Option<&str>| match stype.unwrap_or("standard").to_lowercase().as_str() {
            "spiral" | "spiraltrappa" => mult_spiral,
            "narrow" | "trång" | "trang" => mult_narrow,
            "outdoor" | "utomhustrappa" => mult_outdoor,
            _ => 1.0,
        };

    let origin_stair_mult = get_stair_multiplier(origin_staircase_type.as_deref());
    let dest_stair_mult = get_stair_multiplier(destination_staircase_type.as_deref());

    let mut stairs_surcharge: f64 = 0.0;

    let is_small_elevator =
        |size: Option<&String>| size.map(|s| s.to_lowercase()) == Some("small".to_string());

    // Origin stair surcharge
    if (!origin_has_elevator || is_small_elevator(origin_elevator_size.as_ref()))
        && origin_floor != 0
    {
        let base_stair = (origin_floor.abs() as f64) * stairs_surcharge_per_floor;
        stairs_surcharge += base_stair * origin_stair_mult;
    }
    if origin_has_elevator && is_small_elevator(origin_elevator_size.as_ref()) {
        stairs_surcharge += surcharge_small_elevator;
    }

    // Destination stair surcharge
    if (!destination_has_elevator || is_small_elevator(destination_elevator_size.as_ref()))
        && destination_floor != 0
    {
        let base_stair = (destination_floor.abs() as f64) * stairs_surcharge_per_floor;
        stairs_surcharge += base_stair * dest_stair_mult;
    }
    if destination_has_elevator && is_small_elevator(destination_elevator_size.as_ref()) {
        stairs_surcharge += surcharge_small_elevator;
    }

    stairs_surcharge
}

pub fn calculate_access_and_stair_surcharge_with_multipliers(
    origin_floor: i32,
    destination_floor: i32,
    origin_has_elevator: bool,
    destination_has_elevator: bool,
    origin_staircase_type: Option<String>,
    destination_staircase_type: Option<String>,
    origin_elevator_size: Option<String>,
    destination_elevator_size: Option<String>,
    long_carry_meters: i32,
    requires_crane_hoist: bool,
    stairs_surcharge_per_floor: f64,
    surcharge_long_carry_per_meter: f64,
    surcharge_crane_hoist: f64,
    surcharge_small_elevator: f64,
    mult_spiral: f64,
    mult_narrow: f64,
    mult_outdoor: f64,
) -> f64 {
    let mut stairs_surcharge = calculate_eligible_stair_labor_surcharge(
        origin_floor,
        destination_floor,
        origin_has_elevator,
        destination_has_elevator,
        origin_staircase_type,
        destination_staircase_type,
        origin_elevator_size,
        destination_elevator_size,
        stairs_surcharge_per_floor,
        surcharge_small_elevator,
        mult_spiral,
        mult_narrow,
        mult_outdoor,
    );

    // Long carry surcharge (Non-deductible under Skatteverket RUT)
    if long_carry_meters > 0 {
        stairs_surcharge += (long_carry_meters as f64) * surcharge_long_carry_per_meter;
    }

    // External crane / hoist requirement surcharge (Equipment rental - Non-deductible under Skatteverket RUT)
    if requires_crane_hoist {
        stairs_surcharge += surcharge_crane_hoist;
    }

    stairs_surcharge
}

#[uniffi::export]
pub fn calculate_access_and_stair_surcharge(
    origin_floor: i32,
    destination_floor: i32,
    origin_has_elevator: bool,
    destination_has_elevator: bool,
    origin_staircase_type: Option<String>,
    destination_staircase_type: Option<String>,
    origin_elevator_size: Option<String>,
    destination_elevator_size: Option<String>,
    long_carry_meters: i32,
    requires_crane_hoist: bool,
    stairs_surcharge_per_floor: f64,
    surcharge_long_carry_per_meter: f64,
    surcharge_crane_hoist: f64,
    surcharge_small_elevator: f64,
) -> f64 {
    calculate_access_and_stair_surcharge_with_multipliers(
        origin_floor,
        destination_floor,
        origin_has_elevator,
        destination_has_elevator,
        origin_staircase_type,
        destination_staircase_type,
        origin_elevator_size,
        destination_elevator_size,
        long_carry_meters,
        requires_crane_hoist,
        stairs_surcharge_per_floor,
        surcharge_long_carry_per_meter,
        surcharge_crane_hoist,
        surcharge_small_elevator,
        1.5,
        1.3,
        1.2,
    )
}

#[uniffi::export]
pub fn calculate_packing_materials_tariff_estimate(
    total_volume_m3: f64,
) -> crate::PackingMaterialsTariffBreakdown {
    let vol = total_volume_m3.max(0.0);
    let small_boxes = (vol * 3.5).ceil() as i32;
    let large_boxes = (vol * 2.0).ceil() as i32;
    let wardrobe_boxes = (vol * 0.4).ceil() as i32;
    let tape_rolls = (vol * 0.3).ceil().max(1.0) as i32;
    let stretch_wrap_rolls = (vol * 0.25).ceil().max(1.0) as i32;
    let mattress_bags = (vol * 0.15).ceil() as i32;

    let supplies_cost = (small_boxes as f64 * 25.0)
        + (large_boxes as f64 * 40.0)
        + (wardrobe_boxes as f64 * 120.0)
        + (tape_rolls as f64 * 35.0)
        + (stretch_wrap_rolls as f64 * 150.0)
        + (mattress_bags as f64 * 90.0);

    crate::PackingMaterialsTariffBreakdown {
        total_volume_m3: vol,
        small_boxes_count: small_boxes,
        large_boxes_count: large_boxes,
        wardrobe_boxes_count: wardrobe_boxes,
        tape_rolls_count: tape_rolls,
        stretch_wrap_rolls_count: stretch_wrap_rolls,
        mattress_bags_count: mattress_bags,
        estimated_supplies_cost_sek: supplies_cost,
    }
}

#[uniffi::export]
pub fn convert_volume_to_tariff_weight(
    total_volume_m3: f64,
    is_commercial: bool,
) -> crate::TariffWeightBreakdown {
    let vol_m3 = total_volume_m3.max(0.0);
    let cu_ft = vol_m3 * 35.3147;
    let density_lbs = if is_commercial { 12.0 } else { 7.0 };
    let weight_lbs = cu_ft * density_lbs;
    let weight_kg = weight_lbs * 0.453592;

    let classification = if is_commercial {
        "Commercial Freight (12 lbs/cu.ft)".to_string()
    } else {
        "Household Residential (7 lbs/cu.ft)".to_string()
    };

    let requires_shuttle = vol_m3 > 45.0 || weight_lbs > 10000.0;
    let recommended_payload = (weight_kg * 1.2).ceil();

    crate::TariffWeightBreakdown {
        total_volume_m3: vol_m3,
        total_volume_cu_ft: cu_ft,
        density_lbs_per_cu_ft: density_lbs,
        calculated_weight_lbs: weight_lbs,
        calculated_weight_kg: weight_kg,
        move_type_classification: classification,
        requires_shuttle_truck: requires_shuttle,
        recommended_axle_payload_kg: recommended_payload,
    }
}

#[uniffi::export]
pub async fn calculate_and_save_move_quote(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    // 1. Fetch Job Ticket details
    let mut stmt = conn.prepare(
        "SELECT workspace_id, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, COALESCE(long_carry_meters, 0), COALESCE(toll_fees, 0.0), scheduled_date, route_stops_json FROM job_tickets WHERE id = ?1",
    ).await?;
    let mut rows = stmt.query(crate::params![&job_ticket_id]).await?;
    let (
        job_ws,
        origin_floor,
        destination_floor,
        origin_has_elevator,
        destination_has_elevator,
        long_carry_meters,
        toll_fees,
        scheduled_date,
        route_stops_json,
    ) = if let Some(row) = rows.next().await? {
        (
            row.get::<String>(0)?,
            row.get::<i64>(1)? as i32,
            row.get::<i64>(2)? as i32,
            row.get::<bool>(3)?,
            row.get::<bool>(4)?,
            row.get::<i64>(5)? as i32,
            row.get::<f64>(6)?,
            row.get::<String>(7)?,
            row.get::<Option<String>>(8)?,
        )
    } else {
        return Err(YntraError::NotFoundError("Job not found".to_string()));
    };

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    // 2. Fetch workspace settings:
    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&job_ws],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();

    // 3. Fetch inventory items and calculate volume & specialty surcharges
    let mut inv_stmt = conn.prepare(
        "SELECT quantity, estimated_volume_m3, item_name, item_category, handling_notes FROM move_inventory WHERE job_ticket_id = ?1",
    ).await?;
    let mut inv_rows = inv_stmt.query(crate::params![&job_ticket_id]).await?;
    let mut total_volume = 0.0;
    let mut specialty_surcharge = 0.0;

    let surcharge_piano =
        crate::services::workspaces::get_setting_f64(&settings_json, "surcharge_piano");
    let surcharge_safe =
        crate::services::workspaces::get_setting_f64(&settings_json, "surcharge_safe");
    let surcharge_jacuzzi =
        crate::services::workspaces::get_setting_f64(&settings_json, "surcharge_jacuzzi");
    let surcharge_fragile =
        crate::services::workspaces::get_setting_f64(&settings_json, "surcharge_fragile");
    let surcharge_server_rack =
        crate::services::workspaces::get_setting_f64(&settings_json, "surcharge_server_rack");
    let surcharge_fitness_equipment =
        crate::services::workspaces::get_setting_f64(&settings_json, "surcharge_fitness_equipment");
    let surcharge_marble_glass =
        crate::services::workspaces::get_setting_f64(&settings_json, "surcharge_marble_glass");

    while let Some(row) = inv_rows.next().await? {
        let quantity: i64 = row.get(0)?;
        let vol: f64 = row.get(1)?;
        let item_name: String = row.get(2)?;
        let item_category: String = row.get(3)?;
        let handling_notes: Option<String> = row.get(4)?;
        total_volume += (quantity as f64) * vol;

        let item_fee = calculate_item_specialty_surcharge_extended(
            item_category,
            item_name,
            handling_notes,
            surcharge_piano,
            surcharge_safe,
            surcharge_jacuzzi,
            surcharge_fragile,
            surcharge_server_rack,
            surcharge_fitness_equipment,
            surcharge_marble_glass,
        );
        specialty_surcharge += item_fee * (quantity as f64);
    }
    total_volume = (total_volume * 1000.0).round() / 1000.0;

    // 4. Quoting Calculations:
    let base_rate_per_m3 =
        crate::services::workspaces::get_setting_f64(&settings_json, "moving_base_rate_per_m3");
    let distance_fee_flat =
        crate::services::workspaces::get_setting_f64(&settings_json, "moving_distance_fee_flat");
    let stairs_surcharge_per_floor = crate::services::workspaces::get_setting_f64(
        &settings_json,
        "moving_stairs_surcharge_per_floor",
    );
    let packing_supplies_fee_per_m3 = crate::services::workspaces::get_setting_f64(
        &settings_json,
        "moving_packing_supplies_fee_per_m3",
    );

    // Dynamic Multipliers & Tariffs
    // Inspect job-specific route metadata first to prevent global workspace settings pollution across concurrent jobs
    let job_distance_km: Option<f64> = route_stops_json
        .as_deref()
        .and_then(|json_str| serde_json::from_str::<serde_json::Value>(json_str).ok())
        .and_then(|v| {
            v.get("estimated_distance_km")
                .or_else(|| v.get("distance_km"))
                .or_else(|| v.get("total_distance_km"))
                .and_then(|d| d.as_f64())
        });

    let distance_km = job_distance_km
        .or_else(|| {
            settings_json
                .get("estimated_distance_km")
                .and_then(|v| v.as_f64())
        })
        .unwrap_or(0.0);

    // Depot & Round-Trip Distance Accounting
    let depot_to_origin_km = route_stops_json
        .as_deref()
        .and_then(|json_str| serde_json::from_str::<serde_json::Value>(json_str).ok())
        .and_then(|v| v.get("depot_to_origin_km").and_then(|d| d.as_f64()))
        .or_else(|| {
            settings_json
                .get("depot_to_origin_km")
                .and_then(|v| v.as_f64())
        })
        .unwrap_or(0.0);

    let destination_to_depot_km = route_stops_json
        .as_deref()
        .and_then(|json_str| serde_json::from_str::<serde_json::Value>(json_str).ok())
        .and_then(|v| v.get("destination_to_depot_km").and_then(|d| d.as_f64()))
        .or_else(|| {
            settings_json
                .get("destination_to_depot_km")
                .and_then(|v| v.as_f64())
        })
        .unwrap_or(0.0);

    let is_roundtrip = route_stops_json
        .as_deref()
        .and_then(|json_str| serde_json::from_str::<serde_json::Value>(json_str).ok())
        .and_then(|v| v.get("include_roundtrip").and_then(|b| b.as_bool()))
        .or_else(|| {
            settings_json
                .get("moving_include_roundtrip")
                .and_then(|v| v.as_bool())
        })
        .unwrap_or(false);

    let effective_distance_km = if is_roundtrip {
        (distance_km * 2.0) + depot_to_origin_km + destination_to_depot_km
    } else {
        distance_km + depot_to_origin_km + destination_to_depot_km
    };

    let local_radius =
        crate::services::workspaces::get_setting_f64(&settings_json, "moving_local_radius_km");
    let per_km_rate =
        crate::services::workspaces::get_setting_f64(&settings_json, "moving_per_km_rate");

    let mut distance_fee = distance_fee_flat;
    if effective_distance_km > local_radius {
        distance_fee += (effective_distance_km - local_radius) * per_km_rate;
    }
    distance_fee += toll_fees;

    // Base hourly/labor rate depending on configured pricing model
    let pricing_model_str = crate::services::workspaces::get_setting_str(
        &settings_json,
        "moving_pricing_model",
        "volume",
    );
    let pricing_model = pricing_model_str.as_str();

    let mut base_price = if pricing_model == "hourly" {
        let crew_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM job_crew WHERE job_ticket_id = ?1",
                crate::params![&job_ticket_id],
                |r| r.get(0),
            )
            .await
            .unwrap_or(0);

        let default_crew_size = crate::services::workspaces::get_setting_f64(
            &settings_json,
            "moving_default_crew_size",
        );

        let active_crew_size = if crew_count > 0 {
            crew_count as f64
        } else {
            default_crew_size
        };

        let hourly_rate_per_mover = crate::services::workspaces::get_setting_f64(
            &settings_json,
            "moving_hourly_rate_per_mover",
        );

        let hourly_rate_vehicle = crate::services::workspaces::get_setting_f64(
            &settings_json,
            "moving_hourly_rate_vehicle",
        );

        let hourly_rate = if settings_json.get("moving_hourly_rate_per_mover").is_some()
            || settings_json.get("moving_hourly_rate_vehicle").is_some()
        {
            active_crew_size * hourly_rate_per_mover + hourly_rate_vehicle
        } else {
            crate::services::workspaces::get_setting_f64(&settings_json, "moving_hourly_rate")
        };

        let hours_per_m3 =
            crate::services::workspaces::get_setting_f64(&settings_json, "moving_hours_per_m3");
        let minimum_hours =
            crate::services::workspaces::get_setting_f64(&settings_json, "moving_minimum_hours");
        let estimated_hours = (total_volume * hours_per_m3).max(minimum_hours);
        estimated_hours * hourly_rate
    } else {
        total_volume * base_rate_per_m3
    };

    // Apply Weekend and Peak-Season / End-of-Month Multipliers
    if let Ok(date) = chrono::NaiveDate::parse_from_str(&scheduled_date, "%Y-%m-%d") {
        use chrono::Datelike;
        let weekday = date.weekday();
        let is_weekend = weekday == chrono::Weekday::Sat || weekday == chrono::Weekday::Sun;
        let day_of_month = date.day();
        let is_peak_season = day_of_month >= 25 || day_of_month <= 3;

        let weekend_multiplier = if is_weekend {
            crate::services::workspaces::get_setting_f64(
                &settings_json,
                "moving_weekend_multiplier",
            )
        } else {
            1.0
        };
        let peak_multiplier = if is_peak_season {
            crate::services::workspaces::get_setting_f64(
                &settings_json,
                "moving_peak_season_multiplier",
            )
        } else {
            1.0
        };

        base_price = base_price * weekend_multiplier * peak_multiplier;
    }

    // Add specialty/heavy item handling fees to base labor price
    base_price += specialty_surcharge;

    let surcharge_long_carry_per_meter = crate::services::workspaces::get_setting_f64(
        &settings_json,
        "surcharge_long_carry_per_meter",
    );
    let surcharge_crane_hoist =
        crate::services::workspaces::get_setting_f64(&settings_json, "surcharge_crane_hoist");
    let surcharge_small_elevator =
        crate::services::workspaces::get_setting_f64(&settings_json, "surcharge_small_elevator");

    let route_meta_json: Option<serde_json::Value> = route_stops_json
        .as_deref()
        .and_then(|json_str| serde_json::from_str::<serde_json::Value>(json_str).ok());

    let origin_staircase_type = route_meta_json
        .as_ref()
        .and_then(|v| v.get("origin_staircase_type").and_then(|s| s.as_str()))
        .or_else(|| {
            settings_json
                .get("origin_staircase_type")
                .and_then(|v| v.as_str())
        })
        .map(|s| s.to_string());

    let destination_staircase_type = route_meta_json
        .as_ref()
        .and_then(|v| v.get("destination_staircase_type").and_then(|s| s.as_str()))
        .or_else(|| {
            settings_json
                .get("destination_staircase_type")
                .and_then(|v| v.as_str())
        })
        .map(|s| s.to_string());

    let origin_elevator_size = route_meta_json
        .as_ref()
        .and_then(|v| v.get("origin_elevator_size").and_then(|s| s.as_str()))
        .or_else(|| {
            settings_json
                .get("origin_elevator_size")
                .and_then(|v| v.as_str())
        })
        .map(|s| s.to_string());

    let destination_elevator_size = route_meta_json
        .as_ref()
        .and_then(|v| v.get("destination_elevator_size").and_then(|s| s.as_str()))
        .or_else(|| {
            settings_json
                .get("destination_elevator_size")
                .and_then(|v| v.as_str())
        })
        .map(|s| s.to_string());

    let requires_crane_hoist = route_meta_json
        .as_ref()
        .and_then(|v| v.get("requires_crane_hoist").and_then(|b| b.as_bool()))
        .or_else(|| {
            settings_json
                .get("requires_crane_hoist")
                .and_then(|v| v.as_bool())
        })
        .unwrap_or(false);

    let mult_spiral =
        crate::services::workspaces::get_setting_f64(&settings_json, "mult_spiral_staircase");
    let mult_narrow =
        crate::services::workspaces::get_setting_f64(&settings_json, "mult_narrow_staircase");
    let mult_outdoor =
        crate::services::workspaces::get_setting_f64(&settings_json, "mult_outdoor_staircase");

    let stairs_surcharge = calculate_access_and_stair_surcharge_with_multipliers(
        origin_floor,
        destination_floor,
        origin_has_elevator,
        destination_has_elevator,
        origin_staircase_type,
        destination_staircase_type,
        origin_elevator_size,
        destination_elevator_size,
        long_carry_meters,
        requires_crane_hoist,
        stairs_surcharge_per_floor,
        surcharge_long_carry_per_meter,
        surcharge_crane_hoist,
        surcharge_small_elevator,
        mult_spiral,
        mult_narrow,
        mult_outdoor,
    );
    let actual_supplies_cost: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(quantity * price_per_unit), 0.0) FROM job_packaging_items WHERE job_ticket_id = ?1",
            crate::params![&job_ticket_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(0.0);

    let packing_supplies_fee = if actual_supplies_cost > 0.0 {
        actual_supplies_cost
    } else {
        total_volume * packing_supplies_fee_per_m3
    };
    let now_ms = chrono::Utc::now().timestamp_millis();

    // Check if quote exists to keep its status, default to "sent"
    let mut quote_stmt = conn.prepare(
        "SELECT id, status, manual_price_override, price_discount, base_price, distance_fee, stairs_surcharge, packing_supplies_fee FROM move_quotes WHERE job_ticket_id = ?1 LIMIT 1",
    ).await?;
    let mut q_rows = quote_stmt.query(crate::params![&job_ticket_id]).await?;
    let (
        quote_id,
        quote_status,
        existing_override,
        existing_discount,
        old_base,
        old_dist,
        old_stairs,
        old_supplies,
    ) = if let Some(row) = q_rows.next().await? {
        (
            row.get::<String>(0)?,
            row.get::<String>(1)?,
            row.get::<Option<f64>>(2)?,
            row.get::<Option<f64>>(3)?,
            row.get::<f64>(4).unwrap_or(0.0),
            row.get::<f64>(5).unwrap_or(0.0),
            row.get::<f64>(6).unwrap_or(0.0),
            row.get::<f64>(7).unwrap_or(0.0),
        )
    } else {
        (
            uuid::Uuid::new_v4().to_string(),
            "sent".to_string(),
            None,
            None,
            0.0,
            0.0,
            0.0,
            0.0,
        )
    };
    drop(q_rows);
    drop(quote_stmt);

    let (effective_override, effective_base_price) = if let Some(override_val) = existing_override {
        (Some(override_val), override_val)
    } else {
        (None, base_price)
    };

    let calculated_total =
        effective_base_price + distance_fee + stairs_surcharge + packing_supplies_fee;
    let old_calculated_total = old_base + old_dist + old_stairs + old_supplies;

    let mut total_price = calculated_total;
    if let Some(discount) = existing_discount {
        total_price = (total_price - discount).max(0.0);
    }

    let mut save_status = quote_status.clone();
    if (quote_status == "accepted" || quote_status == "pending_deposit")
        && old_calculated_total > 0.0
        && (calculated_total - old_calculated_total).abs() > 0.01
    {
        save_status = "revised".to_string();
    }

    conn.execute(
        "INSERT OR REPLACE INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at, sync_status, manual_price_override, price_discount) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'pending', ?11, ?12)",
        crate::params![
            quote_id,
            job_ws,
            job_ticket_id,
            effective_base_price,
            distance_fee,
            stairs_surcharge,
            packing_supplies_fee,
            total_price,
            save_status,
            now_ms,
            effective_override,
            existing_discount
        ],
    ).await?;

    if quote_status == "pending_deposit" && save_status == "revised" {
        conn.execute(
            "UPDATE job_tickets SET status = 'pending', updated_at = ?1 WHERE id = ?2 AND workspace_id = ?3 AND status = 'deposit_pending'",
            crate::params![now_ms, &job_ticket_id, &job_ws],
        ).await.ok();
    }

    if old_calculated_total > 0.0 && (total_price - old_calculated_total).abs() > 0.01 {
        let rev_id = uuid::Uuid::new_v4().to_string();
        let reason = "Self-service inventory modification".to_string();
        conn.execute(
            "INSERT INTO move_quote_revisions (id, workspace_id, quote_id, job_ticket_id, actor_user_id, previous_total, new_total, revision_reason, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            crate::params![
                rev_id,
                job_ws.clone(),
                quote_id.clone(),
                job_ticket_id.clone(),
                requester_user_id.clone(),
                old_calculated_total,
                total_price,
                reason,
                now_ms
            ],
        ).await.ok();

        let customer_id = conn
            .query_row(
                "SELECT id FROM users WHERE workspace_id = ?1 AND role = 'client' LIMIT 1",
                crate::params![&job_ws],
                |r| r.get::<String>(0),
            )
            .await
            .unwrap_or_else(|_| requester_user_id.clone());

        let _ = crate::services::jobs::notifications::send_external_notification(
            requester_user_id.clone(),
            job_ws.clone(),
            customer_id,
            "quote_revised".to_string(),
            None,
        )
        .await;
    }

    notify_observers();
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize, uniffi::Record, Clone, Debug, PartialEq)]
pub struct MoveQuoteRevision {
    pub id: String,
    pub workspace_id: String,
    pub quote_id: String,
    pub job_ticket_id: String,
    pub actor_user_id: String,
    pub previous_total: f64,
    pub new_total: f64,
    pub revision_reason: String,
    pub created_at: i64,
}

#[uniffi::export]
pub async fn get_move_quote_revisions(
    requester_user_id: String,
    quote_id: String,
) -> Result<Vec<MoveQuoteRevision>, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, quote_id, job_ticket_id, actor_user_id, previous_total, new_total, revision_reason, created_at FROM move_quote_revisions WHERE quote_id = ?1 ORDER BY created_at DESC",
    ).await?;
    let mut rows = stmt.query(crate::params![quote_id]).await?;

    let mut list = Vec::new();
    while let Some(row) = rows.next().await? {
        list.push(MoveQuoteRevision {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            quote_id: row.get(2)?,
            job_ticket_id: row.get(3)?,
            actor_user_id: row.get(4)?,
            previous_total: row.get(5)?,
            new_total: row.get(6)?,
            revision_reason: row.get(7)?,
            created_at: row.get(8)?,
        });
    }
    Ok(list)
}

#[uniffi::export]
pub async fn add_job_packaging_item(
    requester_user_id: String,
    job_ticket_id: String,
    item_name: String,
    quantity: i32,
    price_per_unit: f64,
    is_leased: bool,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let item_id = uuid::Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();
    let is_leased_int = if is_leased { 1 } else { 0 };

    conn.execute(
        "INSERT INTO job_packaging_items (id, workspace_id, job_ticket_id, item_name, quantity, price_per_unit, is_leased, returned_quantity, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?8)",
        crate::params![
            item_id,
            job_ws,
            job_ticket_id,
            item_name,
            quantity,
            price_per_unit,
            is_leased_int,
            now_ms
        ],
    ).await?;

    // Check if move quote exists
    let quote_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM move_quotes WHERE job_ticket_id = ?1",
            crate::params![&job_ticket_id],
            |r| r.get::<i64>(0),
        )
        .await
        .unwrap_or(0)
        > 0;

    if quote_exists {
        drop(conn);
        calculate_and_save_move_quote(requester_user_id, job_ticket_id).await?;
    } else {
        notify_observers();
    }

    Ok(())
}

#[uniffi::export]
pub async fn remove_job_packaging_item(
    requester_user_id: String,
    item_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws, job_ticket_id): (String, String) = conn
        .query_row(
            "SELECT workspace_id, job_ticket_id FROM job_packaging_items WHERE id = ?1",
            crate::params![&item_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Packaging item not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    conn.execute(
        "DELETE FROM job_packaging_items WHERE id = ?1",
        crate::params![item_id],
    )
    .await?;

    // Recalculate quote if exists
    let quote_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM move_quotes WHERE job_ticket_id = ?1",
            crate::params![&job_ticket_id],
            |r| r.get::<i64>(0),
        )
        .await
        .unwrap_or(0)
        > 0;

    if quote_exists {
        drop(conn);
        calculate_and_save_move_quote(requester_user_id, job_ticket_id).await?;
    } else {
        notify_observers();
    }

    Ok(())
}

#[uniffi::export]
pub async fn update_job_packaging_item_returned(
    requester_user_id: String,
    item_id: String,
    returned_quantity: i32,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws, job_ticket_id): (String, String) = conn
        .query_row(
            "SELECT workspace_id, job_ticket_id FROM job_packaging_items WHERE id = ?1",
            crate::params![&item_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Packaging item not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE job_packaging_items SET returned_quantity = ?1, updated_at = ?2 WHERE id = ?3",
        crate::params![returned_quantity, now_ms, item_id],
    )
    .await?;

    let quote_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM move_quotes WHERE job_ticket_id = ?1",
            crate::params![&job_ticket_id],
            |r| r.get::<i64>(0),
        )
        .await
        .unwrap_or(0)
        > 0;

    if quote_exists {
        drop(conn);
        calculate_and_save_move_quote(requester_user_id, job_ticket_id).await?;
    } else {
        notify_observers();
    }

    Ok(())
}

#[uniffi::export]
pub async fn get_job_packaging_items(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<Vec<JobPackagingItem>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, job_ticket_id, item_name, quantity, price_per_unit, is_leased, returned_quantity, created_at, updated_at, sync_status FROM job_packaging_items WHERE job_ticket_id = ?1 ORDER BY created_at ASC",
    ).await?;

    let mut rows = stmt.query(crate::params![&job_ticket_id]).await?;
    let mut items = Vec::new();
    while let Some(row) = rows.next().await? {
        items.push(JobPackagingItem {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            job_ticket_id: row.get(2)?,
            item_name: row.get(3)?,
            quantity: row.get(4)?,
            price_per_unit: row.get(5)?,
            is_leased: row.get::<i32>(6)? != 0,
            returned_quantity: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            sync_status: row
                .get::<Option<String>>(10)?
                .unwrap_or_else(|| "pending".to_string()),
        });
    }

    Ok(items)
}

#[uniffi::export]
pub async fn update_move_quote_price_adjustments(
    requester_user_id: String,
    job_ticket_id: String,
    manual_price_override: Option<f64>,
    price_discount: Option<f64>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    // Verify workspace scoping
    let (job_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM job_tickets WHERE id = ?1",
            crate::params![&job_ticket_id],
            |r| Ok((r.get(0)?,)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Job not found".to_string()))?;

    if auth.workspace_id != job_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    // Check if quote exists
    let mut quote_stmt = conn
        .prepare("SELECT id FROM move_quotes WHERE job_ticket_id = ?1 LIMIT 1")
        .await?;
    let mut q_rows = quote_stmt.query(crate::params![&job_ticket_id]).await?;
    let has_quote = q_rows.next().await?.is_some();
    drop(q_rows);
    drop(quote_stmt);

    if has_quote {
        conn.execute(
            "UPDATE move_quotes SET manual_price_override = ?1, price_discount = ?2, sync_status = 'pending', updated_at = ?3 WHERE job_ticket_id = ?4",
            crate::params![
                manual_price_override,
                price_discount,
                chrono::Utc::now().timestamp_millis(),
                job_ticket_id
            ],
        ).await?;
    } else {
        let quote_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at, sync_status, manual_price_override, price_discount) VALUES (?1, ?2, ?3, 0.0, 0.0, 0.0, 0.0, 0.0, 'sent', ?4, 'pending', ?5, ?6)",
            crate::params![
                quote_id,
                job_ws,
                job_ticket_id,
                chrono::Utc::now().timestamp_millis(),
                manual_price_override,
                price_discount
            ],
        ).await?;
    }

    // Recalculate quote to save new total price
    calculate_and_save_move_quote(requester_user_id, job_ticket_id).await?;

    Ok(())
}

#[uniffi::export]
pub async fn accept_move_quote_with_deposit(
    requester_user_id: String,
    quote_id: String,
    payment_method: String,
) -> Result<crate::models::QuoteDepositApprovalResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (ws_id, job_id, total_price, current_status): (String, String, f64, String) = conn
        .query_row(
            "SELECT workspace_id, job_ticket_id, total_price, status FROM move_quotes WHERE id = ?1",
            crate::params![&quote_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Quote not found".to_string()))?;

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if current_status == "accepted" {
        return Ok(crate::models::QuoteDepositApprovalResult {
            success: true,
            quote_id,
            deposit_amount: 0.0,
            remaining_balance: total_price,
            payment_session_url: None,
            quote_status: "accepted".to_string(),
            message: "Quote was already accepted.".to_string(),
        });
    }

    let settings_str: String = conn
        .query_row(
            "SELECT settings FROM workspaces WHERE id = ?1",
            crate::params![&ws_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let settings_json: serde_json::Value = serde_json::from_str(&settings_str).unwrap_or_default();

    let target_region = settings_json
        .get("target_region")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            settings_json
                .get("company_country")
                .and_then(|v| v.as_str())
                .unwrap_or("SE")
        });

    let currency = crate::services::workspaces::get_setting_str(
        &settings_json,
        "currency",
        match target_region {
            "US" => "USD",
            "DE" | "FR" | "ES" | "IT" | "NL" | "FI" => "EUR",
            "NO" => "NOK",
            "DK" => "DKK",
            _ => "SEK",
        },
    );

    let deposit_pct = settings_json
        .get("moving_deposit_percent")
        .and_then(|v| v.as_f64())
        .unwrap_or(20.0);
    let deposit_amount = (total_price * (deposit_pct / 100.0)).round().max(500.0);
    let remaining_balance = (total_price - deposit_amount).max(0.0);
    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "UPDATE move_quotes SET status = 'pending_deposit', updated_at = ?1 WHERE id = ?2 AND workspace_id = ?3",
        crate::params![now_ms, &quote_id, &ws_id],
    ).await?;

    conn.execute(
        "UPDATE job_tickets SET status = 'deposit_pending', updated_at = ?1 WHERE id = ?2 AND workspace_id = ?3",
        crate::params![now_ms, &job_id, &ws_id],
    ).await?;

    notify_observers();

    let portal_base_url = super::billing::helpers::get_config_val(
        "payment_portal_url",
        "PAYMENT_PORTAL_URL",
        &settings_json,
    )
    .await
    .or_else(|| {
        let conn_ref = &conn;
        let _ = conn_ref;
        settings_json
            .get("custom_domain")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    })
    .or_else(|| {
        settings_json
            .get("api_base_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    })
    .unwrap_or_else(|| "https://pay.yntra.se".to_string());

    let portal_base_url = portal_base_url.trim_end_matches('/');
    let payment_url = format!(
        "{}/deposit/{}?method={}",
        portal_base_url,
        quote_id,
        payment_method.to_lowercase()
    );

    Ok(crate::models::QuoteDepositApprovalResult {
        success: true,
        quote_id,
        deposit_amount,
        remaining_balance,
        payment_session_url: Some(payment_url),
        quote_status: "pending_deposit".to_string(),
        message: format!(
            "Deposit payment of {:.2} {} initiated via {}. Pending payment completion to confirm booking.",
            deposit_amount, currency, payment_method
        ),
    })
}

#[uniffi::export]
pub async fn confirm_quote_deposit_payment(
    requester_user_id: String,
    quote_id: String,
    _payment_reference: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (ws_id, job_id, current_status): (String, String, String) = conn
        .query_row(
            "SELECT workspace_id, job_ticket_id, status FROM move_quotes WHERE id = ?1",
            crate::params![&quote_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Quote not found".to_string()))?;

    if auth.workspace_id != ws_id {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    if current_status != "pending_deposit" && current_status != "accepted" {
        return Err(YntraError::ValidationError(format!(
            "Cannot confirm deposit payment: quote status is '{}' (deposit payment link invalidated due to quote revisions). Please re-accept updated quote to generate a new deposit link.",
            current_status
        )));
    }

    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "UPDATE move_quotes SET status = 'accepted', accepted_at = ?1, updated_at = ?1 WHERE id = ?2 AND workspace_id = ?3",
        crate::params![now_ms, &quote_id, &ws_id],
    ).await?;

    conn.execute(
        "UPDATE job_tickets SET status = 'assigned', updated_at = ?1 WHERE id = ?2 AND workspace_id = ?3",
        crate::params![now_ms, &job_id, &ws_id],
    ).await?;

    notify_observers();
    Ok(())
}

#[cfg(test)]
mod furniture_inventory_tests {
    use super::*;

    #[tokio::test]
    async fn test_furniture_catalog_and_inventory_summary() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // 1. Verify furniture catalog preset items
        let catalog = get_furniture_catalog();
        assert!(!catalog.is_empty());
        assert!(catalog.iter().any(|p| p.id == "sofa-3p"
            && p.default_volume_m3 == 1.8
            && p.default_weight_kg == 75.0));
        assert!(
            catalog
                .iter()
                .any(|p| p.id == "bed-king" && p.category == "Bedroom")
        );

        // Setup test workspace, user, and job ticket
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-inv-sum-test', 'Inv Sum WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-inv-staff', 'ws-inv-sum-test', 'staff@inv.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at) VALUES ('job-inv-sum-1', 'ws-inv-sum-test', 'Move Inventory Test', 'Test Desc', 'Main St 1', 'medium', 'pending', '2026-07-21', '[]', '2026-07-21', 0)", ()).await.unwrap();

        // 2. Add inventory items with room_name, weight, preset_id
        create_move_inventory_item_with_details(
            "u-inv-staff".to_string(),
            "job-inv-sum-1".to_string(),
            "Living Room".to_string(),
            "3-Seater Sofa".to_string(),
            1,
            1.8,
            Some("Blanket wrap".to_string()),
            Some("Living Room".to_string()),
            Some(75.0),
            Some("sofa-3p".to_string()),
        )
        .await
        .unwrap();

        create_move_inventory_item_with_details(
            "u-inv-staff".to_string(),
            "job-inv-sum-1".to_string(),
            "Bedroom".to_string(),
            "King Bed & Mattress".to_string(),
            1,
            2.4,
            None,
            Some("Master Bedroom".to_string()),
            Some(90.0),
            Some("bed-king".to_string()),
        )
        .await
        .unwrap();

        create_move_inventory_item_with_details(
            "u-inv-staff".to_string(),
            "job-inv-sum-1".to_string(),
            "Boxes".to_string(),
            "Standard Moving Box".to_string(),
            10,
            0.1,
            None,
            Some("Living Room".to_string()),
            Some(15.0),
            Some("box-std".to_string()),
        )
        .await
        .unwrap();

        // 3. Verify get_move_inventory retrieves room_name and estimated_weight_kg
        let items = get_move_inventory("u-inv-staff".to_string(), "job-inv-sum-1".to_string())
            .await
            .unwrap();
        assert_eq!(items.len(), 3);
        let sofa = items
            .iter()
            .find(|i| i.item_name == "3-Seater Sofa")
            .unwrap();
        assert_eq!(sofa.room_name, Some("Living Room".to_string()));
        assert_eq!(sofa.estimated_weight_kg, 75.0);
        assert_eq!(sofa.preset_id, Some("sofa-3p".to_string()));

        // 4. Verify get_move_inventory_summary calculations
        let summary =
            get_move_inventory_summary("u-inv-staff".to_string(), "job-inv-sum-1".to_string())
                .await
                .unwrap();
        // Total vol: 1.8 + 2.4 + (10 * 0.1) = 5.2 m3
        assert_eq!(summary.total_volume_m3, 5.2);
        // Total weight: 75 + 90 + (10 * 15) = 315 kg
        assert_eq!(summary.total_weight_kg, 315.0);
        // Total item count: 1 + 1 + 10 = 12
        assert_eq!(summary.total_item_count, 12);
        // Recommended truck (5.2 * 1.2 = 6.24 -> 6.2 m3)
        assert_eq!(summary.recommended_truck_m3, 6.2);
        // Recommended crew (volume <= 15 and weight <= 350 -> 2 movers)
        assert_eq!(summary.recommended_crew_size, 2);

        // Cleanup
        conn.execute(
            "DELETE FROM move_inventory WHERE job_ticket_id = 'job-inv-sum-1'",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "DELETE FROM move_quotes WHERE job_ticket_id = 'job-inv-sum-1'",
            (),
        )
        .await
        .unwrap();
        conn.execute("DELETE FROM job_tickets WHERE id = 'job-inv-sum-1'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-inv-staff'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-inv-sum-test'", ())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_barcode_scanning_workflow() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Setup test workspace, user, and job ticket
        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-bc-test', 'Barcode WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-bc-staff', 'ws-bc-test', 'staff@bc.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at) VALUES ('job-bc-1', 'ws-bc-test', 'Move Barcode Test', 'Test Desc', 'Main St 1', 'medium', 'pending', '2026-07-21', '[]', '2026-07-21', 0)", ()).await.unwrap();

        // 1. Create inventory item (auto-generates barcode_tag starting with YNT-JOB-BC-)
        create_move_inventory_item_with_details(
            "u-bc-staff".to_string(),
            "job-bc-1".to_string(),
            "Möbler".to_string(),
            "Designer Sofa".to_string(),
            1,
            2.0,
            None,
            Some("Living Room".to_string()),
            Some(80.0),
            None,
        )
        .await
        .unwrap();

        let items = get_move_inventory("u-bc-staff".to_string(), "job-bc-1".to_string())
            .await
            .unwrap();
        assert_eq!(items.len(), 1);
        let barcode = items[0].barcode_tag.clone().unwrap();
        assert!(barcode.starts_with("YNT-"));
        assert_eq!(items[0].scan_status, "unscanned");

        // 2. Scan item -> transition to 'packed'
        let item_packed = scan_inventory_item_by_barcode(
            "u-bc-staff".to_string(),
            "job-bc-1".to_string(),
            barcode.clone(),
            "packed".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(item_packed.scan_status, "packed");
        assert!(item_packed.last_scanned_at.is_some());
        assert_eq!(item_packed.last_scanned_by, Some("u-bc-staff".to_string()));

        // 3. Scan item -> transition to 'loaded'
        let item_loaded = scan_inventory_item_by_barcode(
            "u-bc-staff".to_string(),
            "job-bc-1".to_string(),
            barcode.clone(),
            "loaded".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(item_loaded.scan_status, "loaded");

        // 4. Verify scan manifest report
        let manifest =
            get_inventory_scan_manifest("u-bc-staff".to_string(), "job-bc-1".to_string())
                .await
                .unwrap();
        assert_eq!(manifest.total_items, 1);
        assert_eq!(manifest.loaded_count, 1);
        assert_eq!(manifest.unloaded_count, 0);

        // Cleanup
        conn.execute(
            "DELETE FROM move_inventory WHERE job_ticket_id = 'job-bc-1'",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "DELETE FROM move_quotes WHERE job_ticket_id = 'job-bc-1'",
            (),
        )
        .await
        .unwrap();
        conn.execute("DELETE FROM job_tickets WHERE id = 'job-bc-1'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-bc-staff'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-bc-test'", ())
            .await
            .unwrap();
    }

    #[test]
    fn test_unit_conversions_and_custom_dimensions() {
        // Test unit conversions
        let cu_ft = convert_m3_to_cu_ft(1.0);
        assert_eq!(cu_ft, 35.31);
        let m3 = convert_cu_ft_to_m3(35.3146667);
        assert_eq!(m3, 1.0);

        let lbs = convert_kg_to_lbs(100.0);
        assert_eq!(lbs, 220.5);
        let kg = convert_lbs_to_kg(220.462262);
        assert_eq!(kg, 100.0);

        // Test custom dimension calculations (cm)
        let vol_m3 = calculate_volume_from_dimensions_cm(200.0, 100.0, 90.0);
        assert_eq!(vol_m3, 1.8);

        // Test custom dimension calculations (inches)
        let vol_cu_ft = calculate_volume_from_dimensions_inches(48.0, 24.0, 36.0);
        assert_eq!(vol_cu_ft, 24.0);
    }

    #[tokio::test]
    async fn test_locale_role_matching_and_permissions() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-role-test', 'Role WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-driver-en', 'ws-role-test', 'driver@en.io', 'role-move-driver')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-driver-sv', 'ws-role-test', 'driver@sv.io', 'role-flytt-chauffor')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at) VALUES ('job-role-1', 'ws-role-test', 'Role Ticket', 'Desc', 'Main St 1', 'medium', 'pending', '2026-07-22', '[]', 0, 0)", ()).await.unwrap();

        // Verify driver roles from both English and Swedish templates are denied access to financial quotes
        let quote_res_en =
            get_move_quote("u-driver-en".to_string(), "job-role-1".to_string()).await;
        assert!(quote_res_en.is_err());
        assert!(
            quote_res_en
                .unwrap_err()
                .to_string()
                .contains("Access denied")
        );

        let quote_res_sv =
            get_move_quote("u-driver-sv".to_string(), "job-role-1".to_string()).await;
        assert!(quote_res_sv.is_err());
        assert!(
            quote_res_sv
                .unwrap_err()
                .to_string()
                .contains("Access denied")
        );

        // Add staff users provisioned from templates
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-coord-en', 'ws-role-test', 'coord@en.io', 'role-move-coordinator')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-admin-sv', 'ws-role-test', 'admin@sv.io', 'role-flytt-koordinator')", ()).await.unwrap();

        // Create accepted quote for job
        conn.execute("INSERT OR REPLACE INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at, sync_status) VALUES ('q-role-1', 'ws-role-test', 'job-role-1', 1000.0, 0.0, 0.0, 0.0, 1000.0, 'accepted', 0, 'synced')", ()).await.unwrap();

        // Verify template staff can modify inventory even after quote is accepted (does not fail with ValidationError)
        let create_res = create_move_inventory_item(
            "u-coord-en".to_string(),
            "job-role-1".to_string(),
            "Möbler".to_string(),
            "Table".to_string(),
            1,
            0.5,
            None,
        )
        .await;
        assert!(create_res.is_ok());

        let inv_items = get_move_inventory("u-coord-en".to_string(), "job-role-1".to_string())
            .await
            .unwrap();
        assert_eq!(inv_items.len(), 1);

        let delete_res =
            delete_move_inventory_item("u-admin-sv".to_string(), inv_items[0].id.clone()).await;
        assert!(delete_res.is_ok());

        // Cleanup
        conn.execute("DELETE FROM move_quotes WHERE id = 'q-role-1'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM job_tickets WHERE id = 'job-role-1'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-role-test'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-role-test'", ())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_job_specific_route_access_parameters_isolation() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        // Workspace settings with global crane hoist enabled
        let settings = serde_json::json!({
            "moving_base_rate_per_m3": 100.0,
            "requires_crane_hoist": true,
            "surcharge_crane_hoist": 3500.0
        })
        .to_string();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-iso-test', 'Iso WS', '[\"moving_company\"]', ?1)", crate::params![settings]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-iso-staff', 'ws-iso-test', 'staff@iso.io', 'admin')", ()).await.unwrap();

        // Job 1: Has job-specific route metadata explicitly disabling crane hoist requirement
        let route_json_no_crane = serde_json::json!({
            "requires_crane_hoist": false
        })
        .to_string();
        conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at, route_stops_json) VALUES ('job-iso-1', 'ws-iso-test', 'Job No Crane', 'Desc', 'Main St 1', 'medium', 'pending', '2026-07-22', '[]', 0, 0, ?1)", crate::params![route_json_no_crane]).await.unwrap();

        create_move_inventory_item(
            "u-iso-staff".to_string(),
            "job-iso-1".to_string(),
            "Möbler".to_string(),
            "Desk".to_string(),
            1,
            1.0,
            None,
        )
        .await
        .unwrap();
        calculate_and_save_move_quote("u-iso-staff".to_string(), "job-iso-1".to_string())
            .await
            .unwrap();

        let quote1 = get_move_quote("u-iso-staff".to_string(), "job-iso-1".to_string())
            .await
            .unwrap()
            .unwrap();
        // Crane hoist surcharge (3500.0) should NOT be added to stairs_surcharge because job route metadata explicitly set requires_crane_hoist = false
        assert_eq!(quote1.stairs_surcharge, 0.0);

        // Cleanup
        conn.execute(
            "DELETE FROM move_inventory WHERE job_ticket_id = 'job-iso-1'",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "DELETE FROM move_quotes WHERE job_ticket_id = 'job-iso-1'",
            (),
        )
        .await
        .unwrap();
        conn.execute("DELETE FROM job_tickets WHERE id = 'job-iso-1'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-iso-test'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-iso-test'", ())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_revised_quote_reacceptance_and_invoice_regeneration() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let settings = serde_json::json!({
            "moving_base_rate_per_m3": 500.0,
            "moving_distance_fee_flat": 0.0,
            "moving_packing_supplies_fee_per_m3": 0.0
        })
        .to_string();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-revwf-test', 'Rev WF WS', '[\"moving_company\"]', ?1)", crate::params![settings]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-revwf-staff', 'ws-revwf-test', 'staff@revwf.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at) VALUES ('job-revwf-1', 'ws-revwf-test', 'Rev WF Job', 'Desc', 'Main St 1', 'medium', 'pending', '2026-07-22', '[]', 0, 0)", ()).await.unwrap();

        // 1. Add item & calculate initial quote
        create_move_inventory_item(
            "u-revwf-staff".to_string(),
            "job-revwf-1".to_string(),
            "Möbler".to_string(),
            "Chair".to_string(),
            1,
            0.5,
            None,
        )
        .await
        .unwrap();
        calculate_and_save_move_quote("u-revwf-staff".to_string(), "job-revwf-1".to_string())
            .await
            .unwrap();

        let quote1 = get_move_quote("u-revwf-staff".to_string(), "job-revwf-1".to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(quote1.total_price, 250.0);

        // 2. Accept quote & generate invoice
        accept_move_quote("u-revwf-staff".to_string(), quote1.id.clone())
            .await
            .unwrap();
        let inv1 = crate::services::jobs::billing::generate_move_invoice(
            "u-revwf-staff".to_string(),
            quote1.id.clone(),
            false,
        )
        .await
        .unwrap();
        assert_eq!(inv1.subtotal, 250.0);

        // 3. Add inventory item -> quote recalculates to revised
        create_move_inventory_item(
            "u-revwf-staff".to_string(),
            "job-revwf-1".to_string(),
            "Möbler".to_string(),
            "Table".to_string(),
            1,
            1.0,
            None,
        )
        .await
        .unwrap();
        calculate_and_save_move_quote("u-revwf-staff".to_string(), "job-revwf-1".to_string())
            .await
            .unwrap();

        let quote2 = get_move_quote("u-revwf-staff".to_string(), "job-revwf-1".to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(quote2.status, "revised");
        assert_eq!(quote2.total_price, 750.0);

        // 4. Re-accept quote -> quote transitions back to 'accepted' and updates invoice
        accept_move_quote("u-revwf-staff".to_string(), quote2.id.clone())
            .await
            .unwrap();

        let quote3 = get_move_quote("u-revwf-staff".to_string(), "job-revwf-1".to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(quote3.status, "accepted");

        let inv2 = crate::services::jobs::billing::get_move_invoice(
            "u-revwf-staff".to_string(),
            quote3.id.clone(),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(inv2.subtotal, 750.0);

        // Cleanup
        conn.execute(
            "DELETE FROM move_invoices WHERE quote_id = ?1",
            crate::params![quote3.id],
        )
        .await
        .unwrap();
        conn.execute(
            "DELETE FROM move_quote_revisions WHERE quote_id = ?1",
            crate::params![quote3.id],
        )
        .await
        .unwrap();
        conn.execute(
            "DELETE FROM move_inventory WHERE job_ticket_id = 'job-revwf-1'",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "DELETE FROM move_quotes WHERE job_ticket_id = 'job-revwf-1'",
            (),
        )
        .await
        .unwrap();
        conn.execute("DELETE FROM job_tickets WHERE id = 'job-revwf-1'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-revwf-test'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-revwf-test'", ())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_manual_override_with_itemized_additions() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let settings = serde_json::json!({
            "moving_base_rate_per_m3": 500.0,
            "moving_distance_fee_flat": 0.0,
            "moving_packing_supplies_fee_per_m3": 0.0
        })
        .to_string();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-ovr-test', 'Ovr WS', '[\"moving_company\"]', ?1)", crate::params![settings]).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-ovr-staff', 'ws-ovr-test', 'staff@ovr.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at) VALUES ('job-ovr-1', 'ws-ovr-test', 'Ovr Job', 'Desc', 'Main St 1', 'medium', 'pending', '2026-07-22', '[]', 0, 0)", ()).await.unwrap();

        // 1. Initial quote creation
        create_move_inventory_item(
            "u-ovr-staff".to_string(),
            "job-ovr-1".to_string(),
            "Möbler".to_string(),
            "Desk".to_string(),
            1,
            1.0,
            None,
        )
        .await
        .unwrap();
        calculate_and_save_move_quote("u-ovr-staff".to_string(), "job-ovr-1".to_string())
            .await
            .unwrap();

        // 2. Set manual base labor override to 1200.0
        update_move_quote_price_adjustments(
            "u-ovr-staff".to_string(),
            "job-ovr-1".to_string(),
            Some(1200.0),
            None,
        )
        .await
        .unwrap();

        let quote1 = get_move_quote("u-ovr-staff".to_string(), "job-ovr-1".to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(quote1.base_price, 1200.0);
        assert_eq!(quote1.total_price, 1200.0);

        // 3. Add packaging supplies item ($150)
        add_job_packaging_item(
            "u-ovr-staff".to_string(),
            "job-ovr-1".to_string(),
            "Boxes".to_string(),
            5,
            30.0,
            false,
        )
        .await
        .unwrap();

        // 4. Recalculate quote -> base price stays 1200.0, packing supplies fee is 150.0, total_price is 1350.0
        calculate_and_save_move_quote("u-ovr-staff".to_string(), "job-ovr-1".to_string())
            .await
            .unwrap();

        let quote2 = get_move_quote("u-ovr-staff".to_string(), "job-ovr-1".to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(quote2.base_price, 1200.0);
        assert_eq!(quote2.packing_supplies_fee, 150.0);
        assert_eq!(quote2.total_price, 1350.0);

        // Cleanup
        conn.execute(
            "DELETE FROM job_packaging_items WHERE job_ticket_id = 'job-ovr-1'",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "DELETE FROM move_inventory WHERE job_ticket_id = 'job-ovr-1'",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "DELETE FROM move_quotes WHERE job_ticket_id = 'job-ovr-1'",
            (),
        )
        .await
        .unwrap();
        conn.execute("DELETE FROM job_tickets WHERE id = 'job-ovr-1'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-ovr-test'", ())
            .await
            .unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-ovr-test'", ())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_customer_personal_number_gate_and_rut_checkout() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-pnum-test', 'Pnum WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role, metadata) VALUES ('u-pnum-client', 'ws-pnum-test', 'client@pnum.io', 'client', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at) VALUES ('job-pnum-1', 'ws-pnum-test', 'Pnum Job', 'Desc', 'Main St 1', 'medium', 'pending', '2026-07-22', '[]', 0, 0)", ()).await.unwrap();

        create_move_inventory_item(
            "u-pnum-client".to_string(),
            "job-pnum-1".to_string(),
            "Möbler".to_string(),
            "Table".to_string(),
            1,
            1.0,
            None,
        )
        .await
        .unwrap();
        calculate_and_save_move_quote("u-pnum-client".to_string(), "job-pnum-1".to_string())
            .await
            .unwrap();
        let quote = get_move_quote("u-pnum-client".to_string(), "job-pnum-1".to_string())
            .await
            .unwrap()
            .unwrap();

        // 1. Attempting RUT checkout without providing personal number should return ValidationError
        let fail_res =
            accept_move_quote_with_rut("u-pnum-client".to_string(), quote.id.clone(), true, None)
                .await;
        assert!(fail_res.is_err());
        assert!(
            fail_res
                .unwrap_err()
                .to_string()
                .contains("Missing valid Swedish personal number")
        );

        // Verify quote status was NOT mutated to 'accepted' when validation failed
        let quote_after_fail =
            get_move_quote("u-pnum-client".to_string(), "job-pnum-1".to_string())
                .await
                .unwrap()
                .unwrap();
        assert_ne!(quote_after_fail.status, "accepted");

        // 2. Accepting quote with valid personal number provisions personal_number gate and generates RUT invoice
        let ok_res = accept_move_quote_with_rut(
            "u-pnum-client".to_string(),
            quote.id.clone(),
            true,
            Some("198112189876".to_string()),
        )
        .await;
        assert!(ok_res.is_ok());

        let inv = crate::services::jobs::billing::get_move_invoice(
            "u-pnum-client".to_string(),
            quote.id.clone(),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(inv.rut_deduction > 0.0);

        // Cleanup
        conn.execute(
            "DELETE FROM move_invoices WHERE quote_id = ?1",
            crate::params![quote.id],
        )
        .await
        .ok();
        conn.execute(
            "DELETE FROM move_inventory WHERE job_ticket_id = 'job-pnum-1'",
            (),
        )
        .await
        .ok();
        conn.execute(
            "DELETE FROM move_quotes WHERE job_ticket_id = 'job-pnum-1'",
            (),
        )
        .await
        .ok();
        conn.execute("DELETE FROM job_tickets WHERE id = 'job-pnum-1'", ())
            .await
            .ok();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-pnum-test'", ())
            .await
            .ok();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-pnum-test'", ())
            .await
            .ok();
    }

    #[tokio::test]
    async fn test_self_service_inventory_edits_on_accepted_quote() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-selfed-test', 'SelfEd WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-selfed-client', 'ws-selfed-test', 'client@selfed.io', 'client')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at) VALUES ('job-selfed-1', 'ws-selfed-test', 'SelfEd Job', 'Desc', 'Main St 1', 'medium', 'pending', '2026-07-22', '[]', 0, 0)", ()).await.unwrap();

        // 1. Initial item & quote accepted
        create_move_inventory_item(
            "u-selfed-client".to_string(),
            "job-selfed-1".to_string(),
            "Möbler".to_string(),
            "Lamp".to_string(),
            1,
            0.2,
            None,
        )
        .await
        .unwrap();
        calculate_and_save_move_quote("u-selfed-client".to_string(), "job-selfed-1".to_string())
            .await
            .unwrap();
        let quote1 = get_move_quote("u-selfed-client".to_string(), "job-selfed-1".to_string())
            .await
            .unwrap()
            .unwrap();

        accept_move_quote("u-selfed-client".to_string(), quote1.id.clone())
            .await
            .unwrap();

        // 2. Self-service customer item addition on accepted quote succeeds and transitions quote to revised
        let add_res = create_move_inventory_item(
            "u-selfed-client".to_string(),
            "job-selfed-1".to_string(),
            "Möbler".to_string(),
            "Chair".to_string(),
            1,
            0.5,
            None,
        )
        .await;
        assert!(add_res.is_ok());

        let quote2 = get_move_quote("u-selfed-client".to_string(), "job-selfed-1".to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(quote2.status, "revised");

        // 3. Self-service customer item deletion on revised quote succeeds
        let items = get_move_inventory("u-selfed-client".to_string(), "job-selfed-1".to_string())
            .await
            .unwrap();
        let del_res =
            delete_move_inventory_item("u-selfed-client".to_string(), items[0].id.clone()).await;
        assert!(del_res.is_ok());

        // 4. Completed job blocks non-staff inventory edits
        conn.execute(
            "UPDATE job_tickets SET status = 'completed' WHERE id = 'job-selfed-1'",
            (),
        )
        .await
        .unwrap();
        let fail_add = create_move_inventory_item(
            "u-selfed-client".to_string(),
            "job-selfed-1".to_string(),
            "Möbler".to_string(),
            "Box".to_string(),
            1,
            0.1,
            None,
        )
        .await;
        assert!(fail_add.is_err());
        assert!(
            fail_add
                .unwrap_err()
                .to_string()
                .contains("Cannot modify inventory after job ticket has been completed")
        );

        // Cleanup
        conn.execute(
            "DELETE FROM move_inventory WHERE job_ticket_id = 'job-selfed-1'",
            (),
        )
        .await
        .ok();
        conn.execute(
            "DELETE FROM move_quotes WHERE job_ticket_id = 'job-selfed-1'",
            (),
        )
        .await
        .ok();
        conn.execute("DELETE FROM job_tickets WHERE id = 'job-selfed-1'", ())
            .await
            .ok();
        conn.execute(
            "DELETE FROM users WHERE workspace_id = 'ws-selfed-test'",
            (),
        )
        .await
        .ok();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-selfed-test'", ())
            .await
            .ok();
    }

    #[tokio::test]
    async fn test_barcode_tag_exact_match_priority_and_entropy() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-bc-test', 'BC WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-bc-staff', 'ws-bc-test', 'staff@bc.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at) VALUES ('job-barcode-test-123456', 'ws-bc-test', 'BC Job', 'Desc', 'Main St 1', 'medium', 'pending', '2026-07-22', '[]', 0, 0)", ()).await.unwrap();

        // 1. Create two items with similar prefixes ("BOX" and "BOX1")
        conn.execute(
            "INSERT INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, scan_status, barcode_tag) VALUES ('inv-bc-box', 'ws-bc-test', 'job-barcode-test-123456', 'Boxes', 'Box Short', 1, 0.1, 'unscanned', 'YNT-JOBBARCODET-BOX')",
            (),
        ).await.unwrap();

        conn.execute(
            "INSERT INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, scan_status, barcode_tag) VALUES ('inv-bc-box1', 'ws-bc-test', 'job-barcode-test-123456', 'Boxes', 'Box Specific 1', 1, 0.1, 'unscanned', 'YNT-JOBBARCODET-BOX1')",
            (),
        ).await.unwrap();

        // 2. Scanning 'YNT-JOBBARCODET-BOX1' must match 'inv-bc-box1' exactly, NOT 'inv-bc-box' via fuzzy prefix
        let scanned = scan_inventory_item_by_barcode(
            "u-bc-staff".to_string(),
            "job-barcode-test-123456".to_string(),
            "YNT-JOBBARCODET-BOX1".to_string(),
            "packed".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(scanned.id, "inv-bc-box1");
        assert_eq!(scanned.scan_status, "packed");

        // 3. Verify barcode generation uses 12-char entropy
        create_move_inventory_item(
            "u-bc-staff".to_string(),
            "job-barcode-test-123456".to_string(),
            "Möbler".to_string(),
            "Sofa".to_string(),
            1,
            2.0,
            None,
        )
        .await
        .unwrap();
        let items = get_move_inventory(
            "u-bc-staff".to_string(),
            "job-barcode-test-123456".to_string(),
        )
        .await
        .unwrap();
        let sofa = items.into_iter().find(|i| i.item_name == "Sofa").unwrap();
        let bc_tag = sofa.barcode_tag.unwrap();
        assert!(bc_tag.starts_with("YNT-JOBBARCODETE-"));
        // Segment length after YNT- should be 12 chars + - + 12 chars
        let parts: Vec<&str> = bc_tag.split('-').collect();
        assert_eq!(parts[1].len(), 12);
        assert_eq!(parts[2].len(), 12);

        // Cleanup
        conn.execute(
            "DELETE FROM move_inventory WHERE job_ticket_id = 'job-barcode-test-123456'",
            (),
        )
        .await
        .ok();
        conn.execute(
            "DELETE FROM job_tickets WHERE id = 'job-barcode-test-123456'",
            (),
        )
        .await
        .ok();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-bc-test'", ())
            .await
            .ok();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-bc-test'", ())
            .await
            .ok();
    }

    #[tokio::test]
    async fn test_vehicle_payload_operational_tare_buffer_enforcement() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-payload-buf', 'Payload WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-pay-staff', 'ws-payload-buf', 'staff@pay.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO vehicles (id, workspace_id, name, license_plate, capacity_m3, max_payload_kg, status) VALUES ('v-pay-1000', 'ws-payload-buf', 'Van 1000kg', 'PAY-999', 30.0, 1000.0, 'active')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, assigned_vehicle_id, created_at, updated_at) VALUES ('job-pay-buf-1', 'ws-payload-buf', 'Pay Job', 'Desc', 'Main St 1', 'medium', 'pending', '2026-07-22', '[]', 'v-pay-1000', 0, 0)", ()).await.unwrap();

        // Cargo weight = 600 kg.
        // Operational tare buffer = 2 crew @ 85kg (170kg) + 300kg equipment/fuel = 470 kg.
        // Total operational payload = 600 + 470 = 1070 kg > 1000 kg vehicle limit!
        create_move_inventory_item_with_details(
            "u-pay-staff".to_string(),
            "job-pay-buf-1".to_string(),
            "Möbler".to_string(),
            "Heavy Machinery".to_string(),
            1,
            5.0,
            None,
            None,
            Some(600.0),
            None,
        )
        .await
        .unwrap();

        let summary =
            get_move_inventory_summary("u-pay-staff".to_string(), "job-pay-buf-1".to_string())
                .await
                .unwrap();
        assert!(summary.truck_capacity_exceeded);
        assert!(
            summary
                .truck_capacity_warning
                .unwrap()
                .contains("Total operational payload")
        );

        // Cleanup
        conn.execute(
            "DELETE FROM move_inventory WHERE job_ticket_id = 'job-pay-buf-1'",
            (),
        )
        .await
        .ok();
        conn.execute("DELETE FROM job_tickets WHERE id = 'job-pay-buf-1'", ())
            .await
            .ok();
        conn.execute("DELETE FROM vehicles WHERE id = 'v-pay-1000'", ())
            .await
            .ok();
        conn.execute(
            "DELETE FROM users WHERE workspace_id = 'ws-payload-buf'",
            (),
        )
        .await
        .ok();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-payload-buf'", ())
            .await
            .ok();
    }

    #[tokio::test]
    async fn test_custom_item_bulk_density_weight_fallback() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-custom-wt', 'Custom Weight WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-cust-wt-staff', 'ws-custom-wt', 'staff@custwt.io', 'admin')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at) VALUES ('job-cust-wt-1', 'ws-custom-wt', 'Cust Wt Job', 'Desc', 'Main St 1', 'medium', 'pending', '2026-07-22', '[]', 0, 0)", ()).await.unwrap();

        // Custom item created without explicit weight (2.0 m3 volume)
        create_move_inventory_item_with_details(
            "u-cust-wt-staff".to_string(),
            "job-cust-wt-1".to_string(),
            "Möbler".to_string(),
            "Custom Heavy Workstation".to_string(),
            1,
            2.0,
            None,
            None,
            None, // No explicit weight passed
            None, // No preset ID
        )
        .await
        .unwrap();

        let items = get_move_inventory("u-cust-wt-staff".to_string(), "job-cust-wt-1".to_string())
            .await
            .unwrap();
        assert_eq!(items.len(), 1);
        // Bulk density fallback = 2.0 m3 * 150 kg/m3 = 300.0 kg
        assert_eq!(items[0].estimated_weight_kg, 300.0);

        let summary =
            get_move_inventory_summary("u-cust-wt-staff".to_string(), "job-cust-wt-1".to_string())
                .await
                .unwrap();
        assert_eq!(summary.total_weight_kg, 300.0);

        // Cleanup
        conn.execute(
            "DELETE FROM move_inventory WHERE job_ticket_id = 'job-cust-wt-1'",
            (),
        )
        .await
        .ok();
        conn.execute("DELETE FROM job_tickets WHERE id = 'job-cust-wt-1'", ())
            .await
            .ok();
        conn.execute("DELETE FROM users WHERE workspace_id = 'ws-custom-wt'", ())
            .await
            .ok();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-custom-wt'", ())
            .await
            .ok();
    }

    #[tokio::test]
    async fn test_architectural_access_crew_sizing_recommendations() {
        let _lock = database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        conn.execute("INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-crew-access', 'Crew Access WS', '[\"moving_company\"]', '{}')", ()).await.unwrap();
        conn.execute("INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES ('u-crew-acc-staff', 'ws-crew-access', 'staff@crewacc.io', 'admin')", ()).await.unwrap();

        // 1. Base small move on ground floor -> 2 movers
        conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, origin_floor, origin_has_elevator, destination_floor, destination_has_elevator, created_at, updated_at) VALUES ('job-acc-1', 'ws-crew-access', 'Job 1', 'Desc', 'Main St 1', 'medium', 'pending', '2026-07-22', '[]', 0, 1, 0, 1, 0, 0)", ()).await.unwrap();
        create_move_inventory_item(
            "u-crew-acc-staff".to_string(),
            "job-acc-1".to_string(),
            "Möbler".to_string(),
            "Table".to_string(),
            1,
            0.5,
            None,
        )
        .await
        .unwrap();

        let summary1 =
            get_move_inventory_summary("u-crew-acc-staff".to_string(), "job-acc-1".to_string())
                .await
                .unwrap();
        assert_eq!(summary1.recommended_crew_size, 2);

        // 2. Small move on 4th floor WITHOUT elevator -> 2 base + 1 stair boost = 3 movers
        conn.execute("INSERT OR REPLACE INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, origin_floor, origin_has_elevator, destination_floor, destination_has_elevator, created_at, updated_at) VALUES ('job-acc-2', 'ws-crew-access', 'Job 2', 'Desc', 'Main St 2', 'medium', 'pending', '2026-07-22', '[]', 4, 0, 0, 1, 0, 0)", ()).await.unwrap();
        create_move_inventory_item(
            "u-crew-acc-staff".to_string(),
            "job-acc-2".to_string(),
            "Möbler".to_string(),
            "Table".to_string(),
            1,
            0.5,
            None,
        )
        .await
        .unwrap();

        let summary2 =
            get_move_inventory_summary("u-crew-acc-staff".to_string(), "job-acc-2".to_string())
                .await
                .unwrap();
        assert_eq!(summary2.recommended_crew_size, 3);

        // Cleanup
        conn.execute(
            "DELETE FROM move_inventory WHERE workspace_id = 'ws-crew-access'",
            (),
        )
        .await
        .ok();
        conn.execute(
            "DELETE FROM job_tickets WHERE workspace_id = 'ws-crew-access'",
            (),
        )
        .await
        .ok();
        conn.execute(
            "DELETE FROM users WHERE workspace_id = 'ws-crew-access'",
            (),
        )
        .await
        .ok();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-crew-access'", ())
            .await
            .ok();
    }
}
