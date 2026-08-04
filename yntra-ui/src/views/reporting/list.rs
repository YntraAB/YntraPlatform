use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{ReportItem, update_report_status};

#[derive(Props, Clone, PartialEq)]
pub struct ReportsListProps {
    pub reports: Vec<ReportItem>,
    pub region: String,
    pub report_status_filter: Signal<String>,
    pub report_type_filter: Signal<String>,
    pub selected_report_id: Signal<Option<String>>,
    pub show_report_details_modal: Signal<bool>,
}

#[component]
pub fn ReportsList(props: ReportsListProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let requester_id = state.active_user_id.read().clone();
    let is_manager = *state.active_user_role.read() == "admin" || *state.active_user_role.read() == "platform_admin";

    let reports = props.reports.clone();
    let region = props.region;

    let mut report_status_filter = props.report_status_filter;
    let mut report_type_filter = props.report_type_filter;
    let mut selected_report_id = props.selected_report_id;
    let mut show_report_details_modal = props.show_report_details_modal;

    let mut context_menu_open = use_signal(|| false);
    let mut context_menu_pos = use_signal(|| (0, 0));
    let mut context_menu_report = use_signal(|| Option::<ReportItem>::None);

    let mut status_filter_open = use_signal(|| false);
    let mut type_filter_open = use_signal(|| false);

    let total_reports = reports.len();
    let pending_reports = reports
        .iter()
        .filter(|r| r.status != "resolved" && r.status != "reviewed")
        .count();
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

    rsx! {
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
                        span { class: "text-xs font-bold text-muted-foreground uppercase", { t("reporting-stats-resolved", &region) } }
                        components::LucideIcon { name: "check-circle", class: "h-4 w-4 text-success", }
                    }
                    span { class: "font-extrabold",
                style: "font-size:1.8rem; color:var(--success);", "{resolved_reports}" }
                }
            }

            // Filters Toolbar
            div { class: "flex gap-3 justify-end items-center mb-2",
                span { class: "text-xs font-bold text-muted-foreground uppercase mr-2", { t("reporting-filters-label", &region) } }

                {
                    let current_status_label = match current_status_filter.as_str() {
                        "pending" => t("reporting-status-pending", &region),
                        "reviewed" => t("reporting-status-reviewed", &region),
                        "resolved" => t("reporting-status-resolved", &region),
                        _ => t("reporting-status-all", &region),
                    };

                    rsx! {
                        components::Dropdown {
                            label: current_status_label,
                            open: *status_filter_open.read(),
                            ontoggle: move |_| {
                                let cur = *status_filter_open.read();
                                status_filter_open.set(!cur);
                            },
                            components::DropdownItem {
                                label: t("reporting-status-all", &region),
                                onclick: move |_| {
                                    report_status_filter.set("all".to_string());
                                    status_filter_open.set(false);
                                }
                            }
                            components::DropdownItem {
                                label: t("reporting-status-pending", &region),
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
                    let current_type_label = match current_type_filter.as_str() {
                        "complaint" => t("reporting-types-complaint", &region),
                        "work_injury" => t("reporting-types-work-injury", &region),
                        "incident" => t("reporting-types-incident", &region),
                        "deviation" => t("reporting-types-deviation", &region),
                        "whistleblower" => t("reporting-types-whistleblower", &region),
                        _ => t("reporting-type-all", &region),
                    };

                    rsx! {
                        components::Dropdown {
                            label: current_type_label,
                            open: *type_filter_open.read(),
                            ontoggle: move |_| {
                                let cur = *type_filter_open.read();
                                type_filter_open.set(!cur);
                            },
                            components::DropdownItem {
                                label: t("reporting-type-all", &region),
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

            // Table
            div { class: "dashboard-card w-full", style: "padding:0; overflow:hidden;",
                table { class: "yntra-table w-full text-sm",
                    thead {
                        tr {
                            th { { t("reporting-list-type", &region) } }
                            th { { t("reporting-list-subject", &region) } }
                            th { { t("reporting-list-date", &region) } }
                            th { { t("reporting-list-status", &region) } }
                            th { style: "width:80px;" }
                        }
                    }
                    tbody {
                        if filtered_reports.is_empty() {
                            tr {
                                td {
                                    colspan: 5,
                                    style: "text-align:center; padding:3rem; color:var(--text-muted);",
                                    { t("reporting-list-empty", &region) }
                                }
                            }
                        } else {
                            for r in filtered_reports.iter() {
                                {
                                    let r_id = r.id.clone();
                                    let r_type = r.type_name.clone();
                                    let r_content = r.content.clone();
                                    let r_status = r.status.clone();
                                    let content: serde_json::Value = serde_json::from_str(&r_content).unwrap_or_default();
                                    let sub = content.get("subject").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| t("reporting-no-subject", &region));
                                    let date_str = content.get("date_of_incident").and_then(|v| v.as_str()).unwrap_or("");

                                    let type_badge_label = match r_type.as_str() {
                                        "complaint" => t("reporting-types-complaint", &region),
                                        "work_injury" => t("reporting-types-work-injury", &region),
                                        "incident" => t("reporting-types-incident", &region),
                                        "deviation" => t("reporting-types-deviation", &region),
                                        "whistleblower" => t("reporting-types-whistleblower", &region),
                                        _ => r_type.clone(),
                                    };

                                    let status_badge_label = match r_status.as_str() {
                                        "pending" => t("reporting-status-pending", &region),
                                        "reviewed" => t("reporting-status-reviewed", &region),
                                        "resolved" => t("reporting-status-resolved", &region),
                                        _ => r_status.clone(),
                                    };

                                    let status_class = match r_status.as_str() {
                                        "resolved" => "yntra-badge success",
                                        "reviewed" => "yntra-badge warning",
                                        _ => "yntra-badge warning",
                                    };

                                    let r_c = r.clone();
                                    rsx! {
                                        tr {
                                            key: "{r_id}",
                                            oncontextmenu: move |evt| {
                                                evt.prevent_default();
                                                let coords = evt.client_coordinates();
                                                context_menu_pos.set((coords.x as i32, coords.y as i32));
                                                context_menu_report.set(Some(r_c.clone()));
                                                context_menu_open.set(true);
                                            },
                                            td {
                                                span { class: "yntra-badge", "{type_badge_label}" }
                                            }
                                            td { class: "font-semibold", "{sub}" }
                                            td { "{date_str}" }
                                            td {
                                                span { class: "{status_class}", "{status_badge_label}" }
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

            // Context Menu Overlay
            if let Some(rep) = context_menu_report.read().clone() {
                {
                    let _rep_id = rep.id.clone();
                    let rep_status = rep.status.clone();
                    let is_resolved = rep_status == "resolved";
                    let is_reviewed = rep_status == "reviewed";

                    let req_id_c = requester_id.clone();
                    let rep_id_c = rep.id.clone();
                    let req_id_c2 = requester_id.clone();
                    let rep_id_c2 = rep.id.clone();
                    let rep_id_c3 = rep.id.clone();

                    rsx! {
                        components::ContextMenu {
                            open: *context_menu_open.read(),
                            x: context_menu_pos.read().0,
                            y: context_menu_pos.read().1,
                            onclose: move |_| context_menu_open.set(false),

                            if is_manager {
                                if !is_resolved {
                                    button {
                                        class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                        onclick: move |_| {
                                            let uid = req_id_c.clone();
                                            let rid = rep_id_c.clone();
                                            spawn(async move {
                                                let _ = update_report_status(uid, rid, "resolved".to_string()).await;
                                            });
                                            context_menu_open.set(false);
                                        },
                                        components::LucideIcon { name: "check-circle", size: "14" }
                                        "Mark Resolved"
                                    }
                                }
                                if !is_reviewed {
                                    button {
                                        class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                        onclick: move |_| {
                                            let uid = req_id_c2.clone();
                                            let rid = rep_id_c2.clone();
                                            spawn(async move {
                                                let _ = update_report_status(uid, rid, "reviewed".to_string()).await;
                                            });
                                            context_menu_open.set(false);
                                        },
                                        components::LucideIcon { name: "clock", size: "14" }
                                        "Mark Reviewed"
                                    }
                                }
                            }
                            button {
                                class: "w-full text-left px-3 py-2 text-xs hover:bg-white/5 rounded-md text-foreground flex items-center gap-2 bg-transparent border-0 cursor-pointer",
                                onclick: move |_| {
                                    selected_report_id.set(Some(rep_id_c3.clone()));
                                    show_report_details_modal.set(true);
                                    context_menu_open.set(false);
                                },
                                components::LucideIcon { name: "file-text", size: "14" }
                                "View Details"
                            }
                        }
                    }
                }
            }
        }
    }
}
