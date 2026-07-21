mod tickets;
mod crew;
mod signatures;
mod moves;
mod billing;
mod routing;
mod notifications;
mod public_lead;
pub mod inspections;
pub mod media;
pub mod claims;

#[cfg(test)]
mod tests;

pub use tickets::get_job_tickets;
pub use tickets::create_job_ticket;
pub use tickets::update_job_moving_surcharges;
pub use tickets::update_job_status;
pub use tickets::schedule_job_ticket;
pub use tickets::submit_job_completion;
pub use tickets::get_job_tickets_rkyv;
pub use tickets::update_route_stops;
pub use tickets::optimize_job_route;

pub use crew::assign_vehicle_to_job;
pub use crew::add_crew_member;
pub use crew::remove_crew_member;
pub use crew::get_job_crew;
pub use crew::validate_driver_tachograph_compliance;

pub use signatures::save_job_signature;
pub use signatures::save_job_signature_with_audit_trail;
pub use signatures::get_job_signature;

pub use moves::get_move_inventory;
pub use moves::get_move_quote;
pub use moves::accept_move_quote;
pub use moves::create_move_inventory_item;
pub use moves::delete_move_inventory_item;
pub use moves::calculate_and_save_move_quote;
pub use moves::calculate_item_specialty_surcharge;
pub use moves::calculate_item_specialty_surcharge_extended;
pub use moves::calculate_access_and_stair_surcharge;
pub use moves::calculate_access_and_stair_surcharge_with_multipliers;
pub use moves::update_move_quote_price_adjustments;
pub use moves::add_job_packaging_item;
pub use moves::remove_job_packaging_item;
pub use moves::update_job_packaging_item_returned;
pub use moves::get_job_packaging_items;

pub use billing::generate_move_invoice;
pub use billing::get_move_invoice;
pub use billing::pay_move_invoice;
pub use billing::adjust_invoice_for_actuals;
pub use billing::initiate_swish_payment;
pub use billing::check_swish_payment_status;
pub use billing::initiate_stripe_payment;
pub use billing::initiate_adyen_payment;
pub use billing::get_rut_invoices;
pub use billing::export_skatteverket_claims;
pub use billing::initiate_bankid_skatteverket_session;
pub use billing::submit_skatteverket_claim_direct;
pub use billing::process_swish_payment_webhook;
pub use billing::process_stripe_payment_webhook;
pub use billing::process_onsite_mpos_card_payment;
pub use billing::sync_invoice_to_erp;
pub use billing::reconcile_erp_payments;

pub use routing::{get_directions_url, get_commercial_truck_directions_url, verify_commercial_route_restrictions, geocode};

pub use notifications::send_external_notification;
pub use public_lead::submit_public_booking_lead;

pub use inspections::record_damage_inspection;
pub use inspections::get_job_damage_inspections;
pub use inspections::acknowledge_damage_inspection_by_client;
pub use inspections::delete_damage_inspection;

pub use media::enqueue_offline_media_blob;
pub use media::get_offline_media_pointer;
pub use media::sync_pending_offline_media_blobs;

pub use claims::submit_damaged_item_claim;
pub use claims::update_claim_status;
pub use claims::process_claim_payout;
pub use claims::get_job_claims;

pub use moves::accept_move_quote_with_deposit;
pub use notifications::get_customer_live_tracking_portal;

