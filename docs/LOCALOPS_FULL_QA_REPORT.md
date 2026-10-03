# LocalOps POS QA Report

Historical report from September 30. The seeded-data cleanup and subsequent real Tauri UI tests are documented in [the October 1 WebDriver report](LOCALOPS_UI_WEBDRIVER_QA_REPORT.md).

## Test Environment

- Date: 2026-09-30
- Application: LocalOps POS `0.1.0`
- Repository: local working tree at `C:\Users\hp\Desktop\Mokhethi\POS application`
- Frontend: React 19 + TypeScript + Vite + Tailwind-style utility CSS
- Desktop shell/API: Tauri 2 command handlers
- Backend/domain: Rust workspace, `localops-core` crate + Tauri `app` crate
- Database: local SQLite at `C:\Users\hp\AppData\Local\LocalOps\POS\business.db`
- Authentication: username + numeric PIN; Argon2id current hashes with legacy SHA-256 upgrade on successful login
- Authorization: data-driven roles/permissions through `roles`, `user_roles`, `role_permissions`; centralized `AccessContext`; Tauri command guards
- Spec under test: `docs/LOCALOPS_OWNER_CASHIER_ACCESS_CONTROL_SPEC_V2.md`
- Browser/UI execution note: automated frontend tests and static/code review were run. The desktop shell UI was not manually click-tested from this environment because the native Tauri window cannot be controlled here.

## Executive Summary

The checked-in implementation is substantially aligned with the Owner/Cashier V2 specification. The codebase contains V2 migration `0007_owner_cashier_access_control.sql`, centralized Rust authorization helpers, backend-scoped Cashier DTOs, Owner/Cashier role seeding, session invalidation, blind shift close, scoped receipt reprint, scoped recent sales, and employee-management tests.

The local installed database was initially still schema v6. I created a QA backup, migrated it to schema v7, and seeded realistic demo data for Bar, Food, and Car Wash. After migration, `@thibos` authenticated successfully through the real application authentication function and resolved as an Owner with 38 permissions. A `Demo Cashier` account exists with the exact 8-permission Cashier whitelist.

No Critical or High defects were found in code-level access-control verification. The main residual risk is that a full manual Tauri UI click-through was not possible in this environment, so UI ergonomics, modal behavior, and visual responsiveness are partially verified rather than fully exercised.

## Demo Data Created

Before seeding, a local QA backup was created:

`C:\Users\hp\AppData\Local\LocalOps\POS\Backups\qa-before-seed-20260930-233008.db`

Local database status after QA setup:

- Schema version: 7
- Integrity: `quick_check = ok`
- Business: `Thabo's lifestyle centre`
- Currency/tax: ZAR, VAT enabled at 15%, prices inclusive
- Departments: Bar, Food, Car Wash
- Terminals: Main Till, Kitchen Till, Wash Bay Till
- Categories: Beer, Cider, Soft Drinks, Spirits & Mixers, Meals, Snacks, Wash Services
- Demo products: 14 products
- Demo services: 6 services
- Inventory balances: 8 stock-tracked Bar products
- Transactions: 3 completed QA sales totaling R370.00

Representative records:

- Bar: Castle Lager 330ml, Heineken 330ml, Savanna Dry 330ml, Coca-Cola 300ml, Bonaqua Still Water 500ml, Red Bull 250ml, Jameson Single 25ml, Schweppes Tonic 200ml
- Food: Beef Burger & Chips, Quarter Chicken Meal, Steak, Pap & Chakalaka, Full Breakfast Plate, Large Chips, Chicken Mayo Toastie
- Car Wash: Basic Wash, Wash & Vacuum, Full Valet, SUV Wash, Engine Clean, Interior Cleaning
- Stock states: Castle Lager normal stock, Heineken low stock, Savanna out of stock
- Users: `@thibos` Owner; `@demo-cashier` Demo Cashier
- Sales: `QA-BAR-001`, `QA-FOOD-001`, `QA-WASH-001`

No PINs are recorded in this report.

## Owner Test Results

`@thibos` was verified using the real `user::authenticate_user` path. Result:

- Login: PASS
- Active account: PASS
- System role: `OWNER`
- Effective permissions: 38
- Owner role permission grant: PASS

Owner capabilities were verified by code/tests for product management, costs, inventory visibility, reports, refunds/voids, shift supervision, employee management, backups, and full workspace navigation.

Manual UI click-through for every button/dialog was not completed because the Tauri desktop UI could not be controlled from this environment.

## Cashier Test Results

`Demo Cashier` was created in the local database and assigned through `user_roles` to the V2 `CASHIER` system role.

Effective Cashier permissions:

