import js from "@eslint/js";
import tseslint from "typescript-eslint";
import svelte from "eslint-plugin-svelte";
import globals from "globals";

export default tseslint.config(
  {
    ignores: [
      "dist/**",
      "node_modules/**",
      "src-tauri/**",
      "docs/**",
      "probe/**",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...svelte.configs.recommended,
  {
    // .svelte.ts modules contain runes-enabled TypeScript, not Svelte
    // markup — they must go through the TypeScript parser.
    files: ["**/*.svelte.ts"],
    languageOptions: {
      parser: tseslint.parser,
      parserOptions: {
        sourceType: "module",
        ecmaVersion: "latest",
      },
    },
  },
  {
    files: ["**/*.svelte"],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
        sourceType: "module",
        ecmaVersion: "latest",
        extraFileExtensions: [".svelte"],
      },
      globals: globals.browser,
    },
  },
  {
    files: ["src/**/*.ts"],
    languageOptions: {
      globals: globals.browser,
    },
  },
  {
    files: ["src/**/*.test.ts", "tests/**/*.ts"],
    languageOptions: {
      globals: globals.node,
    },
  },
);
