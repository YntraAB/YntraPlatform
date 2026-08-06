use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use loro::LoroDoc;
use yntra_core::infra::hl7_v2::parse_hl7_v2_message;
use yntra_core::infra::ncpdp_script::parse_ncpdp_script_xml;
use yntra_core::infra::fhir_r4::validate_fhir_r4_payload;
use std::borrow::Cow;

fn bench_loro_crdt_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Loro CRDT State Synchronization");

    group.bench_function("loro_doc_create_and_text_insert", |b| {
        b.iter(|| {
            let doc = LoroDoc::new();
            let text = doc.get_text("workspace_notes");
            text.insert(0, black_box("Patient presented with mild fever. Recommended rest and hydration.")).unwrap();
            black_box(doc);
        });
    });

    let doc_a = LoroDoc::new();
    let text_a = doc_a.get_text("shared_doc");
    text_a.insert(0, "Initial document state from Node A.").unwrap();
    let snapshot_bytes = doc_a.export(loro::ExportMode::Snapshot).unwrap();

    group.throughput(Throughput::Bytes(snapshot_bytes.len() as u64));

    group.bench_function("loro_doc_import_snapshot", |b| {
        b.iter(|| {
            let doc_b = LoroDoc::new();
            doc_b.import(black_box(&snapshot_bytes)).unwrap();
            black_box(doc_b);
        });
    });

    let doc_b = LoroDoc::new();
    doc_b.import(&snapshot_bytes).unwrap();
    let text_b = doc_b.get_text("shared_doc");
    text_b.insert(text_b.to_string().len(), " Concurrent update from Node B.").unwrap();

    let update_b = doc_b.export(loro::ExportMode::Updates { from: Cow::Owned(doc_a.oplog_vv()) }).unwrap();

    group.bench_function("loro_doc_merge_concurrent_updates", |b| {
        b.iter(|| {
            let doc_clone = LoroDoc::new();
            doc_clone.import(&snapshot_bytes).unwrap();
            doc_clone.import(black_box(&update_b)).unwrap();
            black_box(doc_clone);
        });
    });

    group.finish();
}

fn bench_standards_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Healthcare & Financial Standards Parser");

    let hl7_msg = "MSH|^~\\&|EPIC|HOSPITAL|YNTRA|PLATFORM|20260806220000||ADT^A01^ADT_A01|MSG10001|P|2.5\rPID|1||PAT001^^^MRN||DOE^JANE||19850515|F|||123 MAIN ST^^SPRINGFIELD^IL^62701\r";
    group.throughput(Throughput::Bytes(hl7_msg.len() as u64));

    group.bench_function("hl7_v2_parse_er7", |b| {
        b.iter(|| {
            let res = parse_hl7_v2_message(black_box(hl7_msg));
            black_box(res).unwrap();
        });
    });

    let fhir_json = r#"{
        "resourceType": "Patient",
        "id": "pat-example-001",
        "active": true,
        "name": [{"family": "Smith", "given": ["John"]}]
    }"#;
    group.bench_function("fhir_r4_validate_json", |b| {
        b.iter(|| {
            let res = validate_fhir_r4_payload(black_box(fhir_json));
            black_box(res).unwrap();
        });
    });

    let ncpdp_xml = r#"<Message><Header><To>Pharmacy</To><From>Yntra</From></Header><Body><RxChangeRequest><PrescriptionNumber>998877</PrescriptionNumber></RxChangeRequest></Body></Message>"#;
    group.bench_function("ncpdp_script_parse_xml", |b| {
        b.iter(|| {
            let res = parse_ncpdp_script_xml(black_box(ncpdp_xml));
            black_box(res).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_loro_crdt_operations, bench_standards_parsing);
criterion_main!(benches);
