import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { App, setupSteps } from './App';

describe('first-run setup', () => {
  it('renders the offline business setup form and roadmap', () => {
    const markup = renderToStaticMarkup(<App />);

    expect(markup).toContain('Business name');
    expect(markup).toContain('Create business');
    expect(markup).toContain('Offline ready');
    for (const step of setupSteps) expect(markup).toContain(step.replace('&', '&amp;'));
  });
});
