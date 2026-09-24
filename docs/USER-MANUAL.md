# LocalOps POS User Manual

**Software version:** 0.1.0  
**Manual edition:** 1.0  
**Platform:** Windows 10/11  
**Currency and default business timezone:** South African rand (ZAR), Africa/Johannesburg

This manual explains the controls and workflows that exist in the released LocalOps POS application. It is written for business owners, managers, cashiers, and stock-control employees. It does not describe planned features that are absent from the current interface.

## Contents

1. [What LocalOps POS does](#1-what-localops-pos-does)
2. [Important boundaries of this release](#2-important-boundaries-of-this-release)
3. [Installation and data storage](#3-installation-and-data-storage)
4. [First-run business setup](#4-first-run-business-setup)
5. [Signing in, sessions, and permissions](#5-signing-in-sessions-and-permissions)
6. [Workspace navigation](#6-workspace-navigation)
7. [Recommended initial configuration order](#7-recommended-initial-configuration-order)
8. [Catalogue: products and services](#8-catalogue-products-and-services)
9. [Setup tools](#9-setup-tools)
10. [Inventory](#10-inventory)
11. [Point of sale](#11-point-of-sale)
12. [Receipts, refunds, and voids](#12-receipts-refunds-and-voids)
13. [Shifts and expenses](#13-shifts-and-expenses)
14. [Reports](#14-reports)
15. [Backup and recovery](#15-backup-and-recovery)
16. [Daily operating checklists](#16-daily-operating-checklists)
17. [Troubleshooting](#17-troubleshooting)
18. [Data-protection rules](#18-data-protection-rules)
19. [Quick reference for employees](#19-quick-reference-for-employees)

---

## 1. What LocalOps POS does

LocalOps POS is an offline-first Windows application. Its main functions are:

- creating products and services;
- organising items into categories;
- assigning items and prices to departments;
- tracking stock using an append-only movement ledger;
- receiving purchases, transferring stock, recording losses, and completing stock counts;
- completing cash, card, EFT, other, and split-payment sales;
- printing the latest receipt through the Windows print dialog;
- recording refunds and full sale voids without deleting history;
- opening and reconciling cashier shifts;
- recording and voiding operating expenses;
- viewing business, department, payment, expense, shift, and stock reports;
- exporting reports as CSV files; and
- creating, verifying, and restoring local backups.

The application does not require an internet connection for normal operation. The SQLite database on the local computer is the source of truth.

## 2. Important boundaries of this release

These boundaries are part of the current interface and should be understood before deployment.

### 2.1 Owner and employee accounts

First-run setup creates one **Owner** account with all permissions. The current interface does **not** contain screens for creating or editing employees, roles, or permission assignments.

Employee sign-in and permission enforcement are implemented. If a deployment administrator has provisioned employee accounts, those employees can sign in with their own username and PIN. Business owners cannot currently create those accounts from the normal application interface.

Never solve this limitation by sharing the owner's PIN. Contact the LocalOps deployment administrator or software provider when an employee account must be added or changed.

### 2.2 Business structure

First-run setup creates one business, one department, one stock location, and one terminal. The current interface does not contain screens for adding or editing departments, locations, terminals, business tax settings, or payment methods.

The **Transfer stock** form needs at least two stock locations. It remains unavailable when only the first location exists. Additional locations must be provisioned by the deployment administrator.

### 2.3 Catalogue maintenance

The current interface can create categories, units, products, services, packaging, recipes, and department availability. It does not provide edit, deactivate, or delete controls for those records. Check entries carefully before submitting them.

### 2.4 Current transaction form limits

- Each inventory operation form records one product at a time.
- Purchase tax and purchase discount are currently recorded as zero by the interface.
- The interface does not attach receipt images to expenses.
- A receipt can be printed immediately after completing a sale. Historical sales can be opened for refunds or voids, but the current screen does not offer a historical **Reprint** button.
- Product minimum-stock thresholds are not editable in the current interface. New products use a minimum of zero unless separately provisioned.

### 2.5 Features not included

This release does not provide customer accounts, credit sales, lay-by, quotations, online synchronisation, ecommerce, payroll, or cloud login.

## 3. Installation and data storage

### 3.1 Requirements

- Windows 10 or Windows 11;
- Microsoft WebView2 Runtime; and
- permission to install an application for the current Windows user.

### 3.2 Install LocalOps POS

1. Obtain the installer from the approved LocalOps provider.
2. Double-click `LocalOps POS_0.1.0_x64-setup.exe`.
3. Follow the Windows installer prompts.
4. Start **LocalOps POS** from the Start menu or installed shortcut.

The installer may be unsigned in a pre-production deployment. If Windows displays a reputation warning, continue only when the installer came directly from the trusted provider and its checksum has been verified.

### 3.3 Where customer data is stored

Customer data and backups are stored separately from the application binaries:

```text
%LOCALAPPDATA%\LocalOps\POS
```

The main database is named `business.db`. Verified backups are stored in the `Backups` folder below the same data directory.

Older installations that used `%LOCALAPPDATA%\LocalOps POS` are migrated automatically on first launch. LocalOps uses SQLite's backup mechanism so committed WAL data is included, and it retains the old source as a safety copy.

### 3.4 Uninstalling

The normal uninstaller removes the application binaries and Windows registration but retains the customer database and backups. Create and verify a manual backup before uninstalling or upgrading anyway.

## 4. First-run business setup

When no business exists, LocalOps displays **FIRST-RUN SETUP — Build your local workspace**.

Complete every field:

| Field | What to enter |
|---|---|
| Business name | The legal or operating name used in LocalOps |
| Owner name | The name shown for the first owner account |
| First department | The first selling operation, such as `Bar`, `Restaurant`, or `Salon` |
| Main stock location | The first physical stock location, such as `Main Store` |
| Terminal name | A recognisable name for this computer or till, such as `Main Till` |
| Owner username | The name used to sign in, such as `owner` |
| Owner PIN | A numeric PIN containing 4–12 digits |

Then select **Create business**.

The operation is all-or-nothing. LocalOps creates the business, department, location, terminal, owner, full-permission Owner role, audit record, and active session together. If any part fails, it does not leave a half-created business.

After successful setup, the owner enters the workspace automatically.

Record the owner username and PIN securely. PINs are not displayed or recoverable from the application.

## 5. Signing in, sessions, and permissions

### 5.1 Sign in

On **WELCOME BACK — Sign in to this terminal**:

1. Select the **Business**.
2. Select the **Terminal**.
3. Enter the **Username**.
4. Enter the 4–12 digit **PIN**.
5. Select **Sign in**.

Five failed PIN attempts lock the account for five minutes. Wait before trying again and verify the selected business, terminal, and username.

### 5.2 Sign out

Select **Sign out** at the bottom of the navigation sidebar. Employees should sign out whenever they leave the till unattended.

### 5.3 Session expiry

Local sessions expire after five minutes without a successful application action. If an operation reports that the session is missing or expired, sign in again. Unsaved form entries may need to be re-entered.

### 5.4 Permissions

The owner created during first-run setup receives every permission. Provisioned employee accounts may have only selected permissions, including:

- business management;
- department, location, user, and role management;
- product and service management;
- inventory management;
- sale creation;
- payment recording;
- refunds and sale voids;
- shift management;
- report viewing; and
- expense management.

The sidebar may show a section even when the signed-in employee cannot perform its actions. LocalOps checks authority inside the Rust application service. A denied action displays a message such as `Permission denied: products.manage`.

## 6. Workspace navigation

The left sidebar contains:

| Section | Purpose |
|---|---|
| **Point of sale** | Open the till, sell items, print the latest receipt, and process reversals |
| **Catalogue** | Create products and services and view active catalogue items |
| **Setup tools** | Create categories and units; configure packaging, recipes, and department availability |
| **Inventory** | Suppliers, receiving, transfers, waste/damage, stock counts, balances, and movements |
| **Shifts & expenses** | Open or close the terminal shift, record expenses, and inspect history |
| **Reports** | View date-based performance and stock summaries and export CSV |
| **Backup & safety** | Check database health, create backups, and restore verified backups |

Green success notices confirm completed operations. Red notices explain errors or missing authority. Do not repeatedly click a submit button after an error; read the message and correct the cause first.

## 7. Recommended initial configuration order

Complete setup in this order:

1. Create the required categories in **Setup tools**.
2. Create the base measurement units in **Setup tools**.
3. Create products and services in **Catalogue**.
4. Add packaging for products purchased in packs, cases, crates, or other units.
5. Add recipes or consumables for items that consume ingredients when sold.
6. Make each sellable item available to the correct department.
7. Add suppliers in **Inventory**.
8. Receive opening stock or complete a stock count.
9. Open a shift.
10. Complete a controlled test sale and print its receipt.
11. Create a manual backup.

An item does not appear on the till until it has been made available to a department.

## 8. Catalogue: products and services

Open **Catalogue** to create an item and review active items.

### 8.1 Create a product

1. Select **Product** in the item-type list.
2. Enter the **Name**.
3. Enter the selling **Price (ZAR)**, for example `25.00`.
4. Select a **Category**, or leave it as **No category**.
5. Select the product's **Base unit**.
6. Enter the current **Cost (ZAR)**. Receiving stock later updates the persisted latest base-unit cost.
7. Optionally enter a **Barcode**, **SKU**, and **Product code**.
8. Leave **Track stock** enabled if sales and inventory operations must affect its balance.
9. Set **Taxable** as required by the business configuration.
10. Select **Add product**.

Barcode, SKU, and product code values must be unique inside the business. Blank optional identifiers are stored as empty/unassigned values rather than duplicate codes.

### 8.2 Create a service

1. Select **Service**.
2. Enter its name, price, and optional category.
3. Optionally enter the duration in minutes.
4. Set **Taxable** as required.
5. Select **Add service**.

Services do not have a stock balance, but a service can consume stock through a recipe. For example, a cleaning service can consume detergent.

### 8.3 Review the item list

The catalogue table shows:

- item name and category;
- product or service type;
- price;
- barcode, SKU, and product-code lookup values;
- stock tracking or service duration; and
- packaging count and recipe status.

## 9. Setup tools

### 9.1 Categories

Categories organise the catalogue and reports.

1. Enter the category **Name**.
2. Optionally select a **Parent category**.
3. Select **Add category**.

### 9.2 Units

Units use exact rational conversion factors. Each unit has:

- **Code**, such as `EA`, `G`, `KG`, or `L`;
- **Name**;
- **Dimension**: `COUNT`, `WEIGHT`, `VOLUME`, `LENGTH`, or `TIME`;
- **Scale numerator** and **Scale denominator** relative to the dimension's chosen canonical unit; and
- allowed **Decimal places**, from 0 to 6.

Safe examples:

| Unit | Dimension | Numerator | Denominator | Decimal places |
|---|---:|---:|---:|---:|
| Each (`EA`) | COUNT | 1 | 1 | 0 |
| Gram (`G`) as the weight base | WEIGHT | 1 | 1 | 3 |
| Kilogram (`KG`) when gram is the base | WEIGHT | 1000 | 1 | 3 |

Use the same dimension for units that must convert into one another. LocalOps rejects incompatible or mathematically inexact conversions.

### 9.3 Product packaging

Packaging defines a product-specific conversion to the product's base stock unit.

Example: a crate contains 12 bottles, while the product's base unit is one bottle.

1. Select the **Product**.
2. Select the **Package unit**.
3. Enter a clear **Name**, such as `Crate of 12`.
4. Enter `12` as the **Base-unit numerator**.
5. Enter `1` as the **Base-unit denominator**.
6. Select **Add packaging**.

Receiving `2` crates then adds `24` base units to stock. Configured packaging is available in the **Receive purchase** form. The current till sells whole catalogue quantities and does not display a package selector.

### 9.4 Recipe or consumable

A recipe deducts ingredient stock when its owner item is sold.

1. Select the **Sold item**. This can be a product or service.
2. Enter the **Recipe yield**. Use `1` when the listed ingredients produce one sold unit.
3. Select an ingredient product.
4. Enter its quantity and unit.
5. Use **+ Add ingredient** for additional ingredients.
6. Select **Save recipe**.

Quantities support up to six decimal places. Use compatible units. Saving a new recipe replaces the active recipe for that sold item while historical sales retain their original snapshots.

### 9.5 Department availability

1. Select the **Department**.
2. Select the **Item**.
3. Optionally enter a **Price override** for that department.
4. Select **Make available**.

Only active department items appear in **Point of sale**. A price override changes the till price for that department without changing the catalogue's base price.

## 10. Inventory

Open **Inventory — Stock control**. The top area shows tracked-product count, product/location balances, and reconciliation differences.

### 10.1 Read the stock tables

The balance table shows each tracked product's location, on-hand quantity, base unit, and cache version. The recent-movements table is the append-only history behind those balances.

- Positive quantities add stock.
- Negative quantities remove stock.
- The movement type and reference identify the operation that caused the change.

`Reconciliation differences` should be `0`. A non-zero value means a cached balance does not agree with its movement ledger; see [Troubleshooting](#17-troubleshooting).

### 10.2 Add a supplier

1. Enter the supplier **Name**.
2. Optionally enter a phone number.
3. Select **Add supplier**.

Suppliers remain linked to purchase history.

### 10.3 Receive a purchase

1. Enter a unique **Purchase number**.
2. Select a supplier or **No supplier**.
3. Select the location under **Receive into**.
4. Select the product.
5. Select the base unit or configured **Purchase unit** packaging.
6. Enter the purchased **Quantity**.
7. Enter **Unit cost (ZAR)** for the selected purchase unit.
8. Optionally enter an invoice reference.
9. Select `Unpaid`, `Part paid`, or `Paid` as the payment status.
10. Select **Receive stock**.

LocalOps creates the received purchase, updates the product's latest base-unit cost, posts the inventory movement, and updates the balance in one transaction. If any validation fails, none of those changes are saved.

### 10.4 Transfer stock

The transfer form requires two configured locations.

1. Select the **From** location.
2. Select a different **To** location.
3. Select the product.
4. Enter the quantity in its displayed base unit.
5. Select **Complete transfer**.

The outgoing and incoming movements post together. LocalOps rejects a transfer that would make the source stock negative.

### 10.5 Record waste or damage

1. Select the location and product.
2. Enter the quantity lost.
3. Enter a clear reason.
4. Enable **Physical damage** when applicable.
5. Select **Record loss**.

The stock reduction and reason remain auditable.

### 10.6 Complete a stock count

1. Select the location and product.
2. Enter the actual **Counted quantity**.
3. Select **Complete count**.

LocalOps preserves the expected quantity, records the counted quantity, and posts only the variance needed to bring stock to the count. A zero-variance count is still recorded.

### 10.7 Negative stock

Normal operations cannot reduce tracked stock below zero. If a sale, transfer, or loss is rejected for insufficient stock:

1. verify the selected product and location;
2. receive missing stock or complete an authorised stock count; and
3. retry the original operation only after the balance is correct.

Do not create false receiving transactions merely to bypass stock controls.

## 11. Point of sale

### 11.1 Open the till

Sales require an open shift for the selected terminal.

1. Open **Point of sale**.
2. If the header shows **Shift required**, count the starting cash in the drawer.
3. Enter it under **Opening cash**.
4. Select **Open shift**.

The shift can also be opened from **Shifts & expenses**. Only one open shift is allowed for a terminal.

### 11.2 Add items by clicking

Available items appear as buttons with department, name, and price. Select an item to add one unit. Selecting it again increases its quantity.

In the cart:

- select `+` to increase the quantity;
- select `−` to decrease it; and
- reducing a line below one removes it.

### 11.3 Search or scan

The search field accepts item names, barcode values, SKUs, and product codes.

- Typing part of a name or identifier filters the item buttons.
- Pressing **Enter** or selecting **Add item** performs an exact barcode/SKU/product-code match and adds that item.
- A USB barcode scanner should be configured as keyboard input and send Enter after the scanned text.
- Barcode resolution is entirely local; it does not contact an online barcode service.

If `No local item matches` appears, confirm that the identifier was saved on the product and that the product was made available to a department.

### 11.4 Take payment

The default payment methods are **Cash**, **Card**, **EFT**, and **Other**.

For a single cash payment:

1. Select **Pay full cash**.
2. If the customer gives more than the total, replace **Cash tendered** with the actual amount received.
3. Confirm the allocation is marked **Balanced**.
4. Select **Complete sale**.

For a single non-cash payment:

1. Enter the exact sale total beside the method.
2. Confirm **Balanced**.
3. Select **Complete sale**.

For split payment:

1. Enter the amount for each payment method.
2. For a cash portion, enter the actual cash under **Cash tendered**.
3. Make sure the allocated total exactly equals the sale total.
4. Select **Complete sale**.

The button remains disabled if the cart is empty, no shift is open, or payment is not balanced. Non-cash overpayment is rejected. Cash tender can exceed the cash allocation and produces change.

### 11.5 What happens when a sale completes

LocalOps saves the sale, immutable item descriptions and prices, payment records, stock/recipe consumption, shift cash effect, and audit history together. A failure rolls back the entire sale so a partial transaction is not left behind.

## 12. Receipts, refunds, and voids

### 12.1 Print the latest receipt

After a successful sale, **Latest receipt** appears below the till.

1. Review the receipt number, time, items, subtotal, tax, total, payments, and change.
2. Select **Print receipt**.
3. Choose the printer in the Windows print dialog.
4. Confirm print settings and print.

Printing is handled by Windows. If a printer is unavailable, the sale is still safely recorded. Print the receipt before navigating away because historical reprinting is not exposed in the current interface.

### 12.2 Open a recent sale

The **Recent sales & reversals** table lists receipt number, time, status, and total. Select **Open** to display reversal allocation controls.

Refunds and voids require an open shift and the relevant employee permission.

### 12.3 Partial refund

1. Open the sale.
2. Enter a required **Reason**.
3. Under **Items**, enter only the quantity being refunded. Set unwanted lines to `0`.
4. Enable **Return to stock** only when a physical tracked product has actually returned in saleable condition.
5. Under **Payment reversals**, allocate the refund to the original payment records. Set unused allocations to `0.00`.
6. Select **Record refund**.

Refunded quantities and payment amounts cannot exceed what remains refundable on the original sale. LocalOps records append-only reversal events and never rewrites the original receipt.

### 12.4 Full void

1. Open the sale.
2. Enter a reason.
3. Review all item quantities, stock-return choices, and payment allocations.
4. Select **Void full sale**.

A full void reverses the sale through traceable payment, inventory, shift, and audit entries. It does not delete the original sale.

## 13. Shifts and expenses

Open **Shifts & expenses**.

### 13.1 Shift figures

When a shift is open, the page shows:

- **Opening cash** — drawer amount recorded at the start;
- **Expected cash now** — opening cash plus cash sales, less cash refunds and cash expenses; and
- **Shift opened** — start time.

### 13.2 Create an expense category

1. Enter the category name, such as `Transport`, `Cleaning`, or `Repairs`.
2. Select **Create category**.

At least one category and an open shift are required before recording an expense.

### 13.3 Record an expense

1. Select the category.
2. Select a department, or **Business-wide**.
3. Select the payment method, or **Not specified**.
4. Enter the amount.
5. Confirm the date.
6. Enter a useful description.
7. Optionally enter a reference.
8. Select **Record expense**.

A cash expense immediately reduces the shift's expected drawer balance. Other payment methods do not change expected cash.

### 13.4 Void an expense

1. Find a `recorded` entry in **Expense history**.
2. Select **Void**.
3. Enter the reason in the prompt.
4. Confirm the action.

The original expense remains visible with `void` status. If it was a cash expense on the still-open shift, the expected cash effect is reversed.

### 13.5 Close and reconcile a shift

1. Stop processing transactions on the terminal.
2. Count the actual cash in the drawer.
3. Enter **Actual cash counted**.
4. Add optional closing notes.
5. Select **Close shift**.

LocalOps stores the expected amount, actual amount, and variance. A positive variance means more cash was counted than expected; a negative variance means less cash was counted than expected.

The **Shift history** table preserves opening, expected, actual, and variance figures.

## 14. Reports

Open **Reports — Business dashboard**. Report access requires the `reports.view` permission.

### 14.1 Choose a date range

1. Select **From** and **To** dates.
2. Select **Refresh**.

The default range begins on the first day of the current month and ends today.

### 14.2 Summary figures

The dashboard displays:

- net sales;
- estimated gross profit;
- expenses;
- refunds and voids;
- current stock value;
- low-stock balance count;
- gross sales; and
- shift variance.

Estimated gross profit uses persisted sale values and product costs; it should not be treated as a full accounting profit statement.

### 14.3 Detailed tables

- **Departments** shows gross sales, refunds, net sales, and estimated gross profit.
- **Payments** shows received, refunded, and net totals by method.
- **Current stock valuation** shows on-hand quantity, minimum quantity, and latest-cost valuation per location.

### 14.4 Export CSV

1. Choose the required date range.
2. Select **Export CSV**.
3. Save the file when Windows prompts, or find it in the configured downloads location.

The filename follows this pattern:

```text
localops-report-YYYY-MM-DD-to-YYYY-MM-DD.csv
```

## 15. Backup and recovery

Only an account with `business.manage` permission can open **Backup & safety** successfully.

### 15.1 Read the health indicators

A healthy system should show:

| Indicator | Healthy value |
|---|---|
| SQLite integrity | `ok` |
| Foreign-key violations | `0` |
| Stock differences | `0` |
| Unaudited financial events | `0` |
| Schema | Current version equals latest version |
| Overall state | `Healthy` |

Do not ignore a red indicator or `Review` state. Create a manual backup if possible, stop non-essential writes, record the exact values, and contact support.

### 15.2 Automatic backups

LocalOps verifies and retains up to:

- 7 daily backups;
- 4 weekly backups; and
- 12 monthly backups.

Automatic retention is maintained during database startup. Manual and pre-restore backups are kept separately from those scheduled groups.

### 15.3 Create a manual backup

1. Open **Backup & safety**.
2. Confirm that the health state is acceptable.
3. Select **Create manual backup**.
4. Wait for `Verified backup created`.
5. Confirm the new row says **Verified**.

### 15.4 Restore a backup

Restoring replaces the active database state with the chosen backup.

1. Stop transaction activity on the computer.
2. Open **Backup & safety**.
3. Find the required verified backup.
4. Select **Restore**.
5. Read the confirmation and approve it.

Before restoring, LocalOps creates a verified `pre-restore` safety copy. After a successful restore, the current session ends and the application reloads. Sign in using credentials that existed at the time of the restored backup.

The **Restore** control is disabled for an invalid/unverified backup.

### 15.5 External protection

The current interface does not export backups to removable media. For disaster recovery against computer loss, an authorised administrator should periodically copy a verified backup to encrypted external storage while LocalOps is closed. Do not copy the live `business.db`, `-wal`, or `-shm` files as a substitute for a verified backup.

## 16. Daily operating checklists

### 16.1 Opening checklist

- [ ] Start LocalOps POS.
- [ ] Select the correct business and terminal.
- [ ] Sign in with your own account.
- [ ] Confirm there are no red database or stock warnings relevant to your role.
- [ ] Count the opening drawer cash.
- [ ] Open the shift with the correct opening amount.
- [ ] Confirm required products appear on the till.
- [ ] Confirm tracked stock has been received or counted.
- [ ] Test the receipt printer if printing is required.

### 16.2 Sale checklist

- [ ] Confirm the shift is open.
- [ ] Add or scan the correct items.
- [ ] Verify item quantities and total.
- [ ] Allocate the exact payment amount.
- [ ] Enter actual cash tendered when applicable.
- [ ] Confirm **Balanced**.
- [ ] Select **Complete sale** once.
- [ ] Give the displayed change.
- [ ] Print the latest receipt before leaving the screen.

### 16.3 Closing checklist

- [ ] Finish or reverse outstanding transactions.
- [ ] Record all operating expenses.
- [ ] Review expense history.
- [ ] Review the day's reports if authorised.
- [ ] Count the actual drawer cash.
- [ ] Close the shift and review the variance.
- [ ] Investigate and note unexplained variance.
- [ ] Create a verified manual backup according to business policy.
- [ ] Sign out.

## 17. Troubleshooting

### The application says the local database could not be opened

- Close other copies of LocalOps.
- Confirm the Windows user can access `%LOCALAPPDATA%\LocalOps\POS`.
- Do not rename or manually edit the database files.
- Record the full error and contact support.

### I cannot sign in

- Confirm the correct business and terminal are selected.
- Check the username spelling.
- PINs contain 4–12 digits.
- After five failed attempts, wait five minutes before retrying.
- Ask the deployment administrator to confirm that the account and terminal are active.

### An action says `Permission denied`

The signed-in account does not have the required permission. Sign out and use an authorised account, or ask the deployment administrator to change the employee's assigned role. Do not share the owner's PIN.

### No items appear on the till

- Create the item in **Catalogue**.
- Open **Setup tools** and make it available to a department.
- Confirm the item is active and the selected business/terminal is correct.

### A barcode does not add an item

- Confirm the product has that barcode, SKU, or product code.
- Make the item available to a department.
- Confirm the scanner sends Enter after the value.
- Test by typing the exact value and pressing Enter.

### Complete sale is disabled

- Open a shift.
- Add at least one item.
- Make the payment allocation equal the displayed total.
- Correct invalid numeric entries.

### A stock operation or sale reports insufficient stock

- Check the product and location balance.
- Receive missing stock or perform an authorised physical stock count.
- Confirm recipe ingredients have sufficient balances.
- Retry only after the ledger is correct.

### Transfer stock is disabled

At least two locations and one tracked product are required. The current interface cannot add a second location; contact the deployment administrator.

### Expense recording is disabled

Open a shift and create at least one expense category.

### A refund or void fails

- Open a shift.
- Enter a reason.
- Confirm the account has refund or void permission.
- Do not exceed remaining refundable item quantities or payment amounts.
- Allocate payment reversals to the original payment records.

### Receipt printing fails

- Confirm Windows can see the printer.
- Check paper, power, cable/network connection, and Windows print queue.
- Reopen the Windows print dialog and select the correct printer.
- Remember that the completed sale remains saved even when printing fails.

### Report totals look unexpected

- Confirm the From/To dates.
- Review refunds, voids, expenses, and shift variance separately.
- Check that items were assigned to the intended departments.
- Remember that stock valuation uses latest persisted product costs.

### Backup health says `Review`

- Avoid unnecessary transactions.
- Create a manual backup if the control remains available.
- Record every non-zero health indicator and any invalid backup names.
- Contact support before restoring or modifying files manually.

## 18. Data-protection rules

1. Do not put the SQLite database on a shared network drive.
2. Do not open or edit `business.db` with spreadsheet or database tools.
3. Do not delete `business.db-wal` or `business.db-shm` while LocalOps is running.
4. Use **Backup & safety** to create a consistent backup.
5. Close LocalOps before an administrator copies backups to external storage.
6. Keep the owner PIN private and use individual employee accounts when provisioned.
7. Sign out when leaving a terminal unattended.
8. Do not power off the computer during sale completion, restore, or upgrade.
9. Verify a manual backup before major configuration changes or software upgrades.
10. Treat exported CSV reports and external backup copies as confidential business data.

## 19. Quick reference for employees

### Cashier

1. Sign in to the correct terminal.
2. Open the shift with counted opening cash.
3. Add or scan items.
4. Confirm quantity and total.
5. Allocate payment until **Balanced**.
6. Complete the sale once.
7. Give change and print the latest receipt.
8. Ask an authorised employee to handle refunds or voids when you lack permission.
9. Count cash and close the shift at the end of duty.
10. Sign out.

### Stock controller

1. Sign in and open **Inventory**.
2. Confirm reconciliation differences are zero.
3. Add suppliers when required.
4. Receive stock using a unique purchase number.
5. Use transfers only between correctly configured locations.
6. Record every waste/damage event with a reason.
7. Complete physical counts and investigate variances.
8. Never use false stock receipts to bypass negative-stock protection.

### Owner or manager

1. Review catalogue and department availability.
2. Monitor stock balances and movement history.
3. Review refunds, voids, expenses, and shift variances.
4. Review date-range reports and export CSV when required.
5. Check **Backup & safety** regularly.
6. Create verified manual backups according to business policy.
7. Contact the deployment administrator for employee, role, location, department, terminal, tax, or payment-method administration that is not exposed in this release.

---

## Support information to record

When reporting a problem, provide:

- LocalOps POS version (`0.1.0` for this manual);
- Windows version;
- business and terminal selected;
- signed-in role, but never the PIN;
- exact red error text;
- the operation attempted;
- relevant receipt, purchase, or reference number;
- time the issue occurred; and
- the **Backup & safety** health values, if accessible.

Never send a live customer database or backup through an unapproved channel.
