use dioxus::prelude::*;
use crate::components;

// 1. Student Homework Submission Modal
#[derive(Props, Clone, PartialEq)]
pub struct HomeworkSubmitModalProps {
    pub open: bool,
    pub title: String,
    pub desc: String,
    pub submission_text: Signal<String>,
    pub due_date: String,
    pub late_policy: String,
    pub onclose: EventHandler<()>,
    pub onsubmit: EventHandler<()>,
}

#[component]
pub fn HomeworkSubmitModal(props: HomeworkSubmitModalProps) -> Element {
    let mut submission_text = props.submission_text;
    
    let today = yntra_core::infra::time::get_current_datetime_str()[0..10].to_string(); // "YYYY-MM-DD"
    let is_late = today > props.due_date;
    let policy = props.late_policy.clone();
    
    let can_submit = !is_late || policy != "hard_deadline";
    
    rsx! {
        if props.open {
            components::Dialog {
                open: props.open,
                title: format!("Submit: {}", props.title),
                onclose: move |_| props.onclose.call(()),
                div { class: "flex flex-col gap-4 text-sm w-96",
                    div { class: "text-xs text-muted-foreground italic bg-sidebar/30 p-2.5 rounded border border-border/40",
                        "{props.desc}"
                    }
                    
                    if is_late {
                        div { class: "p-3 rounded-lg border text-xs font-semibold flex flex-col gap-1 bg-red-500/10 border-red-500/20 text-red-400",
                            span { "⚠️ Assignment is past the due date ({props.due_date})!" }
                            if policy == "hard_deadline" {
                                span { class: "font-black text-[10px] uppercase tracking-wider text-red-500", "Submissions are blocked by course policy." }
                            } else if policy == "penalty_5" {
                                span { "A 5% daily deduction penalty will be applied to your grade." }
                            } else if policy == "penalty_10" {
                                span { "A 10% daily deduction penalty will be applied to your grade." }
                            }
                        }
                    }
                    
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Your Answer / Homework Submission" }
                        textarea {
                            class: "yntra-input py-2 px-3 h-24 text-xs text-foreground bg-background border border-border",
                            style: "resize: none;",
                            placeholder: "Type your answer or submit assignment details...",
                            value: "{submission_text}",
                            oninput: move |e| submission_text.set(e.value()),
                            disabled: !can_submit
                        }
                    }
                    
                    button {
                        class: "yntra-btn mt-2 flex items-center justify-center gap-1.5",
                        disabled: !can_submit,
                        onclick: move |_| props.onsubmit.call(()),
                        components::LucideIcon { name: "check", size: "14" }
                        "Submit Answer"
                    }
                }
            }
        }
    }
}

// 2. Student Enrollment Modal
#[derive(Props, Clone, PartialEq)]
pub struct StudentEnrollModalProps {
    pub open: bool,
    pub first_name: Signal<String>,
    pub last_name: Signal<String>,
    pub grade: Signal<String>,
    pub contact: Signal<String>,
    pub onclose: EventHandler<()>,
    pub onsubmit: EventHandler<()>,
}

