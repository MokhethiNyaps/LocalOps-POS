# MASTER ARCHITECTURE & DATABASE STANDARD OF TRUTH

## Purpose

You are building a **local-first, offline business management and POS application** for small and medium-sized businesses.

The application must work completely without internet access and without requiring any cloud database, SaaS backend, Supabase, Firebase, or external payment API.

The initial target market is businesses such as:

Bars

Restaurants

Car washes

Taverns

Takeaway businesses

Bottle stores

Small retail businesses

Businesses combining multiple operations under one business

Hospitality/lifestyle businesses

A particularly important use case is a business that operates **multiple types of businesses under one umbrella**, for example:

A property/business has a bar + restaurant + car wash.

This architecture must support that scenario natively.

The system must therefore NOT be designed as:

Bar software

OR

Restaurant software

OR

Car wash software

It must instead be designed as:

BUSINESS

↓

DEPARTMENTS / OPERATIONS

↓

PRODUCTS / SERVICES / RECIPES

↓

INVENTORY / SALES / PAYMENTS

↓

REPORTING

The application should be configurable enough that a future business type can be added without redesigning the database.

# 1\. CORE ARCHITECTURAL PRINCIPLE

The system is a **generic business operations engine**, not a collection of hard-coded industry applications.

The database must model universal business concepts.

Examples:

A beer is a product.

A chicken burger is a sellable product made from ingredients.

A car wash is a service.

A haircut could also be a service.

A room booking could eventually be a service.

A crate of beer is a purchasing/stock unit.

A bottle of beer is a selling/stock unit.

Shampoo is inventory.

Cash is a payment method.

Card is a payment method.

EFT is a payment method.

Bar, Restaurant and Car Wash are departments/operations.

Do not create tables such as:

bar\_products

restaurant\_products

carwash\_products

unless there is an extremely strong technical reason.

Instead use generic tables:

products

services

categories

inventory

inventory\_locations

departments

and connect them using relationships.

# 2\. OFFLINE-FIRST REQUIREMENT

The application must function fully offline.

The primary database must be local.

Use:

**SQLite**

unless there is a documented technical reason not to.

The application must NOT require:

Internet connection

Supabase

Firebase

Cloud PostgreSQL

Remote API

Payment gateway

Authentication server

for normal operation.

The customer should be able to:

Install the application.

Create a business.

Configure departments.

Add products/services.

Take sales.

Record payments.

Deduct stock.

Perform stock counts.

Close shifts.

View reports.

with the internet completely disconnected.

# 3\. LOCAL DATABASE

The SQLite database should be treated as the customer's primary source of truth.

Conceptually:

Application

↓

Business Logic

↓

SQLite

Do not design the application so that the UI directly manipulates database records everywhere.

Use a clean data-access/service layer.

The database should live in an application-data directory, NOT inside the executable/application installation directory.

Conceptually:

Application/

executable

application files

User Data/

business.db

Backups/

backup-YYYY-MM-DD.db

Application updates must never overwrite the customer's business database.

# 4\. BUSINESS HIERARCHY

The fundamental hierarchy is:

BUSINESS

│

├── DEPARTMENTS

│

├── LOCATIONS

│

├── USERS / EMPLOYEES

│

├── PRODUCTS

│

├── SERVICES

│

├── INVENTORY

│

├── SALES

│

├── PAYMENTS

│

├── EXPENSES

│

└── REPORTS

Example:

Moko's Lifestyle Centre

│

├── Bar

│

├── Restaurant

│

└── Car Wash

This is ONE business.

Do not create three independent businesses in the database merely because they operate differently.

# 5\. BUSINESS

The businesses entity represents the legal/operational business using the application.

Example:

business

id: 1

name: Moko's Lifestyle Centre

currency: ZAR

timezone: Africa/Johannesburg

The system should support configurable:

Business name

Trading name

Address

Contact details

Currency

Tax/VAT configuration

Receipt information

Logo

Business settings

The initial default currency should support **South African Rand (ZAR)**.

Do not hard-code ZAR throughout the application.

Currency must be configurable.

# 6\. DEPARTMENTS / OPERATIONS

A department represents an operational part of a business.

Examples:

Bar

Restaurant

Car Wash

Bottle Store

Takeaway

Events

Accommodation

Salon

Retail

A department is NOT a separate business.

Example:

business\_id = 1

department 1:

Bar

department 2:

Restaurant

department 3:

Car Wash

Departments may have:

Name

Description

Active/inactive status

Default selling configuration

Default inventory location

Assigned employees

Assigned POS terminals

Department-specific reports

Do not create an enum such as:

business\_type = BAR | RESTAURANT | CAR\_WASH

as the primary architecture.

A business can have zero, one, or many departments.

# 7\. LOCATIONS

Departments and physical inventory locations are related but NOT identical.

A business may have:

Main Store

Bar Store Room

Restaurant Kitchen

Car Wash Store Room

These are inventory locations.

A department can have a default inventory location, but the database must not assume that every department equals a stock location.

Example:

Business

│

├── Department: Bar

│

├── Department: Restaurant

│

├── Department: Car Wash

│

└── Inventory Locations

├── Main Store

├── Bar Store Room

├── Kitchen

└── Car Wash Store

This allows stock transfers.

# 8\. PRODUCTS AND SERVICES

The system must distinguish between:

### PRODUCTS

Physical or inventory-related sellable things.

Examples:

Beer

Whisky

Coke

Chicken

Burger

Fries

Shampoo

Clothing

Electronics

### SERVICES

Non-inventory activities.

Examples:

Basic Car Wash

Premium Car Wash

Interior Cleaning

Haircut

Consultation

Room Cleaning

However, the selling system should treat both products and services as **sellable items**.

The POS should not need completely different sales logic.

Conceptually:

Sellable Item

├── Product

└── Service

# 9\. PRODUCT CONFIGURATION MUST BE EXTREMELY EASY

A business owner should be able to add a product without understanding databases, inventory theory, accounting, or recipes.

The UI should support a simple form:

ADD PRODUCT

Name:

Category:

Selling Price:

Cost Price:

Track Stock? YES / NO

Stock Unit:

Initial Quantity:

Minimum Stock:

\[ SAVE \]

Advanced configuration can be optional.

Do not force users to fill out 20 fields just to create a product.

# 10\. PRODUCT MUST BE GENERIC

Do NOT assume that a product is:

Beer

Food

Clothing

Retail merchandise

A product should be generic.

For example:

Product:

