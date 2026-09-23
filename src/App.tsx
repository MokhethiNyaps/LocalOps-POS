import { FormEvent, useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export const setupSteps = ['Business setup', 'Departments & locations', 'Products & services', 'Inventory', 'POS & payments'] as const;

type BootstrapTerminal = { id: string; name: string; departmentId: string | null };
type BootstrapBusiness = { id: string; name: string; terminals: BootstrapTerminal[] };
type AppBootstrap = { setupRequired: boolean; businesses: BootstrapBusiness[] };
type SessionView = { businessId: string; terminalId: string; departmentId: string | null; displayName: string };
type SetupResult = { businessId: string; departmentId: string; terminalId: string };
type Category = { id: string; name: string; parentId: string | null };
type Unit = { id: string; code: string; name: string; dimension: string; scaleNum: number; scaleDen: number; decimalPlaces: number };
type Packaging = { id: string; unitId: string; name: string; factorNum: number; factorDen: number; canPurchase: boolean; canSell: boolean };
type CatalogueItem = { id: string; kind: 'PRODUCT' | 'SERVICE'; name: string; categoryId: string | null; priceMinor: number; taxable: boolean; baseUnitId: string | null; costMinor: number | null; trackStock: boolean | null; durationMinutes: number | null; hasRecipe: boolean; packaging: Packaging[] };
type Department = { id: string; name: string };
type CatalogueSnapshot = { businessId: string; terminalId: string; terminalDepartmentId: string | null; categories: Category[]; units: Unit[]; items: CatalogueItem[]; departments: Department[] };
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
  return <main className="shell auth-shell">
    <BrandHeader />
    {startupError && <p className="notice error" role="alert">The local database could not be opened: {startupError}</p>}
    {mode === 'setup'
      ? <SetupView onComplete={(result, displayName) => enterWorkspace({ businessId: result.businessId, terminalId: result.terminalId, departmentId: result.departmentId, displayName })} />
      : <LoginView bootstrap={bootstrap} onLogin={enterWorkspace} />}
    <footer><span>Data stays on this device</span><span>SQLite secured</span><span>Automatic local backups</span></footer>
  </main>;
}

function BrandHeader() {
  return <header><div className="brand" aria-hidden="true">LO</div><div><p className="eyebrow">LOCALOPS POS</p><h1>Your business. One place. Always available.</h1></div><span className="offline">● Offline ready</span></header>;
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
  return <section className="hero"><div><p className="kicker">FIRST-RUN SETUP</p><h2>Build your local workspace</h2><p>Create the business, its first operation, and the owner account in one secure offline step.</p><form onSubmit={createBusiness}><div className="field-grid">
    <Field label="Business name" name="businessName" value={fields.businessName} onChange={updateField} placeholder="Moko’s Lifestyle Centre" />
    <Field label="Owner name" name="ownerDisplayName" value={fields.ownerDisplayName} onChange={updateField} placeholder="Moko" />
    <Field label="First department" name="departmentName" value={fields.departmentName} onChange={updateField} placeholder="Bar" />
    <Field label="Main stock location" name="locationName" value={fields.locationName} onChange={updateField} placeholder="Main Store" />
    <Field label="Terminal name" name="terminalName" value={fields.terminalName} onChange={updateField} placeholder="Main Till" />
    <Field label="Owner username" name="ownerUsername" value={fields.ownerUsername} onChange={updateField} placeholder="owner" />
    <Field label="Owner PIN" name="ownerPin" value={fields.ownerPin} onChange={updateField} placeholder="4–12 digits" type="password" pattern="[0-9]{4,12}" />
  </div><button type="submit" disabled={submitting}>{submitting ? 'Creating securely…' : 'Create business'} <span aria-hidden="true">→</span></button></form>{error && <p className="notice error" role="alert">Setup could not be completed: {error}</p>}</div><aside><strong>Setup journey</strong>{setupSteps.map((step, index) => <div className="step" key={step}><b>{index + 1}</b><span>{step}</span></div>)}</aside></section>;
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
  return <section className="login-card"><p className="kicker">WELCOME BACK</p><h2>Sign in to this terminal</h2><form onSubmit={submit}>
    <label className="field"><span>Business</span><select value={businessId} onChange={event => setBusinessId(event.target.value)} required>{bootstrap.businesses.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}</select></label>
    <label className="field"><span>Terminal</span><select value={terminalId} onChange={event => setTerminalId(event.target.value)} required>{business?.terminals.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}</select></label>
    <label className="field"><span>Username</span><input value={username} onChange={event => setUsername(event.target.value)} autoComplete="username" required /></label>
    <label className="field"><span>PIN</span><input value={pin} onChange={event => setPin(event.target.value)} type="password" inputMode="numeric" pattern="[0-9]{4,12}" autoComplete="current-password" required /></label>
    <button disabled={submitting || !terminalId}>{submitting ? 'Signing in…' : 'Sign in'} <span>→</span></button></form>{error && <p className="notice error" role="alert">{error}</p>}</section>;
}

