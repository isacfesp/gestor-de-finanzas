// Config de Tailwind v3 (versión que trae el binario standalone que
// descarga Trunk automáticamente al ver `data-trunk rel="tailwind-css"`
// en index.html — no requiere Node/npm instalados en el proyecto).
//
// Los colores quedan indirectos (`var(--bg)`, no el hex) para seguir
// reaccionando en vivo a `[data-theme]`, que es lo que alterna
// `src/components/theme.rs`. Los valores en sí viven en
// `styles/tailwind.css` (:root / :root[data-theme]).
/** @type {import('tailwindcss').Config} */
module.exports = {
  // `index.html` incluido a propósito: las clases del loader inicial
  // (`.wasm-loader-*`, ver `styles/tailwind.css`) solo aparecen ahí —
  // vive fuera del árbol de Leptos — y sin este glob Tailwind las purga
  // por "no usadas", dejando el loader sin animación.
  content: ["./src/**/*.rs", "./index.html"],
  theme: {
    extend: {
      colors: {
        bg: "var(--bg)",
        "bg-2": "var(--bg-2)",
        panel: "var(--panel)",
        "panel-2": "var(--panel-2)",
        card: "var(--card)",
        "card-line": "var(--card-line)",
        line: "var(--line)",
        text: "var(--text)",
        muted: "var(--muted)",
        faint: "var(--faint)",
        accent: "var(--accent)",
        "accent-2": "var(--accent-2)",
        "accent-soft": "var(--accent-soft)",
        positive: "var(--positive)",
        negative: "var(--negative)",
        warning: "var(--warning)",
        hover: "var(--hover)",
        sidebar: "var(--sidebar)",
      },
      fontFamily: {
        mono: ["JetBrains Mono", "ui-monospace", '"SF Mono"', "Menlo", "Consolas", "monospace"],
        sans: ["Plus Jakarta Sans", "system-ui", "sans-serif"],
      },
      borderRadius: {
        lg: "16px",
        pane: "11px",
        sm: "9px",
        xs: "6px",
      },
    },
  },
  plugins: [],
};
