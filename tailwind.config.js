/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        'bg-primary': '#0d0d0d',
        'bg-secondary': '#161616',
        'accent': '#6c5ce7',
        'text-primary': '#eaeaea',
        'text-secondary': '#aaaaaa',
        'border': '#2a2a2a',
      },
    },
  },
  plugins: [],
}
