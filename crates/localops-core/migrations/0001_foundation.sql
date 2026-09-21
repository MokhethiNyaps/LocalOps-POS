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
