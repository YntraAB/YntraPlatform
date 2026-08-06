use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tokio::runtime::Runtime;
use yntra_core::database;
use yntra_core::services::csv_import::{execute_csv_import, parse_and_preview_csv};

fn generate_sample_csv(rows: usize, delimiter: &str) -> String {
    let mut csv = format!("Customer Name{}Job Priority{}Cost Estimate\n", delimiter, delimiter);
    for i in 0..rows {
        csv.push_str(&format!(
            "Client_{}{}High{}${}.00\n",
            i, delimiter, delimiter, 100 + i
        ));
    }
    csv
}

fn bench_csv_parse_and_preview(c: &mut Criterion) {
    let mut group = c.benchmark_group("CSV Parsing & Delimiter Detection Preview");
    let rt = Runtime::new().unwrap();

    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    rt.block_on(async {
        let conn = database::acquire_connection().await.unwrap();
        let _ = conn
            .execute(
                "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-csv-bench', 'CSV WS', '[]', '{}')",
                (),
            )
            .await;
        let _ = conn
            .execute(
                "INSERT OR REPLACE INTO users (id, workspace_id, email, password_hash, role) VALUES ('u-csv-admin', 'ws-csv-bench', 'admin@csv.io', 'hash', 'admin')",
                (),
            )
            .await;
    });

    for &row_count in &[10, 100, 1000] {
        let csv_data = generate_sample_csv(row_count, ";");
        group.throughput(Throughput::Elements(row_count as u64));

        group.bench_with_input(
            BenchmarkId::new("parse_and_preview_csv", row_count),
            &row_count,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let preview = parse_and_preview_csv(
                        black_box("u-csv-admin".to_string()),
                        black_box(csv_data.clone()),
                        None,
                    )
                    .await;
                    let _ = black_box(preview);
                });
            },
        );
    }

    group.finish();
}

fn bench_csv_bulk_import_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("CSV Bulk Ingestion Database Execution");
    let rt = Runtime::new().unwrap();

    let _lock = database::DB_TEST_LOCK.lock().unwrap();
    rt.block_on(async {
        let conn = database::acquire_connection().await.unwrap();
        let _ = conn
            .execute(
                "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES ('ws-csv-exec', 'CSV Exec WS', '[]', '{}')",
                (),
            )
            .await;
        let _ = conn
            .execute(
                "INSERT OR REPLACE INTO users (id, workspace_id, email, password_hash, role) VALUES ('u-csv-admin-2', 'ws-csv-exec', 'admin2@csv.io', 'hash', 'platform_admin')",
                (),
            )
            .await;
        let _ = conn
            .execute(
                "INSERT OR REPLACE INTO blocks (id, name, description, icon, category, created_at, fields_schema) VALUES ('blk-csv-target', 'Target Block', 'Desc', 'icon', 'cat', '2026-08-04', '[]')",
                (),
            )
            .await;
    });

    let mappings_json = r#"{"Customer Name": "client_name", "Job Priority": "priority", "Cost Estimate": "cost"}"#.to_string();

    for &row_count in &[50, 200] {
        let csv_data = generate_sample_csv(row_count, ",");
        group.throughput(Throughput::Elements(row_count as u64));

        group.bench_with_input(
            BenchmarkId::new("execute_csv_import", row_count),
            &row_count,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let res = execute_csv_import(
                        black_box("u-csv-admin-2".to_string()),
                        black_box("ws-csv-exec".to_string()),
                        black_box("blk-csv-target".to_string()),
                        black_box(mappings_json.clone()),
                        black_box(csv_data.clone()),
                    )
                    .await;
                    let _ = black_box(res);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_csv_parse_and_preview,
    bench_csv_bulk_import_execution
);
criterion_main!(benches);
