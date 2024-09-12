/** @type {import('tailwindcss').Config} */

const { fontFamily } = require("tailwindcss/defaultTheme");

module.exports = {
  content: ["./themes/**/*.html", "./templates/**/*.html", "./content/**/*.md"],
  theme: {
    extend: {
      typography: ({ theme }) => ({
        DEFAULT: {
          // this is for prose class
          css: {
            "--tw-prose-headings": theme("colors.foreground"),
          },
        },
      }),
      spacing: {},
      colors: {
        background: "var(--background)",
        "background-secondary": "var(--background-secondary)",
        foreground: "var(--foreground)",
        primary: {
          DEFAULT: "var(--primary)",
          foreground: "var(--primary-foreground)",
        },
        secondary: {
          DEFAULT: "var(--secondary)",
          foreground: "var(--secondary-foreground)",
        },
        border: "var(--border)",
        card: {
          DEFAULT: "var(--card)",
          foreground: "var(--card-foreground)",
        },
      },
      borderRadius: {
        lg: `var(--radius)`,
        md: `calc(var(--radius) - 2px)`,
        sm: "calc(var(--radius) - 4px)",
      },
      fontFamily: {
        inter: ["Inter"],
        text: ["Inter"],
        heading: ["Arimo"],
      },
    },
  },
  variants: {
    extend: {
      inset: ["negative"],
    },
  },
  plugins: [require("@tailwindcss/typography")],
};
