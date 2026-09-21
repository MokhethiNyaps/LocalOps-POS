CREATE TABLE schema_migrations (
  version INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  checksum TEXT NOT NULL,
  applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;

CREATE TABLE businesses (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL CHECK (length(trim(name)) > 0),
  trading_name TEXT,
  currency TEXT NOT NULL DEFAULT 'ZAR' CHECK (length(currency) = 3),
  timezone TEXT NOT NULL DEFAULT 'Africa/Johannesburg',
  tax_enabled INTEGER NOT NULL DEFAULT 0 CHECK (tax_enabled IN (0,1)),
  tax_name TEXT,
  tax_rate_ppm INTEGER NOT NULL DEFAULT 0 CHECK (tax_rate_ppm BETWEEN 0 AND 1000000),
  prices_include_tax INTEGER NOT NULL DEFAULT 1 CHECK (prices_include_tax IN (0,1)),
  negative_stock_policy TEXT NOT NULL DEFAULT 'BLOCK' CHECK (negative_stock_policy IN ('BLOCK','OVERRIDE')),
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;

CREATE TABLE locations (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  name TEXT NOT NULL CHECK (length(trim(name)) > 0),
  description TEXT,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE (business_id, name)
) STRICT;
CREATE INDEX idx_locations_business ON locations(business_id);

CREATE TABLE departments (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  name TEXT NOT NULL CHECK (length(trim(name)) > 0),
  description TEXT,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE (business_id, name)
) STRICT;
CREATE INDEX idx_departments_business ON departments(business_id);

CREATE TABLE department_locations (
  department_id TEXT NOT NULL REFERENCES departments(id) ON DELETE RESTRICT,
  location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
  is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0,1)),
  PRIMARY KEY (department_id, location_id)
) STRICT, WITHOUT ROWID;
CREATE UNIQUE INDEX uq_department_default_location ON department_locations(department_id) WHERE is_default = 1;

CREATE TABLE terminals (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  name TEXT NOT NULL CHECK (length(trim(name)) > 0),
  department_id TEXT REFERENCES departments(id) ON DELETE SET NULL,
  location_id TEXT REFERENCES locations(id) ON DELETE SET NULL,
  device_key TEXT NOT NULL,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE (business_id, name),
  UNIQUE (device_key)
) STRICT;
CREATE INDEX idx_terminals_business ON terminals(business_id);
CREATE INDEX idx_terminals_device_key ON terminals(device_key);

