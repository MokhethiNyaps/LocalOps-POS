# LocalOps POS Implementation Plan

## Overview

This plan applies the approved repository architecture and build plan. A phase is complete only when its domain rules, persistence, Tauri boundary, UI, and relevant automated scenarios are working together.

## Phase Summary

1. Architecture foundation
2. Executable foundation
3. Business setup and local identity
4. Catalogue, packaging, and recipes
5. Inventory ledger and operations
6. POS, payments, receipts, and refunds
7. Shifts and expenses
8. Reports and dashboard
9. Backup, restore, migrations, audit, and offline safety
10. Windows packaging, printing adapter, and release validation

## Phase 1: Architecture Foundation

- [x] Technical decisions
- [x] ERD and cardinalities
- [x] V1 schema specification
- [x] Gaps and risks

Success: approved source-of-truth documents exist and agree.

## Phase 2: Executable Foundation

- [x] Tauri/React/Rust workspace
- [x] SQLite bootstrap in application data
- [x] Migration ledger and pre-upgrade backup
- [x] Integrity checks and offline build

Success: application opens a versioned local database and passes bootstrap tests.

## Phase 3: Business Setup and Local Identity

- [x] Business, department, location, terminal, and user repositories
- [x] Argon2id PIN authentication and lockout
- [x] Many-to-many roles and permissions
- [x] Department/location associations and terminal schema reconciliation
- [x] Sessions, local login/logout commands, owner permissions, and atomic setup UI

Success: an owner can configure and securely sign into a business entirely offline.

## Phase 4: Catalogue, Packaging, and Recipes

- [x] Categories, units, sellables, products, and services
- [x] Atomic product/service aggregate creation
- [x] Department availability and price overrides
- [x] Product packaging and exact conversions
- [x] Recipes and service consumables
- [x] Catalogue Tauri commands and management UI

Success: products and services can be configured, assigned, packaged, and composed without industry-specific code.

## Phase 5: Inventory

- [ ] Authoritative movement ledger and cached balances
- [ ] Receiving and purchases
- [ ] Transfers, wastage, and stock counts
- [ ] Negative-stock enforcement and audit
- [ ] Inventory UI and failure scenarios

Success: every stock change is traceable and balances reconcile to movements.

## Phase 6: POS and Payments

- [ ] Atomic sales and immutable snapshots
- [ ] Split payments, cash tender, and change
- [ ] Recipe/product consumption and idempotency
- [ ] Receipts, voids, and refunds
- [ ] POS UI and required success/failure scenarios

Success: sales cannot partially complete and all totals, payments, stock, and reversals remain traceable.

## Phase 7: Shifts and Expenses

- [ ] Shift open/close and cash reconciliation
- [ ] Expense categories and records
- [ ] Permission checks, audit, and UI

Success: cashier periods and expenses reconcile without deleting history.

## Phase 8: Reports and Dashboard

- [ ] Business and department sales totals
- [ ] Payment, expense, stock, variance, and gross-profit summaries
- [ ] Dashboard and exports

Success: persisted facts produce understandable business-wide and department reporting.

## Phase 9: Safety

- [ ] Automatic backup retention and manual backup/restore
- [ ] Migration verification from every supported version
- [ ] Audit coverage and database reconciliation
- [ ] Offline and failure-injection scenarios

Success: customer data survives upgrades and verified recovery works.

## Phase 10: Packaging

- [ ] Windows installer and release configuration
- [ ] Structured receipt data and OS print adapter
- [ ] Barcode input workflow
- [ ] End-to-end release checklist

Success: a Windows customer can install, configure, operate, print, back up, restore, and uninstall without an internet dependency.
