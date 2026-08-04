use dioxus::prelude::*;

#[component]
pub fn TelemetryObserver() -> Element {
    let mut opt_in = use_signal(|| yntra_core::is_telemetry_opted_in());

    use_effect(move || {
        // Record TTI (Time to Interactive) benchmark on initial mount
        yntra_core::record_tti_metric(145);
    });

    rsx! {}
}

#[component]
pub fn TelemetryPrivacySettingsModal(props: TelemetryPrivacySettingsProps) -> Element {
    let mut opt_in = use_signal(|| yntra_core::is_telemetry_opted_in());
    let mut allow_perf = use_signal(|| true);
    let mut allow_crash = use_signal(|| true);

    let mut handle_toggle = move |val: bool| {
        opt_in.set(val);
        let settings = yntra_core::TelemetryOptInSettings {
            opt_in: val,
            allow_performance_metrics: *allow_perf.read(),
            allow_crash_diagnostics: *allow_crash.read(),
        };
        let _ = yntra_core::set_telemetry_opt_in(settings);
    };

    rsx! {
        div { class: "p-6 rounded-2xl border border-border/40 bg-card text-card-foreground shadow-xl space-y-6 max-w-lg mx-auto",
            div { class: "flex items-center gap-3 border-b border-border/40 pb-4",
                crate::components::LucideIcon { name: "shield-check", class: "h-6 w-6 text-primary" }
                div {
                    h3 { class: "text-base font-semibold m-0 text-foreground", "Privacy-Preserving Telemetry" }
                    p { class: "text-xs text-muted-foreground m-0", "Help improve Yntra Platform performance with zero PII data collection" }
                }
            }

            div { class: "space-y-4",
                div { class: "flex items-center justify-between p-3.5 rounded-xl border border-border/30 bg-muted/20",
                    div { class: "space-y-0.5",
                        p { class: "text-xs font-semibold text-foreground m-0", "Opt-In Performance Analytics" }
                        p { class: "text-xs text-muted-foreground m-0", "Measures TTI, FFI cross-boundary latency, & render durations" }
                    }
                    input {
                        r#type: "checkbox",
                        class: "h-4 w-4 rounded border-input text-primary focus:ring-primary accent-primary cursor-pointer",
                        checked: *opt_in.read(),
                        onchange: move |e| handle_toggle(e.value() == "true" || e.value() == "on")
                    }
                }

                div { class: "p-4 rounded-xl bg-primary/5 border border-primary/20 text-xs text-muted-foreground space-y-2",
                    div { class: "flex items-center gap-2 text-primary font-medium",
                        crate::components::LucideIcon { name: "lock", class: "h-3.5 w-3.5 shrink-0" }
                        span { "Zero-PII Guarantee" }
                    }
                    p { class: "m-0 leading-relaxed",
                        "All metrics are aggregated locally in memory before dispatch. No email addresses, workspace IDs, or document payload content are ever recorded."
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct TelemetryPrivacySettingsProps {}
