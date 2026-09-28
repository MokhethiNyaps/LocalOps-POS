import { FormEvent, useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Icon, departmentStyle, type IconName } from './Icons';

export const setupSteps = ['Business setup', 'Departments & locations', 'Products & services', 'Inventory', 'POS & payments'] as const;

type BootstrapTerminal = { id: string; name: string; departmentId: string | null };
type BootstrapBusiness = { id: string; name: string; terminals: BootstrapTerminal[] };
type AppBootstrap = { setupRequired: boolean; businesses: BootstrapBusiness[] };
type SessionView = { businessId: string; terminalId: string; departmentId: string | null; displayName: string };
type SetupResult = { businessId: string; departmentId: string; terminalId: string };
type Category = { id: string; name: string; parentId: string | null };
type Unit = { id: string; code: string; name: string; dimension: string; scaleNum: number; scaleDen: number; decimalPlaces: number };
type Packaging = { id: string; unitId: string; name: string; factorNum: number; factorDen: number; canPurchase: boolean; canSell: boolean };
type CatalogueItem = { id: string; kind: 'PRODUCT' | 'SERVICE'; name: string; categoryId: string | null; priceMinor: number; taxable: boolean; barcode: string | null; sku: string | null; productCode: string | null; baseUnitId: string | null; costMinor: number | null; trackStock: boolean | null; durationMinutes: number | null; hasRecipe: boolean; packaging: Packaging[] };
type Department = { id: string; name: string };
type CatalogueSnapshot = { businessId: string; terminalId: string; terminalDepartmentId: string | null; categories: Category[]; units: Unit[]; items: CatalogueItem[]; departments: Department[] };
type InventoryLocation = { id: string; name: string };
type InventoryPackaging = { id: string; unitId: string; name: string; factorNum: number; factorDen: number };
type InventoryProduct = { id: string; name: string; baseUnitId: string; baseUnitCode: string; packaging: InventoryPackaging[] };
type InventoryBalance = { productId: string; productName: string; locationId: string; locationName: string; quantityMicros: number; version: number };
type InventorySupplier = { id: string; name: string };
type InventoryMovement = { id: string; productName: string; locationName: string; quantityMicros: number; movementType: string; occurredAt: string; referenceType: string; referenceId: string };
type InventorySnapshot = { locations: InventoryLocation[]; products: InventoryProduct[]; balances: InventoryBalance[]; suppliers: InventorySupplier[]; recentMovements: InventoryMovement[]; reconciliationDifferenceCount: number };
export type PosItem = { id: string; departmentId: string; departmentName: string; name: string; kind: string; priceMinor: number; taxable: boolean; sku: string | null; productCode: string | null; barcode: string | null };
type PosPaymentMethod = { id: string; code: string; name: string; kind: string };
type PosShift = { id: string; openingBalanceMinor: number; expectedBalanceMinor: number; openedAt: string };
type RecentSale = { id: string; saleNumber: string; status: string; totalMinor: number; completedAt: string };
type PosSnapshot = { businessName: string; currency: string; taxEnabled: boolean; taxRatePpm: number; pricesIncludeTax: boolean; items: PosItem[]; paymentMethods: PosPaymentMethod[]; openShift: PosShift | null; recentSales: RecentSale[] };
type ReceiptLine = { id: string; description: string; kind: string; quantityMicros: number; unitPriceMinor: number; discountMinor: number; taxMinor: number; lineTotalMinor: number };
type ReceiptPayment = { id: string; methodName: string; methodKind: string; amountMinor: number; tenderedMinor: number | null; changeMinor: number; reference: string | null; status: string };
type SaleReceipt = { id: string; saleNumber: string; currency: string; subtotalMinor: number; discountMinor: number; taxMinor: number; totalMinor: number; amountPaidMinor: number; changeDueMinor: number; completedAt: string; lines: ReceiptLine[]; payments: ReceiptPayment[]; idempotentReplay: boolean };
type OpsShift = { id: string; terminalId: string; openingBalanceMinor: number; expectedBalanceMinor: number; actualBalanceMinor: number | null; varianceMinor: number | null; status: string; openedAt: string; closedAt: string | null; notes: string | null };
type OpsOption = { id: string; name: string; kind: string | null };
type OpsExpense = { id: string; categoryName: string; departmentName: string | null; paymentMethodName: string | null; amountMinor: number; currency: string; expenseDate: string; description: string; reference: string | null; receiptImagePath: string | null; status: string; shiftId: string };
type OperationsSnapshot = { openShift: OpsShift | null; shifts: OpsShift[]; categories: OpsOption[]; departments: OpsOption[]; paymentMethods: OpsOption[]; expenses: OpsExpense[] };
type ReportTotals = { grossSalesMinor: number; refundsMinor: number; netSalesMinor: number; grossProfitMinor: number; expensesMinor: number; shiftVarianceMinor: number; inventoryValueMinor: number; lowStockCount: number };
type DepartmentReport = { departmentId: string; departmentName: string; grossSalesMinor: number; refundsMinor: number; netSalesMinor: number; grossProfitMinor: number };
type PaymentReport = { methodName: string; methodKind: string; receivedMinor: number; refundedMinor: number; netMinor: number };
type StockReport = { productName: string; locationName: string; quantityMicros: number; minimumQuantityMicros: number; valueMinor: number };
type DashboardReport = { startAt: string; endAt: string; totals: ReportTotals; departments: DepartmentReport[]; payments: PaymentReport[]; stock: StockReport[] };
type BackupView = { filename: string; kind: string; sizeBytes: number; verified: boolean };
type SafetyStatus = { schemaVersion: number; latestSchemaVersion: number; integrity: string; foreignKeyViolations: number; inventoryReconciliationDifferences: number; financialEventsWithoutAudit: number; backupCount: number; unverifiedBackupCount: number; healthy: boolean; backups: BackupView[] };
type Act = (action: () => Promise<unknown>, success: string) => Promise<void>;

const errorText = (reason: unknown) => reason instanceof Error ? reason.message : String(reason);

export function App() {
  const [mode, setMode] = useState<'setup' | 'login' | 'workspace'>('setup');
  const [bootstrap, setBootstrap] = useState<AppBootstrap>({ setupRequired: true, businesses: [] });
  const [session, setSession] = useState<SessionView | null>(null);
  const [startupError, setStartupError] = useState<string | null>(null);
  useEffect(() => {
    void invoke<AppBootstrap>('get_app_bootstrap').then(result => {
      setBootstrap(result);
      if (!result.setupRequired) setMode('login');
    }).catch(reason => setStartupError(errorText(reason)));
  }, []);
  const enterWorkspace = (next: SessionView) => { setSession(next); setMode('workspace'); };
  if (mode === 'workspace' && session) return <Workspace session={session} onLogout={() => { setSession(null); setMode('login'); }} />;
  return <main className="min-h-screen bg-background flex flex-col items-center justify-center p-space-lg relative overflow-hidden">
    <div className="absolute inset-0 pointer-events-none bg-[radial-gradient(ellipse_at_top,_var(--tw-gradient-stops))] from-primary/10 via-background to-background"></div>
    <div className="z-10 w-full max-w-6xl mx-auto flex flex-col lg:flex-row items-center gap-space-xl">
      <BrandHeader />
      <div className="flex-1 w-full max-w-md">
        {startupError && <p className="p-4 mb-6 rounded-lg bg-error-container text-on-error-container font-body-md shadow-sm" role="alert">The local database could not be opened: {startupError}</p>}
        {mode === 'setup'
          ? <SetupView onComplete={(result, displayName) => enterWorkspace({ businessId: result.businessId, terminalId: result.terminalId, departmentId: result.departmentId, displayName })} />
          : <LoginView bootstrap={bootstrap} onLogin={enterWorkspace} />}
      </div>
    </div>
    <footer className="absolute bottom-6 w-full text-center flex flex-wrap justify-center gap-6 font-body-sm text-on-surface-variant/70">
      <span className="flex items-center gap-2"><span className="material-symbols-outlined text-sm">shield</span> Data stays on this device</span>
      <span className="flex items-center gap-2"><span className="material-symbols-outlined text-sm">lock</span> Secure local storage</span>
      <span className="flex items-center gap-2"><span className="material-symbols-outlined text-sm">backup</span> Automatic local backups</span>
    </footer>
  </main>;
}

function BrandHeader() {
  return <header className="flex-1 flex flex-col items-start gap-6 max-w-lg">
    <div className="w-16 h-16 rounded-2xl bg-primary-container text-on-primary-container flex items-center justify-center shadow-lg border border-primary/20" aria-hidden="true">
      <span className="material-symbols-outlined text-4xl">point_of_sale</span>
    </div>
    <div>
      <p className="font-badge-label text-primary uppercase tracking-widest mb-3 flex items-center gap-2">
        <span className="w-2 h-2 rounded-full bg-secondary"></span> LOCALOPS POS
      </p>
      <h1 className="font-display text-on-surface mb-4">Your business.<br/>One place.<br/>Always available.</h1>
      <p className="font-body-lg text-on-surface-variant">The lightning-fast, offline-first point of sale system designed for modern touchscreen terminals.</p>
    </div>
  </header>;
}

type SetupFields = { businessName: string; departmentName: string; locationName: string; terminalName: string; ownerUsername: string; ownerDisplayName: string; ownerPin: string };
const initialFields: SetupFields = { businessName: '', departmentName: '', locationName: '', terminalName: '', ownerUsername: '', ownerDisplayName: '', ownerPin: '' };

export function SetupView({ onComplete = () => undefined }: { onComplete?: (result: SetupResult, displayName: string) => void }) {
  const [fields, setFields] = useState(initialFields);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const updateField = (name: keyof SetupFields, value: string) => setFields(current => ({ ...current, [name]: value }));
  async function createBusiness(event: FormEvent<HTMLFormElement>) {
    event.preventDefault(); if (submitting) return; setSubmitting(true); setError(null);
    try { onComplete(await invoke<SetupResult>('complete_initial_setup', { input: fields }), fields.ownerDisplayName.trim()); }
    catch (reason) { setError(errorText(reason)); } finally { setSubmitting(false); }
  }
  return <section className="w-full bg-surface-container-lowest rounded-3xl border border-outline-variant/30 shadow-2xl overflow-hidden flex flex-col">
    <div className="p-8 border-b border-outline-variant/20 bg-surface/50">
      <p className="font-badge-label text-primary uppercase tracking-wider mb-2">FIRST-RUN SETUP</p>
      <h2 className="font-headline-lg text-on-surface mb-2">Build workspace</h2>
      <p className="font-body-md text-on-surface-variant">Create the business, its first operation, and the owner account in one secure offline step.</p>
    </div>
    <div className="p-8">
      <form onSubmit={createBusiness} className="flex flex-col gap-1">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-x-4">
          <Field label="Business name" name="businessName" value={fields.businessName} onChange={updateField} placeholder="Moko’s Lifestyle Centre" />
          <Field label="Terminal name" name="terminalName" value={fields.terminalName} onChange={updateField} placeholder="Main Till" />
          <Field label="First department" name="departmentName" value={fields.departmentName} onChange={updateField} placeholder="Bar" />
          <Field label="Main stock location" name="locationName" value={fields.locationName} onChange={updateField} placeholder="Main Store" />
          <Field label="Owner username" name="ownerUsername" value={fields.ownerUsername} onChange={updateField} placeholder="owner" />
          <Field label="Owner PIN" name="ownerPin" value={fields.ownerPin} onChange={updateField} placeholder="4–12 digits" type="password" pattern="[0-9]{4,12}" />
          <div className="md:col-span-2">
            <Field label="Owner display name" name="ownerDisplayName" value={fields.ownerDisplayName} onChange={updateField} placeholder="Moko" />
          </div>
        </div>
        <button type="submit" disabled={submitting} className="w-full py-4 mt-2 rounded-xl bg-primary text-on-primary font-headline-sm hover:bg-primary-fixed transition-colors disabled:opacity-50 flex items-center justify-center gap-2">
          {submitting ? <span className="material-symbols-outlined animate-spin">sync</span> : null}
          {submitting ? 'Creating securely…' : 'Create business'} 
          {!submitting && <span className="material-symbols-outlined text-lg" aria-hidden="true">arrow_forward</span>}
        </button>
      </form>
      {error && <p className="p-4 mt-6 rounded-lg bg-error-container text-on-error-container font-body-md shadow-sm" role="alert">Setup could not be completed: {error}</p>}
    </div>
  </section>;
}

function LoginView({ bootstrap, onLogin }: { bootstrap: AppBootstrap; onLogin: (session: SessionView) => void }) {
  const firstBusiness = bootstrap.businesses[0];
  const [businessId, setBusinessId] = useState(firstBusiness?.id ?? '');
  const business = bootstrap.businesses.find(value => value.id === businessId) ?? firstBusiness;
  const [terminalId, setTerminalId] = useState(business?.terminals[0]?.id ?? '');
  const [username, setUsername] = useState(''); const [pin, setPin] = useState(''); const [error, setError] = useState<string | null>(null); const [submitting, setSubmitting] = useState(false);
  useEffect(() => { if (!businessId && firstBusiness) setBusinessId(firstBusiness.id); }, [businessId, firstBusiness]);
  useEffect(() => { if (business && !business.terminals.some(value => value.id === terminalId)) setTerminalId(business.terminals[0]?.id ?? ''); }, [business, terminalId]);
  async function submit(event: FormEvent<HTMLFormElement>) { event.preventDefault(); setSubmitting(true); setError(null); try { onLogin(await invoke<SessionView>('login', { businessId, terminalId, username, pin })); } catch (reason) { setError(errorText(reason)); } finally { setSubmitting(false); } }
  return <section className="w-full bg-surface-container-lowest rounded-3xl border border-outline-variant/30 shadow-2xl overflow-hidden flex flex-col">
    <div className="p-8 border-b border-outline-variant/20 bg-surface/50">
      <p className="font-badge-label text-primary uppercase tracking-wider mb-2">WELCOME BACK</p>
      <h2 className="font-headline-lg text-on-surface">Sign in to this terminal</h2>
    </div>
    <div className="p-8">
      <form onSubmit={submit} className="flex flex-col">
        <label className="w-full block mb-4">
          <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Business</span>
          <select value={businessId} onChange={event => setBusinessId(event.target.value)} required className="w-full h-12 px-4 rounded-xl bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all">
            {bootstrap.businesses.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}
          </select>
        </label>
        <label className="w-full block mb-4">
          <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Terminal</span>
          <select value={terminalId} onChange={event => setTerminalId(event.target.value)} required className="w-full h-12 px-4 rounded-xl bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all">
            {business?.terminals.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}
          </select>
        </label>
        <label className="w-full block mb-4">
          <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Username</span>
          <input value={username} onChange={event => setUsername(event.target.value)} autoComplete="username" required className="w-full h-12 px-4 rounded-xl bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all" />
        </label>
        <label className="w-full block mb-6">
          <span className="block font-body-sm font-medium text-on-surface-variant mb-2">PIN</span>
          <input value={pin} onChange={event => setPin(event.target.value)} type="password" inputMode="numeric" pattern="[0-9]{4,12}" autoComplete="current-password" required className="w-full h-12 px-4 rounded-xl bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all tracking-[0.2em] font-bold" />
        </label>
        <button disabled={submitting || !terminalId} className="w-full py-4 rounded-xl bg-primary text-on-primary font-headline-sm hover:bg-primary-fixed transition-colors disabled:opacity-50 flex items-center justify-center gap-2">
          {submitting ? <span className="material-symbols-outlined animate-spin">sync</span> : null}
          {submitting ? 'Signing in…' : 'Sign in'} 
          {!submitting && <span className="material-symbols-outlined text-lg">arrow_forward</span>}
        </button>
      </form>
      {error && <p className="p-4 mt-6 rounded-lg bg-error-container text-on-error-container font-body-md shadow-sm" role="alert">{error}</p>}
    </div>
  </section>;
}