function Workspace({ session, onLogout }: { session: SessionView; onLogout: () => void }) {
  const [snapshot, setSnapshot] = useState<CatalogueSnapshot | null>(null); const [error, setError] = useState<string | null>(null); const [notice, setNotice] = useState<string | null>(null); const [view, setView] = useState<'items' | 'configuration'>('items');
  const refresh = useCallback(async () => { try { setSnapshot(await invoke<CatalogueSnapshot>('get_catalogue_snapshot')); setError(null); } catch (reason) { setError(errorText(reason)); } }, []);
  useEffect(() => { void refresh(); }, [refresh]);
  async function act(action: () => Promise<unknown>, success: string) { setError(null); setNotice(null); try { await action(); setNotice(success); await refresh(); } catch (reason) { setError(errorText(reason)); } }
  async function logout() { try { await invoke('logout'); } finally { onLogout(); } }
  return <main className="workspace"><nav className="sidebar"><div className="brand large">LO</div><p className="eyebrow">LOCALOPS</p><button className={view === 'items' ? 'nav-item active' : 'nav-item'} onClick={() => setView('items')}>Catalogue</button><button className={view === 'configuration' ? 'nav-item active' : 'nav-item'} onClick={() => setView('configuration')}>Setup tools</button><div className="future-nav"><span>Inventory</span><span>Point of sale</span><span>Shifts</span><span>Reports</span></div><div className="user-card"><strong>{session.displayName}</strong><small>Local session</small><button className="link-button" onClick={logout}>Sign out</button></div></nav><section className="content"><header className="content-header"><div><p className="kicker">CATALOGUE</p><h2>{view === 'items' ? 'Products & services' : 'Catalogue setup'}</h2></div><span className="offline">● Working offline</span></header>{error && <p className="notice error" role="alert">{error}</p>}{notice && <p className="notice success" role="status">{notice}</p>}{!snapshot ? <div className="panel">Loading local catalogue…</div> : view === 'items' ? <ItemsView snapshot={snapshot} act={act} /> : <ConfigurationView snapshot={snapshot} act={act} />}</section></main>;
}

