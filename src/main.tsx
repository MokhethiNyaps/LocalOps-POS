import React from 'react';
import ReactDOM from 'react-dom/client';
import './styles.css';

const steps = ['Business setup','Departments & locations','Products & services','Inventory','POS & payments'];
function App() {
  return <main className="shell">
    <header><div className="brand">LO</div><div><p className="eyebrow">LOCALOPS POS</p><h1>Your business. One place. Always available.</h1></div><span className="offline">● Offline ready</span></header>
    <section className="hero"><div><p className="kicker">WELCOME</p><h2>Let’s set up your workspace</h2><p>LocalOps keeps your departments, stock, sales and payments together on this computer—no internet required.</p><button type="button">Create your business <span>→</span></button></div><aside><strong>Setup journey</strong>{steps.map((step,i)=><div className="step" key={step}><b>{i+1}</b><span>{step}</span></div>)}</aside></section>
    <footer><span>Data stays on this device</span><span>SQLite secured</span><span>Automatic local backups</span></footer>
  </main>;
}
ReactDOM.createRoot(document.getElementById('root')!).render(<React.StrictMode><App/></React.StrictMode>);