function Workspace({ session, onLogout }: { session: SessionView; onLogout: () => void }) {
  const [snapshot, setSnapshot] = useState<CatalogueSnapshot | null>(null); const [error, setError] = useState<string | null>(null); const [notice, setNotice] = useState<string | null>(null); const [view, setView] = useState<'pos' | 'items' | 'configuration' | 'inventory' | 'operations' | 'reports' | 'safety'>('pos');
  const refresh = useCallback(async () => { try { setSnapshot(await invoke<CatalogueSnapshot>('get_catalogue_snapshot')); setError(null); } catch (reason) { setError(errorText(reason)); } }, []);
  useEffect(() => { void refresh(); }, [refresh]);
  async function act(action: () => Promise<unknown>, success: string) { setError(null); setNotice(null); try { await action(); setNotice(success); await refresh(); } catch (reason) { setError(errorText(reason)); } }
  async function logout() { try { await invoke('logout'); } finally { onLogout(); } }
  const navigation: { id: typeof view; label: string; detail: string; icon: IconName }[] = [
    { id: 'pos', label: 'Order entry', detail: 'Products & checkout', icon: 'till' },
    { id: 'items', label: 'Catalogue', detail: 'Products & services', icon: 'grid' },
    { id: 'inventory', label: 'Inventory', detail: 'Stock & movements', icon: 'box' },
    { id: 'operations', label: 'Shift & expenses', detail: 'Cash control & closeout', icon: 'cash' },
    { id: 'reports', label: 'Reports', detail: 'Sales & performance', icon: 'chart' },
    { id: 'configuration', label: 'Setup tools', detail: 'Configure your catalogue', icon: 'settings' },
    { id: 'safety', label: 'Backup & safety', detail: 'Protect your business', icon: 'shield' },
  ];
  return <div className="min-h-screen bg-background">
    <a className="sr-only focus:not-sr-only" href="#workspace-content">Skip to workspace</a>
    <header className="fixed top-0 left-0 right-0 h-16 bg-surface-container-lowest z-50 flex items-center justify-between px-space-md shadow-[0_4px_12px_rgba(0,0,0,0.6)]">
      <div className="flex items-center gap-space-sm">
        <div className="w-10 h-10 rounded-lg bg-primary-container flex items-center justify-center shadow-[inset_0_2px_4px_rgba(255,255,255,0.2)]">
          <span className="material-symbols-outlined text-on-primary-container">point_of_sale</span>
        </div>
        <div className="flex flex-col">
          <span className="font-headline-sm text-on-surface tracking-tight">LocalOps</span>
          <span className="font-badge-label text-primary-fixed uppercase tracking-wider">Terminal 1</span>
        </div>
      </div>
      <div className="flex items-center gap-space-md">
        <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-surface-container border border-outline-variant/30">
          <div className="w-2 h-2 rounded-full bg-secondary"></div>
          <span className="font-body-sm text-on-surface-variant font-medium">System Online</span>
        </div>
        <div className="h-8 w-[1px] bg-outline-variant/50"></div>
        <TerminalClock />
        <div className="h-8 w-[1px] bg-outline-variant/50"></div>
        <div className="flex items-center gap-3 pl-2 cursor-pointer group">
          <div className="flex flex-col items-end group-hover:opacity-80 transition-opacity">
            <span className="font-body-md text-on-surface font-medium">{session.displayName}</span>
            <span className="font-body-sm text-primary">Manager</span>
          </div>
          <div className="w-10 h-10 rounded-full bg-tertiary-container flex items-center justify-center border-2 border-surface-container-lowest shadow-[0_0_0_2px_theme(colors.outline-variant)]">
            <span className="font-headline-sm text-on-tertiary-container">{session.displayName.slice(0, 2).toUpperCase()}</span>
          </div>
        </div>
      </div>
    </header>

    <aside className="fixed left-0 top-16 bottom-0 w-72 bg-surface-container-lowest z-40 flex flex-col p-space-sm justify-between shadow-[4px_0_16px_rgba(0,0,0,0.5)]">
      <div className="flex flex-col gap-2">
        <div className="px-3 py-2">
          <span className="font-badge-label text-on-surface-variant uppercase tracking-wider">Workspace</span>
        </div>
        {navigation.map(item => (
          <button key={item.id} title={item.label} onClick={() => setView(item.id)} className={view === item.id ? "w-full flex items-center gap-3 px-4 py-3 rounded-xl bg-secondary-container text-on-secondary-container transition-all shadow-[0_2px_8px_rgba(0,0,0,0.2)]" : "w-full flex items-center gap-3 px-4 py-3 rounded-xl hover:bg-surface-container text-on-surface-variant transition-colors group"}>
            <span className={`material-symbols-outlined ${view === item.id ? '' : 'group-hover:text-on-surface transition-colors'}`}>{
              item.icon === 'till' ? 'point_of_sale' : 
              item.icon === 'grid' ? 'grid_view' : 
              item.icon === 'box' ? 'inventory_2' : 
              item.icon === 'cash' ? 'account_balance_wallet' : 
              item.icon === 'chart' ? 'bar_chart' : 
              item.icon === 'settings' ? 'settings' : 'security'
            }</span>
            <div className="flex flex-col items-start text-left">
              <span className={`font-headline-sm ${view === item.id ? '' : 'group-hover:text-on-surface transition-colors'}`}>{item.label}</span>
              <span className="font-body-sm opacity-80">{item.detail}</span>
            </div>
          </button>
        ))}
      </div>
      <div className="flex flex-col gap-2 mt-auto">
        <div className="p-4 rounded-xl bg-surface-container/50 border border-outline-variant/20">
          <div className="flex items-start gap-3">
            <span className="material-symbols-outlined text-secondary">verified_user</span>
            <div className="flex flex-col">
              <span className="font-body-md text-on-surface font-medium">Local &amp; Secure</span>
              <span className="font-body-sm text-on-surface-variant">Your data stays here</span>
            </div>
          </div>
        </div>
        <button className="w-full flex items-center justify-center gap-2 px-4 py-3 rounded-xl bg-surface-container hover:bg-surface-container-highest text-error transition-colors font-body-md font-medium" onClick={logout}>
          <span className="material-symbols-outlined text-lg">logout</span>
          <span>Sign Out</span>
        </button>
      </div>
    </aside>

    <div className="pl-72">
      <main id="workspace-content" tabIndex={-1} className="w-full pt-16 min-h-screen bg-background p-space-md outline-none">
        {view === 'pos' ? <PosView /> : view === 'inventory' ? <InventoryView /> : view === 'operations' ? <OperationsView /> : view === 'reports' ? <ReportsView /> : view === 'safety' ? <SafetyView /> : <><header className="flex items-center justify-between mb-space-lg"><div><p className="font-badge-label text-primary uppercase tracking-wider mb-1">CATALOGUE</p><h2 className="font-headline-lg text-on-surface">{view === 'items' ? 'Products & services' : 'Catalogue setup'}</h2></div></header>{error && <p className="p-4 mb-6 rounded-lg bg-error-container text-on-error-container font-body-md shadow-sm" role="alert">{error}</p>}{notice && <p className="p-4 mb-6 rounded-lg bg-primary-container text-on-primary-container font-body-md shadow-sm" role="status">{notice}</p>}{!snapshot ? <div className="p-8 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm font-body-md text-on-surface-variant">Loading local catalogue…</div> : view === 'items' ? <ItemsView snapshot={snapshot} act={act} /> : <ConfigurationView snapshot={snapshot} act={act} />}</>}
      </main>
    </div>
  </div>;
}


function TerminalClock() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => { const timer = window.setInterval(() => setNow(new Date()), 1000); return () => window.clearInterval(timer); }, []);
  return <time dateTime={now.toISOString()} className="flex flex-col items-end">
    <span className="font-price-tag text-on-surface">{now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })}</span>
    <span className="font-body-sm text-on-surface-variant">{now.toLocaleDateString([], { weekday: 'short', day: '2-digit', month: 'short' })}</span>
  </time>;
}

