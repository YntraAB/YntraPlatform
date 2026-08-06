use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;
use crate::{
    FurniturePreset, InventoryScanManifest, MoveInventoryItem, MoveInventorySummary,
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
pub fn get_furniture_catalog() -> Vec<FurniturePreset> {
    vec![
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
        FurniturePreset {
            id: "desk-office".to_string(),
            category: "Office".to_string(),
            name: "Work Desk & Ergonomic Chair".to_string(),
            default_volume_m3: 1.0,
            default_weight_kg: 40.0,
            default_handling_notes: None,
        },
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
    let id_part_str = if id_clean.len() >= 12 {
        &id_clean[..12]
    } else {
        &id_clean
    };
    let base_barcode = format!("YNT-{}-{}", job_part, id_part_str);

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
            base_barcode,
        ],
    ).await?;

    let _ = super::pricing::calculate_and_save_move_quote(requester_user_id, job_ticket_id).await;

    notify_observers();
    Ok(())
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
        .map_err(|_| YntraError::NotFoundError("Move inventory item not found".to_string()))?;

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
                    "Cannot modify inventory after job ticket has been completed or is in transit."
                        .to_string(),
                ));
            }
        }
    }

    conn.execute(
        "DELETE FROM move_inventory WHERE id = ?1",
        crate::params![item_id],
    )
    .await?;

    let _ = super::pricing::calculate_and_save_move_quote(requester_user_id, job_ticket_id).await;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn get_move_inventory_summary(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<MoveInventorySummary, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

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

    let items = get_move_inventory(requester_user_id, job_ticket_id).await?;

    let total_item_count = items.iter().map(|i| i.quantity).sum();
    let total_volume_m3 = items
        .iter()
        .map(|i| i.estimated_volume_m3 * (i.quantity as f64))
        .sum();

    let total_weight_kg = items
        .iter()
        .map(|i| {
            let item_weight = if i.estimated_weight_kg > 0.0 {
                i.estimated_weight_kg
            } else {
                i.estimated_volume_m3 * 150.0
            };
            item_weight * (i.quantity as f64)
        })
        .sum();

    let total_weight_lbs = total_weight_kg * 2.20462;
    let total_volume_cu_ft = super::pricing::convert_m3_to_cu_ft(total_volume_m3);
    let recommended_truck_m3 = total_volume_m3 * 1.2;
    let recommended_truck_cu_ft = super::pricing::convert_m3_to_cu_ft(recommended_truck_m3);
    let recommended_crew_size = if total_volume_m3 > 30.0 { 4 } else if total_volume_m3 > 15.0 { 3 } else { 2 };

    Ok(MoveInventorySummary {
        total_volume_m3,
        total_weight_kg,
        total_volume_cu_ft,
        total_weight_lbs,
        total_item_count,
        recommended_truck_m3,
        recommended_truck_cu_ft,
        recommended_crew_size,
        truck_capacity_exceeded: false,
        truck_capacity_warning: None,
    })
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
pub async fn scan_inventory_item_by_barcode(
    requester_user_id: String,
    job_ticket_id: String,
    barcode: String,
    new_status: String,
) -> Result<MoveInventoryItem, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let now_ms = chrono::Utc::now().timestamp_millis();
    let barcode_trimmed = barcode.trim();

    let (item_id, item_ws): (String, String) = conn
        .query_row(
            "SELECT id, workspace_id FROM move_inventory WHERE job_ticket_id = ?1 AND (barcode_tag = ?2 OR id = ?2)",
            crate::params![&job_ticket_id, barcode_trimmed],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .map_err(|_| {
            YntraError::NotFoundError(format!(
                "Inventory barcode tag or item ID '{}' not found for this job ticket",
                barcode_trimmed
            ))
        })?;

    if auth.workspace_id != item_ws {
        return Err(YntraError::AuthError(
            "Access denied: workspace mismatch".to_string(),
        ));
    }

    conn.execute(
        "UPDATE move_inventory SET scan_status = ?1, last_scanned_at = ?2, last_scanned_by = ?3, updated_at = ?4, sync_status = 'pending' WHERE id = ?5",
        crate::params![&new_status, now_ms, &auth.user_id, now_ms, &item_id],
    ).await?;

    let items = get_move_inventory(requester_user_id, job_ticket_id).await?;
    let updated = items
        .into_iter()
        .find(|i| i.id == item_id)
        .ok_or_else(|| YntraError::NotFoundError("Updated item missing".to_string()))?;

    notify_observers();
    Ok(updated)
}
