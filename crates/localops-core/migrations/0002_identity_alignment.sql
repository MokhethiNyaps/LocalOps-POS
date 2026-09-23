-- Reconcile the executable V1 schema with the approved terminal and role model.
ALTER TABLE terminals ADD COLUMN department_id TEXT REFERENCES departments(id) ON DELETE SET NULL;
ALTER TABLE terminals ADD COLUMN device_key TEXT;

UPDATE terminals
SET device_key = COALESCE(NULLIF(trim(code), ''), id)
WHERE device_key IS NULL;

CREATE UNIQUE INDEX uq_terminals_device_key ON terminals(device_key);
CREATE UNIQUE INDEX uq_terminals_business_name ON terminals(business_id, name);
CREATE INDEX idx_terminals_department ON terminals(department_id);
CREATE UNIQUE INDEX uq_locations_business_name ON locations(business_id, name);

CREATE TRIGGER terminals_device_key_required_insert
BEFORE INSERT ON terminals
WHEN NEW.device_key IS NULL OR length(trim(NEW.device_key)) = 0
BEGIN
  SELECT RAISE(ABORT, 'terminal device_key is required');
END;

CREATE TRIGGER terminals_device_key_required_update
BEFORE UPDATE OF device_key ON terminals
WHEN NEW.device_key IS NULL OR length(trim(NEW.device_key)) = 0
BEGIN
  SELECT RAISE(ABORT, 'terminal device_key is required');
END;

-- Preserve assignments created through the legacy single-role column while the
-- executable model moves exclusively to the approved many-to-many join table.
INSERT OR IGNORE INTO user_roles(user_id, role_id)
SELECT u.id, u.role_id
FROM users u
JOIN roles r ON r.id = u.role_id AND r.business_id = u.business_id
WHERE u.role_id IS NOT NULL;
