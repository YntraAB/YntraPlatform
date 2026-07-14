use crate::components;
use crate::state::AppState;
use dioxus::prelude::*;

#[component]
pub fn WorkspacesList(is_platform_admin: bool) -> Element {
    let state = use_context::<AppState>();
    let mut devhub_open = use_signal(|| false);
    let mut refresh_trigger = use_signal(|| 0);

    let workspaces_res = use_resource(move || {
        let _trig = refresh_trigger.read();
        let uid = state.active_user_id.read().clone();
        async move { yntra_core::get_workspaces(uid).await.unwrap_or_default() }
    });

    let mut selected_directory_workspace = state.selected_directory_workspace;
    let mut directory_level = state.directory_level;
    let mut db_trigger = state.db_trigger;
    let teams_guard = state.teams.read();
    let teams = teams_guard.as_ref().cloned().unwrap_or_default();
    let users_guard = state.users.read();
    let users = users_guard.as_ref().cloned().unwrap_or_default();

    rsx! {
        div { class: "relative flex h-full flex-1 flex-col bg-background",
            div { class: "flex h-16 shrink-0 items-center justify-between border-b border-border px-8",
                h2 { class: "flex items-center gap-2 text-base font-medium text-foreground m-0",
                    components::LucideIcon { name: "directory", class: "h-4 w-4 text-primary" }
                    "Workspaces"
                }
                if is_platform_admin {
                    button {
                        class: "yntra-btn h-8 text-xs flex items-center gap-1.5",
                        style: "padding:0.4rem 0.8rem;",
                        onclick: move |_| devhub_open.set(true),
                        svg {
                            xmlns: "http://www.w3.org/2000/svg", width: "14", height: "14", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2.5", stroke_linecap: "round", stroke_linejoin: "round", class: "h-3.5 w-3.5",
                            line { x1: "12", y1: "5", x2: "12", y2: "19" }
                            line { x1: "5", y1: "12", x2: "19", y2: "12" }
                        }
                        "Create Workspace"
                    }
                }
            }
            div { class: "scrollbar-dark w-full flex-1",
                if let Some(workspaces) = workspaces_res.read().as_ref() {
                    for ws in workspaces.iter() {
                        {
                            let ws_id = ws.id.clone();
                            let ws_id_open = ws_id.clone();
                            let ws_id_delete = ws_id.clone();
                            let ws_name = ws.name.clone();

                            let ws_teams_count = teams.iter().filter(|t| t.workspace_id == ws_id).count();
                            let ws_members_count = users.iter().filter(|u| u.workspace_id.as_ref() == Some(&ws_id)).count();

                            rsx! {
                                div {
                                    key: "{ws_id}",
                                    onclick: move |_| {
                                        selected_directory_workspace.set(ws_id_open.clone());
                                        directory_level.set("teams".to_string());
                                    },
                                    class: "group flex cursor-pointer items-center border-b border-border px-8 py-3.5 transition-colors hover:bg-white/[0.02] list-item-hover",

                                    div {
                                        class: "mr-4 flex h-10 w-10 shrink-0 items-center justify-center rounded-[8px] bg-white/[0.04] text-primary transition-colors",
                                        components::LucideIcon { name: "directory", class: "h-5 w-5" }
                                    }

                                    div {
                                        class: "w-64 shrink-0 pr-4 text-[15px] font-medium text-foreground md:w-80",
                                        "{ws_name}"
                                        div { class: "mt-0.5 text-[11px] font-normal uppercase tracking-wider text-muted-foreground/60",
                                            "Företag"
                                        }
                                    }

                                    div { class: "min-w-0 flex-1 pr-4" }

                                    div {
                                        class: "mr-4 flex w-48 shrink-0 items-center justify-end gap-6 text-[13px] text-muted-foreground",
                                        div { class: "flex items-center gap-1.5",
                                            svg {
                                                xmlns: "http://www.w3.org/2000/svg", width: "14", height: "14", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", class: "h-3.5 w-3.5 text-muted-foreground/60",
                                                path { d: "M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" }
                                                circle { cx: "9", cy: "7", r: "4" }
                                                path { d: "M23 21v-2a4 4 0 0 0-3-3.87" }
                                                path { d: "M16 3.13a4 4 0 0 1 0 7.75" }
                                            }
                                            "{ws_teams_count} Teams"
                                        }
                                        div { class: "flex items-center gap-1.5",
                                            svg {
                                                xmlns: "http://www.w3.org/2000/svg", width: "14", height: "14", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", class: "h-3.5 w-3.5 text-muted-foreground/60",
                                                path { d: "M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2" }
                                                circle { cx: "12", cy: "7", r: "4" }
                                            }
                                            "{ws_members_count} Users"
                                        }
                                    }

                                    div {
                                        class: "flex w-12 shrink-0 items-center justify-end gap-2 text-muted-foreground transition-colors group-hover:text-foreground",
                                        if is_platform_admin {
                                            button {
                                                class: "mr-2 transition-colors hover:text-red-500 bg-transparent border-0 cursor-pointer p-0",
                                                onclick: move |e| {
                                                    e.stop_propagation();
                                                    let target_id = ws_id_delete.clone();
                                                    let requester_uid = state.active_user_id.read().clone();
                                                    spawn(async move {
                                                        if yntra_core::delete_workspace_via_hub(requester_uid, target_id).await.is_ok() {
                                                            let next = *refresh_trigger.read() + 1;
                                                            refresh_trigger.set(next);
                                                        }
                                                    });
                                                },
                                                svg {
                                                    xmlns: "http://www.w3.org/2000/svg", width: "16", height: "16", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", class: "h-4 w-4",
                                                    polyline { points: "3 6 5 6 21 6" }
                                                    path { d: "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" }
                                                    line { x1: "10", y1: "11", x2: "10", y2: "17" }
                                                    line { x1: "14", y1: "11", x2: "14", y2: "17" }
                                                }
                                            }
                                        }
                                        components::LucideIcon { name: "chevron-right", class: "h-5 w-5" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if *devhub_open.read() {
            components::DevHubDialog {
                open: *devhub_open.read(),
                onclose: move |_| devhub_open.set(false),
                onsubmit: move |_| {
                    let next = *refresh_trigger.read() + 1;
                    refresh_trigger.set(next);
                    let current = *db_trigger.read();
                    db_trigger.set(current + 1);
                }
            }
        }
    }
}
