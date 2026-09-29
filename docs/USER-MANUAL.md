# LocalOps POS — User Manual

LocalOps POS runs entirely on this computer. There is no cloud account, no
internet sign-in and no remote server. Everything in this manual describes
behaviour that is implemented and enforced by the application itself.

This manual covers the two roles that exist in this release: **Owner** and
**Cashier**.

---

## 1. First run: creating the business

The first time the app opens, it shows **first-run setup**. You enter:

* the business name,
* the first department (for example *Bar* or *Shop*),
* the main stock location,
* the first terminal (till) name,
* the owner username, display name and PIN (4–12 digits).

Pressing **Create business** creates all of it in one step and signs you in as
the Owner. First-run setup can only run once: after a business exists the app
shows the sign-in screen instead, and any further attempt to run setup is
refused by the backend.

**Keep the Owner PIN safe.** It is the only account that can create employees,
reset PINs and restore backups.

---

## 2. Signing in

The sign-in screen asks for the business, the terminal, a username and a PIN.

* The terminal you choose becomes your **session terminal** for as long as you
  are signed in. It cannot be changed without signing out and back in.
* If the terminal belongs to a department, that department becomes your
  **session department**.
* Only active terminals are offered.
* Repeated wrong PINs temporarily lock the account. An Owner can clear the lock
  by reactivating the employee or resetting the PIN.

Signing in on a terminal ends any previous session that was still recorded for
that app instance.

---

## 3. The Cashier workspace

A Cashier sees exactly four things in the sidebar:

| Section | What it does |
| --- | --- |
| **Point of Sale** | Sell items and take payment |
| **My Shift** | Open your shift, close your shift |
| **Recent Sales** | Your sales in the shift you currently have open, and receipt reprints |
| **Sign Out** | End your session |

There is nothing else to find. Costs, profit, stock value, supplier details,
expenses, reports, backups, other people's sales and employee administration
are not merely hidden in the interface — the backend refuses to send them to a
Cashier at all.

### 3.1 Opening your shift

Go to **My Shift**, enter the cash you are starting with, and press
**Open shift**.

* You open a shift for *yourself*, on the terminal you signed in on.
* If somebody else already has an open shift on this till, you cannot open one
  and you cannot take over theirs. They must close it, or the Owner must close
  it for them.

### 3.2 Selling

In **Point of Sale**:

1. Tap items, or scan/type a barcode, SKU or product code and press enter.
2. Adjust quantities in the ticket.
3. Enter the amounts per payment method (there is a one-tap *full cash* helper),
   then complete the sale.

Notes:

* You can only sell items available to **your** department.
* A sale cannot be completed unless **you** have an open shift on this till.
* Discounts and manual price overrides are not available to a Cashier. If the
  price must change, the Owner must do it.
* Refunds and voids are not available to a Cashier. The screen says so instead
  of showing buttons that would fail.

### 3.3 Recent sales and reprints

**Recent Sales** lists only the sales you rang up during the shift you currently
have open, on this till. Selecting one shows the receipt and lets you print it
again.

When your shift is closed, that list is empty — a closed shift's sales are no
longer reprintable from the till. This is deliberate.

### 3.4 Closing your shift (blind close)

Go to **My Shift**, count the drawer, type the amount you actually counted, add
a note if you want, and press **Close my shift**.

You are **not** shown what the till expected, and you are **not** shown the
difference. The application still records the expected amount and the variance
permanently — they are shown to the Owner. This is called a *blind close*, and
it exists so that a count cannot be adjusted to match an expected figure.

After closing you can open a new shift when you next need one.

---

## 4. The Owner workspace

The Owner has every capability in the system:

| Section | What it does |
| --- | --- |
| **Point of Sale** | The full catalogue, plus discounts, refunds and voids |
| **Catalogue** / **Setup tools** | Products, services, categories, units, packaging, recipes, department availability |
| **Inventory** | Stock on hand, movements, transfers, wastage, stock counts, suppliers, receiving |
| **Shift & expenses** | Every shift with expected cash and variance; expense categories and expenses |
| **Reports** | Sales, profit, payments, stock, and CSV export |
| **Employees** | Employee accounts, PIN resets, activation, role assignment |
| **Backup & safety** | Database health, manual backups, restore |

