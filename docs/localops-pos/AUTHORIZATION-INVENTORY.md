# Phase 1 — Repository authorization inventory (Owner/Cashier access control)

This inventory was produced from the working tree before any code changes and is kept
current with the delivered implementation. It satisfies Section 30 (Phase 1),
Section 31 (command classification) and Section 42 (authorization review table) of
`docs/LOCALOPS_OWNER_CASHIER_ACCESS_CONTROL_SPEC_V2.md`.

## 1. Baseline facts verified against the working tree

| Area | Verified fact |
|---|---|
| Migrations | `crates/localops-core/migrations/0001_foundation.sql` … `0006_shift_expenses.sql`, applied by `localops_core::migrate` with `PRAGMA user_version` |
| Roles | `roles(id, business_id, name, system, active, …)`, `UNIQUE(business_id, name)`; no machine role key existed |
| Role assignment | `user_roles(user_id, role_id, …)` many-to-many; legacy `users.role_id` column still present, migrated into `user_roles` by `0002_identity_alignment.sql` |
| Permissions | `permissions(code, description)` seeded with 13 legacy codes; `role_permissions(role_id, permission_code, …)` |
| Authentication | `user::authenticate_user` — Argon2id, legacy SHA-256 upgrade, 5 failed attempts → lockout |
| Sessions | `sessions` table, `session::start_session/resume_session/touch_session/end_session`, 5-minute inactivity expiry |
| Session guard | `src-tauri/src/session_guard.rs::require_active_context` — optional single permission code |
| Terminal → department | `terminals.department_id` (added in `0002`) |
| Shifts | `shifts(user_id, terminal_id, opening/expected/actual/variance, closed_by)`, one open shift per terminal (`idx_shifts_one_open_per_terminal`) |
| Sales | `sales(business_id, department_id, terminal_id, shift_id, cashier_id, …)` — sufficient attribution for scoping |
| Frontend | single `src/App.tsx` view switch, no router; login returned permissions but the session model discarded them |

## 2. Command classification (every exposed Tauri command)

`PUBLIC_PREAUTH` / `AUTHENTICATED_GENERAL` / `CASHIER_SCOPED` / `OWNER_OR_PERMISSION` / `OWNER_ONLY`

