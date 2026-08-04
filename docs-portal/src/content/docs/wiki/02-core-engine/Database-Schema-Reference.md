---
title: "Complete 18-Table Database Schema & Migrations Dictionary"
head:
  - tag: script
    attrs:
      src: /target-blank.js
sidebar:
  hidden: false
---

Yntra Platform utilizes **libSQL / SQLite** with embedded local replicas as the single source of truth across Web, Desktop, iOS, and Android. All tables enforce strict row-level tenant partitioning via `workspace_id`.

:::note
Every table includes a `sync_status` column (`pending`, `synced`, `conflict`) to track offline journal state and libSQL replication status.
:::

---

## 1. Core Tenant & Identity Tables

### 1.1 `workspaces`
Stores workspace tenant metadata, active functional modules, and custom workspace settings.

| Column | Type | Constraints | Description |
| :--- | :--- | :--- | :--- |
| `id` | `TEXT` | `PRIMARY KEY` | Workspace unique identifier (`ws-...`) |
| `name` | `TEXT` | `NOT NULL` | Organization display name |
| `modules_active` | `TEXT` | `DEFAULT '[]'` | JSON array of active functional block keys |
| `settings` | `TEXT` | `DEFAULT '{}'` | Custom workspace settings JSON |
| `sync_status` | `TEXT` | `DEFAULT 'pending'` | Partition replication status (`pending`, `synced`) |

### 1.2 `users`
Stores user profile information, RBAC roles, and preference settings.

| Column | Type | Constraints | Description |
| :--- | :--- | :--- | :--- |
| `id` | `TEXT` | `PRIMARY KEY` | User unique identifier (`usr-...`) |
| `workspace_id` | `TEXT` | `NOT NULL` | Foreign key referencing `workspaces(id)` |
| `email` | `TEXT` | `NOT NULL UNIQUE` | User email address |
| `full_name` | `TEXT` | | User full display name |
| `role` | `TEXT` | `DEFAULT 'field_worker'` | RBAC role (`platform_admin`, `admin`, `supervisor`, `field_worker`) |
| `preferences` | `TEXT` | `DEFAULT '{}'` | User preference JSON |
| `updated_at` | `INTEGER` | `NOT NULL` | Unix timestamp in milliseconds |
| `sync_status` | `TEXT` | `DEFAULT 'pending'` | Replication status |

### 1.3 `teams` & `team_members`
Manages organizational team hierarchies and user assignments.

| Table | Column | Type | Constraints | Description |
| :--- | :--- | :--- | :--- | :--- |
| `teams` | `id` | `TEXT` | `PRIMARY KEY` | Team identifier |
| `teams` | `workspace_id` | `TEXT` | `NOT NULL` | Workspace foreign key |
| `teams` | `name` | `TEXT` | `NOT NULL` | Team display name |
| `team_members` | `team_id` | `TEXT` | `NOT NULL` | Foreign key referencing `teams(id)` |
| `team_members` | `user_id` | `TEXT` | `NOT NULL` | Foreign key referencing `users(id)` |

---

## 2. Operational & Field Work Tables

### 2.1 `time_reports` & `geofence_clock_events`
Tracks field worker time sheets, supervisor approvals, and GPS geofenced clock events.

| Column | Type | Constraints | Description |
| :--- | :--- | :--- | :--- |
| `id` | `TEXT` | `PRIMARY KEY` | Time report ID |
| `workspace_id` | `TEXT` | `NOT NULL` | Foreign key referencing `workspaces(id)` |
| `user_id` | `TEXT` | `NOT NULL` | Foreign key referencing `users(id)` |
| `clock_in_ms` | `INTEGER` | `NOT NULL` | Clock-in Unix timestamp ms |
| `clock_out_ms` | `INTEGER` | | Clock-out Unix timestamp ms |
| `break_duration_mins` | `INTEGER` | `DEFAULT 0` | Total break duration in minutes |
| `latitude` | `REAL` | | Clock event GPS latitude |
| `longitude` | `REAL` | | Clock event GPS longitude |
| `status` | `TEXT` | `DEFAULT 'submitted'` | Approval state (`submitted`, `approved`, `rejected`) |
| `sync_status` | `TEXT` | `DEFAULT 'pending'` | Replication status |

### 2.2 `job_tickets`
Manages field service dispatches, customer locations, and task checklists.