Castle Lager 340ml

Category:

Beer

Selling Price:

25.00

Cost Price:

16.00

Stock Tracked:

YES

Stock Unit:

Bottle

Minimum Stock:

24

Another business could create:

Product:

50kg Cement

Category:

Building Materials

Selling Price:

R180

Cost:

R140

Stock Unit:

Bag

The same database handles both.

# 11\. CATEGORIES

Products should use configurable categories.

Examples:

Bar:

Beer

Spirits

Wine

Soft Drinks

Restaurant:

Starters

Mains

Sides

Desserts

Car Wash:

Chemicals

Consumables

Retail:

Clothing

Shoes

Electronics

Categories belong to a business and may optionally be associated with a department.

Do not hard-code categories.

# 12\. UNITS OF MEASURE

The inventory system must support different units.

Examples:

Piece

Bottle

Can

Box

Crate

Pack

Kilogram

Gram

Litre

Millilitre

Metre

Centimetre

Hour

Service

Do not assume everything is counted as quantity = integer.

A restaurant may need:

Chicken:

kilograms

Cooking Oil:

litres

Sauce:

millilitres

A bar may need:

Beer:

bottles

Whisky:

litres

A car wash may need:

Shampoo:

litres

# 13\. UNIT CONVERSIONS

The system should eventually support conversions.

Example:

1 crate = 12 bottles

1 case = 24 cans

1 litre = 1000 millilitres

1 kilogram = 1000 grams

This is extremely important.

A supplier may sell:

Castle Lager

10 crates

while the business sells:

Castle Lager

1 bottle

The database must allow purchasing units and selling/stock units to differ.

Do not simply store "quantity" without understanding the unit.

# 14\. PRODUCT PACKAGING

Products may have multiple units or packaging configurations.

Example:

Castle Lager

Base Stock Unit:

Bottle

Packaging:

Crate = 12 bottles

Case = 24 bottles

Purchasing:

5 crates

should be convertible to:

60 bottles

if the business configures that relationship.

Do not automatically assume all crates contain the same quantity.

Packaging configuration must be data.

# 15\. PRODUCT COST

Store cost separately from selling price.

Example:

Selling Price: R25

Cost Price: R16

The system should be capable of calculating:

Gross Margin

Gross Profit

Estimated Profit

But do not pretend the calculated profit is accounting-grade unless the accounting model actually supports that level of accuracy.

Keep the initial model simple and transparent.

# 16\. PRODUCT PRICES

Do not assume a product has only one price forever.

The architecture should allow future support for:

Normal Price

Happy Hour Price

Lunch Price

Member Price

Takeaway Price

Wholesale Price

However, do not overbuild this in the first MVP.

The schema should not prevent future pricing models.

# 17\. RESTAURANT RECIPES

Restaurant products may be composed of ingredients.

Example:

Chicken Burger

Selling Price: R85

Recipe:

Chicken Fillet: 150g

Burger Bun: 1

Lettuce: 30g

Tomato: 40g

Sauce: 20ml

When one Chicken Burger is sold:

Chicken Fillet -150g

Burger Bun -1

Lettuce -30g

Tomato -40g

Sauce -20ml

When three are sold:

Chicken Fillet -450g

Burger Bun -3

Lettuce -90g

Tomato -120g

Sauce -60ml

Recipes must be configurable.

Do not hard-code restaurant ingredients.

# 18\. RECIPES MUST SUPPORT SUB-RECIPES

The architecture should allow future support for:

Burger Sauce

being itself a prepared item.

Example:

Burger Sauce

↓

Mayonnaise

Tomato Sauce

Spices

Then:

Chicken Burger

↓

Burger Sauce

This is useful for restaurants and production businesses.

Do not necessarily implement advanced manufacturing logic in V1, but do not architect the database so it becomes impossible later.

# 19\. STOCK

Inventory is not simply a number.

The system must maintain a history of stock movements.

Examples:

Purchase

Sale

Waste

Damage

Adjustment

Transfer

Return

Stock Count

Opening Balance

Consumption

The application should be able to answer:

Why did the stock quantity change?

Every stock movement should have:

Item

Quantity

Unit

Location

Movement type

Date/time

User

Reference

Optional notes

# 20\. STOCK MOVEMENT IS THE SOURCE OF TRUTH

Avoid blindly modifying:

stock\_quantity = 50

without creating a corresponding movement record.

A stock balance may be cached for performance, but the movement ledger must provide the audit history.

Example:

Opening Stock +100

Purchase +50

Sales -80

Waste -5

Transfer -10

\-------------------

Expected 55

This allows the system to reconstruct what happened.

# 21\. STOCK TRANSFERS

The business may move stock between locations.

Example:

Main Store

↓

Bar Store

Transfer:

Castle Lager

20 crates

The system records:

OUT:

Main Store

20 crates

IN:

Bar Store

20 crates

Do not treat this as a sale.

It is a stock transfer.

# 22\. STOCK COUNT

The system must support physical stock counts.

Example:

Expected:

Castle Lager = 131 bottles

Physical count:

126 bottles

Variance:

\-5 bottles

The system must not silently overwrite 131 with 126.

It should record:

Stock Count

Expected: 131

Counted: 126

Variance: -5

User: Manager

Date/time: ...

Then create the corresponding adjustment.

# 23\. WASTAGE

Support explicit wastage.

Examples:

Broken

Expired

Spilled

Damaged

Staff Consumption

Complimentary

Other

Wastage must be visible in reports.

Do not allow employees to simply delete stock.

# 24\. SALES

A sale should contain:

Sale

├── business

├── department

├── terminal

├── employee

├── customer (optional)

├── items

├── discounts

├── payments

└── timestamps

A sale can contain multiple items.

Example:

Sale #10482

Castle Lager ×2 R50

Chicken Wings ×1 R80

Coke ×1 R15

Total R145

# 25\. SPLIT PAYMENTS

The payment model MUST support multiple payments for one sale.

Example:

Sale:

R850

Payments:

Cash:

R300

Card:

R550

Therefore:

Do NOT put a single:

payment\_method

column directly on the sale as the only payment architecture.

Instead use a payment collection:

sale

↓

payments

├── cash

├── card

└── EFT

# 26\. PAYMENT PROCESSING VS PAYMENT RECORDING

V1 does NOT process payments.

It records them.

The customer may use:

Cash

Existing card machine

Bank EFT

Other physical payment methods

The application records the result.

Example:

Customer owes R250.

Cashier:

1\. Takes card payment on existing card machine.

2\. Card machine approves R250.

3\. Cashier selects CARD in your application.

4\. Application records R250 as CARD.

Your application does NOT communicate with the card machine in V1.

No payment API is required.

No payment gateway is required.

No banking integration is required.

This is intentional.

# 27\. PAYMENT DATA MODEL

A payment should contain fields conceptually similar to:

id

sale\_id

method

amount

status

reference

created\_at

created\_by

notes

Payment methods should be configurable where practical.

Initial methods:

Cash

Card

EFT

Other

Do not hard-code the application so that adding another payment method later requires database redesign.

# 28\. CASH HANDLING

Cash sales need additional information where appropriate.

Example:

Sale:

R350

Cash received:

R500

Change:

R150

The system should calculate change.

Cash drawer totals should be associated with shifts.

# 29\. SHIFTS

A cashier should be able to start and end a shift.

Example:

SHIFT OPEN

Employee:

Thabo

Opening Cash:

R1,000

Time:

08:00

At closing:

EXPECTED CASH:

R6,200

ACTUAL CASH:

R5,900

VARIANCE:

\-R300

The system should record the closing.

Do not delete or overwrite historical shifts.

# 30\. DAILY CLOSE

The system should provide a simple "Close Day" / "Close Shift" workflow.

Example:

TODAY

Sales:

R32,450

Cash:

R10,200

Card:

R17,850

EFT:

R4,400

Expenses:

R1,250

The manager should be able to reconcile the figures.

# 31\. DEPARTMENT REPORTING

The owner needs both:

### BUSINESS-WIDE REPORTING

Total Sales

Total Payments

Total Expenses

Estimated Gross Profit

Stock Value

Stock Variance

and:

### DEPARTMENT REPORTING

BAR

Sales

Cost

Gross Profit

Stock

RESTAURANT

Sales

Food Cost

Gross Profit

Stock

CAR WASH

Sales

Consumables

Gross Profit

Never force the owner to run separate applications for each department.

# 32\. CAR WASH MODEL

A car wash should use generic services.

Example:

Basic Wash

R50

Premium Wash

R100

Wash + Interior

R150

Full Detail

R300

A service may have:

Price

Duration

Department

Active status

Optional consumables

Optional stock recipe

Example:

Premium Wash

Price:

R100

Consumes:

Shampoo:

150ml

Tyre Cleaner:

30ml

When the service is sold, consumables can be deducted automatically.

# 33\. SERVICE VS PRODUCT

The database should allow:

Product:

Castle Lager

Service:

Premium Car Wash

Both can appear on the POS.

Example:

CURRENT SALE

Castle Lager R25

Premium Car Wash R100

TOTAL R125

This is important because a mixed-operation business may eventually sell products and services in the same transaction.

# 34\. POS TERMINALS

A business may eventually have multiple computers.

Example:

Bar Till 1

Bar Till 2

Restaurant Till

Manager PC

Each terminal should be identifiable.

Conceptually:

terminal

id

name

department\_id

location\_id

active

Do not assume one business has only one computer.

However, the initial MVP may operate on a single computer with SQLite.

The database model must not prevent future multi-terminal functionality.

# 35\. LOCAL NETWORK FUTURE

Future architecture may support:

POS 1 ─┐

POS 2 ─┼── Local Business Server ── SQLite

POS 3 ─┘

This still works without cloud infrastructure.

Do not implement this complexity unless required by the MVP.

Architect so it can be added later.

# 36\. USERS AND ROLES

Support users/employees.

Example:

Owner

Manager

Cashier

Waiter

Car Wash Operator

Stock Controller

Permissions should eventually control things like:

Sell

Refund

Discount

Change prices

Adjust stock

View reports

Manage employees

Close shifts

Delete/cancel transactions

Do not give every employee unrestricted access.

# 37\. AUDIT TRAIL

Important business actions should be auditable.

Examples:

Price changed

Stock adjusted

Sale cancelled

Refund issued

Payment changed

User created

Product deleted/deactivated

Stock transferred

Shift closed

Record:

Who

What

When

Old value

New value

Reason where appropriate

Never silently mutate important financial or stock history.

# 38\. NEVER DELETE FINANCIAL HISTORY

Sales, payments, stock movements, refunds, and completed shifts should generally be immutable historical records.

Instead of deleting a completed sale:

DELETE SALE

use:

VOID / CANCEL / REFUND

with an audit trail.

This is essential for accountability.

# 39\. PRODUCT DELETION

Products that have historical transactions should generally be:

ACTIVE = FALSE

rather than physically deleted.

Example:

Old Product

Status:

Inactive

Historical sales still reference it.

# 40\. CUSTOMIZATION

The owner must be able to customize:

Departments

Categories

Products

Services

Prices

Costs

Stock units

Minimum stock levels

Recipes

Payment methods

Employees

Roles

Inventory locations

Receipt settings

Business information

Avoid hard-coded business assumptions.

# 41\. DATABASE TENANCY

Even though V1 is local-only, design every major record with business ownership in mind.

For example:

business\_id

department\_id

location\_id

where appropriate.

This prevents accidental mixing of data.

The database must conceptually support:

Business A

└── all its data

Business B

└── all its data

Even if the initial application only supports one business profile per installation.

# 42\. MULTI-BUSINESS FUTURE

Do not confuse:

### Multiple departments

with:

### Multiple businesses

Example:

Moko's Lifestyle Centre

├── Bar

├── Restaurant

└── Car Wash

\= ONE BUSINESS.

Whereas:

Moko's Tavern

Moko's Car Wash

Moko's Restaurant

could potentially be separate businesses.

The architecture should keep this distinction clear.

# 43\. EXPENSES

Support basic expense recording.

Example:

Electricity

Rent

Cleaning

Transport

Supplier Payment

Repairs

Other

An expense should contain:

Amount

Category

Department (optional)

Date

Payment method

User

Description

Reference

A shared expense may belong only to the overall business.

Example:

Electricity:

R5,000

Department:

Business-wide

# 44\. SUPPLIERS

Products need suppliers eventually.

Support:

Supplier

↓

Purchase

↓

Stock Movement

Example:

Supplier:

ABC Distributors

Purchase:

20 crates Castle Lager

Cost:

R3,200

This should increase inventory.

Do not treat a purchase merely as a stock number change.

# 45\. PURCHASES

Purchases should have:

Supplier

Date

Invoice/reference

Items

Quantities

Costs

