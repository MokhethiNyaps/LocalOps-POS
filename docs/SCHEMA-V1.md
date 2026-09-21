# V1 Schema Specification

## Conventions

SQLite `TEXT` UUIDv7 primary keys; UTC timestamps are `TEXT`; booleans are `INTEGER CHECK (value IN (0,1))`; money is `INTEGER` minor units; quantities are `INTEGER` millionths. All foreign keys are indexed. `created_at` and `updated_at` are required. Historical foreign keys use `ON DELETE RESTRICT`. Business-owned unique keys include `business_id`. JSON is only for audit snapshots/settings, never core relational facts.

Notation below: **PK**, **FK**, **NN**, **UQ**, and defaults/checks in brackets. Common audit columns are omitted from rows where obvious.

## Platform and ownership

| Table | Purpose | Columns and constraints |
|---|---|---|
| `schema_migrations` | Migration history | `version INTEGER PK`, `name TEXT NN UQ`, `checksum TEXT NN`, `applied_at TEXT NN` |
| `businesses` | Legal/operating owner | `id TEXT PK`, `name TEXT NN`, `trading_name TEXT`, `currency TEXT NN DEFAULT 'ZAR' CHECK length=3`, `timezone TEXT NN DEFAULT 'Africa/Johannesburg'`, contact/address/receipt fields, tax fields (`tax_enabled`, `tax_name`, `tax_rate_ppm`, `prices_include_tax`), `negative_stock_policy TEXT NN CHECK BLOCK/OVERRIDE`, `active` |
| `departments` | Configurable operation | `id`, `business_id FK NN`, `name NN`, `description`, `default_location_id FK`, `active`; UQ(business,name) |
| `locations` | Physical stock location | `id`, `business_id FK NN`, `name NN`, `description`, `active`; UQ(business,name) |
| `department_locations` | Many-to-many use | `department_id FK`, `location_id FK`, `is_default`; composite PK; max one default enforced by partial UQ index |
| `terminals` | Identified workstation | `id`, `business_id FK NN`, `name NN`, `department_id FK`, `location_id FK`, `device_key TEXT NN`, `active`; UQ(business,name), UQ(device_key) |
| `settings` | Non-core configurable values | `id`, `business_id FK NN`, `key TEXT NN`, `value_json TEXT NN CHECK json_valid`, UQ(business,key) |

## Identity and authorization

| Table | Purpose | Columns and constraints |
|---|---|---|
| `users` | Local employee identity | `id`, `business_id FK NN`, `username NN`, `display_name NN`, `pin_hash NN`, `active`, `failed_attempts DEFAULT 0`, `locked_until`; UQ(business,username) |
| `roles` | Configurable role | `id`, `business_id FK NN`, `name NN`, `system`, `active`; UQ(business,name) |
| `permissions` | Stable permission catalogue | `code TEXT PK`, `description TEXT NN` |
| `user_roles` | User assignment | `user_id FK`, `role_id FK`; composite PK |
| `role_permissions` | Role grant | `role_id FK`, `permission_code FK`; composite PK |
| `sessions` | Local login/session history | `id`, `business_id FK NN`, `user_id FK NN`, `terminal_id FK NN`, `started_at NN`, `last_active_at NN`, `ended_at`, `end_reason`; index(user,ended_at) |

## Catalogue, units and recipes

