use crate::components::LucideIcon;
use dioxus::prelude::*;

#[derive(Props, Clone)]
pub struct GlobalSearchProps {
    pub open: Signal<bool>,
    pub active_section: Signal<String>,
    pub settings_tab: Signal<String>,
    pub report_tab: Signal<String>,
    pub report_type: Signal<String>,
    pub selected_note_team_id: Signal<String>,
    pub active_message_id: Signal<Option<String>>,
}

impl PartialEq for GlobalSearchProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn GlobalSearch(props: GlobalSearchProps) -> Element {
    let mut open = props.open;
    let mut active_section = props.active_section;
    let _settings_tab = props.settings_tab;
    let mut report_tab = props.report_tab;
    let mut report_type = props.report_type;
    let mut selected_note_team_id = props.selected_note_team_id;
    let _active_message_id = props.active_message_id;

    let mut query = use_signal(String::new);

    use_effect(move || {
        let mut eval = document::eval(
            r#"
            window.addEventListener('keydown', (e) => {
                if ((e.key === 'k' && (e.metaKey || e.ctrlKey)) || e.key === '/') {
                    const target = e.target;
                    const isInput = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable;
                    if (!isInput) {
                        e.preventDefault();
                        dioxus.send("toggle");
                    }
                }
            });
        "#,
        );

        spawn(async move {
            while let Ok(msg) = eval.recv::<String>().await {
                if msg == "toggle" {
                    let current = *open.read();
                    open.set(!current);
                }
            }
        });
    });

    if !*open.read() {
        return rsx! {};
    }

    let state = use_context::<crate::state::AppState>();
    let users_data = state.users.read().clone().unwrap_or_default();
    let notes_data = state.notes.read().clone().unwrap_or_default();

    let search_query = query.read().to_lowercase();

    // 1. Navigation items
    let navigation_items = vec![
        ("Dashboard", "dashboard", "layout-dashboard"),
        ("Messaging Inbox", "messaging", "message-square"),
        ("Shifts Calendar", "scheduling", "calendar"),
        ("Handover Logs", "notes", "file-text"),
        ("Timesheets manager", "time", "clock"),
        ("Care Assistance Profiles", "assistance", "heart"),
        ("Teams Directory", "directory", "folder-open"),
        ("Incident Whistleblower logs", "reporting", "alert-triangle"),
        ("Workspace Settings", "settings", "settings"),
    ];

    // Filtered navigation
    let filtered_nav: Vec<_> = navigation_items
        .into_iter()
        .filter(|(label, _, _)| label.to_lowercase().contains(&search_query))
        .collect();

    // 2. Action items
    let action_items = vec![
        ("File a Complaint Report", "complaint"),
        ("Log a Work-Related Injury", "work_injury"),
        ("Report a General Incident", "incident"),
        ("Submit a Safety Deviation", "deviation"),
        ("Anonymous Whistleblower Filing", "whistleblower"),
    ];

    let filtered_actions: Vec<_> = action_items
        .into_iter()
        .filter(|(label, _)| label.to_lowercase().contains(&search_query))
        .collect();

    // 3. Filtered Users
    let filtered_users: Vec<_> = users_data
        .iter()
        .filter(|u| {
            let name = u.full_name.clone().unwrap_or_default().to_lowercase();
            name.contains(&search_query) || u.email.to_lowercase().contains(&search_query)
        })
        .cloned()
        .collect();

    // 4. Filtered Notes
    let filtered_notes: Vec<_> = notes_data
        .iter()
        .filter(|n| n.subject.to_lowercase().contains(&search_query))
        .cloned()
        .collect();