`payments.record`, `pos.catalog.view`, `receipts.reprint_own_current_shift`, `sales.create`, `sales.view_own_current_shift`, `shifts.close_own`, `shifts.open_own`, `shifts.view_own_current`

Denied by default:

Owner/admin permissions, product cost view, inventory management, supplier/purchase access, all-sales view, discounts, refunds, voids, shift history, expected cash, variance, reports, exports, expenses, roles, users, backups, and audit.

Frontend tests verify Cashier navigation shows only:

- Point of Sale
- My Shift
- Recent Sales

Backend tests verify Cashier cannot widen scope by business/terminal/department/user IDs, cannot see product cost fields, cannot discount, cannot access another user's receipt, cannot close another user's shift, and cannot use employee administration.

## POS Workflow Results

Seeded QA transactions:

| Sale | Department | Total | Paid | Change | Status |
|---|---:|---:|---:|---:|---|
| QA-BAR-001 | Bar | R142.00 | R150.00 | R8.00 | Completed |
| QA-FOOD-001 | Food | R133.00 | R133.00 | R0.00 | Completed |
| QA-WASH-001 | Car Wash | R95.00 | R100.00 | R5.00 | Completed |

Math spot-check:

- Sale totals equal line totals.
- Discount is zero for all QA sales.
- Payments cover sale totals.
- Cash change is recorded for cash payments.

Backend sale logic is separately covered by Rust tests for split payments, insufficient stock rollback, payment mismatch rollback, tax calculation, refund, and void behavior.

## Inventory Results

Inventory data exists for stock-tracked Bar products:

- Normal stock: Castle Lager 330ml
- Low stock: Heineken 330ml
- Out of stock: Savanna Dry 330ml

Car Wash services are modeled as `SERVICE` records, not stock-tracked products, so they do not directly affect physical inventory. Drinks made available to Car Wash remain products and can affect inventory if sold.

## UI/UX Findings

Verified by source/static tests:

- Session header uses authenticated display name and role label, not a hard-coded Manager label.
- Cashier navigation is permission-driven.
- Owner navigation exposes full management sections by permission.
- Sign-out clears current `session` state and the workspace is keyed by session ID, reducing stale Owner-to-Cashier UI risk.
- My Shift Cashier UI describes blind close and does not display expected cash/variance in the Cashier workflow.

Not fully verified:

- Button-by-button Tauri UI behavior
- Browser console/runtime warnings
- Modal close behavior
- Responsive/mobile layout
- Print dialog behavior

## Console/API Findings

Validation commands:

- `npm test -- --run`: PASS, 7 frontend tests
- `npm run build`: PASS
- `cargo test --workspace`: PASS, 167 Rust tests
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS

The first frontend test run failed under sandbox restrictions only; rerun outside the sandbox passed.

## Authorization and Security Findings

Strongly verified areas:

- V2 role migration creates stable `OWNER` and `CASHIER` role keys.
- Owner receives all permissions.
- Cashier receives only the whitelist.
- `user_roles` is authoritative.
- Backend command inventory exists in `docs/localops-pos/AUTHORIZATION-INVENTORY.md`.
- Public pre-auth surface is narrowed to bootstrap/login/setup.
- Sensitive reads require permissions.
- Cashier POS catalogue uses a sanitized DTO without cost/margin/valuation fields.
- Cashier POS catalogue is department-scoped by session terminal.
- Cashier recent sales are own/current-shift/session-terminal scoped.
- Receipt lookup is treated as reprint and is scoped.
- Non-zero discounts require `sales.discount`.
- Cashier blind close withholds expected cash and variance.
- Owner close-any preserves original shift owner and records closer.
- Role/PIN/deactivation changes invalidate affected sessions.

## Specification Compliance Matrix