function ItemsView({ snapshot, act }: { snapshot: CatalogueSnapshot; act: Act }) {
  const [kind, setKind] = useState<'PRODUCT' | 'SERVICE'>('PRODUCT'); const [name, setName] = useState(''); const [price, setPrice] = useState(''); const [cost, setCost] = useState(''); const [barcode, setBarcode] = useState(''); const [sku, setSku] = useState(''); const [productCode, setProductCode] = useState(''); const [categoryId, setCategoryId] = useState(''); const [unitId, setUnitId] = useState(snapshot.units[0]?.id ?? ''); const [duration, setDuration] = useState(''); const [taxable, setTaxable] = useState(true); const [trackStock, setTrackStock] = useState(true);
  useEffect(() => { if (!unitId && snapshot.units[0]) setUnitId(snapshot.units[0].id); }, [snapshot.units, unitId]);
  async function submit(event: FormEvent<HTMLFormElement>) { event.preventDefault(); await act(() => { const input = kind === 'PRODUCT' ? { name, priceMinor: parseScaled(price, 2), categoryId: categoryId || null, taxable, baseUnitId: unitId, costMinor: parseScaled(cost || '0', 2), trackStock, minimumQuantityMicros: 0, barcode: barcode || null, sku: sku || null, productCode: productCode || null } : { name, priceMinor: parseScaled(price, 2), categoryId: categoryId || null, taxable, durationMinutes: duration ? Number(duration) : null }; return invoke(kind === 'PRODUCT' ? 'create_catalogue_product' : 'create_catalogue_service', { input }); }, `${name} added to the catalogue.`); setName(''); setPrice(''); setCost(''); setBarcode(''); setSku(''); setProductCode(''); setDuration(''); }
  return <div className="flex flex-col gap-space-md">
    <div className="grid grid-cols-1 md:grid-cols-3 gap-space-sm mb-space-md">
      <div className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col items-center justify-center text-center">
        <span className="font-display text-primary">{snapshot.items.length}</span>
        <span className="font-body-md text-on-surface-variant">Active items</span>
      </div>
      <div className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col items-center justify-center text-center">
        <span className="font-display text-on-surface">{snapshot.items.filter(value => value.kind === 'PRODUCT').length}</span>
        <span className="font-body-md text-on-surface-variant">Products</span>
      </div>
      <div className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col items-center justify-center text-center">
        <span className="font-display text-on-surface">{snapshot.items.filter(value => value.kind === 'SERVICE').length}</span>
        <span className="font-body-md text-on-surface-variant">Services</span>
      </div>
    </div>
    <form className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col" onSubmit={submit}>
      <div className="flex items-center justify-between mb-6 pb-6 border-b border-outline-variant/20">
        <div>
          <h3 className="font-headline-sm text-on-surface">Add an item</h3>
          <p className="font-body-sm text-on-surface-variant">Start with the essentials. Packaging and recipes can be added afterward.</p>
        </div>
        <select value={kind} onChange={event => setKind(event.target.value as 'PRODUCT' | 'SERVICE')} className="h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none">
          <option value="PRODUCT">Product</option>
          <option value="SERVICE">Service</option>
        </select>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        <label className="w-full block">
          <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Name</span>
          <input value={name} onChange={event => setName(event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
        </label>
        <label className="w-full block">
          <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Price (ZAR)</span>
          <input value={price} onChange={event => setPrice(event.target.value)} inputMode="decimal" placeholder="25.00" required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
        </label>
        <label className="w-full block">
          <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Category</span>
          <select value={categoryId} onChange={event => setCategoryId(event.target.value)} className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none">
            <option value="">No category</option>
            {snapshot.categories.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}
          </select>
        </label>
        {kind === 'PRODUCT' ? <>
          <label className="w-full block">
            <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Base unit</span>
            <select value={unitId} onChange={event => setUnitId(event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none">
              <option value="">Choose unit</option>
              {snapshot.units.map(value => <option key={value.id} value={value.id}>{value.code} — {value.name}</option>)}
            </select>
          </label>
          <label className="w-full block">
            <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Cost (ZAR)</span>
            <input value={cost} onChange={event => setCost(event.target.value)} inputMode="decimal" placeholder="0.00" className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
          </label>
          <label className="w-full block">
            <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Barcode (optional)</span>
            <input value={barcode} onChange={event => setBarcode(event.target.value)} inputMode="numeric" className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
          </label>
          <label className="w-full block">
            <span className="block font-body-sm font-medium text-on-surface-variant mb-2">SKU (optional)</span>
            <input value={sku} onChange={event => setSku(event.target.value)} className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
          </label>
          <label className="w-full block">
            <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Product code (optional)</span>
            <input value={productCode} onChange={event => setProductCode(event.target.value)} className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
          </label>
          <div className="flex items-center"><Toggle label="Track stock" checked={trackStock} onChange={setTrackStock} /></div>
        </> : <label className="w-full block">
          <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Duration (minutes)</span>
          <input value={duration} onChange={event => setDuration(event.target.value)} type="number" min="1" className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
        </label>}
        <div className="flex items-center"><Toggle label="Taxable" checked={taxable} onChange={setTaxable} /></div>
      </div>
      <button disabled={kind === 'PRODUCT' && !unitId} className="w-full py-2 rounded-lg bg-primary text-on-primary font-medium hover:bg-primary-fixed transition-colors disabled:opacity-50">Add {kind === 'PRODUCT' ? 'product' : 'service'}</button>
    </form>
    
    <div className="bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm overflow-hidden">
      <div className="overflow-x-auto">
        <table className="w-full text-left border-collapse">
          <thead>
            <tr className="border-b border-outline-variant/20 bg-surface/50">
              <th className="px-4 py-3 font-body-sm font-medium text-on-surface-variant">Item</th>
              <th className="px-4 py-3 font-body-sm font-medium text-on-surface-variant">Type</th>
              <th className="px-4 py-3 font-body-sm font-medium text-on-surface-variant">Price</th>
              <th className="px-4 py-3 font-body-sm font-medium text-on-surface-variant">Local lookup</th>
              <th className="px-4 py-3 font-body-sm font-medium text-on-surface-variant">Stock / duration</th>
              <th className="px-4 py-3 font-body-sm font-medium text-on-surface-variant">Composition</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-outline-variant/10">
            {snapshot.items.map(item => <tr key={item.id} className="hover:bg-surface-container/30 transition-colors">
              <td className="px-4 py-3">
                <div className="flex flex-col">
                  <strong className="font-body-md text-on-surface font-medium">{item.name}</strong>
                  <span className="font-body-sm text-on-surface-variant">{snapshot.categories.find(value => value.id === item.categoryId)?.name ?? 'Uncategorised'}</span>
                </div>
              </td>
              <td className="px-4 py-3"><span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-secondary-container text-on-secondary-container">{item.kind.toLowerCase()}</span></td>
              <td className="px-4 py-3 font-price-tag text-on-surface">{formatMoney(item.priceMinor)}</td>
              <td className="px-4 py-3 font-body-sm text-on-surface-variant">{[item.barcode, item.sku, item.productCode].filter(Boolean).join(' · ') || '—'}</td>
              <td className="px-4 py-3 font-body-sm text-on-surface-variant">{item.kind === 'PRODUCT' ? (item.trackStock ? 'Tracked' : 'Not tracked') : (item.durationMinutes ? `${item.durationMinutes} min` : '—')}</td>
              <td className="px-4 py-3 font-body-sm text-on-surface-variant">{[item.packaging.length ? `${item.packaging.length} package${item.packaging.length === 1 ? '' : 's'}` : '', item.hasRecipe ? 'Recipe' : ''].filter(Boolean).join(' · ') || 'Standard'}</td>
            </tr>)}
            {snapshot.items.length === 0 && <tr><td colSpan={6} className="px-4 py-8 text-center font-body-md text-on-surface-variant italic">No products or services yet.</td></tr>}
          </tbody>
        </table>
      </div>
    </div>
  </div>;
}

function ConfigurationView({ snapshot, act }: { snapshot: CatalogueSnapshot; act: Act }) { return <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-space-md items-start"><CategoryForm snapshot={snapshot} act={act} /><UnitForm act={act} /><PackagingForm snapshot={snapshot} act={act} /><RecipeForm snapshot={snapshot} act={act} /><AvailabilityForm snapshot={snapshot} act={act} /></div>; }

function CategoryForm({ snapshot, act }: { snapshot: CatalogueSnapshot; act: Act }) { 
  const [name, setName] = useState(''); const [parentId, setParentId] = useState(''); 
  return <form className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col gap-4" onSubmit={async event => { event.preventDefault(); await act(() => invoke('create_catalogue_category', { name, parentId: parentId || null }), `Category ${name} created.`); setName(''); }}>
    <div>
      <h3 className="font-headline-sm text-on-surface mb-1">Categories</h3>
      <p className="font-body-sm text-on-surface-variant">Group products and services for browsing and reporting.</p>
    </div>
    <label className="w-full block">
      <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Name</span>
      <input value={name} onChange={event => setName(event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
    </label>
    <label className="w-full block">
      <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Parent category</span>
      <select value={parentId} onChange={event => setParentId(event.target.value)} className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none">
        <option value="">Top level</option>
        {snapshot.categories.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}
      </select>
    </label>
    <button className="w-full py-2 rounded-lg bg-primary text-on-primary font-medium hover:bg-primary-fixed transition-colors mt-2">Add category</button>
    <div className="flex flex-wrap gap-2 mt-2">
      {snapshot.categories.map(value => <span key={value.id} className="px-2 py-1 rounded-md bg-surface-container text-on-surface font-body-sm">{value.name}</span>)}
    </div>
  </form>; 
}

function UnitForm({ act }: { act: Act }) {
  const [code, setCode] = useState(''); const [name, setName] = useState(''); const [dimension, setDimension] = useState('COUNT'); const [scaleNum, setScaleNum] = useState('1'); const [scaleDen, setScaleDen] = useState('1'); const [decimalPlaces, setDecimalPlaces] = useState('0');
  return <form className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col gap-4" onSubmit={async event => { event.preventDefault(); await act(() => invoke('create_catalogue_unit', { input: { code, name, dimension, scaleNum: Number(scaleNum), scaleDen: Number(scaleDen), decimalPlaces: Number(decimalPlaces) } }), `Unit ${code} created.`); setCode(''); setName(''); }}>
    <div>
      <h3 className="font-headline-sm text-on-surface mb-1">Units</h3>
      <p className="font-body-sm text-on-surface-variant">Create exact reusable measurement units using a ratio to the canonical unit.</p>
    </div>
    <div className="grid grid-cols-2 gap-4">
      <label className="w-full block">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Code</span>
        <input value={code} onChange={event => setCode(event.target.value.toUpperCase())} placeholder="KG" required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
      </label>
      <label className="w-full block">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Name</span>
        <input value={name} onChange={event => setName(event.target.value)} placeholder="Kilogram" required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
      </label>
    </div>
    <label className="w-full block">
      <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Dimension</span>
      <select value={dimension} onChange={event => setDimension(event.target.value)} className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none">
        <option>COUNT</option><option>WEIGHT</option><option>VOLUME</option><option>LENGTH</option><option>TIME</option>
      </select>
    </label>
    <div className="grid grid-cols-3 gap-2">
      <label className="w-full block">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Scale Num</span>
        <input value={scaleNum} onChange={event => setScaleNum(event.target.value)} type="number" min="1" required className="w-full h-10 px-2 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
      </label>
      <label className="w-full block">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Scale Den</span>
        <input value={scaleDen} onChange={event => setScaleDen(event.target.value)} type="number" min="1" required className="w-full h-10 px-2 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
      </label>
      <label className="w-full block">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Decimals</span>
        <input value={decimalPlaces} onChange={event => setDecimalPlaces(event.target.value)} type="number" min="0" max="6" required className="w-full h-10 px-2 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
      </label>
    </div>
    <button className="w-full py-2 rounded-lg bg-primary text-on-primary font-medium hover:bg-primary-fixed transition-colors mt-2">Add unit</button>
  </form>;
}

function PackagingForm({ snapshot, act }: { snapshot: CatalogueSnapshot; act: Act }) {
  const products = snapshot.items.filter(value => value.kind === 'PRODUCT'); const [productId, setProductId] = useState(products[0]?.id ?? ''); const [unitId, setUnitId] = useState(snapshot.units[0]?.id ?? ''); const [name, setName] = useState(''); const [factorNum, setFactorNum] = useState(''); const [factorDen, setFactorDen] = useState('1');
  return <form className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col gap-4" onSubmit={async event => { event.preventDefault(); await act(() => invoke('create_catalogue_packaging', { input: { productId, unitId, name, factorNum: Number(factorNum), factorDen: Number(factorDen), canPurchase: true, canSell: true } }), `${name} packaging added.`); setName(''); setFactorNum(''); setFactorDen('1'); }}>
    <div>
      <h3 className="font-headline-sm text-on-surface mb-1">Product packaging</h3>
      <p className="font-body-sm text-on-surface-variant">Define cases, crates, packs, or any exact product-specific conversion.</p>
    </div>
    <label className="w-full block">
      <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Product</span>
      <select value={productId} onChange={event => setProductId(event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none">
        <option value="">Choose product</option>{products.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}
      </select>
    </label>
    <label className="w-full block">
      <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Package unit</span>
      <select value={unitId} onChange={event => setUnitId(event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none">
        <option value="">Choose unit</option>{snapshot.units.map(value => <option key={value.id} value={value.id}>{value.code}</option>)}
      </select>
    </label>
    <label className="w-full block">
      <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Name</span>
      <input value={name} onChange={event => setName(event.target.value)} placeholder="Crate of 12" required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
    </label>
    <div className="grid grid-cols-2 gap-4">
      <label className="w-full block">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Base-unit num</span>
        <input value={factorNum} onChange={event => setFactorNum(event.target.value)} type="number" min="1" required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
      </label>
      <label className="w-full block">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Base-unit den</span>
        <input value={factorDen} onChange={event => setFactorDen(event.target.value)} type="number" min="1" required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
      </label>
    </div>
    <button disabled={!products.length || !snapshot.units.length} className="w-full py-2 rounded-lg bg-primary text-on-primary font-medium hover:bg-primary-fixed transition-colors mt-2 disabled:opacity-50">Add packaging</button>
  </form>;
}

type RecipeRow = { ingredientId: string; quantity: string; unitId: string };
function RecipeForm({ snapshot, act }: { snapshot: CatalogueSnapshot; act: Act }) {
  const products = snapshot.items.filter(value => value.kind === 'PRODUCT');
  const blankRow = (): RecipeRow => ({ ingredientId: products[0]?.id ?? '', quantity: '', unitId: products[0]?.baseUnitId ?? '' });
  const [ownerId, setOwnerId] = useState(snapshot.items[0]?.id ?? ''); const [yieldQuantity, setYieldQuantity] = useState('1'); const [rows, setRows] = useState<RecipeRow[]>([blankRow()]);
  function updateRow(index: number, update: Partial<RecipeRow>) { setRows(current => current.map((row, rowIndex) => rowIndex === index ? { ...row, ...update } : row)); }
  function chooseIngredient(index: number, ingredientId: string) { updateRow(index, { ingredientId, unitId: products.find(value => value.id === ingredientId)?.baseUnitId ?? '' }); }
  return <form className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col gap-4 md:col-span-2 lg:col-span-3 xl:col-span-2" onSubmit={async event => { event.preventDefault(); await act(() => invoke('replace_catalogue_recipe', { input: { ownerSellableId: ownerId, yieldQuantityMicros: parseScaled(yieldQuantity, 6), items: rows.map(row => ({ ingredientProductId: row.ingredientId, quantityMicros: parseScaled(row.quantity, 6), unitId: row.unitId })) } }), 'Recipe saved.'); setRows([blankRow()]); }}>
    <div>
      <h3 className="font-headline-sm text-on-surface mb-1">Recipe or consumable</h3>
      <p className="font-body-sm text-on-surface-variant">Consume one or more ingredient products when a product or service is sold.</p>
    </div>
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
      <label className="w-full block">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Sold item</span>
        <select value={ownerId} onChange={event => setOwnerId(event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none">
          <option value="">Choose item</option>{snapshot.items.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}
        </select>
      </label>
      <label className="w-full block">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Recipe yield</span>
        <input value={yieldQuantity} onChange={event => setYieldQuantity(event.target.value)} inputMode="decimal" required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
      </label>
    </div>
    <div className="flex flex-col gap-2 p-4 bg-surface-container/30 rounded-lg border border-outline-variant/10">
      {rows.map((row, index) => <div className="flex flex-col md:flex-row gap-3 items-end" key={index}>
        <label className="w-full md:flex-[2] block">
          <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Ingredient {index + 1}</span>
          <select value={row.ingredientId} onChange={event => chooseIngredient(index, event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none">
            <option value="">Choose product</option>{products.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}
          </select>
        </label>
        <label className="w-full md:flex-1 block">
          <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Quantity</span>
          <input value={row.quantity} onChange={event => updateRow(index, { quantity: event.target.value })} inputMode="decimal" required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
        </label>
        <label className="w-full md:flex-[1.5] block">
          <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Unit</span>
          <select value={row.unitId} onChange={event => updateRow(index, { unitId: event.target.value })} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none">
            {snapshot.units.map(value => <option key={value.id} value={value.id}>{value.code}</option>)}
          </select>
        </label>
        {rows.length > 1 && <button className="h-10 w-10 flex-shrink-0 flex items-center justify-center rounded-lg bg-error-container text-on-error-container hover:bg-error transition-colors text-xl font-bold" type="button" aria-label={`Remove ingredient ${index + 1}`} onClick={() => setRows(current => current.filter((_, rowIndex) => rowIndex !== index))}>×</button>}
      </div>)}
      <button className="w-full py-2 rounded-lg bg-surface-container border border-outline-variant/30 text-on-surface font-medium hover:bg-surface-container-highest transition-colors mt-2 disabled:opacity-50" type="button" disabled={!products.length} onClick={() => setRows(current => [...current, blankRow()])}>+ Add ingredient</button>
    </div>
    <button disabled={!snapshot.items.length || !products.length} className="w-full py-2 rounded-lg bg-primary text-on-primary font-medium hover:bg-primary-fixed transition-colors mt-2 disabled:opacity-50">Save recipe</button>
  </form>;
}

function AvailabilityForm({ snapshot, act }: { snapshot: CatalogueSnapshot; act: Act }) { 
  const [departmentId, setDepartmentId] = useState(snapshot.terminalDepartmentId ?? snapshot.departments[0]?.id ?? ''); const [sellableId, setSellableId] = useState(snapshot.items[0]?.id ?? ''); const [override, setOverride] = useState(''); 
  return <form className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col gap-4" onSubmit={async event => { event.preventDefault(); await act(() => invoke('set_catalogue_availability', { input: { departmentId, sellableId, priceOverrideMinor: override ? parseScaled(override, 2) : null, active: true } }), 'Department availability updated.'); }}>
    <div>
      <h3 className="font-headline-sm text-on-surface mb-1">Department availability</h3>
      <p className="font-body-sm text-on-surface-variant">Choose where an item can be sold and optionally override its price.</p>
    </div>
    <label className="w-full block">
      <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Department</span>
      <select value={departmentId} onChange={event => setDepartmentId(event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none">
        {snapshot.departments.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}
      </select>
    </label>
    <label className="w-full block">
      <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Item</span>
      <select value={sellableId} onChange={event => setSellableId(event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none">
        <option value="">Choose item</option>{snapshot.items.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}
      </select>
    </label>
    <label className="w-full block">
      <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Price override (optional)</span>
      <input value={override} onChange={event => setOverride(event.target.value)} inputMode="decimal" className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
    </label>
    <button disabled={!snapshot.departments.length || !snapshot.items.length} className="w-full py-2 rounded-lg bg-primary text-on-primary font-medium hover:bg-primary-fixed transition-colors mt-2 disabled:opacity-50">Make available</button>
  </form>; 
}

type CartLine = { item: PosItem; quantity: number };
function PosView() {
  const [departmentFilter, setDepartmentFilter] = useState('');
  const [posTab, setPosTab] = useState<'sale' | 'history'>('sale');
  const [paymentExpanded, setPaymentExpanded] = useState(false);
  const [snapshot, setSnapshot] = useState<PosSnapshot | null>(null); const [cart, setCart] = useState<CartLine[]>([]); const [itemSearch, setItemSearch] = useState(''); const [paymentAmounts, setPaymentAmounts] = useState<Record<string, string>>({}); const [cashTender, setCashTender] = useState(''); const [openingCash, setOpeningCash] = useState('0.00'); const [receipt, setReceipt] = useState<SaleReceipt | null>(null); const [selectedReceipt, setSelectedReceipt] = useState<SaleReceipt | null>(null); const [refundQuantities, setRefundQuantities] = useState<Record<string, string>>({}); const [refundPayments, setRefundPayments] = useState<Record<string, string>>({}); const [restoreLines, setRestoreLines] = useState<Record<string, boolean>>({}); const [refundReason, setRefundReason] = useState(''); const [error, setError] = useState<string | null>(null); const [notice, setNotice] = useState<string | null>(null); const [submitting, setSubmitting] = useState(false);
  const refresh = useCallback(async () => { try { setSnapshot(await invoke<PosSnapshot>('get_pos_snapshot')); setError(null); } catch (reason) { setError(errorText(reason)); } }, []);
  useEffect(() => { void refresh(); }, [refresh]);
  const lineTotal = (line: CartLine) => { const gross = line.item.priceMinor * line.quantity; if (!snapshot?.taxEnabled || !line.item.taxable || snapshot.pricesIncludeTax) return gross; return gross + roundPositiveRatio(gross * snapshot.taxRatePpm, 1_000_000); };
  const total = cart.reduce((sum, line) => sum + lineTotal(line), 0);
  const paymentTotal = snapshot?.paymentMethods.reduce((sum, method) => sum + safeMoneyInput(paymentAmounts[method.id]), 0) ?? 0;
  function addItem(item: PosItem) { const key = `${item.departmentId}:${item.id}`; setCart(current => { const found = current.find(line => `${line.item.departmentId}:${line.item.id}` === key); return found ? current.map(line => `${line.item.departmentId}:${line.item.id}` === key ? { ...line, quantity: line.quantity + 1 } : line) : [...current, { item, quantity: 1 }]; }); }
  function submitItemSearch(event: FormEvent<HTMLFormElement>) { event.preventDefault(); const exact = findBarcodeItem(snapshot?.items ?? [], itemSearch); if (exact) { addItem(exact); setItemSearch(''); setError(null); } else { setError(`No local item matches “${itemSearch.trim()}”.`); } }
  function updateQuantity(index: number, quantity: number) { setCart(current => quantity <= 0 ? current.filter((_, itemIndex) => itemIndex !== index) : current.map((line, itemIndex) => itemIndex === index ? { ...line, quantity } : line)); }
  function payFullCash() { const cash = snapshot?.paymentMethods.find(method => method.kind === 'CASH'); if (!cash) return; setPaymentAmounts({ [cash.id]: formatInputMoney(total) }); setCashTender(formatInputMoney(total)); }
  async function openShift(event: FormEvent<HTMLFormElement>) { event.preventDefault(); setError(null); try { await invoke('open_pos_shift', { input: { openingBalanceMinor: parseScaled(openingCash, 2), openedAt: new Date().toISOString() } }); setNotice('Shift opened. Sales can now be completed.'); await refresh(); } catch (reason) { setError(errorText(reason)); } }
  async function completeSale() { if (!snapshot?.openShift || !cart.length || submitting) return; setSubmitting(true); setError(null); setNotice(null); try { const completed = await invoke<SaleReceipt>('complete_pos_sale', { input: { idempotencyKey: crypto.randomUUID(), saleNumber: null, completedAt: new Date().toISOString(), notes: null, lines: cart.map(line => ({ sellableId: line.item.id, departmentId: line.item.departmentId, quantityMicros: line.quantity * 1_000_000, discountMinor: 0 })), payments: snapshot.paymentMethods.map(method => ({ methodId: method.id, amountMinor: safeMoneyInput(paymentAmounts[method.id]), tenderedMinor: method.kind === 'CASH' ? parseScaled(cashTender || paymentAmounts[method.id] || '0', 2) : safeMoneyInput(paymentAmounts[method.id]), reference: null })).filter(payment => payment.amountMinor > 0) } }); setReceipt(completed); setCart([]); setPaymentAmounts({}); setCashTender(''); setNotice(`Sale ${completed.saleNumber} completed. Receipt ready.`); await refresh(); } catch (reason) { setError(errorText(reason)); } finally { setSubmitting(false); } }
  async function loadReceipt(saleId: string) { try { const loaded = await invoke<SaleReceipt>('get_sale_receipt', { saleId }); setSelectedReceipt(loaded); setRefundQuantities(Object.fromEntries(loaded.lines.map(line => [line.id, formatQuantityInput(line.quantityMicros)]))); setRefundPayments(Object.fromEntries(loaded.payments.map(payment => [payment.id, formatInputMoney(payment.amountMinor)]))); setRestoreLines(Object.fromEntries(loaded.lines.map(line => [line.id, line.kind === 'PRODUCT']))); setRefundReason(''); setError(null); } catch (reason) { setError(errorText(reason)); } }
  async function reverseSale(voidSale: boolean) { if (!selectedReceipt || submitting) return; setSubmitting(true); setError(null); try { await invoke('create_pos_refund', { input: { saleId: selectedReceipt.id, idempotencyKey: crypto.randomUUID(), refundNumber: null, reason: refundReason, occurredAt: new Date().toISOString(), voidSale, items: selectedReceipt.lines.map(line => ({ saleItemId: line.id, quantityMicros: safeQuantityInput(refundQuantities[line.id]), restoreStock: Boolean(restoreLines[line.id]) })).filter(item => item.quantityMicros > 0), payments: selectedReceipt.payments.map(payment => ({ paymentId: payment.id, amountMinor: safeMoneyInput(refundPayments[payment.id]), reference: null })).filter(payment => payment.amountMinor > 0) } }); setNotice(voidSale ? 'Sale voided with traceable reversals.' : 'Refund recorded with explicit payment and stock allocations.'); await refresh(); await loadReceipt(selectedReceipt.id); } catch (reason) { setError(errorText(reason)); } finally { setSubmitting(false); } }
  if (!snapshot) return <><div className="mb-6"><div><p className="font-badge-label text-primary uppercase tracking-wider">POINT OF SALE</p><h2 className="font-headline-lg text-on-surface mt-1">Loading till…</h2></div></div>{error && <p className="p-4 rounded-lg bg-error-container text-on-error-container font-body-md" role="alert">{error}</p>}</>;
  const departments = Array.from(new Map(snapshot.items.map(item => [item.departmentId, item.departmentName])).entries());
  const visibleItems = snapshot.items.filter(item => (!departmentFilter || item.departmentId === departmentFilter) && matchesItemSearch(item, itemSearch));
  const itemCount = cart.reduce((count, line) => count + line.quantity, 0);
  return <>
    <div className="flex items-center justify-between mb-space-md">
      <div className="flex gap-2 p-1 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm">
        <button className={`px-6 py-2 rounded-lg font-body-md font-medium transition-colors flex items-center gap-2 ${posTab === 'sale' ? 'bg-primary text-on-primary shadow-sm' : 'hover:bg-surface-container text-on-surface-variant hover:text-on-surface'}`} onClick={() => setPosTab('sale')}><span className="material-symbols-outlined text-sm">point_of_sale</span> New Order</button>
        <button className={`px-6 py-2 rounded-lg font-body-md font-medium transition-colors flex items-center gap-2 ${posTab === 'history' ? 'bg-primary text-on-primary shadow-sm' : 'hover:bg-surface-container text-on-surface-variant hover:text-on-surface'}`} onClick={() => setPosTab('history')}><span className="material-symbols-outlined text-sm">receipt_long</span> History {snapshot.recentSales.length > 0 && <span className="bg-surface-container text-on-surface px-2 py-0.5 rounded-full text-xs">{snapshot.recentSales.length}</span>}</button>
      </div>
      {!snapshot.openShift && <span className="px-3 py-1 bg-error-container text-on-error-container rounded-full font-body-sm font-medium flex items-center gap-1"><span className="material-symbols-outlined text-sm">warning</span> Shift closed</span>}
    </div>
    {error && <p className="p-4 mb-4 rounded-lg bg-error-container text-on-error-container font-body-md shadow-sm" role="alert">{error}</p>}
    {notice && <p className="p-4 mb-4 rounded-lg bg-secondary-container text-on-secondary-container font-body-md shadow-sm" role="status">{notice}</p>}
    <div hidden={posTab !== 'sale'}>
      {!snapshot.openShift ? (
        <form className="max-w-md mx-auto mt-12 p-8 bg-surface-container-lowest rounded-2xl border border-outline-variant/20 shadow-lg flex flex-col items-center text-center" onSubmit={openShift}>
          <div className="w-20 h-20 rounded-full bg-primary-container text-on-primary-container flex items-center justify-center mb-6">
            <span className="material-symbols-outlined text-4xl">point_of_sale</span>
          </div>
          <p className="font-badge-label text-on-surface-variant uppercase tracking-wider mb-2">Ready for business</p>
          <h3 className="font-headline-md text-on-surface mb-2">Let's open your till</h3>
          <p className="font-body-md text-on-surface-variant mb-8">Count your opening cash to start the shift.<br/>Every sale will be recorded against this till.</p>
          <label className="w-full text-left mb-6">
            <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Opening cash (ZAR)</span>
            <input value={openingCash} onChange={event => setOpeningCash(event.target.value)} inputMode="decimal" required className="w-full h-12 px-4 rounded-xl bg-surface border border-outline-variant/30 text-on-surface text-lg focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all" />
          </label>
          <button className="w-full py-4 rounded-xl bg-primary text-on-primary font-headline-sm hover:bg-primary-fixed transition-colors flex items-center justify-center gap-2">
            <span className="material-symbols-outlined">lock_open</span> Open Shift
          </button>
        </form>
      ) : (
        <div className="flex gap-space-md h-[calc(100vh-10rem)]">
          <section className="flex-1 flex flex-col gap-space-md bg-surface rounded-xl border border-outline-variant/20 shadow-[0_4px_16px_rgba(0,0,0,0.4)] overflow-hidden" aria-label="Product selection">
            <div className="p-space-md bg-surface-container-lowest border-b border-outline-variant/20">
              <form className="relative mb-space-md flex gap-2" onSubmit={submitItemSearch}>
                <div className="relative flex-1">
                  <span className="material-symbols-outlined absolute left-4 top-1/2 -translate-y-1/2 text-on-surface-variant">search</span>
                  <input type="text" placeholder="Search products or scan barcode..." className="w-full h-12 pl-12 pr-10 rounded-xl bg-surface border border-outline-variant/30 text-on-surface placeholder-on-surface-variant focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all font-body-lg" value={itemSearch} onChange={event => setItemSearch(event.target.value)} autoFocus />
                  {itemSearch && <button type="button" className="absolute right-4 top-1/2 -translate-y-1/2 text-on-surface-variant hover:text-on-surface" onClick={() => setItemSearch('')}><span className="material-symbols-outlined">close</span></button>}
                </div>
                <button className="px-4 h-12 bg-surface-container hover:bg-surface-container-highest text-on-surface rounded-xl border border-outline-variant/30 font-body-md transition-colors flex items-center justify-center" title="Add exact barcode" aria-label="Add scanned item"><span className="material-symbols-outlined">barcode_scanner</span></button>
              </form>
              <div className="flex gap-space-sm overflow-x-auto pb-2 scrollbar-hide">
                <button className={`px-4 py-2 rounded-lg font-body-md whitespace-nowrap transition-colors ${!departmentFilter ? 'bg-secondary-container text-on-secondary-container font-medium' : 'bg-surface hover:bg-surface-container-highest text-on-surface-variant border border-outline-variant/20'}`} onClick={() => setDepartmentFilter('')}>All Items</button>
                {departments.map(([id, name]) => (
                  <button key={id} className={`px-4 py-2 rounded-lg font-body-md whitespace-nowrap transition-colors ${departmentFilter === id ? 'bg-secondary-container text-on-secondary-container font-medium' : 'bg-surface hover:bg-surface-container-highest text-on-surface-variant border border-outline-variant/20'}`} onClick={() => setDepartmentFilter(id)}>{name}</button>
                ))}
              </div>
            </div>
            <div className="flex-1 p-space-md overflow-y-auto">
              <div className="grid grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4 gap-space-md">
                {visibleItems.map(item => { 
                  const quantity = cart.find(line => line.item.id === item.id && line.item.departmentId === item.departmentId)?.quantity ?? 0;
                  return <button key={`${item.departmentId}:${item.id}`} onClick={() => addItem(item)} className={`relative flex flex-col p-4 rounded-xl border-2 transition-all text-left overflow-hidden bg-surface-container-lowest ${quantity > 0 ? 'border-secondary/70 shadow-[0_4px_12px_rgba(65,223,163,0.15)]' : 'border-outline-variant/20 hover:border-primary-fixed/50 hover:shadow-[0_4px_12px_rgba(255,181,154,0.1)]'} group`}>
                    <div className="flex justify-between items-start mb-4 w-full">
                      <span className={`material-symbols-outlined ${quantity > 0 ? 'text-secondary' : 'text-on-surface-variant group-hover:text-primary'} transition-colors`}>{item.kind === 'SERVICE' ? 'room_service' : 'inventory_2'}</span>
                      {quantity > 0 && <span className="w-6 h-6 rounded-full bg-secondary text-on-secondary flex items-center justify-center font-body-sm font-bold shadow-sm">{quantity}</span>}
                    </div>
                    <span className="font-headline-sm text-on-surface mb-1 leading-tight">{item.name}</span>
                    <span className="font-body-sm text-on-surface-variant mb-3">{item.departmentName}</span>
                    <span className="font-price-tag text-secondary mt-auto">{formatMoney(item.priceMinor)}</span>
                  </button>; 
                })}
              </div>
              {!visibleItems.length && <div className="flex flex-col items-center justify-center h-full text-on-surface-variant py-12"><span className="material-symbols-outlined text-5xl mb-4 opacity-50">search_off</span><h3 className="font-headline-sm mb-2">{snapshot.items.length ? 'No matching items' : 'Your catalogue starts here'}</h3><p className="font-body-sm text-center max-w-sm mb-6">{snapshot.items.length ? 'Try a different name, barcode or department.' : 'Add products in Catalogue, then make them available in Setup tools.'}</p>{snapshot.items.length > 0 && <button className="px-6 py-2 rounded-lg bg-surface-container hover:bg-surface-container-highest text-on-surface font-medium transition-colors" onClick={() => { setItemSearch(''); setDepartmentFilter(''); }}>Show all items</button>}</div>}
            </div>
          </section>
          
          <aside className="w-[360px] lg:w-[400px] flex flex-col bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-[0_4px_16px_rgba(0,0,0,0.4)] overflow-hidden relative" aria-label="Current sale">
            <div className="p-space-md bg-surface-container border-b border-outline-variant/20">
              <div className="flex justify-between items-center">
                <span className="font-headline-sm text-on-surface">Current Order</span>
                <span className="font-badge-label text-primary uppercase tracking-wider">{itemCount} items</span>
              </div>
            </div>
            
            <div className="flex-1 overflow-y-auto p-space-md">
              {cart.map((line, index) => (
                <div key={`${line.item.departmentId}:${line.item.id}`} className="mb-4 pb-4 border-b border-outline-variant/10 last:border-0 last:pb-0 last:mb-0">
                  <div className="flex justify-between items-start mb-2">
                    <div className="flex flex-col pr-2">
                      <span className="font-body-lg text-on-surface leading-tight mb-1">{line.item.name}</span>
                      <span className="font-body-sm text-on-surface-variant">{line.item.departmentName} · {formatMoney(line.item.priceMinor)} each</span>
                    </div>
                    <span className="font-body-lg font-semibold text-on-surface whitespace-nowrap">{formatMoney(lineTotal(line))}</span>
                  </div>
                  <div className="flex justify-between items-center mt-3">
                    <div className="flex items-center gap-1 bg-surface-container rounded-lg p-1">
                      <button type="button" className="w-8 h-8 rounded bg-surface-container-lowest text-on-surface flex items-center justify-center hover:bg-error/20 hover:text-error transition-colors" onClick={() => updateQuantity(index, line.quantity - 1)}><span className="material-symbols-outlined text-sm">remove</span></button>
                      <span className="font-body-md font-medium w-6 text-center">{line.quantity}</span>
                      <button type="button" className="w-8 h-8 rounded bg-surface-container-lowest text-on-surface flex items-center justify-center hover:bg-primary-fixed hover:text-on-primary-fixed transition-colors" onClick={() => updateQuantity(index, line.quantity + 1)}><span className="material-symbols-outlined text-sm">add</span></button>
                    </div>
                    <button className="text-on-surface-variant hover:text-error transition-colors w-8 h-8 flex items-center justify-center rounded hover:bg-error-container/50" onClick={() => updateQuantity(index, 0)} aria-label={'Remove ' + line.item.name}><span className="material-symbols-outlined text-[20px]">delete</span></button>
                  </div>
                </div>
              ))}
              {!cart.length && <div className="flex flex-col items-center justify-center h-full text-on-surface-variant opacity-70 mt-12"><span className="material-symbols-outlined text-5xl mb-4">shopping_cart</span><span className="font-headline-sm mb-2">Order is empty</span><span className="font-body-sm text-center">Tap items on the left to add them to the order.</span></div>}
            </div>
            
            <div className="p-space-md bg-surface border-t border-outline-variant/20">
              <div className="flex justify-between items-center mb-2">
                <span className="font-body-md text-on-surface-variant">Subtotal</span>
                <span className="font-body-md text-on-surface">{formatMoney(total)}</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="font-body-md text-on-surface-variant">Tax</span>
                <span className="font-body-md text-on-surface">{snapshot.taxEnabled ? (snapshot.pricesIncludeTax ? 'Incl.' : 'Added') : 'None'}</span>
              </div>
              <div className="mt-4 pt-4 border-t border-outline-variant/20 flex justify-between items-end">
                <span className="font-headline-sm text-on-surface">Total Due</span>
                <span className="font-ticket-total text-secondary">{formatMoney(total)}</span>
              </div>
            </div>

            <div className="p-space-md bg-surface-container-lowest flex flex-col gap-2">
              {paymentExpanded ? (
                <div className="flex flex-col gap-2 mb-2 p-3 bg-surface-container rounded-xl border border-outline-variant/20 shadow-inner">
                  <div className="flex justify-between items-center mb-1">
                    <span className="font-badge-label text-on-surface-variant uppercase tracking-wider">Payments</span>
                    <button type="button" className="text-primary text-sm hover:underline font-body-sm" onClick={() => setPaymentExpanded(false)}>Hide details</button>
                  </div>
                  {snapshot.paymentMethods.map(method => (
                    <label key={method.id} className="flex items-center justify-between">
                      <span className="font-body-md flex items-center gap-2"><span className="material-symbols-outlined text-sm">{method.kind === 'CASH' ? 'payments' : 'credit_card'}</span>{method.name}</span>
                      <input value={paymentAmounts[method.id] ?? ''} onChange={event => setPaymentAmounts(current => ({ ...current, [method.id]: event.target.value }))} inputMode="decimal" placeholder="0.00" className="w-24 px-2 py-1 bg-surface-container-lowest border border-outline-variant/30 rounded text-right font-body-md focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
                    </label>
                  ))}
                  {snapshot.paymentMethods.some(method => method.kind === 'CASH' && safeMoneyInput(paymentAmounts[method.id]) > 0) && (
                    <label className="flex items-center justify-between mt-2 pt-2 border-t border-outline-variant/20">
                      <span className="font-body-md text-on-surface-variant">Cash Tendered</span>
                      <input value={cashTender} onChange={event => setCashTender(event.target.value)} inputMode="decimal" placeholder="0.00" className="w-24 px-2 py-1 bg-surface-container-lowest border border-outline-variant/30 rounded text-right font-body-md focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
                    </label>
                  )}
                  <div className="flex justify-between items-center mt-3 pt-3 border-t border-outline-variant/30 font-body-sm">
                    <span>Allocated: <b>{formatMoney(paymentTotal)}</b></span>
                    <span className={paymentTotal === total ? 'text-secondary font-medium' : 'text-error font-medium'}>{paymentTotal === total ? 'Balanced' : formatMoney(total - paymentTotal) + ' remaining'}</span>
                  </div>
                </div>
              ) : (
                <button type="button" className="text-center font-body-sm text-primary py-2 hover:bg-surface-container rounded-lg transition-colors border border-dashed border-primary/30" onClick={() => setPaymentExpanded(true)}>
                  Split Payment / Exact Tender
                </button>
              )}
              
              <button type="button" className="w-full py-4 rounded-xl bg-primary text-on-primary font-headline-sm hover:bg-primary-fixed transition-colors disabled:opacity-50 disabled:hover:bg-primary shadow-sm flex items-center justify-center gap-2" onClick={() => { if (!paymentExpanded && paymentTotal !== total) setPaymentExpanded(true); else void completeSale(); }} disabled={!cart.length || (paymentExpanded && paymentTotal !== total) || submitting}>
                {submitting ? <span className="material-symbols-outlined animate-spin">sync</span> : null}
                {submitting ? 'Completing...' : !paymentExpanded && paymentTotal !== total ? 'Choose payment' : `Pay ${formatMoney(total)}`}
              </button>
              
              {snapshot.paymentMethods.some(method => method.kind === 'CASH') && (
                <button type="button" className="w-full py-3 rounded-xl bg-surface border border-outline-variant/30 text-on-surface hover:bg-surface-container font-body-lg font-medium transition-colors disabled:opacity-50 flex items-center justify-center gap-2" onClick={() => { payFullCash(); setPaymentExpanded(true); }} disabled={!cart.length}>
                  <span className="material-symbols-outlined text-lg">payments</span> Exact Cash
                </button>
              )}
            </div>
          </aside>
        </div>
      )}
    </div>
    
    {receipt && <ReceiptPanel receipt={receipt} title="Latest receipt" businessName={snapshot.businessName} printable />}
    
    <div hidden={posTab !== 'history'}>
      <div className="bg-surface rounded-xl border border-outline-variant/20 shadow-sm overflow-hidden">
        <div className="p-6 border-b border-outline-variant/20 bg-surface-container-lowest">
          <h3 className="font-headline-sm text-on-surface">Recent sales & reversals</h3>
          <p className="font-body-sm text-on-surface-variant mt-1">Open a receipt to allocate a partial refund or full void.</p>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-left font-body-sm">
            <thead className="bg-surface-container border-b border-outline-variant/20">
              <tr>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Receipt</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Time</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Status</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Total</th>
                <th className="px-6 py-3"></th>
              </tr>
            </thead>
            <tbody className="divide-y divide-outline-variant/10">
              {snapshot.recentSales.map(sale => (
                <tr key={sale.id} className="hover:bg-surface-container-lowest transition-colors">
                  <td className="px-6 py-4 font-body-md font-medium text-on-surface">{sale.saleNumber}</td>
                  <td className="px-6 py-4 text-on-surface-variant">{new Date(sale.completedAt).toLocaleString()}</td>
                  <td className="px-6 py-4">
                    <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-surface-container-highest text-on-surface-variant capitalize">
                      {sale.status.toLowerCase().replace('_', ' ')}
                    </span>
                  </td>
                  <td className="px-6 py-4 font-medium text-right text-on-surface">{formatMoney(sale.totalMinor)}</td>
                  <td className="px-6 py-4 text-right">
                    <button className="text-primary hover:text-primary-fixed font-medium transition-colors" onClick={() => void loadReceipt(sale.id)}>Open</button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {!snapshot.recentSales.length && <div className="p-8 text-center text-on-surface-variant font-body-md"><span className="material-symbols-outlined text-4xl mb-2 opacity-50 block">receipt_long</span>No completed sales yet.</div>}
        </div>
      </div>
      
      {selectedReceipt && (
        <div className="mt-space-md bg-surface rounded-xl border border-outline-variant/20 shadow-sm p-6">
          <ReceiptPanel receipt={selectedReceipt} title="Reversal allocation" businessName={snapshot.businessName} />
          
          <div className="mt-6 pt-6 border-t border-outline-variant/20">
            <h4 className="font-headline-sm text-on-surface mb-4">Reversal Details</h4>
            <label className="block mb-6">
              <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Reason for refund/void</span>
              <input type="text" value={refundReason} onChange={event => setRefundReason(event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface-container-lowest border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
            </label>
            
            <div className="grid md:grid-cols-2 gap-8 mb-6">
              <div>
                <h4 className="font-body-md font-medium text-on-surface mb-3 border-b border-outline-variant/10 pb-2">Items</h4>
                {selectedReceipt.lines.map(line => (
                  <div className="flex items-center justify-between gap-4 mb-3" key={line.id}>
                    <span className="font-body-sm text-on-surface-variant flex-1 truncate" title={line.description}>{line.description}</span>
                    <label className="flex items-center gap-2">
                      <span className="sr-only">Quantity</span>
                      <input value={refundQuantities[line.id] ?? ''} onChange={event => setRefundQuantities(current => ({ ...current, [line.id]: event.target.value }))} inputMode="decimal" className="w-16 h-8 px-2 text-right rounded bg-surface-container-lowest border border-outline-variant/30 text-body-sm" />
                    </label>
                    <Toggle label="Restock" checked={Boolean(restoreLines[line.id])} onChange={value => setRestoreLines(current => ({ ...current, [line.id]: value }))} />
                  </div>
                ))}
              </div>
              
              <div>
                <h4 className="font-body-md font-medium text-on-surface mb-3 border-b border-outline-variant/10 pb-2">Payment reversals</h4>
                {selectedReceipt.payments.map(payment => (
                  <div className="flex items-center justify-between gap-4 mb-3" key={payment.id}>
                    <span className="font-body-sm text-on-surface-variant">{payment.methodName}</span>
                    <label className="flex items-center gap-2">
                      <span className="sr-only">Amount</span>
                      <input value={refundPayments[payment.id] ?? ''} onChange={event => setRefundPayments(current => ({ ...current, [payment.id]: event.target.value }))} inputMode="decimal" className="w-24 h-8 px-2 text-right rounded bg-surface-container-lowest border border-outline-variant/30 text-body-sm" />
                    </label>
                  </div>
                ))}
              </div>
            </div>
            
            <div className="flex gap-4 pt-4 border-t border-outline-variant/20">
              <button className="px-6 py-2 rounded-lg bg-surface-container hover:bg-surface-container-highest text-on-surface font-medium transition-colors" onClick={() => void reverseSale(false)} disabled={!refundReason.trim() || submitting}>Record refund</button>
              <button className="px-6 py-2 rounded-lg bg-error text-on-error font-medium hover:opacity-90 transition-opacity" onClick={() => void reverseSale(true)} disabled={!refundReason.trim() || submitting}>Void full sale</button>
            </div>
          </div>
        </div>
      )}
    </div>
  </>;
}

function ReceiptPanel({ receipt, title, businessName, printable = false }: { receipt: SaleReceipt; title: string; businessName: string; printable?: boolean }) { 
  return <section className={`p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col gap-4${printable ? ' print-receipt' : ''}`}>
    <div className="text-center pb-4 border-b border-outline-variant/20"><strong className="block font-headline-md text-on-surface mb-1">{businessName}</strong><span className="font-badge-label text-on-surface-variant uppercase tracking-wider">Receipt</span></div>
    <div className="flex justify-between items-end pb-4 border-b border-outline-variant/10 border-dashed">
      <div>
        <p className="font-badge-label text-primary uppercase tracking-wider mb-1">{title}</p>
        <h3 className="font-headline-sm text-on-surface">{receipt.saleNumber}</h3>
      </div>
      <div className="flex flex-col items-end text-right">
        <span className="font-body-sm text-on-surface-variant mb-1">{new Date(receipt.completedAt).toLocaleString()}</span>
        {printable && <button className="text-primary hover:text-primary-fixed font-medium text-sm transition-colors no-print" onClick={() => window.print()}>Print receipt</button>}
      </div>
    </div>
    <div className="flex flex-col gap-3 py-2">
      {receipt.lines.map(line => <div className="flex justify-between items-start gap-4" key={line.id}>
        <span className="font-body-md text-on-surface-variant flex-1">{formatQuantity(line.quantityMicros)} × {line.description}</span>
        <b className="font-body-md text-on-surface whitespace-nowrap">{formatMoney(line.lineTotalMinor)}</b>
      </div>)}
    </div>
    <div className="flex flex-col gap-2 pt-4 border-t border-outline-variant/10 border-dashed">
      <div className="flex justify-between items-center font-body-md text-on-surface-variant"><span>Subtotal</span> <b>{formatMoney(receipt.subtotalMinor)}</b></div>
      <div className="flex justify-between items-center font-body-md text-on-surface-variant"><span>Tax</span> <b>{formatMoney(receipt.taxMinor)}</b></div>
      <div className="flex justify-between items-end mt-2 pt-2 border-t border-outline-variant/20"><span className="font-headline-sm text-on-surface">Total</span> <strong className="font-ticket-total text-on-surface">{formatMoney(receipt.totalMinor)}</strong></div>
      {receipt.changeDueMinor > 0 && <div className="flex justify-between items-center font-body-md text-on-surface-variant mt-2 pt-2 border-t border-outline-variant/20 border-dashed"><span>Change</span> <b>{formatMoney(receipt.changeDueMinor)}</b></div>}
    </div>
    <div className="flex flex-wrap gap-2 mt-4 pt-4 border-t border-outline-variant/20">
      {receipt.payments.map(payment => <span key={payment.id} className={`px-3 py-1 rounded-full text-xs font-medium ${payment.status === 'REVERSED' ? 'bg-error-container text-on-error-container line-through' : 'bg-secondary-container text-on-secondary-container'}`}>{payment.methodName}: {formatMoney(payment.amountMinor)}{payment.status === 'REVERSED' ? ' reversed' : ''}</span>)}
    </div>
  </section>; 
}

function OperationsView() {
  const [snapshot, setSnapshot] = useState<OperationsSnapshot | null>(null); const [error, setError] = useState<string | null>(null); const [notice, setNotice] = useState<string | null>(null); const [openingCash, setOpeningCash] = useState('0.00'); const [actualCash, setActualCash] = useState(''); const [closeNotes, setCloseNotes] = useState(''); const [categoryName, setCategoryName] = useState(''); const [categoryId, setCategoryId] = useState(''); const [departmentId, setDepartmentId] = useState(''); const [paymentMethodId, setPaymentMethodId] = useState(''); const [amount, setAmount] = useState(''); const [description, setDescription] = useState(''); const [reference, setReference] = useState(''); const [expenseDate, setExpenseDate] = useState(new Date().toISOString().slice(0, 10));
  const refresh = useCallback(async () => { try { const next = await invoke<OperationsSnapshot>('get_operations_snapshot'); setSnapshot(next); setCategoryId(current => current || next.categories[0]?.id || ''); setDepartmentId(current => current || next.departments[0]?.id || ''); setPaymentMethodId(current => current || next.paymentMethods[0]?.id || ''); setError(null); } catch (reason) { setError(errorText(reason)); } }, []);
  useEffect(() => { void refresh(); }, [refresh]);
  async function act(action: () => Promise<unknown>, success: string) { setError(null); setNotice(null); try { await action(); setNotice(success); await refresh(); } catch (reason) { setError(errorText(reason)); } }
  async function openShift(event: FormEvent<HTMLFormElement>) { event.preventDefault(); await act(() => invoke('open_pos_shift', { input: { openingBalanceMinor: parseScaled(openingCash, 2), openedAt: new Date().toISOString() } }), 'Shift opened and ready for cash activity.'); }
  async function closeShift(event: FormEvent<HTMLFormElement>) { event.preventDefault(); await act(() => invoke('close_current_shift', { input: { actualBalanceMinor: parseScaled(actualCash, 2), closedAt: new Date().toISOString(), notes: closeNotes || null } }), 'Shift closed and variance preserved.'); setActualCash(''); setCloseNotes(''); }
  async function addCategory(event: FormEvent<HTMLFormElement>) { event.preventDefault(); await act(() => invoke('create_expense_category', { name: categoryName }), `Expense category ${categoryName} created.`); setCategoryName(''); }
  async function addExpense(event: FormEvent<HTMLFormElement>) { event.preventDefault(); await act(() => invoke('record_operating_expense', { input: { departmentId: departmentId || null, categoryId, paymentMethodId: paymentMethodId || null, amountMinor: parseScaled(amount, 2), expenseDate, description, reference: reference || null, receiptImagePath: null, idempotencyKey: crypto.randomUUID() } }), 'Expense recorded and cash expectation updated when applicable.'); setAmount(''); setDescription(''); setReference(''); }
  async function voidExpense(expenseId: string) { const reason = window.prompt('Reason for voiding this expense?'); if (!reason?.trim()) return; await act(() => invoke('void_operating_expense', { input: { expenseId, occurredAt: new Date().toISOString(), reason } }), 'Expense voided without deleting its history.'); }
  return <>
    <div className="flex items-center justify-between mb-space-lg">
      <div>
        <p className="font-badge-label text-primary uppercase tracking-wider">CASH CONTROL</p>
        <h2 className="font-headline-lg text-on-surface mt-1">Shifts & expenses</h2>
      </div>
      <span className={`px-3 py-1 ${snapshot?.openShift ? 'bg-secondary-container text-on-secondary-container' : 'bg-surface-container text-on-surface-variant'} rounded-full font-body-sm font-medium flex items-center gap-2`}>
        <span className={`w-2 h-2 rounded-full ${snapshot?.openShift ? 'bg-secondary' : 'bg-outline'}`}></span>
        {snapshot?.openShift ? 'Open shift' : 'No open shift'}
      </span>
    </div>
    
    {error && <p className="p-4 mb-4 rounded-lg bg-error-container text-on-error-container font-body-md shadow-sm" role="alert">{error}</p>}
    {notice && <p className="p-4 mb-4 rounded-lg bg-secondary-container text-on-secondary-container font-body-md shadow-sm" role="status">{notice}</p>}
    
    {!snapshot ? <div className="p-8 text-center text-on-surface-variant font-body-md bg-surface-container-lowest rounded-xl">Loading operations…</div> : <>
      {snapshot.openShift ? (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-space-md mb-space-lg">
          <article className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col">
            <span className="font-ticket-total text-on-surface mb-2">{formatMoney(snapshot.openShift.openingBalanceMinor)}</span>
            <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Opening cash</span>
          </article>
          <article className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col">
            <span className="font-ticket-total text-on-surface mb-2">{formatMoney(snapshot.openShift.expectedBalanceMinor)}</span>
            <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Expected cash now</span>
          </article>
          <article className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col">
            <span className="font-ticket-total text-on-surface mb-2">{new Date(snapshot.openShift.openedAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>
            <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Shift opened</span>
          </article>
        </div>
      ) : (
        <form className="max-w-md mb-space-lg p-8 bg-surface-container-lowest rounded-2xl border border-outline-variant/20 shadow-lg flex flex-col" onSubmit={openShift}>
          <h3 className="font-headline-md text-on-surface mb-2">Open shift</h3>
          <p className="font-body-md text-on-surface-variant mb-6">Start a new immutable cashier period for this terminal.</p>
          <label className="w-full text-left mb-6">
            <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Opening cash</span>
            <input value={openingCash} onChange={event => setOpeningCash(event.target.value)} inputMode="decimal" required className="w-full h-12 px-4 rounded-xl bg-surface border border-outline-variant/30 text-on-surface text-lg focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all" />
          </label>
          <button className="w-full py-3 rounded-xl bg-primary text-on-primary font-headline-sm hover:bg-primary-fixed transition-colors flex items-center justify-center gap-2">
            <span className="material-symbols-outlined text-sm">lock_open</span> Open Shift
          </button>
        </form>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-space-md mb-space-lg">
        <form className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col" onSubmit={addCategory}>
          <h3 className="font-headline-sm text-on-surface mb-2">Expense category</h3>
          <p className="font-body-sm text-on-surface-variant mb-6">Categories stay reusable for reports.</p>
          <label className="w-full text-left mb-4">
            <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Name</span>
            <input value={categoryName} onChange={event => setCategoryName(event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all" />
          </label>
          <button className="w-full py-2 mb-4 rounded-lg bg-surface-container hover:bg-surface-container-highest text-on-surface font-medium transition-colors border border-outline-variant/30">Create category</button>
          <div className="flex flex-wrap gap-2 mt-auto">
            {snapshot.categories.map(category => <span key={category.id} className="px-3 py-1 bg-surface-container rounded-full text-xs font-medium text-on-surface-variant">{category.name}</span>)}
          </div>
        </form>

        <form className="lg:col-span-2 p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col" onSubmit={addExpense}>
          <h3 className="font-headline-sm text-on-surface mb-2">Record expense</h3>
          <p className="font-body-sm text-on-surface-variant mb-6">Cash expenses immediately reduce this shift’s expected drawer balance.</p>
          
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-4">
            <label className="w-full">
              <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Category</span>
              <select value={categoryId} onChange={event => setCategoryId(event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none">
                <option value="">Choose category</option>
                {snapshot.categories.map(category => <option key={category.id} value={category.id}>{category.name}</option>)}
              </select>
            </label>
            <label className="w-full">
              <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Department</span>
              <select value={departmentId} onChange={event => setDepartmentId(event.target.value)} className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none">
                <option value="">Business-wide</option>
                {snapshot.departments.map(department => <option key={department.id} value={department.id}>{department.name}</option>)}
              </select>
            </label>
            <label className="w-full">
              <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Payment method</span>
              <select value={paymentMethodId} onChange={event => setPaymentMethodId(event.target.value)} className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none">
                <option value="">Not specified</option>
                {snapshot.paymentMethods.map(method => <option key={method.id} value={method.id}>{method.name}</option>)}
              </select>
            </label>
            <label className="w-full">
              <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Amount</span>
              <input value={amount} onChange={event => setAmount(event.target.value)} inputMode="decimal" required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
            </label>
          </div>
          
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
            <label className="w-full">
              <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Date</span>
              <input value={expenseDate} onChange={event => setExpenseDate(event.target.value)} type="date" required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
            </label>
            <label className="w-full">
              <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Description</span>
              <input value={description} onChange={event => setDescription(event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
            </label>
            <label className="w-full">
              <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Reference</span>
              <input value={reference} onChange={event => setReference(event.target.value)} className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
            </label>
          </div>
          
          <button disabled={!snapshot.openShift || !snapshot.categories.length} className="w-full py-3 mt-auto rounded-xl bg-primary text-on-primary font-headline-sm hover:bg-primary-fixed transition-colors disabled:opacity-50">
            Record expense
          </button>
        </form>
      </div>
      
      {snapshot.openShift && (
        <form className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col md:flex-row md:items-end gap-4 mb-space-lg" onSubmit={closeShift}>
          <div className="flex-1">
            <h3 className="font-headline-sm text-on-surface mb-2">Close & reconcile shift</h3>
            <p className="font-body-sm text-on-surface-variant">Expected cash is frozen alongside the counted amount and variance.</p>
          </div>
          <label className="w-full md:w-48">
            <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Actual cash counted</span>
            <input value={actualCash} onChange={event => setActualCash(event.target.value)} inputMode="decimal" required className="w-full h-12 px-3 rounded-xl bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none text-lg" />
          </label>
          <label className="w-full md:w-64">
            <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Closing notes</span>
            <input value={closeNotes} onChange={event => setCloseNotes(event.target.value)} className="w-full h-12 px-3 rounded-xl bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
          </label>
          <button className="w-full md:w-auto px-6 py-3 h-12 rounded-xl bg-surface-container text-on-surface font-medium hover:bg-surface-container-highest transition-colors border border-outline-variant/30">
            Close shift
          </button>
        </form>
      )}
      
      <div className="bg-surface rounded-xl border border-outline-variant/20 shadow-sm overflow-hidden mb-space-lg">
        <div className="p-6 border-b border-outline-variant/20 bg-surface-container-lowest">
          <h3 className="font-headline-sm text-on-surface">Expense history</h3>
          <p className="font-body-sm text-on-surface-variant mt-1">Financial history is voided, never deleted.</p>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-left font-body-sm">
            <thead className="bg-surface-container border-b border-outline-variant/20">
              <tr>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Date</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Description</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Category</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Department</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Payment</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Amount</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Status</th>
                <th className="px-6 py-3"></th>
              </tr>
            </thead>
            <tbody className="divide-y divide-outline-variant/10">
              {snapshot.expenses.map(expense => (
                <tr key={expense.id} className="hover:bg-surface-container-lowest transition-colors">
                  <td className="px-6 py-4 text-on-surface-variant">{expense.expenseDate}</td>
                  <td className="px-6 py-4">
                    <strong className="block font-body-md font-medium text-on-surface">{expense.description}</strong>
                    <small className="text-on-surface-variant">{expense.reference}</small>
                  </td>
                  <td className="px-6 py-4 text-on-surface">{expense.categoryName}</td>
                  <td className="px-6 py-4 text-on-surface-variant">{expense.departmentName ?? 'Business-wide'}</td>
                  <td className="px-6 py-4 text-on-surface-variant">{expense.paymentMethodName ?? '—'}</td>
                  <td className="px-6 py-4 font-medium text-right text-on-surface">{formatMoney(expense.amountMinor)}</td>
                  <td className="px-6 py-4">
                    <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium capitalize ${expense.status === 'RECORDED' ? 'bg-secondary-container text-on-secondary-container' : 'bg-surface-container-highest text-on-surface-variant'}`}>
                      {expense.status.toLowerCase()}
                    </span>
                  </td>
                  <td className="px-6 py-4 text-right">
                    {expense.status === 'RECORDED' && <button className="text-error hover:text-error/80 font-medium transition-colors" onClick={() => void voidExpense(expense.id)}>Void</button>}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {!snapshot.expenses.length && <div className="p-8 text-center text-on-surface-variant font-body-md">No expenses recorded.</div>}
        </div>
      </div>
      
      <div className="bg-surface rounded-xl border border-outline-variant/20 shadow-sm overflow-hidden">
        <div className="p-6 border-b border-outline-variant/20 bg-surface-container-lowest">
          <h3 className="font-headline-sm text-on-surface">Shift history</h3>
          <p className="font-body-sm text-on-surface-variant mt-1">Expected, actual, and variance remain preserved after close.</p>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-left font-body-sm">
            <thead className="bg-surface-container border-b border-outline-variant/20">
              <tr>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Opened</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Status</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Opening</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Expected</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Actual</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Variance</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-outline-variant/10">
              {snapshot.shifts.map(item => (
                <tr key={item.id} className="hover:bg-surface-container-lowest transition-colors">
                  <td className="px-6 py-4 text-on-surface-variant">{new Date(item.openedAt).toLocaleString()}</td>
                  <td className="px-6 py-4">
                    <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium capitalize ${item.status === 'OPEN' ? 'bg-secondary-container text-on-secondary-container' : 'bg-surface-container-highest text-on-surface-variant'}`}>
                      {item.status.toLowerCase()}
                    </span>
                  </td>
                  <td className="px-6 py-4 text-right text-on-surface font-medium">{formatMoney(item.openingBalanceMinor)}</td>
                  <td className="px-6 py-4 text-right text-on-surface font-medium">{formatMoney(item.expectedBalanceMinor)}</td>
                  <td className="px-6 py-4 text-right text-on-surface font-medium">{item.actualBalanceMinor == null ? '—' : formatMoney(item.actualBalanceMinor)}</td>
                  <td className={`px-6 py-4 text-right font-medium ${(item.varianceMinor ?? 0) < 0 ? 'text-error' : (item.varianceMinor ?? 0) > 0 ? 'text-secondary' : 'text-on-surface'}`}>
                    {item.varianceMinor == null ? '—' : formatMoney(item.varianceMinor)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </>}
  </>;
}

function ReportsView() {
  const today = new Date().toISOString().slice(0, 10); const [startDate, setStartDate] = useState(`${today.slice(0, 8)}01`); const [endDate, setEndDate] = useState(today); const [report, setReport] = useState<DashboardReport | null>(null); const [error, setError] = useState<string | null>(null);
  const range = useCallback(() => ({ startAt: `${startDate}T00:00:00.000Z`, endAt: `${endDate}T23:59:59.999Z` }), [startDate, endDate]);
  const refresh = useCallback(async () => { try { setReport(await invoke<DashboardReport>('get_dashboard_report', { range: range() })); setError(null); } catch (reason) { setError(errorText(reason)); } }, [range]);
  useEffect(() => { void refresh(); }, [refresh]);
  async function exportCsv() { try { const csv = await invoke<string>('export_dashboard_csv', { range: range() }); const url = URL.createObjectURL(new Blob([csv], { type: 'text/csv;charset=utf-8' })); const link = document.createElement('a'); link.href = url; link.download = `localops-report-${startDate}-to-${endDate}.csv`; link.click(); URL.revokeObjectURL(url); } catch (reason) { setError(errorText(reason)); } }
  return <div className="max-w-7xl mx-auto flex flex-col h-full">
    <header className="flex flex-col md:flex-row md:items-end justify-between mb-space-lg gap-4">
      <div>
        <p className="font-body-sm text-primary uppercase tracking-wider mb-1">REPORTS</p>
        <h2 className="font-display text-on-surface">Business dashboard</h2>
      </div>
      <span className="inline-flex items-center gap-2 px-3 py-1 bg-surface-container-highest text-on-surface-variant font-body-sm rounded-full">
        <span className="w-2 h-2 rounded-full bg-secondary"></span> Persisted facts
      </span>
    </header>

    <div className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-wrap items-end gap-4 mb-space-lg">
      <label className="flex-1 min-w-[200px]">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">From</span>
        <input type="date" value={startDate} onChange={event => setStartDate(event.target.value)} className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
      </label>
      <label className="flex-1 min-w-[200px]">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">To</span>
        <input type="date" value={endDate} onChange={event => setEndDate(event.target.value)} className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
      </label>
      <button onClick={() => void refresh()} className="px-6 h-10 rounded-lg bg-primary text-on-primary font-medium hover:bg-primary-fixed transition-colors">Refresh</button>
      <button onClick={() => void exportCsv()} className="px-6 h-10 rounded-lg bg-surface-container text-on-surface font-medium hover:bg-surface-container-highest transition-colors border border-outline-variant/30">Export CSV</button>
    </div>

    {error && <p className="p-4 mb-space-lg rounded-lg bg-error-container text-on-error-container font-body-md shadow-sm" role="alert">{error}</p>}
    
    {!report ? <div className="p-8 text-center text-on-surface-variant font-body-md bg-surface-container-lowest rounded-xl">Calculating local reports…</div> : <>
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-space-md mb-space-lg">
        <article className="p-4 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col justify-between min-h-[100px]">
          <strong className="font-ticket-total text-on-surface mb-1 truncate" title={formatMoney(report.totals.netSalesMinor)}>{formatMoney(report.totals.netSalesMinor)}</strong>
          <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Net sales</span>
        </article>
        <article className="p-4 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col justify-between min-h-[100px]">
          <strong className="font-ticket-total text-on-surface mb-1 truncate" title={formatMoney(report.totals.grossProfitMinor)}>{formatMoney(report.totals.grossProfitMinor)}</strong>
          <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Estimated gross profit</span>
        </article>
        <article className="p-4 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col justify-between min-h-[100px]">
          <strong className="font-ticket-total text-on-surface mb-1 truncate" title={formatMoney(report.totals.expensesMinor)}>{formatMoney(report.totals.expensesMinor)}</strong>
          <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Expenses</span>
        </article>
        <article className="p-4 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col justify-between min-h-[100px]">
          <strong className="font-ticket-total text-on-surface mb-1 truncate" title={formatMoney(report.totals.refundsMinor)}>{formatMoney(report.totals.refundsMinor)}</strong>
          <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Refunds & voids</span>
        </article>
        <article className="p-4 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col justify-between min-h-[100px]">
          <strong className="font-ticket-total text-on-surface mb-1 truncate" title={formatMoney(report.totals.inventoryValueMinor)}>{formatMoney(report.totals.inventoryValueMinor)}</strong>
          <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Stock value</span>
        </article>
        <article className={`p-4 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col justify-between min-h-[100px] ${report.totals.lowStockCount ? 'ring-2 ring-error/50 bg-error/5' : ''}`}>
          <strong className={`font-ticket-total mb-1 ${report.totals.lowStockCount ? 'text-error' : 'text-on-surface'}`}>{report.totals.lowStockCount}</strong>
          <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Low-stock balances</span>
        </article>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-space-lg mb-space-lg">
        <div className="bg-surface rounded-xl border border-outline-variant/20 shadow-sm overflow-hidden flex flex-col">
          <div className="p-6 border-b border-outline-variant/20 bg-surface-container-lowest">
            <h3 className="font-headline-sm text-on-surface">Departments</h3>
            <p className="font-body-sm text-on-surface-variant mt-1">Original line snapshots less refund events.</p>
          </div>
          <div className="overflow-x-auto flex-1">
            <table className="w-full text-left font-body-sm whitespace-nowrap">
              <thead className="bg-surface-container border-b border-outline-variant/20">
                <tr>
                  <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Department</th>
                  <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Gross</th>
                  <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Refunds</th>
                  <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Net</th>
                  <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Gross profit</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-outline-variant/10">
                {report.departments.map(item => <tr key={item.departmentId} className="hover:bg-surface-container-lowest transition-colors">
                  <td className="px-6 py-4"><strong className="font-medium text-on-surface">{item.departmentName}</strong></td>
                  <td className="px-6 py-4 text-right text-on-surface-variant">{formatMoney(item.grossSalesMinor)}</td>
                  <td className="px-6 py-4 text-right text-on-surface-variant">{formatMoney(item.refundsMinor)}</td>
                  <td className="px-6 py-4 text-right font-medium text-on-surface">{formatMoney(item.netSalesMinor)}</td>
                  <td className="px-6 py-4 text-right font-medium text-primary">{formatMoney(item.grossProfitMinor)}</td>
                </tr>)}
              </tbody>
            </table>
            {!report.departments.length && <div className="p-8 text-center text-on-surface-variant font-body-md">No sales in this period.</div>}
          </div>
        </div>

        <div className="bg-surface rounded-xl border border-outline-variant/20 shadow-sm overflow-hidden flex flex-col">
          <div className="p-6 border-b border-outline-variant/20 bg-surface-container-lowest">
            <h3 className="font-headline-sm text-on-surface">Payments</h3>
            <p className="font-body-sm text-on-surface-variant mt-1">Receipts and explicit payment reversals.</p>
          </div>
          <div className="overflow-x-auto flex-1">
            <table className="w-full text-left font-body-sm whitespace-nowrap">
              <thead className="bg-surface-container border-b border-outline-variant/20">
                <tr>
                  <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Method</th>
                  <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Received</th>
                  <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Refunded</th>
                  <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Net</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-outline-variant/10">
                {report.payments.map(item => <tr key={`${item.methodKind}:${item.methodName}`} className="hover:bg-surface-container-lowest transition-colors">
                  <td className="px-6 py-4">
                    <strong className="block font-medium text-on-surface">{item.methodName}</strong>
                    <small className="block text-on-surface-variant">{item.methodKind}</small>
                  </td>
                  <td className="px-6 py-4 text-right text-on-surface-variant">{formatMoney(item.receivedMinor)}</td>
                  <td className="px-6 py-4 text-right text-on-surface-variant">{formatMoney(item.refundedMinor)}</td>
                  <td className="px-6 py-4 text-right font-medium text-on-surface">{formatMoney(item.netMinor)}</td>
                </tr>)}
              </tbody>
            </table>
            {!report.payments.length && <div className="p-8 text-center text-on-surface-variant font-body-md">No payments in this period.</div>}
          </div>
        </div>
      </div>

      <div className="bg-surface rounded-xl border border-outline-variant/20 shadow-sm overflow-hidden mb-space-lg">
        <div className="p-6 border-b border-outline-variant/20 bg-surface-container-lowest">
          <h3 className="font-headline-sm text-on-surface">Current stock valuation</h3>
          <p className="font-body-sm text-on-surface-variant mt-1">Live ledger balances valued at the latest persisted unit cost.</p>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-left font-body-sm whitespace-nowrap">
            <thead className="bg-surface-container border-b border-outline-variant/20">
              <tr>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Product</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Location</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">On hand</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Minimum</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Value</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-outline-variant/10">
              {report.stock.map((item, index) => <tr key={`${item.productName}:${item.locationName}:${index}`} className="hover:bg-surface-container-lowest transition-colors">
                <td className="px-6 py-4"><strong className="font-medium text-on-surface">{item.productName}</strong></td>
                <td className="px-6 py-4 text-on-surface-variant">{item.locationName}</td>
                <td className={`px-6 py-4 text-right font-medium ${item.quantityMicros <= item.minimumQuantityMicros ? 'text-error' : 'text-on-surface'}`}>{formatQuantity(item.quantityMicros)}</td>
                <td className="px-6 py-4 text-right text-on-surface-variant">{formatQuantity(item.minimumQuantityMicros)}</td>
                <td className="px-6 py-4 text-right text-on-surface-variant">{formatMoney(item.valueMinor)}</td>
              </tr>)}
            </tbody>
          </table>
          {!report.stock.length && <div className="p-8 text-center text-on-surface-variant font-body-md">No inventory balances yet.</div>}
        </div>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-4 p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm mt-auto">
        <div className="flex gap-6">
          <span className="flex flex-col"><span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Gross sales</span><strong className="font-ticket-total text-on-surface">{formatMoney(report.totals.grossSalesMinor)}</strong></span>
          <span className="flex flex-col"><span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Shift variance</span><strong className={`font-ticket-total ${report.totals.shiftVarianceMinor < 0 ? 'text-error' : report.totals.shiftVarianceMinor > 0 ? 'text-secondary' : 'text-on-surface'}`}>{formatMoney(report.totals.shiftVarianceMinor)}</strong></span>
        </div>
        <span className="font-body-sm text-on-surface-variant bg-surface-container px-3 py-1 rounded-full">Range {startDate} to {endDate}</span>
      </div>
    </>}
  </div>;
}

function SafetyView() {
  const [status, setStatus] = useState<SafetyStatus | null>(null); const [error, setError] = useState<string | null>(null); const [notice, setNotice] = useState<string | null>(null); const [busy, setBusy] = useState(false);
  const refresh = useCallback(async () => { try { setStatus(await invoke<SafetyStatus>('get_safety_status')); setError(null); } catch (reason) { setError(errorText(reason)); } }, []);
  useEffect(() => { void refresh(); }, [refresh]);
  async function createBackup() { setBusy(true); setError(null); try { const backup = await invoke<BackupView>('create_manual_backup'); setNotice(`Verified backup created: ${backup.filename}`); await refresh(); } catch (reason) { setError(errorText(reason)); } finally { setBusy(false); } }
  async function restoreBackup(filename: string) { if (!window.confirm(`Restore ${filename}? A verified pre-restore safety copy will be created first, and you will be signed out.`)) return; setBusy(true); setError(null); try { await invoke('restore_local_backup', { filename }); window.location.reload(); } catch (reason) { setError(errorText(reason)); setBusy(false); } }
  return <div className="max-w-5xl mx-auto flex flex-col h-full">
    <header className="flex flex-col md:flex-row md:items-end justify-between mb-space-lg gap-4">
      <div>
        <p className="font-body-sm text-primary uppercase tracking-wider mb-1">DATA SAFETY</p>
        <h2 className="font-display text-on-surface">Backup & recovery</h2>
      </div>
      <span className="inline-flex items-center gap-2 px-3 py-1 bg-surface-container-highest text-on-surface-variant font-body-sm rounded-full">
        <span className="w-2 h-2 rounded-full bg-secondary"></span> Fully local
      </span>
    </header>

    {error && <p className="p-4 mb-space-lg rounded-lg bg-error-container text-on-error-container font-body-md shadow-sm" role="alert">{error}</p>}
    {notice && <p className="p-4 mb-space-lg rounded-lg bg-secondary-container text-on-secondary-container font-body-md shadow-sm" role="status">{notice}</p>}

    {!status ? <div className="p-8 text-center text-on-surface-variant font-body-md bg-surface-container-lowest rounded-xl">Verifying database and backups…</div> : <>
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-space-md mb-space-lg">
        <article className={`p-4 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col justify-between min-h-[100px] ${status.integrity !== 'ok' ? 'ring-2 ring-error/50 bg-error/5' : ''}`}>
          <strong className={`font-ticket-total mb-1 truncate ${status.integrity !== 'ok' ? 'text-error' : 'text-on-surface'}`} title={status.integrity}>{status.integrity}</strong>
          <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">SQLite integrity</span>
        </article>
        <article className={`p-4 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col justify-between min-h-[100px] ${status.foreignKeyViolations ? 'ring-2 ring-error/50 bg-error/5' : ''}`}>
          <strong className={`font-ticket-total mb-1 ${status.foreignKeyViolations ? 'text-error' : 'text-on-surface'}`}>{status.foreignKeyViolations}</strong>
          <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Foreign-key violations</span>
        </article>
        <article className={`p-4 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col justify-between min-h-[100px] ${status.inventoryReconciliationDifferences ? 'ring-2 ring-error/50 bg-error/5' : ''}`}>
          <strong className={`font-ticket-total mb-1 ${status.inventoryReconciliationDifferences ? 'text-error' : 'text-on-surface'}`}>{status.inventoryReconciliationDifferences}</strong>
          <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Stock differences</span>
        </article>
        <article className={`p-4 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col justify-between min-h-[100px] ${status.financialEventsWithoutAudit ? 'ring-2 ring-error/50 bg-error/5' : ''}`}>
          <strong className={`font-ticket-total mb-1 ${status.financialEventsWithoutAudit ? 'text-error' : 'text-on-surface'}`}>{status.financialEventsWithoutAudit}</strong>
          <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Unaudited financial events</span>
        </article>
        <article className="p-4 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col justify-between min-h-[100px]">
          <strong className="font-ticket-total text-on-surface mb-1 truncate">v{status.schemaVersion}</strong>
          <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Schema (latest v{status.latestSchemaVersion})</span>
        </article>
        <article className={`p-4 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col justify-between min-h-[100px] ${!status.healthy ? 'ring-2 ring-error/50 bg-error/5' : ''}`}>
          <strong className={`font-ticket-total mb-1 ${!status.healthy ? 'text-error' : 'text-on-surface'}`}>{status.healthy ? 'Healthy' : 'Review'}</strong>
          <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Overall state</span>
        </article>
      </div>

      <div className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col md:flex-row md:items-center justify-between gap-4 mb-space-lg">
        <div>
          <h3 className="font-headline-sm text-on-surface">Verified local backups</h3>
          <p className="font-body-sm text-on-surface-variant mt-1">Automatic retention keeps 7 daily, 4 weekly, and 12 monthly copies. Manual and pre-restore copies are kept separately.</p>
        </div>
        <button onClick={() => void createBackup()} disabled={busy} className="whitespace-nowrap px-6 py-3 rounded-xl bg-primary text-on-primary font-headline-sm hover:bg-primary-fixed transition-colors disabled:opacity-50">
          {busy ? 'Working…' : 'Create manual backup'}
        </button>
      </div>

      <div className="bg-surface rounded-xl border border-outline-variant/20 shadow-sm overflow-hidden mb-space-lg">
        <div className="overflow-x-auto">
          <table className="w-full text-left font-body-sm whitespace-nowrap">
            <thead className="bg-surface-container border-b border-outline-variant/20">
              <tr>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Backup</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Kind</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Size</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-center">Verification</th>
                <th className="px-6 py-3"></th>
              </tr>
            </thead>
            <tbody className="divide-y divide-outline-variant/10">
              {status.backups.map(backup => <tr key={backup.filename} className="hover:bg-surface-container-lowest transition-colors">
                <td className="px-6 py-4"><strong className="font-medium text-on-surface">{backup.filename}</strong></td>
                <td className="px-6 py-4">
                  <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium capitalize bg-surface-container-highest text-on-surface-variant">
                    {backup.kind}
                  </span>
                </td>
                <td className="px-6 py-4 text-right text-on-surface-variant">{formatBytes(backup.sizeBytes)}</td>
                <td className="px-6 py-4 text-center">
                  <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${backup.verified ? 'bg-secondary-container text-on-secondary-container' : 'bg-error-container text-on-error-container'}`}>
                    {backup.verified ? 'Verified' : 'Invalid'}
                  </span>
                </td>
                <td className="px-6 py-4 text-right">
                  <button className="text-primary hover:text-primary-fixed font-medium transition-colors disabled:opacity-50" disabled={!backup.verified || busy} onClick={() => void restoreBackup(backup.filename)}>Restore</button>
                </td>
              </tr>)}
            </tbody>
          </table>
          {!status.backups.length && <div className="p-8 text-center text-on-surface-variant font-body-md">No backups found. Create one now.</div>}
        </div>
      </div>
    </>}
  </div>;
}

function InventoryView() {
  const [snapshot, setSnapshot] = useState<InventorySnapshot | null>(null); const [error, setError] = useState<string | null>(null); const [notice, setNotice] = useState<string | null>(null);
  const refresh = useCallback(async () => { try { setSnapshot(await invoke<InventorySnapshot>('get_inventory_snapshot')); setError(null); } catch (reason) { setError(errorText(reason)); } }, []);
  useEffect(() => { void refresh(); }, [refresh]);
  async function act(action: () => Promise<unknown>, success: string) { setError(null); setNotice(null); try { await action(); setNotice(success); await refresh(); } catch (reason) { setError(errorText(reason)); } }
  return <div className="max-w-7xl mx-auto flex flex-col h-full">
    <header className="flex flex-col md:flex-row md:items-end justify-between mb-space-lg gap-4">
      <div>
        <p className="font-body-sm text-primary uppercase tracking-wider mb-1">INVENTORY</p>
        <h2 className="font-display text-on-surface">Stock control</h2>
      </div>
      <span className="inline-flex items-center gap-2 px-3 py-1 bg-surface-container-highest text-on-surface-variant font-body-sm rounded-full">
        <span className="w-2 h-2 rounded-full bg-secondary"></span> Ledger-backed
      </span>
    </header>

    {error && <p className="p-4 mb-space-lg rounded-lg bg-error-container text-on-error-container font-body-md shadow-sm" role="alert">{error}</p>}
    {notice && <p className="p-4 mb-space-lg rounded-lg bg-secondary-container text-on-secondary-container font-body-md shadow-sm" role="status">{notice}</p>}

    {!snapshot ? <div className="p-8 text-center text-on-surface-variant font-body-md bg-surface-container-lowest rounded-xl">Loading inventory ledger…</div> : <>
      <div className="grid grid-cols-1 md:grid-cols-3 gap-space-md mb-space-lg">
        <article className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col justify-between min-h-[100px]">
          <strong className="font-ticket-total text-on-surface mb-1">{snapshot.products.length}</strong>
          <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Tracked products</span>
        </article>
        <article className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col justify-between min-h-[100px]">
          <strong className="font-ticket-total text-on-surface mb-1">{snapshot.balances.length}</strong>
          <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Product/location balances</span>
        </article>
        <article className={`p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col justify-between min-h-[100px] ${snapshot.reconciliationDifferenceCount ? 'ring-2 ring-error/50 bg-error/5' : ''}`}>
          <strong className={`font-ticket-total mb-1 ${snapshot.reconciliationDifferenceCount ? 'text-error' : 'text-on-surface'}`}>{snapshot.reconciliationDifferenceCount}</strong>
          <span className="font-body-sm text-on-surface-variant uppercase tracking-wider">Reconciliation differences</span>
        </article>
      </div>

      <div className="bg-surface rounded-xl border border-outline-variant/20 shadow-sm overflow-hidden mb-space-lg">
        <div className="overflow-x-auto">
          <table className="w-full text-left font-body-sm whitespace-nowrap">
            <thead className="bg-surface-container border-b border-outline-variant/20">
              <tr>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Product</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Location</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">On hand</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Cache version</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-outline-variant/10">
              {snapshot.balances.map(balance => <tr key={`${balance.productId}:${balance.locationId}`} className="hover:bg-surface-container-lowest transition-colors">
                <td className="px-6 py-4"><strong className="font-medium text-on-surface">{balance.productName}</strong></td>
                <td className="px-6 py-4 text-on-surface-variant">{balance.locationName}</td>
                <td className="px-6 py-4 text-right text-on-surface font-medium">{formatQuantity(balance.quantityMicros)} <span className="text-on-surface-variant font-normal">{snapshot.products.find(value => value.id === balance.productId)?.baseUnitCode}</span></td>
                <td className="px-6 py-4 text-right text-on-surface-variant">{balance.version}</td>
              </tr>)}
            </tbody>
          </table>
          {snapshot.balances.length === 0 && <div className="p-8 text-center text-on-surface-variant font-body-md">No stock has been received yet.</div>}
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-space-lg mb-space-lg items-start">
        <SupplierForm act={act} />
        <ReceiveForm snapshot={snapshot} act={act} />
        <TransferForm snapshot={snapshot} act={act} />
        <WastageForm snapshot={snapshot} act={act} />
        <CountForm snapshot={snapshot} act={act} />
      </div>

      <div className="bg-surface rounded-xl border border-outline-variant/20 shadow-sm overflow-hidden mb-space-lg">
        <div className="p-6 border-b border-outline-variant/20 bg-surface-container-lowest">
          <h3 className="font-headline-sm text-on-surface">Recent movements</h3>
          <p className="font-body-sm text-on-surface-variant mt-1">The append-only source of truth for every balance.</p>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-left font-body-sm whitespace-nowrap">
            <thead className="bg-surface-container border-b border-outline-variant/20">
              <tr>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Time</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Product</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Location</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Type</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider text-right">Quantity</th>
                <th className="px-6 py-3 font-medium text-on-surface-variant uppercase tracking-wider">Reference</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-outline-variant/10">
              {snapshot.recentMovements.map(movement => <tr key={movement.id} className="hover:bg-surface-container-lowest transition-colors">
                <td className="px-6 py-4 text-on-surface-variant">{new Date(movement.occurredAt).toLocaleString()}</td>
                <td className="px-6 py-4 font-medium text-on-surface">{movement.productName}</td>
                <td className="px-6 py-4 text-on-surface-variant">{movement.locationName}</td>
                <td className="px-6 py-4">
                  <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium capitalize bg-surface-container-highest text-on-surface-variant">
                    {movement.movementType.replace('_', ' ').toLowerCase()}
                  </span>
                </td>
                <td className={`px-6 py-4 text-right font-medium ${movement.quantityMicros < 0 ? 'text-error' : movement.quantityMicros > 0 ? 'text-secondary' : 'text-on-surface'}`}>
                  {movement.quantityMicros > 0 ? '+' : ''}{formatQuantity(movement.quantityMicros)}
                </td>
                <td className="px-6 py-4">
                  <span className="text-on-surface block">{movement.referenceType}</span>
                  <small className="text-on-surface-variant block">{movement.referenceId.slice(0, 8)}</small>
                </td>
              </tr>)}
            </tbody>
          </table>
        </div>
      </div>
    </>}
  </div>;
}

function SupplierForm({ act }: { act: Act }) {
  const [name, setName] = useState(''); const [phone, setPhone] = useState('');
  return <form className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col" onSubmit={async event => { event.preventDefault(); await act(() => invoke('create_inventory_supplier', { input: { name, contactName: null, email: null, phone: phone || null } }), `${name} added as a supplier.`); setName(''); setPhone(''); }}>
    <h3 className="font-headline-sm text-on-surface mb-2">Add supplier</h3>
    <p className="font-body-sm text-on-surface-variant mb-6">Suppliers remain linked to purchase history.</p>
    <label className="w-full text-left mb-4">
      <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Name</span>
      <input value={name} onChange={event => setName(event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
    </label>
    <label className="w-full text-left mb-6">
      <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Phone (optional)</span>
      <input value={phone} onChange={event => setPhone(event.target.value)} className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
    </label>
    <button className="w-full py-2 mt-auto rounded-lg bg-surface-container hover:bg-surface-container-highest text-on-surface font-medium transition-colors border border-outline-variant/30">Add supplier</button>
  </form>;
}

function ReceiveForm({ snapshot, act }: { snapshot: InventorySnapshot; act: Act }) {
  const [purchaseNumber, setPurchaseNumber] = useState(''); const [supplierId, setSupplierId] = useState(''); const [locationId, setLocationId] = useState(snapshot.locations[0]?.id ?? ''); const [productId, setProductId] = useState(snapshot.products[0]?.id ?? ''); const product = snapshot.products.find(value => value.id === productId); const [packagingId, setPackagingId] = useState(''); const selectedPackaging = product?.packaging.find(value => value.id === packagingId); const [quantity, setQuantity] = useState(''); const [unitCost, setUnitCost] = useState(''); const [invoice, setInvoice] = useState(''); const [paymentStatus, setPaymentStatus] = useState('UNPAID');
  return <form className="xl:col-span-2 p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col" onSubmit={async event => { event.preventDefault(); await act(() => invoke('receive_inventory_purchase', { input: { purchaseNumber, supplierId: supplierId || null, locationId, invoiceReference: invoice || null, purchasedAt: new Date().toISOString(), taxMinor: 0, discountMinor: 0, paymentStatus, notes: null, items: [{ productId, description: product?.name ?? 'Stock item', quantityMicros: parseScaled(quantity, 6), unitId: selectedPackaging?.unitId ?? product?.baseUnitId, packagingId: packagingId || null, unitCostMinor: parseScaled(unitCost, 2) }] } }), `Purchase ${purchaseNumber} received.`); setPurchaseNumber(''); setQuantity(''); setUnitCost(''); }}>
    <h3 className="font-headline-sm text-on-surface mb-2">Receive purchase</h3>
    <p className="font-body-sm text-on-surface-variant mb-6">Receiving creates the purchase document, latest cost, ledger movement, and balance in one transaction.</p>
    
    <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-4">
      <label className="w-full">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Purchase number</span>
        <input value={purchaseNumber} onChange={event => setPurchaseNumber(event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
      </label>
      <label className="w-full">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Supplier</span>
        <select value={supplierId} onChange={event => setSupplierId(event.target.value)} className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none"><option value="">No supplier</option>{snapshot.suppliers.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}</select>
      </label>
      <LocationSelect locations={snapshot.locations} value={locationId} onChange={setLocationId} label="Receive into" />
    </div>

    <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-4">
      <div className="md:col-span-2"><ProductSelect products={snapshot.products} value={productId} onChange={value => { setProductId(value); setPackagingId(''); }} /></div>
      <label className="w-full">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Purchase unit</span>
        <select value={packagingId} onChange={event => setPackagingId(event.target.value)} className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none"><option value="">{product?.baseUnitCode ?? 'Base unit'}</option>{product?.packaging.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}</select>
      </label>
      <label className="w-full">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Quantity</span>
        <input value={quantity} onChange={event => setQuantity(event.target.value)} inputMode="decimal" required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
      </label>
    </div>

    <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
      <label className="w-full">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Unit cost (ZAR)</span>
        <input value={unitCost} onChange={event => setUnitCost(event.target.value)} inputMode="decimal" required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
      </label>
      <label className="w-full">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Invoice reference</span>
        <input value={invoice} onChange={event => setInvoice(event.target.value)} className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
      </label>
      <label className="w-full">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Payment status</span>
        <select value={paymentStatus} onChange={event => setPaymentStatus(event.target.value)} className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none"><option value="UNPAID">Unpaid</option><option value="PARTIAL">Part paid</option><option value="PAID">Paid</option></select>
      </label>
    </div>

    <button disabled={!snapshot.products.length || !snapshot.locations.length} className="w-full py-3 mt-auto rounded-xl bg-primary text-on-primary font-headline-sm hover:bg-primary-fixed transition-colors disabled:opacity-50">Receive stock</button>
  </form>;
}

function TransferForm({ snapshot, act }: { snapshot: InventorySnapshot; act: Act }) {
  const [fromLocationId, setFrom] = useState(snapshot.locations[0]?.id ?? ''); const [toLocationId, setTo] = useState(snapshot.locations[1]?.id ?? ''); const [productId, setProduct] = useState(snapshot.products[0]?.id ?? ''); const [quantity, setQuantity] = useState('');
  return <form className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col" onSubmit={async event => { event.preventDefault(); await act(() => invoke('complete_inventory_transfer', { input: { fromLocationId, toLocationId, occurredAt: new Date().toISOString(), notes: null, items: [{ productId, quantityMicros: parseScaled(quantity, 6) }] } }), 'Stock transfer completed.'); setQuantity(''); }}>
    <h3 className="font-headline-sm text-on-surface mb-2">Transfer stock</h3>
    <p className="font-body-sm text-on-surface-variant mb-6">Both sides post together with one correlation.</p>
    <div className="flex gap-4 mb-4">
      <div className="flex-1"><LocationSelect locations={snapshot.locations} value={fromLocationId} onChange={setFrom} label="From" /></div>
      <div className="flex-1"><LocationSelect locations={snapshot.locations} value={toLocationId} onChange={setTo} label="To" /></div>
    </div>
    <div className="mb-4"><ProductSelect products={snapshot.products} value={productId} onChange={setProduct} /></div>
    <label className="w-full mb-6">
      <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Quantity</span>
      <input value={quantity} onChange={event => setQuantity(event.target.value)} inputMode="decimal" required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
    </label>
    <button disabled={snapshot.locations.length < 2 || !snapshot.products.length} className="w-full py-2 mt-auto rounded-lg bg-surface-container hover:bg-surface-container-highest text-on-surface font-medium transition-colors border border-outline-variant/30 disabled:opacity-50">Complete transfer</button>
  </form>;
}

function WastageForm({ snapshot, act }: { snapshot: InventorySnapshot; act: Act }) {
  const [locationId, setLocation] = useState(snapshot.locations[0]?.id ?? ''); const [productId, setProduct] = useState(snapshot.products[0]?.id ?? ''); const [quantity, setQuantity] = useState(''); const [reason, setReason] = useState(''); const [damage, setDamage] = useState(false);
  return <form className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col" onSubmit={async event => { event.preventDefault(); await act(() => invoke('record_inventory_wastage', { input: { locationId, reason, occurredAt: new Date().toISOString(), notes: null, damage, items: [{ productId, quantityMicros: parseScaled(quantity, 6) }] } }), 'Stock loss recorded.'); setQuantity(''); setReason(''); }}>
    <h3 className="font-headline-sm text-on-surface mb-2">Waste or damage</h3>
    <p className="font-body-sm text-on-surface-variant mb-6">Every loss remains reasoned and auditable.</p>
    <div className="mb-4"><LocationSelect locations={snapshot.locations} value={locationId} onChange={setLocation} label="Location" /></div>
    <div className="mb-4"><ProductSelect products={snapshot.products} value={productId} onChange={setProduct} /></div>
    <div className="flex gap-4 mb-4">
      <label className="flex-1">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Quantity</span>
        <input value={quantity} onChange={event => setQuantity(event.target.value)} inputMode="decimal" required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
      </label>
      <label className="flex-1">
        <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Reason</span>
        <input value={reason} onChange={event => setReason(event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
      </label>
    </div>
    <div className="mb-6"><Toggle label="Physical damage" checked={damage} onChange={setDamage} /></div>
    <button disabled={!snapshot.products.length} className="w-full py-2 mt-auto rounded-lg bg-surface-container hover:bg-surface-container-highest text-on-surface font-medium transition-colors border border-outline-variant/30 disabled:opacity-50">Record loss</button>
  </form>;
}

function CountForm({ snapshot, act }: { snapshot: InventorySnapshot; act: Act }) {
  const [locationId, setLocation] = useState(snapshot.locations[0]?.id ?? ''); const [productId, setProduct] = useState(snapshot.products[0]?.id ?? ''); const [counted, setCounted] = useState('');
  return <form className="p-6 bg-surface-container-lowest rounded-xl border border-outline-variant/20 shadow-sm flex flex-col" onSubmit={async event => { event.preventDefault(); await act(() => invoke('complete_inventory_stock_count', { input: { locationId, countedAt: new Date().toISOString(), notes: null, items: [{ productId, countedMicros: parseScaled(counted, 6) }] } }), 'Stock count completed and variance posted.'); setCounted(''); }}>
    <h3 className="font-headline-sm text-on-surface mb-2">Stock count</h3>
    <p className="font-body-sm text-on-surface-variant mb-6">Expected stock is preserved before any variance adjustment.</p>
    <div className="mb-4"><LocationSelect locations={snapshot.locations} value={locationId} onChange={setLocation} label="Location" /></div>
    <div className="mb-4"><ProductSelect products={snapshot.products} value={productId} onChange={setProduct} /></div>
    <label className="w-full mb-6">
      <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Counted quantity</span>
      <input value={counted} onChange={event => setCounted(event.target.value)} inputMode="decimal" required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none" />
    </label>
    <button disabled={!snapshot.products.length} className="w-full py-2 mt-auto rounded-lg bg-surface-container hover:bg-surface-container-highest text-on-surface font-medium transition-colors border border-outline-variant/30 disabled:opacity-50">Complete count</button>
  </form>;
}

function ProductSelect({ products, value, onChange }: { products: InventoryProduct[]; value: string; onChange: (value: string) => void }) { 
  return <label className="w-full block">
    <span className="block font-body-sm font-medium text-on-surface-variant mb-2">Product</span>
    <select value={value} onChange={event => onChange(event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none">
      <option value="">Choose product</option>
      {products.map(product => <option key={product.id} value={product.id}>{product.name} ({product.baseUnitCode})</option>)}
    </select>
  </label>; 
}
function LocationSelect({ locations, value, onChange, label }: { locations: InventoryLocation[]; value: string; onChange: (value: string) => void; label: string }) { 
  return <label className="w-full block">
    <span className="block font-body-sm font-medium text-on-surface-variant mb-2">{label}</span>
    <select value={value} onChange={event => onChange(event.target.value)} required className="w-full h-10 px-3 rounded-lg bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none">
      <option value="">Choose location</option>
      {locations.map(location => <option key={location.id} value={location.id}>{location.name}</option>)}
    </select>
  </label>; 
}

export function parseScaled(raw: string, decimals: number): number { const match = /^([+-]?)(\d+)(?:\.(\d+))?$/.exec(raw.trim()); if (!match || (match[3]?.length ?? 0) > decimals) throw new Error(`Enter a number with no more than ${decimals} decimal places`); const fraction = (match[3] ?? '').padEnd(decimals, '0'); const result = (match[1] === '-' ? -1 : 1) * (Number(match[2]) * (10 ** decimals) + Number(fraction || '0')); if (!Number.isSafeInteger(result)) throw new Error('The number is too large'); return result; }
export function findBarcodeItem(items: PosItem[], raw: string): PosItem | undefined { const needle = raw.trim().toLocaleLowerCase(); if (!needle) return undefined; return items.find(item => [item.barcode, item.sku, item.productCode].some(value => value?.toLocaleLowerCase() === needle)); }
function matchesItemSearch(item: PosItem, raw: string): boolean { const needle = raw.trim().toLocaleLowerCase(); return !needle || [item.name, item.barcode, item.sku, item.productCode].some(value => value?.toLocaleLowerCase().includes(needle)); }
const formatMoney = (minor: number) => new Intl.NumberFormat('en-ZA', { style: 'currency', currency: 'ZAR' }).format(minor / 100);
const formatQuantity = (micros: number) => new Intl.NumberFormat('en-ZA', { maximumFractionDigits: 6 }).format(micros / 1_000_000);
const formatInputMoney = (minor: number) => (minor / 100).toFixed(2);
const formatQuantityInput = (micros: number) => String(micros / 1_000_000);
const safeMoneyInput = (value?: string) => { try { return value?.trim() ? parseScaled(value, 2) : 0; } catch { return 0; } };
const safeQuantityInput = (value?: string) => { try { return value?.trim() ? parseScaled(value, 6) : 0; } catch { return 0; } };
const roundPositiveRatio = (numerator: number, denominator: number) => Math.floor((numerator + Math.floor(denominator / 2)) / denominator);
const formatBytes = (bytes: number) => bytes < 1_048_576 ? `${Math.max(1, Math.round(bytes / 1024))} KB` : `${(bytes / 1_048_576).toFixed(1)} MB`;
function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (value: boolean) => void }) { 
  return <label className="flex items-center gap-3 cursor-pointer select-none">
    <div className="relative">
      <input type="checkbox" checked={checked} onChange={event => onChange(event.target.checked)} className="sr-only" />
      <div className={`block w-14 h-8 rounded-full transition-colors ${checked ? 'bg-primary' : 'bg-surface-container-highest'}`}></div>
      <div className={`absolute left-1 top-1 bg-on-primary w-6 h-6 rounded-full transition-transform ${checked ? 'translate-x-6' : ''}`}></div>
    </div>
    <span className="font-body-md text-on-surface">{label}</span>
  </label>; 
}
type FieldProps = { label: string; name: keyof SetupFields; value: string; placeholder: string; type?: string; pattern?: string; onChange: (name: keyof SetupFields, value: string) => void };
function Field({ label, name, value, placeholder, type = 'text', pattern, onChange }: FieldProps) { 
  return <label className="w-full block mb-4" htmlFor={name}>
    <span className="block font-body-sm font-medium text-on-surface-variant mb-2">{label}</span>
    <input id={name} name={name} value={value} onChange={event => onChange(name, event.target.value)} placeholder={placeholder} type={type} pattern={pattern} required className="w-full h-12 px-4 rounded-xl bg-surface border border-outline-variant/30 text-on-surface focus:border-primary focus:ring-1 focus:ring-primary outline-none transition-all placeholder:text-on-surface-variant/50" />
  </label>; 
}
