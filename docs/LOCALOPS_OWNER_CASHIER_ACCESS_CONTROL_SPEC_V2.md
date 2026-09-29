# LocalOps POS — Owner/Cashier Access Control & Authorization Implementation Specification

**Status:** Normative implementation specification — Revision 2  
**Purpose:** Improve and extend the existing LocalOps POS architecture/documentation with a precise Owner vs Cashier security model  
**Applies to:** LocalOps POS desktop application (Tauri + React/TypeScript + Rust application/domain layer + local SQLite)  
**Primary roles in this release:** `OWNER`, `CASHIER`  
**Future roles:** Manager/Supervisor, Stock Controller, Waiter, etc. are intentionally deferred unless required to preserve extensibility  
**Security model:** Authentication + permission checks + read authorization + record scoping + terminal/department/shift scoping + auditability  
**Compatibility principle:** Extend the existing architecture; do not redesign the application or introduce a cloud dependency  
**Revision basis:** Incorporates repository-specific review findings concerning pre-auth commands, terminal selection, Owner supervision of Cashier shifts, multi-role behavior, migration compatibility, receipt scope, session invalidation, and the local SQLite threat boundary

---

## 0. Instructions to the implementation agent

This document is a **normative implementation contract**, not a loose product suggestion.

The implementation agent MUST:

1. Read this document completely before editing code.
2. Read the current repository documentation, especially:
   - `README.md`
   - `docs/MASTER-ARCHITECTURE.md`
   - `docs/SCHEMA-V1.md`
   - `docs/localops-pos/PROGRESS.md`
   - `docs/RELEASE-CHECKLIST.md`

   Documentation removed from the working tree is not an implementation source. Where this specification requires a new `docs/USER-MANUAL.md` after implementation, create it from the completed behavior rather than relying on an obsolete manual.
3. Inspect the current Rust, Tauri command, React/TypeScript, migration, and SQLite implementation before making changes.
4. Produce an inventory of:
   - current permission keys;
   - current role tables/relationships;
   - current authentication/session structures;
   - current Tauri commands;
   - current Rust service/query functions;
   - all current read paths that expose sensitive data;
   - all current write/action paths that require authorization.
5. Preserve existing working behavior unless this specification explicitly changes it.
6. Enforce security in the Rust/application/service layer. Frontend hiding is UX only.
7. Derive security scope from the authenticated session whenever possible.
8. Fail closed when the application cannot prove that a cashier is authorized for requested data.
9. Add automated tests for every security boundary introduced in this document.
10. Update the affected documentation after implementation.
11. Run the repository's full validation commands before considering the work complete.

The implementation agent MUST NOT:

- create a second "cashier app";
- introduce a cloud authentication service;
- introduce Supabase/Firebase/a remote authorization server;
- implement security only by hiding buttons or routes;
- return sensitive data to the frontend and merely hide it visually;
- trust user-supplied `business_id`, `user_id`, `terminal_id`, `department_id`, or `shift_id` without validating them against the authenticated session;
- give Cashier broad "authenticated user" read access;
- give Cashier access to cost, profit, valuation, expected-cash, variance, supplier, expense, report, backup, role, or employee-administration data by default;
- implement Manager/Supervisor override as part of this release unless it is already required by existing code to keep the application functional;
- add direct employee-to-department assignment tables in this release solely for this feature;
- hard-code authorization decisions throughout React components;
- rely on mutable display names such as `"Owner"` or `"Cashier"` as the only machine-level role identity if a stable role key can be introduced safely;
- weaken existing audit, immutable-history, stock-ledger, refund, void, or shift rules;
- hard-delete financial history;
- share or expose the Owner PIN;
- bypass tests to make the build pass.

If current code conflicts with this specification, prefer the requirements in this document for Owner/Cashier access separation while preserving the broader architectural principles in `MASTER-ARCHITECTURE.md`.

## 0.1 Repository-specific baseline to preserve

The implementation agent MUST verify these facts against the working tree before changing code, but this specification is written with the following current LocalOps baseline in mind:

```text
Authentication
- username + PIN
- Argon2id PIN hashing
- failed-login lockout
- approximately five-minute session expiry

Identity / authorization
- roles and permissions are data-driven
- users can have multiple roles through user_roles
- login already returns permissions from Rust
- Tauri commands use a centralized session guard
- many write paths already check permissions

Current role table concept
- id
- business_id
- name
- system
- active

Current legacy permission vocabulary
- business.manage
- departments.manage
- locations.manage
- users.manage
- products.manage
- inventory.manage
- sales.create
- sales.void
- sales.refund
- payments.record
- shifts.manage
- reports.view
- expenses.manage
```

Important repository-specific gaps that this change is expected to address include:

- the React session model currently does not consistently retain/use the permissions returned by login;
- first-run entry constructs session state differently from normal login and must be brought into the same capability model;
- the sidebar/view selection is currently effectively the same for all users;
- several sensitive reads currently require only an authenticated session;
- POS recent sales are currently broader than the required Cashier scope;
- POS catalogue data is broader than the current terminal department and includes sensitive cost information;
- receipt lookup is business-scoped rather than Cashier/shift-scoped;
- sale discounts are accepted through the broad sale-creation path;
- shift/operations data exposes expected cash and variance;
- terminal-to-department association exists;
- direct employee-to-terminal and employee-to-department assignment do not currently exist;
- only one shift can be open per terminal;
- the existing shift model records the shift owner;
- the current database already stores sale scope fields sufficient for tighter authorization, including business, department, terminal, shift, and cashier/user attribution.

The agent MUST use the real repository as the final source of truth for exact symbol, table, migration, command, and field names. Where this section and the working tree differ, document the difference and implement the behavioral requirements in this specification without inventing unnecessary architecture.

---

# 1. Why this change exists

LocalOps already has the essential security foundation:

- local employee authentication;
- username + PIN sign-in;
- an authenticated session;
- business and terminal selection;
- roles;
- permissions;
- service-layer permission checks for sensitive actions;
- auditing for important business events.

However, Owner/Cashier separation is not complete if only the frontend is changed.

A secure implementation requires all of the following:

1. **Navigation visibility** — what screens the user can see.
2. **Component visibility** — what controls the user can see within a screen.
3. **Write/action authorization** — what commands the user may execute.
4. **Read authorization** — what categories of data the user may retrieve.
5. **Record scoping** — whose records the user may retrieve.
6. **Operational scoping** — which business, terminal, department, and shift the request belongs to.
7. **Sensitive-field filtering** — which fields may be returned at all.
8. **Audit attribution** — which authenticated user initiated the action.

The client requirement is therefore not:

> "Hide owner pages from the cashier."

The actual requirement is:

> "A cashier must only be able to perform standard till operations and retrieve the minimum data required for their own current work, even if they attempt to invoke backend commands directly."

---

# 2. Release scope

## 2.1 Required roles

This release MUST implement two production-facing system roles:

- `OWNER`
- `CASHIER`

The permission architecture MUST remain extensible enough to add future roles, but those roles are not required now.

### Owner

Owner is the business administrator.

Owner receives full application access, including sensitive financial, inventory, employee, configuration, backup, and reporting functions.

### Cashier

Cashier is a constrained point-of-sale operator.

Cashier is allowed to:

- sign in to a terminal;
- access a minimal cashier workspace;
- view only sellable items available to the current terminal's department;
- create normal sales;
- record allowed payments;
- print the receipt for a completed sale;
- reprint only authorized receipts;
- open their own shift;
- view limited information about their own current shift;
- view only their own current-shift sales;
- submit a blind closing cash count;
- close their own current shift;
- sign out.

Cashier is denied all other access by default.

---

## 2.2 Explicitly deferred features

The following are NOT required in this release:

- Manager role UI;
- Supervisor role UI;
- manager PIN override workflow;
- employee-to-department assignment;
- employee-to-terminal assignment;
- custom role designer;
- arbitrary permission editor for business owners;
- discount thresholds by role;
- timed manager authorization tokens;
- cloud synchronization of roles;
- remote identity management.

The architecture MUST NOT block these future features.

## 2.3 Resolved repository-specific decisions

The following decisions are **already resolved by this specification**. The implementation agent must not silently choose different product behavior.

### Decision 1 — Public pre-authentication surface

Only the smallest set of commands required to start LocalOps, determine setup state, complete genuinely required first-run setup, and authenticate may be public before a session exists.

The command classification `PUBLIC_PREAUTH` is introduced later in this document.

Expected treatment of known command concepts:

```text
get_app_bootstrap
    -> PUBLIC_PREAUTH

login
    -> PUBLIC_PREAUTH

complete_initial_setup
    -> PUBLIC_PREAUTH only while initial setup is genuinely required

logout / sign_out
    -> AUTHENTICATED_GENERAL

update_trading_name
    -> OWNER_OR_PERMISSION using business.manage

create_business
    -> remove from exposed runtime commands if obsolete;
       otherwise protect according to the actual supported business-creation workflow

list_businesses
    -> must not remain an unrestricted management data endpoint;
       use only a deliberately safe bootstrap/login contract or protect it
```

A command MUST NOT be classified `PUBLIC_PREAUTH` merely because it historically existed without a session check.

### Decision 2 — Terminal selection policy for V1

For this release:

> A Cashier may sign in using any active terminal belonging to the selected/authorized business. The chosen terminal becomes the terminal for that authenticated session and determines the department scope for that session.

This is intentionally **not** employee-to-terminal authorization.

Therefore:

```text
Cashier -> signs out -> signs in at another active business terminal
```

may legitimately produce a different department scope.

The security guarantee in V1 is:

```text
after login, Cashier cannot escape the selected session terminal/department scope
```

not:

```text
Cashier is permanently entitled to only one physical terminal
```

Do NOT introduce employee-to-terminal assignment or device binding solely to strengthen this rule in V1.

A future release may replace this policy with device-bound terminals or explicit employee terminal assignments.

### Decision 3 — Owner reversals during a Cashier-owned open shift

An authenticated Owner with the required permission may process a refund or void against the **current terminal's currently open shift even if that shift belongs to a Cashier**.

Rules:

- the reversal is authorized using the authenticated Owner's permissions;
- the reversal is attributed to the authenticated Owner;
- the reversal remains associated with the terminal's current open shift according to existing accounting rules;
- shift ownership remains with the original Cashier;
- this does not convert the shift into an Owner shift;
- this is a full Owner session, not a manager PIN override;
- this supervisory exception does not automatically grant the Owner permission to create ordinary new sales under another user's shift if the existing sale rules do not already support that.

