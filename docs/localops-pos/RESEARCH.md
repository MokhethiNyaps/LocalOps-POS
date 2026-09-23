# LocalOps POS Delivery Research

## Overview

LocalOps POS is an installed Windows application for offline, multi-department business operations. The binding product and technical research is maintained in the repository-wide architecture documents.

## Source of Truth

- `../MASTER-ARCHITECTURE.md` defines the business model, integrity rules, offline requirements, mandatory scenarios, and definition of done.
- `../TECHNICAL-DECISIONS.md` records the approved Tauri, React, Rust, SQLite, security, money, quantity, backup, and printing decisions.
- `../SCHEMA-V1.md` specifies the target V1 relational model.
- `../ERD.md` defines ownership and cardinality.
- `../GAPS-AND-RISKS.md` records scope boundaries and mitigations.

## Recommended Approach

Deliver the existing `../BUILD-PLAN.md` in tested vertical slices. Keep all mutations in Rust services, use SQLite transactions for multi-record business events, expose services through typed Tauri commands, and make the React UI a client of those commands. Every financial or inventory slice must include failure and rollback tests.

## Current Findings

- Architecture and executable foundations exist.
- Business setup and catalogue are partially implemented.
- The repaired baseline now has secure Argon2id PIN handling, many-to-many roles, atomic catalogue creation, department availability, a working first-run form, and passing frontend/Rust checks.
- Catalogue packaging and recipes are the next incomplete capabilities.
- Inventory onward remains primarily schema-only and requires business services, commands, UI, and scenarios.

## Risks

- Existing customer databases require forward-only, backed-up migrations.
- The initial schema contains legacy columns and terminal-field differences that must be reconciled without editing migration 0001.
- Financial and stock workflows must never be split across independent commits or database transactions.
