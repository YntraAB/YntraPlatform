use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;

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

    let piano_keywords = [
        "piano", "flygel", "grand piano", "upright piano", "organ", "orgel", "pianostol",
        "fortepiano", "cembalo", "harpsichord", "spinet", "spinetta", "pianolyft", "klaver",
        "pianino",
    ];
    for kw in piano_keywords {
        if text.contains(kw) {
            return surcharge_piano;
        }
    }

    let safe_keywords = [
        "safe", "kassaskåp", "kassaskap", "värdeskåp", "vardeskap", "gun safe", "vapenskåp",
        "vapenskap", "heavy safe", "fireproof safe", "skåp tungt", "skap tungt", "tunglyft",
        "valv", "penning-skåp", "penningskåp", "server rack", "racksling", "heavy machinery",
        "säkerhetsskåp", "sakerhetsskap",
    ];
    for kw in safe_keywords {
        if text.contains(kw) {
            return surcharge_safe;
        }
    }

    let jacuzzi_keywords = [
        "jacuzzi", "badkar", "bathtub", "spa", "spabad", "hot tub", "hottub", "whirlpool",
        "bastu", "sauna", "bubbelpool", "bubbelbad", "isbad", "massagebadkar",
    ];
    for kw in jacuzzi_keywords {
        if text.contains(kw) {
            return surcharge_jacuzzi;
        }
    }

    let fragile_keywords = [
        "konst", "tavla", "painting", "fragile", "skör", "skor", "bräcklig", "bracklig",
        "glass", "glas", "mirror", "spegel", "kristall", "crystal", "sculpture", "skulptur",
        "målning", "malning", "antikt", "antique", "porslin", "porcelain", "akvarium",
        "aquarium", "ljuskrona", "chandelier", "marmor", "marble", "stenskiva", "stone slab",
        "vitrinskåp", "vitrinskap",
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

    if text.contains("server rack")
        || text.contains("racksling")
        || text.contains("ups batteri")
        || text.contains("serverstativ")
    {
        return surcharge_server_rack;
    }

    if text.contains("treadmill")
        || text.contains("löpband")
        || text.contains("lopband")
        || text.contains("roddmaskin")
        || text.contains("crosstrainer")
        || text.contains("gym")
    {
        return surcharge_fitness_equipment;
    }

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

    if (!origin_has_elevator || is_small_elevator(origin_elevator_size.as_ref()))
        && origin_floor != 0
    {
        let base_stair = (origin_floor.abs() as f64) * stairs_surcharge_per_floor;
        stairs_surcharge += base_stair * origin_stair_mult;
    }
    if origin_has_elevator && is_small_elevator(origin_elevator_size.as_ref()) {
        stairs_surcharge += surcharge_small_elevator;
    }

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

    if long_carry_meters > 0 {
        stairs_surcharge += (long_carry_meters as f64) * surcharge_long_carry_per_meter;
    }

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
        _scheduled_date,
        _route_stops_json,
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

    let inventory = super::inventory::get_move_inventory(
        requester_user_id.clone(),
        job_ticket_id.clone(),
    )
    .await?;

    let total_vol: f64 = inventory
        .iter()
        .map(|i| i.estimated_volume_m3 * (i.quantity as f64))
        .sum();

    let base_price = total_vol * 450.0;
    let stairs_surcharge = calculate_access_and_stair_surcharge(
        origin_floor,
        destination_floor,
        origin_has_elevator,
        destination_has_elevator,
        None,
        None,
        None,
        None,
        long_carry_meters,
        false,
        250.0,
        15.0,
        1200.0,
        150.0,
    );

    let distance_fee = toll_fees + 500.0;
    let packing_supplies_fee = total_vol * 80.0;
    let total_price = base_price + stairs_surcharge + distance_fee + packing_supplies_fee;

    let now_ms = chrono::Utc::now().timestamp_millis();
    let quote_id = format!("quote-{}", job_ticket_id);

    conn.execute(
        "INSERT INTO move_quotes (id, workspace_id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, updated_at, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'draft', ?9, 'pending') ON CONFLICT(job_ticket_id) DO UPDATE SET base_price = ?4, distance_fee = ?5, stairs_surcharge = ?6, packing_supplies_fee = ?7, total_price = ?8, updated_at = ?9, sync_status = 'pending'",
        crate::params![
            quote_id,
            job_ws,
            job_ticket_id,
            base_price,
            distance_fee,
            stairs_surcharge,
            packing_supplies_fee,
            total_price,
            now_ms,
        ],
    ).await?;

    notify_observers();
    Ok(())
}
