use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dioxus::prelude::*;

#[component]
fn DashboardCard(title: String, status: String, count: u32) -> Element {
    rsx! {
        div { class: "p-4 bg-slate-900 rounded-lg shadow border border-slate-800",
            h3 { class: "text-lg font-bold text-slate-100", "{title}" }
            div { class: "mt-2 flex items-center justify-between",
                span { class: "text-sm text-emerald-400 font-medium", "{status}" }
                span { class: "text-2xl font-semibold text-white", "{count}" }
            }
        }
    }
}

#[component]
fn BenchmarkApp() -> Element {
    rsx! {
        main { class: "min-h-screen bg-slate-950 p-6 font-sans text-slate-200",
            header { class: "mb-6 flex items-center justify-between border-b border-slate-800 pb-4",
                h1 { class: "text-2xl font-bold tracking-tight text-white", "Yntra Platform Control Hub" }
                span { class: "px-2.5 py-1 text-xs font-semibold bg-emerald-500/10 text-emerald-400 rounded-full border border-emerald-500/20",
                    "Local Database Online"
                }
            }
            div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                DashboardCard { title: "Active Workspaces".to_string(), status: "Synchronized".to_string(), count: 12 }
                DashboardCard { title: "Encrypted Encapsulations".to_string(), status: "AES-256-GCM Active".to_string(), count: 1420 }
                DashboardCard { title: "Peer Mesh Connections".to_string(), status: "WebRTC P2P Direct".to_string(), count: 4 }
            }
        }
    }
}

fn bench_dioxus_ssr_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("Dioxus 0.7 SSR Virtual DOM Engine");

    group.bench_function("render_dashboard_app_to_html", |b| {
        b.iter(|| {
            let html = dioxus_ssr::render_element(rsx! {
                BenchmarkApp {}
            });
            black_box(html);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_dioxus_ssr_rendering);
criterion_main!(benches);