| Table | Purpose | Columns and constraints |
|---|---|---|
| `categories` | Configurable grouping | `id`, `business_id FK NN`, `name NN`, `parent_id FK`, `active`; UQ(business,name) |
| `department_categories` | Category availability | `department_id FK`, `category_id FK`; composite PK |
| `units` | Universal/custom unit | `id`, `business_id FK NN`, `code NN`, `name NN`, `dimension NN`, `scale_num INTEGER NN >0`, `scale_den INTEGER NN >0`, `decimal_places INTEGER 0..6`, `active`; UQ(business,code) |
| `sellable_items` | Shared POS identity | `id`, `business_id FK NN`, `kind NN CHECK PRODUCT/SERVICE`, `name NN`, `category_id FK`, `price_minor NN >=0`, `taxable`, `barcode`, `sku`, `product_code`, `active`; partial UQ indexes per business for non-null codes |
| `products` | Product-specific data | `sellable_id PK/FK`, `base_unit_id FK NN`, `cost_minor NN >=0`, `track_stock`, `minimum_quantity_micros >=0` |
| `services` | Service-specific data | `sellable_id PK/FK`, `duration_minutes INTEGER >=0` |
| `department_sellables` | Many-to-many availability | `department_id FK`, `sellable_id FK`, `price_override_minor >=0 nullable`, `active`; composite PK |
| `product_packaging` | Product-specific units | `id`, `business_id FK NN`, `product_id FK NN`, `unit_id FK NN`, `name NN`, `factor_num NN >0`, `factor_den NN >0`, `can_purchase`, `can_sell`; UQ(product,name) |
| `recipes` | Product/service consumption rule | `id`, `business_id FK NN`, `owner_sellable_id FK NN`, `yield_quantity_micros NN >0`, `active`; one active recipe per owner via partial UQ |
| `recipe_items` | Required product input | `id`, `recipe_id FK NN`, `ingredient_product_id FK NN`, `quantity_micros NN >0`, `unit_id FK NN`; UQ(recipe,ingredient) |

## Purchasing and inventory

| Table | Purpose | Columns and constraints |
|---|---|---|
| `suppliers` | Supplier record | `id`, `business_id FK NN`, `name NN`, contact fields, `active`; UQ(business,name) |
| `purchases` | Purchase header | `id`, `business_id FK NN`, `purchase_number NN`, `supplier_id FK`, `location_id FK NN`, `status NN CHECK DRAFT/RECEIVED/VOID`, `invoice_reference`, `purchased_at NN`, totals money integers >=0, `created_by FK NN`; UQ(business,purchase_number) |
| `purchase_items` | Immutable received line snapshot | `id`, `purchase_id FK NN`, `product_id FK NN`, `description NN`, `quantity_micros NN >0`, `unit_id FK NN`, `base_quantity_micros NN >0`, `unit_cost_minor NN >=0`, `line_total_minor NN >=0` |
| `inventory_balances` | Rebuildable balance cache | `business_id FK`, `product_id FK`, `location_id FK`, `quantity_micros NN`, `version NN DEFAULT 0`; composite PK |
| `inventory_movements` | Authoritative stock ledger | `id`, `business_id FK NN`, `product_id FK NN`, `location_id FK NN`, `quantity_micros NN CHECK !=0`, `type NN`, `occurred_at NN`, `user_id FK NN`, `reference_type NN`, `reference_id NN`, `correlation_id NN`, `notes`; indexes(product,location,time), (reference_type,reference_id) |
| `inventory_transfers` | Transfer business event | `id`, `business_id FK NN`, `from_location_id FK NN`, `to_location_id FK NN CHECK differs in service`, `status CHECK DRAFT/COMPLETED/VOID`, `completed_at`, `completed_by FK`, `notes` |
| `inventory_transfer_items` | Transfer quantities | `id`, `transfer_id FK NN`, `product_id FK NN`, `quantity_micros NN >0`; UQ(transfer,product) |
| `stock_counts` | Physical count event | `id`, `business_id FK NN`, `location_id FK NN`, `status CHECK DRAFT/COMPLETED/VOID`, `counted_at`, `completed_by FK`, `notes` |
| `stock_count_items` | Expected/count/variance snapshot | `id`, `stock_count_id FK NN`, `product_id FK NN`, `expected_micros NN`, `counted_micros NN`, `variance_micros NN`, `movement_id FK`; UQ(count,product), check variance=counted-expected |
| `wastage` | Wastage event | `id`, `business_id FK NN`, `location_id FK NN`, `reason NN`, `occurred_at NN`, `user_id FK NN`, `notes` |
| `wastage_items` | Wasted products | `id`, `wastage_id FK NN`, `product_id FK NN`, `quantity_micros NN >0`, `movement_id FK NN`; UQ(wastage,product) |