Total

Payment status

A purchase should generate stock movements.

# 46\. STOCK VALUE

The system should eventually calculate stock value.

Example:

100 bottles × R16 cost

\=

R1,600 stock value

Be explicit about what cost method is being used.

Do not pretend to implement sophisticated accounting inventory valuation without a defined methodology.

The MVP can use a simple configured/latest cost model if clearly documented.

# 47\. DASHBOARD

The dashboard should answer:

"What happened in my business today?"

For example:

TODAY

TOTAL SALES

R32,450

BAR

R12,800

RESTAURANT

R14,650

CAR WASH

R5,000

PAYMENTS

Cash R10,200

Card R17,850

EFT R4,400

LOW STOCK

5 products

STOCK VARIANCE

R680

OPEN SHIFTS

2

The owner should not need to understand accounting to understand this dashboard.

# 48\. PRODUCT CREATION UX

The product creation experience is critical.

Basic mode:

Name

Category

Selling Price

Cost

Track Stock? YES/NO

Quantity

Minimum Stock

SAVE

Advanced mode:

Inventory Unit

Purchase Unit

Selling Unit

Unit Conversion

Supplier

Barcode

SKU

Recipe

Department

Location

Tax

Multiple Prices

Do not expose advanced complexity unless needed.

# 49\. BARCODE SUPPORT

The architecture should allow:

barcode

SKU

product\_code

A USB barcode scanner should work as keyboard input.

No cloud service should be required.

# 50\. RECEIPTS

The application should generate printable receipts.

Receipt information can include:

Business name

Address

Receipt number

Date/time

Cashier

Items

Quantities

Prices

Discount

Total

Payment method

Change

Receipt printer support should be considered but not allowed to contaminate the core database architecture.

# 51\. BACKUPS

Because the application is offline and local, backups are critical.

The application should automatically create local backups.

Example:

Backups/

2026-09-21.db

2026-09-20.db

2026-09-19.db

Keep multiple historical backups.

The user should also be able to manually export a backup.

Potential future destinations:

USB

External drive

Google Drive

OneDrive

Cloud backup

Cloud backup is OPTIONAL and must never be required for normal operation.

# 52\. DATA EXPORT

Provide export options eventually:

CSV

Excel

PDF

Database backup

Especially:

Sales

Stock

Products

Expenses

Payments

Reports

The business owns its data.

Do not trap customer data inside the application.

# 53\. DATABASE DESIGN PRINCIPLES

Follow these principles:

### Principle 1

Do not hard-code industries.

### Principle 2

Do not hard-code products.

### Principle 3

Do not hard-code categories.

### Principle 4

Do not hard-code payment methods where avoidable.

### Principle 5

Do not hard-code stock units.

### Principle 6

Do not hard-code departments.

### Principle 7

Do not hard-code recipes.

### Principle 8

Do not delete financial history.

### Principle 9

Every important stock change should be auditable.

### Principle 10

Every financial transaction should be traceable.

### Principle 11

Offline operation is a core requirement, not an optional mode.

### Principle 12

The database must support a multi-department business.

# 54\. CONCEPTUAL DATABASE

The exact implementation may vary, but the architecture should roughly contain entities such as:

businesses

departments

locations

users

roles

permissions

products

services

categories

units

unit\_conversions

product\_units

recipes

recipe\_items

suppliers

purchases

purchase\_items

inventory

inventory\_movements

inventory\_transfers

stock\_counts

stock\_count\_items

wastage

sales

sale\_items

payments

refunds

expenses

expense\_categories

shifts

terminals

customers

audit\_logs

settings

Do not blindly create every table before understanding the actual requirements.

Normalize where appropriate, but don't create unnecessary complexity.

# 55\. IMPORTANT DATABASE RELATIONSHIP

The conceptual relationship should resemble:

BUSINESS

│

├───────────────┐

│ │

DEPARTMENTS LOCATIONS

│ │

│ │

└──────┬────────┘

│

PRODUCTS

│

RECIPES

│

INVENTORY

│

STOCK MOVEMENTS

│

SALES

│

PAYMENTS

with users, suppliers, expenses, terminals, customers and audit logs connected appropriately.

# 56\. EXAMPLE: BAR + RESTAURANT + CAR WASH

The database should be capable of representing:

BUSINESS

Moko's Lifestyle Centre

Departments:

Bar

Restaurant

Car Wash

Products:

Castle Lager

Coke

Chicken

Potatoes

Cooking Oil

Shampoo

Services:

Basic Wash

Premium Wash

Full Detail

Locations:

Main Store

Bar Store

Kitchen

Car Wash Store

Recipes:

Chicken Burger

Chicken 150g

Bun 1

Lettuce 30g

Tomato 40g

Sauce 20ml

Payments:

Cash

Card

EFT

The entire operation exists in one local database.

# 57\. EXAMPLE TRANSACTION

Customer purchases:

Premium Car Wash R100

Castle Lager ×2 R50

Chicken Burger R85

Total:

R235

The sale can contain products/services from different departments if the business allows it.

Inventory effects:

Castle Lager:

\-2 bottles

Chicken:

\-150g

Burger Bun:

\-1

Lettuce:

\-30g

Tomato:

\-40g

Sauce:

\-20ml

Car Wash Shampoo:

\-150ml

Tyre Cleaner:

\-30ml

Payment:

Card:

R235

This is one customer transaction with multiple operational effects.

# 58\. DO NOT OVERENGINEER THE MVP

The architecture must be extensible, but the initial product should remain simple.

V1 priorities:

1\. Business setup

2\. Departments

3\. Products/services

4\. Categories

5\. Basic inventory

6\. POS

7\. Payment recording

8\. Stock deduction

9\. Stock counting

10\. Shift management

11\. Basic expenses

12\. Daily reporting

13\. Local backups

Do NOT initially build:

Complex accounting

Payroll

Full ERP

Cloud synchronization

Payment gateway integrations

Advanced CRM

Complex loyalty system

AI forecasting

Multi-country tax engine

Advanced manufacturing

Those can come later.

# 59\. USER EXPERIENCE PRINCIPLE

The software is for business owners and employees, not database experts.

Every feature should be evaluated by asking:

"Can a normal shop/bar/restaurant/car-wash employee understand this without training?"

Prefer:

Add Product

Sell

Receive Stock

Count Stock

Transfer Stock

Record Expense

Close Shift

View Report

over technical terminology.

# 60\. ARCHITECTURAL FLEXIBILITY

