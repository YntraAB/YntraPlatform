use crate::database;
use crate::infra::observer::notify_observers;
use crate::infra::errors::YntraError;
use crate::{MoveInventoryItem, MoveQuote, JobPackagingItem, FurniturePreset, MoveInventorySummary, InventoryScanManifest};

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
                scan_status: row.get::<Option<String>>(14)?.unwrap_or_else(|| "unscanned".to_string()),
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

    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, accepted_at, updated_at, sync_status, manual_price_override, price_discount FROM move_quotes WHERE job_ticket_id = ?1 LIMIT 1",
    ).await?;

    let mut rows = stmt.query(crate::params![job_ticket_id]).await?;
    if let Some(row) = rows.next().await? {
        Ok(Some(MoveQuote {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            job_ticket_id: row.get(2)?,
            base_price: row.get::<f64>(3)? as i64,
            distance_fee: row.get::<f64>(4)? as i64,
            stairs_surcharge: row.get::<f64>(5)? as i64,
            packing_supplies_fee: row.get::<f64>(6)? as i64,
            total_price: row.get::<f64>(7)? as i64,
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

    let customer_id = conn.query_row(
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
    ).await;

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

    if auth.role == "guest" || auth.role == "anonymous" || auth.role == "deleted" {
        return Err(YntraError::AuthError(
            "Access denied: insufficient permissions".to_string(),
        ));
    }

    let weight = estimated_weight_kg.unwrap_or(0.0);
    let job_short = if job_ticket_id.len() >= 6 { &job_ticket_id[..6] } else { &job_ticket_id };
    let id_short = if id.len() >= 6 { &id[..6] } else { &id };
    let barcode_tag = format!("YNT-{}-{}", job_short.to_uppercase(), id_short.to_uppercase());

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
            barcode_tag
        ],
    ).await?;

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

    let mut stmt = conn.prepare(
        "SELECT id FROM move_inventory WHERE job_ticket_id = ?1 AND (UPPER(barcode_tag) = ?2 OR UPPER(id) = ?2)"
    ).await?;
    let mut rows = stmt.query(crate::params![&job_ticket_id, &clean_barcode]).await?;

    if let Some(row) = rows.next().await? {
        let item_id: String = row.get(0)?;
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
        let updated = items.into_iter().find(|i| i.id == item_id).ok_or_else(|| YntraError::NotFoundError("Item not found after update".to_string()))?;
        Ok(updated)
    } else {
        Err(YntraError::NotFoundError(format!("No item matching barcode '{}' found on this job", barcode_tag)))
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
pub async fn get_move_inventory_summary(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<MoveInventorySummary, YntraError> {
    let items = get_move_inventory(requester_user_id, job_ticket_id).await?;

    let mut total_vol = 0.0;
    let mut total_weight = 0.0;
    let mut total_count = 0;

    for item in items {
        let q = item.quantity as f64;
        total_vol += item.estimated_volume_m3 * q;
        total_weight += item.estimated_weight_kg * q;
        total_count += item.quantity;
    }

    // Buffer of +20% for truck volume loading efficiency
    let recommended_truck = (total_vol * 1.2 * 10.0).round() / 10.0;

    // Determine crew recommendation
    let recommended_crew = if total_vol > 35.0 || total_weight > 800.0 {
        4
    } else if total_vol > 15.0 || total_weight > 350.0 {
        3
    } else {
        2
    };

    Ok(MoveInventorySummary {
        total_volume_m3: (total_vol * 100.0).round() / 100.0,
        total_weight_kg: (total_weight * 10.0).round() / 10.0,
        total_item_count: total_count,
        recommended_truck_m3: recommended_truck,
        recommended_crew_size: recommended_crew,
    })
}

#[uniffi::export]
pub async fn delete_move_inventory_item(
    requester_user_id: String,
    item_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (item_ws,): (String,) = conn
        .query_row(
            "SELECT workspace_id FROM move_inventory WHERE id = ?1",
            crate::params![&item_id],
            |r| Ok((r.get(0)?,)),
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

    conn.execute(
        "DELETE FROM move_inventory WHERE id = ?1",
        crate::params![item_id],
    ).await?;

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
        "piano", "flygel", "grand piano", "upright piano", "organ", "orgel",
        "pianostol", "fortepiano", "cembalo", "harpsichord", "spinet", "spinetta",
        "pianolyft", "klaver", "pianino"
    ];
    for kw in piano_keywords {
        if text.contains(kw) {
            return surcharge_piano;
        }
    }

    // 2. Safe / Vault / Heavy Metallic / Machinery
    let safe_keywords = [
        "safe", "kassaskåp", "kassaskap", "värdeskåp", "vardeskap", "gun safe",
        "vapenskåp", "vapenskap", "heavy safe", "fireproof safe", "skåp tungt",
        "skap tungt", "tunglyft", "valv", "penning-skåp", "penningskåp", "server rack",
        "racksling", "heavy machinery", "säkerhetsskåp", "sakerhetsskap"
    ];
    for kw in safe_keywords {
        if text.contains(kw) {
            return surcharge_safe;
        }
    }

    // 3. Jacuzzi / Bath / Spa / Sauna
    let jacuzzi_keywords = [
        "jacuzzi", "badkar", "bathtub", "spa", "spabad", "hot tub", "hottub",
        "whirlpool", "bastu", "sauna", "bubbelpool", "bubbelbad", "isbad", "massagebadkar"
    ];
    for kw in jacuzzi_keywords {
        if text.contains(kw) {
            return surcharge_jacuzzi;
        }
    }

    // 4. Fragile / Fine Art / Glass / Mirrors / Antiques
    let fragile_keywords = [
        "konst", "tavla", "painting", "fragile", "skör", "skor", "bräcklig",
        "bracklig", "glass", "glas", "mirror", "spegel", "kristall", "crystal",
        "sculpture", "skulptur", "målning", "malning", "antikt", "antique",
        "porslin", "porcelain", "akvarium", "aquarium", "ljuskrona", "chandelier",
        "marmor", "marble", "stenskiva", "stone slab", "vitrinskåp", "vitrinskap"
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
    if text.contains("server rack") || text.contains("racksling") || text.contains("ups batteri") || text.contains("serverstativ") {
        return surcharge_server_rack;
    }

    // 2. Heavy Fitness Equipment
    if text.contains("treadmill") || text.contains("löpband") || text.contains("lopband") || text.contains("roddmaskin") || text.contains("crosstrainer") || text.contains("gym") {
        return surcharge_fitness_equipment;
    }

    // 3. Marble / Glass Tabletops
    if text.contains("marmorbord") || text.contains("glasbord") || text.contains("glass table") || text.contains("marble table") || text.contains("stenskiva") {
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
    let get_stair_multiplier = |stype: Option<&str>| match stype.unwrap_or("standard").to_lowercase().as_str() {
        "spiral" | "spiraltrappa" => mult_spiral,
        "narrow" | "trång" | "trang" => mult_narrow,
        "outdoor" | "utomhustrappa" => mult_outdoor,
        _ => 1.0,
    };

    let origin_stair_mult = get_stair_multiplier(origin_staircase_type.as_deref());
    let dest_stair_mult = get_stair_multiplier(destination_staircase_type.as_deref());

    let mut stairs_surcharge: f64 = 0.0;

    let is_small_elevator = |size: Option<&String>| {
        size.map(|s| s.to_lowercase()) == Some("small".to_string())
    };

    // Origin stair surcharge
    if (!origin_has_elevator || is_small_elevator(origin_elevator_size.as_ref())) && origin_floor != 0 {
        let base_stair = (origin_floor.abs() as f64) * stairs_surcharge_per_floor;
        stairs_surcharge += base_stair * origin_stair_mult;
    }
    if origin_has_elevator && is_small_elevator(origin_elevator_size.as_ref()) {
        stairs_surcharge += surcharge_small_elevator;
    }

    // Destination stair surcharge
    if (!destination_has_elevator || is_small_elevator(destination_elevator_size.as_ref())) && destination_floor != 0 {
        let base_stair = (destination_floor.abs() as f64) * stairs_surcharge_per_floor;
        stairs_surcharge += base_stair * dest_stair_mult;
    }
    if destination_has_elevator && is_small_elevator(destination_elevator_size.as_ref()) {
        stairs_surcharge += surcharge_small_elevator;
    }

    // Long carry surcharge
    if long_carry_meters > 0 {
        stairs_surcharge += (long_carry_meters as f64) * surcharge_long_carry_per_meter;
    }

    // External crane / hoist requirement surcharge
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
pub async fn calculate_and_save_move_quote(
    requester_user_id: String,
    job_ticket_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    // 1. Fetch Job Ticket details
    let mut stmt = conn.prepare(
        "SELECT workspace_id, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, COALESCE(long_carry_meters, 0), COALESCE(toll_fees, 0.0), scheduled_date FROM job_tickets WHERE id = ?1",
    ).await?;
    let mut rows = stmt.query(crate::params![&job_ticket_id]).await?;
    let (job_ws, origin_floor, destination_floor, origin_has_elevator, destination_has_elevator, long_carry_meters, toll_fees, scheduled_date) = if let Some(row) = rows.next().await? {
        (
            row.get::<String>(0)?,
            row.get::<i64>(1)? as i32,
            row.get::<i64>(2)? as i32,
            row.get::<bool>(3)?,
            row.get::<bool>(4)?,
            row.get::<i64>(5)? as i32,
            row.get::<f64>(6)?,
            row.get::<String>(7)?,
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

    let surcharge_piano = settings_json.get("surcharge_piano").and_then(|v| v.as_f64()).unwrap_or(1500.0);
    let surcharge_safe = settings_json.get("surcharge_safe").and_then(|v| v.as_f64()).unwrap_or(2000.0);
    let surcharge_jacuzzi = settings_json.get("surcharge_jacuzzi").and_then(|v| v.as_f64()).unwrap_or(2500.0);
    let surcharge_fragile = settings_json.get("surcharge_fragile").and_then(|v| v.as_f64()).unwrap_or(500.0);
    let surcharge_server_rack = settings_json.get("surcharge_server_rack").and_then(|v| v.as_f64()).unwrap_or(3000.0);
    let surcharge_fitness_equipment = settings_json.get("surcharge_fitness_equipment").and_then(|v| v.as_f64()).unwrap_or(800.0);
    let surcharge_marble_glass = settings_json.get("surcharge_marble_glass").and_then(|v| v.as_f64()).unwrap_or(600.0);

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

    // 4. Quoting Calculations:
    let base_rate_per_m3 = settings_json
        .get("moving_base_rate_per_m3")
        .and_then(|v| v.as_f64())
        .unwrap_or(500.0);
    let distance_fee_flat = settings_json
        .get("moving_distance_fee_flat")
        .and_then(|v| v.as_f64())
        .unwrap_or(800.0);
    let stairs_surcharge_per_floor = settings_json
        .get("moving_stairs_surcharge_per_floor")
        .and_then(|v| v.as_f64())
        .unwrap_or(300.0);
    let packing_supplies_fee_per_m3 = settings_json
        .get("moving_packing_supplies_fee_per_m3")
        .and_then(|v| v.as_f64())
        .unwrap_or(100.0);

    // Dynamic Multipliers & Tariffs
    let distance_km = settings_json.get("estimated_distance_km").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let local_radius = settings_json.get("moving_local_radius_km").and_then(|v| v.as_f64()).unwrap_or(30.0);
    let per_km_rate = settings_json.get("moving_per_km_rate").and_then(|v| v.as_f64()).unwrap_or(15.0);

    let mut distance_fee = distance_fee_flat as i64;
    if distance_km > local_radius {
        distance_fee += ((distance_km - local_radius) * per_km_rate) as i64;
    }
    distance_fee += toll_fees as i64;

    // Base hourly/labor rate depending on configured pricing model
    let pricing_model = settings_json
        .get("moving_pricing_model")
        .and_then(|v| v.as_str())
        .unwrap_or("volume");

    let mut base_price = if pricing_model == "hourly" {
        let crew_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM job_crew WHERE job_ticket_id = ?1",
                crate::params![&job_ticket_id],
                |r| r.get(0),
            )
            .await
            .unwrap_or(0);

        let default_crew_size = settings_json
            .get("moving_default_crew_size")
            .and_then(|v| v.as_i64())
            .unwrap_or(2) as f64;

        let active_crew_size = if crew_count > 0 {
            crew_count as f64
        } else {
            default_crew_size
        };

        let hourly_rate_per_mover = settings_json
            .get("moving_hourly_rate_per_mover")
            .and_then(|v| v.as_f64())
            .unwrap_or(400.0);

        let hourly_rate_vehicle = settings_json
            .get("moving_hourly_rate_vehicle")
            .and_then(|v| v.as_f64())
            .unwrap_or(400.0);

        let hourly_rate = if settings_json.get("moving_hourly_rate_per_mover").is_some()
            || settings_json.get("moving_hourly_rate_vehicle").is_some()
        {
            active_crew_size * hourly_rate_per_mover + hourly_rate_vehicle
        } else {
            settings_json
                .get("moving_hourly_rate")
                .and_then(|v| v.as_f64())
                .unwrap_or(1200.0)
        };

        let hours_per_m3 = settings_json
            .get("moving_hours_per_m3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.15);
        let minimum_hours = settings_json
            .get("moving_minimum_hours")
            .and_then(|v| v.as_f64())
            .unwrap_or(3.0);
        let estimated_hours = (total_volume * hours_per_m3).max(minimum_hours);
        (estimated_hours * hourly_rate) as i64
    } else {
        (total_volume * base_rate_per_m3) as i64
    };

    // Apply Weekend and Peak-Season / End-of-Month Multipliers
    if let Ok(date) = chrono::NaiveDate::parse_from_str(&scheduled_date, "%Y-%m-%d") {
        use chrono::Datelike;
        let weekday = date.weekday();
        let is_weekend = weekday == chrono::Weekday::Sat || weekday == chrono::Weekday::Sun;
        let day_of_month = date.day();
        let is_peak_season = day_of_month >= 25 || day_of_month <= 3;

        let weekend_multiplier = if is_weekend {
            settings_json.get("moving_weekend_multiplier").and_then(|v| v.as_f64()).unwrap_or(1.25)
        } else {
            1.0
        };
        let peak_multiplier = if is_peak_season {
            settings_json.get("moving_peak_season_multiplier").and_then(|v| v.as_f64()).unwrap_or(1.15)
        } else {
            1.0
        };

        base_price = ((base_price as f64) * weekend_multiplier * peak_multiplier) as i64;
    }

    // Add specialty/heavy item handling fees to base labor price
    base_price += specialty_surcharge as i64;

    let surcharge_long_carry_per_meter = settings_json
        .get("surcharge_long_carry_per_meter")
        .and_then(|v| v.as_f64())
        .unwrap_or(40.0);
    let surcharge_crane_hoist = settings_json
        .get("surcharge_crane_hoist")
        .and_then(|v| v.as_f64())
        .unwrap_or(3500.0);
    let surcharge_small_elevator = settings_json
        .get("surcharge_small_elevator")
        .and_then(|v| v.as_f64())
        .unwrap_or(500.0);

    let origin_staircase_type = settings_json.get("origin_staircase_type").and_then(|v| v.as_str()).map(|s| s.to_string());
    let destination_staircase_type = settings_json.get("destination_staircase_type").and_then(|v| v.as_str()).map(|s| s.to_string());
    let origin_elevator_size = settings_json.get("origin_elevator_size").and_then(|v| v.as_str()).map(|s| s.to_string());
    let destination_elevator_size = settings_json.get("destination_elevator_size").and_then(|v| v.as_str()).map(|s| s.to_string());
    let requires_crane_hoist = settings_json.get("requires_crane_hoist").and_then(|v| v.as_bool()).unwrap_or(false);

    let mult_spiral = settings_json.get("mult_spiral_staircase").and_then(|v| v.as_f64()).unwrap_or(1.5);
    let mult_narrow = settings_json.get("mult_narrow_staircase").and_then(|v| v.as_f64()).unwrap_or(1.3);
    let mult_outdoor = settings_json.get("mult_outdoor_staircase").and_then(|v| v.as_f64()).unwrap_or(1.2);

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
    ) as i64;
    let actual_supplies_cost: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(quantity * price_per_unit), 0.0) FROM job_packaging_items WHERE job_ticket_id = ?1",
            crate::params![&job_ticket_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(0.0);

    let packing_supplies_fee = if actual_supplies_cost > 0.0 {
        actual_supplies_cost as i64
    } else {
        (total_volume * packing_supplies_fee_per_m3) as i64
    };
    let now_ms = chrono::Utc::now().timestamp_millis();
    
    // Check if quote exists to keep its status, default to "sent"
    let mut quote_stmt = conn.prepare(
        "SELECT id, status, manual_price_override, price_discount FROM move_quotes WHERE job_ticket_id = ?1 LIMIT 1",
    ).await?;
    let mut q_rows = quote_stmt.query(crate::params![&job_ticket_id]).await?;
    let (quote_id, quote_status, existing_override, existing_discount) = if let Some(row) = q_rows.next().await? {
        (
            row.get::<String>(0)?,
            row.get::<String>(1)?,
            row.get::<Option<f64>>(2)?,
            row.get::<Option<f64>>(3)?,
        )
    } else {
        (uuid::Uuid::new_v4().to_string(), "sent".to_string(), None, None)
    };

    let calculated_total = base_price + distance_fee + stairs_surcharge + packing_supplies_fee;
    let mut total_price = if let Some(override_val) = existing_override {
        override_val as i64
    } else {
        calculated_total
    };
    if let Some(discount) = existing_discount {
        total_price = (total_price - discount as i64).max(0);
    }

    conn.execute(
        "INSERT OR REPLACE INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at, sync_status, manual_price_override, price_discount) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'pending', ?11, ?12)",
        crate::params![
            quote_id,
            job_ws,
            job_ticket_id,
            base_price as f64,
            distance_fee as f64,
            stairs_surcharge as f64,
            packing_supplies_fee as f64,
            total_price as f64,
            quote_status,
            now_ms,
            existing_override,
            existing_discount
        ],
    ).await?;

    notify_observers();
    Ok(())
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
        .unwrap_or(0) > 0;

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
    ).await?;

    // Recalculate quote if exists
    let quote_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM move_quotes WHERE job_ticket_id = ?1",
            crate::params![&job_ticket_id],
            |r| r.get::<i64>(0),
        )
        .await
        .unwrap_or(0) > 0;

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
    ).await?;

    let quote_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM move_quotes WHERE job_ticket_id = ?1",
            crate::params![&job_ticket_id],
            |r| r.get::<i64>(0),
        )
        .await
        .unwrap_or(0) > 0;

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
            sync_status: row.get::<Option<String>>(10)?.unwrap_or_else(|| "pending".to_string()),
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
    let mut quote_stmt = conn.prepare(
        "SELECT id FROM move_quotes WHERE job_ticket_id = ?1 LIMIT 1",
    ).await?;
    let mut q_rows = quote_stmt.query(crate::params![&job_ticket_id]).await?;
    let has_quote = q_rows.next().await?.is_some();

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
        return Err(YntraError::AuthError("Access denied: workspace mismatch".to_string()));
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

    let deposit_pct = settings_json.get("moving_deposit_percent").and_then(|v| v.as_f64()).unwrap_or(20.0);
    let deposit_amount = (total_price * (deposit_pct / 100.0)).round().max(500.0);
    let remaining_balance = (total_price - deposit_amount).max(0.0);
    let now_ms = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "UPDATE move_quotes SET status = 'accepted', updated_at = ?1 WHERE id = ?2 AND workspace_id = ?3",
        crate::params![now_ms, &quote_id, &ws_id],
    ).await?;

    conn.execute(
        "UPDATE job_tickets SET status = 'assigned', updated_at = ?1 WHERE id = ?2 AND workspace_id = ?3",
        crate::params![now_ms, &job_id, &ws_id],
    ).await?;

    notify_observers();

    let payment_url = format!("https://pay.yntra.se/deposit/{}?method={}", quote_id, payment_method.to_lowercase());

    Ok(crate::models::QuoteDepositApprovalResult {
        success: true,
        quote_id,
        deposit_amount,
        remaining_balance,
        payment_session_url: Some(payment_url),
        quote_status: "accepted".to_string(),
        message: format!("Quote approved! Non-refundable deposit of {:.2} SEK initiated via {}.", deposit_amount, payment_method),
    })
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
        assert!(catalog.iter().any(|p| p.id == "sofa-3p" && p.default_volume_m3 == 1.8 && p.default_weight_kg == 75.0));
        assert!(catalog.iter().any(|p| p.id == "bed-king" && p.category == "Bedroom"));

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
        ).await.unwrap();

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
        ).await.unwrap();

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
        ).await.unwrap();

        // 3. Verify get_move_inventory retrieves room_name and estimated_weight_kg
        let items = get_move_inventory("u-inv-staff".to_string(), "job-inv-sum-1".to_string()).await.unwrap();
        assert_eq!(items.len(), 3);
        let sofa = items.iter().find(|i| i.item_name == "3-Seater Sofa").unwrap();
        assert_eq!(sofa.room_name, Some("Living Room".to_string()));
        assert_eq!(sofa.estimated_weight_kg, 75.0);
        assert_eq!(sofa.preset_id, Some("sofa-3p".to_string()));

        // 4. Verify get_move_inventory_summary calculations
        let summary = get_move_inventory_summary("u-inv-staff".to_string(), "job-inv-sum-1".to_string()).await.unwrap();
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
        conn.execute("DELETE FROM move_inventory WHERE job_ticket_id = 'job-inv-sum-1'", ()).await.unwrap();
        conn.execute("DELETE FROM job_tickets WHERE id = 'job-inv-sum-1'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-inv-staff'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-inv-sum-test'", ()).await.unwrap();
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
        ).await.unwrap();

        let items = get_move_inventory("u-bc-staff".to_string(), "job-bc-1".to_string()).await.unwrap();
        assert_eq!(items.len(), 1);
        let barcode = items[0].barcode_tag.clone().unwrap();
        assert!(barcode.starts_with("YNT-"));
        assert_eq!(items[0].scan_status, "unscanned");

        // 2. Scan item -> transition to 'packed'
        let item_packed = scan_inventory_item_by_barcode("u-bc-staff".to_string(), "job-bc-1".to_string(), barcode.clone(), "packed".to_string()).await.unwrap();
        assert_eq!(item_packed.scan_status, "packed");
        assert!(item_packed.last_scanned_at.is_some());
        assert_eq!(item_packed.last_scanned_by, Some("u-bc-staff".to_string()));

        // 3. Scan item -> transition to 'loaded'
        let item_loaded = scan_inventory_item_by_barcode("u-bc-staff".to_string(), "job-bc-1".to_string(), barcode.clone(), "loaded".to_string()).await.unwrap();
        assert_eq!(item_loaded.scan_status, "loaded");

        // 4. Verify scan manifest report
        let manifest = get_inventory_scan_manifest("u-bc-staff".to_string(), "job-bc-1".to_string()).await.unwrap();
        assert_eq!(manifest.total_items, 1);
        assert_eq!(manifest.loaded_count, 1);
        assert_eq!(manifest.unloaded_count, 0);

        // Cleanup
        conn.execute("DELETE FROM move_inventory WHERE job_ticket_id = 'job-bc-1'", ()).await.unwrap();
        conn.execute("DELETE FROM job_tickets WHERE id = 'job-bc-1'", ()).await.unwrap();
        conn.execute("DELETE FROM users WHERE id = 'u-bc-staff'", ()).await.unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = 'ws-bc-test'", ()).await.unwrap();
    }
}