#[component]
pub fn StudentEnrollModal(props: StudentEnrollModalProps) -> Element {
    let mut first_name = props.first_name;
    let mut last_name = props.last_name;
    let mut grade = props.grade;
    let mut contact = props.contact;

    rsx! {
        if props.open {
            components::Dialog {
                open: props.open,
                title: "Enroll New Student".to_string(),
                onclose: move |_| props.onclose.call(()),
                div { class: "flex flex-col gap-4 text-sm w-80",
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "First Name" }
                        input { class: "yntra-input", placeholder: "e.g. Liam", value: "{first_name}", oninput: move |e| first_name.set(e.value()) }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Last Name" }
                        input { class: "yntra-input", placeholder: "e.g. Johansson", value: "{last_name}", oninput: move |e| last_name.set(e.value()) }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Grade Level / Class" }
                        select {
                            class: "yntra-input py-2 px-3 text-xs bg-background border border-border text-foreground",
                            value: "{grade}",
                            onchange: move |e| grade.set(e.value()),
                            option { value: "Grade 7", "Grade 7" }
                            option { value: "Grade 8", "Grade 8" }
                            option { value: "Grade 9", "Grade 9" }
                            option { value: "Grade 10", "Grade 10" }
                            option { value: "Gymnasiet Yrkesförberedande", "Gymnasiet (Yrkesförberedande)" }
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Parent Contact Email / Phone" }
                        input { class: "yntra-input", placeholder: "e.g. parent@example.se", value: "{contact}", oninput: move |e| contact.set(e.value()) }
                    }
                    
                    button {
                        class: "yntra-btn mt-2",
                        onclick: move |_| props.onsubmit.call(()),
                        "Enroll Student"
                    }
                }
            }
        }
    }
}

// 3. Course Creation Modal
#[derive(Props, Clone, PartialEq)]
pub struct CourseCreateModalProps {
    pub open: bool,
    pub name: Signal<String>,
    pub subject: Signal<String>,
    pub classroom: Signal<String>,
    pub onclose: EventHandler<()>,
    pub onsubmit: EventHandler<()>,
}

#[component]
pub fn CourseCreateModal(props: CourseCreateModalProps) -> Element {
    let mut name = props.name;
    let mut subject = props.subject;
    let mut classroom = props.classroom;

    rsx! {
        if props.open {
            components::Dialog {
                open: props.open,
                title: "Add New Course".to_string(),
                onclose: move |_| props.onclose.call(()),
                div { class: "flex flex-col gap-4 text-sm w-80",
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Course Name" }
                        input { class: "yntra-input", placeholder: "e.g. Algebra I", value: "{name}", oninput: move |e| name.set(e.value()) }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Subject" }
                        input { class: "yntra-input", placeholder: "e.g. Math, Physics, Art", value: "{subject}", oninput: move |e| subject.set(e.value()) }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Classroom / Room" }
                        input { class: "yntra-input", placeholder: "e.g. Room 204", value: "{classroom}", oninput: move |e| classroom.set(e.value()) }
                    }
                    
                    button {
                        class: "yntra-btn mt-2",
                        onclick: move |_| props.onsubmit.call(()),
                        "Create Course"
                    }
                }
            }
        }
    }
}

// 4. Assignment Creation Modal
#[derive(Props, Clone, PartialEq)]
pub struct AssignmentCreateModalProps {
    pub open: bool,
    pub title: Signal<String>,
    pub desc: Signal<String>,
    pub due: Signal<String>,
    pub points: Signal<String>,
    pub onclose: EventHandler<()>,
    pub onsubmit: EventHandler<()>,
}

