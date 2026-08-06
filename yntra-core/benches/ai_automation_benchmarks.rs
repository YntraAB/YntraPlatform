use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use yntra_core::services::ai_automation::{
    evaluate_ai_guardrails, mask_api_key, parse_voice_report_to_proposal,
    parse_voice_transcript_to_report, WorkspaceAiConfig,
};

fn bench_ai_guardrails_and_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("AI Automation & Guardrails Processing");
    group.throughput(Throughput::Elements(1));

    let api_key = "sk-proj-yntra-platform-secret-api-key-1234567890abcdefghijklmnopqrstuvwxyz";
    group.bench_function("mask_api_key", |b| {
        b.iter(|| {
            let masked = mask_api_key(black_box(api_key));
            black_box(masked);
        });
    });

    let config = WorkspaceAiConfig {
        provider: "openai".to_string(),
        api_key_masked: "sk-proj-****".to_string(),
        model_name: "gpt-4o".to_string(),
        guardrails_enabled: true,
        max_allowed_risk: "MEDIUM".to_string(),
        require_human_approval_above_hours: 8.0,
        min_auto_approve_confidence: 0.85,
    };

    group.bench_function("evaluate_ai_guardrails_clean", |b| {
        b.iter(|| {
            let result = evaluate_ai_guardrails(
                black_box(config.clone()),
                black_box("CREATE_TIME_REPORT".to_string()),
                black_box("{\"hours\": 4.0}".to_string()),
                black_box(4.0),
                black_box(0.95),
            );
            black_box(result);
        });
    });

    group.bench_function("evaluate_ai_guardrails_high_risk", |b| {
        b.iter(|| {
            let result = evaluate_ai_guardrails(
                black_box(config.clone()),
                black_box("OVERTIME_AUTO_APPROVAL".to_string()),
                black_box("{\"hours\": 14.0}".to_string()),
                black_box(14.0),
                black_box(0.60),
            );
            black_box(result);
        });
    });

    let voice_transcript = "Client arrived at nine AM. Performed full inspection of the HVAC unit. Arbetade fem timmar med reparation.";
    group.bench_function("parse_voice_transcript_to_report", |b| {
        b.iter(|| {
            let report = parse_voice_transcript_to_report(
                black_box(voice_transcript),
                black_box(Some("2026-08-06")),
            );
            black_box(report);
        });
    });

    group.bench_function("parse_voice_report_to_proposal", |b| {
        b.iter(|| {
            let proposal = parse_voice_report_to_proposal(
                black_box(voice_transcript.to_string()),
                black_box(Some("2026-08-06".to_string())),
            );
            black_box(proposal);
        });
    });

    group.finish();
}

fn bench_ai_guardrails_length_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("AI Guardrails Prompt Length Scaling");

    let config = WorkspaceAiConfig {
        provider: "anthropic".to_string(),
        api_key_masked: "sk-ant-****".to_string(),
        model_name: "claude-3-5-sonnet".to_string(),
        guardrails_enabled: true,
        max_allowed_risk: "LOW".to_string(),
        require_human_approval_above_hours: 6.0,
        min_auto_approve_confidence: 0.90,
    };

    for length in [100, 1000, 10000].iter() {
        let condition_params = "{\"data\": \"".to_string() + &"x".repeat(*length) + "\"}";
        group.throughput(Throughput::Bytes(condition_params.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("evaluate_ai_guardrails", length),
            length,
            |b, _| {
                b.iter(|| {
                    let res = evaluate_ai_guardrails(
                        black_box(config.clone()),
                        black_box("EVAL_SCENARIO".to_string()),
                        black_box(condition_params.clone()),
                        black_box(5.0),
                        black_box(0.92),
                    );
                    black_box(res);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_ai_guardrails_and_parsing,
    bench_ai_guardrails_length_scaling
);
criterion_main!(benches);
