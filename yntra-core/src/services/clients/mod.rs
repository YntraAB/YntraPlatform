mod pnum;
mod profile;

pub use pnum::{normalize_swedish_pnum, personal_numbers_match};
pub use profile::{get_clients, get_clients_rkyv, add_client_via_directory, update_client_profile, delete_client};
