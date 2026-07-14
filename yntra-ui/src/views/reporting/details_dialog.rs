use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{ReportItem, WorkspaceUser, update_report_status};

#[derive(Props, Clone, PartialEq)]
pub struct ReportDetailsDialogProps {
    pub show_report_details_modal: Signal<bool>,
    pub details_report: ReportItem,
    pub users: Vec<WorkspaceUser>,
    pub active_user: WorkspaceUser,
    pub region: String,
    pub is_manager: bool,
}

#[component]
pub fn ReportDetailsDialog(props: ReportDetailsDialogProps) -> Element {
    let mut show_report_details_modal = props.show_report_details_modal;
    let rep = props.details_report;
    let users = props.users.clone();
    let active_user = props.active_user;
    let region = props.region;
    let is_manager = props.is_manager;

    let rep_id = rep.id.clone();
    let rep_type = rep.type_name.clone();
    let content_val: serde_json::Value = serde_json::from_str(&rep.content).unwrap_or_default();
    let subject = content_val
        .get("subject")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| t("reporting-no-subject", &region));
    let desc = content_val
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let date_val = content_val
        .get("date_of_incident")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let reporter_name = if rep.is_anonymous {
        t("reporting-anonymous", &region)
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
    }
    .to_uppercase();

    let details_title = format!(
        "{}{}",
        t("reporting-details-title-prefix", &region),
        display_type_caps
    );

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
