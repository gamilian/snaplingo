/** @type {import('tailwindcss').Config} */

// Semantic tokens live in CSS variables (see src/styles/index.css) so that one
// palette swap flips the whole app between light and dark. Channels are written
// as space-separated RGB so Tailwind's opacity modifiers keep working
// (`bg-surface/95`, `border-line/50`).
const token = (name) => `rgb(var(--${name}) / <alpha-value>)`;

export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // ── Semantic surface / text tokens ───────────────────────────────
        canvas: token('canvas'),
        surface: token('surface'),
        sunken: token('sunken'),
        raised: token('raised'),
        line: token('line'),
        'line-strong': token('line-strong'),
        ink: token('ink'),
        muted: token('muted'),
        faint: token('faint'),

        // ── Accent ───────────────────────────────────────────────────────
        accent: {
          DEFAULT: token('accent'),
          soft: token('accent-soft'),
          line: token('accent-line'),
          text: token('accent-text'),
          hover: token('accent-hover'),
          on: token('accent-on'),
        },

        // ── State ────────────────────────────────────────────────────────
        good: { DEFAULT: token('good'), soft: token('good-soft') },
        warn: { DEFAULT: token('warn'), soft: token('warn-soft') },
        danger: { DEFAULT: token('danger'), soft: token('danger-soft') },

        // Status hues used across the views (errors, warnings, success) map onto
        // the three state tokens so they invert correctly in dark mode instead
        // of leaving a red-50 chip glowing on a near-black card.
        red: {
          50: token('danger-soft'), 100: token('danger-soft'),
          200: token('danger-soft'), 300: token('danger'),
          400: token('danger'), 500: token('danger'),
          600: token('danger'), 700: token('danger'),
          800: token('danger'), 900: token('danger'),
          950: token('danger-soft'),
        },
        rose: {
          50: token('danger-soft'), 100: token('danger-soft'),
          200: token('danger-soft'), 500: token('danger'), 600: token('danger'),
        },
        emerald: {
          50: token('good-soft'), 100: token('good-soft'),
          200: token('good-soft'), 300: token('good'),
          400: token('good'), 500: token('good'),
          600: token('good'), 700: token('good'),
        },
        green: {
          50: token('good-soft'), 100: token('good-soft'),
          400: token('good'), 500: token('good'), 700: token('good'),
        },
        amber: {
          50: token('warn-soft'), 100: token('warn-soft'),
          200: token('warn-soft'), 500: token('warn'),
          600: token('warn'), 700: token('warn'),
          800: token('warn'), 900: token('warn'),
        },
        yellow: { 50: token('warn-soft'), 500: token('warn'), 600: token('warn') },
        blue: {
          50: token('accent-soft'), 100: token('accent-soft'),
          200: token('accent-line'), 300: token('accent-line'),
          400: token('accent'), 500: token('accent'),
          600: token('accent-text'), 700: token('accent-hover'),
          800: token('accent-text'),
        },
        sky: {
          50: token('accent-soft'), 300: token('accent-line'),
          400: token('accent'), 700: token('accent-text'),
        },
        indigo: {
          50: token('accent-soft'), 100: token('accent-line'),
          500: token('accent'),
        },
        purple: {
          100: token('accent-soft'), 500: token('accent'),
          700: token('accent-text'),
        },
        orange: {
          50: token('warn-soft'), 100: token('warn-soft'),
          600: token('warn'),
        },

        // ── Legacy scales, remapped onto tokens ──────────────────────────
        // The palette was built on Tailwind's gray/slate/primary names. Rather
        // than rewrite ~800 class names across 51 files, point those names at
        // the token layer: every existing usage picks up the new palette, and
        // dark mode follows the same variables instead of an !important overlay.
        primary: {
          50: token('accent-soft'),
          100: token('accent-line'),
          200: token('accent-line'),
          300: token('accent'),
          400: token('accent'),
          500: token('accent'),
          600: token('accent'),
          700: token('accent-text'),
        },
        gray: {
          50: token('sunken'),
          100: token('raised'),
          200: token('line'),
          300: token('line-strong'),
          400: token('faint'),
          500: token('muted'),
          600: token('muted'),
          700: token('ink'),
          800: token('ink'),
          900: token('ink'),
          950: token('ink'),
        },
        slate: {
          50: token('sunken'),
          100: token('raised'),
          200: token('line'),
          300: token('line-strong'),
          400: token('faint'),
          500: token('muted'),
          600: token('muted'),
          700: token('ink'),
          800: token('ink'),
          900: token('ink'),
          950: token('ink'),
        },

        // Provider brand colours stay literal — they are other companies' marks.
        google: '#4285f4',
        deepl: '#0f2b46',
        baidu: '#2932e1',
        openai: '#10a37f',
      },
      fontSize: {
        // Six steps, each with one job. The ad-hoc 9–26px values scattered
        // through the views collapse onto these.
        'cap': ['9.5px', { lineHeight: '1.4' }],      // group labels, uppercase
        'hint': ['10.5px', { lineHeight: '1.45' }],   // secondary text, units
        'label': ['12.5px', { lineHeight: '1.45' }],  // setting row labels
        'card': ['13px', { lineHeight: '1.45' }],     // card titles
        'title': ['15px', { lineHeight: '1.3' }],     // window / section titles
        'display': ['19px', { lineHeight: '1.25' }],  // page title

        // Tailwind's default steps sit between the six above. Rather than
        // rewrite ~200 existing call sites, re-point them at the new scale so
        // `text-sm` reads as a row label and `text-xs` as supporting text.
        'xs': ['10.5px', { lineHeight: '1.45' }],
        'sm': ['12.5px', { lineHeight: '1.5' }],
        'base': ['13px', { lineHeight: '1.5' }],
        'lg': ['15px', { lineHeight: '1.45' }],
        'xl': ['17px', { lineHeight: '1.35' }],
        '2xl': ['20px', { lineHeight: '1.3' }],
        '3xl': ['22px', { lineHeight: '1.25' }],
      },
      borderRadius: {
        'sm': '5px',
        'md': '7px',
        'lg': '10px',
        'xl': '13px',
        '2xl': '16px',
      },
      boxShadow: {
        'e1': '0 1px 2px rgb(var(--shadow-color) / 0.06), 0 0 0 1px rgb(var(--shadow-color) / 0.05)',
        'e2': '0 3px 10px rgb(var(--shadow-color) / 0.08), 0 1px 3px rgb(var(--shadow-color) / 0.05)',
        'e3': '0 16px 40px rgb(var(--shadow-color) / 0.18), 0 2px 8px rgb(var(--shadow-color) / 0.10)',
        // Lifts a light panel off the capture scrim, which is dark in both
        // themes, so this one is deliberately opaque rather than tokenised.
        'float': '0 12px 34px rgba(15, 23, 42, 0.28), 0 2px 8px rgba(15, 23, 42, 0.14)',
        // Kept as aliases so existing `shadow-sm/md/lg/xl` call sites get the
        // tightened scale without a rename pass.
        'xs': '0 1px 2px rgb(var(--shadow-color) / 0.05)',
        'sm': '0 1px 2px rgb(var(--shadow-color) / 0.06), 0 0 0 1px rgb(var(--shadow-color) / 0.05)',
        'md': '0 3px 10px rgb(var(--shadow-color) / 0.08), 0 1px 3px rgb(var(--shadow-color) / 0.05)',
        'lg': '0 8px 24px rgb(var(--shadow-color) / 0.12)',
        'xl': '0 16px 40px rgb(var(--shadow-color) / 0.18)',
      },
      spacing: {
        '1': '4px',
        '2': '8px',
        '3': '12px',
        '4': '16px',
        '6': '24px',
        '8': '32px',
        '12': '48px',
      },
      transitionDuration: {
        '150': '150ms',
      },
      transitionTimingFunction: {
        'smooth': 'cubic-bezier(0.4, 0, 0.2, 1)',
      },
    },
  },
  plugins: [],
}