This rule exists so a refund/void is not blocked merely because the Cashier's shift is the one open on that terminal.

### Decision 4 — Owner supervisory shift close

Add or preserve an explicit Owner-grade capability for closing another user's open shift:

```text
shifts.close_any
```

Policy:

- Cashier with `shifts.close_own` may close only a shift they own;
- Owner with `shifts.close_any` may close any open shift belonging to the current business;
- `closed_by` or equivalent audit attribution records the authenticated closer;
- the original shift owner remains unchanged;
- Owner receives full reconciliation information;
- Cashier receives the blind-close response defined later.

### Decision 5 — Roles select workspace; permissions authorize capabilities

LocalOps remains permission-driven.

Rules:

- permissions from all active assigned roles are combined using the existing union/additive model;
- broader data/action access exists only when the corresponding broader permission is present;
- possession of the `CASHIER` system role MUST NOT cancel permissions legitimately granted by another active role;
- the `OWNER` system role selects the full administration workspace and carries full authority;
- a user with only the normal Cashier authority receives the minimal Cashier workspace;
- if legacy/custom roles grant permissions beyond the Cashier baseline, the frontend must not throw those permissions away merely because the user also has `CASHIER`; expose only the UI that the actual effective permissions justify;
- future Manager behavior should be implemented through a Manager workspace plus explicit permissions, not by weakening Cashier rules.

Role names/workspaces are presentation and system-role identity concerns. Permissions remain the primary authorization mechanism.

### Decision 6 — Existing-role compatibility

Preserve authority that was **explicitly granted** through existing management permissions.

Do NOT preserve accidental access that existed only because a read command lacked a permission check.

Legacy compatibility mapping is defined in Section 5.3.

### Decision 7 — Receipt behavior

The receipt returned atomically by a successfully completed sale may be printed immediately.

Any later lookup by `sale_id`, including a second call to a generic receipt command, is a **reprint** and must pass receipt authorization and Cashier row/scope rules.

### Decision 8 — Role assignment source of truth

`user_roles` is authoritative for new and edited role assignments.

If the legacy `users.role_id` column still exists, it is compatibility data only and MUST NOT become the new source of truth.

### Decision 9 — Authorization changes invalidate sessions

When an Owner:

- deactivates a user;
- changes that user's role assignments;
- resets/changes that user's PIN through employee administration;

all active sessions belonging to the affected user MUST be invalidated.

The affected user must authenticate again.

This prevents stale frontend permissions and stale authenticated sessions from surviving identity/authority changes.

### Decision 10 — Local SQLite threat boundary

This specification secures access **through LocalOps application interfaces, Rust/Tauri commands, and normal application workflows**.

It does NOT provide database-at-rest confidentiality against a person who has unrestricted operating-system/filesystem access to the SQLite database.

If the deployment threat model includes a Cashier who is also a Windows administrator or can directly copy/read the LocalOps database files, deployment controls must include appropriate Windows accounts, filesystem permissions, and device/full-disk encryption.

Do not redesign the database encryption/storage architecture as part of this feature.

### Decision 11 — Existing timestamp behavior is unchanged

Authorization hardening does not, by itself, require a redesign of frontend-supplied sale/shift timestamps.

Preserve current timestamp semantics unless a concrete authorization bug requires a narrowly scoped correction.

---

# 3. Security principles

## 3.1 Deny by default

Every authenticated backend command/query must fall into one of these categories:

1. available to every valid authenticated session;
2. available only with one or more explicit permissions;
3. available only when both permission AND scope checks pass.

If a command is not deliberately classified, it MUST be treated as restricted.

---

## 3.2 UI authorization is not security

React may hide navigation and controls, but the Rust/application layer is authoritative.

Example:

```text
Cashier cannot see Reports in the sidebar
```

is NOT sufficient.

The backend must also reject:

```text
get_report(...)
export_report(...)
get_stock_valuation(...)
```

when the current session lacks the required authority.

---

## 3.3 Never send sensitive fields unnecessarily

Do not query a broad Owner model, serialize it, send it to React, and hide fields in the UI.

Prefer role/use-case-specific response DTOs.

Bad:

```text
Product {
  id,
  name,
  selling_price,
  cost_price,
  margin,
  latest_supplier_cost,
  ...
}
```

sent to Cashier and visually hiding `cost_price`.

Good:

```text
CashierSellableItem {
  id,
  name,
  sku,
  barcode,
  category_name,
  selling_price,
  is_service,
  availability_status
}
```

where sensitive fields do not exist in the cashier response.

---

## 3.4 Session-derived scope is authoritative

The authenticated session is the source of truth for:

- `user_id`
- `business_id`
- `terminal_id`

The server/service layer resolves:

```text
terminal_id -> department_id
```

Cashier requests must not be able to switch scope by submitting another employee, terminal, department, business, or shift identifier.

If a command accepts such IDs for Owner functionality, the service MUST separately validate Cashier access before using them.

---

## 3.5 Business isolation always applies

Every relevant query must remain scoped to the authenticated business.

At minimum:

```sql
WHERE business_id = :session_business_id
```

or an equivalent relationship-based ownership check must apply.

An authenticated user from Business A must never retrieve Business B's records.

---

## 3.6 Cashier scope is narrower than business scope

For Cashier, business scope alone is insufficient.

Cashier data is normally constrained by some combination of:

```text
session.business_id
session.terminal_id
resolved terminal.department_id
session.user_id
current open shift
```

The exact rule is defined per capability below.

---

## 3.7 Authorization must fail closed

If a Cashier operation requires:

- a current terminal;
- a department;
- an open shift;
- ownership of the shift;

and that relationship cannot be resolved, reject the operation.

Do not fall back to business-wide data.

---

## 3.8 Application security boundary

The authorization model in this document protects LocalOps APIs, services, Tauri commands, views, and normal application use.

It is not equivalent to encrypting the local SQLite database from an operating-system administrator.

Do not weaken application authorization because the database is local; application-level controls are still required. Conversely, do not claim this feature prevents direct filesystem/database inspection by a privileged local user.

---

# 4. Role identity

## 4.1 Stable system role keys

Authorization logic MUST NOT depend exclusively on mutable display text such as `"Owner"`.

The current role model is understood to contain fields conceptually equivalent to:

```text
id
business_id
name
system
active
```

and does not yet provide an immutable `role_key`.

Add the smallest safe migration required to support stable machine identities such as:

```text
OWNER
CASHIER
```

Recommended conceptual shape:

```text
roles
- id
- business_id
- name
- role_key          nullable for custom/legacy roles
- system
- active
```

Recommended invariant:

```text
UNIQUE (business_id, role_key) WHERE role_key IS NOT NULL
```

Use the repository's naming/conventions and SQLite capabilities.

Do not convert custom role display names into authorization constants.

---

## 4.2 System role behavior

### OWNER

- system role;
- immutable machine role key `OWNER`;
- cannot be deleted through normal employee UI;
- selects the full Owner workspace;
- receives all permissions required by LocalOps;
- newly introduced permissions are granted to Owner by migration/seeding;
- must remain assigned to at least one active usable Owner user per business.

### CASHIER

- system role;
- immutable machine role key `CASHIER`;
- cannot be deleted through normal employee UI;
- selects the Cashier-oriented workspace when the user's effective authority is the normal Cashier baseline;
- permission set is a deliberate whitelist;
- newly introduced permissions are NOT automatically granted to Cashier.

---

## 4.3 Multiple roles and permission precedence

The existing many-to-many `user_roles` model remains authoritative.

Effective permissions are the union of permissions granted by all active roles assigned to the active user in the current business, subject to any existing role/user active-state rules.

Normative behavior:

```text
effective_permissions(user)
    = union(permissions of all active assigned roles)
```

Rules:

1. `CASHIER` does not behave as a deny role.
2. Assigning another authorized role may add capabilities to a user who also has `CASHIER`.
3. A response may include broader fields only when the specific broader permission exists.
4. `OWNER` is the full-administration system role.
5. The frontend must use effective permissions for feature visibility and data requests.
6. The backend must never infer broad authorization merely from a workspace selection.
7. Future Manager support must be expressible through explicit permissions without treating Manager as Owner.

Examples:

```text
sales.view_own_current_shift
    -> own current-shift sale access

sales.view_all
    -> broader business sale access

shifts.expected_cash.view
    -> expected-cash field may be returned

shifts.variance.view
    -> variance field may be returned

products.cost.view
    -> product cost fields may be returned
```

If a non-Owner legacy/custom role gives a user broader permissions, preserve that explicit authority during migration. Do not force a user back to Cashier-level data solely because `CASHIER` is also present.

---

# 5. Permission model

## 5.1 Preserve existing permission keys where possible

The current repository permission vocabulary includes:

```text
business.manage
departments.manage
locations.manage
users.manage
products.manage
inventory.manage
sales.create
sales.void
sales.refund
payments.record
shifts.manage
reports.view
expenses.manage
```

Before changing permissions, inspect the repository and produce a compatibility map.

DO NOT blindly rename existing permission keys.

For every key in the target model below:

- if an equivalent current permission already exists, reuse it;
- if current permission semantics are too broad, introduce a narrower permission;
- if replacing/splitting a broad permission, preserve explicitly granted management authority through the compatibility rules below;
- do not treat previously unprotected reads as an entitlement that must be preserved.

---

## 5.2 Canonical target permission vocabulary

The following is the target authorization vocabulary.

Exact migration details may reuse existing equivalent keys, but behavior MUST match this model.

### Business / administration

```text
business.manage
departments.manage
locations.manage
terminals.manage
users.manage
roles.manage
backups.manage
```

### Catalogue

```text
products.manage
products.cost.view
pos.catalog.view
```

### Inventory / procurement

```text
inventory.manage
inventory.quantity.view
inventory.value.view
suppliers.manage
purchases.manage
```

### Sales / payments / receipts

```text
sales.create
payments.record
sales.view_own_current_shift
sales.view_all
sales.discount
sales.price_override
sales.refund
sales.void
receipts.reprint_own_current_shift
receipts.reprint_all
```

### Shifts

```text
shifts.open_own
shifts.close_own
shifts.close_any
shifts.view_own_current
shifts.view_all
shifts.expected_cash.view
shifts.variance.view
```

### Expenses

```text
expenses.view
expenses.manage
```

### Reports

```text
reports.view
reports.export
```

### Optional audit permission if an audit viewer exists or is introduced

```text
audit.view
```

---

## 5.3 Legacy permission compatibility mapping

The migration must distinguish **explicit authority** from **accidental exposure**.

