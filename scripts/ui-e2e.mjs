import { Builder, By, Key, until } from 'selenium-webdriver';
import { spawn, execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import { randomInt } from 'node:crypto';

const root = process.cwd();
const runDir = path.join(root, 'target', 'ui-e2e', new Date().toISOString().replace(/[:.]/g, '-'));
const dataDir = path.join(runDir, 'data');
await mkdir(dataDir, { recursive: true });
const ownerPin = String(randomInt(100000, 999999));
let cashierPin = String(randomInt(100000, 999999));
const results = [];
let driver;
const server = spawn(path.join(root, 'target/webdriver-tools/bin/tauri-driver.exe'), [
  '--port', '4445', '--native-port', '4446', '--native-driver',
  path.join(root, 'target/webdriver-tools/edge/msedgedriver.exe'),
], { windowsHide: true, env: { ...process.env, LOCALOPS_E2E_DATA_DIR: dataDir } });
let serverLog = '';
server.stdout.on('data', value => { serverLog += value; });
server.stderr.on('data', value => { serverLog += value; });
server.on('error', error => { serverLog += error.message; });

const quoted = text => {
  if (!text.includes("'")) return `'${text}'`;
  return `concat(${text.split("'").map(part => `'${part}'`).join(', "\'", ')})`;
};
const form = title => `//form[.//h3[normalize-space()=${quoted(title)}]]`;
async function element(xpath) {
  return driver.wait(until.elementLocated(By.xpath(xpath)), 15000);
}
async function field(label, scope = '') {
  return element(`${scope}//label[span[normalize-space()=${quoted(label)} or text()[normalize-space()=${quoted(label)}]]]/*[self::input or self::select or self::textarea]`);
}
async function fill(label, value, scope = '') {
  const input = await field(label, scope);
  await input.sendKeys(Key.CONTROL, 'a', Key.NULL, Key.BACK_SPACE);
  await input.sendKeys(value);
}
async function choose(label, value, scope = '') {
  const select = await field(label, scope);
  await select.findElement(By.xpath(`.//option[contains(normalize-space(), ${quoted(value)})]`)).click();
}
async function click(text, scope = '') {
  await tap(`${scope}//button[text()[normalize-space()=${quoted(text)}] or normalize-space()=${quoted(text)} or .//span[normalize-space()=${quoted(text)}] or starts-with(@title, ${quoted(text)})]`);
}
async function tap(xpath) {
  const button = await element(xpath);
  await driver.wait(until.elementIsEnabled(button), 15000);
  await driver.executeScript('arguments[0].scrollIntoView({block: "center", inline: "nearest"})', button);
  await button.click();
}
async function body() {
  try { return await driver.findElement(By.css('body')).getText(); }
  catch (error) { if (error.name === 'StaleElementReferenceError') return ''; throw error; }
}
async function see(text) { await driver.wait(async () => (await body()).includes(text), 15000, `Missing UI text: ${text}`); }
async function status(text) {
  await driver.wait(async () => {
    const alerts = await driver.findElements(By.css('[role="alert"]'));
    for (const alert of alerts) { if (await alert.isDisplayed()) throw new Error(await alert.getText()); }
    return (await body()).includes(text);
  }, 15000, `Missing success message: ${text}`);
}
async function step(name, action) {
  console.log(`TEST ${name}`);
  try {
    await action();
    await writeFile(path.join(runDir, `${results.length + 1}.png`), await driver.takeScreenshot(), 'base64');
    results.push({ name, status: 'PASS' });
    console.log(`PASS ${name}`);
  } catch (error) {
    results.push({ name, status: 'FAIL', error: error.message });
    throw error;
  }
}
async function login(username, pin) {
  await see('Sign in to this terminal');
  await driver.wait(async () => (await driver.findElements(By.css('form input'))).length === 2, 15000);
  await choose('Terminal', 'UI Till');
  await fill('Username', username); await fill('PIN', pin); await click('Sign in');
  await see('Point of Sale');
}
function query(sql) {
  return JSON.parse(execFileSync('sqlite3', ['-readonly', '-json', path.join(dataDir, 'business.db'), sql], { encoding: 'utf8' }) || '[]');
}

try {
  for (let attempt = 0; attempt < 100; attempt++) {
    try { if ((await fetch('http://127.0.0.1:4445/status')).ok) break; } catch {}
    await new Promise(resolve => setTimeout(resolve, 200));
  }
  const capabilities = { browserName: 'wry', 'tauri:options': { application: path.join(root, 'target/ui-test-build/app.exe') } };
  const connect = () => new Builder().usingServer('http://127.0.0.1:4445').withCapabilities(capabilities).build();
  driver = await connect();
  await driver.manage().setTimeouts({ implicit: 0, pageLoad: 30000, script: 10000 });

  await step('First-run business and owner created through setup form', async () => {
    await see('Build workspace');
    for (const [label, value] of Object.entries({ 'Business name': 'LocalOps UI Verification', 'Terminal name': 'UI Till', 'First department': 'Bar', 'Main stock location': 'Main Store', 'Owner username': 'ui-owner', 'Owner display name': 'UI Owner', 'Owner PIN': ownerPin })) await fill(label, value);
    await click('Create business'); await see('UI Owner');
  });
  await step('Category and measurement unit created through setup tools', async () => {
    await click('Setup tools');
    await fill('Name', 'Drinks', form('Categories')); await click('Add category', form('Categories')); await status('Category Drinks created.');
    await fill('Code', 'EA', form('Units')); await fill('Name', 'Each', form('Units')); await click('Add unit', form('Units')); await status('Unit EA created.');
  });
  await step('Owner creates Food and Car Wash departments and their tills through setup tools', async () => {
    for (const [name, terminal] of [['Food', 'Kitchen Till'], ['Car Wash', 'Wash Bay Till']]) {
      await fill('Department name', name, form('Departments & tills')); await fill('Terminal name', terminal, form('Departments & tills')); await choose('Stock location', 'Main Store', form('Departments & tills')); await click('Add department & till', form('Departments & tills')); await status(`Department ${name} and terminal ${terminal} created.`);
    }
    assert.equal(query('SELECT count(*) AS n FROM departments')[0].n, 3);
    assert.equal(query('SELECT count(*) AS n FROM terminals')[0].n, 3);
    assert.equal(query('SELECT count(*) AS n FROM department_locations WHERE is_default=1')[0].n, 3);
  });
  await step('Required catalogue fields prevent empty submissions', async () => {
    await click('Catalogue');
    const submit = await element(`${form('Add an item')}//button[normalize-space()='Add product']`);
    await submit.click();
    assert.equal(await driver.executeScript('return document.querySelector("#workspace-content form").checkValidity()'), false);
    assert.equal(query('SELECT count(*) AS n FROM sellable_items')[0].n, 0);
  });
  await step('Stock product and service created through catalogue form', async () => {
    await click('Catalogue');
    const itemForm = form('Add an item');
    await fill('Name', 'Castle Lager 330ml', itemForm); await fill('Price (ZAR)', '30.00', itemForm); await fill('Cost (ZAR)', '18.00', itemForm);
    await choose('Category', 'Drinks', itemForm); await choose('Base unit', 'Each', itemForm);
    await click('Add product', itemForm); await status('Castle Lager 330ml added to the catalogue.');
    await (await element(`${itemForm}//select[option[@value='SERVICE']]`)).findElement(By.css('option[value="SERVICE"]')).click();
    await fill('Name', 'Basic Wash', itemForm); await fill('Price (ZAR)', '70.00', itemForm); await fill('Duration (minutes)', '20', itemForm);
    await click('Add service', itemForm); await status('Basic Wash added to the catalogue.');
  });
  await step('Department availability entered for both items', async () => {
    await click('Setup tools');
    for (const name of ['Castle Lager 330ml', 'Basic Wash']) {
      await choose('Item', name, form('Department availability')); await click('Make available', form('Department availability')); await status('Department availability updated.');
      assert.equal(query('SELECT count(*) AS n FROM department_sellables')[0].n, name === 'Basic Wash' ? 2 : 1);
    }
  });
  await step('Case packaging is created through setup tools', async () => {
    const packageForm = form('Product packaging');
    await choose('Product', 'Castle Lager 330ml', packageForm); await choose('Package unit', 'EA', packageForm);
    await fill('Name', 'Case of 12', packageForm); await fill('Base-unit num', '12', packageForm); await click('Add packaging', packageForm); await status('Case of 12 packaging added.');
    assert.equal(query('SELECT factor_num FROM product_packaging')[0].factor_num, 12);
  });
  await step('Supplier and stock purchase entered through inventory forms', async () => {
    await click('Inventory');
    await fill('Name', 'UI Beverage Supplier', form('Add supplier')); await click('Add supplier', form('Add supplier')); await status('UI Beverage Supplier added as a supplier.');
    const receive = form('Receive purchase');
    await fill('Purchase number', 'UI-PUR-001', receive); await choose('Supplier', 'UI Beverage Supplier', receive);
    await choose('Product', 'Castle Lager 330ml', receive); await fill('Quantity', '24', receive); await fill('Unit cost (ZAR)', '18.00', receive);
    await click('Receive stock', receive); await status('Purchase UI-PUR-001 received.');
    assert.equal(query('SELECT quantity_micros FROM inventory_balances')[0].quantity_micros, 24000000);
  });
  await step('Cashier employee created through employee form', async () => {
    await click('Employees'); await fill('Username', 'ui-cashier'); await fill('Display name', 'UI Cashier'); await fill('PIN', cashierPin);
    await choose('Role', 'Cashier'); await click('Add employee'); await status('UI Cashier can now sign in on this terminal.');
  });
  await step('Duplicate employee is rejected without creating a second account', async () => {
    await fill('Username', 'ui-cashier'); await fill('Display name', 'Duplicate'); await fill('PIN', cashierPin); await click('Add employee');
    await driver.wait(until.elementLocated(By.css('[role="alert"]')), 15000);
    assert.equal(query('SELECT count(*) AS n FROM users')[0].n, 2);
    await click('Catalogue');
  });
  await step('Invalid login is rejected by the real authentication backend', async () => {
    await click('Sign out'); await see('Sign in to this terminal'); await fill('Username', 'ui-cashier'); await fill('PIN', '000000'); await click('Sign in');
    await driver.wait(until.elementLocated(By.css('[role="alert"]')), 15000); await see('Sign in to this terminal');
  });
  await step('Cashier login exposes only the permitted navigation', async () => {
    await login('ui-cashier', cashierPin); await see('UI Cashier');
    const titles = await driver.executeScript('return Array.from(document.querySelectorAll("nav button")).map(button => button.title)');
    assert.equal(titles.length, 3);
    assert(titles.some(title => title.startsWith('My Shift')));
    assert(!titles.some(title => title.startsWith('Employees') || title.startsWith('Inventory')));
  });
  await step('Cashier opens shift through My Shift', async () => {
    await click('My Shift'); await fill('Opening cash', '100.00'); await click('Open shift'); await status('Shift opened.');
  });
  await step('Cashier sells product and service with cash and change', async () => {
    await click('Point of Sale');
    await click('Castle Lager 330ml'); await click('Basic Wash'); await click('Exact cash');
    await fill('Cash tendered', '150.00');
    await tap("//button[starts-with(normalize-space(), 'Pay R')]");
    await status('Receipt ready.'); await see('Change');
    const sale = query('SELECT total_minor,change_due_minor FROM sales')[0];
    assert.equal(sale.total_minor, 10000); assert.equal(sale.change_due_minor, 5000);
    assert.equal(query('SELECT quantity_micros FROM inventory_balances')[0].quantity_micros, 23000000);
  });
  await step('Cashier recent sales and receipt reopen through UI', async () => {
    await click('Recent Sales'); await see('S-');
    await tap("//button[.//span[starts-with(normalize-space(), 'S-')]]"); await see('Castle Lager 330ml');
  });
  await step('Cashier closes shift without expected cash or variance fields', async () => {
    await click('My Shift');
    assert.equal((await driver.findElements(By.xpath("//label[span[normalize-space()='Expected cash' or normalize-space()='Variance']]"))).length, 0);
    await fill('Counted cash', '200.00'); await click('Close my shift'); await status('Shift closed.');
    assert.equal(query("SELECT status FROM shifts")[0].status, 'CLOSED');
  });
  await step('Closed shift sales disappear from Cashier recent sales', async () => {
    await click('Recent Sales'); await see('No sales in your current shift yet.'); assert(!(await body()).includes('S-'));
  });
  await step('Owner reports and backup run through UI', async () => {
    await click('Sign out'); await login('ui-owner', ownerPin); await click('Reports'); await see('Business dashboard');
    const gross = await element("//span[span[normalize-space()='Gross sales']]/strong");
    assert.match(await gross.getText(), /100[,.]00/);
    await click('Backup & safety'); await see('SQLITE INTEGRITY'); await click('Create manual backup'); await status('Verified backup created:');
  });
  await step('Real SQLite integrity, stock ledger and role persistence verified', async () => {
    assert.equal(query('PRAGMA quick_check')[0].quick_check, 'ok'); assert.equal(query('PRAGMA foreign_key_check').length, 0);
    assert.equal(query('SELECT count(*) AS n FROM users')[0].n, 2);
    assert.equal(query('SELECT count(*) AS n FROM inventory_movements')[0].n, 2);
  });
  await step('Last active Owner cannot be deactivated through employee controls', async () => {
    await click('Employees');
    await click('Deactivate', "//div[span/span[contains(normalize-space(), 'UI Owner')]]");
    await driver.wait(until.elementLocated(By.css('[role="alert"]')), 15000);
    assert.equal(query("SELECT active FROM users WHERE username='ui-owner'")[0].active, 1);
  });
  await step('Owner opens a shift and refunds the cashier sale with stock restoration', async () => {
    await click('Shift & expenses'); await fill('Opening cash', '500.00', form('Open shift')); await click('Open shift', form('Open shift')); await status('Shift opened and ready for cash activity.');
    await click('Point of Sale'); await click('History'); await click('Open'); await fill('Reason for refund/void', 'UI verification return'); await click('Record refund'); await status('Refund recorded with explicit payment and stock allocations.');
    assert.equal(query('SELECT status FROM sales')[0].status, 'REFUNDED');
    assert.equal(query('SELECT quantity_micros FROM inventory_balances')[0].quantity_micros, 24000000);
  });
  await step('Waste and stock count are entered through inventory forms', async () => {
    await click('Inventory'); await fill('Quantity', '2', form('Waste or damage')); await fill('Reason', 'Broken bottles', form('Waste or damage')); await click('Record loss', form('Waste or damage')); await status('Stock loss recorded.');
    assert.equal(query('SELECT quantity_micros FROM inventory_balances')[0].quantity_micros, 22000000);
    await fill('Counted quantity', '20', form('Stock count')); await click('Complete count', form('Stock count')); await status('Stock count completed and variance posted.');
    assert.equal(query('SELECT quantity_micros FROM inventory_balances')[0].quantity_micros, 20000000);
  });
  await step('Split cash and card checkout persists both payment methods', async () => {
    await click('Point of Sale'); await click('Castle Lager 330ml'); await click('Exact cash'); await fill('Cash', '10.00'); await fill('Card', '20.00'); await fill('Cash tendered', '10.00');
    await tap("//button[starts-with(normalize-space(), 'Pay R')]"); await status('Receipt ready.');
    const sale = query("SELECT id FROM sales WHERE cashier_id=(SELECT id FROM users WHERE username='ui-owner')")[0];
    assert.equal(query(`SELECT count(*) AS n FROM payments WHERE sale_id='${sale.id}'`)[0].n, 2);
    assert.equal(query('SELECT quantity_micros FROM inventory_balances')[0].quantity_micros, 19000000);
  });
  await step('Insufficient stock checkout fails without a partial sale or payment', async () => {
    await click('Castle Lager 330ml');
    for (let i = 1; i < 20; i++) await tap("//button[@aria-label='Increase Castle Lager 330ml']");
    await click('Exact cash'); await tap("//button[starts-with(normalize-space(), 'Pay R')]");
    await see('inventory movement would make stock negative');
    assert.equal(query('SELECT count(*) AS n FROM sales')[0].n, 2);
    assert.equal(query('SELECT quantity_micros FROM inventory_balances')[0].quantity_micros, 19000000);
    await tap("//button[@aria-label='Remove Castle Lager 330ml']");
  });
  await step('Owner fully voids a split-payment sale through History', async () => {
    await click('Catalogue'); await click('Point of Sale'); await click('History');
    const saleNumber = query("SELECT sale_number FROM sales WHERE cashier_id=(SELECT id FROM users WHERE username='ui-owner')")[0].sale_number;
    await click('Open', `//tr[td[normalize-space()=${quoted(saleNumber)}]]`);
    await fill('Reason for refund/void', 'UI verification full void'); await click('Void full sale'); await status('Sale voided with traceable reversals.');
    assert.equal(query("SELECT status FROM sales WHERE sale_number='" + saleNumber + "'")[0].status, 'VOID');
    assert.equal(query('SELECT quantity_micros FROM inventory_balances')[0].quantity_micros, 20000000);
  });
  await step('Expense category, cash expense and reasoned void are entered through UI', async () => {
    await click('Shift & expenses'); await fill('Name', 'Cleaning', form('Expense category')); await click('Create category', form('Expense category')); await status('Expense category Cleaning created.');
    await choose('Category', 'Cleaning', form('Record expense')); await choose('Payment method', 'Cash', form('Record expense')); await fill('Amount', '10.00', form('Record expense')); await fill('Description', 'Cleaning supplies', form('Record expense')); await click('Record expense', form('Record expense')); await status('Expense recorded and cash expectation updated when applicable.');
    await click('Void'); await driver.wait(until.alertIsPresent(), 15000); await driver.switchTo().alert().sendKeys('UI verification correction'); await driver.switchTo().alert().accept(); await status('Expense voided without deleting its history.');
    assert.equal(query('SELECT status FROM expenses')[0].status, 'VOID');
  });
  await step('Owner closes and reconciles the shift through UI', async () => {
    await fill('Actual cash counted', '400.00'); await click('Close shift'); await status('Shift closed and variance preserved.');
    const shift = query("SELECT variance_minor FROM shifts JOIN users ON users.id=shifts.user_id WHERE users.username='ui-owner'")[0];
    assert.equal(shift.variance_minor, 0);
  });
  await step('Cashier PIN reset and deactivation operate through employee controls', async () => {
    await click('Employees');
    const cashierRow = "//div[span/span[normalize-space()='UI Cashier']]";
    await click('Reset PIN', cashierRow); await driver.wait(until.alertIsPresent(), 15000);
    cashierPin = String(randomInt(100000, 999999)); await driver.switchTo().alert().sendKeys(cashierPin); await driver.switchTo().alert().accept(); await status('PIN reset.');
    await click('Deactivate', cashierRow); await status('UI Cashier deactivated and signed out.');
    await click('Sign out'); await see('Sign in to this terminal'); await fill('Username', 'ui-cashier'); await fill('PIN', cashierPin); await click('Sign in');
    await driver.wait(until.elementLocated(By.css('[role="alert"]')), 15000); await see('Sign in to this terminal');
  });
  await step('Cashier reactivation permits login with the UI-reset PIN', async () => {
    await login('ui-owner', ownerPin); await click('Employees');
    await click('Reactivate', "//div[span/span[normalize-space()='UI Cashier']]"); await status('UI Cashier reactivated.');
    await click('Sign out'); await login('ui-cashier', cashierPin); await see('UI Cashier');
    await click('Sign out'); await login('ui-owner', ownerPin);
  });
  await step('Backup restore through UI restores records and signs the user out', async () => {
    await click('Backup & safety'); await click('Create manual backup'); await status('Verified backup created:');
    const backupStatus = await driver.findElement(By.css('[role="status"]')).getText();
    const filename = backupStatus.match(/manual-[^\s]+\.db/)[0];
    await click('Catalogue');
    const itemForm = form('Add an item');
    await (await element(`${itemForm}//select[option[@value='SERVICE']]`)).findElement(By.css('option[value="SERVICE"]')).click();
    await fill('Name', 'Restore probe service', itemForm); await fill('Price (ZAR)', '5.00', itemForm); await click('Add service', itemForm); await status('Restore probe service added to the catalogue.');
    await click('Backup & safety'); await click('Restore', `//tr[td[contains(normalize-space(), ${quoted(filename)})]]`);
    await driver.wait(until.alertIsPresent(), 15000); await driver.switchTo().alert().accept(); await see('Sign in to this terminal');
    await login('ui-owner', ownerPin); await click('Catalogue'); await see('Castle Lager 330ml');
    assert.equal(query("SELECT count(*) AS n FROM sellable_items WHERE name='Restore probe service'")[0].n, 0);
    assert.equal(query('PRAGMA quick_check')[0].quick_check, 'ok'); assert.equal(query('PRAGMA foreign_key_check').length, 0);
  });
  await step('Application restarts and UI-created data persists', async () => {
    await driver.quit(); driver = await connect(); await login('ui-owner', ownerPin); await click('Catalogue'); await see('Castle Lager 330ml'); await see('Basic Wash');
  });
} catch (error) {
  console.error(error.stack);
  if (driver) {
    await writeFile(path.join(runDir, 'failure.png'), await driver.takeScreenshot().catch(() => ''), 'base64');
    await writeFile(path.join(runDir, 'failure.txt'), await body().catch(() => 'Window unavailable'));
  }
  process.exitCode = 1;
} finally {
  if (driver) await driver.quit().catch(() => {});
  server.kill();
  await writeFile(path.join(runDir, 'results.json'), JSON.stringify({ dataDir, results }, null, 2));
  await writeFile(path.join(runDir, 'webdriver.log'), serverLog);
  console.log(`Artifacts: ${runDir}`);
}
