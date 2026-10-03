import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

// The stylesheets and shared code live outside this folder (../../mockup, ../shared).
export default defineConfig({
  plugins: [solid()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, fs: { allow: ["../.."] } },
});