### Preserve explicit management authority

Where an existing active role explicitly has one of these broad permissions, grant the corresponding narrower permissions required to preserve the role's intended management capability.

| Existing explicit permission | Compatibility grants / treatment |
|---|---|
| `shifts.manage` | Preserve `shifts.manage` if needed for compatibility and grant `shifts.open_own`, `shifts.close_own`, `shifts.close_any`, `shifts.view_all`, `shifts.expected_cash.view`, `shifts.variance.view` |
| `reports.view` | Keep `reports.view`; also grant `reports.export` if report export currently relies on `reports.view` and the role previously could export |
| `inventory.manage` | Keep `inventory.manage`; grant `inventory.quantity.view` and management-grade inventory visibility required by current inventory screens; grant `inventory.value.view` where current `inventory.manage` explicitly exposed valuation/cost information |
| `products.manage` | Keep `products.manage`; grant `products.cost.view` where the current product-management workflow exposes costs |
| `business.manage` | Keep `business.manage`; grant `backups.manage` if backup/restore currently derives authority from `business.manage` |
| `users.manage` | Keep `users.manage`; grant `roles.manage` where current user-management authority includes role assignment/management |

The agent must inspect real screens/services before finalizing each compatibility grant.

### Do not preserve accidental broad reads

If a user could previously read sensitive data solely because a command required only a valid session and no explicit permission, that access is considered an authorization gap.

Examples include broad operations snapshots, cost-bearing catalogue snapshots, business-wide recent sales, broad receipt lookup, and similar session-only reads.

Those paths must be closed. Do not seed a new permission to every existing user merely to preserve that accidental visibility.

### Cashier migration rule

Default/system Cashier remains a whitelist role.

Do not grant broad compatibility permissions to Cashier merely because historical code exposed the data.

### Owner migration rule

Owner receives all current and newly introduced permissions.

---

# 6. Required role-to-permission assignments

## 6.1 Owner permission policy

Owner MUST have every permission required to use all LocalOps features.

At minimum, Owner receives all canonical permissions in Section 5 plus any existing permissions not listed here.

Every migration that introduces a new permission MUST ensure every business's system `OWNER` role receives it.

Owner access must not depend on remembering to manually update a role after an application upgrade.

---

## 6.2 Cashier permission whitelist

Default Cashier receives ONLY:

```text
pos.catalog.view
sales.create
payments.record
sales.view_own_current_shift
receipts.reprint_own_current_shift
shifts.open_own
shifts.close_own
shifts.view_own_current
```

Cashier MUST NOT receive by default:

```text
business.manage
departments.manage
locations.manage
terminals.manage
users.manage
roles.manage
backups.manage

products.manage
products.cost.view

inventory.manage
inventory.quantity.view
inventory.value.view
suppliers.manage
purchases.manage

sales.view_all
sales.discount
sales.price_override
sales.refund
sales.void
receipts.reprint_all

shifts.close_any
shifts.view_all
shifts.expected_cash.view
shifts.variance.view

expenses.view
expenses.manage

reports.view
reports.export

audit.view
```

If existing permission names differ, map the behavior exactly.

---

# 7. Access matrix

The following table is normative for the two system roles in this release. Explicit additional permissions from other active roles may broaden a non-Owner user's capabilities as defined in Section 4.3.

| Capability | Owner | Default Cashier | Default Cashier scope | Backend enforcement |
|---|---:|---:|---|---|
| Sign in | Yes | Yes | Any active terminal in current business under V1 policy | Authentication |
| Sign out | Yes | Yes | Own session | Session |
| View full Owner workspace | Yes | No | N/A | Frontend workspace/capabilities |
| View minimal Cashier workspace | Optional | Yes | Current session | Frontend workspace/capabilities |
| View POS sellable catalogue | Yes | Yes | Current session terminal department only | Permission + scope |
| View product selling price | Yes | Yes | Sellable item only | Scoped DTO |
| View product cost | Yes | No | Never without broader permission | Permission + field exclusion |
| View margin/profit | Yes | No | Never without broader permission | Permission + field exclusion |
| Create normal sale | Yes subject to existing shift rules | Yes | Current business + terminal + department + own open shift | Permission + scope |
| Record payment | Yes | Yes | Authorized sale/shift | Permission + scope |
| Apply non-zero discount | Yes | No | Denied by default | `sales.discount` |
| Override selling price | Yes | No | Denied by default | `sales.price_override` |
| Refund | Yes | No | Owner may use current terminal's open shift even if Cashier-owned | `sales.refund` + supervisory shift rule |
| Void | Yes | No | Owner may use current terminal's open shift even if Cashier-owned | `sales.void` + supervisory shift rule |
| Print receipt returned by completed sale | Yes | Yes | Sale completed by authorized current command | Completed-sale response |
| Reprint receipt later | Yes | Yes, limited | Own sale in current open shift on current terminal | Permission + row scope |
| View business-wide recent sales | Yes | No | Never by default | `sales.view_all` |
| View own current-shift sales | Yes | Yes | Own current open shift only | Permission + row scope |
| Open own shift | Yes | Yes | Self + current terminal | Permission + scope |
| View own current shift | Yes | Yes | Own current shift | Permission + field filtering |
| Close own shift | Yes | Yes | Own current shift | `shifts.close_own` |
| Close another user's shift | Yes | No | Any open shift in current business | `shifts.close_any` |
| See expected cash | Yes | No | Never without permission | `shifts.expected_cash.view` |
| See variance | Yes | No | Never without permission | `shifts.variance.view` |
| View other shifts/history | Yes | No | Never by default | `shifts.view_all` |
| View stock management | Yes | No | Never by default | Permission |
| View stock valuation | Yes | No | Never by default | Permission |
| View suppliers | Yes | No | Never by default | Permission |
| View purchases | Yes | No | Never by default | Permission |
| View inventory movements | Yes | No | Never by default | Permission |
| View expenses | Yes | No | Never by default | Permission |
| Create/manage expenses | Yes | No | Never by default | Permission |
| View reports | Yes | No | Never by default | Permission |
| Export reports | Yes | No | Never by default | Permission |
| Manage products | Yes | No | Never by default | Permission |
| Manage departments/locations/terminals | Yes | No | Never by default | Permission |
| Manage employees | Yes | No | Never by default | Permission |
| Manage roles | Yes | No | Never by default | Permission |
| Backup/restore | Yes | No | Never by default | Permission |
| View audit logs | Yes if supported | No | Never by default | Permission |

---

# 8. Cashier workspace

## 8.1 Navigation

When the authenticated user is a Cashier, the normal navigation should contain only:

```text
Point of Sale
My Shift
Recent Sales
Sign Out
```

Do not show:

```text
Catalogue
Setup Tools
Inventory
Suppliers
Purchases
Expenses
Reports
Employees
Roles
Business Settings
Backup & Safety
```

Do not show inaccessible areas merely to produce a permission error after click.

---

## 8.2 Protected view/navigation behavior

Do not introduce a React routing framework solely for this feature.

If routing exists, apply route guards.

If the application uses a monolithic `App.tsx` or internal view switch, guard workspace/view selection **before rendering the protected view or requesting its data**.

If a Cashier attempts to select/navigate to an Owner-only view:

- do not render sensitive data;
- return to an allowed Cashier view or show a generic Access Denied state;
- do not fetch the restricted data first.

Backend authorization remains mandatory regardless of frontend navigation structure.

---

## 8.3 Frontend permission handling

Create one centralized frontend authorization/capability layer.

Conceptually:

```ts
hasPermission(permission)
hasAnyPermission([...])
isSystemRole("OWNER")
isSystemRole("CASHIER")
```

Do not scatter checks like:

```ts
if (user.roleName === "Cashier")
```

throughout unrelated components.

Role may select the broad workspace, but actual controls should normally be driven by permissions.


---

## 8.4 Authenticated identity display

The application header/user identity area must display the actual authenticated employee and an accurate role/workspace description.

Remove any hard-coded label such as:

```text
Manager
```

when it does not reflect the real session.

At minimum show the authenticated user's display/name or username. If a role label is shown, derive it from active system roles/effective workspace rather than hard-coding it.

Do not expose permission IDs in normal Cashier UI.

---

# 9. Authenticated session context

The session context available to the application layer should include or resolve:

```text
session_id
user_id
business_id
terminal_id
role/system-role identity
permission set
expiry information
```

The service layer must be able to derive:

```text
terminal -> department_id
```

and when required, distinguish between:

```text
current open shift owned by session user on session terminal
```

and:

```text
current open shift on session terminal regardless of owner
```

The first is used for normal Cashier operations. The second is used only by explicitly authorized supervisory flows such as Owner refund/void against a Cashier-owned shift.

Do not allow the frontend to claim the active user or terminal for a Cashier-sensitive operation.

---

# 10. Terminal and department scope

## 10.1 V1 terminal selection policy

Do NOT add employee-to-department or employee-to-terminal assignment in this release.

For V1:

> A Cashier may choose any active terminal belonging to their business during sign-in. The selected terminal becomes part of the authenticated session and determines the Cashier's department context for that session.

Conceptually:

```text
login
  -> selected active terminal in business
      -> authenticated session.terminal_id
          -> terminal.department_id
              -> operational department scope
```

This means terminal selection is **session context**, not a permanent employee entitlement.

A Cashier may sign out and deliberately sign in at another active business terminal. That is permitted in V1.

After authentication, however, the Cashier may not expand or change scope by submitting arbitrary terminal/department IDs.

If a terminal has no valid active department where a department is required, fail closed.

A future release may implement:

- device-to-terminal binding; or
- employee-to-terminal assignment;

but neither is part of this feature.

---

## 10.2 POS catalogue scoping

The current POS catalogue/snapshot must be tightened.

For a default Cashier it MUST return only items that are:

- owned by the authenticated business;
- active;
- sellable;
- available to the department associated with the session terminal;
- appropriate for existing product/service sale rules.

A Bar terminal must not receive Restaurant-only sellables simply because they belong to the same business.

Owner/catalogue-management queries may be broader when authorized.

---

## 10.3 Cashier POS DTO

Use a Cashier-specific or sanitized response contract.

Conceptual example:

```rust
struct CashierSellableItemDto {
    id: ItemId,
    name: String,
    sku: Option<String>,
    barcode: Option<String>,
    category_name: Option<String>,
    selling_price: Money,
    item_kind: SellableItemKind,
    availability_status: AvailabilityStatus,
}
```

It MUST NOT contain:

