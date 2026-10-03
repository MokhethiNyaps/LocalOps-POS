.bail on
PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;
DELETE FROM refund_payments WHERE refund_id IN (SELECT id FROM refunds WHERE sale_id GLOB 'qa-sale-*');
DELETE FROM refunds WHERE sale_id GLOB 'qa-sale-*';
DELETE FROM payments WHERE sale_id GLOB 'qa-sale-*';
DELETE FROM sales WHERE id GLOB 'qa-sale-*';
DELETE FROM shifts WHERE id GLOB 'qa-shift-*';
DELETE FROM inventory_movements WHERE id GLOB 'qa-*';
DELETE FROM inventory_balances WHERE product_id GLOB 'qa-*' AND NOT EXISTS (
  SELECT 1 FROM inventory_movements m WHERE m.product_id = inventory_balances.product_id
);
DELETE FROM department_sellables WHERE sellable_id GLOB 'qa-*';
DELETE FROM sellable_items WHERE id GLOB 'qa-*'
  AND NOT EXISTS (SELECT 1 FROM sale_items s WHERE s.sellable_id = sellable_items.id)
  AND NOT EXISTS (SELECT 1 FROM inventory_movements m WHERE m.product_id = sellable_items.id);
-- Keep referenced identities for receipts and subsequent staff history.
UPDATE sellable_items SET active = 0 WHERE id GLOB 'qa-*';
DELETE FROM categories WHERE id GLOB 'qa-*';
DELETE FROM units WHERE id GLOB 'qa-*'
  AND NOT EXISTS (SELECT 1 FROM products p WHERE p.base_unit_id = units.id);
DELETE FROM sessions WHERE user_id = 'qa-user-demo-cashier';
DELETE FROM users WHERE id = 'qa-user-demo-cashier';
DELETE FROM payment_methods WHERE id GLOB 'qa-pay-*'
  AND NOT EXISTS (SELECT 1 FROM payments p WHERE p.method_id = payment_methods.id);
DELETE FROM terminals WHERE id GLOB 'qa-terminal-*'
  AND NOT EXISTS (SELECT 1 FROM sessions s WHERE s.terminal_id = terminals.id)
  AND NOT EXISTS (SELECT 1 FROM shifts s WHERE s.terminal_id = terminals.id);
UPDATE terminals SET active = 0 WHERE id GLOB 'qa-terminal-*';
DELETE FROM departments WHERE id GLOB 'qa-dept-*'
  AND NOT EXISTS (SELECT 1 FROM terminals t WHERE t.department_id = departments.id)
  AND NOT EXISTS (SELECT 1 FROM sale_items s WHERE s.department_id = departments.id);
UPDATE departments SET active = 0 WHERE id GLOB 'qa-dept-*';
COMMIT;
PRAGMA quick_check;
PRAGMA foreign_key_check;
SELECT 'remaining_seeded_sales', count(*) FROM sales WHERE id GLOB 'qa-*';
SELECT 'remaining_active_seeded_items', count(*) FROM sellable_items WHERE id GLOB 'qa-*' AND active = 1;
SELECT 'preserved_nonseeded_sales', count(*) FROM sales WHERE id NOT GLOB 'qa-*';