The database must support businesses that don't fit the original three examples.

For example:

Hotel

├── Restaurant

├── Bar

└── Laundry

Shopping Centre

├── Restaurant

├── Car Wash

└── Retail

Butchery

├── Retail

└── Takeaway

Salon

├── Hair

└── Retail Products

The database should not need redesigning for these.

The configuration should create the structure.

# 61\. GOLDEN RULE

Whenever you are about to add code, ask:

"Am I building a generic business capability, or am I accidentally hard-coding one industry?"

If the latter, stop and redesign.

Example:

BAD:

if businessType === "bar":

deductBeer()

GOOD:

processSale()

↓

processSaleItems()

↓

applyProductInventoryRules()

The system should understand the product configuration, not the industry.

# 62\. SECOND GOLDEN RULE

The database is the source of truth.

The UI is only a way to manipulate and display the business data.

Do not create hidden business state in frontend components that isn't represented properly in the database.

Every important business event must be persistable.

# 63\. THIRD GOLDEN RULE

Prefer configuration over code.

If a business wants:

Premium Wash

R120

the owner changes configuration/data.

The developer should not have to modify code.

If a business adds:

New Department:

Bottle Store

the application should support it through configuration.

# 64\. FOURTH GOLDEN RULE

Never sacrifice data integrity for convenience.

For example:

Do not allow:

Sale says R500

Payments say R450

without clearly representing the difference.

Do not allow:

Expected stock = 100

Actual stock = 80

to silently become 80.

Record the variance.

Business history must be explainable.

# 65\. DEVELOPMENT PROCESS

Before writing significant application code:

Understand this architecture.

Inspect the existing repository.

Identify the current stack.

Identify the current database strategy.

Produce a database/entity relationship plan.

Identify conflicts with this specification.

Explain those conflicts before changing the architecture.

Implement incrementally.

Test database constraints.

Test offline operation.

Test business scenarios.

Do not rewrite working architecture merely because you prefer another framework.

Do not introduce cloud infrastructure unless explicitly required.

# 66\. REQUIRED TEST SCENARIOS

Before considering the core system complete, test at minimum:

### Scenario A — Bar

Create:

Bar

Castle Lager

Coke

Sell:

Castle ×2

Coke ×1

Verify:

Sale recorded.

Payment recorded.

Stock deducted.

Report updated.

### Scenario B — Restaurant

Create:

Chicken Burger

with recipe:

Chicken 150g

Bun 1

Lettuce 30g

Tomato 40g

Sauce 20ml

Sell one.

Verify every ingredient is deducted correctly.

### Scenario C — Car Wash

Create:

Premium Wash

with consumables:

Shampoo 150ml

Tyre Cleaner 30ml

Sell one.

Verify consumables are deducted.

### Scenario D — Multi-department business

Create:

Bar

Restaurant

Car Wash

Make sales in all three.

Verify:

Business Total

+

Department Totals

are both correct.

### Scenario E — Split payment

Sale:

R850

Payment:

Cash R300

Card R550

Verify:

Total payments = R850

### Scenario F — Stock variance

Expected:

100

Counted:

95

Verify:

Variance = -5

and that the adjustment is auditable.

### Scenario G — Offline

Disconnect the computer from the internet.

Verify that the user can:

Open application

Sell

Record payment

Update stock

Count stock

Close shift

View reports

without errors caused by lack of internet.

# 67\. FINAL STANDARD OF TRUTH

This specification is the architectural standard of truth for the project.

When future requirements conflict with this document:

Identify the conflict.

Explain it.

Determine whether the new requirement genuinely requires architectural change.

Preserve the core principles unless there is a compelling technical reason to change them.

The core product is:

**A local-first, offline, configurable business operations and POS system where one business can contain multiple departments such as a bar, restaurant, and car wash, while sharing products, inventory, sales, payments, employees, locations, and reporting where appropriate.**

The application must be built around **generic business entities and configurable relationships**, not around hard-coded industry-specific logic.

The most important architectural distinction is:

BUSINESS

↓

DEPARTMENTS

↓

PRODUCTS / SERVICES

↓

RECIPES / CONFIGURATION

↓

INVENTORY

↓

SALES

↓

PAYMENTS

↓

REPORTING

And the most important operational promise is:

**Install it on the customer's computer and it works. No internet and no cloud database are required for normal operation.**

# 68\. TECHNICAL DECISIONS & AGENT EXECUTION STANDARD

This section exists because the Master Architecture defines the business/domain requirements, but an autonomous development agent must not be allowed to invent major technical decisions.

The rules in this section are mandatory unless a documented technical conflict is discovered in the existing repository.

The agent must NOT silently choose an alternative architecture simply because it is familiar, convenient, or faster to implement.

If an architectural decision is not explicitly defined here, the agent must:

Inspect the existing repository and architecture.

Determine whether the missing decision is material.

Identify the available options.

Select the option that best preserves the principles of this specification.

Document the decision.

If the decision could materially affect data integrity, portability, or future architecture, STOP before implementation and request approval.

# 69\. EXISTING REPOSITORY VS GREENFIELD

Before writing application code, determine whether the repository is:

GREENFIELD

or:

EXISTING APPLICATION

### If GREENFIELD

The agent must establish the application structure, technology stack, database layer, migrations, testing strategy, and local storage architecture before implementing business features.

### If an EXISTING APPLICATION exists

The agent MUST:

Inspect the repository.

Identify the current framework.

Identify the current runtime/platform.

Identify the current database.

Identify the current ORM/query layer.

Identify the existing migration strategy.

Identify existing authentication/session handling.

Identify existing testing infrastructure.

Identify existing printing/device integrations.

Identify existing architectural constraints.

The agent must NOT automatically replace the existing stack.

If the existing architecture conflicts with this Master Architecture, produce a conflict report before making destructive changes.

# 70\. REQUIRED FIRST DELIVERABLE BEFORE SIGNIFICANT CODE

Before implementing significant application functionality, the agent must produce three documents:

## A. Technical Decisions Record

At minimum:

Application platform

Technology stack

Database technology

Database access layer

Primary-key strategy

Money representation

Tax representation

Rounding strategy

Unit/conversion strategy

Concurrency strategy

Authentication/session strategy

Migration strategy

Backup strategy

Printing strategy

Barcode strategy

Testing strategy

## B. Entity Relationship Diagram

The agent must produce an ERD showing:

Business

Departments

Locations

Users

Roles

Permissions

Products

Services

Categories