```text
cost_price
latest_cost
supplier_cost
gross_margin
gross_profit
inventory_value
recipe_cost
valuation
internal procurement metadata
```

If the POS requires information to enforce stock, enforce it server-side.

Do not expose sensitive cost data merely because internal business logic needs it.

---

# 11. Sale creation

## 11.1 Cashier sale scope

Cashier may create a sale only when:

1. session is valid;
2. `sales.create` is granted;
3. terminal belongs to the session business;
4. terminal resolves to the department context;
5. an appropriate open shift exists;
6. that shift belongs to the current cashier under the V1 Cashier rule;
7. sellable items are valid for the current terminal department;
8. all existing stock/recipe/payment invariants pass.

The created sale must record the authenticated user/cashier identity.

Do not accept a frontend-provided `cashier_id` as authoritative.

---

## 11.2 Sale user identity

For Cashier-created sales:

```text
sale.created_by_user_id = session.user_id
```

or the equivalent existing audit field.

If the sale model uses multiple employee-related fields, inspect existing semantics and ensure the initiating cashier is permanently attributable.

---

# 12. Discount authorization

This is a backend requirement, not just a UI requirement.

If the sale input permits line-level or sale-level discounts, enforce:

```text
discount == 0
    -> sales.create is sufficient

discount > 0
    -> require sales.create
    -> require sales.discount
```

Cashier does not receive `sales.discount`.

Therefore any non-zero discount submitted by a default Cashier must fail even if they manipulate the frontend request.

The frontend should also hide or disable discount controls for Cashier.

---

## 12.1 Future threshold design

Do not implement thresholds now unless already required.

The design should allow future rules such as:

```text
0%                    -> normal cashier sale
>0% and <= threshold  -> sales.discount
>threshold             -> manager authorization
```

---

# 13. Price override authorization

If the POS allows editing a line price or sale price independently of a stored product price:

- Owner may do so with `sales.price_override`;
- Cashier must not;
- direct backend manipulation must be rejected.

If no price-override feature exists, do not invent one solely for this work.

---

# 14. Payments

Cashier receives `payments.record`.

Payment recording must still be scoped to:

- current business;
- authorized/current sale;
- current terminal;
- current shift as required by existing transaction design.

Cashier must not be able to use a payment command to alter historical sales outside their scope.

Existing payment audit/history requirements remain unchanged.

---

# 15. Recent sales

## 15.1 Exact default Cashier definition

"Recent Sales" for a default Cashier means:

> Sales performed by the currently authenticated Cashier during that Cashier's current open shift on the current session terminal.

Required conceptual filter:

```sql
WHERE sale.business_id = :session_business_id
  AND sale.terminal_id = :session_terminal_id
  AND sale.cashier_id = :session_user_id
  AND sale.shift_id = :current_cashier_shift_id
```

Use the real schema field names. The current sale model is understood to already contain business, department, terminal, shift, and cashier/user attribution fields.

Do not implement Cashier recent sales as "latest N sales for the business."

The Cashier query must first resolve an open shift owned by the authenticated Cashier. An existing open shift owned by another user is NOT the current Cashier's shift.

---

## 15.2 Cashier recent-sale DTO

Return only what the Cashier requires operationally, such as:

```text
sale_id
receipt_number
completed_at
status
total
payment summary if operationally required
```

Do not include:

```text
cost
profit
margin
business-wide comparisons
internal valuation
other employee management data
```

---

## 15.3 Sale detail and receipt reprint

Cashier may open/reprint a sale only if it satisfies the same ownership/scope rule.

Prefer a database query that includes scope in the lookup rather than:

1. fetch arbitrary sale by ID;
2. return it;
3. decide in React whether to display it.

Unauthorized sale IDs must not reveal sensitive details.

---

## 15.4 Behavior after the Cashier closes the shift

The default Cashier permission is intentionally **current-open-shift** access.

Immediately after that shift closes:

- there is no current open shift for that Cashier on that terminal;
- `My Shift` shows no active shift / closed state;
- the Cashier's current-shift Recent Sales list becomes unavailable/empty;
- receipt reprints from the now-closed shift are no longer authorized under `receipts.reprint_own_current_shift`;
- Owner retains access through broader permissions.

Do not silently reinterpret `current_shift` as "most recently closed shift."

If the product later wants Cashiers to reprint receipts after close, introduce a deliberate new permission/scope rule.

---

# 16. Refunds and voids

Default Cashier has neither:

```text
sales.refund
sales.void
```

Cashier UI must not expose executable Refund/Void controls.

Backend must reject direct refund/void attempts from default Cashier.

Owner retains authorized refund/void functionality and all existing invariants:

- reason required where currently required;
- append-only reversal;
- no destructive rewriting of original financial history;
- inventory reversal rules;
- payment reversal rules;
- audit logging.

---

## 16.1 Owner reversal during a Cashier-owned shift

LocalOps allows only one open shift per terminal, and the shift has an owning user.

Therefore, V1 MUST support this supervisory case:

```text
Cashier owns the current terminal's open shift
Cashier signs out
Owner signs in on that terminal
Owner has sales.refund and/or sales.void
Owner processes the authorized reversal
```

The service must NOT reject the Owner merely because:

```text
open_shift.user_id != session.user_id
```

when the action is a permitted Owner supervisory refund/void.

Required behavior:

- validate Owner permission normally;
- resolve the session terminal's currently open shift;
- associate the reversal with that open shift according to existing financial rules;
- attribute the reversal action to the authenticated Owner;
- retain the Cashier as the original shift owner;
- do not mutate shift ownership;
- write the appropriate audit event;
- preserve all existing reversal/accounting invariants.

This is a narrowly defined supervisory exception for authorized reversals. It is not a generic rule that every Owner action may use another user's shift.

---

## 16.2 Manager override is deferred

Do not implement manager PIN override in this release.

The future model may record:

```text
initiated_by_user_id
authorized_by_user_id
authorization_reason
authorization_time
```

but no manager override workflow is required now.

For V1, the Owner signs in as the Owner and the action is attributed directly to the Owner.

---

# 17. Receipt printing

## 17.1 Receipt returned by sale completion

The safest normal sale flow is:

```text
complete_pos_sale(...)
    -> persists sale/payment/stock effects atomically
    -> returns completed receipt payload
    -> UI prints that returned payload
```

Printing the receipt returned directly by the successfully completed sale is allowed as part of that authorized operation.

It MUST NOT require a second unrestricted `get_sale_receipt(sale_id)` call.

---

## 17.2 Reprint

Any receipt lookup performed later by `sale_id` is a reprint.

Default Cashier may reprint only if:

- `receipts.reprint_own_current_shift` is granted;
- sale belongs to the session business;
- sale belongs to the session terminal;
- sale was created by the authenticated Cashier;
- sale belongs to that Cashier's current open shift.

Owner with:

```text
receipts.reprint_all
```

may use the broader Owner workflow.

After Cashier shift close, the old shift is no longer eligible for default Cashier reprints.

---

# 18. Shift authorization

Cashier receives:

```text
shifts.open_own
shifts.close_own
shifts.view_own_current
```

Cashier does not receive by default:

```text
shifts.close_any
shifts.view_all
shifts.expected_cash.view
shifts.variance.view
```

Owner receives all of the above, including:

```text
shifts.close_any
```

---

## 18.1 Open own shift

Cashier may open a shift only for:

```text
user_id = session.user_id
terminal_id = session.terminal_id
business_id = session.business_id
```

Preserve the existing one-open-shift-per-terminal invariant.

Important behavior when the terminal already has an open shift:

- if the existing open shift belongs to the authenticated Cashier, return/use it only if the existing product flow deliberately supports idempotent "open my current shift" behavior;
- if the existing open shift belongs to another user, DO NOT return it as though it were the current Cashier's shift;
- the Cashier must receive a clear terminal/shift-in-use error or equivalent safe state;
- only a user with supervisory authority such as `shifts.close_any` may resolve an abandoned other-user shift.

The frontend must not choose another employee as the Cashier shift owner.

---

## 18.2 View own current shift

Cashier may see only their own current open shift and only operational fields required during the shift.

Allowed examples:

```text
opened_at
opening_cash entered for that shift if product policy allows
current shift status
own shift identifier if required
terminal identity required for workflow
```

Sensitive management fields are excluded unless separately authorized.

---

## 18.3 Owner close-any supervision

Owner with `shifts.close_any` may close an open shift even when:

```text
shift.user_id != session.user_id
```

provided the shift belongs to the current business and all existing close-shift invariants are met.

Required attribution:

```text
shift.user_id / opened_by
    -> original shift owner remains unchanged

closed_by
    -> authenticated Owner/supervisory user
```

Owner receives the full reconciliation response allowed by:

```text
shifts.expected_cash.view
shifts.variance.view
```

This capability is intended for abandoned/unfinished Cashier shifts and supervisory reconciliation.

Cashier never receives `shifts.close_any` by default.

---

# 19. Blind shift close

Cashier shift close MUST use a blind-count model.

Cashier enters:

```text
actual_cash_counted
optional closing note
```

Backend calculates expected cash and variance using the existing business rules.

The backend persists:

```text
expected_cash
actual_cash
variance
```

but the Cashier response MUST NOT reveal:

```text
expected_cash
variance
```

unless the user also has the corresponding management permissions.

---

## 19.1 Cashier close response

Conceptual Cashier response:

```text
shift_id
status = CLOSED
closed_at
actual_cash_submitted
```

Do not include:

```text
expected_cash
variance
cash_difference
```

---

## 19.2 Owner shift view and supervisory close

Owner may retrieve:

```text
opening amount
expected amount
actual amount
variance
opened_by
closed_by
terminal
department
times
notes
history
```

subject to current business rules.

When an Owner closes another user's shift using `shifts.close_any`:

- the Owner is not subject to the Cashier blind-response restriction;
- the full reconciliation may be returned if the Owner has expected/variance permissions;
- the audit trail must distinguish the original shift owner from the authenticated closer.

---

# 20. Sensitive reads requiring Owner-grade authorization

The implementation agent must audit every backend read command.

At minimum, Cashier must be prevented from retrieving the following data unless a future explicit permission is deliberately granted.

## 20.1 Catalogue/configuration reads

Restricted:

- product cost;
- recipe cost;
- packaging/cost configuration;
- catalogue administration metadata;
- inactive administrative item lists;
- configuration intended for product maintenance.

Cashier may receive only sanitized sellable-item data through the POS query.

---

## 20.2 Inventory reads

Restricted by default:

- stock balance management screens;
- stock valuation;
- latest costs;
- inventory movement history;
- transfers;
- stock counts;
- waste/damage history;
- inventory locations beyond what is needed internally.

