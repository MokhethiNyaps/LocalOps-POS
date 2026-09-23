import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { App, parseScaled, setupSteps } from './App';

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
