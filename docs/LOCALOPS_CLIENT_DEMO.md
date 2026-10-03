# LocalOps Client Demo

Created on 2026-10-03 by typing and clicking through the real Tauri desktop UI
with Selenium WebDriver. No business records were inserted with SQL. Read-only
SQLite queries were used to verify results and safely resume the UI runner.

## Installed App

The normal installed release was updated and reopened. Its database is
`%LOCALAPPDATA%\LocalOps\POS\business.db`.

Business: **Thabo's Lifestyle Centre**.

Demo usernames and generated PINs are in `target/client-demo/Demo Login.txt`.
These are demonstration accounts, not the previous owner's credentials.
Sign in as the owner on Main Till to show the full workspace. The cashier
account can sign in on any of the three tills to demonstrate restricted access.

## Demo Contents

- Bar / Main Till, Food / Kitchen Till, Car Wash / Wash Bay Till.
- 14 products, 6 car-wash services, 7 categories, EA and ML units.
- Product prices, costs, barcodes, SKUs, department availability and crate packaging.
- One supplier and 8 paid stock purchases.
- Heineken low-stock example and Savanna out-of-stock example recorded as wastage.
- Owner and cashier accounts with separate permissions.
- Bar cash receipt R142.00, Food card receipt R133.00, Car Wash cash receipt R95.00.
- Closed Bar and Car Wash cashier shifts; open Food cashier and Bar owner shifts.
- Consumables expense R20.00 and a verified manual backup.

All departments use Main Store. Meal products are untracked sellables rather than
ingredient recipes. VAT remains disabled by the normal first-run setup defaults.
This is demonstration data in the installed app, not live client trading records.

## Screens And Checks

Use Point of Sale, Catalogue, Inventory, Shift & expenses, Reports, Employees,
and Backup & safety to show the populated workspace. Owner History includes
the three sample receipts. Screenshots and summary are in `target/client-demo`.

Verification: 31 isolated real-desktop UI workflow checks, 7 frontend tests,
and 166 core tests passed. The actual demo database passed SQLite quick_check
and foreign_key_check.
These checks cover the exercised workflows, not every possible UI combination.

## Reproduction

`npm run build:ui-test` creates the instrumented embedded-frontend test build.
`npm run test:ui` runs the regression suite against a separate test database.
`npm run demo:ui` populates the installed app's database through its real UI.
The demo runner refuses a different existing business and resumes this demo
using its local progress and credentials files. Keep those files to resume it.
It requires the existing tauri-driver and matching Edge driver in
`target/webdriver-tools`. Do not run it against live business data.

`npm run release:windows` builds the normal production executable and installer
without the test-only database override. The previous installed executable was
preserved as `target/pre-client-demo-installed-app.exe` before replacement.
