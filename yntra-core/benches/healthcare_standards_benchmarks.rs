use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use yntra_core::infra::fhir_r4::{FhirPatient, FhirHumanName, FhirIdentifier, FhirAddress};
use yntra_core::infra::hl7_v2::{encode_mllp_frame, decode_mllp_frames, parse_hl7_v2_message};
use yntra_core::infra::ncpdp_script::{validate_ncpdp_script_payload, validate_dea_number};

fn bench_fhir_r4_patient_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("HL7 FHIR R4 Patient Serialization");

    let patient = FhirPatient {
        resource_type: "Patient".to_string(),
        id: "p-987654321".to_string(),
        active: true,
        identifier: vec![FhirIdentifier {
            use_type: Some("official".to_string()),
            system: Some("urn:oid:2.16.840.1.113883.4.1".to_string()),
            value: "999-00-1234".to_string(),
        }],
        name: vec![FhirHumanName {
            use_type: Some("official".to_string()),
            text: Some("Dr. Eleanor Vance, MD".to_string()),
            family: Some("Vance".to_string()),
            given: vec!["Eleanor".to_string()],
            prefix: vec!["Dr.".to_string()],
            suffix: vec!["MD".to_string()],
        }],
        gender: Some("female".to_string()),
        birth_date: Some("1985-04-12".to_string()),
        address: vec![FhirAddress {
            use_type: Some("home".to_string()),
            line: vec!["742 Evergreen Terrace".to_string()],
            city: Some("Springfield".to_string()),
            state: Some("OR".to_string()),
            postal_code: Some("97477".to_string()),
            country: Some("USA".to_string()),
        }],
        telecom: vec![],
    };

    let serialized = serde_json::to_string(&patient).unwrap();

    group.throughput(Throughput::Bytes(serialized.len() as u64));

    group.bench_function("serialize_fhir_patient_json", |b| {
        b.iter(|| {
            let res = serde_json::to_string(black_box(&patient)).unwrap();
            black_box(res);
        });
    });

    group.bench_function("deserialize_fhir_patient_json", |b| {
        b.iter(|| {
            let res: FhirPatient = serde_json::from_str(black_box(&serialized)).unwrap();
            black_box(res);
        });
    });

    group.finish();
}

fn bench_hl7_v2_message_parsing_and_mllp(c: &mut Criterion) {
    let mut group = c.benchmark_group("HL7 v2.x & MLLP Protocol Framing");

    let hl7_msg = "MSH|^~\\&|EPIC|HOSPITAL|YNTRA|CLINIC|20260806223000||ADT^A01^ADT_A01|MSG100001|P|2.5.1\rPID|1||P-10101^^^MRN||SMITH^JOHN^A||19800101|M|||123 MAIN ST^^BOSTON^MA^02115\rPV1|1|I|3W^301^1";

    group.throughput(Throughput::Bytes(hl7_msg.len() as u64));

    group.bench_function("parse_hl7_v2_message_segments", |b| {
        b.iter(|| {
            let res = parse_hl7_v2_message(black_box(hl7_msg));
            black_box(res);
        });
    });

    group.bench_function("encode_mllp_frame", |b| {
        b.iter(|| {
            let res = encode_mllp_frame(black_box(hl7_msg));
            black_box(res);
        });
    });

    let framed = encode_mllp_frame(hl7_msg);
    group.bench_function("decode_mllp_frames", |b| {
        b.iter(|| {
            let mut buf = framed.clone();
            let res = decode_mllp_frames(black_box(&mut buf));
            black_box(res);
        });
    });

    group.finish();
}

fn bench_ncpdp_script_e_prescribing(c: &mut Criterion) {
    let mut group = c.benchmark_group("NCPDP SCRIPT e-Prescribing Engine");

    group.bench_function("validate_ncpdp_script_payload", |b| {
        b.iter(|| {
            let res = validate_ncpdp_script_payload(
                black_box("1234567890"),
                black_box("0987654321"),
                black_box("Amoxicillin 500mg Oral Capsule"),
                black_box(30.0),
                black_box(10),
            );
            black_box(res).unwrap();
        });
    });

    group.bench_function("validate_dea_number", |b| {
        b.iter(|| {
            let res = validate_dea_number(black_box("AB1234567"));
            black_box(res).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_fhir_r4_patient_serialization,
    bench_hl7_v2_message_parsing_and_mllp,
    bench_ncpdp_script_e_prescribing
);
criterion_main!(benches);