Do not grant Cashier `inventory.quantity.view` in this release unless a concrete client requirement is added.

POS stock validation can happen internally without exposing inventory management.

---

## 20.3 Supplier/procurement reads

Cashier must not retrieve:

- suppliers;
- supplier contact/configuration data;
- purchases;
- purchase costs;
- receiving history;
- procurement records.

---

## 20.4 Expense reads

Cashier must not retrieve:

- expense history;
- expense totals;
- expense categories if they are only needed for management;
- business expense reporting.

No Cashier expense creation in this release.

---

## 20.5 Shift management reads

Cashier must not retrieve:

- other cashiers' shifts;
- historical business shift list;
- expected cash;
- variance;
- reconciliation data for other users;
- business-wide terminal cash position.

---

## 20.6 Sales management reads

Cashier must not retrieve:

- all recent business sales;
- sales by other employees;
- previous shifts;
- other terminals;
- other departments;
- cost/profit fields.

---

## 20.7 Reports

All report queries require `reports.view`.

All report exports require `reports.export` or the existing stricter equivalent.

Cashier gets neither.

---

## 20.8 Backups and recovery

Backup/restore/health functionality must be Owner-only.

Prefer a dedicated:

```text
backups.manage
```

permission if introducing it is safe.

If current code uses `business.manage`, preserve compatibility but ensure Cashier cannot invoke any backup/restore command.

Restore is especially sensitive and must never be exposed to Cashier.

---

# 21. Read authorization implementation pattern

Create centralized authorization/scoping helpers in the Rust/application layer rather than duplicating ad hoc rules in every command.

Conceptual API only; adapt to repository conventions:

```rust
fn require_permission(
    ctx: &AccessContext,
    permission: PermissionKey,
) -> Result<(), AppError>;

fn require_any_permission(
    ctx: &AccessContext,
    permissions: &[PermissionKey],
) -> Result<(), AppError>;

fn resolve_terminal_department(
    ctx: &AccessContext,
) -> Result<DepartmentId, AppError>;

fn require_current_cashier_shift(
    ctx: &AccessContext,
) -> Result<ShiftId, AppError>;

fn require_sale_owned_by_current_cashier_shift(
    ctx: &AccessContext,
    sale_id: SaleId,
) -> Result<SaleScope, AppError>;
```

Avoid making Tauri command handlers themselves the only security boundary if the same application service can be called from other code.

The service/domain application boundary should remain authoritative.

---

# 22. Parameter tampering requirements

Automated tests must prove that a Cashier cannot escalate access by changing request parameters.

Examples:

Cashier submits:

```text
user_id = owner's id
```

Result: reject or ignore and derive `session.user_id`.

Cashier submits:

```text
terminal_id = another terminal
```

Result: reject unless it equals the authenticated session's terminal under Cashier rules.

Cashier submits:

```text
department_id = Restaurant
```

while signed into Bar terminal.

Result: reject/ignore; resolve Bar department from terminal.

Cashier submits:

```text
shift_id = another cashier's shift
```

Result: reject.

Cashier submits:

```text
sale_id = another cashier's sale
```

Result: do not return sale detail/reprint.

---

# 23. Owner workspace

Owner retains the full LocalOps workspace.

Owner navigation may include all existing areas, for example:

```text
Dashboard / Reports
Point of Sale
Sales
Catalogue
Setup Tools
Inventory
Suppliers / Purchases
Shifts & Expenses
Employees
Business Setup
Backup & Safety
```

Use the repository's actual screen structure rather than inventing duplicate screens.

---

# 24. Employee management UI

The existing identity/role services must be exposed through a minimal Owner-facing employee workflow.

Do not build a generic enterprise IAM suite.

`user_roles` is the authoritative role-assignment relationship for all new/edited employees.

If `users.role_id` still exists, treat it as legacy compatibility data. Do not revive it as the application source of truth.

---

## 24.1 Required employee list

Owner can view employees for the current business.

Suggested fields:

```text
Name
Username
Role(s)
Active/Inactive
Last relevant status if already available
Actions
```

Do not display PIN values or hashes.

---

## 24.2 Create Cashier

Required fields:

```text
display/name
username
PIN
confirm PIN
role = CASHIER
active = true
```

Use existing secure PIN hashing and validation.

Role assignment MUST be written through `user_roles`.

The Owner should not need to understand permission IDs to create a standard Cashier.

---

## 24.3 Edit employee

Owner may:

- update employee display/name;
- activate/deactivate a Cashier;
- reset/change Cashier PIN;
- assign/remove supported roles through `user_roles`;
- view role/capability summary.

Prefer deactivation to destructive deletion where the employee has historical transactions.

---

## 24.4 Last-Owner protection

This rule MUST be enforced in Rust/application logic, not only in the UI.

At minimum, reject an operation that would leave zero active users assigned to the active system `OWNER` role for the business.

A usable Owner for this invariant means, at minimum:

- user is active;
- user is assigned through `user_roles` to the active system `OWNER` role;
- that role is active.

Do not allow the normal employee-management flow to deactivate the final active Owner or remove that final Owner's Owner-role assignment.

Do not expose Owner PIN values/hashes.

---

## 24.5 Session invalidation after identity/authority changes

When an Owner:

- deactivates a user;
- resets/changes that user's PIN;
- changes that user's role assignments;

invalidate all active sessions belonging to the affected user.

The next protected command from an invalidated session must fail according to the existing session-expired/unauthorized behavior.

The user must sign in again, which refreshes both backend authority and frontend effective permissions.

This rule is preferred over leaving a logged-in frontend with stale navigation permissions while backend permissions have changed.

---

## 24.6 Cashier restrictions

Cashier must not be able to:

- list employees through management endpoints;
- create employees;
- deactivate employees;
- reset other users' PINs;
- change role assignments;
- grant themselves Owner access.

---

## 24.7 Role editor is out of scope

Do not build arbitrary custom permission editing in this release.

Owner/Cashier system role assignments should be deterministic.

Existing custom roles must still migrate safely and keep explicitly granted authority according to Section 5.3.

---

# 25. Database migration requirements

Exact migrations depend on the current schema.

The agent must inspect existing migrations and tables first.

The final migration set must be:

- deterministic;
- safe for existing business data;
- compatible with offline SQLite;
- consistent with repository migration conventions;
- covered by migration tests if available.

---

## 25.1 Stable role key

Add `role_key` or equivalent only if no immutable machine role key currently exists.

Backfill system roles carefully.

Do not infer a custom role's machine identity solely from arbitrary display text when ambiguous.

Expected system keys:

```text
OWNER
CASHIER
```

---

## 25.2 Permission rows

Ensure all required new permission keys exist.

Do not duplicate existing equivalent permissions.

---

## 25.3 Owner grants

Every existing system Owner role must receive all newly introduced permissions.

For new businesses, first-run setup must create Owner with the complete permission set.

---

## 25.4 Cashier system role

Ensure each initialized business has a system Cashier role with the exact default whitelist.

Do not add broad legacy management permissions to Cashier.

---

## 25.5 `user_roles` authority

All new role assignment code must read/write the current many-to-many `user_roles` relationship.

If `users.role_id` remains for backward compatibility:

- do not use it as the authoritative source for effective permissions;
- do not make new employee management depend on it;
- only maintain it if existing compatibility code requires synchronized legacy data, and document that behavior.

---

## 25.6 Existing custom/employee roles

Preserve explicitly granted authority using Section 5.3's compatibility mapping.

Do not automatically grant new sensitive permissions merely because an account existed.

Do not preserve session-only accidental reads as implied custom-role authority.

---

## 25.7 Session invalidation support

If the existing sessions table/service cannot invalidate all sessions for a user, add the smallest service/repository capability necessary to do so safely.

Role change, user deactivation, and PIN reset must call that invalidation path transactionally or with clearly safe ordering.

---

# 26. Authorization errors

Use the existing application error style.

Permission denial should remain understandable, for example:

```text
Permission denied: reports.view
```

Scope violations should not leak sensitive data.

For example, when a Cashier requests another employee's sale, prefer a generic authorization/not-found style outcome consistent with the application's error model rather than returning the sale and then denying a field.

---

# 27. Audit requirements

Existing audit-oriented design must be preserved.

At minimum, continue to audit important actions such as:

- user created;
- user activated/deactivated;
- PIN reset if the current audit model supports this safely;
- role assignment changed;
- sale completed;
- refund;
- void;
- stock adjustment;
- shift opened;
- shift closed;
- backup/restore;
- price change.

Never store plaintext PINs in audit records.

Cashier sales and shift events must be attributable to the actual authenticated Cashier.

---

# 28. Frontend data handling rules

## 28.1 No sensitive prefetch

Do not prefetch Owner-only data and then hide it based on role.

A Cashier workspace should not issue requests for:

- reports;
- inventory valuation;
- suppliers;
- expenses;
- shift history;
- employee lists;
- backups;
- product costs.

---

## 28.2 Cache/state reset on sign-out

On sign-out or session expiry, clear role-sensitive client state.

A Cashier signing in after an Owner must not see stale Owner report data from React state/cache.

Review any global store/query cache and invalidate protected data when session identity changes.

---

## 28.3 Session switch safety

When user changes:

```text
Owner -> Sign out -> Cashier
```

the Cashier UI must initialize from Cashier-safe data only.

Test this flow explicitly.


---

## 28.4 Consistent frontend session/capability model

Normal login already returns permission data from the backend. The frontend session model must retain and use it.

Initial setup must enter the application through the same effective session/capability shape as normal login.

Do not maintain two incompatible frontend session representations where:

```text
normal login -> has permissions
first-run setup -> manually constructed session without permissions
```

After setup, obtain/construct the canonical authenticated session response using the same semantics as normal login.

---

## 28.5 Permission refresh policy

Do not add a live permission-refresh mechanism unless clearly simpler than existing session invalidation.

This release uses the following policy:

```text
user deactivation
role assignment change
PIN reset/change through employee administration
    -> invalidate all affected user's active sessions
    -> require re-login
```

This ensures frontend and backend authority converge after authentication changes.

---

# 29. First-run setup

Existing first-run behavior creates:

- business;
- first department;
- stock location;
- terminal;
- Owner;
- Owner role;
- permissions;
- active session.

Update first-run setup so the authorization model remains consistent.

At minimum:

- Owner system role is correctly identified;
- Owner has all current permissions;
- Cashier system role exists or is created through the standard business/system-role initialization path;
- creating Cashier role must not compromise atomic setup semantics;
- the frontend enters the workspace using the same canonical permission-bearing session model used by normal login.

