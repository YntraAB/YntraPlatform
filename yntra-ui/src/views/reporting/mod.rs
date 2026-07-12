use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::WorkspaceUser;

pub mod details_dialog;
pub mod list;
pub mod send;

use details_dialog::ReportDetailsDialog;

#[derive(Props, Clone)]
pub struct ReportingViewProps {
    pub active_user: WorkspaceUser,
    pub report_tab: Signal<String>,
    pub report_status_filter: Signal<String>,
    pub report_type_filter: Signal<String>,
    pub report_type: Signal<String>,
    pub report_date: Signal<String>,
    pub report_subject: Signal<String>,
    pub report_description: Signal<String>,
    pub report_is_anonymous: Signal<bool>,
    pub selected_report_id: Signal<Option<String>>,
    pub show_report_details_modal: Signal<bool>,
}

impl PartialEq for ReportingViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn ReportingView(props: ReportingViewProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let active_user = props.active_user;
    let users = state.users.read().clone().unwrap_or_default();
    let reports = state.reports.read().clone().unwrap_or_default();

    let mut report_tab = props.report_tab;
    let report_status_filter = props.report_status_filter;
    let report_type_filter = props.report_type_filter;
    let report_type = props.report_type;
    let report_date = props.report_date;
    let report_subject = props.report_subject;
    let report_description = props.report_description;
    let report_is_anonymous = props.report_is_anonymous;
    let selected_report_id = props.selected_report_id;
    let show_report_details_modal = props.show_report_details_modal;

    let is_manager = active_user.role == "platform_admin" || active_user.role == "admin";
    let user_prefs: serde_json::Value = serde_json::from_str(&active_user.preferences).unwrap_or_default();
    let region = user_prefs.get("language").and_then(|l| l.as_str()).unwrap_or("US").to_string();
    let current_tab = report_tab.read().clone();

    let has_details_report = selected_report_id.read().clone();
    let details_report = reports
        .iter()
        .find(|r| Some(r.id.clone()) == has_details_report)
        .cloned();

    rsx! {
        div {
            class: "mx-auto w-full max-w-5xl",
            style: "padding: 2rem; display: flex; flex-direction: column; gap: 1.5rem; box-sizing: border-box;",
            div {
                class: "flex items-center gap-3 p-4 rounded-xl",
                style: "background:rgba(16, 185, 129, 0.1); border:1px solid rgba(16, 185, 129, 0.2);",
                span {
                    class: "text-xl font-bold",
                    style: "color:var(--success);",
                    "🛡"
                }
                p {
                    class: "m-0 text-sm font-medium",
                    style: "color:hsl(158.1, 64.4%, 51.6%); line-height:1.4;",
                    { t("reporting-confidentiality-notice", &region) }
                }
            }

            div { class: "login-tabs w-80 mb-6", 
                button {
                    class: if current_tab == "send" { "login-tab-btn active" } else { "login-tab-btn" },
                    onclick: move |_| report_tab.set("send".to_string()),
                    { t("reporting-tabs-send", &region) }
                }
                button {
                    class: if current_tab == "list" { "login-tab-btn active" } else { "login-tab-btn" },
                    onclick: move |_| report_tab.set("list".to_string()),
                    if is_manager {
                        { t("reporting-tabs-all-reports", &region) }
                    } else {
                        { t("reporting-tabs-my-reports", &region) }
                    }
                }
            }

            if current_tab == "send" {
                send::ReportSubmitForm {
                    active_user: active_user.clone(),
                    region: region.clone(),
                    report_type,
                    report_date,
                    report_subject,
                    report_description,
                    report_is_anonymous,
                    report_tab,
                }
            } else {
                list::ReportsList {
                    reports,
                    region: region.clone(),
                    report_status_filter,
                    report_type_filter,
                    selected_report_id,
                    show_report_details_modal,
                }
            }
        }

        if let Some(rep) = details_report {
            ReportDetailsDialog {
                show_report_details_modal,
                details_report: rep,
                users,
                active_user,
                region,
                is_manager,
            }
        }
    }
}
