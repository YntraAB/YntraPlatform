use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use yntra_core::services::dynamic_entities::{
    filter_entity_data_for_role, validate_entity_data_write_permissions, FieldPermissionPolicy,
};

fn bench_dynamic_entity_rbac_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("Dynamic Entity RBAC Attribute Filtering");

    let payload_json = r#"{
        "patient_id": "p-100",
        "ssn": "000-12-3456",
        "first_name": "Alexander",
        "last_name": "Fleming",
        "diagnosis_code": "ICD10-J20.9",
        "billing_address": "123 Medical Center Way",
        "credit_card_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "blood_type": "O-Positive",
        "attending_physician": "Dr. Sarah Connor"
    }"#;

    let policies = vec![
        FieldPermissionPolicy {
            role: "nurse".to_string(),
            read_allowed_fields: vec![
                "patient_id".to_string(),
                "first_name".to_string(),
                "last_name".to_string(),
                "diagnosis_code".to_string(),
                "blood_type".to_string(),
                "attending_physician".to_string(),
            ],
            write_allowed_fields: vec![
                "diagnosis_code".to_string(),
                "blood_type".to_string(),
            ],
        },
        FieldPermissionPolicy {
            role: "billing_clerk".to_string(),
            read_allowed_fields: vec![
                "patient_id".to_string(),
                "first_name".to_string(),
                "last_name".to_string(),
                "billing_address".to_string(),
            ],
            write_allowed_fields: vec![
                "billing_address".to_string(),
            ],
        },
    ];

    group.throughput(Throughput::Bytes(payload_json.len() as u64));

    group.bench_function("filter_entity_data_nurse_role", |b| {
        b.iter(|| {
            let res = filter_entity_data_for_role(
                black_box(payload_json),
                black_box("nurse"),
                black_box(&policies),
            );
            black_box(res);
        });
    });

    group.bench_function("filter_entity_data_billing_clerk_role", |b| {
        b.iter(|| {
            let res = filter_entity_data_for_role(
                black_box(payload_json),
                black_box("billing_clerk"),
                black_box(&policies),
            );
            black_box(res);
        });
    });

    group.bench_function("validate_entity_data_write_nurse_allowed", |b| {
        let nurse_write = r#"{"diagnosis_code": "ICD10-J20.9", "blood_type": "O-Positive"}"#;
        b.iter(|| {
            let res = validate_entity_data_write_permissions(
                black_box(nurse_write),
                black_box("nurse"),
                black_box(&policies),
            );
            black_box(res).unwrap();
        });
    });

    group.finish();
}

fn bench_csv_ingestion_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Bulk CSV Data Ingestion & Delimiter Detection");

    let mut csv_data = String::from("id,name,role,department,salary_grade,status,joined_date\n");
    for i in 0..500 {
        csv_data.push_str(&format!("usr-{},Employee {},Engineer,Software Dev,Grade-{},Active,2026-01-15\n", i, i, i % 5));
    }

    group.throughput(Throughput::Bytes(csv_data.len() as u64));

    group.bench_function("parse_csv_lines_iteration", |b| {
        b.iter(|| {
            let count = black_box(&csv_data).lines().count();
            black_box(count);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_dynamic_entity_rbac_filtering,
    bench_csv_ingestion_parsing
);
criterion_main!(benches);
