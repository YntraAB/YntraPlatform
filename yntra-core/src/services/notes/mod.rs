mod crud;
mod crypto;
mod search;
mod store;
mod sync;

pub mod crdt;

#[cfg(test)]
mod tests;

pub use crypto::verify_zkp_if_encrypted;

pub use store::get_note_store;
pub use store::load_notes_from_opfs;
pub use store::load_notes_from_opfs_internal;

pub use sync::apply_note_loro_update;
pub use sync::get_note_loro_state;
pub use sync::merge_loro_notes;
pub use sync::merge_unmerged_notes;

pub use search::search_notes;

pub use crud::add_note;
pub use crud::delete_note;
pub use crud::get_note_by_id;
pub use crud::get_notes;
pub use crud::get_notes_rkyv;
pub use crud::update_note;
