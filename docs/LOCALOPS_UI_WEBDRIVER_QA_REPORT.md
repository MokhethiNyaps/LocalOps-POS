# LocalOps POS: Tauri UI Verification

Date: 2026-10-01, Africa/Johannesburg.

Subsequent reset: at the user's request, all LocalOps business databases, backups, WebView storage and UI test artifacts were deleted on October 1. The application was reopened with zero businesses, users, departments and sales. Artifact paths below record the completed test run; those screenshots and databases are no longer present. Source code, the test harness and this report were retained.

## Outcome

30 of 30 desktop UI checks passed against the real Tauri application, Rust command handlers and SQLite. Test records were created by typing into forms and clicking buttons with Selenium WebDriver. No database inserts, mocked Tauri commands, or direct command invocations populated the test database. Read-only SQLite queries checked persisted results.

One checkout defect was found and fixed: automatic sale numbers used the first eight characters of a UUIDv7, which are timestamp bits shared by successive transactions. Two sales made close together failed with a duplicate receipt-number error. Automatic sale, refund and void numbers now include the full transaction UUID. Explicitly supplied numbers and existing receipts remain unchanged. Two regression tests cover consecutive sales and refunds.

## Installed Database Cleanup

Before cleanup, SQLite's online backup saved the current database, including committed WAL data, to:

`C:\Users\hp\AppData\Local\LocalOps\POS\Backups\before-qa-cleanup-20261001.db`

Cleanup removed the three seeded QA sales, their lines/payments, three seeded shifts, seeded inventory balances, 19 unused catalogue records, seven categories, two units, SnapScan and the demo cashier. The two existing staff accounts and one subsequently created sale were preserved.

The subsequent sale references Basic Wash; staff session history also references the QA Food and Car Wash terminals. Those service/terminal/department identities were retained as inactive records to preserve receipt and staff history. All seeded department availability was removed. No active seeded sellable items or seeded sales remain. Historical backups were retained.

After cleanup: SQLite integrity `ok`, no foreign-key violations, two staff accounts, one non-seeded sale. Cleanup did not restore an older database or remove later user activity.

## Test Environment

- Windows, Tauri 2, debug test build with embedded production frontend assets.
- Selenium WebDriver, official `tauri-driver` 2.1.0, Microsoft Edge WebDriver/WebView2 154.0.4258.37.
- Desktop window configured at 1280 x 900 logical pixels; screenshots reflect Windows DPI scaling.
- Separate application identifier: `za.co.localops.pos.e2e`.
- Compile-time `e2e` feature requires an absolute `LOCALOPS_E2E_DATA_DIR`; it never falls back to installed data when that environment variable is missing.
- Startup, safety checks, backup creation and restore all resolve the same isolated test directory.
- Each run starts in a new directory under `target/ui-e2e`. PINs are generated in memory and omitted from reports and logs.
- Test executable is copied to `target/ui-test-build/app.exe` so a normal development build cannot replace the executable used by the suite.
- Setup follows the [official Tauri WebDriver guidance](https://v2.tauri.app/develop/tests/webdriver/manual-setup/).

## Passing UI Checks

1. First-run business, department, location, terminal and Owner creation.
2. Category and measurement-unit entry.
3. Native required-field validation rejects an empty catalogue form.
4. Stock product and service creation.
5. Department availability for both items.
6. Case-of-12 packaging creation.
7. Supplier creation and receipt of 24 stock units.
8. Cashier employee creation with a Cashier role.
9. Duplicate employee rejected without another account.
10. Invalid PIN rejected by real authentication.
11. Cashier login shows exactly the three permitted navigation sections.
12. Cashier opens a shift with R100.00 float.
13. R100.00 product/service sale with R150.00 tender and R50.00 change; product stock falls from 24 to 23.
14. Current-shift sales and receipt reopening.
15. Cashier blind close with R200.00 counted cash, without expected-cash or variance fields.
16. Closed-shift sales disappear from Cashier current-shift history.
17. Owner dashboard shows R100.00 gross sales; manual backup succeeds.
18. SQLite integrity, foreign keys, stock movements and users persist correctly.
19. Last active Owner cannot be deactivated.
20. Owner opens R500.00 float and refunds the earlier sale; stock returns to 24.
21. Waste entry reduces stock to 22; a count adjusts it to 20.
22. Split R10.00 cash/R20.00 card checkout persists two payments and reduces stock to 19.
23. Attempted sale of 20 units fails with insufficient stock; no partial sale or stock change occurs.
24. Owner voids the split-payment sale; stock returns to 20.
25. Expense category, R10.00 cash expense and reasoned expense void.
26. Owner closes with R400.00 and zero variance.
27. PIN reset and deactivation; deactivated Cashier cannot sign in.
28. Reactivation allows Cashier login with the new PIN.
29. Manual backup restored through its confirmation dialog; probe service disappears and the user is signed out.
30. Application process restarts; Owner login and UI-created catalogue persist.

## Evidence And Validation

Successful run: `target/ui-e2e/2026-09-30T22-46-54-792Z` (directory timestamp is UTC; the local test date is October 1).

- `results.json`: all 30 checks marked PASS.
- `1.png` through `30.png`: screenshots after each check.
- `data/business.db`: real database populated through UI operations.
- `data/Backups`: verified test backups, including restore safety copies.
- `npm test -- --run`: 7 frontend tests passed.
- `cargo test --workspace`: 169 Rust tests passed, including two new numbering regressions.
- `cargo test -p app --features e2e`: 5 command/session tests passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- Tauri test build and frontend build: passed.
- Test executable launched without `LOCALOPS_E2E_DATA_DIR`: exited with code 1 and "E2E data directory is required", confirming no production fallback.
- `npm run release:windows`: passed. Updated installer: `target/release/bundle/nsis/LocalOps POS_0.1.0_x64-setup.exe`.

Earlier harness attempts exposed selector, page-load and viewport-scrolling issues, which were corrected before the successful run. The receipt-number collision was an application defect, separately reproduced and repaired.

## Remaining Coverage

This is evidence for the listed workflows, not a claim that every control and integration works.

- Physical receipt printing, printer drivers, scanners and cash drawers were not exercised. Receipt rendering/reopening was verified.
- Stock transfers need two locations. This frontend does not expose UI forms to add further locations, departments or terminals, so those configurations were not inserted by bypassing the UI.
- Recipe/consumable entry, hierarchy edge cases, custom role creation/assignment, tax configuration and CSV download were not covered by this UI suite.
- Touch behavior, minimum-window/mobile layout, accessibility audits, offline network transitions and long-duration concurrency were not exhaustively tested.
- The installed executable was not replaced during these tests. The source and test build contain the numbering fix; the rebuilt installer is the delivery artifact.

## Repeating The Tests

From the repository root, install npm dependencies. Install `tauri-driver` with:

```powershell
cargo install tauri-driver --locked --root target/webdriver-tools
```

Place a [Microsoft Edge WebDriver](https://developer.microsoft.com/en-us/microsoft-edge/tools/webdriver/) matching the installed WebView2 version at `target/webdriver-tools/edge/msedgedriver.exe`. These tools are already present on this machine.

```powershell
npm run build:ui-test
npm run test:ui
```

The suite closes its test application after completion and preserves screenshots, results and the isolated database. It does not populate or reset the installed business database.
