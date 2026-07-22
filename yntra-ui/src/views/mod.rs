pub mod client_portal;
pub mod dashboard;
pub mod directory;
pub mod jobs;
pub mod login;
pub mod messaging;
pub mod notes;
pub mod reporting;
pub mod scheduling;
pub mod settings;
pub mod setup;
pub mod time;
pub mod todos;

pub use client_portal::ClientPortalView;
pub use dashboard::DashboardView;
pub use directory::DirectoryView;
pub use jobs::{BookingWidgetView, DispatchView, FleetView, JobsView, LiveMapView, RutExportsView};
pub use login::LoginView;
pub use messaging::MessagingView;
pub use notes::NotesView;
pub use reporting::ReportingView;
pub use scheduling::SchedulingView;
pub use settings::SettingsView;
pub use setup::SetupView;
pub use time::TimeView;
pub use todos::TodosView;
pub mod dynamic_block;
pub use dynamic_block::DynamicBlockView;

pub mod school;
pub use school::{AcademicsView, AttendanceView, FinanceView, LibraryView, StudentDirectoryView, HealthClinicView, ReportCardsView};

pub mod care;
pub use care::{JournalsView, MedicationsView};
