use crate::components::{Button, LucideIcon};
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{get_industry_templates, install_industry_template, IndustryTemplate};

#[derive(Props, Clone, PartialEq)]
pub struct TemplateMarketplaceProps {
    pub active_user_id: String,
    pub workspace_id: String,
    pub locale: String,
    pub oninstall: EventHandler<String>,
    pub onclose: EventHandler<()>,
}

#[component]
pub fn TemplateMarketplace(props: TemplateMarketplaceProps) -> Element {
    let loc = props.locale.as_str();
    let mut selected_category = use_signal(|| "All".to_string());
    let mut installing_id = use_signal(|| Option::<String>::None);

    let req_uid = props.active_user_id.clone();
    let templates_res = use_resource(move || {
        let r_uid = req_uid.clone();
        async move {
            get_industry_templates(r_uid).await.unwrap_or_default()
        }
    });

    let templates = templates_res.read().clone().unwrap_or_default();
    let sel_cat = selected_category.read().clone();

    let filtered_templates: Vec<IndustryTemplate> = if sel_cat == "All" {
        templates.clone()
    } else {
        templates.iter().filter(|t| t.category == sel_cat).cloned().collect()
    };

    let categories = vec![
        "All",
        "HVAC & Field Service",
        "Healthcare & Care Services",
        "Logistics & Transport",
        "Property Management",
        "Retail & Asset Management",
    ];

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4 sm:p-6",
            div { class: "w-full max-w-5xl h-[85vh] rounded-3xl border border-border bg-card p-6 shadow-2xl flex flex-col gap-6 overflow-hidden",
                // Header
                div { class: "flex items-center justify-between border-b border-border pb-4",
                    div { class: "flex items-center gap-3",
                        div { class: "h-11 w-11 rounded-2xl bg-primary/10 flex items-center justify-center text-primary shadow-sm",
                            LucideIcon { name: "store", class: "h-6 w-6" }
                        }
                        div {
                            h2 { class: "text-lg font-bold text-foreground m-0 flex items-center gap-2",
                                "1-Click Industry Template Marketplace"
                                span { class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase bg-emerald-500/10 text-emerald-600 border border-emerald-500/20", "No-Code Ready" }
                            }
                            p { class: "text-xs text-muted-foreground m-0 mt-0.5",
                                "Deploy pre-built operational schemas, views, and seed data into your workspace instantly"
                            }
                        }
                    }

                    button {
                        class: "p-2 rounded-xl border border-border bg-secondary text-muted-foreground hover:text-foreground cursor-pointer transition-all",
                        onclick: move |_| props.onclose.call(()),
                        LucideIcon { name: "x", class: "h-5 w-5" }
                    }
                }

                // Filter Category Chips
                div { class: "flex flex-wrap items-center gap-2 pb-1 border-b border-border/40",
                    for cat in categories.iter() {
                        {
                            let is_active = sel_cat == *cat;
                            let cat_str = cat.to_string();
                            rsx! {
                                button {
                                    key: "{cat}",
                                    class: format!(
                                        "px-3.5 py-1.5 rounded-xl text-xs font-semibold transition-all cursor-pointer border {}",
                                        if is_active { "bg-primary text-primary-foreground border-primary shadow-sm" } else { "bg-secondary/60 text-secondary-foreground border-border hover:bg-secondary" }
                                    ),
                                    onclick: move |_| selected_category.set(cat_str.clone()),
                                    "{cat}"
                                }
                            }
                        }
                    }
                }

                // Grid of Templates
                div { class: "flex-1 overflow-y-auto pr-1 grid grid-cols-1 md:grid-cols-2 gap-5",
                    if filtered_templates.is_empty() {
                        div { class: "col-span-full p-12 text-center text-muted-foreground italic",
                            "No industry templates found for this category."
                        }
                    } else {
                        for tpl in filtered_templates.iter() {
                            {
                                let tpl_id = tpl.id.clone();
                                let is_curr_installing = installing_id.read().as_deref() == Some(&tpl_id);
                                let schema_fields: Vec<serde_json::Value> = serde_json::from_str(&tpl.fields_schema).unwrap_or_default();
                                
                                rsx! {
                                    div {
                                        key: "{tpl.id}",
                                        class: "rounded-2xl border border-border bg-background p-5 flex flex-col justify-between gap-4 shadow-sm hover:border-primary/40 hover:shadow-md transition-all group",
                                        
                                        div { class: "space-y-3",
                                            div { class: "flex items-start justify-between gap-2",
                                                div { class: "flex items-center gap-2.5",
                                                    div { class: "h-9 w-9 rounded-xl bg-primary/10 text-primary flex items-center justify-center group-hover:scale-105 transition-transform",
                                                        LucideIcon { name: &tpl.icon, class: "h-5 w-5" }
                                                    }
                                                    h3 { class: "text-sm font-bold text-foreground m-0 group-hover:text-primary transition-colors", "{tpl.name}" }
                                                }
                                                span { class: "px-2 py-0.5 rounded-full text-[10px] font-semibold bg-secondary text-secondary-foreground border border-border shrink-0",
                                                    "{tpl.category}"
                                                }
                                            }

                                            p { class: "text-xs text-muted-foreground m-0 leading-relaxed", "{tpl.description}" }

                                            // Schema Tags
                                            div { class: "space-y-1 pt-1",
                                                span { class: "text-[10px] font-bold text-muted-foreground uppercase tracking-wider", "Included Schema Fields" }
                                                div { class: "flex flex-wrap gap-1 text-[10px]",
                                                    for field in schema_fields.iter().take(5) {
                                                        {
                                                            let label = field.get("label").and_then(|v| v.as_str()).unwrap_or("Field");
                                                            let field_type = field.get("type").and_then(|v| v.as_str()).unwrap_or("text");
                                                            rsx! {
                                                                span { key: "{label}", class: "px-2 py-0.5 rounded-md bg-muted text-muted-foreground font-medium",
                                                                    "{label} ({field_type})"
                                                                }
                                                            }
                                                        }
                                                    }
                                                    if schema_fields.len() > 5 {
                                                        span { class: "px-2 py-0.5 rounded-md bg-muted text-muted-foreground font-semibold",
                                                            "+{schema_fields.len() - 5} more"
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        // Action Footer
                                        div { class: "flex items-center justify-between pt-3 border-t border-border/40",
                                            span { class: "text-[11px] font-medium text-emerald-600 flex items-center gap-1",
                                                LucideIcon { name: "check-circle", class: "h-3.5 w-3.5" }
                                                "Includes Sample Seed Data"
                                            }

                                            Button {
                                                class: "text-xs h-8 px-4 rounded-xl bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm cursor-pointer",
                                                disabled: is_curr_installing,
                                                onclick: {
                                                    let t_id = tpl.id.clone();
                                                    let req_uid = props.active_user_id.clone();
                                                    let ws_id = props.workspace_id.clone();
                                                    let oninstall_cb = props.oninstall.clone();
                                                    move |_| {
                                                        installing_id.set(Some(t_id.clone()));
                                                        let req_uid = req_uid.clone();
                                                        let ws_id = ws_id.clone();
                                                        let t_id = t_id.clone();
                                                        let oninstall_cb = oninstall_cb.clone();
                                                        spawn(async move {
                                                            if let Ok(installed_block) = install_industry_template(req_uid, ws_id, t_id).await {
                                                                oninstall_cb.call(installed_block.id);
                                                            }
                                                            installing_id.set(None);
                                                        });
                                                    }
                                                },
                                                if is_curr_installing {
                                                    LucideIcon { name: "refresh-cw", class: "h-3.5 w-3.5 animate-spin mr-1" }
                                                    "Installing..."
                                                } else {
                                                    LucideIcon { name: "download-cloud", class: "h-3.5 w-3.5 mr-1" }
                                                    "1-Click Install"
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
