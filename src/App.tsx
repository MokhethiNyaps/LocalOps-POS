import { FormEvent, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export const setupSteps = [
  'Business setup',
  'Departments & locations',
  'Products & services',
  'Inventory',
  'POS & payments',
] as const;

type SetupFields = {
  businessName: string;
  departmentName: string;
  locationName: string;
  terminalName: string;
  ownerUsername: string;
  ownerDisplayName: string;
  ownerPin: string;
};

const initialFields: SetupFields = {
  businessName: '',
  departmentName: '',
  locationName: '',
  terminalName: '',
  ownerUsername: '',
  ownerDisplayName: '',
  ownerPin: '',
};

export function App() {
  const [fields, setFields] = useState(initialFields);
  const [submitting, setSubmitting] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  function updateField(name: keyof SetupFields, value: string) {
    setFields((current) => ({ ...current, [name]: value }));
  }

  async function createBusiness(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting) return;
    setSubmitting(true);
    setMessage(null);
    setError(null);
    try {
      await invoke('complete_initial_setup', { input: fields });
      setMessage(`${fields.businessName.trim()} is ready and you are signed in as the owner.`);
      setFields(initialFields);
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
        <p className="kicker">FIRST-RUN SETUP</p>
        <h2>Build your local workspace</h2>
        <p>Create the business, its first operation, and the owner account in one secure offline step.</p>
        <form onSubmit={createBusiness}>
          <div className="field-grid">
            <Field label="Business name" name="businessName" value={fields.businessName} onChange={updateField} placeholder="Moko’s Lifestyle Centre" />
            <Field label="Owner name" name="ownerDisplayName" value={fields.ownerDisplayName} onChange={updateField} placeholder="Moko" />
            <Field label="First department" name="departmentName" value={fields.departmentName} onChange={updateField} placeholder="Bar" />
            <Field label="Main stock location" name="locationName" value={fields.locationName} onChange={updateField} placeholder="Main Store" />
            <Field label="Terminal name" name="terminalName" value={fields.terminalName} onChange={updateField} placeholder="Main Till" />
            <Field label="Owner username" name="ownerUsername" value={fields.ownerUsername} onChange={updateField} placeholder="owner" />
            <Field label="Owner PIN" name="ownerPin" value={fields.ownerPin} onChange={updateField} placeholder="4–12 digits" type="password" pattern="[0-9]{4,12}" />
          </div>
          <button type="submit" disabled={submitting}>
            {submitting ? 'Creating securely…' : 'Create business'} <span aria-hidden="true">→</span>
          </button>
        </form>
        {message && <p className="notice success" role="status">{message}</p>}
        {error && <p className="notice error" role="alert">Setup could not be completed: {error}</p>}
      </div>
      <aside>
        <strong>Setup journey</strong>
        {setupSteps.map((step, index) => <div className="step" key={step}><b>{index + 1}</b><span>{step}</span></div>)}
      </aside>
    </section>
    <footer><span>Data stays on this device</span><span>SQLite secured</span><span>Automatic local backups</span></footer>
  </main>;
}

type FieldProps = {
  label: string;
  name: keyof SetupFields;
  value: string;
  placeholder: string;
  type?: string;
  pattern?: string;
  onChange: (name: keyof SetupFields, value: string) => void;
};

function Field({ label, name, value, placeholder, type = 'text', pattern, onChange }: FieldProps) {
  return <label className="field" htmlFor={name}>
    <span>{label}</span>
    <input
      id={name}
      name={name}
      value={value}
      onChange={(event) => onChange(name, event.target.value)}
      placeholder={placeholder}
      type={type}
      pattern={pattern}
      required
    />
  </label>;
}