If first-run currently performs all setup in one transaction, preserve that all-or-nothing behavior.

---

# 30. Recommended implementation sequence

Follow this order unless repository realities require a documented adjustment.

## Phase 1 — Repository authorization inventory

Before code changes, produce an internal implementation checklist mapping:

```text
Tauri command
-> Rust service/query
-> current permission
-> returned data
-> target role/scope
```

Specifically identify broad authenticated reads.

---

## Phase 2 — Permission/schema migration

Implement:

- stable role key if required;
- new narrow permission rows;
- Owner grants;
- system Cashier role;
- legacy permission compatibility.

Add migration tests where available.

---

## Phase 3 — Central access context and helpers

Implement/refine:

- session-to-access-context resolution;
- permission helper;
- terminal department resolver;
- current Cashier shift resolver;
- scoped sale lookup.

Do not start with frontend hiding.

---

## Phase 4 — Read authorization hardening

Protect all sensitive read commands.

Split Owner DTOs from Cashier DTOs where necessary.

Priority reads:

1. POS snapshot/catalogue;
2. recent sales;
3. sale detail/receipt;
4. shifts;
5. product/catalogue admin;
6. inventory;
7. suppliers/purchases;
8. expenses;
9. reports;
10. backups;
11. employee/role data.

---

## Phase 5 — Write authorization hardening

Implement:

- non-zero discount permission check;
- price override check if applicable;
- Cashier sale scope;
- payment scope;
- own-shift open/close rules;
- refund/void denial for Cashier.

---

## Phase 6 — Blind shift close

Ensure Cashier close response excludes expected cash and variance.

Ensure Owner management views retain reconciliation details.

---

## Phase 7 — Cashier frontend workspace

Implement:

- role/capability-aware navigation;
- guarded route/view selection appropriate to the existing frontend architecture;
- POS-safe data calls;
- My Shift;
- own Current-Shift Recent Sales;
- receipt reprint scope;
- state clearing between users.

---

## Phase 8 — Owner employee management

Implement minimal:

- employee list;
- create Cashier;
- edit/deactivate Cashier;
- reset Cashier PIN;
- role display/assignment limited to system roles needed for this release.

---

## Phase 9 — Tests

Complete all security and regression tests in Section 34.

---

## Phase 10 — Documentation

Update:

- `docs/MASTER-ARCHITECTURE.md`
- `docs/USER-MANUAL.md`
- `docs/localops-pos/PROGRESS.md`
- any command/API/developer docs affected by permission changes
- release checklist if role testing should become a release gate.

---

# 31. Backend command classification checklist

The agent must classify EVERY exposed Tauri/backend command into exactly one of the following security categories:

```text
PUBLIC_PREAUTH
AUTHENTICATED_GENERAL
OWNER_OR_PERMISSION
CASHIER_SCOPED
OWNER_ONLY
```

The classification must be recorded during Phase 1.

---

## 31.1 PUBLIC_PREAUTH

This category is intentionally very narrow.

A command may be `PUBLIC_PREAUTH` only when it is required to:

- bootstrap the application before login;
- determine whether first-run setup is required;
- complete first-run setup when setup is genuinely required;
- authenticate a user.

Expected examples:

```text
get_app_bootstrap
login
complete_initial_setup   // only while setup-required state is true
```

A command MUST NOT remain public merely because legacy code predates session enforcement.

Known legacy command concepts must be reviewed explicitly:

```text
update_trading_name
    -> protect with business.manage

create_business
    -> remove from exposed runtime commands if obsolete,
       otherwise protect according to the supported creation workflow

list_businesses
    -> do not expose as unrestricted management data;
       replace with deliberate safe bootstrap/login data or protect it
```

---

## 31.2 AUTHENTICATED_GENERAL

These require a valid session but no business capability beyond being the current authenticated user.

Potential examples:

```text
get_current_session
logout / sign_out
```

Do not classify a command as general merely because it is a read.

---

## 31.3 CASHIER_SCOPED

These require both permission and the narrow session/user/terminal/department/shift scope applicable to the operation.

Examples:

```text
get_cashier_pos_catalog
get_own_current_shift
get_own_current_shift_sales
reprint_own_current_shift_receipt
open_own_shift
close_own_shift
create_sale
record_payment
```

The same underlying service may support broader Owner access through a different permission/scope branch, but default Cashier behavior must remain scoped.

---

## 31.4 OWNER_OR_PERMISSION

Examples:

```text
manage_product
refund_sale
void_sale
view_reports
manage_inventory
view_all_sales
close_any_shift
```

Authorization is based on explicit permissions, not merely the string `"Owner"`.

The Owner system role receives all required permissions.

---

## 31.5 OWNER_ONLY

Use this only where the product deliberately does not support delegation in V1.

Examples may include:

```text
restore_backup
manage_system_role definitions
```

Even these should preferably still have a named permission internally for extensibility, but no non-Owner role receives it in V1.

---

## 31.6 Classification acceptance rule

No exposed command may remain in an implicit category such as:

```text
"it has always been public"
"it is only a read"
"the frontend doesn't show it"
```

Every command must have an intentional classification and matching tests.

---

# 32. Suggested query design

Prefer scoped database queries over post-query filtering.

Bad:

```rust
let sales = repo.list_recent_sales(business_id)?;
sales.into_iter()
     .filter(|sale| sale.user_id == session.user_id)
```

Better:

```rust
repo.list_sales_for_cashier_shift(
    session.business_id,
    session.terminal_id,
    session.user_id,
    current_shift_id,
)
```

The exact implementation should follow current repository/data-access patterns.

---

# 33. Sensitive serialization tests

Add tests that verify Cashier-facing serialized responses do not contain forbidden fields.

For example, a Cashier POS response should not serialize keys matching concepts such as:

```text
cost
cost_price
latest_cost
margin
profit
valuation
supplier_cost
expected_cash
variance
```

Do not rely only on UI snapshot tests.

Test the backend DTO/serialization boundary.

---

# 34. Mandatory test plan

The work is not complete until the following behavior is tested.

## 34.1 Authentication/session tests

- Owner can sign in.
- Cashier can sign in.
- Invalid Cashier PIN remains rejected.
- Existing lockout behavior remains intact.
- Session expiry behavior remains intact.
- Session includes/resolves correct business and terminal.
- Sign-out clears authorization context.

---

## 34.2 Owner positive tests

Owner can still:

- use POS;
- manage products;
- see costs;
- manage inventory;
- see suppliers/purchases;
- view all sales;
- refund/void;
- apply discounts if supported;
- open/manage shifts;
- see expected cash and variance;
- view expenses;
- view reports;
- export reports;
- manage employees;
- access backups/restores.

These tests protect against accidental over-restriction.

---

## 34.3 Cashier positive tests

Cashier can:

- load cashier POS;
- see only current terminal department sellables;
- add/scan permitted items;
- create a normal zero-discount sale;
- record payment;
- print newly completed receipt;
- open own shift;
- view own current shift;
- list own current-shift sales;
- reprint own current-shift receipt;
- submit blind closing count;
- close own shift;
- sign out.

---

## 34.4 Cashier negative action tests

Cashier cannot:

- create/update/deactivate products;
- apply a non-zero discount;
- override price;
- refund;
- void;
- adjust stock;
- transfer stock;
- receive purchases;
- create supplier;
- create/manage expenses;
- change business settings;
- manage users;
- manage roles;
- run backup;
- restore backup;
- export reports.

---

## 34.5 Cashier negative read tests

Cashier cannot retrieve:

- product costs;
- margins/profit;
- catalogue administration data;
- inventory valuation;
- inventory movement history;
- supplier list;
- purchase history;
- expense history;
- report data;
- report exports;
- all sales;
- another cashier's sale;
- previous-shift sales;
- another terminal's sale;
- other shift history;
- expected cash;
- shift variance;
- employee list;
- role/permission administration data;
- backup metadata if sensitive.

---

## 34.6 Scope-tampering tests

For a signed-in Cashier:

- passing another `business_id` fails;
- passing another `terminal_id` fails;
- passing another `department_id` fails or is ignored in favor of session-derived scope;
- passing another `user_id` fails or is ignored;
- passing another `shift_id` fails;
- passing another user's `sale_id` fails;
- reprinting a receipt from another shift fails.

---

## 34.7 Department catalogue tests

Given:

```text
Business
- Bar department
- Restaurant department

Bar terminal -> Bar department
Restaurant terminal -> Restaurant department
```

and items with department-specific availability:

- Bar Cashier sees Bar sellables;
- Bar Cashier does not receive Restaurant-only sellables;
- Restaurant Cashier sees Restaurant sellables;
- common items appear only according to configured availability rules.

---

## 34.8 Discount tests

- Cashier zero-discount sale succeeds.
- Cashier 1-cent/non-zero discount fails.
- Hiding the UI discount field is not the only test; invoke the backend path directly.
- Owner discount succeeds where existing business rules permit.

---

## 34.9 Blind-close tests

Cashier close command:

- accepts actual counted cash;
- closes own shift;
- stores expected/actual/variance correctly;
- returned Cashier payload excludes expected;
- returned Cashier payload excludes variance.

Owner shift-management query:

- can retrieve expected;
- can retrieve actual;
- can retrieve variance.

---

## 34.10 Cross-login state-leak test

Flow:

1. Owner signs in.
2. Owner opens Reports/Inventory/Employee data.
3. Owner signs out.
4. Cashier signs in.
5. Assert no cached Owner-only data remains visible.
6. Assert Cashier frontend does not automatically refetch restricted Owner endpoints.
7. Assert header/user identity shows the actual Cashier and does not retain a hard-coded/stale Owner/Manager label.

---

## 34.11 Employee-management tests

Owner:

- creates Cashier;
- Cashier role assignment is correct;
- PIN is stored using existing secure hashing mechanism;
- Owner can deactivate Cashier;
- deactivated Cashier cannot sign in;
- Owner can reset Cashier PIN.

Cashier:

- cannot list employees;
- cannot create employees;
- cannot change roles;
- cannot reset PINs for other users.

---

## 34.12 Public/pre-authentication command tests

- `get_app_bootstrap` works without a session only with safe bootstrap data.
- `login` works without a session.
- `complete_initial_setup` works without a session only when setup is genuinely required.
- `complete_initial_setup` is rejected once setup is complete.
- `update_trading_name` is rejected without a valid session/`business.manage`.
- legacy `create_business` is not publicly callable unless the supported product workflow explicitly requires it.
- unrestricted `list_businesses` management data is not publicly exposed.

