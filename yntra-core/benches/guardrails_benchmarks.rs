use criterion::{black_box, criterion_group, criterion_main, Criterion};
use yntra_core::services::semantic_guardrails::{
    ResolutionPolicyConfig, IntentLeaseRecord, SemanticConflictRecord,
};
use yntra_core::infra::compliance::ComplianceRegistry;

fn bench_semantic_guardrail_structures(c: &mut Criterion) {
    let mut group = c.benchmark_group("Semantic Guardrails & Compliance Rules");

    group.bench_function("labor_rule_lookup_us", |b| {
        b.iter(|| {
            let rule = ComplianceRegistry::get_rule(black_box("US"));
            black_box(rule);
        });
    });

    group.bench_function("labor_rule_lookup_de", |b| {
        b.iter(|| {
            let rule = ComplianceRegistry::get_rule(black_box("DE"));
            black_box(rule);
        });
    });

    let policy = ResolutionPolicyConfig {
        workspace_id: "ws-test-123".to_string(),
        default_strategy: "higher_role_wins".to_string(),
        auto_resolve_high_severity: true,
    };

    group.bench_function("resolution_policy_clone", |b| {
        b.iter(|| {
            let cloned = black_box(&policy).clone();
            black_box(cloned);
        });
    });

    let lease = IntentLeaseRecord {
        id: "lease-001".to_string(),
        workspace_id: "ws-test-123".to_string(),
        entity_table: "care_plans".to_string(),
        entity_id: "entity-888".to_string(),
        user_id: "user-456".to_string(),
        user_name: "Dr. Alice".to_string(),
        expires_at_ms: 1800000000000,
    };

    group.bench_function("intent_lease_clone", |b| {
        b.iter(|| {
            let cloned = black_box(&lease).clone();
            black_box(cloned);
        });
    });

    let conflict = SemanticConflictRecord {
        id: "conflict-777".to_string(),
        workspace_id: "ws-test-123".to_string(),
        domain: "domain-care".to_string(),
        entity_table: "prescriptions".to_string(),
        entity_id: "rx-999".to_string(),
        colliding_entity_id: Some("rx-1000".to_string()),
        conflict_type: "DUPLICATE_DOSAGE_TIME_SLOT".to_string(),
        severity: "CRITICAL".to_string(),
        conflict_details_json: r#"{"reason": "Dosage collision detected between 14:00 and 14:30"}"#.to_string(),
        status: "OPEN".to_string(),
        created_at: 1770000000,
        updated_at: 1770000000,
    };

    group.bench_function("semantic_conflict_record_clone", |b| {
        b.iter(|| {
            let cloned = black_box(&conflict).clone();
            black_box(cloned);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_semantic_guardrail_structures);
criterion_main!(benches);
