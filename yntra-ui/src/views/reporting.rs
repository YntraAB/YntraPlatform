use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::ReportItem;
use yntra_core::WorkspaceUser;
use yntra_core::{add_report, update_report_status};

#[derive(Props, Clone)]
pub struct ReportingViewProps {
    pub active_user: WorkspaceUser,
    pub users: Vec<WorkspaceUser>,
    pub reports: Vec<ReportItem>,
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
    let active_user = props.active_user;
    let users = props.users.clone();
    let reports = props.reports.clone();

    let mut report_tab = props.report_tab;
    let mut report_status_filter = props.report_status_filter;
    let mut report_type_filter = props.report_type_filter;
    let mut report_type = props.report_type;
    let mut report_date = props.report_date;
    let mut report_subject = props.report_subject;
    let mut report_description = props.report_description;
    let mut report_is_anonymous = props.report_is_anonymous;
    let mut selected_report_id = props.selected_report_id;
    let mut show_report_details_modal = props.show_report_details_modal;
    let mut report_type_open = use_signal(|| false);
    let mut status_filter_open = use_signal(|| false);
    let mut type_filter_open = use_signal(|| false);

    let is_manager = active_user.role == "platform_admin" || active_user.role == "admin";
    let user_prefs: serde_json::Value = serde_json::from_str(&active_user.preferences).unwrap_or_default();
    let region = user_prefs.get("language").and_then(|l| l.as_str()).unwrap_or("US").to_string();
    let current_tab = report_tab.read().clone();

    let total_reports = reports.len();
    let pending_reports = reports.iter().filter(|r| r.status != "resolved" && r.status != "reviewed").count();
    let resolved_reports = reports.iter().filter(|r| r.status == "resolved").count();
    let current_status_filter = report_status_filter.read().clone();
    let current_type_filter = report_type_filter.read().clone();
    let mut filtered_reports = reports.clone();

    if current_status_filter != "all" {
        filtered_reports.retain(|r| r.status == current_status_filter);
    }
    if current_type_filter != "all" {
        filtered_reports.retain(|r| r.type_name == current_type_filter);
    }

    let has_details_report = selected_report_id.read().clone();
    let details_report = reports
        .iter()
        .find(|r| Some(r.id.clone()) == has_details_report)
        .cloned();