function ItemsView({ snapshot, act }: { snapshot: CatalogueSnapshot; act: Act }) {
  const [kind, setKind] = useState<'PRODUCT' | 'SERVICE'>('PRODUCT'); const [name, setName] = useState(''); const [price, setPrice] = useState(''); const [cost, setCost] = useState(''); const [categoryId, setCategoryId] = useState(''); const [unitId, setUnitId] = useState(snapshot.units[0]?.id ?? ''); const [duration, setDuration] = useState(''); const [taxable, setTaxable] = useState(true); const [trackStock, setTrackStock] = useState(true);
  useEffect(() => { if (!unitId && snapshot.units[0]) setUnitId(snapshot.units[0].id); }, [snapshot.units, unitId]);
  async function submit(event: FormEvent<HTMLFormElement>) { event.preventDefault(); await act(() => { const input = kind === 'PRODUCT' ? { name, priceMinor: parseScaled(price, 2), categoryId: categoryId || null, taxable, baseUnitId: unitId, costMinor: parseScaled(cost || '0', 2), trackStock, minimumQuantityMicros: 0 } : { name, priceMinor: parseScaled(price, 2), categoryId: categoryId || null, taxable, durationMinutes: duration ? Number(duration) : null }; return invoke(kind === 'PRODUCT' ? 'create_catalogue_product' : 'create_catalogue_service', { input }); }, `${name} added to the catalogue.`); setName(''); setPrice(''); setCost(''); setDuration(''); }
  return <><form className="panel compact-form" onSubmit={submit}><div className="panel-title"><div><h3>Add an item</h3><p>Start with the essentials. Packaging and recipes can be added afterward.</p></div><select value={kind} onChange={event => setKind(event.target.value as 'PRODUCT' | 'SERVICE')}><option value="PRODUCT">Product</option><option value="SERVICE">Service</option></select></div><div className="form-row">
    <label className="field"><span>Name</span><input value={name} onChange={event => setName(event.target.value)} required /></label><label className="field"><span>Price (ZAR)</span><input value={price} onChange={event => setPrice(event.target.value)} inputMode="decimal" placeholder="25.00" required /></label><label className="field"><span>Category</span><select value={categoryId} onChange={event => setCategoryId(event.target.value)}><option value="">No category</option>{snapshot.categories.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}</select></label>
    {kind === 'PRODUCT' ? <><label className="field"><span>Base unit</span><select value={unitId} onChange={event => setUnitId(event.target.value)} required><option value="">Choose unit</option>{snapshot.units.map(value => <option key={value.id} value={value.id}>{value.code} — {value.name}</option>)}</select></label><label className="field"><span>Cost (ZAR)</span><input value={cost} onChange={event => setCost(event.target.value)} inputMode="decimal" placeholder="0.00" /></label><Toggle label="Track stock" checked={trackStock} onChange={setTrackStock} /></> : <label className="field"><span>Duration (minutes)</span><input value={duration} onChange={event => setDuration(event.target.value)} type="number" min="1" /></label>}<Toggle label="Taxable" checked={taxable} onChange={setTaxable} /></div><button disabled={kind === 'PRODUCT' && !unitId}>Add {kind === 'PRODUCT' ? 'product' : 'service'}</button></form>
    <div className="stats"><article><strong>{snapshot.items.length}</strong><span>Active items</span></article><article><strong>{snapshot.items.filter(value => value.kind === 'PRODUCT').length}</strong><span>Products</span></article><article><strong>{snapshot.items.filter(value => value.kind === 'SERVICE').length}</strong><span>Services</span></article></div>
    <div className="panel table-panel"><table><thead><tr><th>Item</th><th>Type</th><th>Price</th><th>Stock / duration</th><th>Composition</th></tr></thead><tbody>{snapshot.items.map(item => <tr key={item.id}><td><strong>{item.name}</strong><small>{snapshot.categories.find(value => value.id === item.categoryId)?.name ?? 'Uncategorised'}</small></td><td><span className="pill">{item.kind.toLowerCase()}</span></td><td>{formatMoney(item.priceMinor)}</td><td>{item.kind === 'PRODUCT' ? (item.trackStock ? 'Tracked' : 'Not tracked') : (item.durationMinutes ? `${item.durationMinutes} min` : '—')}</td><td>{[item.packaging.length ? `${item.packaging.length} package${item.packaging.length === 1 ? '' : 's'}` : '', item.hasRecipe ? 'Recipe' : ''].filter(Boolean).join(' · ') || 'Standard'}</td></tr>)}</tbody></table>{snapshot.items.length === 0 && <p className="empty">No products or services yet.</p>}</div></>;
}

function ConfigurationView({ snapshot, act }: { snapshot: CatalogueSnapshot; act: Act }) { return <div className="config-grid"><CategoryForm snapshot={snapshot} act={act} /><UnitForm act={act} /><PackagingForm snapshot={snapshot} act={act} /><RecipeForm snapshot={snapshot} act={act} /><AvailabilityForm snapshot={snapshot} act={act} /></div>; }

function CategoryForm({ snapshot, act }: { snapshot: CatalogueSnapshot; act: Act }) { const [name, setName] = useState(''); const [parentId, setParentId] = useState(''); return <form className="panel" onSubmit={async event => { event.preventDefault(); await act(() => invoke('create_catalogue_category', { name, parentId: parentId || null }), `Category ${name} created.`); setName(''); }}><h3>Categories</h3><p>Group products and services for browsing and reporting.</p><label className="field"><span>Name</span><input value={name} onChange={event => setName(event.target.value)} required /></label><label className="field"><span>Parent category</span><select value={parentId} onChange={event => setParentId(event.target.value)}><option value="">Top level</option>{snapshot.categories.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}</select></label><button>Add category</button><div className="chips">{snapshot.categories.map(value => <span key={value.id}>{value.name}</span>)}</div></form>; }

