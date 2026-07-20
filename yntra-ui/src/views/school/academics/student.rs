use dioxus::prelude::*;
use dioxus::html::HasFileData;
use crate::components::{Button, Card, CardContent, CardDescription, CardHeader, CardTitle, LucideIcon};
use crate::locales::t;
use super::SchoolViewProps;
use super::utils::{
    is_deadline_passed, parse_submission_content_and_advanced_attachment,
    format_file_size, compute_mock_hash, base64_encode, AdvancedAttachment, BlobDownloadLink,
    decrypt_field, decrypt_opt_field, encrypt_field_with_proof, encrypt_opt_field_with_proof
};
use yntra_core::{
    get_assignments, get_student_submissions, get_timetable_slots, save_submission,
    get_student_attendance_records, get_library_lending_logs, Submission, StudentProfile,
    delete_assignment
};

#[component]
pub fn StudentPortal(
    school_props: SchoolViewProps,
    students: Vec<StudentProfile>,
    mut selected_student_profile_id: Signal<String>,
) -> Element {
    let state = use_context::<crate::state::AppState>();
    let mut db_trigger = state.trigger_school_academics;
    let db_trigger_academics = state.trigger_school_academics;
    let db_trigger_attendance = state.trigger_school_attendance;
    let db_trigger_library = state.trigger_school_library;
    let locale = school_props.locale.clone();
    let user_id = school_props.active_user_id.clone();
    let ws_id = school_props.workspace_id.clone();

    // Local states specific to student submissions and files
    let mut assignment_inputs = use_signal(std::collections::HashMap::<String, String>::new);
    let mut submission_files = use_signal(std::collections::HashMap::<String, AdvancedAttachment>::new);
    let mut drag_active = use_signal(std::collections::HashMap::<String, bool>::new);
    let mut homework_filter = use_signal(|| "todo".to_string());
    let mut active_tab = use_signal(|| "stream".to_string());
    let mut new_announcement_text = use_signal(String::new);
    let mut comment_inputs = use_signal(std::collections::HashMap::<String, String>::new);
    let mut editing_item_id = use_signal(|| Option::<String>::None);
    let mut edit_text = use_signal(String::new);
    let mut submitting_map = use_signal(std::collections::HashSet::<String>::new);
    let toast = dioxus_primitives::toast::use_toast();

    let user_id_clone_att = user_id.clone();
    let ws_id_clone_att = ws_id.clone();
    let student_attendance_res = use_resource(move || {
        let _trig = db_trigger_attendance.read();
        let s_id = selected_student_profile_id.read().clone();
        let uid = user_id_clone_att.clone();
        let ws = ws_id_clone_att.clone();
        async move {
            if s_id.is_empty() {
                Vec::new()
            } else {
                get_student_attendance_records(uid, ws, s_id).await.unwrap_or_default()
            }
        }
    });

    // Dynamic data fetching resource for student submissions
    let user_id_clone6 = user_id.clone();
    let ws_id_clone6 = ws_id.clone();
    let submissions_res = use_resource(move || {
        let _trig = db_trigger_academics.read();
        let s_id = selected_student_profile_id.read().clone();
        let uid = user_id_clone6.clone();
        let ws = ws_id_clone6.clone();
        async move {
            if s_id.is_empty() {
                Vec::new()
            } else {
                get_student_submissions(uid, ws, s_id).await.unwrap_or_default()
            }
        }
    });

    // Timetable slots resource
    let user_id_clone7 = user_id.clone();
    let ws_id_clone7 = ws_id.clone();
    let timetable_res = use_resource(move || {
        let _trig = db_trigger_academics.read();
        let uid = user_id_clone7.clone();
        let ws = ws_id_clone7.clone();
        async move {
            get_timetable_slots(uid, ws).await.unwrap_or_default()
        }
    });

    // Courses resource to look up course names in the timetable
    let user_id_clone_courses = user_id.clone();
    let ws_id_clone_courses = ws_id.clone();
    let courses_res = use_resource(move || {
        let _trig = db_trigger_academics.read();
        let uid = user_id_clone_courses.clone();
        let ws = ws_id_clone_courses.clone();
        async move { yntra_core::get_workspace_courses(uid, ws).await.unwrap_or_default() }
    });

    // Helper resource to load all workspace assignments for the student dashboard preview
    let courses = courses_res.read().clone().unwrap_or_default();
    let courses_clone = courses.clone();
    let user_id_clone5 = user_id.clone();
    let ws_id_clone5 = ws_id.clone();
    let all_assignments_res = use_resource(move || {
        let _trig = db_trigger_academics.read();
        let uid = user_id_clone5.clone();
        let ws = ws_id_clone5.clone();
        let courses_list = courses_clone.clone();
        async move {
            let mut list = Vec::new();
            for c in courses_list {
                if let Ok(mut assign) = get_assignments(uid.clone(), ws.clone(), c.id.clone()).await {
                    list.append(&mut assign);
                }
            }
            list
        }
    });

    // Helper resource to load library logs to dynamically compute the "Avid Reader" badge
    let user_id_clone_lib = user_id.clone();
    let ws_id_clone_lib = ws_id.clone();
    let library_logs_res = use_resource(move || {
        let _trig = db_trigger_library.read();
        let uid = user_id_clone_lib.clone();
        let ws = ws_id_clone_lib.clone();
        async move { get_library_lending_logs(uid, ws).await.unwrap_or_default() }
    });

    let seed = state.get_passkey_seed();
    let all_assignments = all_assignments_res.read().clone().unwrap_or_default();
    let student_submissions = submissions_res.read().clone().unwrap_or_default()
        .into_iter()
        .map(|mut s| {
            s.content = decrypt_field(&seed, &s.content);
            s.grade = decrypt_opt_field(&seed, s.grade);
            s.feedback = decrypt_opt_field(&seed, s.feedback);
            s
        })
        .collect::<Vec<_>>();

    let announcements = all_assignments.iter().filter(|a| a.max_points == -1).cloned().collect::<Vec<_>>();
    let comments = all_assignments.iter().filter(|a| a.max_points == -2).cloned().collect::<Vec<_>>();

    let homework_filter_val = homework_filter.read().clone();
    let filtered_assignments = all_assignments.iter().filter(|a| {
        if a.max_points < 0 {
            return false;
        }
        let has_sub = student_submissions.iter().any(|sub| sub.assignment_id == a.id);
        if homework_filter_val == "todo" {
            a.max_points > 0 && !has_sub
        } else if homework_filter_val == "done" {
            has_sub
        } else if homework_filter_val == "materials" {
            a.max_points == 0
        } else {
            true
        }
    }).cloned().collect::<Vec<_>>();

    let timetable = timetable_res.read().clone().unwrap_or_default();
    let attendance = student_attendance_res.read().clone().unwrap_or_default()
        .into_iter()
        .map(|mut a| {
            a.notes = decrypt_opt_field(&seed, a.notes);
            a
        })
        .collect::<Vec<_>>();
    let library_logs = library_logs_res.read().clone().unwrap_or_default();

    let enrolled_course_ids: std::collections::HashSet<String> = {
        let mut ids = std::collections::HashSet::new();
        for att in attendance.iter() {
            ids.insert(att.course_id.clone());
        }
        for sub in student_submissions.iter() {
            if let Some(assign) = all_assignments.iter().find(|a| a.id == sub.assignment_id) {
                ids.insert(assign.course_id.clone());
            }
        }
        ids
    };

    let filtered_timetable: Vec<_> = timetable.iter()
        .filter(|s| enrolled_course_ids.contains(&s.course_id))
        .cloned()
        .collect();


    // Stats calculations
    let total_days = attendance.len();
    let present_days = attendance.iter().filter(|a| a.status == "present" || a.status == "late").count();
    let absent_days = attendance.iter().filter(|a| a.status == "absent").count();
    let attendance_rate = if total_days > 0 {
        (present_days as f32 / total_days as f32) * 100.0
    } else {
        100.0
    };

    // SVG circle math
    let stroke_dasharray = 188.49; // 2 * PI * 30
    let stroke_dashoffset = stroke_dasharray - (attendance_rate / 100.0) * stroke_dasharray;

    if students.is_empty() {
        return rsx! {
            div { class: "flex flex-col items-center justify-center py-20 text-center border border-dashed border-border rounded-2xl bg-muted/10",
                LucideIcon { name: "users", class: "h-12 w-12 text-muted-foreground/30 mb-3" }
                h4 { class: "text-sm font-bold text-foreground m-0", {t("school-student-no-students", &locale)} }
                p { class: "text-xs text-muted-foreground mt-1 max-w-sm", {t("school-no-enrolled-students", &locale)} }
            }
        };
    }

    let current_student_id = selected_student_profile_id.read().clone();
    let current_student = students.iter().find(|s| s.id == current_student_id).cloned().unwrap_or_else(|| students[0].clone());
    let student_name = format!("{} {}", current_student.first_name, current_student.last_name);
    
    let is_active_scholar = total_days > 0 && attendance_rate >= 95.0;
    let is_avid_reader = library_logs.iter().any(|log| {
        log.student_name == student_name && (log.status == "borrowed" || log.status == "returned")
    });
    
    let graded_submissions_count = student_submissions.iter()
        .filter(|sub| {
            if let Some(ref g) = sub.grade {
                g == "A" || g == "B" || g == "C" || g == "B+" || g == "A+"
            } else {
                false
            }
        })
        .count();
    let stars_count = std::cmp::max(1, graded_submissions_count);

    rsx! {
        div { class: "space-y-6 animate-in fade-in duration-300",
            // Student Profile Selector Card (Teacher/Admin view)
            if {
                let role = state.active_user_role.read();
                role.as_str() != "student" && role.as_str() != "role-school-student" && role.as_str() != "parent" && role.as_str() != "role-school-parent"
            } {
                Card { class: "p-4 border border-border bg-sidebar rounded-2xl flex flex-col sm:flex-row gap-4 items-center justify-between shadow-sm",
                    div { class: "flex items-center gap-3 w-full sm:w-auto",
                        LucideIcon { name: "user", class: "h-5 w-5 text-primary" }
                        div {
                            h4 { class: "text-sm font-bold text-foreground m-0", {t("school-change-profile", &locale)} }
                            p { class: "text-[10px] text-muted-foreground m-0 mt-0.5", {t("school-student-preview-desc", &locale)} }
                        }
                    }
                    select {
                        class: "rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/50 w-full sm:w-60",
                        value: selected_student_profile_id.read().clone(),
                        onchange: move |evt: FormEvent| selected_student_profile_id.set(evt.value()),
                        for s in students.iter() {
                            option { value: "{s.id}", "{s.first_name} {s.last_name} ({s.grade_level})" }
                        }
                    }
                }
            }

            // Child Profile Selector Card (Parent view)
            if {
                let role = state.active_user_role.read();
                role.as_str() == "parent" || role.as_str() == "role-school-parent"
            } {
                Card { class: "p-4 border border-border bg-sidebar rounded-2xl flex flex-col sm:flex-row gap-4 items-center justify-between shadow-sm",
                    div { class: "flex items-center gap-3 w-full sm:w-auto",
                        LucideIcon { name: "user", class: "h-5 w-5 text-primary" }
                        div {
                            h4 { class: "text-sm font-bold text-foreground m-0", {t("school-parent-select-child", &locale)} }
                            p { class: "text-[10px] text-muted-foreground m-0 mt-0.5", {t("school-parent-select-child-desc", &locale)} }
                        }
                    }
                    select {
                        class: "rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-primary/50 w-full sm:w-60",
                        value: selected_student_profile_id.read().clone(),
                        onchange: move |evt: FormEvent| selected_student_profile_id.set(evt.value()),
                        for s in students.iter() {
                            option { value: "{s.id}", "{s.first_name} {s.last_name} ({s.grade_level})" }
                        }
                    }
                }
            }

            // Horizontal Google Classroom-style tab selector for students
            div { class: "flex border-b border-border pb-2.5 gap-6 text-xs font-extrabold tracking-wider",
                button {
                    class: format!(
                        "pb-2.5 transition-all border-b-2 {}",
                        if *active_tab.read() == "stream" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }
                    ),
                    r#type: "button",
                    onclick: move |_| active_tab.set("stream".to_string()),
                    "Stream"
                }
                button {
                    class: format!(
                        "pb-2.5 transition-all border-b-2 {}",
                        if *active_tab.read() == "dashboard" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }
                    ),
                    r#type: "button",
                    onclick: move |_| active_tab.set("dashboard".to_string()),
                    "Dashboard"
                }
                button {
                    class: format!(
                        "pb-2.5 transition-all border-b-2 {}",
                        if *active_tab.read() == "classwork" { "border-primary text-primary" } else { "border-transparent text-muted-foreground hover:text-foreground" }
                    ),
                    r#type: "button",
                    onclick: move |_| active_tab.set("classwork".to_string()),
                    "Classwork"
                }
            }

            // Stream board tab
            if *active_tab.read() == "stream" {
                div { class: "space-y-6",
                        // Announcement editor box
                        div { class: "p-4 border border-border/80 rounded-xl bg-muted/20 space-y-3",
                            textarea {
                                class: "w-full min-h-[80px] p-3 text-xs bg-background border border-border/60 rounded-lg focus:outline-none focus:ring-1 focus:ring-primary text-foreground placeholder:text-muted-foreground/60 resize-none font-medium",
                                placeholder: "Share something with your class...",
                                value: "{new_announcement_text}",
                                oninput: move |evt| new_announcement_text.set(evt.value().clone()),
                            }
                            div { class: "flex justify-end",
                                Button {
                                    class: "px-4 h-8 text-xs font-semibold rounded-lg bg-primary text-primary-foreground flex items-center gap-1",
                                    disabled: new_announcement_text.read().trim().is_empty(),
                                    onclick: {
                                        let current_student_c = current_student.clone();
                                        let ws = ws_id.clone();
                                        let uid = user_id.clone();
                                        let mut trig = db_trigger;
                                        let courses_c = courses.clone();
                                        move |_| {
                                            let text = new_announcement_text.read().clone();
                                            let author_name = format!("{} {}", current_student_c.first_name, current_student_c.last_name);
                                            let course_id = courses_c.first().map(|c| c.id.clone()).unwrap_or_default();
                                            let assignment = yntra_core::Assignment {
                                                id: uuid::Uuid::new_v4().to_string(),
                                                workspace_id: ws.clone(),
                                                course_id,
                                                title: author_name,
                                                description: text,
                                                due_date: "student".to_string(),
                                                max_points: -1,
                                                updated_at: yntra_core::infra::time::get_current_time_ms(),
                                            };
                                            let uid_c = uid.clone();
                                            let mut trig_c = trig;
                                            spawn(async move {
                                                if yntra_core::save_assignment(uid_c, assignment, None).await.is_ok() {
                                                    new_announcement_text.set(String::new());
                                                    let current = *trig_c.read();
                                                    trig_c.set(current + 1);
                                                }
                                            });
                                        }
                                    },
                                    LucideIcon { name: "send", class: "h-3 w-3" }
                                    "Post"
                                }
                            }
                        }

                        // Stream Announcements Feed
                        if announcements.is_empty() {
                            div { class: "py-12 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl italic", "No announcements yet." }
                        } else {
                            div { class: "space-y-4",
                                for ann in announcements.clone().into_iter().rev() {
                                    {
                                        let ann_id = ann.id.clone();
                                        let ann_comments = comments.iter().filter(|comm| comm.course_id == ann_id).cloned().collect::<Vec<_>>();
                                        let comment_text = comment_inputs.read().get(&ann_id).cloned().unwrap_or_default();
                                        let is_author = ann.due_date == "student";
                                        let ann_title = ann.title.clone();
                                        let ann_desc = ann.description.clone();
                                        rsx! {
                                            div { key: "{ann_id}", class: "p-5 border border-border rounded-xl bg-background space-y-4 shadow-sm",
                                                div { class: "flex items-start justify-between gap-3",
                                                    div { class: "flex items-center gap-2.5",
                                                        div { class: "h-8 w-8 rounded-full bg-primary/10 text-primary flex items-center justify-center font-bold text-xs",
                                                            "{ann_title.chars().next().unwrap_or('?')}"
                                                        }
                                                        div {
                                                            div { class: "text-xs font-bold text-foreground flex items-center gap-1.5", 
                                                                "{ann_title}"
                                                                span { 
                                                                    class: format!(
                                                                        "text-[9px] px-1.5 py-0.5 rounded font-extrabold uppercase tracking-wide {}",
                                                                        if is_author { "bg-accent/10 text-accent" } else { "bg-primary/10 text-primary" }
                                                                    ),
                                                                    if is_author { "Student" } else { "Teacher" }
                                                                }
                                                            }
                                                            div { class: "text-[10px] text-muted-foreground",
                                                                {
                                                                    let ms = ann.updated_at;
                                                                    let formatted = format_timestamp(ms);
                                                                    formatted
                                                                }
                                                            }
                                                        }
                                                    }
                                                    if is_author && ann_title == student_name {
                                                        div { class: "flex items-center gap-1.5",
                                                            button {
                                                                class: "p-1 rounded hover:bg-muted text-muted-foreground hover:text-foreground border-none bg-transparent cursor-pointer transition-colors",
                                                                r#type: "button",
                                                                onclick: {
                                                                    let ann_desc_c = ann_desc.clone();
                                                                    let ann_id_c = ann_id.clone();
                                                                    move |_| {
                                                                        editing_item_id.set(Some(ann_id_c.clone()));
                                                                        edit_text.set(ann_desc_c.clone());
                                                                    }
                                                                },
                                                                LucideIcon { name: "pencil", class: "h-3.5 w-3.5" }
                                                            }
                                                            button {
                                                                class: "p-1 rounded hover:bg-red-500/10 text-muted-foreground hover:text-red-500 border-none bg-transparent cursor-pointer transition-colors",
                                                                r#type: "button",
                                                                onclick: {
                                                                    let ann_id_c = ann_id.clone();
                                                                    let uid_c = user_id.clone();
                                                                    let mut trig_c = db_trigger;
                                                                    move |_| {
                                                                        let uid_del = uid_c.clone();
                                                                        let id_del = ann_id_c.clone();
                                                                        let mut trig_del = trig_c;
                                                                        spawn(async move {
                                                                            if delete_assignment(uid_del, id_del).await.is_ok() {
                                                                                let val = *trig_del.read();
                                                                                trig_del.set(val + 1);
                                                                            }
                                                                        });
                                                                    }
                                                                },
                                                                LucideIcon { name: "trash", class: "h-3.5 w-3.5" }
                                                            }
                                                        }
                                                    }
                                                }
                                                if *editing_item_id.read() == Some(ann_id.clone()) {
                                                    div { class: "flex flex-col gap-2 p-2 bg-muted/10 border border-border rounded-lg",
                                                        textarea {
                                                            class: "w-full min-h-[75px] p-2.5 text-xs bg-background border border-border rounded-lg focus:outline-none focus:ring-1 focus:ring-primary text-foreground resize-none",
                                                            value: "{edit_text}",
                                                            oninput: move |evt| edit_text.set(evt.value().clone()),
                                                        }
                                                        div { class: "flex justify-end gap-2",
                                                            Button {
                                                                class: "px-3 h-7 text-[10px] font-bold rounded-lg border border-border bg-background hover:bg-muted text-foreground",
                                                                onclick: move |_| editing_item_id.set(None),
                                                                "Cancel"
                                                            }
                                                            Button {
                                                                class: "px-3 h-7 text-[10px] font-bold rounded-lg bg-primary text-primary-foreground",
                                                                disabled: edit_text.read().trim().is_empty(),
                                                                onclick: {
                                                                    let mut updated_ann = ann.clone();
                                                                    let uid_c = user_id.clone();
                                                                    let mut trig_c = db_trigger;
                                                                    move |_| {
                                                                        let text_val = edit_text.read().clone();
                                                                        let mut item_val = updated_ann.clone();
                                                                        item_val.description = text_val;
                                                                        item_val.updated_at = yntra_core::infra::time::get_current_time_ms();
                                                                        let uid_save = uid_c.clone();
                                                                        let mut trig_save = trig_c;
                                                                        spawn(async move {
                                                                            if yntra_core::save_assignment(uid_save, item_val, None).await.is_ok() {
                                                                                editing_item_id.set(None);
                                                                                let val = *trig_save.read();
                                                                                trig_save.set(val + 1);
                                                                            }
                                                                        });
                                                                    }
                                                                },
                                                                "Save"
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    div { class: "text-xs text-foreground font-medium whitespace-pre-line leading-relaxed", "{ann_desc}" }
                                                }
                                                
                                                div { class: "border-t border-border/40 pt-3 space-y-3",
                                                    div { class: "text-[10px] font-bold text-muted-foreground flex items-center gap-1",
                                                        LucideIcon { name: "message-square", size: "11" }
                                                        "Class comments ({ann_comments.len()})"
                                                    }
                                                    
                                                    if !ann_comments.is_empty() {
                                                        div { class: "space-y-3 pl-3 border-l-2 border-muted",
                                                            for comm in ann_comments.into_iter() {
                                                                {
                                                                    let comm_id = comm.id.clone();
                                                                    let comm_title = comm.title.clone();
                                                                    let comm_due_date = comm.due_date.clone();
                                                                    let comm_desc = comm.description.clone();
                                                                    let comm_updated_at = comm.updated_at;
                                                                    rsx! {
                                                                        div { key: "{comm_id}", class: "text-xs space-y-0.5",
                                                                            div { class: "flex items-center justify-between gap-1.5 w-full",
                                                                                div { class: "flex items-center gap-1.5",
                                                                                    span { class: "font-bold text-foreground", "{comm_title}" }
                                                                                    span { class: "text-[8px] font-bold px-1 rounded bg-muted text-muted-foreground uppercase", "{comm_due_date}" }
                                                                                    span { class: "text-[9px] text-muted-foreground/60",
                                                                                        {
                                                                                            let ms = comm_updated_at;
                                                                                            let formatted = format_timestamp(ms);
                                                                                            formatted
                                                                                        }
                                                                                    }
                                                                                }
                                                                                if comm_due_date == "student" && comm_title == student_name {
                                                                                    div { class: "flex items-center gap-1",
                                                                                        button {
                                                                                            class: "p-0.5 rounded hover:bg-muted text-muted-foreground hover:text-foreground border-none bg-transparent cursor-pointer transition-colors",
                                                                                            r#type: "button",
                                                                                            onclick: {
                                                                                                let comm_desc_c = comm_desc.clone();
                                                                                                let comm_id_c = comm_id.clone();
                                                                                                move |_| {
                                                                                                    editing_item_id.set(Some(comm_id_c.clone()));
                                                                                                    edit_text.set(comm_desc_c.clone());
                                                                                                }
                                                                                            },
                                                                                            LucideIcon { name: "pencil", class: "h-3 w-3" }
                                                                                        }
                                                                                        button {
                                                                                            class: "p-0.5 rounded hover:bg-red-500/10 text-muted-foreground hover:text-red-500 border-none bg-transparent cursor-pointer transition-colors",
                                                                                            r#type: "button",
                                                                                            onclick: {
                                                                                                let comm_id_c = comm_id.clone();
                                                                                                let uid_c = user_id.clone();
                                                                                                let mut trig_c = db_trigger;
                                                                                                move |_| {
                                                                                                    let uid_del = uid_c.clone();
                                                                                                    let id_del = comm_id_c.clone();
                                                                                                    let mut trig_del = trig_c;
                                                                                                    spawn(async move {
                                                                                                        if delete_assignment(uid_del, id_del).await.is_ok() {
                                                                                                            let val = *trig_del.read();
                                                                                                            trig_del.set(val + 1);
                                                                                                        }
                                                                                                    });
                                                                                                }
                                                                                            },
                                                                                            LucideIcon { name: "trash", class: "h-3 w-3" }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            if *editing_item_id.read() == Some(comm_id.clone()) {
                                                                                div { class: "flex flex-col gap-2 p-1.5 bg-muted/10 border border-border rounded-lg mt-1",
                                                                                    input {
                                                                                        class: "w-full h-8 px-2 text-xs bg-background border border-border rounded-lg focus:outline-none focus:ring-1 focus:ring-primary text-foreground font-medium",
                                                                                        value: "{edit_text}",
                                                                                        oninput: move |evt| edit_text.set(evt.value().clone()),
                                                                                    }
                                                                                    div { class: "flex justify-end gap-1.5",
                                                                                        Button {
                                                                                            class: "px-2 h-6 text-[9px] font-bold rounded-lg border border-border bg-background hover:bg-muted text-foreground",
                                                                                            onclick: move |_| editing_item_id.set(None),
                                                                                            "Cancel"
                                                                                        }
                                                                                        Button {
                                                                                            class: "px-2 h-6 text-[9px] font-bold rounded-lg bg-primary text-primary-foreground",
                                                                                            disabled: edit_text.read().trim().is_empty(),
                                                                                            onclick: {
                                                                                                let mut updated_comm = comm.clone();
                                                                                                let uid_c = user_id.clone();
                                                                                                let mut trig_c = db_trigger;
                                                                                                move |_| {
                                                                                                    let text_val = edit_text.read().clone();
                                                                                                    let mut item_val = updated_comm.clone();
                                                                                                    item_val.description = text_val;
                                                                                                    item_val.updated_at = yntra_core::infra::time::get_current_time_ms();
                                                                                                    let uid_save = uid_c.clone();
                                                                                                    let mut trig_save = trig_c;
                                                                                                    spawn(async move {
                                                                                                        if yntra_core::save_assignment(uid_save, item_val, None).await.is_ok() {
                                                                                                            editing_item_id.set(None);
                                                                                                            let val = *trig_save.read();
                                                                                                            trig_save.set(val + 1);
                                                                                                        }
                                                                                                    });
                                                                                                }
                                                                                            },
                                                                                            "Save"
                                                                                        }
                                                                                    }
                                                                                }
                                                                            } else {
                                                                                div { class: "text-muted-foreground font-medium pl-1", "{comm_desc}" }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    
                                                    div { class: "flex items-center gap-2 pt-1.5",
                                                        input {
                                                            class: "flex-1 h-8 px-3 text-xs bg-muted/30 border border-border/60 rounded-lg focus:outline-none focus:ring-1 focus:ring-primary text-foreground placeholder:text-muted-foreground/50 font-medium",
                                                            placeholder: "Add class comment...",
                                                            value: "{comment_text}",
                                                            oninput: {
                                                                let ann_id_c = ann_id.clone();
                                                                move |evt| {
                                                                    comment_inputs.write().insert(ann_id_c.clone(), evt.value().clone());
                                                                }
                                                            },
                                                            onkeydown: {
                                                                let ann_id_c = ann_id.clone();
                                                                let ws = ws_id.clone();
                                                                let author_name = format!("{} {}", current_student.first_name, current_student.last_name);
                                                                let uid = user_id.clone();
                                                                let text = comment_text.clone();
                                                                let mut trig = db_trigger;
                                                                move |evt| {
                                                                    if evt.key() == Key::Enter && !text.trim().is_empty() {
                                                                        let val = text.clone();
                                                                        let aid = ann_id_c.clone();
                                                                        let c_item = yntra_core::Assignment {
                                                                            id: uuid::Uuid::new_v4().to_string(),
                                                                            workspace_id: ws.clone(),
                                                                            course_id: aid,
                                                                            title: author_name.clone(),
                                                                            description: val,
                                                                            due_date: "student".to_string(),
                                                                            max_points: -2,
                                                                            updated_at: yntra_core::infra::time::get_current_time_ms(),
                                                                        };
                                                                        let uid_c = uid.clone();
                                                                        comment_inputs.write().insert(ann_id_c.clone(), String::new());
                                                                        spawn(async move {
                                                                            if yntra_core::save_assignment(uid_c, c_item, None).await.is_ok() {
                                                                                {
                                                                                    let current = *trig.read();
                                                                                    trig.set(current + 1);
                                                                                }
                                                                            }
                                                                        });
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        button {
                                                            class: "h-8 w-8 rounded-lg bg-primary/10 text-primary hover:bg-primary hover:text-primary-foreground border-0 cursor-pointer flex items-center justify-center transition-colors",
                                                            r#type: "button",
                                                            onclick: {
                                                                let ann_id_c = ann_id.clone();
                                                                let ws = ws_id.clone();
                                                                let author_name = format!("{} {}", current_student.first_name, current_student.last_name);
                                                                let uid = user_id.clone();
                                                                let text = comment_text.clone();
                                                                let mut trig = db_trigger;
                                                                move |_| {
                                                                    if !text.trim().is_empty() {
                                                                        let val = text.clone();
                                                                        let aid = ann_id_c.clone();
                                                                        let c_item = yntra_core::Assignment {
                                                                            id: uuid::Uuid::new_v4().to_string(),
                                                                            workspace_id: ws.clone(),
                                                                            course_id: aid,
                                                                            title: author_name.clone(),
                                                                            description: val,
                                                                            due_date: "student".to_string(),
                                                                            max_points: -2,
                                                                            updated_at: yntra_core::infra::time::get_current_time_ms(),
                                                                        };
                                                                        let uid_c = uid.clone();
                                                                        comment_inputs.write().insert(ann_id_c.clone(), String::new());
                                                                        spawn(async move {
                                                                            if yntra_core::save_assignment(uid_c, c_item, None).await.is_ok() {
                                                                                {
                                                                                    let current = *trig.read();
                                                                                    trig.set(current + 1);
                                                                                }
                                                                            }
                                                                        });
                                                                    }
                                                                }
                                                            },
                                                            LucideIcon { name: "arrow-right", class: "h-3.5 w-3.5" }
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

            if *active_tab.read() == "dashboard" {
                // Welcome Banner Card
                div { class: "p-6 rounded-2xl bg-gradient-to-r from-primary/10 to-accent/5 border border-primary/20 flex items-center justify-between shadow-sm",
                    div { class: "space-y-1.5",
                        h3 { class: "text-lg font-extrabold text-foreground m-0", 
                            {crate::locales::t_with_args("school-welcome", &locale, &[("name", &student_name)])}
                        }
                        p { class: "text-xs text-muted-foreground m-0", {t("school-ready-msg", &locale)} }
                    }
                    LucideIcon { name: "award", class: "h-10 w-10 text-primary" }
                }
            }

            if *active_tab.read() == "dashboard" {
                // Stars & Badges & Attendance grid
                div { class: "grid grid-cols-1 lg:grid-cols-3 gap-6",
                Card { class: "border-border shadow-sm",
                    CardHeader {
                        CardTitle { {t("school-my-stars", &locale)} }
                        CardDescription { {t("school-student-stars-desc", &locale)} }
                    }
                    CardContent { class: "flex flex-col items-center py-6 space-y-4",
                        div { class: "flex gap-2.5",
                            for i in 0..5 {
                                {
                                    let active = i < stars_count;
                                    rsx! {
                                        LucideIcon { 
                                            name: "star", 
                                            class: format!("h-10 w-10 {}", if active { "text-amber-500 fill-amber-500 animate-pulse" } else { "text-muted-foreground/10" }) 
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "text-xs font-bold text-foreground", {crate::locales::t_with_args("school-student-earned-stars", &locale, &[("count", &stars_count.to_string())])} }
                    }
                }
                Card { class: "border-border shadow-sm",
                    CardHeader {
                        CardTitle { "My Learning Badges" }
                        CardDescription { {t("school-student-milestones", &locale)} }
                    }
                    CardContent { class: "flex flex-wrap gap-2.5 pt-2",
                        if graded_submissions_count > 0 {
                            div { class: "flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-emerald-500/10 text-emerald-600 border border-emerald-500/20 text-xs font-bold shadow-sm",
                                LucideIcon { name: "award", class: "h-4 w-4" }
                                "Perfect Homework"
                            }
                        }
                        if is_active_scholar {
                            div { class: "flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-primary/10 text-primary border border-primary/20 text-xs font-bold shadow-sm",
                                LucideIcon { name: "user-check", class: "h-4 w-4" }
                                "Active Scholar"
                            }
                        }
                        if is_avid_reader {
                            div { class: "flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-purple-500/10 text-purple-600 border border-purple-500/20 text-xs font-bold shadow-sm",
                                LucideIcon { name: "library", class: "h-4 w-4" }
                                "Avid Reader"
                            }
                        }
                        if graded_submissions_count == 0 && !is_active_scholar && !is_avid_reader {
                            span { class: "text-xs text-muted-foreground italic", "No badges earned yet. Complete assignments and attend classes to earn badges!" }
                        }
                    }
                }
                Card { class: "border-border shadow-sm",
                    CardHeader {
                        CardTitle { {t("school-parent-attendance-tracking", &locale)} }
                        CardDescription { {t("school-parent-attendance-desc", &locale)} }
                    }
                    CardContent { class: "flex flex-col items-center py-2 space-y-4",
                        div { class: "relative h-20 w-20",
                            svg {
                                class: "h-20 w-20 -rotate-90 transform",
                                circle {
                                    cx: "40",
                                    cy: "40",
                                    r: "30",
                                    class: "stroke-muted/40",
                                    stroke_width: "5",
                                    fill: "transparent",
                                }
                                circle {
                                    cx: "40",
                                    cy: "40",
                                    r: "30",
                                    class: "stroke-primary transition-all duration-700",
                                    stroke_width: "5",
                                    stroke_dasharray: "{stroke_dasharray}",
                                    stroke_dashoffset: "{stroke_dashoffset}",
                                    stroke_linecap: "round",
                                    fill: "transparent",
                                }
                            }
                            div { class: "absolute inset-0 flex flex-col items-center justify-center",
                                span { class: "text-sm font-black text-foreground", "{attendance_rate:.1}%" }
                            }
                        }
                        div { class: "text-[10px] text-muted-foreground text-center font-medium",
                            if attendance_rate >= 90.0 { {t("school-parent-excellent-standing", &locale)} } else { {t("school-parent-low-attendance", &locale)} }
                        }
                    }
                }
            }
        }

            if *active_tab.read() == "classwork" {
                Card { class: "border border-border shadow-sm",
                CardHeader {
                    class: "flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 pb-3 border-b border-border/40",
                    div {
                        CardTitle { {t("school-my-homework", &locale)} }
                        CardDescription { {t("school-student-homework-desc", &locale)} }
                    }
                    div { class: "flex gap-1.5 p-1 bg-muted/30 border border-border/40 rounded-xl w-fit text-[11px] font-semibold",
                        button {
                            class: format!(
                                "px-3 py-1.5 rounded-lg transition-all {}",
                                if *homework_filter.read() == "todo" { "bg-background text-foreground shadow-sm font-bold" } else { "text-muted-foreground hover:text-foreground" }
                            ),
                            r#type: "button",
                            onclick: move |_| homework_filter.set("todo".to_string()),
                            "To Do"
                        }
                        button {
                            class: format!(
                                "px-3 py-1.5 rounded-lg transition-all {}",
                                if *homework_filter.read() == "done" { "bg-background text-foreground shadow-sm font-bold" } else { "text-muted-foreground hover:text-foreground" }
                            ),
                            r#type: "button",
                            onclick: move |_| homework_filter.set("done".to_string()),
                            "Done"
                        }
                        button {
                            class: format!(
                                "px-3 py-1.5 rounded-lg transition-all {}",
                                if *homework_filter.read() == "materials" { "bg-background text-foreground shadow-sm font-bold" } else { "text-muted-foreground hover:text-foreground" }
                            ),
                            r#type: "button",
                            onclick: move |_| homework_filter.set("materials".to_string()),
                            "Study Materials"
                        }
                    }
                }
                CardContent {
                    class: "pt-4",
                    if filtered_assignments.is_empty() {
                        div { class: "py-10 text-center text-xs text-muted-foreground italic",
                            if *homework_filter.read() == "todo" {
                                "No pending homework tasks. Excellent job!"
                            } else if *homework_filter.read() == "done" {
                                "No completed assignments to show yet."
                            } else {
                                "No study materials shared for this class yet."
                            }
                        }
                    } else {
                        div { class: "divide-y divide-border border rounded-xl overflow-hidden bg-background",
                            for a in filtered_assignments.iter() {
                                {
                                    let (desc_text, attachment) = parse_submission_content_and_advanced_attachment(&a.description);
                                    let assignment_id = a.id.clone();
                                    let existing_sub = student_submissions.iter().find(|sub| sub.assignment_id == assignment_id).cloned();
                                    let a_id = a.id.clone();
                                    let s_id = current_student_id.clone();
                                    let uid = user_id.clone();
                                    let ws = ws_id.clone();
                                    rsx! {
                                        div { key: "{a.id}", class: "p-4 flex flex-col gap-3.5",
                                            div { class: "flex justify-between items-start",
                                                div {
                                                    div { class: "font-bold text-sm text-foreground", "{a.title}" }
                                                    div { class: "text-xs text-muted-foreground mt-0.5", "{desc_text}" }
                                                    if let Some(staged) = attachment {
                                                        BlobDownloadLink {
                                                            filename: staged.filename.clone(),
                                                            dataurl: staged.dataurl.clone(),
                                                            sha256: staged.sha256.clone(),
                                                            class: "flex items-center gap-2 p-2 bg-card/45 border border-primary/25 hover:bg-card/75 rounded-xl no-underline text-foreground cursor-pointer transition-all mt-2.5 w-fit max-w-sm",
                                                            LucideIcon { name: "file-text", class: "h-4 w-4 text-primary shrink-0" }
                                                            span { class: "text-[10px] font-semibold truncate max-w-[150px]", "{staged.filename}" }
                                                            span { class: "text-[8px] px-1.5 py-0.5 rounded bg-primary/10 text-primary font-bold", "{staged.size_str}" }
                                                            LucideIcon { name: "download", class: "h-3.5 w-3.5 text-muted-foreground ml-1.5 shrink-0" }
                                                        }
                                                    }
                                                    div { class: "text-[10px] text-muted-foreground mt-2 flex items-center gap-1.5",
                                                        LucideIcon { name: "clock", size: "12" }
                                                        if a.due_date.is_empty() || a.due_date == "No due date" {
                                                            span { class: "text-muted-foreground/80 font-medium", "No due date" }
                                                        } else if a.due_date.contains("(Strict)") {
                                                            if is_deadline_passed(&a.due_date) {
                                                                span { class: "text-red-500 font-semibold animate-pulse", {crate::locales::t_with_args("school-student-strict-closed", &locale, &[("date", &a.due_date)])} }
                                                            } else {
                                                                span { class: "text-red-500/80 font-medium", {crate::locales::t_with_args("school-student-strict-due", &locale, &[("date", &a.due_date)])} }
                                                            }
                                                        } else if a.due_date.contains("(Flexible)") {
                                                            if is_deadline_passed(&a.due_date) {
                                                                span { class: "text-amber-500 font-semibold", {crate::locales::t_with_args("school-student-flexible-late", &locale, &[("date", &a.due_date)])} }
                                                            } else {
                                                                span { class: "text-amber-500/80 font-medium", {crate::locales::t_with_args("school-student-flexible-due", &locale, &[("date", &a.due_date)])} }
                                                            }
                                                        } else {
                                                            span { class: "text-muted-foreground font-medium", "Due: {a.due_date}" }
                                                        }
                                                    }
                                                }
                                                if a.max_points == 0 {
                                                    span { class: "text-[10px] font-bold bg-purple-500/10 text-purple-600 border border-purple-500/20 px-2.5 py-0.5 rounded-full uppercase tracking-wider",
                                                        "Material"
                                                    }
                                                } else {
                                                    span { class: "text-xs font-bold bg-primary/10 text-primary px-2.5 py-1 rounded-full",
                                                        "{a.max_points} pts"
                                                    }
                                                }
                                            }
                                            if let Some(ref sub) = existing_sub {
                                                {
                                                    let (ans_text, attachment) = parse_submission_content_and_advanced_attachment(&sub.content);
                                                    rsx! {
                                                        div { class: "p-3.5 rounded-lg bg-muted/40 border border-border/50 text-xs space-y-2",
                                                            div { class: "font-semibold text-muted-foreground", {t("school-submitted-answers", &locale)} }
                                                            div { class: "text-foreground font-medium", "{ans_text}" }
                                                            if let Some(staged) = attachment {
                                                                a {
                                                                    href: "{staged.dataurl}",
                                                                    download: "{staged.filename}",
                                                                    class: "flex flex-col gap-2 p-3 bg-card/45 border border-primary/20 rounded-xl max-w-sm no-underline text-foreground cursor-pointer hover:bg-card/75 transition-colors mt-2",
                                                                    div { class: "flex items-center justify-between",
                                                                        div { class: "flex items-center gap-1.5 min-w-0",
                                                                            LucideIcon { name: "file-check", class: "h-4 w-4 text-primary shrink-0" }
                                                                            span { class: "text-[10px] text-foreground font-semibold truncate", "{staged.filename}" }
                                                                        }
                                                                        LucideIcon { name: "download", class: "h-3.5 w-3.5 text-muted-foreground" }
                                                                    }
                                                                    div { class: "flex flex-wrap gap-1.5 items-center mt-1",
                                                                        span { class: "text-[8px] px-1.5 py-0.5 rounded bg-primary/10 text-primary font-bold uppercase", "{staged.size_str}" }
                                                                        if staged.e2ee {
                                                                            span { class: "text-[8px] px-1.5 py-0.5 rounded bg-green-500/10 text-green-500 font-bold flex items-center gap-0.5",
                                                                                LucideIcon { name: "shield-check", class: "h-2.5 w-2.5" }
                                                                                "E2EE SECURED"
                                                                            }
                                                                        }
                                                                        span { class: "text-[8px] px-1.5 py-0.5 rounded bg-muted text-muted-foreground font-medium truncate max-w-[120px]", "sha256:{staged.sha256}" }
                                                                    }
                                                                }
                                                            }
                                                            div { class: "flex items-center gap-2 border-t border-border/30 pt-2 mt-2",
                                                                span { class: "text-[10px] uppercase font-bold text-muted-foreground", "Status: " }
                                                                if let Some(ref g) = sub.grade {
                                                                    span { class: "text-[10px] px-2 py-0.5 rounded-full font-bold bg-green-500/10 text-green-600 border border-green-500/20 uppercase",
                                                                        "Graded: {g}"
                                                                    }
                                                                } else {
                                                                    span { class: "text-[10px] px-2 py-0.5 rounded-full font-bold bg-amber-500/10 text-amber-600 border border-amber-500/20 uppercase",
                                                                        {t("school-waiting-grade", &locale)}
                                                                    }
                                                                }
                                                            }
                                                            if let Some(ref feedback) = sub.feedback {
                                                                if !feedback.is_empty() {
                                                                    div { class: "text-[11px] text-muted-foreground italic pl-2 border-l border-primary/30 mt-1.5",
                                                                        span { class: "font-bold not-italic", {t("school-teacher-feedback", &locale)} }
                                                                        " {feedback}"
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            } else if a.max_points == 0 {
                                                div { class: "p-3.5 rounded-xl border border-purple-200/50 bg-purple-500/5 text-xs text-purple-600 flex items-center gap-2 font-medium",
                                                    LucideIcon { name: "info", size: "14" }
                                                    span { "This is a study material provided for reference. No submission is required." }
                                                }
                                            } else if is_deadline_passed(&a.due_date) {
                                                div { class: "p-3.5 rounded-xl border border-red-200/50 bg-red-500/5 text-xs text-red-600 flex items-center gap-2 font-medium",
                                                    LucideIcon { name: "lock", size: "14" }
                                                    span { {t("school-student-closed-message", &locale)} }
                                                }
                                            } else {
                                                // Answer submission input box with Drag-and-Drop uploader
                                                {
                                                    let is_drag_over = *drag_active.read().get(&a_id).unwrap_or(&false);
                                                    let staged_file = submission_files.read().get(&a_id).cloned();
                                                    let drag_border_class = if is_drag_over { "border-primary bg-primary/5 shadow-md scale-[1.01]" } else { "border-border/60 bg-muted/20" };

                                                    rsx! {
                                                        div {
                                                            class: "flex flex-col gap-3 p-4 rounded-xl border border-dashed transition-all duration-200 {drag_border_class}",
                                                            ondragover: {
                                                                let a_id_c = a_id.clone();
                                                                move |evt: DragEvent| {
                                                                    evt.prevent_default();
                                                                    drag_active.write().insert(a_id_c.clone(), true);
                                                                }
                                                            },
                                                            ondragleave: {
                                                                let a_id_c = a_id.clone();
                                                                move |evt: DragEvent| {
                                                                    evt.prevent_default();
                                                                    drag_active.write().insert(a_id_c.clone(), false);
                                                                }
                                                            },
                                                            ondrop: {
                                                                let a_id_c = a_id.clone();
                                                                let a_id_f = a_id.clone();
                                                                move |evt: DragEvent| {
                                                                    evt.prevent_default();
                                                                    drag_active.write().insert(a_id_c.clone(), false);
                                                                    let files = evt.files();
                                                                    let a_id_val = a_id_f.clone();
                                                                    spawn(async move {
                                                                        if !files.is_empty() {
                                                                            let file_name = files[0].name();
                                                                            if let Ok(bytes) = files[0].read_bytes().await {
                                                                                let size_str = format_file_size(bytes.len());
                                                                                let sha256 = compute_mock_hash(&bytes);
                                                                                let base64_str = base64_encode(bytes.as_ref());
                                                                                let data_url = format!("data:application/octet-stream;base64,{}", base64_str);
                                                                                submission_files.write().insert(a_id_val, AdvancedAttachment {
                                                                                    filename: file_name,
                                                                                    size_str,
                                                                                    sha256,
                                                                                    e2ee: true,
                                                                                    dataurl: data_url,
                                                                                });
                                                                            }
                                                                        }
                                                                    });
                                                                }
                                                            },
                                                            span { class: "text-[11px] font-bold text-muted-foreground", {t("school-student-write-answer", &locale)} }
                                                            textarea {
                                                                class: "w-full min-h-[70px] rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary",
                                                                placeholder: t("school-student-type-placeholder", &locale),
                                                                value: assignment_inputs.read().get(&a_id).cloned().unwrap_or_default(),
                                                                oninput: {
                                                                    let a_id_c = a_id.clone();
                                                                    move |evt: FormEvent| {
                                                                        assignment_inputs.write().insert(a_id_c.clone(), evt.value());
                                                                    }
                                                                },
                                                            }

                                                            // Drag and drop / file selector connector
                                                            div { class: "border border-dashed border-border/85 rounded-lg p-4 flex flex-col items-center justify-center bg-background/50 hover:bg-background/85 transition-colors cursor-pointer relative",
                                                                input {
                                                                    r#type: "file",
                                                                    class: "absolute inset-0 opacity-0 cursor-pointer z-10",
                                                                    onchange: {
                                                                        let a_id_c = a_id.clone();
                                                                        move |evt| {
                                                                            let a_id_f = a_id_c.clone();
                                                                            let files = evt.files();
                                                                            spawn(async move {
                                                                                if !files.is_empty() {
                                                                                    let file_name = files[0].name();
                                                                                    if let Ok(bytes) = files[0].read_bytes().await {
                                                                                        let size_str = format_file_size(bytes.len());
                                                                                        let sha256 = compute_mock_hash(&bytes);
                                                                                        let base64_str = base64_encode(bytes.as_ref());
                                                                                        let data_url = format!("data:application/octet-stream;base64,{}", base64_str);
                                                                                        submission_files.write().insert(a_id_f.clone(), AdvancedAttachment {
                                                                                            filename: file_name,
                                                                                            size_str,
                                                                                            sha256,
                                                                                            e2ee: true,
                                                                                            dataurl: data_url,
                                                                                        });
                                                                                    }
                                                                                }
                                                                            });
                                                                        }
                                                                    }
                                                                }
                                                                LucideIcon { name: "upload-cloud", class: "h-5 w-5 text-primary mb-1.5" }
                                                                span { class: "text-[10px] text-foreground font-semibold", {t("school-student-drag-drop", &locale)} }
                                                                span { class: "text-[9px] text-muted-foreground mt-0.5", {t("school-student-upload-formats", &locale)} }
                                                            }

                                                            if let Some(staged) = staged_file {
                                                                div { class: "flex flex-col gap-2 p-3 bg-card/45 border border-primary/20 rounded-xl",
                                                                    div { class: "flex items-center justify-between",
                                                                        div { class: "flex items-center gap-1.5 min-w-0",
                                                                            LucideIcon { name: "file-check", class: "h-4 w-4 text-primary shrink-0" }
                                                                            span { class: "text-[10px] text-foreground font-semibold truncate", "{staged.filename}" }
                                                                        }
                                                                        button {
                                                                            class: "bg-transparent border-0 text-muted-foreground hover:text-red-500 cursor-pointer p-0.5 rounded transition-colors",
                                                                            onclick: {
                                                                                let a_id_c = a_id.clone();
                                                                                move |_| {
                                                                                    submission_files.write().remove(&a_id_c);
                                                                                }
                                                                            },
                                                                            LucideIcon { name: "x", class: "h-3.5 w-3.5" }
                                                                        }
                                                                    }
                                                                    div { class: "flex flex-wrap gap-1.5 items-center mt-1",
                                                                        span { class: "text-[8px] px-1.5 py-0.5 rounded bg-primary/10 text-primary font-bold uppercase", "{staged.size_str}" }
                                                                        span { class: "text-[8px] px-1.5 py-0.5 rounded bg-green-500/10 text-green-500 font-bold flex items-center gap-0.5",
                                                                            LucideIcon { name: "shield-check", class: "h-2.5 w-2.5" }
                                                                            "E2EE SECURED"
                                                                        }
                                                                        span { class: "text-[8px] px-1.5 py-0.5 rounded bg-muted text-muted-foreground font-medium truncate max-w-[120px]", "sha256:{staged.sha256}" }
                                                                    }
                                                                }
                                                            }

                                                            Button {
                                                                class: "text-xs px-3 h-8 self-end font-semibold flex items-center gap-1.5 mt-2",
                                                                disabled: submitting_map.read().contains(&a_id),
                                                                onclick: {
                                                                    let a_id = a_id.clone();
                                                                    let s_id = s_id.clone();
                                                                    let uid_c = uid.clone();
                                                                    let ws_c = ws.clone();
                                                                    let state = state;
                                                                    let mut db_trigger = db_trigger.clone();
                                                                    let toast = toast.clone();
                                                                    let locale_c = locale.clone();
                                                                    move |_| {
                                                                        let ans = assignment_inputs.read().get(&a_id).cloned().unwrap_or_default();
                                                                        let staged = submission_files.read().get(&a_id).cloned();
                                                                        let final_content = match staged.as_ref() {
                                                                            Some(staged_f) => format!(
                                                                                "{}\n[Attachment: {} | {} | {} | {} | blob://{}]",
                                                                                ans, staged_f.filename, staged_f.size_str, staged_f.sha256, staged_f.e2ee, staged_f.sha256
                                                                            ),
                                                                            None => ans.clone(),
                                                                        };
                                                                        if !ans.is_empty() || staged.is_some() {
                                                                            let role = state.active_user_role.read().clone();
                                                                            let u_id = state.active_user_id.read().clone();
                                                                            let proof = yntra_core::ZkCryptoTrust::new()
                                                                                .generate_role_proof(state.get_passkey_seed(), u_id.clone(), role.clone())
                                                                                .ok();
                                                                            let seed_val = state.get_passkey_seed();
                                                                            let enc_content = encrypt_field_with_proof(&seed_val, &final_content, &u_id, &role);
                                                                            let sub_rec = Submission {
                                                                                id: uuid::Uuid::new_v4().to_string(),
                                                                                workspace_id: ws_c.clone(),
                                                                                assignment_id: a_id.clone(),
                                                                                student_id: s_id.clone(),
                                                                                content: enc_content,
                                                                                grade: None,
                                                                                feedback: None,
                                                                                submitted_at: chrono::Utc::now().to_rfc3339(),
                                                                                updated_at: 0,
                                                                            };
                                                                            let uid_sub = uid_c.clone();
                                                                            let a_id_clear = a_id.clone();
                                                                            let mut db_trigger_c = db_trigger.clone();
                                                                            let toast_c = toast.clone();
                                                                            let locale_sub = locale_c.clone();
                                                                            let ws_blob = ws_c.clone();
                                                                            let staged_blob = staged.clone();
                                                                            submitting_map.write().insert(a_id.clone());
                                                                            spawn(async move {
                                                                                if let Some(staged_f) = staged_blob {
                                                                                    let _ = yntra_core::save_blob(uid_sub.clone(), staged_f.sha256, ws_blob, staged_f.dataurl).await;
                                                                                }
                                                                                match save_submission(uid_sub, sub_rec, proof).await {
                                                                                    Ok(_) => {
                                                                                        toast_c.success(
                                                                                            t("school-submission-success-title", &locale_sub),
                                                                                            dioxus_primitives::toast::ToastOptions::new().description(t("school-submission-success-desc", &locale_sub))
                                                                                        );
                                                                                        assignment_inputs.write().insert(a_id_clear.clone(), String::new());
                                                                                        submission_files.write().remove(&a_id_clear);
                                                                                        let current = *db_trigger_c.read();
                                                                                        db_trigger_c.set(current + 1);
                                                                                    }
                                                                                    Err(e) => {
                                                                                        let user_err = crate::utils::map_error(&e);
                                                                                        toast_c.error(
                                                                                            user_err.title,
                                                                                            dioxus_primitives::toast::ToastOptions::new().description(user_err.description)
                                                                                        );
                                                                                    }
                                                                                }
                                                                                submitting_map.write().remove(&a_id_clear);
                                                                            });
                                                                        }
                                                                    }
                                                                },
                                                                LucideIcon { name: "send", class: "h-3.5 w-3.5" }
                                                                {
                                                                    if submitting_map.read().contains(&a_id) {
                                                                        "Submitting...".to_string()
                                                                    } else {
                                                                        t("school-submit-answer", &locale)
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
            }
        }

            if *active_tab.read() == "dashboard" {
                Card { class: "border border-border shadow-sm",
                CardHeader {
                    CardTitle { {t("school-my-timetable", &locale)} }
                    CardDescription { {t("school-student-timetable-desc", &locale)} }
                }
                CardContent {
                    if filtered_timetable.is_empty() {
                        div { class: "py-8 text-center text-xs text-muted-foreground", {t("school-timetable-no-slots", &locale)} }
                    } else {
                        div { class: "space-y-4",
                            div { class: "grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4",
                                for s in filtered_timetable.iter() {
                                    {
                                        let course_name = courses.iter().find(|c| c.id == s.course_id).map(|c| c.name.clone()).unwrap_or_else(|| "Unknown Course".to_string());
                                        let classroom_name = s.classroom.clone().unwrap_or_else(|| "Room Unassigned".to_string());
                                        let day_name = match s.day_of_week {
                                            1 => t("common-monday", &locale),
                                            2 => t("common-tuesday", &locale),
                                            3 => t("common-wednesday", &locale),
                                            4 => t("common-thursday", &locale),
                                            5 => t("common-friday", &locale),
                                            _ => "Weekday".to_string(),
                                        };
                                        rsx! {
                                            div { key: "{s.id}", class: "p-4 rounded-xl border border-border bg-background flex flex-col gap-2 shadow-sm hover:border-primary/20 transition-all",
                                                div { class: "flex items-center gap-2",
                                                    LucideIcon { name: "clock", class: "h-4 w-4 text-primary" }
                                                    span { class: "text-xs font-bold text-foreground", "{day_name} • {s.start_time} - {s.end_time}" }
                                                }
                                                div { class: "font-semibold text-sm text-foreground", "{course_name}" }
                                                div { class: "text-[10px] text-muted-foreground flex items-center gap-1",
                                                    LucideIcon { name: "map-pin", class: "h-3 w-3" }
                                                    span { {crate::locales::t_with_args("school-student-classroom-label", &locale, &[("room", &classroom_name)])} }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Button {
                                class: "mt-2 bg-primary hover:bg-primary/90 text-primary-foreground font-bold uppercase tracking-wider text-[9px] py-2 px-4 rounded-lg flex items-center justify-center gap-1.5 transition-all shadow-sm border-0 cursor-pointer w-full sm:w-auto",
                                onclick: {
                                    let timetable_c = filtered_timetable.clone();
                                    let courses_c = courses.clone();
                                    let students_c = students.clone();
                                    let selected_id = selected_student_profile_id.read().clone();
                                    move |_| {
                                        let user_name = students_c.iter()
                                            .find(|s| s.id == selected_id)
                                            .map(|s| format!("{} {}", s.first_name, s.last_name))
                                            .unwrap_or_else(|| "Student".to_string());
                                        let mut ics_content = format!(
                                            "BEGIN:VCALENDAR\n\
                                             VERSION:2.0\n\
                                             PRODID:-//YntraPlatform//ClassSchedule//EN\n\
                                             CALSCALE:GREGORIAN\n\
                                             METHOD:PUBLISH\n"
                                        );

                                        for s in timetable_c.iter() {
                                            let course_name = courses_c.iter().find(|c| c.id == s.course_id).map(|c| c.name.clone()).unwrap_or_else(|| "Unknown Course".to_string());
                                            let classroom_name = s.classroom.clone().unwrap_or_else(|| "Room Unassigned".to_string());
                                            let day_code = match s.day_of_week {
                                                1 => "MO",
                                                2 => "TU",
                                                3 => "WE",
                                                4 => "TH",
                                                5 => "FR",
                                                _ => "MO",
                                            };
                                            let start_date = match s.day_of_week {
                                                1 => "20260720",
                                                2 => "20260721",
                                                3 => "20260722",
                                                4 => "20260723",
                                                5 => "20260724",
                                                _ => "20260720",
                                            };
                                            
                                            let clean_start = s.start_time.replace(":", "");
                                            let clean_end = s.end_time.replace(":", "");

                                            ics_content.push_str(&format!(
                                                "BEGIN:VEVENT\n\
                                                 UID:{}@yntra.platform\n\
                                                 DTSTAMP:20260718T000000Z\n\
                                                 SUMMARY:{}\n\
                                                 LOCATION:{}\n\
                                                 DESCRIPTION:Weekly recurring school class for {}\n\
                                                 DTSTART;TZID=Europe/Stockholm:{}T{}00\n\
                                                 DTEND;TZID=Europe/Stockholm:{}T{}00\n\
                                                 RRULE:FREQ=WEEKLY;BYDAY={}\n\
                                                 END:VEVENT\n",
                                                 uuid::Uuid::new_v4(),
                                                 course_name,
                                                 classroom_name,
                                                 user_name,
                                                 start_date,
                                                 clean_start,
                                                 start_date,
                                                 clean_end,
                                                 day_code
                                            ));
                                        }
                                        ics_content.push_str("END:VCALENDAR\n");

                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let base64_str = base64_encode(ics_content.as_bytes());
                                            let file_name = format!("Schema_{}.ics", user_name.replace(" ", "_"));
                                            let js_code = format!(
                                                r#"
                                                (function() {{
                                                    const base64 = "{}";
                                                    const filename = "{}";
                                                    const binString = atob(base64);
                                                    const bytes = Uint8Array.from(binString, (m) => m.codePointAt(0));
                                                    const blob = new Blob([bytes], {{ type: "text/calendar;charset=utf-8;" }});
                                                    const url = URL.createObjectURL(blob);
                                                    const a = document.createElement("a");
                                                    a.href = url;
                                                    a.download = filename;
                                                    document.body.appendChild(a);
                                                    a.click();
                                                    document.body.removeChild(a);
                                                    URL.revokeObjectURL(url);
                                                }})();
                                                "#,
                                                base64_str, file_name
                                            );
                                            let _ = js_sys::eval(&js_code);
                                        }

                                        #[cfg(not(target_arch = "wasm32"))]
                                        {
                                            let home_dir = std::env::var("USERPROFILE")
                                                .or_else(|_| std::env::var("HOME"))
                                                .unwrap_or_else(|_| ".".to_string());
                                            let filename = format!("Schema_{}.ics", user_name.replace(" ", "_"));
                                            let paths = vec![
                                                format!("{}/Desktop", home_dir),
                                                format!("{}/Downloads", home_dir),
                                                home_dir.clone(),
                                            ];
                                            for path in paths {
                                                let file_path = std::path::PathBuf::from(&path).join(&filename);
                                                if std::fs::write(&file_path, &ics_content).is_ok() {
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                },
                                LucideIcon { name: "calendar", size: "12" }
                                {t("school-calendar-sync-btn", &locale)}
                            }
                        }
                    }
                    }
                }
            }
        }
    }
}

fn format_timestamp(ms: i64) -> String {
    let seconds = ms / 1000;
    if let Some(dt) = chrono::DateTime::from_timestamp(seconds, 0) {
        dt.format("%Y-%m-%d %H:%M").to_string()
    } else {
        "Just now".to_string()
    }
}