    rsx! {
        div {
            class: "mx-auto w-full max-w-5xl",
            style: "padding: 2rem; display: flex; flex-direction: column; gap: 1.5rem; box-sizing: border-box;",
            div { class: "flex items-center gap-3 p-4 rounded-xl",
                    style: "background:rgba(16, 185, 129, 0.1); border:1px solid rgba(16, 185, 129, 0.2);",
            span { class: "text-xl font-bold",
                    style: "color:var(--success);",
                "🛡"
            }
            p { class: "m-0 text-sm font-medium",
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
            div { class: "dashboard-card", style: "max-width: 600px;",
                h3 { class: "mt-0 mb-2",
                    { t("reporting-form-create-title", &region) }
                }
                p { class: "mt-0 mb-6 text-muted-foreground text-sm",
                    { t("reporting-form-create-desc", &region) }
                }

                div { class: "flex flex-col gap-4",
                    div { class: "grid gap-4",
                    style: "grid-template-columns: 1fr 1fr;",
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-xs font-bold text-muted-foreground",
                                { t("reporting-form-type-label", &region) }
                            }
                            {
                                let report_type_label = match report_type.read().as_str() {
                                    "complaint" => t("reporting-types-complaint", &region),
                                    "work_injury" => t("reporting-types-work-injury", &region),
                                    "incident" => t("reporting-types-incident", &region),
                                    "deviation" => t("reporting-types-deviation", &region),
                                    "whistleblower" => t("reporting-types-whistleblower", &region),
                                    _ => t("reporting-form-select-type", &region),
                                };

                                rsx! {
                                    components::Dropdown {
                                        label: report_type_label,
                                        open: *report_type_open.read(),
                                        ontoggle: move |_| {
                                            let cur = *report_type_open.read();
                                            report_type_open.set(!cur);
                                        },
                                        components::DropdownItem {
                                            label: t("reporting-types-complaint", &region),
                                            onclick: move |_| {
                                                report_type.set("complaint".to_string());
                                                report_type_open.set(false);
                                            }
                                        }
                                        components::DropdownItem {
                                            label: t("reporting-types-work-injury", &region),
                                            onclick: move |_| {
                                                report_type.set("work_injury".to_string());
                                                report_type_open.set(false);
                                            }
                                        }
                                        components::DropdownItem {
                                            label: t("reporting-types-incident", &region),
                                            onclick: move |_| {
                                                report_type.set("incident".to_string());
                                                report_type_open.set(false);
                                            }
                                        }
                                        components::DropdownItem {
                                            label: t("reporting-types-deviation", &region),
                                            onclick: move |_| {
                                                report_type.set("deviation".to_string());
                                                report_type_open.set(false);
                                            }
                                        }
                                        components::DropdownItem {
                                            label: t("reporting-types-whistleblower", &region),
                                            onclick: move |_| {
                                                report_type.set("whistleblower".to_string());
                                                report_type_open.set(false);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-xs font-bold text-muted-foreground",
                                { t("reporting-form-date-label", &region) }
                            }
                            input {
                                class: "yntra-input",
                                r#type: "date",
                                value: "{report_date}",
                                oninput: move |e| report_date.set(e.value()),
                            }
                        }
                    }

                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground",
                            { t("reporting-form-subject", &region) }
                        }
                        input {
                            class: "yntra-input",
                            value: "{report_subject}",
                            placeholder: t("reporting-form-subject-help", &region),
                            oninput: move |e| report_subject.set(e.value()),
                        }
                    }

                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground",
                            { t("reporting-form-description", &region) }
                        }
                        textarea {
                            class: "yntra-input",
                            style: "height:120px; resize:none;",
                            value: "{report_description}",
                            placeholder: t("reporting-form-description-placeholder", &region),
                            oninput: move |e| report_description.set(e.value()),
                        }
                    }

                    div { class: "p-3 bg-white/[0.02] border border-border rounded-lg flex items-center gap-2",
                        components::Checkbox {
                            checked: *report_is_anonymous.read(),
                            onchange: move |val| report_is_anonymous.set(val),
                            label: t("reporting-form-is-anonymous", &region),
                        }
                    }

                    button {
                        class: "yntra-btn",
                        onclick: {
                            let active_user_c = active_user.clone();
                            move |_| {
                                let sub = report_subject.read().trim().to_string();
                                let desc = report_description.read().trim().to_string();
                                if !sub.is_empty() && !desc.is_empty() {
                                    let workspace_id = active_user_c.workspace_id.clone().unwrap_or_else(|| "workspace-1".to_string());
                                    let user_id = active_user_c.id.clone();
                                    let r_type = report_type.read().clone();
                                    let is_anon = *report_is_anonymous.read();
                                    let date_val = report_date.read().clone();
                                    spawn(async move {
                                        if let Ok(report) = add_report(
                                            workspace_id,
                                            user_id,
                                            r_type,
                                            is_anon,
                                            sub,
                                            desc,
                                            date_val,
                                        ).await {
                                            if is_anon {
                                                let js = format!(
                                                    r#"
                                                    try {{
                                                        let ids = JSON.parse(localStorage.getItem("yntra_anon_report_ids") || "[]");
                                                        ids.push("{}");
                                                        localStorage.setItem("yntra_anon_report_ids", JSON.stringify(ids));
                                                    }} catch(e) {{}}
                                                    "#,
                                                    report.id
                                                );
                                                let _ = dioxus::document::eval(&js);
                                            }
                                        }
                                    });
                                    report_subject.set(String::new());
                                    report_description.set(String::new());
                                    report_is_anonymous.set(false);
                                    report_tab.set("list".to_string());
                                }
                            }
                        },
                        { t("reporting-form-submit", &region) }
                    }
                }
            }
        } else {
            div { class: "flex flex-col gap-6",
                // Stats Grid
                div { class: "grid grid-cols-3 gap-4",
                    // Card 1: Total
                    div { class: "bg-white/[0.02] border border-border p-5 rounded-xl flex flex-col gap-2",
                        div { class: "flex justify-between items-center",
                            span { class: "text-xs font-bold text-muted-foreground uppercase", { t("reporting-stats-total", &region) } }
                            components::LucideIcon { name: "file-text", class: "h-4 w-4 text-muted", }
                        }
                        span { class: "font-extrabold text-foreground",
                    style: "font-size:1.8rem;", "{total_reports}" }
                    }

                    // Card 2: Pending
                    div { class: "bg-white/[0.02] border border-border p-5 rounded-xl flex flex-col gap-2",
                        div { class: "flex justify-between items-center",
                            span { class: "text-xs font-bold text-muted-foreground uppercase", { t("reporting-stats-awaiting-review", &region) } }
                            components::LucideIcon { name: "clock", class: "h-4 w-4 text-warning", }
                        }
                        span { class: "font-extrabold",
                    style: "font-size:1.8rem; color:var(--warning);", "{pending_reports}" }
                    }

                    // Card 3: Resolved
                    div { class: "bg-white/[0.02] border border-border p-5 rounded-xl flex flex-col gap-2",
                        div { class: "flex justify-between items-center",
                            span { class: "text-xs font-bold text-muted-foreground uppercase", { t("reporting-status-resolved", &region) } }
                            components::LucideIcon { name: "check-circle", class: "h-4 w-4 text-success", }
                        }
                        span { class: "font-extrabold",
                    style: "font-size:1.8rem; color:var(--success);", "{resolved_reports}" }
                    }
                }

                div { class: "dashboard-card",
                    div { class: "flex justify-between items-center mb-4",
                    div { class: "flex gap-3",
                        {
                            let status_filter_label = match report_status_filter.read().as_str() {
                                "pending" => t("reporting-stats-awaiting-review", &region),
                                "reviewed" => t("reporting-status-reviewed", &region),
                                "resolved" => t("reporting-status-resolved", &region),
                                _ => t("reporting-filter-all-statuses", &region),
                            };

                            rsx! {
                                components::Dropdown {
                                    label: status_filter_label,
                                    open: *status_filter_open.read(),
                                    ontoggle: move |_| {
                                        let cur = *status_filter_open.read();
                                        status_filter_open.set(!cur);
                                    },
                                    components::DropdownItem {
                                        label: t("reporting-filter-all-statuses", &region),
                                        onclick: move |_| {
                                            report_status_filter.set("all".to_string());
                                            status_filter_open.set(false);
                                        }
                                    }
                                    components::DropdownItem {
                                        label: t("reporting-stats-awaiting-review", &region),
                                        onclick: move |_| {
                                            report_status_filter.set("pending".to_string());
                                            status_filter_open.set(false);
                                        }
                                    }
                                    components::DropdownItem {
                                        label: t("reporting-status-reviewed", &region),
                                        onclick: move |_| {
                                            report_status_filter.set("reviewed".to_string());
                                            status_filter_open.set(false);
                                        }
                                    }
                                    components::DropdownItem {
                                        label: t("reporting-status-resolved", &region),
                                        onclick: move |_| {
                                            report_status_filter.set("resolved".to_string());
                                            status_filter_open.set(false);
                                        }
                                    }
                                }
                            }
                        }
                        {
                            let type_filter_label = match report_type_filter.read().as_str() {
                                "complaint" => t("reporting-types-complaint", &region),
                                "work_injury" => t("reporting-types-work-injury", &region),
                                "incident" => t("reporting-types-incident", &region),
                                "deviation" => t("reporting-types-deviation", &region),
                                "whistleblower" => t("reporting-types-whistleblower", &region),
                                _ => t("reporting-filter-all-types", &region),
                            };

                            rsx! {
                                components::Dropdown {
                                    label: type_filter_label,
                                    open: *type_filter_open.read(),
                                    ontoggle: move |_| {
                                        let cur = *type_filter_open.read();
                                        type_filter_open.set(!cur);
                                    },
                                    components::DropdownItem {
                                        label: t("reporting-filter-all-types", &region),
                                        onclick: move |_| {
                                            report_type_filter.set("all".to_string());
                                            type_filter_open.set(false);
                                        }
                                    }
                                    components::DropdownItem {
                                        label: t("reporting-types-complaint", &region),
                                        onclick: move |_| {
                                            report_type_filter.set("complaint".to_string());
                                            type_filter_open.set(false);
                                        }
                                    }
                                    components::DropdownItem {
                                        label: t("reporting-types-work-injury", &region),
                                        onclick: move |_| {
                                            report_type_filter.set("work_injury".to_string());
                                            type_filter_open.set(false);
                                        }
                                    }
                                    components::DropdownItem {
                                        label: t("reporting-types-incident", &region),
                                        onclick: move |_| {
                                            report_type_filter.set("incident".to_string());
                                            type_filter_open.set(false);
                                        }
                                    }
                                    components::DropdownItem {
                                        label: t("reporting-types-deviation", &region),
                                        onclick: move |_| {
                                            report_type_filter.set("deviation".to_string());
                                            type_filter_open.set(false);
                                        }
                                    }
                                    components::DropdownItem {
                                        label: t("reporting-types-whistleblower", &region),
                                        onclick: move |_| {
                                            report_type_filter.set("whistleblower".to_string());
                                            type_filter_open.set(false);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                table { class: "yntra-table",
                    thead {
                        tr {
                            th { { t("reporting-list-type", &region) } }
                            th { { t("reporting-list-reporter", &region) } }
                            th { { t("reporting-form-subject", &region) } }
                            th { { t("reporting-list-date", &region) } }
                            th { { t("reporting-list-status", &region) } }
                            th { { t("reporting-list-details", &region) } }
                        }
                    }
                    tbody {
                        if filtered_reports.is_empty() {
                            tr {
                                td {
                                    colspan: "6",
                                    class: "text-center p-8 text-muted-foreground/60",
                                    { t("reporting-list-no-records", &region) }
                                }
                            }
                        }
                        for r in filtered_reports.iter() {
                            {
                                let r_id = r.id.clone();
                                let content_val: serde_json::Value = serde_json::from_str(&r.content)
                                    .unwrap_or_default();
                                let subject = content_val
                                    .get("subject")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| t("reporting-no-subject", &region));
                                let date_val = content_val
                                    .get("date_of_incident")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(&r.created_at);
                                let reporter_name = if r.is_anonymous {
                                    { t("reporting-anonymous", &region) }
                                } else {
                                    users
                                        .iter()
                                        .find(|u| u.id == r.user_id)
                                        .and_then(|u| u.full_name.clone())
                                        .unwrap_or_else(|| t("reporting-unknown", &region))
                                };
                                let display_type = match r.type_name.as_str() {
                                    "complaint" => t("reporting-types-complaint", &region),
                                    "work_injury" => t("reporting-types-work-injury", &region),
                                    "incident" => t("reporting-types-incident", &region),
                                    "deviation" => t("reporting-types-deviation", &region),
                                    "whistleblower" => t("reporting-types-whistleblower", &region),
                                    _ => r.type_name.clone(),
                                };
                                rsx! {
                                    tr {
                                        td { class: "font-bold",
                     style: "text-transform:capitalize;", "{display_type}" }
                                        td { class: if r.is_anonymous { "text-muted" } else { "" }, "{reporter_name}" }
                                        td { "{subject}" }
                                        td { "{date_val}" }
                                        td {
                                            match r.status.as_str() {
                                                "resolved" => rsx! {
                                                    span { class: "font-bold",
                     style: "color:var(--success);", { t("reporting-status-resolved", &region) } }
                                                },
                                                "reviewed" => rsx! {
                                                    span { class: "text-primary font-bold", { t("reporting-status-reviewed", &region) } }
                                                },
                                                _ => rsx! {
                                                    span { class: "font-bold",
                     style: "color:var(--warning);", { t("reporting-status-pending-label", &region) } }
                                                },
                                            }
                                        }
                                        td {
                                            button {
                                                class: "yntra-btn text-xs",
                                                style: "padding:0.3rem 0.6rem;",
                                                onclick: move |_| {
                                                    selected_report_id.set(Some(r_id.clone()));
                                                    show_report_details_modal.set(true);
                                                },
                                                { t("common-view", &region) }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

        if let Some(rep) = details_report {
            {
                let rep_id = rep.id.clone();
                let rep_type = rep.type_name.clone();
                let content_val: serde_json::Value = serde_json::from_str(&rep.content)
                    .unwrap_or_default();
                let subject = content_val
                    .get("subject")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| t("reporting-no-subject", &region));
                let desc = content_val.get("description").and_then(|v| v.as_str()).unwrap_or("");
                let date_val = content_val
                    .get("date_of_incident")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let reporter_name = if rep.is_anonymous {
                    { t("reporting-anonymous", &region) }
                } else {
                    users
                        .iter()
                        .find(|u| u.id == rep.user_id)
                        .and_then(|u| u.full_name.clone())
                        .unwrap_or_else(|| t("reporting-unknown", &region))
                };
                let display_type_caps = match rep_type.as_str() {
                    "complaint" => t("reporting-types-complaint", &region),
                    "work_injury" => t("reporting-types-work-injury", &region),
                    "incident" => t("reporting-types-incident", &region),
                    "deviation" => t("reporting-types-deviation", &region),
                    "whistleblower" => t("reporting-types-whistleblower", &region),
                    _ => rep_type.clone(),
                }.to_uppercase();
                let details_title = format!("{}{}", t("reporting-details-title-prefix", &region), display_type_caps);
                rsx! {
                    components::Dialog {
                        open: *show_report_details_modal.read(),
                        title: details_title,
                        onclose: move |_| show_report_details_modal.set(false),
                        div { class: "flex flex-col gap-5 text-sm",
                            div { class: "grid gap-4",
                    style: "grid-template-columns:1fr 1fr;",
                                div {
                                    div { class: "text-muted-foreground/60 text-xs font-bold uppercase",
                                        { t("reporting-list-reporter", &region) }
                                    }
                                    div { class: "font-bold text-foreground",
                     style: "margin-top:0.2rem;",
                                        "{reporter_name}"
                                    }
                                }
                                div {
                                    div { class: "text-muted-foreground/60 text-xs font-bold uppercase",
                                        { t("reporting-form-date-of-incident", &region) }
                                    }
                                    div { class: "font-bold text-foreground",
                     style: "margin-top:0.2rem;",
                                        "{date_val}"
                                    }
                                }
                            }
                            div {
                                div { class: "text-muted-foreground/60 text-xs font-bold uppercase",
                                    { t("reporting-form-subject", &region) }
                                }
                                div { class: "font-bold",
                     style: "color:hsl(229.7, 93.5%, 81.8%); margin-top:0.2rem; font-size:1.05rem;",
                                    "{subject}"
                                }
                            }
                            div {
                                div { class: "text-muted-foreground/60 text-xs font-bold uppercase",
                     style: "margin-bottom:0.3rem;",
                                    { t("reporting-form-description", &region) }
                                }
                                div { class: "border border-border p-4 rounded-lg text-muted-foreground overflow-y-auto",
                     style: "background:rgba(0,0,0,0.25); line-height:1.45; white-space:pre-wrap; max-height:180px;",
                                    "{desc}"
                                }
                            }

                            if is_manager {
                                {
                                    let rep_id_resolve = rep_id.clone();
                                    let rep_id_review = rep_id.clone();
                                    rsx! {
                                        div { class: "border-t border-border pt-4 mt-2",
                                            div { class: "text-muted-foreground/60 text-xs font-bold uppercase mb-2",
                                                { t("reporting-admin-manage-status", &region) }
                                            }
                                            div { class: "flex gap-2",
                                                button {
                                                    class: "yntra-btn text-xs",
                                                    style: "background:var(--success); padding:0.4rem 0.8rem;",
                                                    onclick: {
                                                        let ws_uid = active_user.id.clone();
                                                        let r_id = rep_id_resolve.clone();
                                                        move |_| {
                                                            let ws_uid_c = ws_uid.clone();
                                                            let r_id_c = r_id.clone();
                                                            spawn(async move {
                                                                let _ = update_report_status(ws_uid_c, r_id_c, "resolved".to_string()).await;
                                                            });
                                                            show_report_details_modal.set(false);
                                                        }
                                                    },
                                                    { t("reporting-admin-mark-resolved", &region) }
                                                }
                                                button {
                                                    class: "yntra-btn text-xs",
                                                    style: "background:var(--accent); padding:0.4rem 0.8rem;",
                                                    onclick: {
                                                        let ws_uid = active_user.id.clone();
                                                        let r_id = rep_id_review.clone();
                                                        move |_| {
                                                            let ws_uid_c = ws_uid.clone();
                                                            let r_id_c = r_id.clone();
                                                            spawn(async move {
                                                                let _ = update_report_status(ws_uid_c, r_id_c, "reviewed".to_string()).await;
                                                            });
                                                            show_report_details_modal.set(false);
                                                        }
                                                    },
                                                    { t("reporting-admin-mark-reviewed", &region) }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
    }
    }
    }
}