### 4.1 Managing employees

**Employees** shows everybody in the business with their username, role and
status.

* **Add employee** — username, display name, PIN and role. New employees default
  to the **Cashier** role.
* **Reset PIN** — sets a new PIN. The employee is signed out of every terminal
  immediately.
* **Deactivate / Reactivate** — a deactivated employee cannot sign in, and is
  signed out of every terminal immediately. Reactivating also clears a lockout.
* **Role** — changing an employee's role signs them out immediately so the new
  authority applies from their next sign-in. Role assignments live in the
  employee record itself, not in a cached copy.

You cannot deactivate or demote the last active Owner. Create a second Owner
first if you need to hand the business over.

Employee records never expose PIN hashes.

### 4.2 Supervising shifts

**Shift & expenses** shows the open shift and recent shift history, with
opening cash, expected cash, counted cash and the variance, plus who opened the
shift and who closed it.

If a Cashier leaves without closing, the Owner can close that shift from this
screen. The shift keeps its original owner — only the "closed by" record shows
the Owner — so the history stays honest.

### 4.3 Refunds and voids

Refunds and voids are Owner capabilities and are performed from the Point of
Sale history. If a Cashier's shift is open on the till, the reversal is recorded
against that open shift so the drawer maths stays correct, but the shift still
belongs to the Cashier and the audit trail records the Owner as the actor.

### 4.4 Backups

**Backup & safety** reports the schema version, integrity checks, inventory
reconciliation and audit coverage, and lists local backups. Creating a manual
backup can be delegated with the backup permission; **restoring** a backup is
restricted to the Owner account in this release, and it signs the current
session out.

---

## 5. What each role may do

| Capability | Owner | Cashier |
| --- | --- | --- |
| See the POS catalogue | Whole business | Own department only |
| Create sales, take payments | Yes | Yes |
| Discounts, price overrides | Yes | No |
| Refunds, voids | Yes | No |
| See sales | All | Own, current open shift only |
| Reprint receipts | Any sale | Own, current open shift only |
| Open a shift | Yes | Own, on the session terminal |
| Close a shift | Own and anyone's | Own only, blind |
| See expected cash / variance | Yes | No |
| Products, costs, inventory, suppliers, purchases | Yes | No |
| Expenses | Yes | No |
| Reports and CSV export | Yes | No |
| Employees, PINs, roles | Yes | No |
| Backups | Yes (restore: Owner only) | No |

---

## 6. Security boundary you should understand

LocalOps keeps everything on this device. That is a feature, and it has limits
you should plan around:

* **PINs are hashed.** They are not recoverable; an Owner can only reset them.
* **Authorization is enforced in the application service layer**, not in the
  screens. Hiding a button is a convenience; the refusal happens in the backend.
* **The database file itself is not encrypted at rest.** Anyone with operating
  system access to the computer, or to an unencrypted copy of the backups
  folder, can read the raw file with ordinary SQLite tools. Protect the device
  the way you would protect a cash drawer: a password-protected Windows account,
  full-disk encryption (BitLocker) if your hardware supports it, a locked room,
  and backups kept on media you control.
* **Roles are not a substitute for supervision.** Blind close, immutable
  financial history and the audit trail are there so that mistakes and
  irregularities are visible after the fact.

---

## 7. Known limitations in this release

* Only two roles ship as system roles: **Owner** and **Cashier**. There is no
  Manager or Supervisor workspace, and no manager-PIN override at the till.
* There is no custom role designer or permission editor in the interface.
  Custom roles created directly in the data layer are honoured by the backend.
* Employees are not assigned to specific departments or specific terminals. A
  Cashier may sign in at any active terminal in the business; their scope then
  follows that terminal's department.
* Scope cannot be switched mid-session. Sign out and sign in again to change
  terminal.
* The Cashier's recent-sales list and receipt reprints cover the current open
  shift only.
* The database is not encrypted at rest (see section 6).
