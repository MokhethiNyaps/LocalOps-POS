import plugin from 'tailwindcss/plugin';

/**
 * Single source of truth for the LocalOps type scale.
 *
 * Every token below produces two utilities:
 *   .font-<token>  → the complete type style (family, size, line-height, tracking, weight)
 *   .text-<token>  → size, line-height, tracking, weight only
 *
 * The composite `.font-*` utility exists because the interface is authored with
 * `font-headline-lg`, `font-body-md`, … and a bare `font-*` utility would otherwise
 * only set the family, silently dropping the whole scale.
 */
const display = ['Plus Jakarta Sans Variable', 'Plus Jakarta Sans', 'Segoe UI', 'system-ui', 'sans-serif'];
const body = ['Inter Variable', 'Inter', 'Segoe UI', 'system-ui', 'sans-serif'];

const typeScale = {
  display: { family: display, fontSize: '40px', lineHeight: '46px', letterSpacing: '-0.03em', fontWeight: '800' },
  'display-numpad': { family: display, fontSize: '38px', lineHeight: '44px', letterSpacing: '-0.02em', fontWeight: '800', numeric: true },
  'ticket-total': { family: display, fontSize: '32px', lineHeight: '38px', letterSpacing: '-0.02em', fontWeight: '800', numeric: true },
  'headline-lg': { family: display, fontSize: '28px', lineHeight: '34px', letterSpacing: '-0.02em', fontWeight: '700' },
  'headline-md': { family: display, fontSize: '22px', lineHeight: '28px', letterSpacing: '-0.01em', fontWeight: '700' },
  'headline-sm': { family: display, fontSize: '18px', lineHeight: '24px', fontWeight: '700' },
  'keypad-key': { family: display, fontSize: '24px', lineHeight: '28px', fontWeight: '700', numeric: true },
  'price-tag': { family: display, fontSize: '17px', lineHeight: '22px', fontWeight: '800', numeric: true },
  'badge-label': { family: display, fontSize: '12px', lineHeight: '16px', letterSpacing: '0.06em', fontWeight: '800' },
  'body-lg': { family: body, fontSize: '16px', lineHeight: '22px', fontWeight: '600' },
  'body-md': { family: body, fontSize: '14px', lineHeight: '20px', fontWeight: '500' },
  'body-sm': { family: body, fontSize: '12px', lineHeight: '16px', fontWeight: '400' },
};

const fontFamily = Object.fromEntries(Object.entries(typeScale).map(([token, value]) => [token, value.family]));
const fontSize = Object.fromEntries(Object.entries(typeScale).map(([token, { fontSize: size, lineHeight, letterSpacing, fontWeight }]) => [
  token,
  [size, { lineHeight, ...(letterSpacing ? { letterSpacing } : {}), fontWeight }],
]));

/** @type {import('tailwindcss').Config} */
export default {
  darkMode: "class",
  content: [
    "./index.html",
    "./tests/**/*.{html,ts,tsx}",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        "on-tertiary-fixed": "#001a42", "outline": "#ab897d", "on-secondary-container": "#004931", "surface-container": "#1d2025", "surface-tint": "#ffb59a", "primary-fixed-dim": "#ffb59a", "primary-container": "#ff5c00", "secondary-fixed-dim": "#41dfa3", "error": "#ffb4ab", "on-tertiary-fixed-variant": "#004395", "on-surface-variant": "#e4beb1", "on-primary-container": "#521800", "inverse-primary": "#a73a00", "on-background": "#e1e2e9", "on-error-container": "#ffdad6", "inverse-surface": "#e1e2e9", "tertiary-fixed": "#d8e2ff", "secondary-container": "#00c389", "on-error": "#690005", "error-container": "#93000a", "primary-fixed": "#ffdbce", "surface-container-highest": "#32353a", "secondary": "#41dfa3", "surface-container-lowest": "#0b0e13", "tertiary-fixed-dim": "#adc6ff", "on-secondary": "#003825", "outline-variant": "#5b4137", "surface-container-high": "#272a2f", "on-secondary-fixed-variant": "#005237", "on-secondary-fixed": "#002114", "on-primary-fixed-variant": "#802a00", "tertiary": "#adc6ff", "primary": "#ffb59a", "surface-bright": "#36393f", "background": "#101418", "surface-variant": "#32353a", "on-surface": "#e1e2e9", "surface": "#171b21", "secondary-fixed": "#64fcbd", "tertiary-container": "#5190ff", "surface-dim": "#101418", "on-tertiary": "#002e6a", "inverse-on-surface": "#2e3036", "on-primary-fixed": "#370e00", "on-primary": "#5a1b00", "surface-container-low": "#191c21", "on-tertiary-container": "#002960"
      },
      borderRadius: { DEFAULT: "0.25rem", lg: "0.5rem", xl: "0.75rem", "2xl": "1rem", "3xl": "1.5rem", full: "9999px" },
      spacing: { "space-xs": "0.375rem", "space-sm": "0.75rem", "space-md": "1rem", "space-lg": "1.5rem", "space-xl": "2rem", margin: "1rem", gutter: "1rem", header: "4rem", rail: "5rem", sidebar: "17rem" },
      fontFamily,
      fontSize,
    }
  },
  plugins: [
    /* Composite type utilities: `.font-<token>` applies the complete style. */
    plugin(({ addUtilities }) => {
      addUtilities(Object.fromEntries(Object.entries(typeScale).map(([token, value]) => [
        `.font-${token}`,
        {
          fontFamily: value.family.join(', '),
          fontSize: value.fontSize,
          lineHeight: value.lineHeight,
          ...(value.letterSpacing ? { letterSpacing: value.letterSpacing } : {}),
          fontWeight: value.fontWeight,
          ...(value.numeric ? { fontVariantNumeric: 'tabular-nums' } : {}),
        },
      ])));
      addUtilities({
        '.scrollbar-hide': {
          'scrollbar-width': 'none',
          '-ms-overflow-style': 'none',
          '&::-webkit-scrollbar': { display: 'none' },
        },
      });
    }),
  ],
}
