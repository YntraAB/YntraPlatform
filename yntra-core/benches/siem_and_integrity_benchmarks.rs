use criterion::{black_box, criterion_group, criterion_main, Criterion};
use yntra_core::services::siem::{format_siem_event_cef, format_siem_event_syslog_rfc5424};
use yntra_core::infra::observer::{
    reconcile_foreground_state, get_current_state_sequence_number, notify_observers,
};

fn bench_siem_log_formatting(c: &mut Criterion) {
    let mut group = c.benchmark_group("SIEM Event Log Formatting Throughput");

    let workspace_id = "ws-sec-audit-999".to_string();
    let actor_id = "usr-admin-sec-01".to_string();
    let action_type = "PATIENT_RECORD_PHI_EXPORT".to_string();
    let severity = "HIGH".to_string();
    let description = "User exported 150 patient records to encrypted CSV file".to_string();

    group.bench_function("format_siem_event_cef", |b| {
        b.iter(|| {
            let res = format_siem_event_cef(
                black_box(workspace_id.clone()),
                black_box(actor_id.clone()),
                black_box(action_type.clone()),
                black_box(severity.clone()),
                black_box(description.clone()),
            );
            black_box(res);
        });
    });

    group.bench_function("format_siem_event_syslog_rfc5424", |b| {
        b.iter(|| {
            let res = format_siem_event_syslog_rfc5424(
                black_box(workspace_id.clone()),
                black_box(actor_id.clone()),
                black_box(action_type.clone()),
                black_box(severity.clone()),
                black_box(description.clone()),
            );
            black_box(res);
        });
    });

    group.finish();
}

fn bench_observer_notification_and_reconciliation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Reactive Database Observer & State Sequence");

    group.bench_function("get_current_state_sequence_number", |b| {
        b.iter(|| {
            let seq = get_current_state_sequence_number();
            black_box(seq);
        });
    });

    group.bench_function("reconcile_foreground_state", |b| {
        b.iter(|| {
            let report = reconcile_foreground_state(black_box(42));
            black_box(report);
        });
    });

    group.bench_function("notify_observers_table_event", |b| {
        b.iter(|| {
            notify_observers();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_siem_log_formatting,
    bench_observer_notification_and_reconciliation
);
criterion_main!(benches);