function UnitForm({ act }: { act: Act }) {
  const [code, setCode] = useState(''); const [name, setName] = useState(''); const [dimension, setDimension] = useState('COUNT'); const [scaleNum, setScaleNum] = useState('1'); const [scaleDen, setScaleDen] = useState('1'); const [decimalPlaces, setDecimalPlaces] = useState('0');
  return <form className="panel" onSubmit={async event => { event.preventDefault(); await act(() => invoke('create_catalogue_unit', { input: { code, name, dimension, scaleNum: Number(scaleNum), scaleDen: Number(scaleDen), decimalPlaces: Number(decimalPlaces) } }), `Unit ${code} created.`); setCode(''); setName(''); }}><h3>Units</h3><p>Create exact reusable measurement units using a ratio to the dimension’s canonical unit.</p><div className="form-row two"><label className="field"><span>Code</span><input value={code} onChange={event => setCode(event.target.value.toUpperCase())} placeholder="KG" required /></label><label className="field"><span>Name</span><input value={name} onChange={event => setName(event.target.value)} placeholder="Kilogram" required /></label></div><label className="field"><span>Dimension</span><select value={dimension} onChange={event => setDimension(event.target.value)}><option>COUNT</option><option>WEIGHT</option><option>VOLUME</option><option>LENGTH</option><option>TIME</option></select></label><div className="form-row thirds"><label className="field"><span>Scale numerator</span><input value={scaleNum} onChange={event => setScaleNum(event.target.value)} type="number" min="1" required /></label><label className="field"><span>Scale denominator</span><input value={scaleDen} onChange={event => setScaleDen(event.target.value)} type="number" min="1" required /></label><label className="field"><span>Decimal places</span><input value={decimalPlaces} onChange={event => setDecimalPlaces(event.target.value)} type="number" min="0" max="6" required /></label></div><button>Add unit</button></form>;
}

function PackagingForm({ snapshot, act }: { snapshot: CatalogueSnapshot; act: Act }) {
  const products = snapshot.items.filter(value => value.kind === 'PRODUCT'); const [productId, setProductId] = useState(products[0]?.id ?? ''); const [unitId, setUnitId] = useState(snapshot.units[0]?.id ?? ''); const [name, setName] = useState(''); const [factorNum, setFactorNum] = useState(''); const [factorDen, setFactorDen] = useState('1');
  return <form className="panel" onSubmit={async event => { event.preventDefault(); await act(() => invoke('create_catalogue_packaging', { input: { productId, unitId, name, factorNum: Number(factorNum), factorDen: Number(factorDen), canPurchase: true, canSell: true } }), `${name} packaging added.`); setName(''); setFactorNum(''); setFactorDen('1'); }}><h3>Product packaging</h3><p>Define cases, crates, packs, or any exact product-specific conversion.</p><label className="field"><span>Product</span><select value={productId} onChange={event => setProductId(event.target.value)} required><option value="">Choose product</option>{products.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}</select></label><label className="field"><span>Package unit</span><select value={unitId} onChange={event => setUnitId(event.target.value)} required><option value="">Choose unit</option>{snapshot.units.map(value => <option key={value.id} value={value.id}>{value.code}</option>)}</select></label><label className="field"><span>Name</span><input value={name} onChange={event => setName(event.target.value)} placeholder="Crate of 12" required /></label><div className="form-row two"><label className="field"><span>Base-unit numerator</span><input value={factorNum} onChange={event => setFactorNum(event.target.value)} type="number" min="1" required /></label><label className="field"><span>Base-unit denominator</span><input value={factorDen} onChange={event => setFactorDen(event.target.value)} type="number" min="1" required /></label></div><button disabled={!products.length || !snapshot.units.length}>Add packaging</button></form>;
}

