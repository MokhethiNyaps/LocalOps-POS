CREATE UNIQUE INDEX idx_shifts_one_open_per_terminal
  ON shifts(terminal_id)
  WHERE status = 'OPEN';

ALTER TABLE expenses ADD COLUMN shift_id TEXT REFERENCES shifts(id) ON DELETE RESTRICT;
ALTER TABLE expenses ADD COLUMN terminal_id TEXT REFERENCES terminals(id) ON DELETE RESTRICT;
ALTER TABLE expenses ADD COLUMN idempotency_key TEXT;
ALTER TABLE expenses ADD COLUMN voided_at TEXT;
ALTER TABLE expenses ADD COLUMN void_reason TEXT;

CREATE UNIQUE INDEX idx_expenses_idempotency
  ON expenses(business_id, idempotency_key)
  WHERE idempotency_key IS NOT NULL;
CREATE INDEX idx_expenses_shift ON expenses(shift_id);
