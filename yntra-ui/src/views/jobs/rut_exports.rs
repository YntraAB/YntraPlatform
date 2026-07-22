use crate::components;
use crate::locales::t;
use dioxus::prelude::*;
use yntra_core::{
    BankIdAuthSession, SkatteverketSubmitResult,
    initiate_bankid_skatteverket_session, submit_skatteverket_claim_direct,
};

fn trigger_download(toast: &dioxus_primitives::toast::Toasts, locale: &str, content: &str, file_name: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        toast.info(
            t("school-toast-download-started", locale),
            dioxus_primitives::toast::ToastOptions::new().description(t("school-toast-browser-download-desc", locale))
        );

        let base64_str = crate::views::school::academics::utils::base64_encode(content.as_bytes());
        let js_code = format!(
            r#"
            (function() {{
                const base64 = "{}";
                const filename = "{}";
                const binString = atob(base64);
                const bytes = Uint8Array.from(binString, (m) => m.codePointAt(0));
                const blob = new Blob([bytes], {{ type: "application/octet-stream" }});
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
        let file_path = rfd::FileDialog::new()
            .set_file_name(file_name)
            .save_file();
        if let Some(path) = file_path {
            if std::fs::write(&path, content).is_ok() {
                let desc = format!("{} {}", t("school-toast-saved-to", locale), path.display());
                toast.success(
                    t("school-toast-export-success", locale),
                    dioxus_primitives::toast::ToastOptions::new().description(desc)
                );
            } else {
                toast.error(
                    t("school-toast-export-failed", locale),
                    dioxus_primitives::toast::ToastOptions::new().description(t("school-toast-export-failed-desc", locale))
                );
            }
        }
    }
}

#[derive(Props, Clone)]
pub struct RutExportsViewProps {
    pub active_user_id: Signal<String>,
    pub auth_region: Signal<String>,
    pub db_trigger: Signal<u32>,
}

impl PartialEq for RutExportsViewProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn RutExportsView(props: RutExportsViewProps) -> Element {
    let toast = dioxus_primitives::toast::use_toast();
    let region = props.auth_region.read().clone();
    let db_trig = *props.db_trigger.read();
    let state = use_context::<crate::state::AppState>();
    let workspace_opt = state.workspace.read().clone();
    let settings_json: serde_json::Value = if let Some(ref ws) = workspace_opt {
        serde_json::from_str(&ws.settings).unwrap_or_default()
    } else {
        serde_json::Value::Null
    };

    let has_skatteverket_cert = settings_json
        .get("skatteverket_corporate_cert")
        .and_then(|v| v.as_str())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    let rut_invoices_res = use_resource(move || {
        let trig = db_trig;
        let uid = props.active_user_id.read().clone();
        async move {
            let _ = trig;
            yntra_core::get_rut_invoices(uid).await.unwrap_or_default()
        }
    });

    let selected_rut_invoices = use_signal(std::collections::HashSet::<String>::new);
    let list = rut_invoices_res.read().clone().unwrap_or_default();

    let mut show_skatteverket_modal = use_signal(|| false);
    let mut skatteverket_bankid_session = use_signal(|| Option::<BankIdAuthSession>::None);
    let mut skatteverket_submit_result = use_signal(|| Option::<SkatteverketSubmitResult>::None);
    let mut skatteverket_loading = use_signal(|| false);
    let mut bankid_personal_number = use_signal(|| "".to_string());

    rsx! {
        div { class: "mx-auto w-full max-w-5xl p-6 flex flex-col gap-6",
            div { class: "flex flex-col gap-1",
                h1 { class: "text-2xl font-bold text-foreground", "RUT-avdrag & Skatteverket" }
                p { class: "text-sm text-muted-foreground", "Bulkhantering av RUT-avdrag, XML/CSV-export och direkt e-legitimerad inskickning." }
            }

            components::Card {
                class: "w-full p-6 flex flex-col gap-4",
                div { class: "flex justify-between items-center pb-4 border-b border-border",
                    div {
                        h2 { class: "text-lg font-bold", "RUT-avdrag bulkhantering" }
                        p { class: "text-xs text-muted-foreground", "Markera betalda fakturor för att ladda ner XML- eller CSV-underlag för Skatteverket." }
                    }
                    div { class: "flex gap-2",
                        button {
                            class: "py-1.5 px-3 bg-primary text-primary-foreground hover:opacity-90 rounded text-xs font-bold border-0 cursor-pointer flex items-center gap-1.5 transition-all disabled:opacity-50",
                            disabled: selected_rut_invoices.read().is_empty(),
                            onclick: {
                                let uid = props.active_user_id.read().clone();
                                let toast_c = toast.clone();
                                let region_c = region.clone();
                                move |_| {
                                    let uid = uid.clone();
                                    let toast_c = toast_c.clone();
                                    let region_c = region_c.clone();
                                    let ids: Vec<String> = selected_rut_invoices.read().iter().cloned().collect();
                                    spawn(async move {
                                        if let Ok(xml_content) = yntra_core::export_skatteverket_claims(uid, ids, "xml".to_string()).await {
                                            trigger_download(&toast_c, &region_c, &xml_content, "Skatteverket_RUT_Bulk.xml");
                                        }
                                    });
                                }
                            },
                            components::LucideIcon { name: "download", size: "14" }
                            "Exportera XML (Begäran)"
                        }
                        button {
                            class: "py-1.5 px-3 bg-secondary text-secondary-foreground hover:opacity-90 rounded text-xs font-bold border-0 cursor-pointer flex items-center gap-1.5 transition-all disabled:opacity-50",
                            disabled: selected_rut_invoices.read().is_empty(),
                            onclick: {
                                let uid = props.active_user_id.read().clone();
                                let toast_c = toast.clone();
                                let region_c = region.clone();
                                move |_| {
                                    let uid = uid.clone();
                                    let toast_c = toast_c.clone();
                                    let region_c = region_c.clone();
                                    let ids: Vec<String> = selected_rut_invoices.read().iter().cloned().collect();
                                    spawn(async move {
                                        if let Ok(csv_content) = yntra_core::export_skatteverket_claims(uid, ids, "csv".to_string()).await {
                                            trigger_download(&toast_c, &region_c, &csv_content, "Skatteverket_RUT_Bulk.csv");
                                        }
                                    });
                                }
                            },
                            components::LucideIcon { name: "download", size: "14" }
                            "Exportera CSV"
                        }
                        button {
                            class: "py-1.5 px-3 bg-blue-600 text-white hover:opacity-90 rounded text-xs font-bold border-0 cursor-pointer flex items-center gap-1.5 transition-all disabled:opacity-50",
                            disabled: selected_rut_invoices.read().is_empty(),
                            onclick: move |_| {
                                skatteverket_bankid_session.set(None);
                                skatteverket_submit_result.set(None);
                                skatteverket_loading.set(false);
                                show_skatteverket_modal.set(true);
                            },
                            components::LucideIcon { name: "send", size: "14" }
                            "Skicka Direkt"
                        }
                    }
                }

                if list.is_empty() {
                    div { class: "text-center p-8 text-muted-foreground",
                        components::LucideIcon { name: "inbox", size: "32", class: "icon-muted mb-2" }
                        p { class: "text-sm", "Inga betalda fakturor med RUT-avdrag hittades." }
                    }
                } else {
                    table { class: "w-full text-left text-xs border-collapse",
                        thead { class: "bg-muted/30 border-b border-border/50 text-[10px] font-extrabold uppercase tracking-wider text-muted-foreground",
                            tr {
                                th { class: "p-3 w-10 text-center",
                                    input {
                                        type: "checkbox",
                                        checked: selected_rut_invoices.read().len() == list.len(),
                                        onchange: {
                                            let list_clone = list.clone();
                                            move |evt| {
                                                let mut selected = selected_rut_invoices.clone();
                                                if evt.value() == "true" {
                                                    let set: std::collections::HashSet<String> = list_clone.iter().map(|item| item.invoice_id.clone()).collect();
                                                    selected.set(set);
                                                } else {
                                                    selected.set(std::collections::HashSet::new());
                                                }
                                            }
                                        }
                                    }
                                }
                                th { class: "p-3", "Faktura ID" }
                                th { class: "p-3", "Uppdrag" }
                                th { class: "p-3", "Kund" }
                                th { class: "p-3", "Personnummer" }
                                th { class: "p-3", "Betaldatum" }
                                th { class: "p-3 text-right", "RUT Belopp" }
                                th { class: "p-3 text-center", "Status" }
                            }
                        }
                        tbody {
                            for item in list.iter() {
                                {
                                    let is_checked = selected_rut_invoices.read().contains(&item.invoice_id);
                                    let inv_id = item.invoice_id.clone();
                                    rsx! {
                                        tr { key: "{item.invoice_id}", class: "border-b border-border/10 hover:bg-muted/10 transition-colors",
                                            td { class: "p-3 text-center",
                                                input {
                                                    type: "checkbox",
                                                    checked: is_checked,
                                                    onchange: move |evt| {
                                                        let mut selected = selected_rut_invoices.clone();
                                                        let mut set = (*selected.read()).clone();
                                                        if evt.value() == "true" {
                                                            set.insert(inv_id.clone());
                                                        } else {
                                                            set.remove(&inv_id);
                                                        }
                                                        selected.set(set);
                                                    }
                                                }
                                            }
                                            td { class: "p-3 font-semibold font-mono text-[10px]", "{item.invoice_id}" }
                                            td { class: "p-3 font-semibold", "{item.job_title}" }
                                            td { class: "p-3", "{item.customer_name}" }
                                            td { class: "p-3 font-mono text-[11px]", "{item.customer_pnum}" }
                                            td { class: "p-3", "{item.payment_date}" }
                                            td { class: "p-3 text-right font-bold text-emerald-500", "{item.rut_amount} kr" }
                                            td { class: "p-3 text-center",
                                                {
                                                    let (badge_text, badge_style) = match item.status.as_str() {
                                                        "claimed" | "submitted" => ("Inskickad", "bg-blue-500/10 text-blue-500"),
                                                        _ => ("Klar för inskick", "bg-emerald-500/10 text-emerald-500"),
                                                    };
                                                    rsx! {
                                                        span { class: "px-1.5 py-0.5 rounded text-[10px] font-bold {badge_style}",
                                                            "{badge_text}"
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

            // Skatteverket Direct Submission Modal
            if *show_skatteverket_modal.read() {
                div {
                    class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-in fade-in duration-200",
                    div { class: "w-full max-w-lg rounded-2xl border border-border bg-sidebar p-6 shadow-2xl space-y-5",
                        div { class: "flex items-center justify-between border-b border-border/40 pb-3",
                            div { class: "flex items-center gap-2.5",
                                div { class: "rounded-xl bg-blue-500/10 p-2 text-blue-500",
                                    components::LucideIcon { name: "send", class: "h-5 w-5" }
                                }
                                div {
                                    h3 { class: "text-base font-bold text-foreground m-0", "Direkt-inskick till Skatteverket" }
                                    p { class: "text-xs text-muted-foreground m-0", "REST API V3 Direct Gateway (BankID Signering)" }
                                }
                            }
                            button {
                                onclick: move |_| show_skatteverket_modal.set(false),
                                class: "rounded-lg p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground border-0 bg-transparent cursor-pointer transition-all",
                                components::LucideIcon { name: "x", size: "18" }
                            }
                        }

                        if let Some(ref res) = *skatteverket_submit_result.read() {
                            div { class: "space-y-4 text-center py-4",
                                div { class: "inline-flex h-12 w-12 items-center justify-center rounded-full bg-emerald-500/10 text-emerald-500 mb-1",
                                    components::LucideIcon { name: "check-circle", class: "h-6 w-6" }
                                }
                                h4 { class: "text-sm font-bold text-foreground m-0", "Begäran Godkänd av Skatteverket!" }
                                p { class: "text-xs text-muted-foreground max-w-xs mx-auto", "Ärendet har registrerats och kvitterats i Skatteverkets RUT-portal." }
                                div { class: "bg-muted/30 p-3 rounded-xl border border-border/40 text-left font-mono text-[11px] space-y-1.5",
                                    div { class: "flex justify-between", span { class: "text-muted-foreground", "Kvittens-ID:" } span { class: "font-bold text-foreground", "{res.reference_number}" } }
                                    div { class: "flex justify-between", span { class: "text-muted-foreground", "Behandlade poster:" } span { class: "font-bold text-foreground", "{res.total_claims} st" } }
                                    div { class: "flex justify-between", span { class: "text-muted-foreground", "Totalt utbetalt belopp:" } span { class: "font-bold text-emerald-500", "{res.total_amount} kr" } }
                                }
                                button {
                                    onclick: move |_| show_skatteverket_modal.set(false),
                                    class: "w-full rounded-xl bg-primary py-2.5 text-xs font-bold text-primary-foreground border-0 cursor-pointer shadow hover:opacity-90 transition-all",
                                    "Stäng Fönster"
                                }
                            }
                        } else if let Some(ref session) = *skatteverket_bankid_session.read() {
                            div { class: "space-y-4 text-center py-2",
                                p { class: "text-xs text-muted-foreground", "Öppna din BankID-app på din mobila enhet och signera begäran för Skatteverket." }
                                div { class: "flex justify-center py-2",
                                    img {
                                        src: "{session.qr_data}",
                                        class: "h-44 w-44 rounded-xl border-2 border-border shadow-md"
                                    }
                                }
                                div { class: "flex items-center justify-center gap-2 text-xs font-semibold text-blue-500 animate-pulse",
                                    components::LucideIcon { name: "loader-2", class: "h-4 w-4 animate-spin" }
                                    "Väntar på BankID-signering..."
                                }
                                button {
                                    onclick: {
                                        let uid = props.active_user_id.read().clone();
                                        let selected_ids: Vec<String> = selected_rut_invoices.read().iter().cloned().collect();
                                        move |_| {
                                            let uid = uid.clone();
                                            let selected_ids = selected_ids.clone();
                                            skatteverket_loading.set(true);
                                            spawn(async move {
                                                if let Ok(res) = submit_skatteverket_claim_direct(uid, "auth-token-sim".to_string(), selected_ids).await {
                                                    skatteverket_submit_result.set(Some(res));
                                                }
                                                skatteverket_loading.set(false);
                                            });
                                        }
                                    },
                                    class: "w-full rounded-xl bg-emerald-600 py-2.5 text-xs font-bold text-white border-0 cursor-pointer shadow hover:opacity-90 transition-all",
                                    "Simulera BankID Signerad"
                                }
                            }
                        } else {
                            div { class: "space-y-4",
                                div { class: "rounded-xl bg-muted/40 p-3.5 border border-border/40 space-y-2",
                                    div { class: "flex justify-between text-xs",
                                        span { class: "text-muted-foreground", "Valda Fakturor för Inskick:" }
                                        span { class: "font-bold text-foreground", "{selected_rut_invoices.read().len()} st" }
                                    }
                                    div { class: "flex justify-between text-xs",
                                        span { class: "text-muted-foreground", "Autentiseringsmetod:" }
                                        span { class: "font-bold text-blue-500", if has_skatteverket_cert { "Företagscertifikat (.p12)" } else { "BankID E-Legitimation" } }
                                    }
                                }

                                if !has_skatteverket_cert {
                                    div { class: "space-y-1.5",
                                        label { class: "text-xs font-bold text-muted-foreground block", "Firmatecknares Personnummer (YYMMDD-XXXX)" }
                                        input {
                                            r#type: "text",
                                            placeholder: "19850101-1234",
                                            value: "{bankid_personal_number}",
                                            oninput: move |e| bankid_personal_number.set(e.value()),
                                            class: "w-full text-xs p-2.5 rounded-xl border border-border bg-background text-foreground focus:outline-none focus:border-primary transition-all",
                                        }
                                    }
                                }

                                button {
                                    disabled: *skatteverket_loading.read(),
                                    onclick: {
                                        let uid = props.active_user_id.read().clone();
                                        move |_| {
                                            let uid = uid.clone();
                                            skatteverket_loading.set(true);
                                            spawn(async move {
                                                if let Ok(sess) = initiate_bankid_skatteverket_session(uid).await {
                                                    skatteverket_bankid_session.set(Some(sess));
                                                }
                                                skatteverket_loading.set(false);
                                            });
                                        }
                                    },
                                    class: "w-full rounded-xl bg-blue-600 py-3 text-xs font-bold text-white shadow-md hover:bg-blue-700 transition-all border-0 cursor-pointer flex items-center justify-center gap-2 disabled:opacity-50",
                                    if *skatteverket_loading.read() {
                                        components::LucideIcon { name: "loader-2", class: "h-4 w-4 animate-spin" }
                                        span { "Ansluter Skatteverket Direct..." }
                                    } else {
                                        components::LucideIcon { name: "shield-check", class: "h-4 w-4" }
                                        span { "Initiera BankID Signering" }
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
