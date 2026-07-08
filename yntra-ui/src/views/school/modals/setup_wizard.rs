use dioxus::prelude::*;
use crate::components;

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
                                            key: "{val}",
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
                                            key: "{val}",
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
                                        key: "{val}",
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
                                                span {
                                                    key: "{b}",
                                                    class: "text-[8px] font-extrabold px-1.5 py-0.5 rounded bg-sidebar/50 text-foreground border border-border/20 shadow-sm", "{b}"
                                                }
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
                                        key: "{val}",
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
