/** @type {import('tailwindcss').Config} */
export default {
  darkMode: "class",
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        "on-tertiary-fixed": "#001a42", "outline": "#ab897d", "on-secondary-container": "#004931", "surface-container": "#1d2025", "surface-tint": "#ffb59a", "primary-fixed-dim": "#ffb59a", "primary-container": "#ff5c00", "secondary-fixed-dim": "#41dfa3", "error": "#ffb4ab", "on-tertiary-fixed-variant": "#004395", "on-surface-variant": "#e4beb1", "on-primary-container": "#521800", "inverse-primary": "#a73a00", "on-background": "#e1e2e9", "on-error-container": "#ffdad6", "inverse-surface": "#e1e2e9", "tertiary-fixed": "#d8e2ff", "secondary-container": "#00c389", "on-error": "#690005", "error-container": "#93000a", "primary-fixed": "#ffdbce", "surface-container-highest": "#32353a", "secondary": "#41dfa3", "surface-container-lowest": "#0b0e13", "tertiary-fixed-dim": "#adc6ff", "on-secondary": "#003825", "outline-variant": "#5b4137", "surface-container-high": "#272a2f", "on-secondary-fixed-variant": "#005237", "on-secondary-fixed": "#002114", "on-primary-fixed-variant": "#802a00", "tertiary": "#adc6ff", "primary": "#ffb59a", "surface-bright": "#36393f", "background": "#101418", "surface-variant": "#32353a", "on-surface": "#e1e2e9", "surface": "#101418", "secondary-fixed": "#64fcbd", "tertiary-container": "#5190ff", "surface-dim": "#101418", "on-tertiary": "#002e6a", "inverse-on-surface": "#2e3036", "on-primary-fixed": "#370e00", "on-primary": "#5a1b00", "surface-container-low": "#191c21", "on-tertiary-container": "#002960"
      },
      borderRadius: { DEFAULT: "0.25rem", lg: "0.5rem", xl: "0.75rem", full: "9999px" },
      spacing: { "space-sm": "0.75rem", "space-lg": "1.5rem", "space-md": "1rem", margin: "1rem", gutter: "1rem", "space-xs": "0.375rem", "space-xl": "2rem" },
      fontFamily: {
        "badge-label": ["Plus Jakarta Sans", "sans-serif"],
        "keypad-key": ["Plus Jakarta Sans", "sans-serif"],
        "headline-sm": ["Plus Jakarta Sans", "sans-serif"],
        "display-numpad": ["Plus Jakarta Sans", "sans-serif"],
        "price-tag": ["Plus Jakarta Sans", "sans-serif"],
        "body-md": ["Inter", "sans-serif"],
        "ticket-total": ["Plus Jakarta Sans", "sans-serif"],
        "body-lg": ["Inter", "sans-serif"],
        "headline-md": ["Plus Jakarta Sans", "sans-serif"],
        "body-sm": ["Inter", "sans-serif"],
        "headline-lg": ["Plus Jakarta Sans", "sans-serif"]
      },
      fontSize: {
        "badge-label": ["12px", { lineHeight: "14px", letterSpacing: "0.06em", fontWeight: "800" }],
        "keypad-key": ["24px", { lineHeight: "28px", fontWeight: "700" }],
        "headline-sm": ["18px", { lineHeight: "24px", fontWeight: "700" }],
        "display-numpad": ["38px", { lineHeight: "44px", letterSpacing: "-0.02em", fontWeight: "800" }],
        "price-tag": ["17px", { lineHeight: "20px", fontWeight: "800" }],
        "body-md": ["14px", { lineHeight: "20px", fontWeight: "500" }],
        "ticket-total": ["32px", { lineHeight: "36px", letterSpacing: "-0.02em", fontWeight: "800" }],
        "body-lg": ["16px", { lineHeight: "22px", fontWeight: "600" }],
        "headline-md": ["22px", { lineHeight: "28px", letterSpacing: "-0.01em", fontWeight: "700" }],
        "body-sm": ["12px", { lineHeight: "16px", fontWeight: "400" }],
        "headline-lg": ["28px", { lineHeight: "34px", letterSpacing: "-0.02em", fontWeight: "700" }]
      }
    }
  },
  plugins: [],
}
