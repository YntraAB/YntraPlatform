---
title: "Self-Hosting sqld & libSQL Primary Clusters"
head:
  - tag: script
    attrs:
      src: /target-blank.js
sidebar:
  hidden: false
---

This operator guide describes deploying a self-hosted **`sqld` (libSQL primary server)** for local-first database replication.

---

## 1. Docker Compose Setup

Run `docker compose up -d` using the included [`docker-compose.yml`](https://github.com/YntraAB/YntraPlatform/blob/main/docker-compose.yml):

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

Configure your client `.env` or call [`configure_database_sync`](https://github.com/YntraAB/YntraPlatform/blob/main/yntra-core/src/database/sync.rs#L78-L83):

```env
LIBSQL_URL="http://localhost:8080"
LIBSQL_AUTH_TOKEN="your-production-auth-token"
```