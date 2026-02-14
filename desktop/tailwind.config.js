/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        "laminar-black": "#0a0a0a",
        "laminar-white": "#fafafa",
        "laminar-gray": "#404040",
        "laminar-green": "#22c55e",
        "laminar-red": "#ef4444",
        "laminar-yellow": "#eab308"
      },
      fontFamily: {
        sans: ["Inter", "ui-sans-serif", "system-ui", "sans-serif"],
        mono: ["JetBrains Mono", "ui-monospace", "SFMono-Regular", "monospace"]
      }
    }
  },
  plugins: []
};