---

## 34.13 Owner-supervision tests during Cashier shift

Given a Cashier-owned open shift on Terminal A:

- Cashier signs out.
- Owner signs in on Terminal A.
- Owner with `sales.refund` can refund an eligible sale without taking shift ownership.
- Owner with `sales.void` can void an eligible sale without taking shift ownership.
- reversal audit attribution records Owner as actor.
- shift owner remains the Cashier.
- default Cashier still cannot refund/void.

---

## 34.14 Owner close-any tests

Given an open shift owned by a Cashier:

- default Cashier other than the owner cannot close it;
- Owner with `shifts.close_any` can close it;
- original shift owner remains unchanged;
- `closed_by` records Owner;
- Owner receives full reconciliation where authorized;
- Cashier blind-close behavior is unchanged for own shifts.

---

## 34.15 Multi-role/effective-permission tests

- permissions combine across multiple active roles.
- assigning `CASHIER` does not remove a broader permission granted by another role.
- a user without `products.cost.view` does not receive product costs even if another broad UI screen is visible.
- a user with `products.cost.view` can receive cost fields only through an authorized endpoint.
- `OWNER` receives all expected permissions.
- inactive roles do not grant effective permissions.

---

## 34.16 Role migration compatibility tests

For representative existing custom roles:

- `shifts.manage` receives the intended split shift capabilities;
- current report exporters retain export ability if historically granted through `reports.view`;
- product managers retain required cost visibility;
- business managers retain backup authority if historically implied by explicit `business.manage`;
- user managers retain intended role-management authority;
- accounts that only had accidental session-only reads do not receive new sensitive permissions.

---

## 34.17 Session invalidation tests

When Owner changes another user's authority/account:

- deactivation invalidates all that user's sessions;
- role change invalidates all that user's sessions;
- PIN reset/change invalidates all that user's sessions;
- subsequent command using old session fails;
- re-login returns the new permission set;
- frontend initializes from new permissions.

---

## 34.18 Receipt lifecycle tests

- completed-sale receipt is printable directly from the sale completion result;
- a later `sale_id` receipt lookup is treated as a reprint;
- Cashier can reprint own current-open-shift receipt;
- Cashier cannot reprint another user's receipt;
- Cashier cannot reprint from a previously closed shift under the V1 policy;
- Owner can reprint historical receipts when authorized.

---

## 34.19 Terminal-selection policy tests

- Cashier may select any active terminal belonging to their business at login under V1.
- Cashier cannot select a terminal belonging to another business.
- selected terminal determines session department.
- after login, payload tampering cannot switch the session to another terminal/department.
- signing out and legitimately signing in at another active business terminal produces the new terminal/department scope.

---

# 35. Security acceptance criteria

The implementation is accepted only when all statements below are true.

### AC-01
A default Cashier cannot retrieve product cost, margin, profit, or valuation data by invoking backend commands directly.

### AC-02
A default Cashier POS catalogue is restricted to sellables available to the department associated with the session terminal.

### AC-03
A default Cashier can view only sales created by that cashier during the cashier's current open shift on the current terminal.

### AC-04
A default Cashier cannot retrieve another cashier's sale by guessing/submitting a sale ID.

### AC-05
A default Cashier cannot apply any non-zero discount.

### AC-06
A default Cashier cannot refund or void a sale.

### AC-07
A default Cashier can open and close only their own shift on the current terminal.

### AC-08
A default Cashier closing a shift does not receive expected cash or variance in the response.

### AC-09
A default Cashier cannot retrieve historical shift reconciliation data.

### AC-10
A default Cashier cannot retrieve suppliers, purchases, inventory management, expenses, reports, employees, roles, or backup data.

### AC-11
Owner retains all existing administrative and operational functionality.

### AC-12
Frontend navigation for Cashier contains only the approved Cashier workspace.

### AC-13
Deep linking or manually invoking a Tauri command does not bypass backend authorization.

### AC-14
Changing frontend-supplied user/terminal/department/shift IDs cannot expand Cashier scope.

### AC-15
Switching from Owner to Cashier clears sensitive cached frontend state.

### AC-16
Every new permission is automatically assigned to the system Owner role during setup/migration as required.

### AC-17
Cashier role is seeded as a whitelist, not "all except a few."

### AC-18
Sensitive Owner DTOs are not reused unfiltered for Cashier responses.

### AC-19
All authorization changes have automated tests.

### AC-20
Existing offline-first behavior remains intact.

### AC-21
Every exposed command is intentionally classified as `PUBLIC_PREAUTH`, `AUTHENTICATED_GENERAL`, `CASHIER_SCOPED`, `OWNER_OR_PERMISSION`, or `OWNER_ONLY`.

### AC-22
Only bootstrap, login, and setup-required initial setup remain public before authentication; legacy public management commands are removed or protected.

### AC-23
V1 terminal selection is explicit: a Cashier may select any active terminal in their business at sign-in, but cannot switch terminal/department scope inside the authenticated session by tampering with request IDs.

### AC-24
An authorized Owner can refund/void against the current terminal's Cashier-owned open shift without changing shift ownership, and the reversal is attributed to the Owner.

### AC-25
An Owner with `shifts.close_any` can close an abandoned/unfinished Cashier shift while preserving original shift ownership and recording the authenticated closer.

### AC-26
Permissions combine across active roles; `CASHIER` does not negate explicitly granted broader permissions.

### AC-27
New/edited role assignments use `user_roles` as the source of truth, and role/PIN/deactivation changes invalidate affected active sessions.

### AC-28
The documentation states the local SQLite/filesystem threat boundary and does not claim application authorization protects against unrestricted OS-level database access.

---

# 36. Definition of done

The agent may consider this feature complete only after:

1. schema/migration changes are implemented safely;
2. Owner/Cashier system roles are deterministic;
3. new narrow permissions exist;
4. Cashier has the exact approved whitelist;
5. all backend reads are audited;
6. sensitive reads are permission-protected;
7. Cashier rows are scoped by user/shift/terminal/department as specified;
8. Cashier DTOs exclude sensitive fields;
9. sale discounts are backend-authorized;
10. refund/void remain denied to default Cashier;
11. shift permissions are split;
12. blind closing is implemented;
13. Cashier navigation is minimal;
14. frontend protected state is cleared on identity change;
15. Owner employee management exists at the required minimal level;
16. Owner functionality is regression-tested;
17. Cashier positive and negative tests pass;
18. scope-tampering tests pass;
19. every exposed command is classified, including public pre-auth commands;
20. Owner refund/void against a Cashier-owned current terminal shift is implemented/tested;
21. `shifts.close_any` Owner supervision is implemented/tested;
22. legacy permission compatibility mapping is implemented/tested;
23. `user_roles` is authoritative for new role assignments;
24. user deactivation, PIN reset, and role changes invalidate sessions;
25. post-shift Cashier sales/reprint behavior matches the current-open-shift policy;
26. first-run and normal login share a consistent permission-bearing frontend session model;
27. documentation includes the local SQLite threat boundary;
28. documentation is updated;
29. all repository validation commands pass.

Required validation commands from the current project documentation include:

```bash
npm test -- --run
npm run build
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

If the repository adds further required validation commands, run those too.

---

# 37. Non-goals / anti-scope-creep rules

Do not use this feature request as justification to redesign unrelated systems.

Do not:

- rewrite the entire database;
- replace SQLite;
- replace Tauri;
- replace React;
- add online accounts;
- implement multi-location cloud sync;
- implement payroll;
- implement customer accounts;
- add Manager UI;
- build a generic policy engine;
- build ABAC beyond the scope rules in this document;
- add department assignment to employees;
- add terminal assignment to employees;
- redesign the receipt system beyond the required completed-receipt vs reprint authorization split;
- rewrite refund accounting beyond the Owner supervisory shift rule required here;
- redesign timestamp trust/time-source behavior merely because authorization is being hardened;
- add database-at-rest encryption as part of this feature;
- change inventory costing unless needed to prevent data exposure.

Keep the change focused on Owner/Cashier access control.

---

# 38. Future Manager/Supervisor compatibility

Although Manager is out of scope, do not make future Manager support impossible.

The future model should be able to grant a Manager some combination of:

```text
sales.discount
sales.refund
sales.void
receipts.reprint_all
sales.view_all
shifts.view_all
shifts.close_any
shifts.expected_cash.view
shifts.variance.view
inventory.quantity.view
```

without automatically granting:

```text
roles.manage
users.manage
backups.manage
business.manage
products.cost.view
inventory.value.view
```

This is why permissions must remain granular.

---

# 39. Future manager override design constraint

Do not implement now, but preserve the possibility that a restricted action can eventually contain:

```text
initiating_cashier_user_id
authorizing_manager_user_id
permission_authorized
reason
authorized_at
terminal_id
shift_id
sale_id
```

A future override should authorize a single action, not silently upgrade the Cashier's session to Manager.

---

# 40. Documentation changes required after implementation

## 40.1 `docs/MASTER-ARCHITECTURE.md`

Update the authentication/role/permission sections to state:

- Owner/Cashier system roles;
- permissions protect both writes and sensitive reads;
- session-derived scoping;
- terminal -> department Cashier scope;
- own-current-open-shift sale/reprint scope;
- V1 any-active-business-terminal login policy;
- Owner supervisory refund/void against a Cashier-owned open shift;
- `shifts.close_any`;
- multiple-role permission union semantics;
- public pre-authentication command boundary;
- session invalidation after role/PIN/account changes;
- blind shift closing;
- local SQLite/filesystem threat boundary;
- no frontend-only authorization.

Do not remove the generic extensible role architecture.

---

## 40.2 `docs/USER-MANUAL.md`

Replace the current statement that the sidebar may show inaccessible sections.

Document the actual Cashier workspace.

Add Owner instructions for:

- employee list;
- creating a Cashier;
- resetting a Cashier PIN;
- activating/deactivating a Cashier.

Document Cashier restrictions explicitly.

Document that the Owner PIN must never be shared.

Document blind cash close: Cashier enters actual count without seeing expected/variance.

Document that after a Cashier closes a shift, V1 current-shift Recent Sales and Cashier reprints from that closed shift are no longer available.

Document that V1 permits a Cashier to select any active terminal in the business at login; this is not permanent employee-to-terminal assignment.

---

## 40.3 Progress / release documentation

Update implementation progress and release checklist to include:

- Owner login smoke test;
- Cashier login smoke test;
- Cashier denied report test;
- Cashier denied product cost test;
- Cashier scope test;
- blind-close test;
- Owner-to-Cashier state-clear test.

---

# 41. Repository-specific implementation notes

The implementation agent should confirm these observations against the current working tree and use them to prioritize Phase 1.

## 41.1 Current permission baseline

The current permission set is understood to include:

```text
business.manage
departments.manage
locations.manage
users.manage
products.manage
inventory.manage
sales.create
sales.void
sales.refund
payments.record
shifts.manage
reports.view
expenses.manage
```

The narrower permissions in this specification therefore require migration/seeding work.

---

## 41.2 Current role/assignment model

Current roles are understood to contain:

```text
id
business_id
name
system
active
```

There is no immutable role key today.

User-role assignment is many-to-many through:

```text
user_roles
```

and must remain authoritative.

A legacy:

```text
users.role_id
```

may still exist for compatibility but is not the target source of truth.

---

## 41.3 Current frontend session mismatch

Normal login is understood to return:

```text
permissions: Vec<String>
```

from Rust.

The frontend session view currently does not consistently model/use this list, and first-run setup enters the workspace through separately constructed session state.

Unify these paths.

---

## 41.4 Current POS/catalogue exposure

The current POS catalogue is understood to:

- select active sellables across the business's departments rather than only the session terminal's department;
- include product cost data that must not be sent to default Cashier.

Split/sanitize the response.

---

## 41.5 Current sale scope

The sale schema already contains the key scope fields conceptually equivalent to:

```text
business_id
department_id
terminal_id
shift_id
cashier_id
```

Use these to implement row-level Cashier queries rather than redesigning the sale table.

---

## 41.6 Current open-shift behavior

Only one shift may be open per terminal.

The current open-shift service is understood to be capable of returning an existing open shift without first proving it belongs to the requesting user.

Fix that for Cashier:

```text
existing open shift owned by another user
    !=
