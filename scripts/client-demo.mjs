import { Builder, By, Key, until } from 'selenium-webdriver';
import { spawn, execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { randomInt } from 'node:crypto';
import path from 'node:path';
import assert from 'node:assert/strict';

const root = process.cwd();
const artifacts = path.join(root, 'target/client-demo');
const dataDir = path.join(process.env.LOCALAPPDATA, 'LocalOps/POS');
const database = path.join(dataDir, 'business.db');
const businessName = "Thabo's Lifestyle Centre";
await mkdir(artifacts, { recursive: true });
const query = sql => JSON.parse(execFileSync('sqlite3', ['-readonly', '-json', database, sql], { encoding: 'utf8' }) || '[]');
const quote = value => `'${value.replaceAll("'", "''")}'`;
const existing = query('SELECT name FROM businesses');
if (existing.length && (existing.length !== 1 || existing[0].name !== businessName)) throw new Error('A different business already exists. Demo creation stopped without changing it.');
let credentials;
try { credentials = JSON.parse(await readFile(path.join(artifacts, 'credentials.json'), 'utf8')); }
catch {
  if (existing.length) throw new Error('Existing demo credentials are unavailable. No data was changed.');
  credentials = { owner: { username: 'demo-owner', pin: String(randomInt(100000, 999999)) }, cashier: { username: 'demo-cashier', pin: String(randomInt(100000, 999999)) } };
  await writeFile(path.join(artifacts, 'credentials.json'), JSON.stringify(credentials, null, 2));
  await writeFile(path.join(artifacts, 'Demo Login.txt'), `Owner username: ${credentials.owner.username}\nOwner PIN: ${credentials.owner.pin}\n\nCashier username: ${credentials.cashier.username}\nCashier PIN: ${credentials.cashier.pin}\n`);
}
let completed;
try { completed = JSON.parse(await readFile(path.join(artifacts, 'progress.json'), 'utf8')); } catch { completed = []; }
const server = spawn(path.join(root, 'target/webdriver-tools/bin/tauri-driver.exe'), [
  '--port', '4447', '--native-port', '4448', '--native-driver', path.join(root, 'target/webdriver-tools/edge/msedgedriver.exe'),
], { windowsHide: true, env: { ...process.env, LOCALOPS_E2E_DATA_DIR: dataDir } });
let driver;
let log = '';
server.stdout.on('data', value => { log += value; }); server.stderr.on('data', value => { log += value; });
const xpathQuote = text => !text.includes("'") ? `'${text}'` : `concat(${text.split("'").map(part => `'${part}'`).join(', "\'", ')})`;
const form = title => `//form[.//h3[normalize-space()=${xpathQuote(title)}]]`;
const find = xpath => driver.wait(until.elementLocated(By.xpath(xpath)), 20000);
async function tap(xpath) {
  const element = await find(xpath); await driver.wait(until.elementIsEnabled(element), 15000);
  await driver.executeScript('arguments[0].scrollIntoView({block:"center",inline:"nearest"})', element); await element.click();
}
const click = (text, scope = '') => tap(`${scope}//button[text()[normalize-space()=${xpathQuote(text)}] or normalize-space()=${xpathQuote(text)} or .//span[normalize-space()=${xpathQuote(text)}] or starts-with(@title,${xpathQuote(text)})]`);
const field = (label, scope = '') => find(`${scope}//label[span[normalize-space()=${xpathQuote(label)} or text()[normalize-space()=${xpathQuote(label)}]]]/*[self::input or self::select]`);
async function fill(label, value, scope = '') {
  const element = await field(label, scope); await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', element);
  await element.sendKeys(Key.CONTROL, 'a', Key.NULL, Key.BACK_SPACE); await element.sendKeys(String(value));
}
async function choose(label, text, scope = '') {
  const select = await field(label, scope); await driver.executeScript('arguments[0].scrollIntoView({block:"center"})', select);
  await select.findElement(By.xpath(`.//option[contains(normalize-space(),${xpathQuote(text)})]`)).click();
}
const body = () => driver.findElement(By.css('body')).getText().catch(() => '');
const see = text => driver.wait(async () => (await body()).includes(text), 20000, `Missing screen text: ${text}`);
async function success(text) {
  await driver.wait(async () => {
    for (const alert of await driver.findElements(By.css('[role="alert"]'))) if (await alert.isDisplayed()) throw new Error(await alert.getText());
    return (await body()).includes(text);
  }, 20000, `Missing success: ${text}`);
}
async function record(name) {
  if (!completed.includes(name)) completed.push(name);
  await writeFile(path.join(artifacts, 'progress.json'), JSON.stringify(completed, null, 2)); console.log(`DONE ${name}`);
}
async function login(account, terminal) {
  await see('Sign in to this terminal'); await choose('Terminal', terminal); await fill('Username', account.username); await fill('PIN', account.pin); await click('Sign in'); await see('Point of Sale');
}
async function snapshot(name) { await writeFile(path.join(artifacts, `${name}.png`), await driver.takeScreenshot(), 'base64'); }

const products = [
  ['Castle Lager 330ml', 'Beer', 25, 15, 48, 12],
  ['Heineken 330ml', 'Beer', 32, 19, 4, 6],
  ['Savanna Dry 330ml', 'Cider', 30, 18, 12, 6],
  ['Coca-Cola 300ml', 'Soft Drinks', 17, 9, 48, 12],
  ['Bonaqua Still Water 500ml', 'Soft Drinks', 12, 6, 60, 12],
  ['Red Bull 250ml', 'Soft Drinks', 35, 20, 24, 6],
  ['Jameson Single 25ml', 'Spirits & Mixers', 30, 18, 50, 10],
  ['Schweppes Tonic 200ml', 'Spirits & Mixers', 15, 7, 48, 12],
  ['Beef Burger & Chips', 'Meals', 85, 45, null, 0],
  ['Quarter Chicken Meal', 'Meals', 75, 38, null, 0],
  ['Steak, Pap & Chakalaka', 'Meals', 120, 65, null, 0],
  ['Full Breakfast Plate', 'Meals', 65, 32, null, 0],
  ['Large Chips', 'Snacks', 31, 12, null, 0],
  ['Chicken Mayo Toastie', 'Snacks', 45, 20, null, 0],
];
const services = [['Basic Wash', 70, 20], ['Wash & Vacuum', 95, 35], ['Full Valet', 350, 120], ['SUV Wash', 100, 30], ['Engine Clean', 120, 45], ['Interior Cleaning', 180, 60]];
const categories = ['Beer', 'Cider', 'Soft Drinks', 'Spirits & Mixers', 'Meals', 'Snacks', 'Wash Services'];

try {
  for (let attempt = 0; attempt < 100; attempt++) {
    try { if ((await fetch('http://127.0.0.1:4447/status')).ok) break; } catch {}
    await new Promise(resolve => setTimeout(resolve, 200));
  }
  driver = await new Builder().usingServer('http://127.0.0.1:4447').withCapabilities({ browserName: 'wry', 'tauri:options': { application: path.join(root, 'target/ui-test-build/app.exe') } }).build();
  if (!existing.length) {
    await see('Build workspace');
    for (const [label, value] of Object.entries({ 'Business name': businessName, 'First department': 'Bar', 'Main stock location': 'Main Store', 'Terminal name': 'Main Till', 'Owner username': credentials.owner.username, 'Owner display name': 'Thabo - Owner', 'Owner PIN': credentials.owner.pin })) await fill(label, value);
    await click('Create business'); await see('Point of Sale'); await record('Business and Owner');
  } else await login(credentials.owner, 'Main Till');
  await click('Setup tools');
  for (const [name, terminal] of [['Food', 'Kitchen Till'], ['Car Wash', 'Wash Bay Till']]) {
    if (query(`SELECT id FROM departments WHERE name=${quote(name)}`).length) continue;
    await fill('Department name', name, form('Departments & tills')); await fill('Terminal name', terminal, form('Departments & tills')); await choose('Stock location', 'Main Store', form('Departments & tills')); await click('Add department & till', form('Departments & tills')); await success(`Department ${name} and terminal ${terminal} created.`); await record(`Department ${name}`);
  }
  for (const name of categories) {
    if (query(`SELECT id FROM categories WHERE name=${quote(name)}`).length) continue;
    await fill('Name', name, form('Categories')); await click('Add category', form('Categories')); await success(`Category ${name} created.`); await record(`Category ${name}`);
  }
  for (const [code, name, dimension] of [['EA', 'Each', 'COUNT'], ['ML', 'Millilitre', 'VOLUME']]) {
    if (query(`SELECT id FROM units WHERE code=${quote(code)}`).length) continue;
    await fill('Code', code, form('Units')); await fill('Name', name, form('Units')); await choose('Dimension', dimension, form('Units')); await click('Add unit', form('Units')); await success(`Unit ${code} created.`); await record(`Unit ${code}`);
  }
  await click('Catalogue'); const itemForm = form('Add an item');
  for (const [index, [name, category, price, cost, quantity, minimum]] of products.entries()) {
    if (query(`SELECT id FROM sellable_items WHERE name=${quote(name)}`).length) continue;
    await fill('Name', name, itemForm); await fill('Price (ZAR)', price.toFixed(2), itemForm); await fill('Cost (ZAR)', cost.toFixed(2), itemForm); await fill('Minimum stock', minimum, itemForm);
    await choose('Category', category, itemForm); await choose('Base unit', 'Each', itemForm); await fill('Barcode (optional)', `600100000${String(index + 1).padStart(4, '0')}`, itemForm); await fill('SKU (optional)', `DEMO-${String(index + 1).padStart(3, '0')}`, itemForm);
    const toggle = await find(`${itemForm}//label[span[normalize-space()='Track stock']]//input[@type='checkbox']`);
    if ((await toggle.isSelected()) !== (quantity !== null)) await tap(`${itemForm}//label[span[normalize-space()='Track stock']]`);
    await click('Add product', itemForm); await success(`${name} added to the catalogue.`); await record(name);
  }
  await (await find(`${itemForm}//select[option[@value='SERVICE']]`)).findElement(By.css('option[value="SERVICE"]')).click();
  for (const [name, price, duration] of services) {
    if (query(`SELECT id FROM sellable_items WHERE name=${quote(name)}`).length) continue;
    await fill('Name', name, itemForm); await fill('Price (ZAR)', price.toFixed(2), itemForm); await choose('Category', 'Wash Services', itemForm); await fill('Duration (minutes)', duration, itemForm); await click('Add service', itemForm); await success(`${name} added to the catalogue.`); await record(name);
  }
  await snapshot('catalogue'); await click('Setup tools');
  const sharedDrinks = ['Coca-Cola 300ml', 'Bonaqua Still Water 500ml'];
  const availability = { Bar: products.slice(0, 8).map(row => row[0]), Food: [...products.slice(8).map(row => row[0]), ...sharedDrinks], 'Car Wash': [...services.map(row => row[0]), ...sharedDrinks] };
  for (const [department, names] of Object.entries(availability)) for (const name of names) {
    if (query(`SELECT 1 FROM department_sellables ds JOIN departments d ON d.id=ds.department_id JOIN sellable_items s ON s.id=ds.sellable_id WHERE d.name=${quote(department)} AND s.name=${quote(name)}`).length) continue;
    await choose('Department', department, form('Department availability')); await choose('Item', name, form('Department availability')); await click('Make available', form('Department availability')); await success('Department availability updated.'); await record(`${department}: ${name}`);
  }
  if (!query('SELECT id FROM product_packaging').length) {
    await choose('Product', 'Castle Lager 330ml', form('Product packaging')); await choose('Package unit', 'EA', form('Product packaging')); await fill('Name', 'Crate of 12', form('Product packaging')); await fill('Base-unit num', '12', form('Product packaging')); await click('Add packaging', form('Product packaging')); await success('Crate of 12 packaging added.');
  }
  await click('Inventory');
  if (!query('SELECT id FROM suppliers').length) {
    await fill('Name', 'Demo Beverage Distributors', form('Add supplier')); await fill('Phone (optional)', '011 555 0100', form('Add supplier')); await click('Add supplier', form('Add supplier')); await success('Demo Beverage Distributors added as a supplier.');
  }
  for (const [index, [name, , , cost, quantity]] of products.slice(0, 8).entries()) {
    const number = `DEMO-PUR-${String(index + 1).padStart(3, '0')}`;
    if (query(`SELECT id FROM purchases WHERE purchase_number=${quote(number)}`).length) continue;
    await fill('Purchase number', number, form('Receive purchase')); await choose('Supplier', 'Demo Beverage Distributors', form('Receive purchase')); await choose('Product', name, form('Receive purchase')); await fill('Quantity', quantity, form('Receive purchase')); await fill('Unit cost (ZAR)', cost.toFixed(2), form('Receive purchase')); await choose('Payment status', 'Paid', form('Receive purchase')); await fill('Invoice reference', `INV-${number}`, form('Receive purchase')); await click('Receive stock', form('Receive purchase')); await success(`Purchase ${number} received.`); await record(number);
  }
  if (!completed.includes('Out-of-stock example')) {
    await choose('Product', 'Savanna Dry 330ml', form('Waste or damage')); await fill('Quantity', '12', form('Waste or damage')); await fill('Reason', 'Expired display stock - demo example', form('Waste or damage')); await click('Record loss', form('Waste or damage')); await success('Stock loss recorded.'); await record('Out-of-stock example');
  }
  await snapshot('inventory'); await click('Employees');
  if (!query(`SELECT id FROM users WHERE username=${quote(credentials.cashier.username)}`).length) {
    await fill('Username', credentials.cashier.username); await fill('Display name', 'Thembi - Cashier'); await fill('PIN', credentials.cashier.pin); await choose('Role', 'Cashier'); await click('Add employee'); await success('Thembi - Cashier can now sign in on this terminal.');
  }
  await snapshot('employees'); await click('Sign out');
  const showcaseSales = [
    { title: 'Bar receipt', terminal: 'Main Till', opening: '200.00', total: 14200, cash: true, tender: '150.00', closing: '342.00', items: [['Castle Lager 330ml', 2], ['Coca-Cola 300ml', 1], ['Jameson Single 25ml', 2], ['Schweppes Tonic 200ml', 1]] },
    { title: 'Food receipt', terminal: 'Kitchen Till', opening: '100.00', total: 13300, cash: false, items: [['Beef Burger & Chips', 1], ['Large Chips', 1], ['Coca-Cola 300ml', 1]] },
    { title: 'Car Wash receipt', terminal: 'Wash Bay Till', opening: '150.00', total: 9500, cash: true, tender: '100.00', closing: '245.00', items: [['Wash & Vacuum', 1]] },
  ];
  for (const sale of showcaseSales) {
    if (completed.includes(sale.title)) continue;
    await login(credentials.cashier, sale.terminal); await click('My Shift');
    if ((await body()).includes('Open your shift')) { await fill('Opening cash', sale.opening); await click('Open shift'); await success('Shift opened.'); }
    await click('Point of Sale');
    for (const [name, quantity] of sale.items) for (let i = 0; i < quantity; i++) await click(name);
    if (sale.cash) { await click('Exact cash'); await fill('Cash tendered', sale.tender); }
    else { await click('Split payment / exact tender'); await fill('Card', (sale.total / 100).toFixed(2)); }
    await tap("//button[starts-with(normalize-space(),'Pay R')]"); await success('Receipt ready.');
    await click('Recent Sales'); await see('S-'); await tap("//button[.//span[starts-with(normalize-space(),'S-')]]"); await snapshot(sale.title.replaceAll(' ', '-'));
    const stored = query(`SELECT total_minor FROM sales JOIN terminals ON terminals.id=sales.terminal_id WHERE terminals.name=${quote(sale.terminal)} ORDER BY completed_at DESC LIMIT 1`)[0]; assert.equal(stored.total_minor, sale.total);
    if (sale.closing) { await click('My Shift'); await fill('Counted cash', sale.closing); await click('Close my shift'); await success('Shift closed.'); }
    await record(sale.title); await click('Sign out');
  }
  await login(credentials.owner, 'Main Till'); await click('Shift & expenses');
  await see('Shifts & expenses');
  await driver.wait(async () => { const text = await body(); return text.includes('Start a new immutable cashier period') || text.includes('Expected cash now'); }, 20000);
  if ((await body()).includes('Start a new immutable cashier period')) { await fill('Opening cash', '500.00', form('Open shift')); await click('Open shift', form('Open shift')); await success('Shift opened and ready for cash activity.'); }
  if (!query('SELECT id FROM expense_categories').length) { await fill('Name', 'Consumables', form('Expense category')); await click('Create category', form('Expense category')); await success('Expense category Consumables created.'); }
  if (!query('SELECT id FROM expenses').length) {
    await choose('Category', 'Consumables', form('Record expense')); await choose('Payment method', 'Cash', form('Record expense')); await fill('Amount', '20.00', form('Record expense')); await fill('Description', 'Cleaning cloths and till supplies', form('Record expense')); await fill('Reference', 'DEMO-EXP-001', form('Record expense')); await click('Record expense', form('Record expense')); await success('Expense recorded and cash expectation updated when applicable.');
  }
  await snapshot('shifts-and-expenses'); await click('Reports'); await see('Business dashboard'); await snapshot('reports');
  await click('Backup & safety'); await click('Create manual backup'); await success('Verified backup created:'); await snapshot('safety');
  await click('Point of Sale'); await see('Castle Lager 330ml'); await driver.executeScript('window.scrollTo(0,0)'); await snapshot('point-of-sale');
  assert.equal(query('SELECT count(*) AS n FROM sellable_items')[0].n, 20);
  assert.equal(query('SELECT count(*) AS n FROM departments')[0].n, 3);
  assert.equal(query('SELECT count(*) AS n FROM terminals')[0].n, 3);
  assert.equal(query('SELECT sum(total_minor) AS total FROM sales')[0].total, 37000);
  assert.equal(query('PRAGMA quick_check')[0].quick_check, 'ok'); assert.equal(query('PRAGMA foreign_key_check').length, 0);
  await writeFile(path.join(artifacts, 'summary.json'), JSON.stringify({ businessName, dataDir, products: 14, services: 6, departments: 3, terminals: 3, sales: 3, salesTotal: 370, entryMethod: 'Selenium typing and clicks in the real Tauri UI' }, null, 2));
  console.log('Client demo complete: 20 items, 3 departments, 3 tills and R370.00 in sales.');
} catch (error) {
  console.error(error.stack); process.exitCode = 1;
  if (driver) { await snapshot('failure').catch(() => {}); await writeFile(path.join(artifacts, 'failure.txt'), await body()); }
} finally {
  if (driver) await driver.quit().catch(() => {}); server.kill(); await writeFile(path.join(artifacts, 'webdriver.log'), log);
}
