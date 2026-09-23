import { FormEvent, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export const setupSteps = [
  'Business setup',
  'Departments & locations',
  'Products & services',
  'Inventory',
  'POS & payments',
] as const;

export function App() {
  const [businessName, setBusinessName] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function createBusiness(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const name = businessName.trim();
    if (!name || submitting) return;

    setSubmitting(true);
    setMessage(null);
    setError(null);
    try {
      await invoke<string>('create_business', { name });
      setMessage(`${name} is ready. Next, add its departments and locations.`);
      setBusinessName('');
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setSubmitting(false);
    }
  }

  return <main className="shell">
    <header>
      <div className="brand" aria-hidden="true">LO</div>
      <div><p className="eyebrow">LOCALOPS POS</p><h1>Your business. One place. Always available.</h1></div>
      <span className="offline">● Offline ready</span>
    </header>
    <section className="hero">
      <div>
        <p className="kicker">WELCOME</p>
        <h2>Let’s set up your workspace</h2>
        <p>LocalOps keeps your departments, stock, sales and payments together on this computer—no internet required.</p>
        <form onSubmit={createBusiness}>
          <label htmlFor="business-name">Business name</label>
          <div className="create-row">
            <input
              id="business-name"
              name="businessName"
              value={businessName}
              onChange={(event) => setBusinessName(event.target.value)}
              placeholder="e.g. Moko’s Lifestyle Centre"
              autoComplete="organization"
              required
            />
            <button type="submit" disabled={!businessName.trim() || submitting}>
              {submitting ? 'Creating…' : 'Create business'} <span aria-hidden="true">→</span>
            </button>
          </div>
        </form>
        {message && <p className="notice success" role="status">{message}</p>}
        {error && <p className="notice error" role="alert">Could not create the business: {error}</p>}
      </div>
      <aside>
        <strong>Setup journey</strong>
        {setupSteps.map((step, index) => <div className="step" key={step}><b>{index + 1}</b><span>{step}</span></div>)}
      </aside>
    </section>
    <footer><span>Data stays on this device</span><span>SQLite secured</span><span>Automatic local backups</span></footer>
  </main>;
}
