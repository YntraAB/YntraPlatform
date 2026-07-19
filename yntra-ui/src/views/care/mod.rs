use dioxus::prelude::*;

pub mod journals;
pub mod medications;

pub use journals::JournalsView;
pub use medications::MedicationsView;

#[derive(Props, Clone, PartialEq)]
pub struct CareViewProps {
    pub active_user_id: String,
    pub workspace_id: String,
    pub block_id: String,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}
