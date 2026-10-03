import tseslint from "typescript-eslint";
import solid from "eslint-plugin-solid/configs/typescript";

// eslint-plugin-solid is the guard for Solid's tracking-scope discipline (ADR-0047).
export default [
  { ignores: ["dist", "src-tauri"] },
  ...tseslint.configs.recommended,
  { files: ["src/**/*.{ts,tsx}"], ...solid },
];
