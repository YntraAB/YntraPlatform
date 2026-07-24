use crate::components;
use crate::state::AppState;
use dioxus::prelude::*;

#[component]
pub fn TeamsList(
    is_admin: bool,
    is_platform_admin: bool,
    mut show_role_manager_modal: Signal<bool>,
    mut show_template_manager_modal: Signal<bool>,
) -> Element {
    let state = use_context::<AppState>();
    let mut directory_level = state.directory_level;
    let mut selected_directory_team = state.selected_directory_team;
    let mut show_add_team_modal = state.show_add_team_modal;
    let teams_guard = state.teams.read();
    let teams = teams_guard.as_ref().cloned().unwrap_or_default();
    let clients_guard = state.clients.read();
    let clients = clients_guard.as_ref().cloned().unwrap_or_default();

    let auth_region = state.auth_region.read().clone();
    let templates_btn_label = crate::locales::t("templates-btn", &auth_region);

    rsx! {
        div { class: "relative flex h-full flex-1 flex-col bg-background",
            div { class: "flex h-16 shrink-0 items-center justify-between border-b border-border px-8",
                h2 { class: "flex items-center gap-2 text-base font-medium text-foreground m-0",
                    svg {
                        xmlns: "http://www.w3.org/2000/svg", width: "16", height: "16", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", class: "h-4 w-4 text-primary",
                        path { d: "M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" }
                        circle { cx: "9", cy: "7", r: "4" }
                        path { d: "M23 21v-2a4 4 0 0 0-3-3.87" }
                        path { d: "M16 3.13a4 4 0 0 1 0 7.75" }
                    }
                    "Teams"
                }
                div { class: "flex items-center gap-2",
                    if is_platform_admin {
                        button {
                            class: "yntra-btn secondary flex items-center gap-1.5 text-xs h-8 px-3 rounded-lg border border-border bg-transparent text-foreground hover:bg-white/[0.04] cursor-pointer font-semibold transition-all",
                            onclick: move |_| show_template_manager_modal.set(true),
                            components::LucideIcon { name: "layout", class: "h-4 w-4", }
                            "{templates_btn_label}"
                        }
                    }
                    if is_admin {
                        button {
                            class: "yntra-btn secondary h-8 border border-border bg-transparent text-xs text-foreground hover:bg-white/[0.04] px-4 py-2 rounded-lg font-semibold cursor-pointer",
                            onclick: move |_| show_role_manager_modal.set(true),
                            "Manage Roles"
                        }
                    }
                    if is_admin {
                        button {
                            class: "yntra-btn h-8 text-xs flex items-center gap-1.5",
                            style: "padding:0.4rem 0.8rem;",
                            onclick: move |_| show_add_team_modal.set(true),
                            svg {
                                xmlns: "http://www.w3.org/2000/svg", width: "14", height: "14", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2.5", stroke_linecap: "round", stroke_linejoin: "round", class: "h-3.5 w-3.5",
                                line { x1: "12", y1: "5", x2: "12", y2: "19" }
                                line { x1: "5", y1: "12", x2: "19", y2: "12" }
                            }
                            "Create Team"
                        }
                    }
                }
            }
            div { class: "scrollbar-dark w-full flex-1",
                if is_admin {
                    div {
                        onclick: move |_| {
                            selected_directory_team.set(Some("all_members".to_string()));
                            directory_level.set("members".to_string());
                        },
                        class: "group flex cursor-pointer items-center border-b border-border bg-primary/5 px-8 py-3 transition-colors hover:bg-white/[0.02] list-item-hover",
                        div {
                            class: "mr-4 flex h-10 w-10 shrink-0 items-center justify-center rounded-[8px] bg-primary/20 text-primary transition-colors",
                            svg {
                                xmlns: "http://www.w3.org/2000/svg", width: "20", height: "20", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", class: "h-5 w-5",
                                path { d: "M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" }
                                circle { cx: "9", cy: "7", r: "4" }
                                path { d: "M23 21v-2a4 4 0 0 0-3-3.87" }
                                path { d: "M16 3.13a4 4 0 0 1 0 7.75" }
                            }
                        }
                        div {
                            class: "w-64 shrink-0 pr-4 text-[15px] font-medium text-foreground md:w-80",
                            "All Members"
                            div { class: "mt-0.5 text-[11px] font-normal text-muted-foreground/60",
                                "View all members in the organization"
                            }
                        }
                        div { class: "min-w-0 flex-1 pr-4" }
                        div {
                            class: "flex w-12 shrink-0 items-center justify-end text-muted-foreground transition-colors group-hover:text-foreground",
                            components::LucideIcon { name: "chevron-right", class: "h-5 w-5" }
                        }
                    }
                }

                if teams.is_empty() {
                    div {
                        class: "flex flex-col items-center justify-center text-muted-foreground/60 gap-4 py-20",
                        components::LucideIcon { name: "directory", size: "48", color: "var(--text-muted)", class: "opacity-20" }
                        p { class: "text-sm m-0", "No teams found." }
                    }
                } else {
                    for team in teams.iter() {
                        {
                            let t_id = team.id.clone();
                            let t_name = team.name.clone();
                            let team_patients_count = clients.iter().filter(|c| c.team_id.as_ref() == Some(&t_id)).count();

                            rsx! {
                                div {
                                    key: "{t_id}",
                                    onclick: move |_| {
                                        selected_directory_team.set(Some(t_id.clone()));
                                        directory_level.set("members".to_string());
                                    },
                                    class: "group flex cursor-pointer items-center border-b border-border px-8 py-3.5 transition-colors hover:bg-white/[0.02] list-item-hover",

                                    div {
                                        class: "mr-4 flex h-10 w-10 shrink-0 items-center justify-center rounded-[8px] bg-white/[0.04] font-bold text-primary transition-colors",
                                        "{t_name.chars().next().unwrap_or('T')}"
                                    }

                                    div {
                                        class: "w-64 shrink-0 pr-4 text-[15px] font-medium text-foreground md:w-80",
                                        "{t_name}"
                                        div { class: "mt-0.5 text-[11px] font-normal uppercase tracking-wider text-muted-foreground/60",
                                            "Led by: "
                                            span { class: "text-muted-foreground", "Care Manager" }
                                        }
                                    }

                                    div { class: "min-w-0 flex-1 pr-4" }

                                    div {
                                        class: "flex w-48 shrink-0 items-center justify-end gap-3",
                                        span {
                                            class: "inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-[10px] font-medium bg-white/[0.03] border border-border text-muted-foreground",
                                            svg {
                                                xmlns: "http://www.w3.org/2000/svg", width: "12", height: "12", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2.5", stroke_linecap: "round", stroke_linejoin: "round", class: "mr-1 h-3 w-3",
                                                path { d: "M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2" }
                                                circle { cx: "12", cy: "7", r: "4" }
                                            }
                                            "Staff"
                                        }
                                        span {
                                            class: "inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-[10px] font-medium bg-primary/10 border border-primary/20 text-primary",
                                            svg {
                                                xmlns: "http://www.w3.org/2000/svg", width: "12", height: "12", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2.5", stroke_linecap: "round", stroke_linejoin: "round", class: "mr-1 h-3 w-3",
                                                path { d: "M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z" }
                                            }
                                            "{team_patients_count} Patients"
                                        }
                                    }

                                    div {
                                        class: "flex w-16 shrink-0 items-center justify-end gap-2 text-muted-foreground transition-colors group-hover:text-foreground",
                                        components::LucideIcon { name: "chevron-right", class: "h-5 w-5" }
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
