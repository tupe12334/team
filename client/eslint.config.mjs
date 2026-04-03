import { defineConfig, globalIgnores } from "eslint/config";
import nextVitals from "eslint-config-next/core-web-vitals";
import nextTs from "eslint-config-next/typescript";
import agent from "eslint-config-agent";

const warnRules = new Set([
  "ddd/require-spec-file",
  "single-export/single-export",
  "jsx-classname/require-classname",
  "max-lines",
  "max-lines-per-function",
  "error/no-generic-error",
  "error/no-literal-error-message",
  "error/require-custom-error",
  "no-restricted-syntax",
  "import/no-namespace",
  "import/order",
  "default/no-localhost",
  "security/detect-object-injection",
  "@typescript-eslint/dot-notation",
  "@typescript-eslint/no-confusing-void-expression",
  "@typescript-eslint/no-floating-promises",
  "@typescript-eslint/no-misused-promises",
  "@typescript-eslint/no-necessary-condition",
  "@typescript-eslint/no-unnecessary-condition",
  "@typescript-eslint/no-unnecessary-type-assertion",
  "@typescript-eslint/no-unnecessary-type-parameters",
  "@typescript-eslint/no-unsafe-argument",
  "@typescript-eslint/no-unsafe-assignment",
  "@typescript-eslint/no-unsafe-call",
  "@typescript-eslint/no-unsafe-member-access",
  "@typescript-eslint/restrict-template-expressions",
  "@typescript-eslint/no-deprecated",
]);

function downgradeToWarn(configs) {
  return configs.map((cfg) => {
    if (!cfg.rules) return cfg;
    const rules = Object.fromEntries(
      Object.entries(cfg.rules).map(([name, value]) => {
        if (!warnRules.has(name)) return [name, value];
        const severity = Array.isArray(value) ? value[0] : value;
        if (severity !== "error" && severity !== 2) return [name, value];
        if (Array.isArray(value)) return [name, ["warn", ...value.slice(1)]];
        return [name, "warn"];
      })
    );
    return { ...cfg, rules };
  });
}

// eslint-config-next redefines react-hooks, so drop it from agent config
const agentWithoutReactHooks = agent.filter(
  (cfg) => cfg.name !== "react-hooks/recommended"
);

const eslintConfig = defineConfig([
  ...downgradeToWarn(agentWithoutReactHooks),
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
]);

export default eslintConfig;
