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
    rsx! {
        div { class: "grid-4",
            div { class: "dashboard-card border-l-4 border-emerald-500",
                div { class: "card-title", "Attested Approved Hours" }
                div { class: "card-value", "{props.approved_hrs} hrs" }
                div { class: "card-sub", "Attested shifts logged locally" }
            }
            div { class: "dashboard-card border-l-4 border-amber-500",
                div { class: "card-title", "Awaiting Attestation" }
                div { class: "card-value", "{props.pending_hrs} hrs" }
                div { class: "card-sub", "{props.pending_cnt} pending time records" }
            }
            div { class: "dashboard-card border-l-4 border-red-500",
                div { class: "card-title", "Rejected/Disputed Hours" }
                div { class: "card-value", "{props.rejected_hrs} hrs" }
                div { class: "card-sub", "Requires employee revision" }
            }
            div { class: "dashboard-card",
                div { class: "card-title", "Resolution Rate" }
                div { class: "card-value", "{props.resolution_rate}" }
                div { class: "card-sub", "Resolved vs open records" }
            }
        }
    }
}
