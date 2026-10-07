import { defineConfig } from "vitest/config";
import solid from "vite-plugin-solid";

// The stylesheets and shared code live outside this folder (../../mockup, ../shared).
export default defineConfig({
  plugins: [solid()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, fs: { allow: ["../.."] } },
  // The tests sit beside the framework-free code they test, in ../shared. The node
  // environment, because vite-plugin-solid would otherwise ask for jsdom, and these need
  // no DOM.
  test: { dir: "..", include: ["shared/**/*.test.ts"], environment: "node" },
});
