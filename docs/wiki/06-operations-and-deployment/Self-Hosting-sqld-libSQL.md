# Self-Hosting `sqld` & libSQL Primary Clusters

This operator guide describes deploying a self-hosted **`sqld` (libSQL primary server)** for local-first database replication.

---

## 1. Docker Compose Setup

Run `docker compose up -d` using the included [`docker-compose.yml`](file:///c:/Users/hellich/Desktop/YntraPlatform/docker-compose.yml):

```yaml
version: '3.8'

services:
  sqld:
    image: ghcr.io/tursodatabase/sqld:latest
    ports:
      - "8080:8080"
      - "5001:5001"
    environment:
      SQLD_NODE: "primary"
      SQLD_HTTP_LISTEN_ADDR: "0.0.0.0:8080"
    volumes:
      - sqld_data:/var/lib/sqld

volumes:
  sqld_data:
```

---

## 2. Client Connection Configuration

Configure your client `.env` or call `configure_database_sync()`:

```env
LIBSQL_URL="http://localhost:8080"
LIBSQL_AUTH_TOKEN="your-production-auth-token"
```

---

## 3. High Availability (HA) & Disaster Recovery

> [!IMPORTANT]
> A single `sqld` primary instance represents a single point of failure for write replication. Production deployments must run streaming WAL backups and monitoring.

### Continuous WAL Replication & Litestream S3 Backup
To prevent data loss in the event of hardware failure, configure continuous WAL (Write-Ahead Logging) streaming to S3/GCS using Litestream or `sqld` replication sidecars:

```yaml
  litestream:
    image: litestream/litestream:latest
    args: ['replicate', '/var/lib/sqld/dbs/default/data', 's3://yntra-backups-bucket/sqld']
    environment:
      LITESTREAM_ACCESS_KEY_ID: "${AWS_ACCESS_KEY_ID}"
      LITESTREAM_SECRET_ACCESS_KEY: "${AWS_SECRET_ACCESS_KEY}"
    volumes:
      - sqld_data:/var/lib/sqld:ro
```

### Health Probes & Monitoring
* **HTTP Health Check**: `GET /health` returns HTTP `200 OK` when the primary node is healthy and accepting SQL transactions.
* **Replication Lag Metric**: Prometheus metrics exposed on `:9090/metrics` track `sqld_replication_lag_seconds`. Alerts trigger if lag exceeds **5 seconds**.