Units

Product Units / Packaging

Recipes

Recipe Items

Suppliers

Purchases

Purchase Items

Inventory

Inventory Movements

Transfers

Stock Counts

Wastage

Sales

Sale Items

Payments

Refunds

Expenses

Shifts

Terminals

Customers

Audit Logs

Settings

The ERD must show:

Primary keys

Foreign keys

Cardinality

Required vs optional relationships

Unique constraints

Important indexes

Business ownership boundaries

## C. Schema Specification

For every table actually required by V1, document:

Table

Purpose

Column

Type

Nullable

Default

Primary Key

Foreign Key

Unique constraint

Check constraint

Index

Delete behaviour

Update behaviour

Do not begin implementation of the database merely from conceptual entity names.

# 71\. PLATFORM DECISION

The application platform must be explicitly identified before implementation.

The agent must NOT assume:

Desktop

Android tablet

Web application

Mobile application

Cross-platform application

without establishing the intended platform.

The existing architecture contains desktop-oriented assumptions including:

Installation on the customer's computer

USB barcode scanner as keyboard input

Local SQLite database

Local receipt printing

Offline operation

Local filesystem backups

Therefore, the default interpretation of the current specification is:

The application is a local installed POS/business application running on a customer's computer.

If the repository establishes a different platform, the agent must identify the conflict before implementation.

The platform decision must explicitly document:

Target OS

Target hardware

Screen/input model

Keyboard/mouse support

Touch support

Barcode scanner support

Receipt printer support

Filesystem access

Local database access

Offline requirements

Do not allow the agent to silently convert the architecture into a cloud web application.

# 72\. TECHNOLOGY STACK

The exact framework may depend on the existing repository.

If the project is greenfield, the agent must select a stack appropriate for:

Offline-first operation

Local SQLite

Local filesystem access

Reliable database transactions

Local printing

USB/keyboard barcode scanning

Long-term maintainability

Packaging/installability

The agent must document the selected stack before implementation.

The agent must NOT select technology solely because it is currently popular.

The architecture is more important than framework preference.

# 73\. SQLITE IS THE V1 SOURCE OF TRUTH

SQLite remains the primary V1 database unless a documented technical conflict makes this impossible.

The application must continue to work with:

No Internet

No Cloud Database

No Supabase

No Firebase

No Remote API

No Authentication Server

The SQLite database is the customer's business data.

The application must never treat a remote service as the authoritative source of truth for normal operation.

# 74\. PRIMARY KEY STRATEGY

The primary-key strategy must be explicitly chosen before schema implementation.

For entities that may eventually participate in:

backups

imports

exports

synchronization

multi-terminal operation

local network operation

multi-business operation

the architecture should prefer stable identifiers that are safe to merge across databases.

The agent must document whether identifiers are:

UUID

ULID

INTEGER

OTHER

and explain the choice.

Do NOT allow different tables to casually use different ID strategies without a documented reason.

If future synchronization is a real architectural goal, identifiers must be selected with synchronization in mind from the beginning.

# 75\. BUSINESS OWNERSHIP AND FOREIGN KEYS

Major business-owned entities must have an unambiguous relationship to the owning business.

Where appropriate:

business\_id

department\_id

location\_id

user\_id

terminal\_id

must be represented explicitly.

The agent must prevent records from one business accidentally referencing records belonging to another business.

Database constraints and application-level validation should both be considered where appropriate.

# 76\. MONEY MUST NEVER USE FLOATING-POINT

Money must NOT be represented using binary floating-point values such as:

float

double

for monetary calculations.

The implementation must use a fixed-precision monetary representation.

The exact implementation may be:

integer minor units

or another explicitly documented fixed-point/decimal strategy.

For example, if using minor units:

R25.00 = 2500

R16.50 = 1650

The agent must document the representation.

All calculations involving:

price

cost

sale totals

discounts

tax

payments

refunds

expenses

profit

must use the chosen exact representation.

Never rely on floating-point arithmetic for financial totals.

# 77\. CURRENCY

V1 is conceptually single-currency per business.

A business has one configured operating currency:

business.currency

The initial default is:

ZAR

but ZAR must NOT be hard-coded throughout the application.

The architecture must leave room for future multi-currency support without pretending that V1 implements it.

Do not build a multi-country/multi-currency accounting engine into V1.

# 78\. TAX / VAT

Tax handling must be explicitly represented rather than hidden inside price calculations.

The system must support configuration for:

Tax enabled

Tax name

Tax rate

Price includes tax

Price excludes tax

Taxable / non-taxable

The exact V1 tax model must be documented before implementation.

The agent must never silently assume that:

R115

means either:

R100 + VAT

or:

R115 including VAT

without an explicit business configuration.

Tax calculations must use the same fixed-precision money strategy described above.

Tax rounding must be deterministic and documented.

# 79\. ROUNDING

The system must have one documented monetary rounding strategy.

The agent must explicitly define:

Price rounding

Tax rounding

Line-total rounding

Discount rounding

Payment rounding

Refund rounding

Report rounding

The same rule must be applied consistently.

Do not allow different screens or services to independently implement rounding.

Financial calculations should be centralized in the business/domain layer.

# 80\. UNITS AND PACKAGING ARE RELATED BUT DISTINCT

The architecture must distinguish between:

## Universal units

Examples:

gram

kilogram

millilitre

litre

piece

and:

## Product-specific packaging

Examples:

Castle Lager

Crate = 12 bottles

Castle Lager

Case = 24 bottles

Universal unit relationships and product-specific packaging must not accidentally become two unrelated conversion systems.

The agent must define a coherent model that allows:

Base Unit

Purchase Unit

Selling Unit

Packaging Unit

Conversion Factor

while preserving the distinction between:

general unit conversion

and:

product-specific packaging

Example:

1 litre = 1000 millilitres

is a general unit relationship.

Whereas:

1 Castle Lager crate = 12 Castle Lager bottles

is product-specific packaging.

Do not assume every crate contains the same quantity.

# 81\. INVENTORY QUANTITY PRECISION

Inventory quantities must not automatically be integers.

The schema must support quantities such as:

1 bottle

12 bottles

1.5 litres

150ml

2.75kg

The quantity representation and precision must be documented.

Inventory calculations must use exact or appropriately precise numeric handling rather than binary floating-point arithmetic.

# 82\. MULTI-TERMINAL CONCURRENCY

V1 may run on a single computer.

Future architecture may support:

POS 1

POS 2

POS 3

