-- Owner/Cashier access control: stable system role keys, narrow permissions,
-- deterministic Owner grants, a whitelist Cashier system role, and legacy
-- permission compatibility. Safe for existing business data and fully offline.

-- 1. Stable machine role identity -------------------------------------------
ALTER TABLE roles ADD COLUMN role_key TEXT;

UPDATE roles
SET role_key = 'OWNER'
WHERE system = 1 AND role_key IS NULL AND upper(trim(name)) = 'OWNER';

UPDATE roles
SET role_key = 'CASHIER'
WHERE system = 1 AND role_key IS NULL AND upper(trim(name)) = 'CASHIER';

CREATE UNIQUE INDEX uq_roles_business_role_key
  ON roles(business_id, role_key)
  WHERE role_key IS NOT NULL;

-- 2. Narrow permission vocabulary -------------------------------------------
INSERT OR IGNORE INTO permissions (code, description) VALUES
  ('terminals.manage', 'Manage terminals'),
  ('roles.manage', 'Manage roles and role assignments'),
  ('backups.manage', 'Create, verify and restore local backups'),
  ('products.cost.view', 'View product cost and margin information'),
  ('pos.catalog.view', 'View the point-of-sale sellable catalogue'),
  ('inventory.quantity.view', 'View inventory quantities'),
  ('inventory.value.view', 'View inventory valuation and costs'),
  ('suppliers.manage', 'View and manage suppliers'),
  ('purchases.manage', 'View and manage purchases'),
  ('sales.view_own_current_shift', 'View own sales for the current open shift'),
  ('sales.view_all', 'View all business sales'),
  ('sales.discount', 'Apply discounts to sales'),
  ('sales.price_override', 'Override selling prices'),
  ('receipts.reprint_own_current_shift', 'Reprint own current-shift receipts'),
  ('receipts.reprint_all', 'Reprint any receipt'),
  ('shifts.open_own', 'Open own shift'),
  ('shifts.close_own', 'Close own shift'),
  ('shifts.close_any', 'Close any open shift in the business'),
  ('shifts.view_own_current', 'View own current shift'),
  ('shifts.view_all', 'View all shifts and shift history'),
  ('shifts.expected_cash.view', 'View expected shift cash'),
  ('shifts.variance.view', 'View shift cash variance'),
  ('expenses.view', 'View expenses'),
  ('reports.export', 'Export reports'),
  ('audit.view', 'View audit history');

-- 3. Legacy compatibility: preserve explicitly granted management authority ---
-- shifts.manage
INSERT OR IGNORE INTO role_permissions (role_id, permission_code)
SELECT rp.role_id, p.code
FROM role_permissions rp
JOIN roles r ON r.id = rp.role_id
JOIN permissions p ON p.code IN (
  'shifts.open_own', 'shifts.close_own', 'shifts.close_any',
  'shifts.view_own_current', 'shifts.view_all',
  'shifts.expected_cash.view', 'shifts.variance.view')
WHERE rp.permission_code = 'shifts.manage'
  AND COALESCE(r.role_key, '') <> 'CASHIER';

-- reports.view
INSERT OR IGNORE INTO role_permissions (role_id, permission_code)
SELECT rp.role_id, 'reports.export'
FROM role_permissions rp
JOIN roles r ON r.id = rp.role_id
WHERE rp.permission_code = 'reports.view'
  AND COALESCE(r.role_key, '') <> 'CASHIER';

-- inventory.manage
INSERT OR IGNORE INTO role_permissions (role_id, permission_code)
SELECT rp.role_id, p.code
FROM role_permissions rp
JOIN roles r ON r.id = rp.role_id
JOIN permissions p ON p.code IN (
  'inventory.quantity.view', 'inventory.value.view',
  'suppliers.manage', 'purchases.manage')
WHERE rp.permission_code = 'inventory.manage'
  AND COALESCE(r.role_key, '') <> 'CASHIER';

-- products.manage
INSERT OR IGNORE INTO role_permissions (role_id, permission_code)
SELECT rp.role_id, p.code
FROM role_permissions rp
JOIN roles r ON r.id = rp.role_id
JOIN permissions p ON p.code IN ('products.cost.view', 'pos.catalog.view')
WHERE rp.permission_code = 'products.manage'
  AND COALESCE(r.role_key, '') <> 'CASHIER';

-- business.manage
INSERT OR IGNORE INTO role_permissions (role_id, permission_code)
SELECT rp.role_id, p.code
FROM role_permissions rp
JOIN roles r ON r.id = rp.role_id
JOIN permissions p ON p.code IN ('backups.manage', 'terminals.manage')
WHERE rp.permission_code = 'business.manage'
  AND COALESCE(r.role_key, '') <> 'CASHIER';

-- users.manage
INSERT OR IGNORE INTO role_permissions (role_id, permission_code)
SELECT rp.role_id, 'roles.manage'
FROM role_permissions rp
JOIN roles r ON r.id = rp.role_id
WHERE rp.permission_code = 'users.manage'
  AND COALESCE(r.role_key, '') <> 'CASHIER';

-- sales.create: an explicit selling role keeps the till capabilities it needs.
INSERT OR IGNORE INTO role_permissions (role_id, permission_code)
SELECT rp.role_id, p.code
FROM role_permissions rp
JOIN roles r ON r.id = rp.role_id
JOIN permissions p ON p.code IN (
  'pos.catalog.view', 'sales.view_own_current_shift',
  'receipts.reprint_own_current_shift', 'shifts.open_own',
  'shifts.close_own', 'shifts.view_own_current')
WHERE rp.permission_code = 'sales.create'
  AND COALESCE(r.role_key, '') <> 'CASHIER';

-- 4. Owner receives every permission ----------------------------------------
INSERT OR IGNORE INTO role_permissions (role_id, permission_code)
SELECT r.id, p.code
FROM roles r
CROSS JOIN permissions p
WHERE r.role_key = 'OWNER';

-- 5. Every business has a whitelist Cashier system role ----------------------
INSERT INTO roles (id, business_id, name, role_key, system)
SELECT
  lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-7' ||
  substr(lower(hex(randomblob(2))), 2) || '-a' ||
  substr(lower(hex(randomblob(2))), 2) || '-' || lower(hex(randomblob(6))),
  b.id,
  'Cashier',
  'CASHIER',
  1
FROM businesses b
WHERE NOT EXISTS (
  SELECT 1 FROM roles r
  WHERE r.business_id = b.id
    AND (r.role_key = 'CASHIER' OR upper(trim(r.name)) = 'CASHIER')
);

INSERT OR IGNORE INTO role_permissions (role_id, permission_code)
SELECT r.id, p.code
FROM roles r
JOIN permissions p ON p.code IN (
  'pos.catalog.view',
  'sales.create',
  'payments.record',
  'sales.view_own_current_shift',
  'receipts.reprint_own_current_shift',
  'shifts.open_own',
  'shifts.close_own',
  'shifts.view_own_current')
WHERE r.role_key = 'CASHIER';

CREATE INDEX idx_role_permissions_permission ON role_permissions(permission_code);
