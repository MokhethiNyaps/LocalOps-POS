ALTER TABLE refunds ADD COLUMN idempotency_key TEXT;
ALTER TABLE refunds ADD COLUMN shift_id TEXT REFERENCES shifts(id) ON DELETE RESTRICT;
ALTER TABLE refunds ADD COLUMN terminal_id TEXT REFERENCES terminals(id) ON DELETE RESTRICT;
ALTER TABLE refunds ADD COLUMN refunded_at TEXT;

CREATE UNIQUE INDEX idx_refunds_idempotency
  ON refunds(business_id, idempotency_key)
  WHERE idempotency_key IS NOT NULL;

CREATE TABLE sale_item_consumptions (
  sale_item_id TEXT NOT NULL REFERENCES sale_items(id) ON DELETE CASCADE,
  product_id TEXT NOT NULL REFERENCES products(sellable_id) ON DELETE RESTRICT,
  location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
  quantity_micros INTEGER NOT NULL CHECK (quantity_micros > 0),
  PRIMARY KEY (sale_item_id, product_id, location_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE refund_item_consumptions (
  id TEXT PRIMARY KEY NOT NULL,
  refund_item_id TEXT NOT NULL REFERENCES refund_items(id) ON DELETE CASCADE,
  product_id TEXT NOT NULL REFERENCES products(sellable_id) ON DELETE RESTRICT,
  location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
  quantity_micros INTEGER NOT NULL CHECK (quantity_micros > 0),
  movement_id TEXT NOT NULL REFERENCES inventory_movements(id) ON DELETE RESTRICT,
  UNIQUE (refund_item_id, product_id, location_id)
) STRICT;

CREATE INDEX idx_sale_item_consumptions_product
  ON sale_item_consumptions(product_id, location_id);
CREATE INDEX idx_refund_item_consumptions_item
  ON refund_item_consumptions(refund_item_id);
