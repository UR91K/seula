import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// The stylesheets and shared code live outside this folder (../../mockup, ../shared).
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1421, strictPort: true, fs: { allow: ["../.."] } },
});