Cashier's current shift
```

---

## 41.7 Current receipt lookup

Receipt lookup is understood to validate business ownership but not current Cashier/shift ownership.

Keep business-wide lookup for authorized Owner workflows and add a scoped Cashier reprint path/check.

---

## 41.8 Current discount path

Sale completion currently accepts discount values while relying on broad sale/payment permissions.

Add explicit `sales.discount` enforcement for non-zero discounts.

---

## 41.9 Current operations snapshot

The existing operations/shift snapshot is understood to contain management-sensitive data such as:

```text
expected balance
actual balance
variance
shift history
expense history
```

and cannot be reused unchanged for default Cashier.

---

## 41.10 Broad read review rule

For every broad read, answer:

1. Who may execute it?
2. What category of data does it return?
3. Whose records may be returned?
4. Which business/terminal/department/shift scope applies?
5. Which fields are permitted for this caller?

If any answer is unclear, treat the command as restricted until a safe contract is defined.

---

# 42. Recommended authorization review table

Before coding, create a temporary engineering table based on the actual working tree:

| Command/service | Command class | Current auth | Current returned data | Target permission | Target scope | DTO/action change |
|---|---|---|---|---|---|---|
| `get_app_bootstrap` | `PUBLIC_PREAUTH` | inspect | bootstrap state | none/pre-auth | safe bootstrap only | remove sensitive fields |
| `login` | `PUBLIC_PREAUTH` | credentials | session + permissions | authentication | selected business/active terminal | canonical session |
| `complete_initial_setup` | `PUBLIC_PREAUTH` conditional | inspect | setup result | setup-required only | first-run only | reject after setup |
| trading-name update | `OWNER_OR_PERMISSION` | inspect | business mutation | `business.manage` | current business | protect legacy path |
| POS snapshot | `CASHIER_SCOPED` / broader Owner path | session only / inspect | sellables + sensitive fields | `pos.catalog.view` | business + session terminal department | Cashier DTO |
| Recent sales | `CASHIER_SCOPED` / Owner path | session only / inspect | business-wide? | own-current-shift or `sales.view_all` | user + current open shift + terminal | split/scoped query |
| Sale detail | scoped | inspect | full sale | receipt permission / all-sales | own current open shift for Cashier | scoped lookup |
| Receipt lookup | scoped | business-only / inspect | receipt | reprint own/all | user + shift + terminal | tighten |
| Shift history | `OWNER_OR_PERMISSION` | inspect | expected/actual/variance | `shifts.view_all` | business | Owner/authorized |
| Close own shift | `CASHIER_SCOPED` | inspect | reconciliation result | `shifts.close_own` | own open shift | blind Cashier response |
| Close any shift | `OWNER_OR_PERMISSION` | new/legacy | reconciliation result | `shifts.close_any` | current business | supervisory attribution |
| Refund/void | `OWNER_OR_PERMISSION` | inspect | reversal | refund/void | current terminal open shift; Owner may use Cashier-owned shift | actor != shift owner allowed |
| Report | `OWNER_OR_PERMISSION` | `reports.view` | financial | `reports.view` | business | preserve |
| Backup | `OWNER_ONLY` or permission | `business.manage` currently / inspect | backup controls | `backups.manage` or compatible | business | Owner |
| Employee role edit | `OWNER_OR_PERMISSION` | inspect/new | identity | `users.manage`/`roles.manage` | business | `user_roles` + invalidate sessions |

Do not implement from this example table blindly. Replace placeholder descriptions with actual command/service symbols found in the repository.

---

# 43. Required behavior examples

## Example A — Bar Cashier

Session:

```text
user = Cashier Thabo
business = Moko's Lifestyle Centre
terminal = Main Bar Till
terminal.department = Bar
open shift = Thabo / Main Bar Till
```

Cashier POS query may return:

```text
Castle Lager
Heineken
Coke
Jameson
```

if those items are configured for Bar.

It must not return Restaurant-only items merely because they belong to the same business.

It must not return product cost.

---

## Example B — Cashier tries direct discount manipulation

Frontend hides discount field.

Cashier manually causes command payload:

```text
Castle Lager
selling price = R25
discount = R5
```

Backend sees user lacks:

```text
sales.discount
```

Result:

```text
Permission denied
```

No discounted sale is persisted.

---

## Example C — Cashier guesses another sale ID

Thabo's current shift sale IDs:

```text
101
102
103
```

Another cashier owns sale:

```text
99
```

Thabo requests sale 99.

Backend must not return full sale detail.

Access is denied/not found according to application convention.

---

## Example D — Blind close

System internally calculates:

```text
Expected cash: R5,030
```

Cashier enters:

```text
Actual count: R4,820
```

Database stores:

```text
expected = R5,030
actual = R4,820
variance = -R210
```

Cashier sees:

```text
Shift closed successfully
Actual cash submitted: R4,820
```

Cashier does not see expected or variance.

Owner later sees full reconciliation.

---

## Example E — Owner signs out, Cashier signs in

Owner previously viewed:

```text
Gross profit
Stock valuation
Expenses
Shift variance
Employees
```

After logout:

- all protected frontend cache/store state is cleared;
- Cashier signs in;
- no Owner-only values flash or remain visible;
- Cashier app loads only Cashier-safe endpoints.

---

## Example F — Owner refund during a Cashier-owned open shift

```text
Terminal: Main Bar Till
Open shift owner: Thabo (Cashier)
Current authenticated user: Owner
```

Owner has:

```text
sales.refund
```

Owner performs an eligible refund.

Required result:

```text
refund actor = Owner
shift = Thabo's still-open shift
shift owner remains Thabo
refund is audited
```

The service must not reject solely because Owner is not the shift owner.

---

## Example G — Owner closes abandoned Cashier shift

Thabo leaves without closing.

Owner opens shift management and closes Thabo's open shift using:

```text
shifts.close_any
```

Required result:

```text
opened_by/shift owner = Thabo
closed_by = Owner
expected/actual/variance = available to Owner
audit = records supervisory close
```

---

## Example H — V1 terminal switch requires re-authentication, not payload tampering

Cashier signs in at:

```text
Main Bar Till -> Bar department
```

During that session, they cannot submit:

```text
terminal_id = Restaurant Till
department_id = Restaurant
```

to switch scope.

However, under the V1 policy they may:

```text
sign out
sign in
choose Restaurant Till (if active and in the same business)
```

and then receive the Restaurant department scope for the new session.

This is intentional until explicit employee/device terminal restrictions are implemented.

---

## Example I — Shift close ends default Cashier sale-history access

Cashier closes Shift 555.

Before close:

```text
sales.view_own_current_shift
receipts.reprint_own_current_shift
```

allow access to eligible Shift 555 sales.

After close:

```text
current open shift = none
```

therefore Shift 555 no longer qualifies for default Cashier Recent Sales or reprint access.

Owner retains historical access.

---

# 44. Final architectural statement

The completed model must behave as:

```text
Authenticated Session
        |
        v
Access Context
(user, business, terminal, permissions)
        |
        +------------------------------+
        |                              |
        v                              v
Permission Check                 Scope Resolution
                                terminal -> department
                                user -> own open shift
        |                              |
        +---------------+--------------+
                        |
                        v
              Authorized Service/Query
                        |
             +----------+-----------+
             |                      |
             v                      v
       Owner DTO/Result       Cashier-safe DTO/Result
```

For Cashier-sensitive operations, the application must be able to answer:

```text
WHO are you?
WHAT are you allowed to do?
WHICH data category are you allowed to read?
WHOSE records are you allowed to read?
ON WHICH terminal?
IN WHICH department?
DURING WHICH shift?
WHICH fields may be returned?
```

Only after all required checks pass should data or an action be returned.

---

# 45. Final implementation mandate

This change is an extension of LocalOps, not a redesign.

The target outcome is:

> Owner experiences LocalOps as a complete business-management application.

and:

> Cashier experiences LocalOps as a focused till application with only the minimum information and authority required to sell, take payment, manage their own current shift, and access their own current-shift receipts.

The implementation is not secure if the Cashier can recover restricted information by:

- manually navigating to a hidden route;
- invoking a Tauri command directly;
- changing an ID in a payload;
- requesting an unscoped read command;
- inspecting a response that contains hidden sensitive fields;
- reusing stale Owner frontend state;
- invoking legacy pre-auth management commands;
- treating another user's open shift as the Cashier's shift;
- retaining an old session after role/PIN/account changes.

All such paths must be closed by deliberate command classification, backend authorization, session invalidation, and scoping.

The V1 threat boundary ends at the LocalOps application interface: direct SQLite/filesystem access by a privileged operating-system user requires deployment controls outside this feature.

**End of specification — Revision 2.**