    rsx! {
        div {
            class: "flex items-center justify-center p-4",
                    style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; z-index: 999; backdrop-filter: blur(8px); background: rgba(0, 0, 0, 0.6);",
            onclick: move |_| {
                open.set(false);
                query.set(String::new());
            },
            style {
                r#"
                .cmd-dialog {{
                    background: var(--bg-main);
                    border: 1px solid var(--border-color);
                    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.85);
                    animation: zoomIn 0.15s ease-out;
                    width: 100%;
                    max-width: 512px;
                    max-height: 60vh;
                    border-radius: 12px;
                    overflow: hidden;
                    display: flex;
                    flex-direction: column;
                }}
                .cmd-results {{
                    flex: 1;
                    overflow-y: auto;
                    padding: 0.5rem;
                    display: flex;
                    flex-direction: column;
                    gap: 0.25rem;
                }}
                .cmd-input {{
                    flex: 1;
                    background: transparent;
                    border: none;
                    outline: none;
                    color: var(--text-primary);
                    font-size: 0.875rem;
                    font-family: inherit;
                    width: 100%;
                }}
                .cmd-input::placeholder {{
                    color: var(--text-muted);
                }}
                .cmd-item {{
                     display: flex;
                     align-items: center;
                     gap: 0.75rem;
                     width: 100%;
                     padding: 0.65rem 1rem;
                     border-radius: 8px;
                     font-size: 0.85rem;
                     color: var(--text-secondary);
                     background: transparent;
                     border: none;
                    text-align: left;
                    cursor: pointer;
                    transition: all 0.15s;
                }}
                .cmd-item:hover {{
                    background: rgba(255, 255, 255, 0.04);
                    color: var(--text-primary);
                }}
                .cmd-section-title {{
                    font-size: 0.7rem;
                    font-weight: 600;
                    text-transform: uppercase;
                    color: var(--text-muted);
                    padding: 0.5rem 1rem 0.25rem 1rem;
                    letter-spacing: 0.05em;
                }}
                .cmd-separator {{
                    height: 1px;
                    background: var(--border-color);
                    margin: 0.4rem 0;
                }}
                @keyframes zoomIn {{
                    from {{ transform: scale(0.97); opacity: 0; }}
                    to {{ transform: scale(1); opacity: 1; }}
                }}
                "#
            }
            div {
                class: "cmd-dialog",
                onclick: move |e| e.stop_propagation(),

                // Search Input Header
                div {
                    class: "flex items-center border-b border-border gap-3",
                    style: "padding: 0.85rem 1.25rem;",
                    LucideIcon { name: "search", class: "h-4 w-4 text-text-secondary", }
                    input {
                        class: "cmd-input",
                        placeholder: "Sök i Yntra...",
                        value: "{query}",
                        oninput: move |e| query.set(e.value()),
                        autofocus: true,
                    }
                    span {
                        class: "border border-border text-muted-foreground/60 font-medium",
                    style: "font-family: inherit; font-size: 0.65rem; background: rgba(255,255,255,0.05); padding: 0.1rem 0.35rem; border-radius: 4px;",
                        "ESC"
                    }
                }

                // Results Container
                div {
                    class: "cmd-results scrollbar-thin",

                    // If all are empty, show empty state
                    if filtered_nav.is_empty() && filtered_actions.is_empty() && filtered_notes.is_empty() && filtered_users.is_empty() {
                        div {
                            class: "text-center text-muted-foreground/60 text-sm flex flex-col items-center gap-2",
                    style: "padding: 2.5rem 1.5rem;",
                            LucideIcon { name: "search", class: "h-6 w-6 text-text-muted", }
                            span { "Inga resultat hittades." }
                        }
                    }

                    // 1. Navigation items group
                    if !filtered_nav.is_empty() {
                        div {
                            div { class: "cmd-section-title", "Navigation" }
                            for (label, section, icon) in filtered_nav.iter().cloned() {
                                button {
                                    class: "cmd-item gap-3",
                                    onclick: move |_| {
                                        active_section.set(section.to_string());
                                        open.set(false);
                                        query.set(String::new());
                                    },
                                    LucideIcon {
                                        name: icon,
                                        class: "h-4 w-4 text-primary",
                                    }
                                    span { "{label}" }
                                }
                            }
                        }
                    }

                    // Divider
                    if !filtered_nav.is_empty() && (!filtered_actions.is_empty() || !filtered_notes.is_empty() || !filtered_users.is_empty()) {
                        div { class: "cmd-separator", }
                    }

                    // 2. Action items group
                    if !filtered_actions.is_empty() {
                        div {
                            div { class: "cmd-section-title", "Åtgärder" }
                            for (label, r_type) in filtered_actions.iter().cloned() {
                                button {
                                    class: "cmd-item gap-3",
                                    onclick: move |_| {
                                        active_section.set("reporting".to_string());
                                        report_tab.set("send".to_string());
                                        report_type.set(r_type.to_string());
                                        open.set(false);
                                        query.set(String::new());
                                    },
                                    LucideIcon {
                                        name: "alert-circle",
                                        class: "h-4 w-4 text-amber-500",
                                    }
                                    span { "{label}" }
                                }
                            }
                        }
                    }

                    // Divider
                    if !filtered_actions.is_empty() && (!filtered_notes.is_empty() || !filtered_users.is_empty()) {
                        div { class: "cmd-separator", }
                    }

                    // 3. notes items group
                    if !filtered_notes.is_empty() {
                        div {
                            div { class: "cmd-section-title", "Daganteckningar" }
                            for note in filtered_notes.iter() {
                                {
                                    let _note_id = note.id.clone();
                                    let team_id = note.team_id.clone();
                                    let subject = note.subject.clone();
                                    rsx! {
                                        button {
                                            class: "cmd-item gap-3",
                                            onclick: move |_| {
                                                active_section.set("notes".to_string());
                                                selected_note_team_id.set(team_id.clone());
                                                open.set(false);
                                                query.set(String::new());
                                            },
                                            LucideIcon { name: "file-text", class: "h-4 w-4 text-emerald-500", }
                                            span { "{subject}" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Divider
                    if !filtered_notes.is_empty() && !filtered_users.is_empty() {
                        div { class: "cmd-separator", }
                    }

                    // 4. Caregivers items group
                    if !filtered_users.is_empty() {
                        div {
                            div { class: "cmd-section-title", "Personal & Användare" }
                            for user in filtered_users.iter() {
                                {
                                    let user_name = user.full_name.clone().unwrap_or_else(|| user.email.clone());
                                    let user_email = user.email.clone();
                                    rsx! {
                                        button {
                                            class: "cmd-item gap-3",
                                            onclick: move |_| {
                                                active_section.set("directory".to_string());
                                                open.set(false);
                                                query.set(String::new());
                                            },
                                            LucideIcon { name: "settings", class: "h-4 w-4 text-primary", }
                                            div { class: "flex flex-col",
                                                span { "{user_name}" }
                                                span { class: "text-[10px] text-gray-500", "{user_email}" }
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