## Sales, payments and refunds

| Table | Purpose | Columns and constraints |
|---|---|---|
| `customers` | Optional local customer | `id`, `business_id FK NN`, `name NN`, contact fields, `active`; index(business,name) |
| `shifts` | Cashier/terminal period | `id`, `business_id FK NN`, `terminal_id FK NN`, `user_id FK NN`, `status CHECK OPEN/CLOSED`, opening/expected/actual/variance cash integers, `opened_at NN`, `closed_at`, `closed_by FK`; one open shift per terminal/user via partial indexes |
| `sales` | Sale header | `id`, `business_id FK NN`, `sale_number NN`, `idempotency_key NN`, `department_id FK`, `terminal_id FK NN`, `shift_id FK NN`, `customer_id FK`, `cashier_id FK NN`, `status CHECK DRAFT/COMPLETED/VOID/PART_REFUNDED/REFUNDED`, money totals NN, `currency NN`, `completed_at`; UQ(business,sale_number), UQ(business,idempotency_key) |
| `sale_items` | Immutable item/price/tax snapshot | `id`, `sale_id FK NN`, `sellable_id FK NN`, `department_id FK NN`, `description NN`, `kind NN`, `quantity_micros NN >0`, unit/discount/tax/line money NN, `tax_rate_ppm NN`, `metadata_json`; index(sale) |
| `payment_methods` | Configurable method | `id`, `business_id FK NN`, `code NN`, `name NN`, `kind CHECK CASH/CARD/EFT/OTHER`, `active`; UQ(business,code) |
| `payments` | Payment allocation | `id`, `business_id FK NN`, `sale_id FK NN`, `method_id FK NN`, `amount_minor NN >0`, `tendered_minor`, `change_minor DEFAULT 0`, `status CHECK RECORDED/REVERSED`, `reference`, `created_by FK NN`, `created_at NN`; cash/service checks |
| `refunds` | Append-only reversal event | `id`, `business_id FK NN`, `sale_id FK NN`, `refund_number NN`, `reason NN`, `total_minor NN >0`, `status CHECK COMPLETED/VOID`, `created_by FK NN`, `created_at NN`; UQ(business,refund_number) |
| `refund_items` | Original-line reversal | `id`, `refund_id FK NN`, `sale_item_id FK NN`, `quantity_micros NN >0`, `amount_minor NN >0`, `restore_stock NN`, `location_id FK`; cumulative limits enforced transactionally |
| `refund_payments` | Reversal allocation trace | `id`, `refund_id FK NN`, `payment_id FK NN`, `amount_minor NN >0`, `reference`; cumulative limits enforced transactionally |

## Expenses and audit

| Table | Purpose | Columns and constraints |
|---|---|---|
| `expense_categories` | Configurable expense grouping | `id`, `business_id FK NN`, `name NN`, `active`; UQ(business,name) |
| `expenses` | Basic expense record | `id`, `business_id FK NN`, `department_id FK`, `category_id FK NN`, `payment_method_id FK`, `amount_minor NN >0`, `currency NN`, `expense_date NN`, `description NN`, `reference`, `created_by FK NN`, `status CHECK RECORDED/VOID` |
| `audit_logs` | Append-only important-action log | `id`, `business_id FK NN`, `user_id FK`, `terminal_id FK`, `action NN`, `entity_type NN`, `entity_id NN`, `occurred_at NN`, `reason`, `old_json`, `new_json`, `correlation_id NN`; indexes(business,time), (entity_type,entity_id) |

## Service-enforced cross-row invariants

SQLite checks cannot safely express every cross-row rule. Transactional Rust services enforce business ownership alignment; product/service subtype correctness; exact payment equality; cash change rules; stock availability; recipe conversion; cumulative refund limits; refund allocation equality; one coherent transfer pair; count variance movement; immutable completed events; and balance-cache equality with movement sums. Integration tests intentionally violate each invariant.