CREATE TABLE settings (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  key TEXT NOT NULL,
  value_json TEXT NOT NULL CHECK (json_valid(value_json)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE (business_id, key)
) STRICT;
CREATE INDEX idx_settings_business ON settings(business_id);

CREATE TABLE users (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  username TEXT NOT NULL,
  display_name TEXT NOT NULL,
  pin_hash TEXT NOT NULL,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  failed_attempts INTEGER NOT NULL DEFAULT 0 CHECK (failed_attempts >= 0),
  locked_until TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE (business_id, username)
) STRICT;
CREATE INDEX idx_users_business ON users(business_id);

CREATE TABLE roles (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  name TEXT NOT NULL,
  system INTEGER NOT NULL DEFAULT 0 CHECK (system IN (0,1)),
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE (business_id, name)
) STRICT;
CREATE INDEX idx_roles_business ON roles(business_id);

CREATE TABLE permissions (
  code TEXT PRIMARY KEY NOT NULL,
  description TEXT NOT NULL
) STRICT;

INSERT INTO permissions (code, description) VALUES
  ('business.manage', 'Manage business settings'),
  ('departments.manage', 'Manage departments'),
  ('locations.manage', 'Manage locations'),
  ('users.manage', 'Manage users and roles'),
  ('products.manage', 'Manage products and services'),
  ('inventory.manage', 'Manage inventory'),
  ('sales.create', 'Create sales'),
  ('sales.void', 'Void sales'),
  ('sales.refund', 'Process refunds'),
  ('payments.record', 'Record payments'),
  ('shifts.manage', 'Manage shifts'),
  ('reports.view', 'View reports'),
  ('expenses.manage', 'Manage expenses');

CREATE TABLE user_roles (
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  role_id TEXT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
  assigned_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  assigned_by TEXT REFERENCES users(id) ON DELETE SET NULL,
  PRIMARY KEY (user_id, role_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE role_permissions (
  role_id TEXT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
  permission_code TEXT NOT NULL REFERENCES permissions(code) ON DELETE CASCADE,
  granted_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  granted_by TEXT REFERENCES users(id) ON DELETE SET NULL,
  PRIMARY KEY (role_id, permission_code)
) STRICT, WITHOUT ROWID;

CREATE TABLE sessions (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  terminal_id TEXT NOT NULL REFERENCES terminals(id) ON DELETE RESTRICT,
  started_at TEXT NOT NULL,
  last_active_at TEXT NOT NULL,
  ended_at TEXT,
  end_reason TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX idx_sessions_user ON sessions(user_id, ended_at);
CREATE INDEX idx_sessions_business ON sessions(business_id);

-- Catalogue and units
CREATE TABLE categories (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  parent_id TEXT REFERENCES categories(id) ON DELETE SET NULL,
  name TEXT NOT NULL,
  description TEXT,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE (business_id, name)
) STRICT;
CREATE INDEX idx_categories_business ON categories(business_id);

CREATE TABLE department_categories (
  department_id TEXT NOT NULL REFERENCES departments(id) ON DELETE CASCADE,
  category_id TEXT NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
  PRIMARY KEY (department_id, category_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE units (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  code TEXT NOT NULL,
  name TEXT NOT NULL,
  dimension TEXT NOT NULL,
  scale_num INTEGER NOT NULL CHECK (scale_num > 0),
  scale_den INTEGER NOT NULL CHECK (scale_den > 0),
  decimal_places INTEGER NOT NULL DEFAULT 0 CHECK (decimal_places BETWEEN 0 AND 6),
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE (business_id, code)
) STRICT;
CREATE INDEX idx_units_business ON units(business_id);

-- Sellable items (shared identity for products and services)
CREATE TABLE sellable_items (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  kind TEXT NOT NULL CHECK (kind IN ('PRODUCT', 'SERVICE')),
  name TEXT NOT NULL,
  category_id TEXT REFERENCES categories(id) ON DELETE SET NULL,
  price_minor INTEGER NOT NULL DEFAULT 0 CHECK (price_minor >= 0),
  taxable INTEGER NOT NULL DEFAULT 1 CHECK (taxable IN (0,1)),
  barcode TEXT,
  sku TEXT,
  product_code TEXT,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX idx_sellable_items_business ON sellable_items(business_id);
CREATE INDEX idx_sellable_items_kind ON sellable_items(kind);
CREATE UNIQUE INDEX idx_sellable_items_barcode ON sellable_items(business_id, barcode) WHERE barcode IS NOT NULL;
CREATE UNIQUE INDEX idx_sellable_items_sku ON sellable_items(business_id, sku) WHERE sku IS NOT NULL;
CREATE UNIQUE INDEX idx_sellable_items_product_code ON sellable_items(business_id, product_code) WHERE product_code IS NOT NULL;

-- Product-specific data
CREATE TABLE products (
  sellable_id TEXT PRIMARY KEY NOT NULL REFERENCES sellable_items(id) ON DELETE CASCADE,
  base_unit_id TEXT NOT NULL REFERENCES units(id) ON DELETE RESTRICT,
  cost_minor INTEGER NOT NULL DEFAULT 0 CHECK (cost_minor >= 0),
  track_stock INTEGER NOT NULL DEFAULT 1 CHECK (track_stock IN (0,1)),
  minimum_quantity_micros INTEGER NOT NULL DEFAULT 0 CHECK (minimum_quantity_micros >= 0)
) STRICT;

-- Service-specific data
CREATE TABLE services (
  sellable_id TEXT PRIMARY KEY NOT NULL REFERENCES sellable_items(id) ON DELETE CASCADE,
  duration_minutes INTEGER NOT NULL DEFAULT 0 CHECK (duration_minutes >= 0)
) STRICT;

-- Department-sellable availability
CREATE TABLE department_sellables (
  department_id TEXT NOT NULL REFERENCES departments(id) ON DELETE CASCADE,
  sellable_id TEXT NOT NULL REFERENCES sellable_items(id) ON DELETE CASCADE,
  price_override_minor INTEGER CHECK (price_override_minor >= 0),
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  PRIMARY KEY (department_id, sellable_id)
) STRICT, WITHOUT ROWID;

-- Product packaging/conversions
CREATE TABLE product_packaging (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  product_id TEXT NOT NULL REFERENCES products(sellable_id) ON DELETE CASCADE,
  unit_id TEXT NOT NULL REFERENCES units(id) ON DELETE RESTRICT,
  name TEXT NOT NULL,
  factor_num INTEGER NOT NULL CHECK (factor_num > 0),
  factor_den INTEGER NOT NULL CHECK (factor_den > 0),
  can_purchase INTEGER NOT NULL DEFAULT 1 CHECK (can_purchase IN (0,1)),
  can_sell INTEGER NOT NULL DEFAULT 1 CHECK (can_sell IN (0,1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE (product_id, name)
) STRICT;
CREATE INDEX idx_product_packaging_product ON product_packaging(product_id);

-- Recipes (for products/services that consume ingredients)
CREATE TABLE recipes (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  owner_sellable_id TEXT NOT NULL REFERENCES sellable_items(id) ON DELETE RESTRICT,
  yield_quantity_micros INTEGER NOT NULL CHECK (yield_quantity_micros > 0),
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX idx_recipes_business ON recipes(business_id);
CREATE UNIQUE INDEX idx_recipes_owner_active ON recipes(owner_sellable_id) WHERE active = 1;

CREATE TABLE recipe_items (
  id TEXT PRIMARY KEY NOT NULL,
  recipe_id TEXT NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
  ingredient_product_id TEXT NOT NULL REFERENCES products(sellable_id) ON DELETE RESTRICT,
  quantity_micros INTEGER NOT NULL CHECK (quantity_micros > 0),
  unit_id TEXT NOT NULL REFERENCES units(id) ON DELETE RESTRICT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE (recipe_id, ingredient_product_id)
) STRICT;
CREATE INDEX idx_recipe_items_recipe ON recipe_items(recipe_id);

-- Inventory
CREATE TABLE suppliers (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  name TEXT NOT NULL,
  contact_name TEXT,
  email TEXT,
  phone TEXT,
  address TEXT,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE (business_id, name)
) STRICT;
CREATE INDEX idx_suppliers_business ON suppliers(business_id);

CREATE TABLE purchases (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  purchase_number TEXT NOT NULL,
  supplier_id TEXT REFERENCES suppliers(id) ON DELETE SET NULL,
  location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
  status TEXT NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT', 'RECEIVED', 'VOID')),
  invoice_reference TEXT,
  purchased_at TEXT NOT NULL,
  total_minor INTEGER NOT NULL DEFAULT 0 CHECK (total_minor >= 0),
  tax_minor INTEGER NOT NULL DEFAULT 0 CHECK (tax_minor >= 0),
  discount_minor INTEGER NOT NULL DEFAULT 0 CHECK (discount_minor >= 0),
  notes TEXT,
  created_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE (business_id, purchase_number)
) STRICT;
CREATE INDEX idx_purchases_business ON purchases(business_id);
CREATE INDEX idx_purchases_supplier ON purchases(supplier_id);
CREATE INDEX idx_purchases_location ON purchases(location_id);

CREATE TABLE purchase_items (
  id TEXT PRIMARY KEY NOT NULL,
  purchase_id TEXT NOT NULL REFERENCES purchases(id) ON DELETE CASCADE,
  product_id TEXT NOT NULL REFERENCES products(sellable_id) ON DELETE RESTRICT,
  description TEXT NOT NULL,
  quantity_micros INTEGER NOT NULL CHECK (quantity_micros > 0),
  unit_id TEXT NOT NULL REFERENCES units(id) ON DELETE RESTRICT,
  base_quantity_micros INTEGER NOT NULL CHECK (base_quantity_micros > 0),
  unit_cost_minor INTEGER NOT NULL DEFAULT 0 CHECK (unit_cost_minor >= 0),
  line_total_minor INTEGER NOT NULL DEFAULT 0 CHECK (line_total_minor >= 0)
) STRICT;
CREATE INDEX idx_purchase_items_purchase ON purchase_items(purchase_id);

-- Inventory balances (cached, rebuildable from movements)
CREATE TABLE inventory_balances (
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  product_id TEXT NOT NULL REFERENCES products(sellable_id) ON DELETE RESTRICT,
  location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
  quantity_micros INTEGER NOT NULL DEFAULT 0,
  version INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  PRIMARY KEY (business_id, product_id, location_id)
) STRICT, WITHOUT ROWID;

-- Inventory movements (authoritative ledger)
CREATE TABLE inventory_movements (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  product_id TEXT NOT NULL REFERENCES products(sellable_id) ON DELETE RESTRICT,
  location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
  quantity_micros INTEGER NOT NULL CHECK (quantity_micros != 0),
  type TEXT NOT NULL CHECK (type IN ('PURCHASE', 'SALE', 'WASTE', 'DAMAGE', 'ADJUSTMENT', 'TRANSFER_OUT', 'TRANSFER_IN', 'COUNT', 'OPENING', 'CONSUMPTION')),
  occurred_at TEXT NOT NULL,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  reference_type TEXT NOT NULL,
  reference_id TEXT NOT NULL,
  correlation_id TEXT NOT NULL,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX idx_inventory_movements_product_location ON inventory_movements(product_id, location_id);
CREATE INDEX idx_inventory_movements_time ON inventory_movements(occurred_at);
CREATE INDEX idx_inventory_movements_reference ON inventory_movements(reference_type, reference_id);
CREATE INDEX idx_inventory_movements_correlation ON inventory_movements(correlation_id);

-- Inventory transfers
CREATE TABLE inventory_transfers (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  from_location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
  to_location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
  status TEXT NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT', 'COMPLETED', 'VOID')),
  completed_at TEXT,
  completed_by TEXT REFERENCES users(id) ON DELETE SET NULL,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  CHECK (from_location_id != to_location_id)
) STRICT;
CREATE INDEX idx_transfers_business ON inventory_transfers(business_id);

CREATE TABLE inventory_transfer_items (
  id TEXT PRIMARY KEY NOT NULL,
  transfer_id TEXT NOT NULL REFERENCES inventory_transfers(id) ON DELETE CASCADE,
  product_id TEXT NOT NULL REFERENCES products(sellable_id) ON DELETE RESTRICT,
  quantity_micros INTEGER NOT NULL CHECK (quantity_micros > 0),
  UNIQUE (transfer_id, product_id)
) STRICT;
CREATE INDEX idx_transfer_items_transfer ON inventory_transfer_items(transfer_id);

-- Stock counts
CREATE TABLE stock_counts (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
  status TEXT NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT', 'COMPLETED', 'VOID')),
  counted_at TEXT,
  completed_by TEXT REFERENCES users(id) ON DELETE SET NULL,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX idx_stock_counts_business ON stock_counts(business_id);
CREATE INDEX idx_stock_counts_location ON stock_counts(location_id);

CREATE TABLE stock_count_items (
  id TEXT PRIMARY KEY NOT NULL,
  stock_count_id TEXT NOT NULL REFERENCES stock_counts(id) ON DELETE CASCADE,
  product_id TEXT NOT NULL REFERENCES products(sellable_id) ON DELETE RESTRICT,
  expected_micros INTEGER NOT NULL,
  counted_micros INTEGER NOT NULL,
  variance_micros INTEGER NOT NULL,
  movement_id TEXT NOT NULL REFERENCES inventory_movements(id) ON DELETE RESTRICT,
  UNIQUE (stock_count_id, product_id),
  CHECK (variance_micros = counted_micros - expected_micros)
) STRICT;
CREATE INDEX idx_stock_count_items_count ON stock_count_items(stock_count_id);

-- Wastage
CREATE TABLE wastage (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
  reason TEXT NOT NULL,
  occurred_at TEXT NOT NULL,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX idx_wastage_business ON wastage(business_id);

CREATE TABLE wastage_items (
  id TEXT PRIMARY KEY NOT NULL,
  wastage_id TEXT NOT NULL REFERENCES wastage(id) ON DELETE CASCADE,
  product_id TEXT NOT NULL REFERENCES products(sellable_id) ON DELETE RESTRICT,
  quantity_micros INTEGER NOT NULL CHECK (quantity_micros > 0),
  movement_id TEXT NOT NULL REFERENCES inventory_movements(id) ON DELETE RESTRICT,
  UNIQUE (wastage_id, product_id)
) STRICT;
CREATE INDEX idx_wastage_items_wastage ON wastage_items(wastage_id);

-- Sales, payments and refunds
CREATE TABLE customers (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  name TEXT NOT NULL,
  email TEXT,
  phone TEXT,
  address TEXT,
  notes TEXT,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX idx_customers_business ON customers(business_id);
CREATE INDEX idx_customers_name ON customers(business_id, name);

CREATE TABLE shifts (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  terminal_id TEXT NOT NULL REFERENCES terminals(id) ON DELETE RESTRICT,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  status TEXT NOT NULL DEFAULT 'OPEN' CHECK (status IN ('OPEN', 'CLOSED')),
  opening_balance_minor INTEGER NOT NULL DEFAULT 0 CHECK (opening_balance_minor >= 0),
  expected_balance_minor INTEGER NOT NULL DEFAULT 0 CHECK (expected_balance_minor >= 0),
  actual_balance_minor INTEGER,
  variance_minor INTEGER,
  opened_at TEXT NOT NULL,
  closed_at TEXT,
  closed_by TEXT REFERENCES users(id) ON DELETE SET NULL,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX idx_shifts_terminal ON shifts(terminal_id, status);
CREATE INDEX idx_shifts_user ON shifts(user_id);
CREATE INDEX idx_shifts_business ON shifts(business_id);

CREATE TABLE sales (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  sale_number TEXT NOT NULL,
  idempotency_key TEXT NOT NULL,
  department_id TEXT REFERENCES departments(id) ON DELETE SET NULL,
  terminal_id TEXT NOT NULL REFERENCES terminals(id) ON DELETE RESTRICT,
  shift_id TEXT NOT NULL REFERENCES shifts(id) ON DELETE RESTRICT,
  customer_id TEXT REFERENCES customers(id) ON DELETE SET NULL,
  cashier_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  status TEXT NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT', 'COMPLETED', 'VOID', 'PART_REFUNDED', 'REFUNDED')),
  subtotal_minor INTEGER NOT NULL DEFAULT 0 CHECK (subtotal_minor >= 0),
  discount_minor INTEGER NOT NULL DEFAULT 0 CHECK (discount_minor >= 0),
  tax_minor INTEGER NOT NULL DEFAULT 0 CHECK (tax_minor >= 0),
  total_minor INTEGER NOT NULL DEFAULT 0 CHECK (total_minor >= 0),
  amount_paid_minor INTEGER NOT NULL DEFAULT 0 CHECK (amount_paid_minor >= 0),
  change_due_minor INTEGER NOT NULL DEFAULT 0 CHECK (change_due_minor >= 0),
  currency TEXT NOT NULL DEFAULT 'ZAR',
  completed_at TEXT,
  voided_at TEXT,
  void_reason TEXT,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE (business_id, sale_number),
  UNIQUE (business_id, idempotency_key)
) STRICT;
CREATE INDEX idx_sales_business ON sales(business_id);
CREATE INDEX idx_sales_terminal ON sales(terminal_id);
CREATE INDEX idx_sales_shift ON sales(shift_id);
CREATE INDEX idx_sales_cashier ON sales(cashier_id);
CREATE INDEX idx_sales_status ON sales(status);

CREATE TABLE sale_items (
  id TEXT PRIMARY KEY NOT NULL,
  sale_id TEXT NOT NULL REFERENCES sales(id) ON DELETE CASCADE,
  sellable_id TEXT NOT NULL REFERENCES sellable_items(id) ON DELETE RESTRICT,
  department_id TEXT NOT NULL REFERENCES departments(id) ON DELETE RESTRICT,
  description TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('PRODUCT', 'SERVICE')),
  quantity_micros INTEGER NOT NULL CHECK (quantity_micros > 0),
  unit_price_minor INTEGER NOT NULL DEFAULT 0 CHECK (unit_price_minor >= 0),
  discount_minor INTEGER NOT NULL DEFAULT 0 CHECK (discount_minor >= 0),
  tax_rate_ppm INTEGER NOT NULL DEFAULT 0 CHECK (tax_rate_ppm BETWEEN 0 AND 1000000),
  tax_minor INTEGER NOT NULL DEFAULT 0 CHECK (tax_minor >= 0),
  line_total_minor INTEGER NOT NULL DEFAULT 0 CHECK (line_total_minor >= 0),
  cost_minor INTEGER NOT NULL DEFAULT 0 CHECK (cost_minor >= 0),
  metadata_json TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX idx_sale_items_sale ON sale_items(sale_id);

CREATE TABLE payment_methods (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  code TEXT NOT NULL,
  name TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('CASH', 'CARD', 'EFT', 'OTHER')),
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE (business_id, code)
) STRICT;
CREATE INDEX idx_payment_methods_business ON payment_methods(business_id);

CREATE TABLE payments (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  sale_id TEXT NOT NULL REFERENCES sales(id) ON DELETE RESTRICT,
  method_id TEXT NOT NULL REFERENCES payment_methods(id) ON DELETE RESTRICT,
  amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
  tendered_minor INTEGER CHECK (tendered_minor >= 0),
  change_minor INTEGER NOT NULL DEFAULT 0 CHECK (change_minor >= 0),
  status TEXT NOT NULL DEFAULT 'RECORDED' CHECK (status IN ('RECORDED', 'REVERSED')),
  reference TEXT,
  created_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX idx_payments_sale ON payments(sale_id);
CREATE INDEX idx_payments_method ON payments(method_id);
CREATE INDEX idx_payments_business ON payments(business_id);

CREATE TABLE refunds (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  sale_id TEXT NOT NULL REFERENCES sales(id) ON DELETE RESTRICT,
  refund_number TEXT NOT NULL,
  reason TEXT NOT NULL,
  total_minor INTEGER NOT NULL CHECK (total_minor > 0),
  status TEXT NOT NULL DEFAULT 'COMPLETED' CHECK (status IN ('COMPLETED', 'VOID')),
  created_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  voided_at TEXT,
  void_reason TEXT,
  UNIQUE (business_id, refund_number)
) STRICT;
CREATE INDEX idx_refunds_sale ON refunds(sale_id);
CREATE INDEX idx_refunds_business ON refunds(business_id);

CREATE TABLE refund_items (
  id TEXT PRIMARY KEY NOT NULL,
  refund_id TEXT NOT NULL REFERENCES refunds(id) ON DELETE CASCADE,
  sale_item_id TEXT NOT NULL REFERENCES sale_items(id) ON DELETE RESTRICT,
  quantity_micros INTEGER NOT NULL CHECK (quantity_micros > 0),
  amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
  restore_stock INTEGER NOT NULL DEFAULT 0 CHECK (restore_stock IN (0,1)),
  location_id TEXT REFERENCES locations(id) ON DELETE SET NULL,
  movement_id TEXT REFERENCES inventory_movements(id) ON DELETE SET NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX idx_refund_items_refund ON refund_items(refund_id);

CREATE TABLE refund_payments (
  id TEXT PRIMARY KEY NOT NULL,
  refund_id TEXT NOT NULL REFERENCES refunds(id) ON DELETE CASCADE,
  payment_id TEXT NOT NULL REFERENCES payments(id) ON DELETE RESTRICT,
  amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
  reference TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX idx_refund_payments_refund ON refund_payments(refund_id);

-- Expenses and audit
CREATE TABLE expense_categories (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  name TEXT NOT NULL,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE (business_id, name)
) STRICT;
CREATE INDEX idx_expense_categories_business ON expense_categories(business_id);

CREATE TABLE expenses (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  department_id TEXT REFERENCES departments(id) ON DELETE SET NULL,
  category_id TEXT NOT NULL REFERENCES expense_categories(id) ON DELETE RESTRICT,
  payment_method_id TEXT REFERENCES payment_methods(id) ON DELETE SET NULL,
  amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
  currency TEXT NOT NULL DEFAULT 'ZAR',
  expense_date TEXT NOT NULL,
  description TEXT NOT NULL,
  reference TEXT,
  receipt_image_path TEXT,
  status TEXT NOT NULL DEFAULT 'RECORDED' CHECK (status IN ('RECORDED', 'VOID')),
  created_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX idx_expenses_business ON expenses(business_id);
CREATE INDEX idx_expenses_category ON expenses(category_id);
CREATE INDEX idx_expenses_department ON expenses(department_id);

CREATE TABLE audit_logs (
  id TEXT PRIMARY KEY NOT NULL,
  business_id TEXT NOT NULL REFERENCES businesses(id) ON DELETE RESTRICT,
  user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
  terminal_id TEXT REFERENCES terminals(id) ON DELETE SET NULL,
  action TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  occurred_at TEXT NOT NULL,
  reason TEXT,
  old_json TEXT,
  new_json TEXT,
  correlation_id TEXT NOT NULL
) STRICT;
CREATE INDEX idx_audit_logs_business_time ON audit_logs(business_id, occurred_at);
CREATE INDEX idx_audit_logs_entity ON audit_logs(entity_type, entity_id);
