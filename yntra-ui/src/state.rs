use dioxus::prelude::*;
use yntra_core::{Workspace, WorkspaceUser, Team, TeamEvent, MessageItem, DailyNote, TimeReport, ClientProfile, ReportItem};

#[derive(Clone, Copy)]
pub struct AppState {
    pub db_trigger: Signal<u32>,
    pub active_user_id: Signal<String>,
    pub active_section: Signal<String>,
    pub needs_setup: Signal<bool>,
    pub two_factor_user: Signal<Option<WorkspaceUser>>,
    pub logged_in: Signal<bool>,
    pub login_tab: Signal<String>,
    pub show_bankid_modal: Signal<bool>,
    pub scanning_state: Signal<String>,
    pub login_email: Signal<String>,
    pub login_password: Signal<String>,
    pub login_error: Signal<Option<String>>,
    pub auth_region: Signal<String>,
    
    // View state sub-signals
    pub selected_note_team_id: Signal<String>,
    pub selected_note_id: Signal<Option<String>>,
    pub is_note_composing: Signal<bool>,
    pub selected_client_id: Signal<String>,
    
    // Calendar signals
    pub calendar_year: Signal<i32>,
    pub calendar_month: Signal<u32>,
    pub selected_calendar_date: Signal<String>,
    
    // Form inputs
    pub note_subject: Signal<String>,
    pub note_content: Signal<String>,
    pub event_title: Signal<String>,
    pub event_team: Signal<String>,
    pub event_assignee: Signal<String>,
    pub event_recipient: Signal<String>,
    pub event_start: Signal<String>,
    pub event_end: Signal<String>,
    
    pub time_date: Signal<String>,
    pub time_start: Signal<String>,
    pub time_end: Signal<String>,
    pub time_hours: Signal<String>,
    pub time_note: Signal<String>,
    pub time_view_tab: Signal<String>,
    pub time_filter_status: Signal<String>,
    pub selected_time_reports: Signal<Vec<String>>,
    pub time_search_query: Signal<String>,
    
    pub journal_content: Signal<String>,
    pub med_name: Signal<String>,
    pub med_dosage: Signal<String>,
    pub med_frequency: Signal<String>,
    pub med_instructions: Signal<String>,
    
    // Reporting signals
    pub report_tab: Signal<String>,
    pub report_type: Signal<String>,
    pub report_subject: Signal<String>,
    pub report_description: Signal<String>,
    pub report_date: Signal<String>,
    pub report_is_anonymous: Signal<bool>,
    pub report_status_filter: Signal<String>,
    pub report_type_filter: Signal<String>,
    pub selected_report_id: Signal<Option<String>>,
    pub show_report_details_modal: Signal<bool>,
    
    // Directory signals
    pub directory_level: Signal<String>,
    pub selected_directory_workspace: Signal<String>,
    pub selected_directory_team: Signal<Option<String>>,
    pub show_add_team_modal: Signal<bool>,
    pub show_invite_member_modal: Signal<bool>,
    pub show_client_manager_modal: Signal<bool>,
    pub new_team_name: Signal<String>,
    pub new_member_email: Signal<String>,
    pub new_member_name: Signal<String>,
    pub new_member_role: Signal<String>,
    pub new_client_first_name: Signal<String>,
    pub new_client_last_name: Signal<String>,
    pub new_client_personal_number: Signal<String>,
    pub new_client_care_level: Signal<String>,
    
    // Settings signals
    pub settings_tab: Signal<String>,
    pub settings_name: Signal<String>,
    pub settings_brand_color: Signal<String>,
    pub settings_logo_url: Signal<String>,
    pub settings_save_status: Signal<String>,
    
    // Account signals
    pub account_name: Signal<String>,
    pub account_phone: Signal<String>,
    pub account_preferences: Signal<String>,
    pub account_save_status: Signal<String>,
    pub last_synced_user_id: Signal<String>,
    
    // Leave request signals
    pub scheduling_sidebar_tab: Signal<String>,
    pub leave_type: Signal<String>,
    pub leave_start: Signal<String>,
    pub leave_end: Signal<String>,
    pub leave_reason: Signal<String>,
    pub leave_save_status: Signal<String>,
    
    // UI layout / search state signals
    pub globalsearch_open: Signal<bool>,
    pub header_profile_open: Signal<bool>,
    pub time_group_expanded: Signal<bool>,
    pub filter_categories: Signal<Vec<String>>,
    
    // Messaging signals
    pub messaging_view_tab: Signal<String>,
    pub active_message_id: Signal<Option<String>>,
    pub compose_recipient_id: Signal<Option<String>>,
    pub compose_subject: Signal<String>,
    pub compose_body: Signal<String>,
    pub compose_status: Signal<String>,

    // Resolved database query signals
    pub workspace: Resource<Workspace>,
    pub users: Resource<Vec<WorkspaceUser>>,
    pub teams: Resource<Vec<Team>>,
    pub events: Resource<Vec<TeamEvent>>,
    pub messages: Resource<Vec<MessageItem>>,
    pub notes: Resource<Vec<DailyNote>>,
    pub time_reports: Resource<Vec<TimeReport>>,
    pub clients: Resource<Vec<ClientProfile>>,
    pub reports: Resource<Vec<ReportItem>>,
    pub workspaces: Resource<Vec<Workspace>>,
}
