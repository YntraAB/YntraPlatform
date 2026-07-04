use crate::components;
use crate::locales;
use dioxus::prelude::*;
use yntra_core::StudentProfile;

#[derive(Props, Clone)]
pub struct HealthRegistryProps {
    pub students: Vec<StudentProfile>,
    pub workspace_id: String,
    pub db_trigger: Signal<u32>,
    pub locale: String,
}

impl PartialEq for HealthRegistryProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn HealthRegistry(props: HealthRegistryProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let requester_user_id = state.active_user_id.read().clone();
    let students = props.students.clone();
    let workspace_id = props.workspace_id.clone();
    let mut db_trigger = props.db_trigger;
    let locale = props.locale.clone();
    let db_trig = *db_trigger.read();

    let mut selected_student_id = use_signal(|| students.first().map(|s| s.id.clone()));
    let selected_student = students.iter().find(|s| Some(s.id.clone()) == *selected_student_id.read()).cloned();

    // Fetch health records
    let s_id = selected_student_id.read().clone().unwrap_or_default();
    let req_id_records = requester_user_id.clone();
    let records_res = use_resource(move || {
        let _ = db_trig;
        let id = s_id.clone();
        let r_id = req_id_records.clone();
        async move {
            yntra_core::get_health_records(r_id, id).await.unwrap_or_default()
        }
    });
    let records = records_res.read().clone().unwrap_or_default();

    // Fetch incidents
    let s_id_inc = selected_student_id.read().clone().unwrap_or_default();
    let req_id_incidents = requester_user_id.clone();
    let incidents_res = use_resource(move || {
        let _ = db_trig;
        let id = s_id_inc.clone();
        let r_id = req_id_incidents.clone();
        async move {
            yntra_core::get_health_incidents(r_id, id).await.unwrap_or_default()
        }
    });
    let incidents = incidents_res.read().clone().unwrap_or_default();

    // Form inputs for Nurse Log
    let mut visit_reason = use_signal(String::new);
    let mut treatment = use_signal(String::new);
    let mut notes = use_signal(String::new);

    // Form inputs for Immunization update
    let mut editing_vaccine = use_signal(|| Option::<String>::None);
    let mut vaccine_status = use_signal(|| "completed".to_string());
    let mut vaccine_date = use_signal(String::new);

    rsx! {
        div { class: "flex flex-col gap-6",
            // Hero Title block
            div {
                class: "relative p-6 rounded-2xl overflow-hidden border border-primary/20 shadow-lg flex flex-col md:flex-row justify-between items-start md:items-center gap-4 bg-gradient-to-r from-primary/10 via-purple-500/5 to-indigo-500/10",
                div { class: "flex items-center gap-4",
                    div { class: "p-3 rounded-xl bg-primary/10 text-primary",
                        components::LucideIcon { name: "heart", size: "32", class: "text-primary" }
                    }
                    div { class: "flex flex-col gap-1",
                        h2 { class: "text-2xl font-black text-foreground m-0 flex items-center gap-2",
                            {locales::t("school-health-title", &locale)}
                        }
                        p { class: "text-xs text-muted-foreground m-0 font-medium",
                            {locales::t("school-health-desc", &locale)}
                        }
                    }
                }
            }

            // Main Grid
            div { class: "grid gap-6 lg:grid-cols-4 items-start",
                // Left Column: Student List
                div { class: "lg:col-span-1 flex flex-col gap-4 bg-sidebar/20 border border-border/40 p-4 rounded-xl shadow-sm",
                    h3 { class: "text-xs font-black uppercase text-muted-foreground tracking-wider m-0 px-1 pb-2 border-b border-border/30",
                        {locales::t("school-report-roster", &locale)}
                    }
                    div { class: "flex flex-col gap-1.5 max-h-[500px] overflow-y-auto",
                        for s in students.iter() {
                            {
                                let s_id = s.id.clone();
                                let s_name = format!("{} {}", s.first_name, s.last_name);
                                let s_grade = s.grade_level.clone();
                                let is_selected = Some(s_id.clone()) == *selected_student_id.read();
                                let bg_cls = if is_selected { "bg-primary/10 border-primary text-primary" } else { "bg-transparent border-transparent text-muted-foreground hover:bg-muted hover:text-foreground" };
                                rsx! {
                                    button {
                                        key: "{s_id}",
                                        class: "w-full text-left p-3 rounded-xl border transition-all flex justify-between items-center {bg_cls}",
                                        onclick: move |_| {
                                            selected_student_id.set(Some(s_id.clone()));
                                            editing_vaccine.set(None);
                                        },
                                        div { class: "flex flex-col gap-0.5",
                                            span { class: "text-xs font-black", "{s_name}" }
                                            span { class: "text-[10px] opacity-75 font-bold", "Grade: {s_grade}" }
                                        }
                                        components::LucideIcon { name: "chevron-right", size: "14" }
                                    }
                                }
                            }
                        }
                    }
                }

                // Right Columns: Selected Student medical information
                div { class: "lg:col-span-3 flex flex-col gap-6",
                    if let Some(ref student) = selected_student {
                        div { class: "flex flex-col gap-6",
                            // Selected Student Banner & Medical Alert Card
                            div { class: "flex flex-col gap-4 p-5 border border-border/40 bg-sidebar/10 rounded-2xl shadow-md",
                                div { class: "flex justify-between items-center flex-wrap gap-4",
                                    div { class: "flex items-center gap-3",
                                        div { class: "p-3 rounded-xl bg-emerald-500/10 text-emerald-400 shadow-inner",
                                            components::LucideIcon { name: "directory", size: "24" }
                                        }
                                        div { class: "flex flex-col gap-0.5",
                                            h3 { class: "text-lg font-black text-foreground m-0 tracking-tight", "{student.first_name} {student.last_name}" }
                                            span { class: "text-xs text-muted-foreground font-semibold flex items-center gap-1.5", 
                                                components::LucideIcon { name: "phone", size: "12", class: "text-muted-foreground" }
                                                "Emergency Contact: {student.parent_contact.clone().unwrap_or_else(|| \"None\".to_string())}"
                                            }
                                        }
                                    }
                                    
                                    // Blood Type & Key Vitals
                                    div { class: "flex items-center gap-3 bg-sidebar/20 p-2.5 rounded-xl border border-border/30",
                                        div { class: "flex flex-col items-center px-3 border-r border-border/30",
                                            span { class: "text-[9px] font-black uppercase text-muted-foreground tracking-widest", "Blood Type" }
                                            span { class: "text-sm font-black text-rose-400", "O-Positive" }
                                        }
                                        div { class: "flex flex-col items-center px-2",
                                            span { class: "text-[9px] font-black uppercase text-muted-foreground tracking-widest", "Health Alert" }
                                            span { class: "text-[10px] font-black px-2 py-0.5 rounded-full bg-rose-500/10 text-rose-400 border border-rose-500/15 animate-pulse", "Peanut Allergy" }
                                        }
                                    }
                                }

                                // Quick Info Bar
                                div { class: "grid gap-3 grid-cols-2 md:grid-cols-4 border-t border-border/20 pt-4 text-xs font-semibold",
                                    div { class: "bg-sidebar/30 p-3 rounded-xl border border-border/10 flex flex-col gap-0.5",
                                        span { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "Asthma Inhaler" }
                                        span { class: "text-foreground", "At Reception Office" }
                                    }
                                    div { class: "bg-sidebar/30 p-3 rounded-xl border border-border/10 flex flex-col gap-0.5",
                                        span { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "Allergy Severity" }
                                        span { class: "text-amber-400 font-bold", "Severe (EpiPen Active)" }
                                    }
                                    div { class: "bg-sidebar/30 p-3 rounded-xl border border-border/10 flex flex-col gap-0.5",
                                        span { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "Height / Weight" }
                                        span { class: "text-foreground", "142 cm / 36.5 kg" }
                                    }
                                    div { class: "bg-sidebar/30 p-3 rounded-xl border border-border/10 flex flex-col gap-0.5",
                                        span { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "General Fitness" }
                                        span { class: "text-emerald-400 font-bold", "Excellent (BMI 18.1)" }
                                    }
                                }
                            }

                            // Sub-grids: Vaccine schedule & Incident logging
                            div { class: "grid gap-6 md:grid-cols-2",
                                // Section A: Vaccine Schedules
                                components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4",
                                    h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                        components::LucideIcon { name: "award", size: "16", class: "text-primary" }
                                        {locales::t("school-health-immunizations", &locale)}
                                    }

                                    div { class: "flex flex-col gap-3",
                                        for v in vec!["Covid-19", "MMR", "Polio", "DTaP", "Hepatitis B"] {
                                            {
                                                let record = records.iter().find(|r| r.vaccine_name == v);
                                                let status = record.map(|r| r.status.clone()).unwrap_or_else(|| "pending".to_string());
                                                let admin_date = record.and_then(|r| r.administered_at.clone());
                                                let is_editing = editing_vaccine.read().as_ref() == Some(&v.to_string());
                                                
                                                rsx! {
                                                    div { class: "border border-border/30 p-3 rounded-lg bg-sidebar/40 flex flex-col gap-2",
                                                        div { class: "flex justify-between items-center",
                                                            div { class: "flex flex-col gap-0.5",
                                                                span { class: "text-xs font-black text-foreground", "{v}" }
                                                                if let Some(date) = admin_date {
                                                                    span { class: "text-[10px] text-muted-foreground font-bold", "Administered: {date}" }
                                                                } else {
                                                                    span { class: "text-[10px] text-muted-foreground italic", "No date recorded" }
                                                                }
                                                            }
                                                            
                                                            div { class: "flex items-center gap-2",
                                                                match status.as_str() {
                                                                    "completed" => rsx! { span { class: "px-2 py-0.5 rounded text-[9px] font-black bg-emerald-500/10 text-emerald-400 border border-emerald-500/20", "Completed" } },
                                                                    "exempted" => rsx! { span { class: "px-2 py-0.5 rounded text-[9px] font-black bg-indigo-500/10 text-indigo-400 border border-indigo-500/20", "Exempt" } },
                                                                    _ => rsx! { span { class: "px-2 py-0.5 rounded text-[9px] font-black bg-amber-500/10 text-amber-400 border border-amber-500/20", "Pending" } }
                                                                }
                                                                
                                                                button {
                                                                    class: "p-1 rounded hover:bg-muted text-muted-foreground hover:text-foreground transition-all",
                                                                    onclick: move |_| {
                                                                        editing_vaccine.set(Some(v.to_string()));
                                                                        vaccine_status.set(status.clone());
                                                                        vaccine_date.set(String::new());
                                                                    },
                                                                    components::LucideIcon { name: "pen-tool", size: "12" }
                                                                }
                                                            }
                                                        }
                                                        
                                                        if is_editing {
                                                            div { class: "mt-1 pt-2 border-t border-border/30 flex flex-col gap-2",
                                                                div { class: "grid gap-2 grid-cols-2",
                                                                    select {
                                                                        class: "yntra-input py-1 text-[10px] bg-sidebar w-full",
                                                                        value: "{vaccine_status}",
                                                                        onchange: move |e| vaccine_status.set(e.value()),
                                                                        option { value: "completed", "Completed" }
                                                                        option { value: "pending", "Pending" }
                                                                        option { value: "exempted", "Exempt" }
                                                                    }
                                                                    input {
                                                                        r#type: "date",
                                                                        class: "yntra-input py-1 text-[10px] w-full",
                                                                        value: "{vaccine_date}",
                                                                        oninput: move |e| vaccine_date.set(e.value()),
                                                                    }
                                                                }
                                                                div { class: "flex justify-end gap-1.5",
                                                                    button {
                                                                        class: "px-2 py-1 text-[9px] font-bold rounded bg-muted hover:bg-muted/80 text-foreground transition-all",
                                                                        onclick: move |_| editing_vaccine.set(None),
                                                                        "Cancel"
                                                                    }
                                                                    button {
                                                                        class: "px-2 py-1 text-[9px] font-bold rounded bg-primary text-primary-foreground hover:bg-primary/80 transition-all",
                                                                        onclick: {
                                                                            let ws = workspace_id.clone();
                                                                            let s_id = student.id.clone();
                                                                            let v_name = v.to_string();
                                                                            let req_id = requester_user_id.clone();
                                                                            move |_| {
                                                                                let ws_c = ws.clone();
                                                                                let s_id_c = s_id.clone();
                                                                                let v_name_c = v_name.clone();
                                                                                let r_id = req_id.clone();
                                                                                let status_c = vaccine_status.read().clone();
                                                                                let date_c = Some(vaccine_date.read().clone()).filter(|s| !s.is_empty());
                                                                                spawn(async move {
                                                                                    if yntra_core::save_health_record(r_id, None, ws_c, s_id_c, v_name_c, status_c, date_c).await.is_ok() {
                                                                                        editing_vaccine.set(None);
                                                                                        let current = *db_trigger.read();
                                                                                        db_trigger.set(current + 1);
                                                                                    }
                                                                                });
                                                                            }
                                                                        },
                                                                        "Save"
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

                                // Section B: Nurse Logs & Visit entry
                                div { class: "flex flex-col gap-6",
                                    // 1. Visit Log Form
                                    components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-3",
                                        h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                            components::LucideIcon { name: "plus", size: "16", class: "text-primary" }
                                            {locales::t("school-health-log-visit", &locale)}
                                        }
                                        
                                        // Form Reason
                                        div { class: "flex flex-col gap-1",
                                            label { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "Reason for Visit" }
                                            input {
                                                r#type: "text",
                                                placeholder: "e.g. Fever, Headache, Scraped elbow",
                                                class: "yntra-input text-xs w-full",
                                                value: "{visit_reason}",
                                                oninput: move |e| visit_reason.set(e.value()),
                                            }
                                        }

                                        // Form Treatment
                                        div { class: "flex flex-col gap-1",
                                            label { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "Treatment Administered" }
                                            input {
                                                r#type: "text",
                                                placeholder: "e.g. Ice pack, Bandage, Sent home",
                                                class: "yntra-input text-xs w-full",
                                                value: "{treatment}",
                                                oninput: move |e| treatment.set(e.value()),
                                            }
                                        }

                                        // Form Notes
                                        div { class: "flex flex-col gap-1",
                                            label { class: "text-[9px] font-black uppercase text-muted-foreground tracking-wider", "Nurse Notes" }
                                            textarea {
                                                placeholder: "Additional check-in notes...",
                                                class: "yntra-input text-xs w-full h-16 resize-none",
                                                value: "{notes}",
                                                oninput: move |e| notes.set(e.value()),
                                            }
                                        }

                                        button {
                                            class: "yntra-btn text-xs font-bold py-2 flex items-center justify-center gap-1.5 shadow-sm mt-1",
                                            onclick: {
                                                let ws = workspace_id.clone();
                                                let s_id = student.id.clone();
                                                move |_| {
                                                    let ws_c = ws.clone();
                                                    let s_c = s_id.clone();
                                                    let reason = visit_reason.read().clone();
                                                    let treat = treatment.read().clone();
                                                    let note_text = Some(notes.read().clone()).filter(|s| !s.is_empty());
                                                    let now_str = std::time::SystemTime::now()
                                                        .duration_since(std::time::UNIX_EPOCH)
                                                        .map(|d| {
                                                            let secs = d.as_secs() as i64;
                                                            let mut days = secs / 86400;
                                                            let mut year = 1970;
                                                            loop {
                                                                let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
                                                                let days_in_year = if leap { 366 } else { 365 };
                                                                if days >= days_in_year {
                                                                    days -= days_in_year;
                                                                    year += 1;
                                                                } else {
                                                                    break;
                                                                }
                                                            }
                                                            let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
                                                            let month_lengths = if leap {
                                                                [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
                                                            } else {
                                                                [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
                                                            };
                                                            let mut month = 1;
                                                            for &length in month_lengths.iter() {
                                                                if days >= length {
                                                                    days -= length;
                                                                    month += 1;
                                                                } else {
                                                                    break;
                                                                }
                                                            }
                                                            let day = days + 1;
                                                            let tod_secs = secs % 86400;
                                                            let hour = tod_secs / 3600;
                                                            let min = (tod_secs % 3600) / 60;
                                                            format!("{:04}-{:02}-{:02} {:02}:{:02}", year, month, day, hour, min)
                                                        })
                                                        .unwrap_or_else(|_| "2026-07-04 12:00".to_string());
                                                        
                                                    let req_id = requester_user_id.clone();
                                                    spawn(async move {
                                                        if yntra_core::save_health_incident(req_id, None, ws_c, s_c, reason, treat, now_str, None, note_text).await.is_ok() {
                                                            visit_reason.set(String::new());
                                                            treatment.set(String::new());
                                                            notes.set(String::new());
                                                            let current = *db_trigger.read();
                                                            db_trigger.set(current + 1);
                                                        }
                                                    });
                                                }
                                            },
                                            components::LucideIcon { name: "plus", size: "14" }
                                            "{locales::t(\"school-health-log-btn\", &locale)}"
                                        }
                                    }

                                    // 2. Incident List / Timeline
                                    components::Card { class: "p-5 border-border/40 bg-sidebar/20 flex flex-col gap-4 flex-1",
                                        h3 { class: "text-sm font-black uppercase text-muted-foreground m-0 flex items-center gap-2",
                                            components::LucideIcon { name: "time", size: "16", class: "text-primary" }
                                            {locales::t("school-health-visit-history", &locale)}
                                        }

                                        if incidents.is_empty() {
                                            p { class: "text-xs text-muted-foreground italic text-center p-4 m-0", "No visits logged yet." }
                                        } else {
                                            div { class: "flex flex-col gap-3",
                                                for incident in incidents.iter() {
                                                    div { key: "{incident.id}", class: "border-l-4 border-primary pl-3 py-0.5 flex flex-col gap-0.5",
                                                        div { class: "text-xs font-black text-foreground flex justify-between items-center",
                                                            span { "{incident.visit_reason}" }
                                                            span { class: "text-[10px] text-muted-foreground font-normal", "{incident.checked_in_at}" }
                                                        }
                                                        span { class: "text-[10px] text-muted-foreground", "Treatment: {incident.treatment}" }
                                                        if let Some(ref note) = incident.notes {
                                                            p { class: "text-[10px] text-muted-foreground italic m-0 mt-0.5", "\"{note}\"" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        p { class: "text-xs text-muted-foreground italic text-center p-8 bg-sidebar/10 rounded-xl", "Please select a student to manage health records." }
                    }
                }
            }
        }
    }
}
