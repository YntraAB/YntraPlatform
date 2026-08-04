use crate::components;
use dioxus::prelude::*;
use yntra_core::{ActionTrigger, DailyAiDigest, VoiceReportProposal, Workspace, WorkspaceUser};

#[derive(Props, Clone, PartialEq)]
pub struct AgenticAiProps {
    pub active_user: WorkspaceUser,
    pub workspace: Workspace,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

#[component]
pub fn AgenticAiView(props: AgenticAiProps) -> Element {
    let mut active_tab = use_signal(|| "voice".to_string());

    // Voice Automation State
    let mut voice_transcript = use_signal(|| "Logged 9.5 hours today doing emergency HVAC system diagnostics and client consultation. Overtime required.".to_string());
    let mut parsed_proposal = use_signal(|| Option::<VoiceReportProposal>::None);
    let mut voice_status_msg = use_signal(|| Option::<String>::None);
    let mut is_submitting_voice = use_signal(|| false);

    // Natural Language Action Triggers State
    let mut trigger_prompt = use_signal(|| "flag time reports over 8h for approval".to_string());
    let mut trigger_status_msg = use_signal(|| Option::<String>::None);
    let mut is_evaluating_triggers = use_signal(|| false);
    let mut eval_summary = use_signal(|| Option::<String>::None);

    // Daily AI Digest State
    let mut digest_date = use_signal(|| "2026-08-04".to_string());
    let mut generated_digest = use_signal(|| Option::<DailyAiDigest>::None);
    let mut is_generating_digest = use_signal(|| false);

    let active_user_id = props.active_user.id.clone();
    let workspace_id = props.workspace.id.clone();
    let mut db_trigger = props.db_trigger;

    // Asynchronous resource for action triggers list
    let uid_trig = active_user_id.clone();
    let ws_trig = workspace_id.clone();
    let triggers_res = use_resource(move || {
        let u = uid_trig.clone();
        let w = ws_trig.clone();
        let _t = *db_trigger.read();
        async move {
            yntra_core::get_action_triggers(u, w).await
        }
    });

    // Parse voice transcript handler
    let handle_parse_voice = move |_| {
        let transcript = voice_transcript.read().clone();
        let proposal = yntra_core::parse_voice_report_to_proposal(transcript, Some(digest_date.read().clone()));
        parsed_proposal.set(Some(proposal));
        voice_status_msg.set(Some("Voice transcript analyzed and converted into structured report proposal.".to_string()));
    };

    // Submit voice time report handler
    let uid_submit = active_user_id.clone();
    let ws_submit = workspace_id.clone();
    let handle_submit_voice = move |_| {
        is_submitting_voice.set(true);
        voice_status_msg.set(None);
        let u = uid_submit.clone();
        let w = ws_submit.clone();
        let transcript = voice_transcript.read().clone();
        let date_str = digest_date.read().clone();

        spawn(async move {
            match yntra_core::submit_voice_time_report(u, w, transcript, Some(date_str)).await {
                Ok(report) => {
                    is_submitting_voice.set(false);
                    voice_status_msg.set(Some(format!("Time report logged successfully! (ID: {}, Hours: {}, Status: {})", report.id, report.hours, report.status)));
                    db_trigger.with_mut(|v| *v += 1);
                }
                Err(e) => {
                    is_submitting_voice.set(false);
                    voice_status_msg.set(Some(format!("Error logging report: {}", e)));
                }
            }
        });
    };

    // Create action trigger handler
    let uid_create_trig = active_user_id.clone();
    let ws_create_trig = workspace_id.clone();
    let handle_create_trigger = move |_| {
        let prompt = trigger_prompt.read().clone();
        if prompt.trim().is_empty() {
            return;
        }
        let u = uid_create_trig.clone();
        let w = ws_create_trig.clone();

        spawn(async move {
            match yntra_core::create_natural_language_trigger(u, w, prompt).await {
                Ok(trig) => {
                    trigger_status_msg.set(Some(format!("Created trigger: '{}' ({})", trig.natural_language_prompt, trig.condition_type)));
                    trigger_prompt.set(String::new());
                    db_trigger.with_mut(|v| *v += 1);
                }
                Err(e) => {
                    trigger_status_msg.set(Some(format!("Error creating trigger: {}", e)));
                }
            }
        });
    };

    // Evaluate pending triggers handler
    let uid_eval = active_user_id.clone();
    let ws_eval = workspace_id.clone();
    let handle_evaluate_triggers = move |_| {
        is_evaluating_triggers.set(true);
        eval_summary.set(None);
        let u = uid_eval.clone();
        let w = ws_eval.clone();

        spawn(async move {
            match yntra_core::evaluate_pending_triggers(u, w).await {
                Ok(res) => {
                    is_evaluating_triggers.set(false);
                    eval_summary.set(Some(res.summary));
                    db_trigger.with_mut(|v| *v += 1);
                }
                Err(e) => {
                    is_evaluating_triggers.set(false);
                    eval_summary.set(Some(format!("Evaluation error: {}", e)));
                }
            }
        });
    };

    // Generate daily AI digest handler
    let uid_digest = active_user_id.clone();
    let ws_digest = workspace_id.clone();
    let handle_generate_digest = move |_| {
        is_generating_digest.set(true);
        let u = uid_digest.clone();
        let w = ws_digest.clone();
        let d = digest_date.read().clone();

        spawn(async move {
            match yntra_core::generate_daily_ai_digest(u, w, d).await {
                Ok(digest) => {
                    is_generating_digest.set(false);
                    generated_digest.set(Some(digest));
                    db_trigger.with_mut(|v| *v += 1);
                }
                Err(e) => {
                    is_generating_digest.set(false);
                }
            }
        });
    };

    rsx! {
        div { class: "p-6 max-w-7xl mx-auto space-y-6",
            // Header Section
            div { class: "flex justify-between items-center bg-gray-900 border border-gray-800 rounded-xl p-6 shadow-xl",
                div {
                    h1 { class: "text-2xl font-bold text-white flex items-center gap-3",
                        span { class: "text-blue-400 text-3xl", "🤖" }
                        "Agentic AI & Voice Automation"
                    }
                    p { class: "text-gray-400 text-sm mt-1",
                        "SOTA voice-to-structured reporting, natural language trigger rules, and automated daily AI summaries."
                    }
                }
                div { class: "flex gap-2 bg-gray-800 p-1.5 rounded-lg border border-gray-700",
                    button {
                        class: if *active_tab.read() == "voice" { "px-4 py-2 text-sm font-medium rounded-md bg-blue-600 text-white shadow-md transition" } else { "px-4 py-2 text-sm font-medium rounded-md text-gray-300 hover:text-white transition" },
                        onclick: move |_| active_tab.set("voice".to_string()),
                        "🎙️ Voice Reports"
                    }
                    button {
                        class: if *active_tab.read() == "triggers" { "px-4 py-2 text-sm font-medium rounded-md bg-blue-600 text-white shadow-md transition" } else { "px-4 py-2 text-sm font-medium rounded-md text-gray-300 hover:text-white transition" },
                        onclick: move |_| active_tab.set("triggers".to_string()),
                        "⚡ Action Triggers"
                    }
                    button {
                        class: if *active_tab.read() == "digest" { "px-4 py-2 text-sm font-medium rounded-md bg-blue-600 text-white shadow-md transition" } else { "px-4 py-2 text-sm font-medium rounded-md text-gray-300 hover:text-white transition" },
                        onclick: move |_| active_tab.set("digest".to_string()),
                        "📊 Daily AI Digest"
                    }
                }
            }

            // Voice Automation View
            if *active_tab.read() == "voice" {
                div { class: "grid grid-cols-1 lg:grid-cols-2 gap-6",
                    // Left Input Card
                    div { class: "bg-gray-900 border border-gray-800 rounded-xl p-6 space-y-4 shadow-lg",
                        h2 { class: "text-lg font-semibold text-white flex items-center gap-2",
                            "🎤 Mobile Voice Audio / Transcript Input"
                        }
                        p { class: "text-gray-400 text-xs",
                            "Speak or type natural language report. Core AI automatically parses hours, dates, categories, and policy flags."
                        }
                        div { class: "space-y-2",
                            label { class: "text-xs font-semibold text-gray-300 uppercase tracking-wider", "Spoken Audio Transcript" }
                            textarea {
                                class: "w-full h-32 bg-gray-950 border border-gray-800 rounded-lg p-3 text-gray-200 text-sm focus:border-blue-500 focus:outline-none transition",
                                value: "{voice_transcript}",
                                oninput: move |e| voice_transcript.set(e.value())
                            }
                        }
                        div { class: "flex gap-2 flex-wrap",
                            button {
                                class: "px-3 py-1.5 bg-gray-800 hover:bg-gray-700 text-xs text-gray-300 rounded-md border border-gray-700 transition",
                                onclick: move |_| voice_transcript.set("Logged 9.5 hours emergency HVAC system repair & consultation. Overtime.".to_string()),
                                "Preset: 9.5h Overtime"
                            }
                            button {
                                class: "px-3 py-1.5 bg-gray-800 hover:bg-gray-700 text-xs text-gray-300 rounded-md border border-gray-700 transition",
                                onclick: move |_| voice_transcript.set("Worked 4 hours on morning client onboarding and routine maintenance.".to_string()),
                                "Preset: 4h Standard"
                            }
                        }
                        div { class: "flex gap-3 pt-2",
                            button {
                                class: "flex-1 px-4 py-2.5 bg-blue-600 hover:bg-blue-500 text-white font-medium text-sm rounded-lg shadow transition",
                                onclick: handle_parse_voice,
                                "🔍 Analyze Transcript"
                            }
                            button {
                                class: "flex-1 px-4 py-2.5 bg-green-600 hover:bg-green-500 text-white font-medium text-sm rounded-lg shadow transition disabled:opacity-50",
                                disabled: *is_submitting_voice.read(),
                                onclick: handle_submit_voice,
                                if *is_submitting_voice.read() { "Logging..." } else { "🚀 Submit Time Report" }
                            }
                        }
                        if let Some(msg) = voice_status_msg.read().as_ref() {
                            div { class: "p-3 bg-blue-950/50 border border-blue-800 text-blue-300 text-xs rounded-lg", "{msg}" }
                        }
                    }

                    // Right Proposal Preview Card
                    div { class: "bg-gray-900 border border-gray-800 rounded-xl p-6 space-y-4 shadow-lg",
                        h2 { class: "text-lg font-semibold text-white flex items-center gap-2",
                            "✨ Structured Proposal Output"
                        }
                        if let Some(prop) = parsed_proposal.read().as_ref() {
                            div { class: "space-y-4 bg-gray-950 border border-gray-800 rounded-lg p-5",
                                div { class: "flex justify-between items-center border-b border-gray-800 pb-3",
                                    span { class: "text-sm text-gray-400", "Parsed Hours:" }
                                    span { class: "text-xl font-bold text-blue-400", "{prop.hours} hrs" }
                                }
                                div { class: "flex justify-between items-center border-b border-gray-800 pb-3",
                                    span { class: "text-sm text-gray-400", "Category / Sector:" }
                                    span { class: "text-sm font-semibold text-emerald-400 bg-emerald-950/50 px-2.5 py-1 rounded border border-emerald-800", "{prop.category}" }
                                }
                                div { class: "flex justify-between items-center border-b border-gray-800 pb-3",
                                    span { class: "text-sm text-gray-400", "Policy Approval Flag:" }
                                    span {
                                        class: if prop.requires_approval { "text-xs font-bold text-amber-400 bg-amber-950/60 px-2.5 py-1 rounded border border-amber-800" } else { "text-xs font-bold text-green-400 bg-green-950/60 px-2.5 py-1 rounded border border-green-800" },
                                        if prop.requires_approval { "⚠️ Manager Attestation Needed (> 8h)" } else { "✅ Auto Approval Eligible" }
                                    }
                                }
                                div { class: "flex justify-between items-center border-b border-gray-800 pb-3",
                                    span { class: "text-sm text-gray-400", "AI Confidence Score:" }
                                    span { class: "text-sm font-semibold text-purple-400", "{prop.confidence_score * 100.0:.0}%" }
                                }
                                div { class: "space-y-1",
                                    span { class: "text-xs text-gray-400 uppercase tracking-wider font-semibold", "Extracted Note:" }
                                    p { class: "text-xs text-gray-300 bg-gray-900 p-3 rounded border border-gray-800 italic", "\"{prop.note}\"" }
                                }
                            }
                        } else {
                            div { class: "h-64 flex flex-col items-center justify-center text-gray-500 border border-dashed border-gray-800 rounded-lg p-6 text-center",
                                span { class: "text-4xl mb-2", "🧠" }
                                p { class: "text-sm font-medium", "No transcript analyzed yet" }
                                p { class: "text-xs text-gray-600 mt-1", "Click 'Analyze Transcript' to view AI voice extraction results." }
                            }
                        }
                    }
                }
            }

            // Natural Language Action Triggers View
            if *active_tab.read() == "triggers" {
                div { class: "space-y-6",
                    // Create Trigger Box
                    div { class: "bg-gray-900 border border-gray-800 rounded-xl p-6 space-y-4 shadow-lg",
                        h2 { class: "text-lg font-semibold text-white flex items-center gap-2",
                            "⚡ Natural Language Action Triggers"
                        }
                        p { class: "text-gray-400 text-xs",
                            "Enter policy rules in plain English. The rule engine compiles them into active database triggers."
                        }
                        div { class: "flex gap-3",
                            input {
                                class: "flex-1 bg-gray-950 border border-gray-800 rounded-lg px-4 py-2.5 text-sm text-white focus:border-blue-500 focus:outline-none transition",
                                placeholder: "e.g., flag time reports over 8h for approval",
                                value: "{trigger_prompt}",
                                oninput: move |e| trigger_prompt.set(e.value())
                            }
                            button {
                                class: "px-5 py-2.5 bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium rounded-lg shadow transition",
                                onclick: handle_create_trigger,
                                "➕ Add Trigger"
                            }
                            button {
                                class: "px-5 py-2.5 bg-gray-800 hover:bg-gray-700 text-gray-200 text-sm font-medium rounded-lg border border-gray-700 transition disabled:opacity-50",
                                disabled: *is_evaluating_triggers.read(),
                                onclick: handle_evaluate_triggers,
                                if *is_evaluating_triggers.read() { "Evaluating..." } else { "🔄 Run Policy Engine" }
                            }
                        }
                        if let Some(msg) = trigger_status_msg.read().as_ref() {
                            div { class: "p-3 bg-blue-950/50 border border-blue-800 text-blue-300 text-xs rounded-lg", "{msg}" }
                        }
                        if let Some(summary) = eval_summary.read().as_ref() {
                            div { class: "p-3 bg-purple-950/50 border border-purple-800 text-purple-300 text-xs rounded-lg font-mono", "{summary}" }
                        }
                    }

                    // Active Triggers Table
                    div { class: "bg-gray-900 border border-gray-800 rounded-xl p-6 shadow-lg",
                        h3 { class: "text-md font-semibold text-white mb-4", "Active Rule Triggers" }
                        if let Some(Ok(triggers)) = triggers_res.read().as_ref() {
                            if triggers.is_empty() {
                                div { class: "text-center text-gray-500 py-8 text-sm", "No active triggers defined." }
                            } else {
                                div { class: "overflow-x-auto",
                                    table { class: "w-full text-left border-collapse",
                                        thead {
                                            tr { class: "border-b border-gray-800 text-xs font-semibold text-gray-400 uppercase tracking-wider",
                                                th { class: "p-3", "Natural Language Prompt" }
                                                th { class: "p-3", "Condition Type" }
                                                th { class: "p-3", "Action Type" }
                                                th { class: "p-3", "Status" }
                                                th { class: "p-3 text-right", "Actions" }
                                            }
                                        }
                                        tbody {
                                            for trig in triggers.iter() {
                                                tr { key: "{trig.id}", class: "border-b border-gray-800/60 hover:bg-gray-800/30 text-sm text-gray-200 transition",
                                                    td { class: "p-3 font-medium text-white", "{trig.natural_language_prompt}" }
                                                    td { class: "p-3 font-mono text-xs text-blue-400", "{trig.condition_type}" }
                                                    td { class: "p-3 font-mono text-xs text-amber-400", "{trig.action_type}" }
                                                    td { class: "p-3",
                                                        span { class: "px-2 py-0.5 text-xs font-bold bg-emerald-950 text-emerald-400 border border-emerald-800 rounded", "ACTIVE" }
                                                    }
                                                    td { class: "p-3 text-right",
                                                        {
                                                            let tid = trig.id.clone();
                                                            let uid_del = active_user_id.clone();
                                                            let mut db_trig_del = db_trigger;
                                                            rsx! {
                                                                button {
                                                                    class: "px-3 py-1 bg-red-950/60 hover:bg-red-900 border border-red-800 text-red-300 text-xs rounded transition",
                                                                    onclick: move |_| {
                                                                        let t = tid.clone();
                                                                        let u = uid_del.clone();
                                                                        spawn(async move {
                                                                            let _ = yntra_core::delete_action_trigger(u, t).await;
                                                                            db_trig_del.with_mut(|v| *v += 1);
                                                                        });
                                                                    },
                                                                    "Delete"
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

            // Daily AI Digest View
            if *active_tab.read() == "digest" {
                div { class: "space-y-6",
                    // Controls Box
                    div { class: "bg-gray-900 border border-gray-800 rounded-xl p-6 flex flex-wrap justify-between items-center gap-4 shadow-lg",
                        div { class: "flex items-center gap-3",
                            label { class: "text-sm text-gray-300 font-medium", "Target Date:" }
                            input {
                                class: "bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-white focus:border-blue-500 focus:outline-none",
                                value: "{digest_date}",
                                oninput: move |e| digest_date.set(e.value())
                            }
                        }
                        button {
                            class: "px-6 py-2.5 bg-blue-600 hover:bg-blue-500 text-white font-medium text-sm rounded-lg shadow transition disabled:opacity-50",
                            disabled: *is_generating_digest.read(),
                            onclick: handle_generate_digest,
                            if *is_generating_digest.read() { "Generating AI Digest..." } else { "🪄 Generate AI Digest" }
                        }
                    }

                    // Digest Presentation Card
                    if let Some(digest) = generated_digest.read().as_ref() {
                        div { class: "bg-gray-900 border border-gray-800 rounded-xl p-6 space-y-6 shadow-xl",
                            // Summary Cards
                            div { class: "grid grid-cols-2 md:grid-cols-4 gap-4",
                                div { class: "bg-gray-950 border border-gray-800 p-4 rounded-lg",
                                    span { class: "text-xs text-gray-400 font-medium uppercase", "Total Logged Hours" }
                                    p { class: "text-2xl font-bold text-blue-400 mt-1", "{digest.total_hours_logged:.1} hrs" }
                                }
                                div { class: "bg-gray-950 border border-gray-800 p-4 rounded-lg",
                                    span { class: "text-xs text-gray-400 font-medium uppercase", "Total Reports" }
                                    p { class: "text-2xl font-bold text-gray-200 mt-1", "{digest.total_reports_count}" }
                                }
                                div { class: "bg-gray-950 border border-gray-800 p-4 rounded-lg",
                                    span { class: "text-xs text-gray-400 font-medium uppercase", "Flagged Items" }
                                    p { class: "text-2xl font-bold text-amber-400 mt-1", "{digest.flagged_reports_count}" }
                                }
                                div { class: "bg-gray-950 border border-gray-800 p-4 rounded-lg",
                                    span { class: "text-xs text-gray-400 font-medium uppercase", "Audit Events" }
                                    p { class: "text-2xl font-bold text-purple-400 mt-1", "{digest.audit_events_count}" }
                                }
                            }

                            // Markdown Render Box
                            div { class: "bg-gray-950 border border-gray-800 rounded-lg p-6 font-mono text-xs text-gray-300 leading-relaxed whitespace-pre-wrap",
                                "{digest.markdown_digest}"
                            }
                        }
                    } else {
                        div { class: "bg-gray-900 border border-gray-800 rounded-xl p-12 text-center text-gray-500 shadow-lg",
                            span { class: "text-5xl block mb-3", "📊" }
                            p { class: "text-md font-semibold text-gray-300", "No Daily AI Digest Generated Yet" }
                            p { class: "text-xs text-gray-500 mt-1", "Select a date and click 'Generate AI Digest' to build an automated summary." }
                        }
                    }
                }
            }
        }
    }
}
