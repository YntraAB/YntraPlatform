mod pnum;
mod profile;

pub use pnum::{normalize_swedish_pnum, personal_numbers_match};
pub use profile::{
    add_client_via_directory, delete_client, get_clients, get_clients_rkyv, update_client_profile,
};