↓

Local Business Server

↓

SQLite

However, the agent must NOT pretend that simply pointing multiple terminals at the same SQLite file is a complete multi-terminal architecture.

For V1:

Single-machine SQLite

is the default.

For future multi-terminal support, the architecture must explicitly define the concurrency model.

Potential approaches include:

Local server / single writer

WAL

Transactional write queue

Different database architecture

The agent must not implement multi-terminal concurrency merely because the schema contains a terminal\_id.

Multi-terminal operation is a future architectural capability unless explicitly included in MVP scope.

# 83\. TRANSACTION INTEGRITY

Operations that modify multiple related records must be atomic.

For example, completing a sale may involve:

Sale

Sale Items

Payments

Inventory Movements

Recipe Consumption

These operations must not partially succeed.

The application must use database transactions so that either:

the entire business operation succeeds

or:

the entire operation is rolled back

Example:

A sale must never exist as completed while its required inventory deductions failed silently.

# 84\. PAYMENT TOTAL INTEGRITY

The system must enforce:

sum(payments) = amount\_due

for a completed fully paid sale, subject to explicitly defined rounding/tolerance rules.

The application must reject or clearly represent:

Sale = R500

Payments = R450

rather than silently considering the sale paid.

Overpayment behaviour must also be explicitly defined.

For cash:

Amount due = R350

Cash received = R500

Change = R150

is valid.

For non-cash payment methods, overpayment must not be silently accepted unless explicitly supported.

# 85\. AUTHENTICATION AND SESSION MODEL

Roles and permissions are not sufficient by themselves.

The agent must define how an employee identifies themselves at a terminal.

The V1 authentication mechanism must be explicitly selected, for example:

PIN

Password

Username + password

Other local authentication

The chosen mechanism must work fully offline.

The session model must define:

Login

Logout

Session ownership

Inactive timeout

Permission checks

Shift association

Terminal association

Manager override

Do not introduce a cloud authentication server.

A cashier's identity must be recorded against important transactions.

# 86\. ROLE AND PERMISSION MODEL

Permissions must be represented as data rather than scattered throughout the frontend.

Examples:

SELL

REFUND

VOID

DISCOUNT

CHANGE\_PRICE

ADJUST\_STOCK

TRANSFER\_STOCK

VIEW\_REPORTS

MANAGE\_USERS

CLOSE\_SHIFT

MANAGE\_PRODUCTS

The exact permission set may evolve.

The architecture must allow permissions to be extended without rewriting the application architecture.

Sensitive actions must be checked at the business/service layer, not only by hiding UI buttons.

# 87\. REFUNDS AND VOIDS

Refunds and voids must be treated as business events, not destructive deletion.

The agent must define:

### Full refund

Entire sale reversed.

### Partial refund

One or more sale items, or part of an item's quantity, reversed.

The system must determine and record:

Original sale

Refund amount

Refunded items

Quantity refunded

Reason

User

Date/time

Payment reversal information

Inventory reversal

Audit information

Refunds must not mutate the historical original sale into something that makes the original transaction impossible to reconstruct.

# 88\. REFUND INVENTORY BEHAVIOUR

Where a refunded product was previously deducted from inventory, the refund process must explicitly determine whether stock is:

returned to inventory

or:

not returned

depending on the nature of the refund.

For example:

Unopened physical product returned

may restore stock.

Whereas:

Consumed food

Completed car wash

Used service

should not automatically create inventory.

This decision must be represented by the refund workflow rather than guessed from the word "refund."

# 89\. SPLIT-PAYMENT REFUNDS

Because a sale may contain:

Cash

Card

EFT

the refund model must not assume one original payment.

For partial or full refunds, the system must record:

refund amount

payment(s) affected

amount reversed per payment

The exact V1 policy for refunding split payments must be documented.

Do not silently rewrite the original payments.

# 90\. NEGATIVE STOCK POLICY

The application must explicitly define whether negative stock is allowed.

The default V1 policy should be:

Negative stock is NOT silently allowed.

If a sale would reduce available stock below zero, the application must either:

block the sale

or:

require an explicitly authorized override

if the business configuration permits it.

If an override is allowed, record:

user

reason

timestamp

item

quantity

and make the event auditable.

Do not allow negative inventory to appear accidentally because a validation was forgotten.

# 91\. STOCK ADJUSTMENT INTEGRITY

Stock balances must never be silently overwritten.

Any adjustment must generate an inventory movement.

Stock count:

Expected = 100

Counted = 95

Variance = -5

must create an auditable adjustment of:

\-5

The original expected quantity must remain reconstructable.

# 92\. SCHEMA MIGRATIONS

The database is customer data and must survive application upgrades.

Application updates must NOT simply replace the SQLite database.

The project must have an explicit schema migration strategy.

Every schema change must be represented as a versioned migration.

Conceptually:

Database Version 1

↓

Migration 2

↓

Migration 3

↓

Migration 4

The migration system must support:

New tables

New columns

New indexes

Constraint changes where safely possible

Data transformations

Rollback/recovery strategy where appropriate

Before applying migrations to customer data:

Backup

↓

Migration

↓

Integrity verification

A failed migration must not leave the customer's database silently corrupted.

# 93\. BACKUP SAFETY

Automatic backups are mandatory because SQLite is the primary customer data store.

Before destructive database migrations or major recovery operations:

Create backup

Verify backup

Perform operation

Verify resulting database

The backup system must not overwrite the only available backup.

Multiple historical backups should be retained according to a documented retention policy.

# 94\. RECEIPT PRINTING

Receipt generation must be separated from the core business/database layer.

The business layer should produce structured receipt data.

A printer adapter can then render that data.

The architecture must allow:

Receipt Data

↓

Printer Adapter

↓

Physical Printer

The agent must explicitly choose the V1 printing strategy:

OS print dialog

or:

direct printer protocol such as ESC/POS

Do not hard-code printer-specific logic into sales or database code.

# 95\. BARCODE INPUT

Barcode scanning must remain local.

For desktop V1, a USB scanner may operate as keyboard input.

The barcode value must resolve through local product data.

The architecture must support:

barcode

SKU

product\_code

without requiring an online barcode service.

# 96\. PRODUCT RELATIONSHIP CARDINALITY

The agent must explicitly define relationship cardinalities.

Examples requiring decisions:

Can one product belong to multiple categories?

Can one category belong to multiple departments?

Can one product be sold in multiple departments?

Can one product have multiple selling units?