| Requirement | Expected | Actual | Status | Evidence |
|---|---|---|---|---|
| Owner role exists | Stable `OWNER` system role | Migrated local DB has `OWNER`; 38 permissions | PASS | DB query; `role.rs`; migration 0007 |
| Cashier role exists | Stable `CASHIER` system role | Migrated local DB has `CASHIER`; 8 permissions | PASS | DB query; `CASHIER_PERMISSIONS` |
| Owner login | `@thibos` authenticates as Owner | Auth path returned pass, Owner role true | PASS | temporary auth check output |
| Cashier creation | Owner can create Cashier via `user_roles` | Code supports; demo cashier assigned via `user_roles` | PASS | `employee.rs`; DB query |
| Cashier whitelist | Only approved permissions | Exact whitelist present | PASS | DB query; Rust tests |
| Owner full access | Owner receives all permissions | Owner has 38 permissions | PASS | DB query; Rust tests |
| Cashier minimal workspace | POS/My Shift/Recent Sales only | Frontend test passes | PASS | `src/App.test.tsx` |
| Backend authorization | Rust/application layer enforces | `AccessContext`, command guards, tests | PASS | `access.rs`, `session_guard.rs` |
| Product cost hidden from Cashier | No cost/margin fields in Cashier DTO | Cashier DTO has no cost fields; serialization test | PASS | `cashier.rs` |
| Catalogue department scope | Cashier sees terminal department only | Test passes | PASS | `cashier_catalogue_is_limited...` |
| Recent sales scope | Own current open shift only | Test passes | PASS | `recent_sales_are_scoped...` |
| Receipt reprint scope | Own current shift or Owner all | Test passes | PASS | `authorize_receipt_reprint` |
| Discount authorization | Non-zero discount denied to Cashier | Test passes | PASS | `authorize_sale_request` |
| Refund/void denied to Cashier | Cashier lacks permissions | Whitelist excludes both; command requires permission | PASS | DB query; `sales_commands.rs` |
| Blind shift close | Cashier response excludes expected/variance | Test passes | PASS | `blind_close_hides...` |
| Owner close any | Owner can close Cashier shift | Test passes | PASS | `owner_can_close...` |
| Session invalidation | Role/PIN/deactivation ends sessions | Test passes | PASS | `employee.rs` |
| Public preauth boundary | Only safe bootstrap/login/setup public | Code inventory documents removal/protection | PASS | `commands.rs`, auth inventory |
| Direct-route protection | Hidden UI not enough; backend must deny | Backend command checks present; no router deep links | PASS | code review |
| Manual UI workflow coverage | Exercise every UI element | Not possible from this environment | NOT TESTABLE | native Tauri window unavailable |
| Console/network inspection | Inspect runtime console/network | Not possible from this environment | NOT TESTABLE | no controllable Tauri UI |

## Defects

### LOC-QA-001

- Severity: Medium
- Role: Both
- Area: QA coverage / UI runtime
- Preconditions: Running in this Codex environment
- Steps to reproduce: Attempt full desktop UI click-through automation
- Expected behavior: QA agent can open and control the Tauri desktop app
- Actual behavior: Native Tauri UI could not be controlled here; UI behavior was verified through source review, unit tests, and database/backend checks
- Relevant specification requirement: Sections 6, 10, 13, 14
- Evidence: Validation commands pass, but no browser console/network trace from live desktop UI
- Recommended fix: Run a follow-up manual QA pass on the desktop app or add Playwright/WebDriver-compatible UI harness/mocks for Tauri `invoke`

### LOC-QA-002

- Severity: Low
- Role: Owner
- Area: Employee Management UI
- Preconditions: Owner opens Employees screen
- Steps to reproduce: Open role dropdown for an employee
- Expected behavior: UI should clearly prevent accidental creation of active users with no usable role unless that is intentional
- Actual behavior: Source shows a `No role` option in the role selector. Backend last-Owner protection prevents removing the last Owner, but non-Owner users can potentially be left active with no role/permissions.
- Relevant specification requirement: Sections 24.2, 24.3
- Evidence: `src/App.tsx` EmployeesView role selector
- Recommended fix: Either remove `No role` for active employees or add an explicit confirmation/status explaining that the employee will not be able to use protected app functions.

### LOC-QA-003

- Severity: Low
- Role: Cashier
- Area: POS/API side effect
- Preconditions: Payment methods missing for a business
- Steps to reproduce: Cashier loads POS snapshot
- Expected behavior: Read-style Cashier POS snapshot should avoid management mutations where possible
- Actual behavior: `get_pos_snapshot` calls `payment::ensure_default_methods`, which may create default payment methods during a Cashier-accessible read.
- Relevant specification requirement: Sections 3.1, 20, 31
- Evidence: `src-tauri/src/sales_commands.rs`
- Recommended fix: Ensure default payment methods during setup/migration or Owner bootstrap, or document this as an intentional authenticated general initialization side effect.

## Recommendations

### Critical Before Release

None found in code-level Owner/Cashier access-control verification.

### Important Before Release

- Complete a manual desktop UI pass on Windows with the current migrated database.
- Add automated Tauri UI/e2e coverage for login, employee creation, POS sale, shift close, and Cashier denial flows.
- Add an explicit test that `get_pos_snapshot` does not leak cost-like serialized keys.

### UI/UX Improvements

- Clarify employee role changes that leave an account with no role.
- Add visible low/out-of-stock treatment in POS if not already shown at runtime.
- Add clearer empty states for no current shift and no current-shift sales.

### Future Improvements

- Add a supported QA seed command or fixture import flow instead of direct DB seeding.
- Add a controllable web preview harness that mocks Tauri commands for frontend regression testing.
- Add exportable command-authorization tests that call every exposed command as Owner and Cashier.

