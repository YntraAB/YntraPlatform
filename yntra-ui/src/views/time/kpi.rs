use dioxus::prelude::*;

#[derive(Props, Clone)]
pub struct KpiSummaryProps {
    pub approved_hrs: f64,
    pub pending_hrs: f64,
    pub pending_cnt: usize,
    pub rejected_hrs: f64,
    pub resolution_rate: String,
}

impl PartialEq for KpiSummaryProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn KpiSummary(props: KpiSummaryProps) -> Element {
    let state = use_context::<crate::state::AppState>();
    let region = state.auth_region.read();

    rsx! {
        div { class: "grid-4",
            div { class: "dashboard-card border-l-4 border-emerald-500",
                div { class: "card-title", "{crate::locales::t(\"time-kpi-attested-approved-hours\", &region)}" }
                div { class: "card-value", "{props.approved_hrs} hrs" }
                div { class: "card-sub", "{crate::locales::t(\"time-kpi-attested-shifts-logged-locally\", &region)}" }
            }
            div { class: "dashboard-card border-l-4 border-amber-500",
                div { class: "card-title", "{crate::locales::t(\"time-kpi-awaiting-attestation\", &region)}" }
                div { class: "card-value", "{props.pending_hrs} hrs" }
                div { class: "card-sub", "{props.pending_cnt} pending time records" }
            }
            div { class: "dashboard-card border-l-4 border-red-500",
                div { class: "card-title", "Rejected/Disputed Hours" }
                div { class: "card-value", "{props.rejected_hrs} hrs" }
                div { class: "card-sub", "{crate::locales::t(\"time-kpi-requires-employee-revision\", &region)}" }
            }
            div { class: "dashboard-card",
                div { class: "card-title", "{crate::locales::t(\"time-kpi-resolution-rate\", &region)}" }
                div { class: "card-value", "{props.resolution_rate}" }
                div { class: "card-sub", "{crate::locales::t(\"time-kpi-resolved-vs-open-records\", &region)}" }
            }
        }
    }
}
