# Role-Based Access Control (RBAC) & Tenant Isolation

Yntra Platform enforces strict **Row-Level Tenant Isolation** and **Role-Based Access Control (RBAC)** inside `yntra-core`.

---

## 1. Supported User Roles

1. **`platform_admin`**: Full system-wide administration access across workspaces.
2. **`admin`**: Workspace administrator with user management, module toggling, and schema design permissions.
3. **`supervisor`**: Team leads who approve time reports, assign jobs, and manage operational dispatches.
4. **`field_worker`**: Operational field staff with access to job tickets, notes, messaging, and time clock.

---

## 2. Row-Level Tenant Partitioning

All database queries filter strictly by `workspace_id`. Cross-workspace queries by non-admin users return `YntraError::AuthError("Access denied: workspace mismatch")`.
