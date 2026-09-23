CREATE TABLE stock_count_items_v2 (
  id TEXT PRIMARY KEY NOT NULL,
  stock_count_id TEXT NOT NULL REFERENCES stock_counts(id) ON DELETE CASCADE,
  product_id TEXT NOT NULL REFERENCES products(sellable_id) ON DELETE RESTRICT,
  expected_micros INTEGER NOT NULL,
  counted_micros INTEGER NOT NULL,
  variance_micros INTEGER NOT NULL,
  movement_id TEXT REFERENCES inventory_movements(id) ON DELETE RESTRICT,
  UNIQUE (stock_count_id, product_id),
  CHECK (variance_micros = counted_micros - expected_micros),
  CHECK (
    (variance_micros = 0 AND movement_id IS NULL) OR
    (variance_micros != 0 AND movement_id IS NOT NULL)
  )
) STRICT;

INSERT INTO stock_count_items_v2(
  id, stock_count_id, product_id, expected_micros, counted_micros,
  variance_micros, movement_id
)
SELECT id, stock_count_id, product_id, expected_micros, counted_micros,
       variance_micros, movement_id
FROM stock_count_items;

DROP TABLE stock_count_items;
ALTER TABLE stock_count_items_v2 RENAME TO stock_count_items;
CREATE INDEX idx_stock_count_items_count ON stock_count_items(stock_count_id);
