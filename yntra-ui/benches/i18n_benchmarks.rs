use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

fn bench_fluent_i18n_bundle(c: &mut Criterion) {
    let mut group = c.benchmark_group("Fluent i18n Localization Engine");

    let ftl_source = r#"
welcome-title = Welcome back, { $username }!
sync-status-online = Local database synchronized with peer mesh.
dashboard-active-workspaces = Active Workspaces: { $count }
patient-record-header = Patient File: { $patient_name } (ID: { $mrn })
"#;

    group.bench_function("parse_fluent_resource", |b| {
        b.iter(|| {
            let res = FluentResource::try_new(black_box(ftl_source.to_string())).unwrap();
            black_box(res);
        });
    });

    let res = FluentResource::try_new(ftl_source.to_string()).unwrap();
    let lang_en: LanguageIdentifier = "en-US".parse().unwrap();
    let mut bundle = FluentBundle::new(vec![lang_en]);
    bundle.add_resource(res).unwrap();

    group.bench_function("format_simple_message", |b| {
        b.iter(|| {
            let msg = bundle.get_message("sync-status-online").unwrap();
            let pattern = msg.value().unwrap();
            let mut errors = vec![];
            let formatted = bundle.format_pattern(black_box(pattern), None, &mut errors);
            black_box(formatted);
        });
    });

    group.bench_function("format_parameterized_message", |b| {
        b.iter(|| {
            let msg = bundle.get_message("welcome-title").unwrap();
            let pattern = msg.value().unwrap();
            let mut args = FluentArgs::new();
            args.set("username", "Dr. Alexander");
            let mut errors = vec![];
            let formatted = bundle.format_pattern(black_box(pattern), Some(&args), &mut errors);
            black_box(formatted);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_fluent_i18n_bundle);
criterion_main!(benches);
