mod billing;
pub mod claims;
mod crew;
pub mod crew_payroll;
pub mod hvac;
pub mod inspections;
pub mod media;
mod moves;
mod notifications;
pub mod pdf;
mod public_lead;
mod routing;
mod signatures;
pub mod sit_storage;
mod tickets;

pub use hvac::*;

pub use crew_payroll::calculate_mover_job_payroll_split;
pub use crew_payroll::distribute_job_customer_tip;
pub use crew_payroll::get_job_tip_distribution;
pub use pdf::generate_printable_bol_html;
pub use pdf::generate_printable_invoice_html;
pub use sit_storage::assign_job_to_warehouse_vault;
pub use sit_storage::calculate_sit_recurring_billing_summary;
pub use sit_storage::get_job_warehouse_vaults;
pub use sit_storage::release_job_from_warehouse_vault;

#[cfg(test)]
mod tests;

pub use tickets::create_job_ticket;
pub use tickets::get_job_tickets;
pub use tickets::get_job_tickets_rkyv;
pub use tickets::get_mover_field_sheet_manifest;
pub use tickets::optimize_job_route;
pub use tickets::schedule_job_ticket;
pub use tickets::submit_job_completion;
pub use tickets::update_job_moving_surcharges;
pub use tickets::update_job_status;
pub use tickets::update_route_stops;

pub use crew::add_crew_member;
pub use crew::assign_vehicle_to_job;
pub use crew::get_job_crew;
pub use crew::get_recommended_crew_dispatch_equipment;
pub use crew::remove_crew_member;
pub use crew::validate_crew_equipment_and_physical_matching;
pub use crew::validate_driver_tachograph_compliance;
pub use crew::validate_vehicle_dispatch_capacity;

pub use signatures::generate_bill_of_lading;
pub use signatures::get_bill_of_lading;
pub use signatures::get_job_signature;
pub use signatures::save_job_signature;
pub use signatures::save_job_signature_with_audit_trail;
pub use signatures::sign_bill_of_lading_phase;

pub use moves::MoveQuoteRevision;
pub use moves::accept_move_quote;
pub use moves::accept_move_quote_with_rut;
pub use moves::add_job_packaging_item;
pub use moves::calculate_access_and_stair_surcharge;
pub use moves::calculate_access_and_stair_surcharge_with_multipliers;
pub use moves::calculate_and_save_move_quote;
pub use moves::calculate_item_specialty_surcharge;
pub use moves::calculate_item_specialty_surcharge_extended;
pub use moves::calculate_packing_materials_tariff_estimate;
pub use moves::convert_volume_to_tariff_weight;
pub use moves::create_move_inventory_item;
pub use moves::create_move_inventory_item_with_details;
pub use moves::delete_move_inventory_item;
pub use moves::get_furniture_catalog;
pub use moves::get_inventory_scan_manifest;
pub use moves::get_job_packaging_items;
pub use moves::get_move_inventory;
pub use moves::get_move_inventory_summary;
pub use moves::get_move_quote;
pub use moves::get_move_quote_revisions;
pub use moves::remove_job_packaging_item;
pub use moves::scan_inventory_item_by_barcode;
pub use moves::update_customer_personal_number;
pub use moves::update_job_packaging_item_returned;
pub use moves::update_move_quote_price_adjustments;

pub use billing::adjust_invoice_for_actuals;
pub use billing::check_swish_payment_status;
pub use billing::confirm_mobile_pos_terminal_payment;
pub use billing::export_skatteverket_claims;
pub use billing::export_skatteverket_claims_detailed;
pub use billing::export_skatteverket_claims_strict;
pub use billing::export_skatteverket_claims_with_options;
pub use billing::generate_move_invoice;
pub use billing::get_accounting_general_ledger_summary;
pub use billing::get_erp_sync_history;
pub use billing::get_fleet_fuel_receipts;
pub use billing::get_move_invoice;
pub use billing::get_rut_invoices;
pub use billing::initiate_adyen_payment;
pub use billing::initiate_bankid_skatteverket_session;
pub use billing::initiate_mobile_pos_terminal_session;
pub use billing::initiate_stripe_payment;
pub use billing::initiate_swish_payment;
pub use billing::pay_move_invoice;
pub use billing::process_onsite_mpos_card_payment;
pub use billing::process_stripe_payment_webhook;
pub use billing::process_swish_payment_webhook;
pub use billing::reconcile_erp_payments;
pub use billing::record_fleet_fuel_receipt;
pub use billing::submit_skatteverket_claim_direct;
pub use billing::sync_fuel_receipts_to_erp;
pub use billing::sync_invoice_to_erp;
pub use billing::sync_payroll_journal_to_erp;
pub use billing::validate_customer_personal_number_for_rut;
pub use billing::validate_skatteverket_claim_batch;

pub use routing::{
    calculate_multi_segment_move_route, geocode, get_commercial_truck_directions_url,
    get_directions_url, get_job_multi_segment_route, verify_commercial_route_restrictions,
};

pub use notifications::{
    get_customer_live_tracking_portal, send_dispatch_departure_eta_sms, send_driver_en_route_alert,
    send_external_notification, send_live_eta_update_alert, send_post_move_review_request,
};
pub use public_lead::ingest_third_party_lead_webhook;
pub use public_lead::submit_public_booking_lead;

pub use inspections::acknowledge_damage_inspection_by_client;
pub use inspections::delete_damage_inspection;
pub use inspections::get_job_damage_inspections;
pub use inspections::record_damage_inspection;

pub use media::enqueue_offline_media_blob;
pub use media::get_offline_media_pointer;
pub use media::sync_pending_offline_media_blobs;

pub use claims::get_job_claims;
pub use claims::process_claim_payout;
pub use claims::submit_damaged_item_claim;
pub use claims::update_claim_status;

pub use moves::accept_move_quote_with_deposit;
pub use moves::confirm_quote_deposit_payment;
