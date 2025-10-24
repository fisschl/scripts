import { sxzz } from "@sxzz/eslint-config";
import oxlint from "eslint-plugin-oxlint";

export default sxzz({}, [
  ...oxlint.configs["flat/recommended"],
  {
    rules: {
      "import/no-duplicates": "off",
      "vue/require-default-prop": "off",
      "vue/multi-word-component-names": "off",
      "vue/component-name-in-template-casing": [
        "error",
        "PascalCase",
        { registeredComponentsOnly: false },
      ],
    },
  },
  {
    ignores: ["**/dist/**", "**/assets/**", "**/*.js", "**/*.d.ts", "**/*.mjs"],
  },
]);