type RecipeRow = { ingredientId: string; quantity: string; unitId: string };
function RecipeForm({ snapshot, act }: { snapshot: CatalogueSnapshot; act: Act }) {
  const products = snapshot.items.filter(value => value.kind === 'PRODUCT');
  const blankRow = (): RecipeRow => ({ ingredientId: products[0]?.id ?? '', quantity: '', unitId: products[0]?.baseUnitId ?? '' });
  const [ownerId, setOwnerId] = useState(snapshot.items[0]?.id ?? ''); const [yieldQuantity, setYieldQuantity] = useState('1'); const [rows, setRows] = useState<RecipeRow[]>([blankRow()]);
  function updateRow(index: number, update: Partial<RecipeRow>) { setRows(current => current.map((row, rowIndex) => rowIndex === index ? { ...row, ...update } : row)); }
  function chooseIngredient(index: number, ingredientId: string) { updateRow(index, { ingredientId, unitId: products.find(value => value.id === ingredientId)?.baseUnitId ?? '' }); }
  return <form className="panel recipe-panel" onSubmit={async event => { event.preventDefault(); await act(() => invoke('replace_catalogue_recipe', { input: { ownerSellableId: ownerId, yieldQuantityMicros: parseScaled(yieldQuantity, 6), items: rows.map(row => ({ ingredientProductId: row.ingredientId, quantityMicros: parseScaled(row.quantity, 6), unitId: row.unitId })) } }), 'Recipe saved.'); setRows([blankRow()]); }}><h3>Recipe or consumable</h3><p>Consume one or more ingredient products when a product or service is sold.</p><div className="form-row two"><label className="field"><span>Sold item</span><select value={ownerId} onChange={event => setOwnerId(event.target.value)} required><option value="">Choose item</option>{snapshot.items.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}</select></label><label className="field"><span>Recipe yield</span><input value={yieldQuantity} onChange={event => setYieldQuantity(event.target.value)} inputMode="decimal" required /></label></div>{rows.map((row, index) => <div className="recipe-row" key={index}><label className="field"><span>Ingredient {index + 1}</span><select value={row.ingredientId} onChange={event => chooseIngredient(index, event.target.value)} required><option value="">Choose product</option>{products.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}</select></label><label className="field"><span>Quantity</span><input value={row.quantity} onChange={event => updateRow(index, { quantity: event.target.value })} inputMode="decimal" required /></label><label className="field"><span>Unit</span><select value={row.unitId} onChange={event => updateRow(index, { unitId: event.target.value })} required>{snapshot.units.map(value => <option key={value.id} value={value.id}>{value.code}</option>)}</select></label>{rows.length > 1 && <button className="icon-button" type="button" aria-label={`Remove ingredient ${index + 1}`} onClick={() => setRows(current => current.filter((_, rowIndex) => rowIndex !== index))}>×</button>}</div>)}<button className="secondary-button" type="button" disabled={!products.length} onClick={() => setRows(current => [...current, blankRow()])}>+ Add ingredient</button><button disabled={!snapshot.items.length || !products.length}>Save recipe</button></form>;
}

function AvailabilityForm({ snapshot, act }: { snapshot: CatalogueSnapshot; act: Act }) { const [departmentId, setDepartmentId] = useState(snapshot.terminalDepartmentId ?? snapshot.departments[0]?.id ?? ''); const [sellableId, setSellableId] = useState(snapshot.items[0]?.id ?? ''); const [override, setOverride] = useState(''); return <form className="panel" onSubmit={async event => { event.preventDefault(); await act(() => invoke('set_catalogue_availability', { input: { departmentId, sellableId, priceOverrideMinor: override ? parseScaled(override, 2) : null, active: true } }), 'Department availability updated.'); }}><h3>Department availability</h3><p>Choose where an item can be sold and optionally override its price.</p><label className="field"><span>Department</span><select value={departmentId} onChange={event => setDepartmentId(event.target.value)} required>{snapshot.departments.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}</select></label><label className="field"><span>Item</span><select value={sellableId} onChange={event => setSellableId(event.target.value)} required><option value="">Choose item</option>{snapshot.items.map(value => <option key={value.id} value={value.id}>{value.name}</option>)}</select></label><label className="field"><span>Price override (optional)</span><input value={override} onChange={event => setOverride(event.target.value)} inputMode="decimal" /></label><button disabled={!snapshot.departments.length || !snapshot.items.length}>Make available</button></form>; }

function parseScaled(raw: string, decimals: number): number { const match = /^([+-]?)(\d+)(?:\.(\d+))?$/.exec(raw.trim()); if (!match || (match[3]?.length ?? 0) > decimals) throw new Error(`Enter a number with no more than ${decimals} decimal places`); const fraction = (match[3] ?? '').padEnd(decimals, '0'); const result = (match[1] === '-' ? -1 : 1) * (Number(match[2]) * (10 ** decimals) + Number(fraction || '0')); if (!Number.isSafeInteger(result)) throw new Error('The number is too large'); return result; }
const formatMoney = (minor: number) => new Intl.NumberFormat('en-ZA', { style: 'currency', currency: 'ZAR' }).format(minor / 100);
function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (value: boolean) => void }) { return <label className="toggle"><input type="checkbox" checked={checked} onChange={event => onChange(event.target.checked)} /><span>{label}</span></label>; }
type FieldProps = { label: string; name: keyof SetupFields; value: string; placeholder: string; type?: string; pattern?: string; onChange: (name: keyof SetupFields, value: string) => void };
function Field({ label, name, value, placeholder, type = 'text', pattern, onChange }: FieldProps) { return <label className="field" htmlFor={name}><span>{label}</span><input id={name} name={name} value={value} onChange={event => onChange(name, event.target.value)} placeholder={placeholder} type={type} pattern={pattern} required /></label>; }
