use criterion::{black_box, criterion_group, criterion_main, Criterion};
use yntra_core::services::dynamic_entities::{
    filter_entity_data_for_role, FieldPermissionPolicy,
};
use yntra_core::services::siem::{format_siem_event_cef, format_siem_event_syslog_rfc5424};
use yntra_core::services::time_integrity::TimeIntegrityEngine;

fn bench_services_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Domain Services & Entity Processing");

    let json_data = r#"{
        "patient_id": "P-999",
        "ssn": "000-12-3456",
        "diagnosis": "Acute Bronchitis",
        "notes": "Patient advised rest",
        "billing_code": "ICD10-J20.9",
        "credit_card": "4111-2222-3333-4444"
    }"#;

    let policies = vec![FieldPermissionPolicy {
        role: "FIELD_WORKER".to_string(),
        read_allowed_fields: vec!["patient_id".to_string(), "notes".to_string()],
        write_allowed_fields: vec!["notes".to_string()],
    }];

    group.bench_function("filter_entity_data_for_admin_role", |b| {
        b.iter(|| {
            let filtered = filter_entity_data_for_role(
                black_box(json_data),
                black_box("ADMIN"),
                black_box(&policies),
            );
            black_box(filtered);
        });
    });

    group.bench_function("filter_entity_data_for_field_worker_role", |b| {
        b.iter(|| {
            let filtered = filter_entity_data_for_role(
                black_box(json_data),
                black_box("FIELD_WORKER"),
                black_box(&policies),
            );
            black_box(filtered);
        });
    });

    group.finish();
}

fn bench_siem_logging_formatters(c: &mut Criterion) {
    let mut group = c.benchmark_group("SIEM Compliance Log Streaming Formatters");

    group.bench_function("format_siem_event_cef", |b| {
        b.iter(|| {
            let cef = format_siem_event_cef(
                black_box("ws_care_hospital".to_string()),
                black_box("user_dr_smith".to_string()),
                black_box("READ_PHI_RECORD".to_string()),
                black_box("HIGH".to_string()),
                black_box("Dr. Smith accessed patient PHI record for Emergency treatment".to_string()),
            );
            black_box(cef);
        });
    });

    group.bench_function("format_siem_event_syslog_rfc5424", |b| {
        b.iter(|| {
            let syslog = format_siem_event_syslog_rfc5424(
                black_box("ws_care_hospital".to_string()),
                black_box("user_dr_smith".to_string()),
                black_box("READ_PHI_RECORD".to_string()),
                black_box("HIGH".to_string()),
                black_box("Dr. Smith accessed patient PHI record for Emergency treatment".to_string()),
            );
            black_box(syslog);
        });
    });

    group.finish();
}

fn bench_time_integrity_engine(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vector Clock & Time Integrity Monotonicity");

    group.bench_function("verify_timestamp_integrity_valid", |b| {
        b.iter(|| {
            let record = TimeIntegrityEngine::verify_timestamp_integrity(
                black_box("device_mobile_android_001"),
                black_box(1770000100),
                black_box(1770000000),
                black_box(42),
            );
            black_box(record);
        });
    });

    group.bench_function("verify_timestamp_integrity_backward_skew", |b| {
        b.iter(|| {
            let record = TimeIntegrityEngine::verify_timestamp_integrity(
                black_box("device_mobile_android_001"),
                black_box(1769999000), // skewed 1000ms backward
                black_box(1770000000),
                black_box(42),
            );
            black_box(record);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_services_processing,
    bench_siem_logging_formatters,
    bench_time_integrity_engine
);
criterion_main!(benches);