Can one product have multiple suppliers?

Can one location serve multiple departments?

Can one department use multiple locations?

Can one recipe contain another recipe?

Can one service consume inventory?

Can one sale contain items from multiple departments?

The implementation must follow the documented cardinality.

Do not infer these relationships from UI convenience.

# 97\. LOCATION / DEPARTMENT RELATIONSHIP

A department and an inventory location are NOT the same concept.

A location may:

serve one department

or potentially:

serve multiple departments

The architecture must not enforce a one-to-one relationship unless the business requirement explicitly demands it.

Departments may have a default location, but inventory ownership and physical storage remain separate concepts.

# 98\. PRODUCTS, SERVICES AND SELLABLE ITEMS

Products and services remain distinct domain concepts.

However, the POS should operate on a common sellable-item abstraction.

The agent must ensure that:

Product

and:

Service

can both appear in:

Sale

Sale Item

Receipt

Payment

Reporting

without duplicating the entire sales architecture.

Inventory effects are determined by configuration.

Examples:

Beer

→ inventory deduction

Chicken Burger

→ recipe inventory deduction

Premium Car Wash

→ optional consumable deduction

Haircut

→ no inventory deduction

# 99\. TESTING MUST INCLUDE FAILURE CASES

The existing happy-path scenarios remain mandatory.

However, an autonomous agent must ALSO test failure and integrity cases.

At minimum:

### Negative stock

Attempt to sell more stock than available.

Verify the configured negative-stock policy is enforced.

### Over-refund

Attempt to refund more than was originally sold.

Verify rejection.

### Duplicate refund

Attempt to refund the same transaction twice.

Verify the system prevents an invalid second refund.

### Payment mismatch

Attempt:

Sale = R500

Payments = R450

Verify completion is blocked or explicitly marked unpaid.

### Payment overage

Attempt an invalid non-cash overpayment.

Verify policy enforcement.

### Split payment refund

Refund part of a sale originally paid using multiple payment methods.

Verify payment reversal records remain traceable.

### Stock count variance

Verify that:

Expected ≠ Counted

creates an auditable adjustment.

### Failed sale transaction

Force an inventory/payment/database failure during sale completion.

Verify that partial records are not left behind.

### Duplicate submission

Submit the same sale operation twice.

Verify the system does not accidentally create duplicate financial transactions.

### Offline operation

Disconnect network access and test the complete operational loop.

# 100\. REQUIRED AGENT STARTUP PROCEDURE

Before modifying application code, the agent must follow this sequence:

1\. Read this Master Architecture.

↓

2\. Inspect the repository.

↓

3\. Determine GREENFIELD or EXISTING.

↓

4\. Identify the current stack.

↓

5\. Identify the current database.

↓

6\. Identify migration strategy.

↓

7\. Identify authentication strategy.

↓

8\. Identify platform/hardware assumptions.

↓

9\. Produce ERD.

↓

10\. Produce Technical Decisions Record.

↓

11\. Identify conflicts/gaps.

↓

12\. Identify decisions requiring human approval.

↓

13\. Obtain approval where required.

↓

14\. Implement database foundation.

↓

15\. Implement business/service layer.

↓

16\. Implement UI.

↓

17\. Run automated tests.

↓

18\. Run business scenario tests.

↓

19\. Run failure/integrity tests.

↓

20\. Test offline operation.

↓

21\. Test backup and restore.

↓

22\. Verify migrations.

The agent must not skip directly from:

Read specification

to:

Write application code

# 101\. AGENT DECISION RULE

Whenever the specification does not explicitly answer a technical question, the agent must classify the question as:

### LOW-RISK

The agent may make a reasonable implementation decision if it:

Does not alter the business model.

Does not affect customer data compatibility.

Does not affect financial calculations.

Does not affect security.

Does not affect future synchronization.

Does not create irreversible architectural debt.

The agent must document the decision.

### HIGH-RISK

The agent must stop and request approval if the decision affects:

Database structure

Primary keys

Money representation

Tax calculation

Financial history

Inventory integrity

Authentication

Authorization

Data migration

Backup/recovery

Concurrency

Platform

Long-term synchronization

Customer data compatibility

Do not guess on high-risk architectural decisions.

# 102\. NO SILENT ARCHITECTURAL DRIFT

The agent must never silently introduce:

Cloud database

External authentication

Payment gateway

Online-only dependency

Industry-specific tables

Hard-coded business types

Hard-coded categories

Hard-coded payment methods

Hard-coded stock units

Floating-point money

Destructive deletion of financial history

Unversioned database changes

because they are convenient.

If such a change appears technically necessary, the conflict must be documented first.

# 103\. DEFINITION OF DONE

The core system is not considered complete merely because the UI works.

A feature is complete only when:

UI

↓

Business Logic

↓

Database

↓

Audit / History

↓

Reporting

↓

Tests

are consistent.

For financial or inventory functionality, the feature must additionally demonstrate:

Correct totals

Correct stock effects

Correct payment effects

Correct audit trail

Correct failure behaviour

Correct recovery behaviour

# 104\. FINAL AGENT INSTRUCTION

You are an implementation agent, not an architecture improvisation engine.

This Master Architecture defines the business truth.

Your responsibility is to:

preserve the generic business model;

preserve offline-first operation;

preserve SQLite as the V1 source of truth;

preserve data integrity;

preserve auditability;

preserve customer ownership of data;

avoid unnecessary complexity;

identify technical ambiguity before it becomes code;

document material technical decisions;

never silently invent architecture that changes the product.

When uncertain:

DO NOT GUESS ABOUT HIGH-RISK ARCHITECTURE.

Instead:

IDENTIFY

→ DOCUMENT

→ EXPLAIN

→ REQUEST DECISION

→ IMPLEMENT

The objective is not merely to produce working software.

The objective is to produce software whose:

architecture

database

business rules

financial records

inventory records

audit history

backups

migrations

and future extensibility

remain understandable and trustworthy years after the first installation.

# 105\. ARCHITECTURAL PRIORITY ORDER

When trade-offs occur, use this priority order:

1\. Data integrity

2\. Financial correctness

3\. Inventory correctness

4\. Customer data safety

5\. Offline reliability

6\. Auditability

7\. Architectural consistency

8\. Maintainability

9\. Extensibility

10\. User experience

11\. Performance optimisation

12\. Implementation convenience

Implementation convenience must never outrank data integrity.

A shortcut that makes development easier but makes customer data harder to trust is not an acceptable shortcut.