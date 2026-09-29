import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { App, findBarcodeItem, hasAnyPermission, hasPermission, hasSystemRole, navigationFor, parseScaled, setupSteps, type PosItem, type SessionView } from './App';

describe('first-run setup', () => {
  it('renders the offline business setup form and roadmap', () => {
    const markup = renderToStaticMarkup(<App />);

    expect(markup).toContain('Business name');
    expect(markup).toContain('Create business');
    expect(markup).toContain('Offline ready');
    for (const step of setupSteps) expect(markup).toContain(step.replace('&', '&amp;'));
  });
});

describe('fixed-precision input', () => {
  it('converts money and quantities without binary floating-point arithmetic', () => {
    expect(parseScaled('25.05', 2)).toBe(2505);
    expect(parseScaled('1.000001', 6)).toBe(1_000_001);
    expect(() => parseScaled('1.0000001', 6)).toThrow('no more than 6');
  });
});

describe('local barcode input', () => {
  it('resolves scanner text against barcode, SKU, and product code without a network', () => {
    const item: PosItem = { id: 'lager', departmentId: 'bar', departmentName: 'Bar', name: 'Lager', kind: 'PRODUCT', priceMinor: 2500, taxable: true, sku: 'SKU-001', productCode: 'BEER-1', barcode: '6001000000012' };
    expect(findBarcodeItem([item], '6001000000012')).toBe(item);
    expect(findBarcodeItem([item], 'sku-001')).toBe(item);
    expect(findBarcodeItem([item], 'unknown')).toBeUndefined();
  });
});

const CASHIER_PERMISSIONS = ['pos.catalog.view', 'sales.create', 'payments.record', 'sales.view_own_current_shift', 'receipts.reprint_own_current_shift', 'shifts.open_own', 'shifts.close_own', 'shifts.view_own_current'];
const OWNER_PERMISSIONS = [...CASHIER_PERMISSIONS, 'business.manage', 'users.manage', 'roles.manage', 'products.manage', 'products.cost.view', 'inventory.manage', 'inventory.quantity.view', 'inventory.value.view', 'suppliers.manage', 'purchases.manage', 'sales.view_all', 'sales.refund', 'sales.void', 'sales.discount', 'sales.price_override', 'receipts.reprint_all', 'shifts.view_all', 'shifts.close_any', 'shifts.expected_cash.view', 'shifts.variance.view', 'expenses.manage', 'expenses.view', 'reports.view', 'reports.export', 'backups.manage', 'audit.view'];

const sessionFor = (permissions: string[], systemRoles: string[], roleLabel: string): SessionView => ({
  sessionId: 'session-1', businessId: 'business-1', businessName: 'Moko Trading', userId: 'user-1', username: 'user',
  displayName: 'User', terminalId: 'terminal-1', terminalName: 'Main Till', departmentId: 'department-1',
  departmentName: 'Shop', permissions, systemRoles, roleLabel,
});

describe('cashier workspace navigation', () => {
  const cashier = sessionFor(CASHIER_PERMISSIONS, ['CASHIER'], 'Cashier');
  const owner = sessionFor(OWNER_PERMISSIONS, ['OWNER'], 'Owner');

  it('shows a cashier only Point of Sale, My Shift and Recent Sales', () => {
    expect(navigationFor(cashier).map(item => item.label)).toEqual(['Point of Sale', 'My Shift', 'Recent Sales']);
  });

  it('never offers a cashier owner-only sections', () => {
    const labels = navigationFor(cashier).map(item => item.label);
    for (const forbidden of ['Catalogue', 'Inventory', 'Shift & expenses', 'Reports', 'Employees', 'Setup tools', 'Backup & safety']) {
      expect(labels).not.toContain(forbidden);
    }
  });

  it('gives the owner the full workspace including employee management', () => {
    const labels = navigationFor(owner).map(item => item.label);
    expect(labels).toContain('Employees');
    expect(labels).toContain('Backup & safety');
    expect(labels).toContain('Reports');
    expect(labels).not.toContain('My Shift');
  });

  it('exposes permission helpers that mirror the backend whitelist', () => {
    expect(hasPermission(cashier, 'sales.create')).toBe(true);
    expect(hasPermission(cashier, 'sales.refund')).toBe(false);
    expect(hasAnyPermission(cashier, ['reports.view', 'shifts.open_own'])).toBe(true);
    expect(hasSystemRole(cashier, 'OWNER')).toBe(false);
    expect(hasSystemRole(owner, 'OWNER')).toBe(true);
  });
});
