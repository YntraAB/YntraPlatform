use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use yntra_core::database::parser::{
    clean_sql, has_write_keyword, extract_table_name_ast, extract_table_name, split_sql_statements,
};

fn bench_sql_sanitization_and_cleaning(c: &mut Criterion) {
    let mut group = c.benchmark_group("SQL Query Sanitization & Clean");

    let raw_sql = r#"
        -- Multi-tenant row-level access verification query
        SELECT p.id, p.first_name, p.last_name, p.medical_record_number /* confidential */
        FROM patients AS p
        INNER JOIN workspace_tenants AS wt ON p.workspace_id = wt.id
        WHERE wt.tenant_code = 'CLINIC-ALPHA'
          AND p.active = 1
          AND p.created_at >= '2026-01-01 00:00:00'; -- filter active records
    "#;

    group.throughput(Throughput::Bytes(raw_sql.len() as u64));

    group.bench_function("clean_sql_strip_comments", |b| {
        b.iter(|| {
            let cleaned = clean_sql(black_box(raw_sql));
            black_box(cleaned);
        });
    });

    group.bench_function("has_write_keyword_check", |b| {
        b.iter(|| {
            let res = has_write_keyword(black_box(raw_sql));
            black_box(res);
        });
    });

    group.finish();
}

fn bench_sql_ast_table_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("SQL Query AST Table Name Extraction");

    let select_sql = "SELECT * FROM workspace_audit_trail WHERE workspace_id = 'ws-100' ORDER BY timestamp DESC LIMIT 50;";
    let insert_sql = "INSERT INTO patient_vitals (id, patient_id, heart_rate, blood_pressure, recorded_at) VALUES ('v-1', 'p-100', 72, '120/80', 1700000000);";

    group.bench_function("extract_table_name_regex_select", |b| {
        b.iter(|| {
            let res = extract_table_name(black_box(select_sql));
            black_box(res);
        });
    });

    group.bench_function("extract_table_name_ast_select", |b| {
        b.iter(|| {
            let res = extract_table_name_ast(black_box(select_sql));
            black_box(res);
        });
    });

    group.bench_function("extract_table_name_ast_insert", |b| {
        b.iter(|| {
            let res = extract_table_name_ast(black_box(insert_sql));
            black_box(res);
        });
    });

    group.finish();
}

fn bench_sql_script_splitting(c: &mut Criterion) {
    let mut group = c.benchmark_group("SQL Script Statement Splitter");

    let multi_statement_sql = r#"
        CREATE TABLE IF NOT EXISTS care_notes (id TEXT PRIMARY KEY, patient_id TEXT, author_id TEXT, note TEXT);
        INSERT INTO care_notes VALUES ('n-1', 'p-100', 'u-50', 'Patient responding well to treatment plan');
        UPDATE patient_records SET status = 'STABLE' WHERE id = 'p-100';
        DELETE FROM transient_locks WHERE locked_at < 1600000000;
        SELECT COUNT(*) FROM care_notes;
    "#;

    group.throughput(Throughput::Bytes(multi_statement_sql.len() as u64));

    group.bench_function("split_sql_statements", |b| {
        b.iter(|| {
            let stmts: Vec<&str> = split_sql_statements(black_box(multi_statement_sql)).collect();
            black_box(stmts);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_sql_sanitization_and_cleaning,
    bench_sql_ast_table_extraction,
    bench_sql_script_splitting
);
criterion_main!(benches);
