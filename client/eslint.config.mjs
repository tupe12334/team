import { defineConfig, globalIgnores } from "eslint/config";
import nextVitals from "eslint-config-next/core-web-vitals";
import nextTs from "eslint-config-next/typescript";
import agent from "eslint-config-agent";

// eslint-config-next redefines react-hooks, so drop it from agent config
const agentWithoutReactHooks = agent.filter(
  (cfg) => cfg.name !== "react-hooks/recommended"
);

const eslintConfig = defineConfig([
  ...agentWithoutReactHooks,
  ...nextVitals,
  ...nextTs,
  // Override default ignores of eslint-config-next.
  globalIgnores([
    // Default ignores of eslint-config-next:
    ".next/**",
    "out/**",
    "build/**",
    "next-env.d.ts",
  ]),
  // Test file overrides — relax rules that don't apply in spec context.
  {
    files: ["**/*.spec.{ts,tsx}"],
    rules: {
      "no-restricted-syntax": "off",
      "error/no-literal-error-message": "off",
      "@typescript-eslint/no-confusing-void-expression": "off",
      "import/first": "off",
      "import/order": "off",
      "prefer-promise-reject-errors": "off",
    },
  },
]);

export default eslintConfig;
