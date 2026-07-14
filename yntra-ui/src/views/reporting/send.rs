use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::WorkspaceUser;
use yntra_core::add_report;

#[derive(Props, Clone, PartialEq)]
pub struct ReportSubmitFormProps {
    pub active_user: WorkspaceUser,
    pub region: String,
    pub report_type: Signal<String>,
    pub report_date: Signal<String>,
    pub report_subject: Signal<String>,
    pub report_description: Signal<String>,
    pub report_is_anonymous: Signal<bool>,
    pub report_tab: Signal<String>,
}

#[component]
pub fn ReportSubmitForm(props: ReportSubmitFormProps) -> Element {
    let active_user = props.active_user;
    let region = props.region;
    let mut report_type = props.report_type;
    let mut report_date = props.report_date;
    let mut report_subject = props.report_subject;
    let mut report_description = props.report_description;
    let mut report_is_anonymous = props.report_is_anonymous;
    let mut report_tab = props.report_tab;
    let mut report_type_open = use_signal(|| false);

    rsx! {
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
                                    let req_uid = user_id.clone();
                                    if let Ok(report) = add_report(
                                        req_uid,
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
    }
}