| Column | Type | Constraints | Description |
| :--- | :--- | :--- | :--- |
| `id` | `TEXT` | `PRIMARY KEY` | Job ticket ID (`job-...`) |
| `workspace_id` | `TEXT` | `NOT NULL` | Foreign key referencing `workspaces(id)` |
| `title` | `TEXT` | `NOT NULL` | Summary of work required |
| `description` | `TEXT` | | Detailed job instructions |
| `location_address` | `TEXT` | | Service destination address |
| `priority` | `TEXT` | `DEFAULT 'normal'` | Priority (`low`, `normal`, `high`, `urgent`) |
| `status` | `TEXT` | `DEFAULT 'open'` | Job lifecycle (`open`, `in_progress`, `completed`, `canceled`) |
| `scheduled_date` | `TEXT` | | Execution target date |
| `checklist_json` | `TEXT` | `DEFAULT '[]'` | Operational checklist items JSON |
| `sync_status` | `TEXT` | `DEFAULT 'pending'` | Replication status |

### 2.3 `offline_media_blobs`
Queues binary media attachments (photos, videos, signatures) for chunked background replication.

| Column | Type | Constraints | Description |
| :--- | :--- | :--- | :--- |
| `hash` | `TEXT` | `PRIMARY KEY` | Deterministic SHA-256 pointer (`sha256:...`) |
| `workspace_id` | `TEXT` | `NOT NULL` | Foreign key referencing `workspaces(id)` |
| `job_ticket_id` | `TEXT` | `NOT NULL` | Associated job ticket or entity ID |
| `media_type` | `TEXT` | `NOT NULL` | Media classification (`photo`, `video`, `signature`) |
| `compressed_blob` | `BLOB` | `NOT NULL` | Compressed binary payload |
| `upload_status` | `TEXT` | `DEFAULT 'queued_offline'` | Upload queue state (`queued_offline`, `uploading`, `synced`) |

---

## 3. Care Assistance & Healthcare Tables

### 3.1 `care_clients`, `medication_logs`, `handover_notes`

| Table | Column | Type | Constraints | Description |
| :--- | :--- | :--- | :--- | :--- |
| `care_clients` | `id` | `TEXT` | `PRIMARY KEY` | Care client identifier |
| `care_clients` | `personal_number` | `TEXT` | `NOT NULL` | Encrypted Swedish Personal Number |
| `care_clients` | `care_level` | `TEXT` | `DEFAULT 'standard'` | Assigned assistance level |
| `medication_logs` | `client_id` | `TEXT` | `NOT NULL` | Client foreign key |
| `medication_logs` | `administered_at_ms` | `INTEGER` | `NOT NULL` | Administration timestamp ms |
| `handover_notes` | `shift_date` | `TEXT` | `NOT NULL` | Shift date pointer |

---

## 4. Vehicle Fleet & Inspection Tables

### 4.1 `fleet_vehicles` & `inspection_logs`

| Column | Type | Constraints | Description |
| :--- | :--- | :--- | :--- |
| `vin` | `TEXT` | `PRIMARY KEY` | Vehicle Identification Number |
| `license_plate` | `TEXT` | `NOT NULL UNIQUE` | Vehicle registration plate |
| `current_mileage_km` | `INTEGER` | `NOT NULL` | Odometers reading in km |
| `status` | `TEXT` | `DEFAULT 'available'` | Fleet state (`available`, `in_service`, `maintenance`) |

---

## 5. Collaboration & Dynamic Entity Tables

### 5.1 `notes` & `messages`
Collaborative markdown documents and instant team chat messages with Loro/Automerge binary CRDT state.

| Table | Column | Type | Constraints | Description |
| :--- | :--- | :--- | :--- | :--- |
| `notes` | `id` | `TEXT` | `PRIMARY KEY` | Note ID |
| `notes` | `content` | `TEXT` | | Markdown document content |
| `notes` | `crdt_state` | `BLOB` | | Loro CRDT binary delta state |
| `messages` | `channel_id` | `TEXT` | `NOT NULL` | Chat channel or direct message target |
| `messages` | `sender_id` | `TEXT` | `NOT NULL` | Message author user ID |

### 5.2 `dynamic_blocks` & `dynamic_entity_instances`
Allows non-technical administrators to declare custom forms and operational database tables dynamically.

---

## 6. Enterprise Security & Audit Tables

### 6.1 `audit_logs`, `push_tokens`, `sso_sessions`, `bankid_sessions`

| Table | Purpose | Key Attributes |
| :--- | :--- | :--- |
| `audit_logs` | Immutable GDPR & compliance action log | `user_id`, `action`, `resource_type`, `ip_address`, `timestamp_ms` |
| `push_tokens` | Mobile APNs / FCM background push tokens | `user_id`, `os_type`, `device_token`, `registered_at` |
| `sso_sessions` | Enterprise OIDC / SAML SSO tokens | `session_id`, `domain`, `state_token`, `authorization_url` |
| `bankid_sessions` | Swedish BankID v6 auth state | `order_ref`, `auto_start_token`, `qr_code_svg` |