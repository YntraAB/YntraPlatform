use dioxus::prelude::*;
use yntra_core::{is_mdm_update_disabled, process_update_manifest};

#[component]
pub fn AutoUpdateToast() -> Element {
    let mut show_banner = use_signal(|| false);
    let mut update_version = use_signal(|| String::new());
    let mut release_notes = use_signal(|| String::new());
    let mut is_downloading = use_signal(|| false);
    let mut download_progress = use_signal(|| 0u32);
    let mut is_ready_to_restart = use_signal(|| false);
    let is_mdm_managed = is_mdm_update_disabled();

    // Initial check on mount
    use_effect(move || {
        let current_version = env!("CARGO_PKG_VERSION");
        // Simulated background update check
        let sample_manifest_json = r#"{
            "version": "0.2.0",
            "release_notes": "Added cross-platform native installers, EV code signing, and automatic delta updates.",
            "pub_date": "2026-08-04T12:00:00Z",
            "download_url": "https://releases.yntra.se/v0.2.0/yntra-ui.exe",
            "signature": "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "min_supported_version": "0.1.0"
        }"#;

        if let Ok(result) = process_update_manifest(sample_manifest_json, current_version, None) {
            if result.update_available {
                if let Some(m) = result.manifest {
                    update_version.set(m.version);
                    release_notes.set(m.release_notes);
                    show_banner.set(true);
                }
            }
        }
    });

    let handle_download = move |_| {
        is_downloading.set(true);
        // Simulate background chunk download & Ed25519 signature verification
        spawn(async move {
            for progress in (10..=100).step_by(30) {
                download_progress.set(progress);
                yntra_core::infra::time::sleep_ms(300).await;
            }
            is_downloading.set(false);
            is_ready_to_restart.set(true);
        });
    };

    let handle_restart = move |_| {
        // Trigger application restart / replacement process
        show_banner.set(false);
    };

    if !show_banner() {
        return rsx! {};
    }

    rsx! {
        div {
            class: "fixed bottom-6 right-6 z-50 max-w-md w-full bg-slate-900/95 backdrop-blur-md text-white border border-cyan-500/30 rounded-xl p-5 shadow-2xl transition-all duration-300 animate-in fade-in slide-in-from-bottom-5",
            div {
                class: "flex items-start justify-between gap-3",
                div {
                    class: "flex items-center gap-3",
                    div {
                        class: "p-2 rounded-lg bg-cyan-500/20 text-cyan-400 border border-cyan-500/30",
                        svg {
                            class: "w-5 h-5",
                            fill: "none",
                            stroke: "currentColor",
                            view_box: "0 0 24 24",
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                stroke_width: "2",
                                d: "M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
                            }
                        }
                    }
                    div {
                        h4 { class: "text-sm font-semibold text-white", "Yntra Platform v{update_version} Available" }
                        p { class: "text-xs text-slate-400 mt-0.5", "{release_notes}" }
                    }
                }
                button {
                    class: "text-slate-400 hover:text-white transition-colors p-1",
                    onclick: move |_| show_banner.set(false),
                    "✕"
                }
            }

            if is_downloading() {
                div {
                    class: "mt-4 space-y-1.5",
                    div { class: "flex justify-between text-xs text-slate-300",
                        span { "Downloading & verifying Ed25519 signature..." }
                        span { "{download_progress}%" }
                    }
                    div { class: "w-full bg-slate-800 rounded-full h-1.5 overflow-hidden border border-slate-700",
                        div {
                            class: "bg-cyan-500 h-full transition-all duration-200 rounded-full",
                            style: "width: {download_progress}%"
                        }
                    }
                }
            } else if is_ready_to_restart() {
                div {
                    class: "mt-4 flex items-center justify-end gap-2",
                    button {
                        class: "px-3 py-1.5 text-xs font-medium bg-gradient-to-r from-emerald-500 to-teal-600 hover:from-emerald-400 hover:to-teal-500 text-white rounded-lg transition-all shadow-md shadow-emerald-900/30 flex items-center gap-1.5",
                        onclick: handle_restart,
                        "Restart & Install Update"
                    }
                }
            } else if is_mdm_managed {
                div {
                    class: "mt-4 flex items-center justify-between gap-2 border-t border-slate-800 pt-3",
                    span { class: "text-[11px] text-amber-400 font-semibold flex items-center gap-1",
                        "🛡️ Managed by Enterprise MDM (Updates deployed by IT)"
                    }
                    button {
                        class: "px-3 py-1.5 text-xs font-medium text-slate-300 hover:text-white hover:bg-slate-800 rounded-lg transition-colors",
                        onclick: move |_| show_banner.set(false),
                        "Dismiss"
                    }
                }
            } else {
                div {
                    class: "mt-4 flex items-center justify-end gap-2",
                    button {
                        class: "px-3 py-1.5 text-xs font-medium text-slate-300 hover:text-white hover:bg-slate-800 rounded-lg transition-colors",
                        onclick: move |_| show_banner.set(false),
                        "Remind Me Later"
                    }
                    button {
                        class: "px-3 py-1.5 text-xs font-medium bg-cyan-600 hover:bg-cyan-500 text-white rounded-lg transition-all shadow-md shadow-cyan-900/30 flex items-center gap-1.5",
                        onclick: handle_download,
                        "Download Update"
                    }
                }
            }
        }
    }
}
