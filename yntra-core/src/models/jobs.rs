use rkyv::{Archive, Deserialize, Serialize};

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct JobTicket {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub description: String,
    pub location_address: String,
    pub priority: String,
    pub status: String,
    pub assigned_user_id: Option<String>,
    pub scheduled_date: String,
    pub checklist_json: String,
    pub completion_report: Option<String>,
    pub created_at: String,
    pub updated_at: i64,
    pub sync_status: String,
    pub origin_address: Option<String>,
    pub destination_address: Option<String>,
    pub origin_floor: i32,
    pub destination_floor: i32,
    pub origin_has_elevator: bool,
    pub destination_has_elevator: bool,
    pub origin_parking_permit_needed: bool,
    pub destination_parking_permit_needed: bool,
    pub assigned_vehicle_id: Option<String>,
    pub route_stops_json: Option<String>,
    pub long_carry_meters: i32,
    pub toll_fees: f64,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct MoveInventoryItem {
    pub id: String,
    pub workspace_id: String,
    pub job_ticket_id: String,
    pub item_category: String,
    pub item_name: String,
    pub quantity: i32,
    pub estimated_volume_m3: f64,
    pub handling_notes: Option<String>,
    pub updated_at: i64,
    pub sync_status: String,
    pub room_name: Option<String>,
    pub estimated_weight_kg: f64,
    pub preset_id: Option<String>,
    pub barcode_tag: Option<String>,
    pub scan_status: String,
    pub last_scanned_at: Option<i64>,
    pub last_scanned_by: Option<String>,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct InventoryScanManifest {
    pub job_ticket_id: String,
    pub total_items: i32,
    pub packed_count: i32,
    pub loaded_count: i32,
    pub unloaded_count: i32,
    pub missing_count: i32,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct FurniturePreset {
    pub id: String,
    pub category: String,
    pub name: String,
    pub default_volume_m3: f64,
    pub default_weight_kg: f64,
    pub default_handling_notes: Option<String>,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct MoveInventorySummary {
    pub total_volume_m3: f64,
    pub total_weight_kg: f64,
    pub total_volume_cu_ft: f64,
    pub total_weight_lbs: f64,
    pub total_item_count: i32,
    pub recommended_truck_m3: f64,
    pub recommended_truck_cu_ft: f64,
    pub recommended_crew_size: i32,
    pub truck_capacity_exceeded: bool,
    pub truck_capacity_warning: Option<String>,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct MoveQuote {
    pub id: String,
    pub workspace_id: String,
    pub job_ticket_id: String,
    pub base_price: i64,
    pub distance_fee: i64,
    pub stairs_surcharge: i64,
    pub packing_supplies_fee: i64,
    pub total_price: i64,
    pub status: String,
    pub accepted_at: Option<i64>,
    pub updated_at: i64,
    pub sync_status: String,
    pub manual_price_override: Option<f64>,
    pub price_discount: Option<f64>,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct CommercialRouteRestrictions {
    pub low_bridge_warning: bool,
    pub environmental_zone_warning: bool,
    pub weight_limit_warning: bool,
    pub parking_permit_required: bool,
    pub restriction_details: Vec<String>,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct JobPackagingItem {
    pub id: String,
    pub workspace_id: String,
    pub job_ticket_id: String,
    pub item_name: String,
    pub quantity: i32,
    pub price_per_unit: f64,
    pub is_leased: bool,
    pub returned_quantity: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub sync_status: String,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct DamageInspection {
    pub id: String,
    pub workspace_id: String,
    pub job_ticket_id: String,
    pub item_inventory_id: Option<String>,
    pub item_name: String,
    pub damage_type: String,
    pub severity: String,
    pub annotations: Option<String>,
    pub photo_url: Option<String>,
    pub timestamp_ms: i64,
    pub inspector_user_id: String,
    pub client_acknowledged: bool,
    pub client_signature_svg: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub sync_status: String,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct DriverComplianceStatus {
    pub is_compliant: bool,
    pub license_class: String,
    pub required_license_class: String,
    pub total_driving_hours_today: f64,
    pub max_allowed_daily_hours: f64,
    pub rest_period_compliant: bool,
    pub compliance_warnings: Vec<String>,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct OfflineMediaPointer {
    pub hash_pointer: String,
    pub media_type: String,
    pub original_size_bytes: i64,
    pub compressed_size_bytes: i64,
    pub compression_ratio_percent: f64,
    pub mime_type: String,
    pub upload_status: String,
    pub created_at: i64,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct DamagedItemClaim {
    pub id: String,
    pub workspace_id: String,
    pub job_ticket_id: String,
    pub item_name: String,
    pub description: String,
    pub claimed_amount: f64,
    pub approved_amount: Option<f64>,
    pub repair_quote_amount: Option<f64>,
    pub insurance_reference: Option<String>,
    pub photo_urls_json: String,
    pub status: String,
    pub settlement_notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct ClaimPayoutResult {
    pub success: bool,
    pub claim_id: String,
    pub payout_amount: f64,
    pub insurance_reference: String,
    pub new_status: String,
    pub message: String,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct CustomerLiveTrackingPortal {
    pub job_ticket_id: String,
    pub driver_name: String,
    pub driver_phone: Option<String>,
    pub vehicle_license_plate: Option<String>,
    pub current_lat: f64,
    pub current_lon: f64,
    pub estimated_arrival_mins: i32,
    pub route_status: String,
    pub live_tracking_url: String,
    pub last_updated_at: i64,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct QuoteDepositApprovalResult {
    pub success: bool,
    pub quote_id: String,
    pub deposit_amount: f64,
    pub remaining_balance: f64,
    pub payment_session_url: Option<String>,
    pub quote_status: String,
    pub message: String,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct MoveVehicle {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub license_plate: String,
    pub capacity_m3: f64,
    pub status: String,
    pub updated_at: i64,
    pub sync_status: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub last_ping: Option<i64>,
    pub gps_device_id: Option<String>,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct MoveSignature {
    pub id: String,
    pub workspace_id: String,
    pub job_ticket_id: String,
    pub signer_name: String,
    pub signature_data_base64: String,
    pub signed_at: i64,
    pub sync_status: String,
    pub ip_address: Option<String>,
    pub geolocation: Option<String>,
    pub device_fingerprint: Option<String>,
    pub terms_version: Option<String>,
    pub terms_hash: Option<String>,
    pub signature_hash: Option<String>,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct BillOfLading {
    pub bol_number: String,
    pub workspace_id: String,
    pub job_ticket_id: String,
    pub carrier_name: String,
    pub shipper_name: String,
    pub origin_address: String,
    pub destination_address: String,
    pub valuation_option: String,
    pub valuation_declared_amount: f64,
    pub valuation_deductible: f64,
    pub valuation_premium: f64,
    pub total_estimated_weight_lbs: f64,
    pub legal_terms: String,
    pub customer_signature_hash: Option<String>,
    pub created_at: i64,
    pub origin_signature_hash: Option<String>,
    pub destination_signature_hash: Option<String>,
    pub signed_origin_at: Option<i64>,
    pub signed_destination_at: Option<i64>,
    pub document_tamper_hash: String,
    pub inventory_manifest_json: String,
    pub carrier_dot_number: Option<String>,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct RouteSegment {
    pub segment_index: i32,
    pub start_address: String,
    pub end_address: String,
    pub distance_km: f64,
    pub estimated_duration_minutes: f64,
    pub segment_type: String,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct MultiSegmentRouteSummary {
    pub total_distance_km: f64,
    pub total_duration_minutes: f64,
    pub total_segments: i32,
    pub segments: Vec<RouteSegment>,
    pub storage_in_transit_stops: i32,
}

#[derive(
    uniffi::Record,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct CrewDispatchRequirement {
    pub is_matched: bool,
    pub required_crew_count: i32,
    pub assigned_crew_count: i32,
    pub required_license_class: String,
    pub required_equipment: Vec<String>,
    pub missing_equipment: Vec<String>,
    pub warnings: Vec<String>,
}