| Command | Before | Classification | Enforced permission(s) | Scope / DTO change |
|---|---|---|---|---|
| `get_app_bootstrap` | public | `PUBLIC_PREAUTH` | none | Only business id/name + active terminal id/name/department. No management data |
| `login` | public | `PUBLIC_PREAUTH` | credentials | Returns canonical session: permissions + system role keys + terminal/department |
| `complete_initial_setup` | public, always callable | `PUBLIC_PREAUTH` **only while setup is required** | rejected once a business exists | Returns the canonical permission-bearing session |
| `create_business` | public | **removed from the runtime handler** (obsolete; first-run setup is the supported workflow) | n/a | n/a |
| `list_businesses` | public management read | **removed from the runtime handler** (bootstrap provides the safe login contract) | n/a | n/a |
| `update_trading_name` | public | `OWNER_OR_PERMISSION` | `business.manage` | Session business only; the caller-supplied id is ignored |
| `logout` | session | `AUTHENTICATED_GENERAL` | none | Own session |
| `get_session_capabilities` | new | `AUTHENTICATED_GENERAL` | none | Own session identity, permissions, role keys |
| `get_catalogue_snapshot` | session only (cost-bearing) | `OWNER_OR_PERMISSION` | `products.manage` | Cost fields only with `products.cost.view` |
| `get_department_stock_locations` | new | `OWNER_ONLY` | `departments.manage` | Active stock locations in the session business |
| `create_department_terminal` | new | `OWNER_ONLY` | `departments.manage`, `terminals.manage` | Creates one department, till and default stock link atomically; location must belong to the session business |
| `create_catalogue_*`, `replace_catalogue_recipe`, `set_catalogue_availability` | `products.manage` | `OWNER_OR_PERMISSION` | `products.manage` | unchanged |
| `get_inventory_snapshot` | session only | `OWNER_OR_PERMISSION` | `inventory.quantity.view` | Costs/valuation only with `inventory.value.view`; suppliers only with `suppliers.manage` |
| `create_inventory_supplier` | `inventory.manage` | `OWNER_OR_PERMISSION` | `suppliers.manage` | unchanged |
| `receive_inventory_purchase` | `inventory.manage` | `OWNER_OR_PERMISSION` | `purchases.manage` | unchanged |
| `complete_inventory_transfer` / `record_inventory_wastage` / `complete_inventory_stock_count` | `inventory.manage` | `OWNER_OR_PERMISSION` | `inventory.manage` | unchanged |
| `get_pos_snapshot` | session only, business-wide catalogue + business-wide recent sales + expected cash | `CASHIER_SCOPED` | `pos.catalog.view` | Terminal-department catalogue, no cost; own-current-shift sales unless `sales.view_all`; expected cash only with `shifts.expected_cash.view` |
| `open_pos_shift` | `shifts.manage` | `CASHIER_SCOPED` | `shifts.open_own` | Own shift on the session terminal; another user's open shift is never returned |
| `complete_pos_sale` | `sales.create` | `CASHIER_SCOPED` | `sales.create` + `payments.record`, `sales.discount` for non-zero discounts | Session business/terminal/department + own open shift (supervisory users need `shifts.close_any`) |
| `get_sale_receipt` | session + business only | `CASHIER_SCOPED` / `OWNER_OR_PERMISSION` | `receipts.reprint_all` or `receipts.reprint_own_current_shift` | Cashier: own sale, own current open shift, session terminal |
| `get_own_current_shift` | new | `CASHIER_SCOPED` | `shifts.view_own_current` | Own open shift, blind fields |
| `get_own_current_shift_sales` | new | `CASHIER_SCOPED` | `sales.view_own_current_shift` | Own current-shift sales only |
| `close_own_shift` | new (was `close_current_shift`) | `CASHIER_SCOPED` | `shifts.close_own` | Own open shift; blind response |
| `close_any_shift` | new | `OWNER_OR_PERMISSION` | `shifts.close_any` | Any open shift in the business; `closed_by` = actor |
| `create_pos_refund` | `sales.refund` / `sales.void` | `OWNER_OR_PERMISSION` | `sales.refund` / `sales.void` | Supervisory: may use the terminal's Cashier-owned open shift without taking ownership |
| `get_operations_snapshot` | session only (expected/variance/expenses) | `OWNER_OR_PERMISSION` | `shifts.view_all` | Expected/variance filtered by `shifts.expected_cash.view` / `shifts.variance.view`; expenses require `expenses.view` |
| `create_expense_category`, `record_operating_expense`, `void_operating_expense` | `expenses.manage` | `OWNER_OR_PERMISSION` | `expenses.manage` | unchanged |
| `get_dashboard_report` | `reports.view` | `OWNER_OR_PERMISSION` | `reports.view` | unchanged |
| `export_dashboard_csv` | `reports.view` | `OWNER_OR_PERMISSION` | `reports.export` | unchanged |
| `get_safety_status` | `business.manage` | `OWNER_OR_PERMISSION` | `backups.manage` | unchanged |
| `create_manual_backup` | `business.manage` | `OWNER_OR_PERMISSION` | `backups.manage` | unchanged |
| `restore_local_backup` | `business.manage` | `OWNER_ONLY` | `backups.manage` + `OWNER` system role | Owner system role required in V1 |
| `list_employees` | new | `OWNER_OR_PERMISSION` | `users.manage` | Business scope, never PIN hashes |
| `create_employee` | new | `OWNER_OR_PERMISSION` | `users.manage` + `roles.manage` | `user_roles` is authoritative |
| `update_employee` | new | `OWNER_OR_PERMISSION` | `users.manage` | Last-Owner protection; invalidates sessions |
| `reset_employee_pin` | new | `OWNER_OR_PERMISSION` | `users.manage` | Invalidates sessions |
| `set_employee_roles` | new | `OWNER_OR_PERMISSION` | `users.manage` + `roles.manage` | Last-Owner protection; invalidates sessions |

## 3. Broad authenticated reads found (Section 41.10 review)

| Read | Problem | Resolution |
|---|---|---|
| `get_catalogue_snapshot` | session-only, returned `costMinor` for every item | `products.manage`; cost suppressed without `products.cost.view` |
| `get_inventory_snapshot` | session-only, returned balances, suppliers, movements | `inventory.quantity.view`; value/suppliers gated |
| `get_pos_snapshot` | session-only, business-wide catalogue, business-wide last 25 sales, expected cash | `pos.catalog.view` + terminal-department scope + own-shift sales + gated expected cash |
| `get_operations_snapshot` | session-only, shift history with expected/variance and all expenses | `shifts.view_all` + field gating + `expenses.view` |
| `get_sale_receipt` | any sale in the business | reprint permissions + Cashier row scope |
| `list_businesses` | unrestricted management list | removed from the runtime surface |

Accidental session-only reads are **not** preserved as entitlements (Section 5.3).
