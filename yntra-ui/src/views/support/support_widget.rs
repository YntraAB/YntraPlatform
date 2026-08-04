use crate::components;
use dioxus::prelude::*;
use yntra_core::{HelpdeskArticle, SupportTicket, SupportTicketMessage, Workspace, WorkspaceUser};

#[derive(Props, Clone, PartialEq)]
pub struct SupportWidgetProps {
    pub active_user: WorkspaceUser,
    pub workspace: Workspace,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

#[component]
pub fn SupportWidget(props: SupportWidgetProps) -> Element {
    let mut is_open = use_signal(|| false);
    let mut active_tab = use_signal(|| "tour".to_string());

    // Helpdesk Search signals
    let mut search_query = use_signal(String::new);
    let mut selected_article = use_signal(|| Option::<HelpdeskArticle>::None);

    // Ticket Creation signals
    let mut ticket_subject = use_signal(String::new);
    let mut ticket_category = use_signal(|| "technical".to_string());
    let mut ticket_priority = use_signal(|| "medium".to_string());
    let mut ticket_message = use_signal(String::new);
    let mut selected_ticket = use_signal(|| Option::<SupportTicket>::None);
    let mut thread_input = use_signal(String::new);

    let mut is_submitting = use_signal(|| false);
    let mut status_msg = use_signal(|| Option::<String>::None);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let active_user_id = props.active_user.id.clone();

    // Asynchronous resource for helpdesk articles
    let articles_res = use_resource(move || {
        let uid = active_user_id.clone();
        let q = search_query.read().clone();
        let _trig = *props.db_trigger.read();
        async move { yntra_core::search_helpdesk_articles(uid, q, "all".to_string()).await }
    });

    let active_user_id_t = props.active_user.id.clone();
    let workspace_id_t = props.workspace.id.clone();

    // Asynchronous resource for tickets
    let tickets_res = use_resource(move || {
        let uid = active_user_id_t.clone();
        let wsid = workspace_id_t.clone();
        let _trig = *props.db_trigger.read();
        async move { yntra_core::get_user_support_tickets(uid, wsid).await }
    });

    let handle_create_ticket = {
        let req_uid = props.active_user.id.clone();
        let wsid = props.workspace.id.clone();
        let mut db_trigger = props.db_trigger;
        move |_| {
            let subj = ticket_subject.read().trim().to_string();
            let cat = ticket_category.read().clone();
            let prio = ticket_priority.read().clone();
            let msg = ticket_message.read().trim().to_string();

            if subj.is_empty() || msg.is_empty() {
                error_msg.set(Some("Please fill in both subject and message.".to_string()));
                return;
            }

            is_submitting.set(true);
            error_msg.set(None);
            status_msg.set(None);

            let uid = req_uid.clone();
            let ws = wsid.clone();

            spawn(async move {
                match yntra_core::create_support_ticket(uid, ws, subj, cat, prio, msg).await {
                    Ok(t) => {
                        is_submitting.set(false);
                        ticket_subject.set(String::new());
                        ticket_message.set(String::new());
                        status_msg.set(Some(format!(
                            "Support Ticket {} created successfully!",
                            t.id
                        )));
                        selected_ticket.set(Some(t));
                        let trig_val = *db_trigger.read();
                        db_trigger.set(trig_val + 1);
                    }
                    Err(e) => {
                        is_submitting.set(false);
                        error_msg.set(Some(format!("Failed to create support ticket: {}", e)));
                    }
                }
            });
        }
    };

    let handle_send_thread_message = {
        let req_uid = props.active_user.id.clone();
        let wsid = props.workspace.id.clone();
        let mut db_trigger = props.db_trigger;
        move |_| {
            let msg_text = thread_input.read().trim().to_string();
            let current_t = selected_ticket.read().clone();

            if msg_text.is_empty() || current_t.is_none() {
                return;
            }

            let t_id = current_t.unwrap().id;
            is_submitting.set(true);

            let uid = req_uid.clone();
            let ws = wsid.clone();

            spawn(async move {
                match yntra_core::add_support_ticket_message(uid, ws, t_id, msg_text).await {
                    Ok(updated) => {
                        is_submitting.set(false);
                        thread_input.set(String::new());
                        selected_ticket.set(Some(updated));
                        let trig_val = *db_trigger.read();
                        db_trigger.set(trig_val + 1);
                    }
                    Err(e) => {
                        is_submitting.set(false);
                        error_msg.set(Some(format!("Failed to post message: {}", e)));
                    }
                }
            });
        }
    };

    rsx! {
        div { class: "fixed bottom-6 right-6 z-50 flex flex-col items-end pointer-events-auto",
            if *is_open.read() {
                div { class: "mb-4 w-80 md:w-96 h-[520px] bg-card border border-border/50 rounded-2xl shadow-2xl flex flex-col overflow-hidden animate-in slide-in-from-bottom-5 duration-200",
                    div { class: "p-4 bg-primary text-primary-foreground flex items-center justify-between shadow-sm",
                        div { class: "flex items-center gap-2",
                            components::LucideIcon { name: "life-buoy", class: "h-5 w-5" }
                            span { class: "font-extrabold text-sm tracking-tight", "Yntra Help & Guidance" }
                        }
                        button {
                            class: "h-7 w-7 rounded-full hover:bg-white/20 flex items-center justify-center transition-colors text-white",
                            onclick: move |_| is_open.set(false),
                            components::LucideIcon { name: "x", class: "h-4 w-4" }
                        }
                    }

                    div { class: "grid grid-cols-3 bg-muted/40 border-b border-border/30 text-xs font-semibold text-muted-foreground",
                        button {
                            class: format!(
                                "py-2.5 transition-colors border-b-2 text-center flex items-center justify-center gap-1.5 {}",
                                if *active_tab.read() == "tour" { "border-primary text-primary bg-background font-bold" } else { "border-transparent hover:text-foreground" }
                            ),
                            onclick: move |_| active_tab.set("tour".to_string()),
                            components::LucideIcon { name: "compass", class: "h-3.5 w-3.5" }
                            "Tour"
                        }
                        button {
                            class: format!(
                                "py-2.5 transition-colors border-b-2 text-center flex items-center justify-center gap-1.5 {}",
                                if *active_tab.read() == "helpdesk" { "border-primary text-primary bg-background font-bold" } else { "border-transparent hover:text-foreground" }
                            ),
                            onclick: move |_| active_tab.set("helpdesk".to_string()),
                            components::LucideIcon { name: "book-open", class: "h-3.5 w-3.5" }
                            "Helpdesk"
                        }
                        button {
                            class: format!(
                                "py-2.5 transition-colors border-b-2 text-center flex items-center justify-center gap-1.5 {}",
                                if *active_tab.read() == "tickets" { "border-primary text-primary bg-background font-bold" } else { "border-transparent hover:text-foreground" }
                            ),
                            onclick: move |_| active_tab.set("tickets".to_string()),
                            components::LucideIcon { name: "message-square", class: "h-3.5 w-3.5" }
                            "Tickets"
                        }
                    }

                    div { class: "flex-1 overflow-y-auto p-4 space-y-4 text-xs text-foreground bg-background",
                        match active_tab.read().as_str() {
                            "tour" => rsx! {
                                div { class: "space-y-4 animate-in fade-in duration-150",
                                    div { class: "p-4 rounded-xl bg-primary/10 border border-primary/20 space-y-2",
                                        div { class: "flex items-center gap-2 text-primary font-bold text-sm",
                                            components::LucideIcon { name: "sparkles", class: "h-4 w-4" }
                                            "Interactive Platform Tour"
                                        }
                                        p { class: "text-muted-foreground m-0 leading-relaxed text-xs",
                                            "Take a guided 5-step tour of the Yntra Platform to learn navigation, workspace dispatching, RBAC admin controls, and billing engines."
                                        }
                                    }

                                    div { class: "space-y-2",
                                        div { class: "flex items-center justify-between p-3 rounded-xl border border-border/30 bg-muted/20",
                                            div { class: "flex items-center gap-2.5",
                                                components::LucideIcon { name: "map-pin", class: "h-4 w-4 text-primary" }
                                                span { class: "font-semibold", "Workspace Onboarding Tour" }
                                            }
                                            span { class: "text-[10px] font-bold uppercase text-emerald-500 bg-emerald-500/10 px-2 py-0.5 rounded-full border border-emerald-500/20", "5 Steps" }
                                        }

                                        button {
                                            class: "w-full py-2.5 rounded-xl bg-primary text-primary-foreground text-xs font-bold hover:opacity-90 transition-all shadow-sm flex items-center justify-center gap-2",
                                            onclick: {
                                                let uid = props.active_user.id.clone();
                                                let ws = props.workspace.id.clone();
                                                let mut db_trig = props.db_trigger;
                                                move |_| {
                                                    let u = uid.clone();
                                                    let w = ws.clone();
                                                    spawn(async move {
                                                        let _ = yntra_core::reset_product_tour(u, w, "onboarding_tour".to_string()).await;
                                                        let trig_val = *db_trig.read();
                                                        db_trig.set(trig_val + 1);
                                                    });
                                                }
                                            },
                                            components::LucideIcon { name: "play", class: "h-4 w-4" }
                                            "Start Guided Product Tour"
                                        }
                                    }
                                }
                            },

                            "helpdesk" => rsx! {
                                div { class: "space-y-4 animate-in fade-in duration-150",
                                    if let Some(art) = selected_article.read().as_ref() {
                                        div { class: "space-y-3",
                                            button {
                                                class: "text-xs font-semibold text-primary hover:underline flex items-center gap-1",
                                                onclick: move |_| selected_article.set(None),
                                                components::LucideIcon { name: "arrow-left", class: "h-3.5 w-3.5" }
                                                "Back to Search"
                                            }
                                            div { class: "space-y-1 border-b border-border/30 pb-2",
                                                h4 { class: "font-bold text-sm text-foreground m-0", "{art.title}" }
                                                span { class: "text-[10px] font-bold text-primary uppercase tracking-wider", "{art.category}" }
                                            }
                                            div { class: "text-xs text-muted-foreground whitespace-pre-wrap leading-relaxed font-sans",
                                                "{art.content_markdown}"
                                            }
                                        }
                                    } else {
                                        div { class: "space-y-3",
                                            div { class: "relative",
                                                input {
                                                    r#type: "text",
                                                    placeholder: "Search help articles (e.g. SITHS, CalDAV, Billing)...",
                                                    value: "{search_query}",
                                                    class: "w-full pl-9 pr-3 py-2 rounded-xl border border-input bg-background text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary/50",
                                                    oninput: move |e: Event<FormData>| search_query.set(e.value())
                                                }
                                                components::LucideIcon { name: "search", class: "absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" }
                                            }

                                            match &*articles_res.read() {
                                                Some(Ok(list)) if !list.is_empty() => rsx! {
                                                    div { class: "space-y-2",
                                                        for art in list.iter() {
                                                            div {
                                                                key: "{art.id}",
                                                                class: "p-3 rounded-xl border border-border/30 bg-card hover:bg-accent/40 cursor-pointer transition-colors space-y-1",
                                                                onclick: {
                                                                    let article_clone = art.clone();
                                                                    move |_| selected_article.set(Some(article_clone.clone()))
                                                                },
                                                                div { class: "flex items-center justify-between",
                                                                    span { class: "font-bold text-xs text-foreground", "{art.title}" }
                                                                    components::LucideIcon { name: "chevron-right", class: "h-3.5 w-3.5 text-muted-foreground" }
                                                                }
                                                                p { class: "text-[11px] text-muted-foreground line-clamp-2 m-0", "{art.summary}" }
                                                            }
                                                        }
                                                    }
                                                },
                                                _ => rsx! {
                                                    div { class: "py-6 text-center text-xs text-muted-foreground", "No helpdesk articles found" }
                                                }
                                            }
                                        }
                                    }
                                }
                            },

                            "tickets" => rsx! {
                                div { class: "space-y-4 animate-in fade-in duration-150",
                                    if let Some(t) = selected_ticket.read().as_ref() {
                                        div { class: "space-y-3 flex flex-col h-full",
                                            div { class: "flex items-center justify-between border-b border-border/30 pb-2",
                                                button {
                                                    class: "text-xs font-semibold text-primary hover:underline flex items-center gap-1",
                                                    onclick: move |_| selected_ticket.set(None),
                                                    components::LucideIcon { name: "arrow-left", class: "h-3.5 w-3.5" }
                                                    "Back to Tickets"
                                                }
                                                span { class: "px-2 py-0.5 rounded-full text-[10px] font-bold uppercase bg-primary/10 text-primary border border-primary/20",
                                                    "{t.status}"
                                                }
                                            }

                                            h4 { class: "font-bold text-xs text-foreground m-0", "{t.subject}" }

                                            div { class: "space-y-2 max-h-48 overflow-y-auto p-2 bg-muted/20 rounded-xl border border-border/30",
                                                for m in serde_json::from_str::<Vec<SupportTicketMessage>>(&t.messages_json).unwrap_or_default().iter() {
                                                    div {
                                                        key: "{m.timestamp}",
                                                        class: format!(
                                                            "p-2.5 rounded-lg text-xs space-y-1 max-w-[85%] {}",
                                                            if m.is_staff { "bg-primary/10 text-foreground ml-auto border border-primary/20" } else { "bg-card text-foreground border border-border/30" }
                                                        ),
                                                        div { class: "flex items-center justify-between text-[10px] text-muted-foreground font-bold",
                                                            span { "{m.sender_name}" }
                                                        }
                                                        p { class: "m-0 whitespace-pre-wrap", "{m.message}" }
                                                    }
                                                }
                                            }

                                            div { class: "flex gap-2 pt-2 border-t border-border/30",
                                                input {
                                                    r#type: "text",
                                                    placeholder: "Write follow-up reply...",
                                                    value: "{thread_input}",
                                                    class: "flex-1 px-3 py-1.5 rounded-xl border border-input bg-background text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary/50",
                                                    oninput: move |e: Event<FormData>| thread_input.set(e.value())
                                                }
                                                button {
                                                    class: "px-3 py-1.5 rounded-xl bg-primary text-primary-foreground text-xs font-bold hover:opacity-90 transition-all shadow-sm",
                                                    disabled: *is_submitting.read(),
                                                    onclick: handle_send_thread_message,
                                                    "Send"
                                                }
                                            }
                                        }
                                    } else {
                                        div { class: "space-y-4",
                                            div { class: "p-3 rounded-xl bg-muted/20 border border-border/40 space-y-3",
                                                h4 { class: "font-bold text-xs text-foreground m-0 flex items-center gap-1.5",
                                                    components::LucideIcon { name: "plus-circle", class: "h-3.5 w-3.5 text-primary" }
                                                    "Submit Support Ticket"
                                                }
                                                input {
                                                    r#type: "text",
                                                    placeholder: "Subject summary...",
                                                    value: "{ticket_subject}",
                                                    class: "w-full px-3 py-1.5 rounded-xl border border-input bg-background text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary/50",
                                                    oninput: move |e: Event<FormData>| ticket_subject.set(e.value())
                                                }
                                                textarea {
                                                    placeholder: "Describe your question or issue...",
                                                    value: "{ticket_message}",
                                                    rows: "3",
                                                    class: "w-full px-3 py-1.5 rounded-xl border border-input bg-background text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary/50 resize-none",
                                                    oninput: move |e: Event<FormData>| ticket_message.set(e.value())
                                                }
                                                button {
                                                    class: "w-full py-2 rounded-xl bg-primary text-primary-foreground text-xs font-bold hover:opacity-90 transition-all shadow-sm",
                                                    disabled: *is_submitting.read(),
                                                    onclick: handle_create_ticket,
                                                    {if *is_submitting.read() { "Submitting..." } else { "Submit Support Ticket" }}
                                                }
                                            }

                                            match &*tickets_res.read() {
                                                Some(Ok(tickets)) if !tickets.is_empty() => rsx! {
                                                    div { class: "space-y-2",
                                                        h5 { class: "font-bold text-[11px] text-muted-foreground uppercase tracking-wider m-0", "Your Support Tickets" }
                                                        for t in tickets.iter() {
                                                            div {
                                                                key: "{t.id}",
                                                                class: "p-3 rounded-xl border border-border/30 bg-card hover:bg-accent/40 cursor-pointer transition-colors space-y-1",
                                                                onclick: {
                                                                    let t_clone = t.clone();
                                                                    move |_| selected_ticket.set(Some(t_clone.clone()))
                                                                },
                                                                div { class: "flex items-center justify-between",
                                                                    span { class: "font-bold text-xs text-foreground", "{t.subject}" }
                                                                    span { class: "px-2 py-0.5 rounded-full text-[9px] font-bold uppercase bg-primary/10 text-primary border border-primary/20",
                                                                        "{t.status}"
                                                                    }
                                                                }
                                                                p { class: "text-[11px] text-muted-foreground m-0", "Category: {t.category} | Priority: {t.priority}" }
                                                            }
                                                        }
                                                    }
                                                },
                                                _ => rsx! {
                                                    div { class: "py-4 text-center text-xs text-muted-foreground", "No tickets submitted yet" }
                                                }
                                            }
                                        }
                                    }
                                }
                            },

                            _ => rsx! { div {} }
                        }
                    }
                }
            }

            button {
                class: "h-14 w-14 rounded-full bg-primary text-primary-foreground shadow-2xl hover:scale-105 transition-all flex items-center justify-center ring-4 ring-primary/20",
                onclick: move |_| {
                    let cur = *is_open.read();
                    is_open.set(!cur);
                },
                components::LucideIcon { name: if *is_open.read() { "x" } else { "life-buoy" }, class: "h-6 w-6" }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct ProductTourOverlayProps {
    pub active_user: WorkspaceUser,
    pub workspace: Workspace,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

#[component]
pub fn ProductTourOverlay(props: ProductTourOverlayProps) -> Element {
    let active_user_id = props.active_user.id.clone();
    let workspace_id = props.workspace.id.clone();
    let mut db_trigger = props.db_trigger;

    let steps_res = use_resource(move || {
        let uid = active_user_id.clone();
        async move { yntra_core::get_product_tour_steps(uid, "onboarding_tour".to_string()).await }
    });

    let active_user_id_p = props.active_user.id.clone();
    let workspace_id_p = props.workspace.id.clone();

    let progress_res = use_resource(move || {
        let uid = active_user_id_p.clone();
        let wsid = workspace_id_p.clone();
        let _trig = *db_trigger.read();
        async move { yntra_core::get_user_tour_progress(uid, wsid, "onboarding_tour".to_string()).await }
    });

    let progress = match &*progress_res.read() {
        Some(Ok(p)) => Some(p.clone()),
        _ => None,
    };

    if progress.as_ref().map(|p| p.completed).unwrap_or(true) {
        return rsx! {};
    }

    let current_step_num = progress.as_ref().map(|p| p.current_step).unwrap_or(1);
    let steps_list = match &*steps_res.read() {
        Some(Ok(list)) => list.clone(),
        _ => vec![],
    };

    let active_step = steps_list
        .iter()
        .find(|s| s.step_number == current_step_num)
        .cloned();

    if active_step.is_none() {
        return rsx! {};
    }

    let step = active_step.unwrap();

    let handle_next_step = {
        let req_uid = props.active_user.id.clone();
        let wsid = props.workspace.id.clone();
        let next_s = current_step_num + 1;
        move |_| {
            let uid = req_uid.clone();
            let ws = wsid.clone();
            spawn(async move {
                let _ = yntra_core::complete_product_tour_step(
                    uid,
                    ws,
                    "onboarding_tour".to_string(),
                    next_s,
                )
                .await;
                let trig_val = *db_trigger.read();
                db_trigger.set(trig_val + 1);
            });
        }
    };

    let handle_skip = {
        let req_uid = props.active_user.id.clone();
        let wsid = props.workspace.id.clone();
        move |_| {
            let uid = req_uid.clone();
            let ws = wsid.clone();
            spawn(async move {
                let _ = yntra_core::complete_product_tour_step(
                    uid,
                    ws,
                    "onboarding_tour".to_string(),
                    5,
                )
                .await;
                let trig_val = *db_trigger.read();
                db_trigger.set(trig_val + 1);
            });
        }
    };

    rsx! {
        div { class: "fixed inset-0 z-50 pointer-events-none flex items-center justify-center p-4 bg-background/40 backdrop-blur-sm animate-in fade-in duration-200",
            div { class: "pointer-events-auto max-w-md w-full bg-card border border-primary/40 rounded-2xl p-6 shadow-2xl space-y-4 ring-2 ring-primary/20",
                div { class: "flex items-center justify-between border-b border-border/30 pb-3",
                    div { class: "flex items-center gap-2",
                        components::LucideIcon { name: "compass", class: "h-5 w-5 text-primary" }
                        span { class: "font-extrabold text-sm text-foreground", "Product Tour Step {step.step_number} / 5" }
                    }
                    span { class: "text-[10px] font-bold uppercase tracking-wider bg-primary/10 text-primary px-2.5 py-0.5 rounded-full border border-primary/20",
                        "{step.tour_name}"
                    }
                }

                div { class: "space-y-2",
                    h3 { class: "text-base font-extrabold text-foreground m-0", "{step.title}" }
                    p { class: "text-xs text-muted-foreground leading-relaxed m-0", "{step.description}" }
                }

                div { class: "flex items-center justify-between pt-4 border-t border-border/30",
                    button {
                        class: "text-xs font-semibold text-muted-foreground hover:text-foreground transition-colors",
                        onclick: handle_skip,
                        "Skip Tour"
                    }

                    button {
                        class: "px-4 py-2 rounded-xl bg-primary text-primary-foreground text-xs font-bold hover:opacity-90 transition-all shadow-sm flex items-center gap-1.5",
                        onclick: handle_next_step,
                        {if step.step_number >= 5 { "Finish Tour" } else { "Next Step" }}
                        components::LucideIcon { name: "arrow-right", class: "h-3.5 w-3.5" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn SupportCenterView(
    active_user_id: String,
    workspace_id: String,
    block_id: String,
    db_trigger: Signal<u32>,
    locale: String,
) -> Element {
    rsx! {
        div { class: "p-6 space-y-6 max-w-5xl mx-auto animate-in fade-in duration-300",
            div { class: "space-y-1 border-b border-border/40 pb-4",
                h2 { class: "text-2xl font-extrabold text-foreground tracking-tight flex items-center gap-2",
                    components::LucideIcon { name: "life-buoy", class: "h-6 w-6 text-primary" }
                    "Support & Knowledge Desk Center"
                }
                p { class: "text-xs text-muted-foreground",
                    "Browse self-serve help articles, start interactive onboarding tours, or communicate directly with Yntra support specialists."
                }
            }

            div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                div { class: "p-6 rounded-2xl bg-card border border-border/40 shadow-sm space-y-3",
                    h3 { class: "font-bold text-base text-foreground m-0 flex items-center gap-2",
                        components::LucideIcon { name: "book-open", class: "h-5 w-5 text-primary" }
                        "Self-Serve Knowledge Base"
                    }
                    p { class: "text-xs text-muted-foreground m-0", "Search through guides, SITHS smart card setups, CalDAV roster sync, and billing manuals." }
                }

                div { class: "p-6 rounded-2xl bg-card border border-border/40 shadow-sm space-y-3",
                    h3 { class: "font-bold text-base text-foreground m-0 flex items-center gap-2",
                        components::LucideIcon { name: "message-square", class: "h-5 w-5 text-primary" }
                        "Direct Support Ticketing"
                    }
                    p { class: "text-xs text-muted-foreground m-0", "Submit questions directly to system admins and track response progress in real time." }
                }
            }
        }
    }
}
