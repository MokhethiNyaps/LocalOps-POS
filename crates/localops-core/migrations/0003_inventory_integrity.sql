ALTER TABLE purchases ADD COLUMN payment_status TEXT NOT NULL DEFAULT 'UNPAID'
  CHECK (payment_status IN ('UNPAID', 'PARTIAL', 'PAID'));

CREATE UNIQUE INDEX idx_inventory_movements_idempotency
  ON inventory_movements(
    business_id, product_id, location_id, type, reference_type, reference_id
  );
