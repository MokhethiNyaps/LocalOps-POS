// Development-only browser fixture. Not imported by the production entry point.
// Every IPC command is intercepted; this page cannot write to a real POS database.
import React from 'react';
import { createRoot } from 'react-dom/client';
import { mockIPC } from '@tauri-apps/api/mocks';
import { App } from '../src/App';
import '../src/styles.css';

const params = new URLSearchParams(location.search);
const departments = [{ id: 'kitchen', name: 'Kitchen' }, { id: 'bar', name: 'Bar' }, { id: 'salon', name: 'Salon' }];
const products = [
  ['burger', 'Flame Double Cheeseburger', 'kitchen', 8500],
  ['wings', 'Peri-Peri Wings · 8pc', 'kitchen', 7500],
  ['fries', 'Loaded Cheesy Fries', 'kitchen', 4500],
  ['platter', 'Chakalaka Braai Platter', 'kitchen', 16500],
  ['lemonade', 'Craft Lemonade · 500ml', 'bar', 3500],
  ['coffee', 'Cappuccino', 'bar', 3200],
  ['lager', 'House Lager · 330ml', 'bar', 3000],
  ['smash', 'Truffle Aioli Smash Roll', 'kitchen', 9500],
  ['pudding', 'Malva Pudding & Custard', 'kitchen', 4800],
  ['water', 'Still Water · 500ml', 'bar', 1800],
  ['cut', 'Classic Haircut', 'salon', 12000],
  ['treatment', 'Wash & Treatment', 'salon', 15000],
] as const;
const items = products.map(([id, name, departmentId, priceMinor], index) => ({ id, name, departmentId, departmentName: departments.find(d => d.id === departmentId)!.name, priceMinor, taxable: true, kind: departmentId === 'salon' ? 'SERVICE' : 'PRODUCT', sku: 'SKU-' + (index + 1).toString().padStart(3, '0'), productCode: id.toUpperCase(), barcode: '600100000' + index.toString().padStart(4, '0') }));
const paymentMethods = [{ id: 'cash', code: 'CASH', name: 'Cash', kind: 'CASH' }, { id: 'card', code: 'CARD', name: 'Card', kind: 'CARD' }];
let shift = params.has('closed') ? null : { id: 'shift-preview', openingBalanceMinor: 50000, expectedBalanceMinor: 85000, openedAt: new Date().toISOString() };
const receipts: any[] = [];
const commands: { command: string; payload: unknown }[] = [];
const snapshot = () => ({ businessName: 'Moko’s Lifestyle Centre', currency: 'ZAR', taxEnabled: false, taxRatePpm: 0, pricesIncludeTax: true, items: params.has('empty') ? [] : items, paymentMethods, openShift: shift, recentSales: receipts.map(receipt => ({ id: receipt.id, saleNumber: receipt.saleNumber, status: 'COMPLETED', totalMinor: receipt.totalMinor, completedAt: receipt.completedAt })) });
mockIPC((command, payload: any) => {
  commands.push({ command, payload });
  const audit = document.getElementById('preview-audit');
  if (audit) audit.textContent = JSON.stringify(commands, null, 2);
  switch (command) {
    case 'get_app_bootstrap': return { setupRequired: params.has('setup'), businesses: [{ id: 'preview', name: 'Moko’s Lifestyle Centre', terminals: [{ id: 'preview-till', name: 'Main Till — preview', departmentId: null }] }] };
    case 'login': return { businessId: 'preview', terminalId: 'preview-till', departmentId: null, displayName: 'Thabo M.' };
    case 'logout': return null;
    case 'get_pos_snapshot': return snapshot();
    case 'open_pos_shift': shift = { id: 'shift-preview', openingBalanceMinor: payload.input.openingBalanceMinor, expectedBalanceMinor: payload.input.openingBalanceMinor, openedAt: payload.input.openedAt }; return shift;
    case 'get_catalogue_snapshot': return { businessId: 'preview', terminalId: 'preview-till', terminalDepartmentId: null, categories: [], units: [{ id: 'each', code: 'EA', name: 'Each', dimension: 'COUNT', scaleNum: 1, scaleDen: 1, decimalPlaces: 0 }], departments, items: items.map(item => ({ ...item, categoryId: null, baseUnitId: 'each', costMinor: 1200, trackStock: true, durationMinutes: item.kind === 'SERVICE' ? 30 : null, hasRecipe: false, packaging: [] })) };
    case 'complete_pos_sale': {
      if (params.has('fail')) throw new Error('Preview: insufficient stock. Sale was not completed.');
      const lines = payload.input.lines.map((line: any, index: number) => { const item = items.find(item => item.id === line.sellableId)!; return { id: 'line-' + index, description: item.name, kind: item.kind, quantityMicros: line.quantityMicros, unitPriceMinor: item.priceMinor, discountMinor: line.discountMinor, taxMinor: 0, lineTotalMinor: item.priceMinor * line.quantityMicros / 1_000_000 }; });
      const totalMinor = lines.reduce((total: number, line: any) => total + line.lineTotalMinor, 0);
      if (payload.input.payments.reduce((total: number, payment: any) => total + payment.amountMinor, 0) !== totalMinor) throw new Error('Preview: payment allocation mismatch');
      const payments = payload.input.payments.map((payment: any) => ({ id: payment.methodId, methodName: paymentMethods.find(method => method.id === payment.methodId)!.name, methodKind: payment.methodId === 'cash' ? 'CASH' : 'CARD', ...payment, changeMinor: Math.max(0, payment.tenderedMinor - payment.amountMinor), status: 'COMPLETED' }));
      const receipt = { id: 'sale-' + (receipts.length + 1), saleNumber: 'PREVIEW-' + (receipts.length + 1).toString().padStart(4, '0'), currency: 'ZAR', subtotalMinor: totalMinor, discountMinor: 0, taxMinor: 0, totalMinor, amountPaidMinor: totalMinor, changeDueMinor: payments.reduce((total: number, payment: any) => total + payment.changeMinor, 0), completedAt: payload.input.completedAt, lines, payments, idempotentReplay: false };
      receipts.unshift(receipt); return receipt;
    }
    case 'get_sale_receipt': return receipts.find(receipt => receipt.id === payload.saleId);
    case 'create_pos_refund': return { id: 'preview-refund' };
    case 'get_inventory_snapshot': return { locations: [{ id: 'store', name: 'Main Store' }], products: items.filter(item => item.kind === 'PRODUCT').map(item => ({ id: item.id, name: item.name, baseUnitId: 'each', baseUnitCode: 'EA', packaging: [] })), balances: items.filter(item => item.kind === 'PRODUCT').map(item => ({ productId: item.id, productName: item.name, locationId: 'store', locationName: 'Main Store', quantityMicros: 24000000, version: 1 })), suppliers: [], recentMovements: [], reconciliationDifferenceCount: 0 };
    case 'get_operations_snapshot': return { openShift: shift, shifts: shift ? [{ ...shift, terminalId: 'preview-till', actualBalanceMinor: null, varianceMinor: null, status: 'OPEN', closedAt: null, notes: null }] : [], categories: [], departments, paymentMethods, expenses: [] };
    case 'get_dashboard_report': return { startAt: payload.range.startAt, endAt: payload.range.endAt, totals: { grossSalesMinor: 348050, refundsMinor: 1850, netSalesMinor: 346200, grossProfitMinor: 142050, expensesMinor: 32400, shiftVarianceMinor: 0, inventoryValueMinor: 965000, lowStockCount: 0 }, departments: departments.map(d => ({ departmentId: d.id, departmentName: d.name, grossSalesMinor: 116000, refundsMinor: 0, netSalesMinor: 116000, grossProfitMinor: 47000 })), payments: [{ methodName: 'Cash', methodKind: 'CASH', receivedMinor: 89050, refundedMinor: 0, netMinor: 89050 }, { methodName: 'Card', methodKind: 'CARD', receivedMinor: 259000, refundedMinor: 1850, netMinor: 257150 }], stock: [] };
    case 'get_safety_status': return { schemaVersion: 6, latestSchemaVersion: 6, integrity: 'ok', foreignKeyViolations: 0, inventoryReconciliationDifferences: 0, financialEventsWithoutAudit: 0, backupCount: 0, unverifiedBackupCount: 0, healthy: true, backups: [] };
    default: throw new Error('UI preview does not implement ' + command);
  }
});
createRoot(document.getElementById('root')!).render(<><App /><details style={{ position: 'fixed', bottom: 0, left: 0, zIndex: 20, fontSize: 9, color: '#b5c0d0', background: '#161b22', maxWidth: '100vw' }}><summary style={{ padding: '3px 7px', cursor: 'pointer' }}>UI preview · sample data only</summary><pre id="preview-audit" style={{ maxHeight: 250, overflow: 'auto', width: 500, maxWidth: '100vw' }} /></details></>);
