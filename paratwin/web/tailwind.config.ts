import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./app/**/*.{ts,tsx}", "./components/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        paragon: {
          50: "#eef4fb",
          100: "#d9e6f6",
          200: "#b3cdec",
          300: "#82abdf",
          400: "#4b82cd",
          500: "#215aa9",
          600: "#1c4d92",
          700: "#173f77",
          800: "#12315d",
          900: "#0d2444",
          950: "#081729"
        },
        accent: { teal: "#0f8b8d", orange: "#d97b29", green: "#2e8b57", red: "#c0392b" }
      },
      fontFamily: {
        sans: ["-apple-system", "BlinkMacSystemFont", "Segoe UI", "Inter", "Roboto", "sans-serif"]
      }
    }
  },
  plugins: []
};
export default config;