#[component]
pub fn AssignmentCreateModal(props: AssignmentCreateModalProps) -> Element {
    let mut title = props.title;
    let mut desc = props.desc;
    let mut due = props.due;
    let mut points = props.points;

    rsx! {
        if props.open {
            components::Dialog {
                open: props.open,
                title: "Create New Assignment".to_string(),
                onclose: move |_| props.onclose.call(()),
                div { class: "flex flex-col gap-4 text-sm w-80",
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Title" }
                        input { class: "yntra-input", placeholder: "e.g. Algebra Quiz 1", value: "{title}", oninput: move |e| title.set(e.value()) }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Instructions / Description" }
                        textarea {
                            class: "yntra-input py-2 px-3 h-20 text-xs text-foreground bg-background border border-border",
                            style: "resize: none;",
                            placeholder: "Provide assignment details...",
                            value: "{desc}",
                            oninput: move |e| desc.set(e.value())
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Due Date" }
                        input { r#type: "date", class: "yntra-input", value: "{due}", oninput: move |e| due.set(e.value()) }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Maximum Points" }
                        input { r#type: "number", class: "yntra-input", value: "{points}", oninput: move |e| points.set(e.value()) }
                    }
                    
                    button {
                        class: "yntra-btn mt-2",
                        onclick: move |_| props.onsubmit.call(()),
                        "Post Assignment"
                    }
                }
            }
        }
    }
}

// 5. Submission Grading Modal
#[derive(Props, Clone, PartialEq)]
pub struct GradeSubmissionModalProps {
    pub open: bool,
    pub grade: Signal<String>,
    pub feedback: Signal<String>,
    pub grading_system: String,
    pub onclose: EventHandler<()>,
    pub onsubmit: EventHandler<()>,
}

#[component]
pub fn GradeSubmissionModal(props: GradeSubmissionModalProps) -> Element {
    let mut grade = props.grade;
    let mut feedback = props.feedback;
    let grading_system = props.grading_system.clone();

    rsx! {
        if props.open {
            components::Dialog {
                open: props.open,
                title: "Grade Submission".to_string(),
                onclose: move |_| props.onclose.call(()),
                div { class: "flex flex-col gap-4 text-sm w-80",
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Grade" }
                        
                        if grading_system == "1-100" {
                            input {
                                r#type: "number",
                                min: "0",
                                max: "100",
                                class: "yntra-input py-2 px-3 text-xs bg-background border border-border text-foreground h-9 rounded-lg focus:outline-none focus:ring-1 focus:ring-primary",
                                placeholder: "Enter score (0-100)",
                                value: "{grade}",
                                oninput: move |e| grade.set(e.value()),
                            }
                        } else {
                            select {
                                class: "yntra-input py-2 px-3 text-xs bg-background border border-border text-foreground h-9 rounded-lg focus:outline-none",
                                value: "{grade}",
                                onchange: move |e| grade.set(e.value()),
                                if grading_system == "U-G" {
                                    option { value: "U", "U (Underkänd)" }
                                    option { value: "G", "G (Godkänd)" }
                                } else if grading_system == "U-G-VG" {
                                    option { value: "U", "U (Underkänd)" }
                                    option { value: "G", "G (Godkänd)" }
                                    option { value: "VG", "VG (Väl Godkänd)" }
                                } else if grading_system == "U-3-4-5" {
                                    option { value: "U", "U (Underkänd)" }
                                    option { value: "3", "3" }
                                    option { value: "4", "4" }
                                    option { value: "5", "5" }
                                } else if grading_system == "1-10" {
                                    option { value: "1", "1" }
                                    option { value: "2", "2" }
                                    option { value: "3", "3" }
                                    option { value: "4", "4" }
                                    option { value: "5", "5" }
                                    option { value: "6", "6" }
                                    option { value: "7", "7" }
                                    option { value: "8", "8" }
                                    option { value: "9", "9" }
                                    option { value: "10", "10" }
                                } else {
                                    option { value: "A", "A" }
                                    option { value: "B", "B" }
                                    option { value: "C", "C" }
                                    option { value: "D", "D" }
                                    option { value: "E", "E" }
                                    option { value: "F", "F" }
                                }
                            }
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-bold text-muted-foreground uppercase", "Feedback" }
                        textarea {
                            class: "yntra-input py-2 px-3 h-20 text-xs text-foreground bg-background border border-border",
                            style: "resize: none;",
                            placeholder: "Add feedback notes for the student...",
                            value: "{feedback}",
                            oninput: move |e| feedback.set(e.value())
                        }
                    }
                    
                    button {
                        class: "yntra-btn mt-2",
                        onclick: move |_| props.onsubmit.call(()),
                        "Save Grade"
                    }
                }
            }
        }
    }
}

// 6. School Setup Wizard Modal
#[derive(Props, Clone, PartialEq)]
pub struct SchoolSetupWizardModalProps {
    pub open: bool,
    pub onsubmit: EventHandler<(String, String, String, String)>,
}

#[component]
pub fn SchoolSetupWizardModal(props: SchoolSetupWizardModalProps) -> Element {
    let mut step = use_signal(|| 1);
    let mut selected_country = use_signal(|| "SE".to_string());
    let mut selected_school_type = use_signal(|| "secondary".to_string());
    let mut selected_grading = use_signal(|| "A-F".to_string());
    let mut selected_late = use_signal(|| "none".to_string());

    rsx! {
        if props.open {
            components::Dialog {
                open: props.open,
                title: "Initialize School Portal Defaults".to_string(),
                onclose: move |_| {}, // cannot close without completing onboarding
                div { class: "flex flex-col gap-5 text-sm w-[420px] p-1",
                    // Progress Indicator Header
                    div { class: "flex justify-between items-center text-xs text-muted-foreground border-b border-border/40 pb-3",
                        span { class: "font-semibold", "Step {step} of 3" }
                        div { class: "flex gap-1.5",
                            div { class: format!("w-8 h-1.5 rounded-full transition-all duration-300 {}", if *step.read() >= 1 { "bg-primary" } else { "bg-muted" }) }
                            div { class: format!("w-8 h-1.5 rounded-full transition-all duration-300 {}", if *step.read() >= 2 { "bg-primary" } else { "bg-muted" }) }
                            div { class: format!("w-8 h-1.5 rounded-full transition-all duration-300 {}", if *step.read() >= 3 { "bg-primary" } else { "bg-muted" }) }
                        }
                    }

                    if *step.read() == 1 {
                        div { class: "flex flex-col gap-4",
                            div { class: "flex flex-col gap-1",
                                h4 { class: "text-sm font-bold text-foreground m-0", "Regional & Institution Setup" }
                                p { class: "text-xs text-muted-foreground m-0 leading-relaxed", "Configure your country and type of education to pre-select defaults." }
                            }
                            
                            // Country Cards Grid
                            div { class: "flex flex-col gap-2",
                                label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground/80", "Country / Region" }
                                div { class: "grid grid-cols-2 gap-2",
                                    for (val, flag, label, desc) in &[
                                        ("SE", "🇸🇪", "Sweden", "sv • Stockholm time"),
                                        ("US", "🇺🇸", "United States", "en • US timezones"),
                                        ("GB", "🇬🇧", "United Kingdom", "en • London time"),
                                        ("Other", "🌐", "International", "UTC • Custom locale"),
                                    ] {
                                        div {
                                            class: format!("border p-2.5 rounded-xl cursor-pointer hover:border-primary/50 transition-all flex items-center gap-2.5 bg-white/[0.01] {}",
                                                if *selected_country.read() == *val { "border-primary bg-primary/5 ring-1 ring-primary/30" } else { "border-border/30" }
                                            ),
                                            onclick: move |_| selected_country.set(val.to_string()),
                                            span { class: "text-xl", "{flag}" }
                                            div { class: "flex flex-col",
                                                span { class: "font-black text-foreground text-[11px]", "{label}" }
                                                span { class: "text-[9px] text-muted-foreground", "{desc}" }
                                            }
                                        }
                                    }
                                }
                            }

                            // School Type Grid
                            div { class: "flex flex-col gap-2",
                                label { class: "text-[10px] font-bold uppercase tracking-wider text-muted-foreground/80", "School Type / Level" }
                                div { class: "grid grid-cols-3 gap-2",
                                    for (val, icon, label, desc) in &[
                                        ("primary", "baby", "Primary", "Grades 1-9"),
                                        ("secondary", "book-open", "Gymnasium", "Grades 10-12"),
                                        ("university", "graduation-cap", "University", "Higher Ed"),
                                    ] {
                                        div {
                                            class: format!("border p-3 rounded-xl cursor-pointer hover:border-primary/50 transition-all flex flex-col items-center text-center gap-1.5 bg-white/[0.01] {}",
                                                if *selected_school_type.read() == *val { "border-primary bg-primary/5 ring-1 ring-primary/30" } else { "border-border/30" }
                                            ),
                                            onclick: move |_| selected_school_type.set(val.to_string()),
                                            components::LucideIcon { name: *icon, size: "18", class: "text-primary/70" }
                                            div { class: "flex flex-col",
                                                span { class: "font-black text-foreground text-[10px] whitespace-nowrap", "{label}" }
                                                span { class: "text-[8px] text-muted-foreground mt-0.5", "{desc}" }
                                            }
                                        }
                                    }
                                }
                            }
                            
                            button {
                                class: "yntra-btn text-xs font-bold py-2.5 mt-2 flex items-center justify-center gap-1.5 shadow-md",
                                onclick: move |_| {
                                    let c = selected_country.read().clone();
                                    let t = selected_school_type.read().clone();
                                    if c == "SE" {
                                        if t == "primary" {
                                            selected_grading.set("U-G".to_string());
                                        } else if t == "secondary" {
                                            selected_grading.set("A-F".to_string());
                                        } else if t == "university" {
                                            selected_grading.set("U-G-VG".to_string());
                                        }
                                    } else {
                                        selected_grading.set("A-F".to_string());
                                    }
                                    step.set(2);
                                },
                                "Continue to Grading Defaults"
                                components::LucideIcon { name: "arrow-right", size: "14" }
                            }
                        }
                    } else if *step.read() == 2 {
                        div { class: "flex flex-col gap-3",
                            div { class: "flex flex-col gap-1",
                                h4 { class: "text-sm font-bold text-foreground m-0", "Choose default grading system" }
                                p { class: "text-xs text-muted-foreground m-0 leading-relaxed", "Pre-selected based on your region. You can adjust the scale below." }
                            }
                            
                            div { class: "flex flex-col gap-2 max-h-[300px] overflow-y-auto pr-1",
                                for (val, title, badges) in &[
                                    ("A-F", "A-F (Letter Grades)", &["A", "B", "C", "D", "E", "F"] as &[&str]),
                                    ("U-G-VG", "U, G, VG (Swedish University)", &["U", "G", "VG"] as &[&str]),
                                    ("U-G", "U, G (Swedish Pass/Fail)", &["U", "G"] as &[&str]),
                                    ("U-3-4-5", "U, 3, 4, 5 (Swedish Engineering)", &["U", "3", "4", "5"] as &[&str]),
                                    ("1-100", "0-100 (Percentage Scale)", &["95%"] as &[&str]),
                                    ("1-10", "1-10 (Numeric Scale)", &["8.5"] as &[&str]),
                                ] {
                                    div {
                                        class: format!("border p-3 rounded-xl cursor-pointer hover:border-primary/60 transition-all bg-white/[0.01] flex justify-between items-center {}",
                                            if *selected_grading.read() == *val { "border-primary bg-primary/5 ring-1 ring-primary/30" } else { "border-border/30" }
                                        ),
                                        onclick: move |_| selected_grading.set(val.to_string()),
                                        div { class: "flex flex-col gap-0.5",
                                            div { class: "font-black text-foreground text-xs", "{title}" }
                                            div { class: "text-[9px] text-muted-foreground max-w-[200px]", 
                                                match *val {
                                                    "A-F" => "Standard letter grade evaluation scale",
                                                    "U-G-VG" => "Swedish Pass with distinction, Pass, Fail scale",
                                                    "U-G" => "Swedish simplified pass/fail marks",
                                                    "U-3-4-5" => "Swedish technical engineering grades",
                                                    "1-100" => "Detailed percentage score system",
                                                    _ => "Standard numeric rating marks (1-10)"
                                                }
                                            }
                                        }
                                        div { class: "flex gap-1 items-center flex-wrap max-w-[120px] justify-end",
                                            for b in *badges {
                                                span { class: "text-[8px] font-extrabold px-1.5 py-0.5 rounded bg-sidebar/50 text-foreground border border-border/20 shadow-sm", "{b}" }
                                            }
                                        }
                                    }
                                }
                            }
                            
                            div { class: "flex gap-2.5 mt-2",
                                button {
                                    class: "yntra-btn border border-border/40 bg-transparent text-muted-foreground hover:text-foreground text-xs font-bold flex-1 py-2 flex items-center justify-center gap-1",
                                    onclick: move |_| step.set(1),
                                    components::LucideIcon { name: "arrow-left", size: "14" }
                                    "Back"
                                }
                                button {
                                    class: "yntra-btn text-xs font-bold flex-1 py-2 flex items-center justify-center gap-1 shadow-md",
                                    onclick: move |_| step.set(3),
                                    "Continue"
                                    components::LucideIcon { name: "arrow-right", size: "14" }
                                }
                            }
                        }
                    } else {
                        div { class: "flex flex-col gap-3",
                            div { class: "flex flex-col gap-1",
                                h4 { class: "text-sm font-bold text-foreground m-0", "Choose late submission policy" }
                                p { class: "text-xs text-muted-foreground m-0 leading-relaxed", "Set workspace behaviors for homework submitted past the due date." }
                            }
                            
                            div { class: "flex flex-col gap-2.5",
                                for (val, title, desc, icon, badge) in &[
                                    ("none", "None (No Penalties)", "Late assignments are accepted normally with no score deductions", "check-circle", "Free"),
                                    ("hard_deadline", "Hard Deadline", "Block student upload attempts completely after the due date", "x-circle", "Block"),
                                    ("penalty_5", "5% Daily Deduction", "Deduct 5% of max points for each day late", "minus-circle", "-5%/day"),
                                    ("penalty_10", "10% Daily Deduction", "Deduct 10% of max points for each day late", "minus-circle", "-10%/day"),
                                ] {
                                    div {
                                        class: format!("border p-3 rounded-xl cursor-pointer hover:border-primary/60 transition-all bg-white/[0.01] flex justify-between items-center {}",
                                            if *selected_late.read() == *val { "border-primary bg-primary/5 ring-1 ring-primary/30" } else { "border-border/30" }
                                        ),
                                        onclick: move |_| selected_late.set(val.to_string()),
                                        div { class: "flex items-start gap-2.5",
                                            components::LucideIcon { name: *icon, size: "18", class: "text-primary/70 mt-0.5" }
                                            div { class: "flex flex-col gap-0.5",
                                                div { class: "font-black text-foreground text-xs", "{title}" }
                                                div { class: "text-[9px] text-muted-foreground leading-normal max-w-[220px]", "{desc}" }
                                            }
                                        }
                                        span { class: "text-[8px] font-black px-2 py-0.5 rounded-full bg-primary/10 text-primary border border-primary/20", "{badge}" }
                                    }
                                }
                            }
                            
                            div { class: "flex gap-2.5 mt-2",
                                button {
                                    class: "yntra-btn border border-border/40 bg-transparent text-muted-foreground hover:text-foreground text-xs font-bold flex-1 py-2 flex items-center justify-center gap-1",
                                    onclick: move |_| step.set(2),
                                    components::LucideIcon { name: "arrow-left", size: "14" }
                                    "Back"
                                }
                                button {
                                    class: "yntra-btn text-xs font-bold flex-1 py-2 flex items-center justify-center gap-1 shadow-md",
                                    onclick: move |_| {
                                        let g = selected_grading.read().clone();
                                        let l = selected_late.read().clone();
                                        let c = selected_country.read().clone();
                                        let t = selected_school_type.read().clone();
                                        props.onsubmit.call((g, l, c, t));
                                    },
                                    components::LucideIcon { name: "check", size: "14" }
                                    "Finish Setup"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
