import js from '@eslint/js';
import ts from '@typescript-eslint/eslint-plugin';
import vueParser from 'vue-eslint-parser';
import tsParser from '@typescript-eslint/parser';
import vue from 'eslint-plugin-vue';
import prettier from 'eslint-config-prettier';

const browserGlobals = {
  document: 'readonly',
  window: 'readonly',
  console: 'readonly',
  localStorage: 'readonly',
  CustomEvent: 'readonly',
  KeyboardEvent: 'readonly',
  InputEvent: 'readonly',
  HTMLElement: 'readonly',
  HTMLInputElement: 'readonly',
  HTMLSelectElement: 'readonly',
  HTMLAnchorElement: 'readonly',
  navigator: 'readonly',
  MutationObserver: 'readonly',
  ResizeObserver: 'readonly',
  IntersectionObserver: 'readonly',
  MediaQueryList: 'readonly',
  self: 'readonly',
  global: 'readonly',
  fetch: 'readonly',
  setTimeout: 'readonly',
  clearTimeout: 'readonly',
  setInterval: 'readonly',
  clearInterval: 'readonly',
  MouseEvent: 'readonly',
  Node: 'readonly',
  Event: 'readonly',
  Element: 'readonly',
  Blob: 'readonly',
  URL: 'readonly',
  process: 'readonly',
  requestAnimationFrame: 'readonly',
  cancelAnimationFrame: 'readonly',
  performance: 'readonly',
  FrameRequestCallback: 'readonly',
};

const tsRules = {
  ...ts.configs.recommended.rules,
  'vue/multi-word-component-names': 'off',
  'vue/no-unused-vars': 'error',
  '@typescript-eslint/no-unused-vars': 'error',
  '@typescript-eslint/no-explicit-any': 'warn',
  '@typescript-eslint/no-empty-object-type': 'off',
  'no-console': process.env.NODE_ENV === 'production' ? 'warn' : 'off',
  'no-debugger': process.env.NODE_ENV === 'production' ? 'warn' : 'off',
};

export default [
  {
    ignores: [
      'node_modules/',
      'dist/',
      'coverage/',
      '*.min.js',
      'public/',
      'src-tauri/',
      'src-tauri/target/',
      'eslint.config.js',
      'vite.config.ts',
      'scripts/',
    ],
  },
  js.configs.recommended,
  ...vue.configs['flat/recommended'],
  {
    files: ['**/*.{vue,ts}'],
    languageOptions: {
      globals: browserGlobals,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
      },
    },
    plugins: {
      '@typescript-eslint': ts,
    },
    rules: tsRules,
  },
  {
    files: ['**/*.vue'],
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
        parser: tsParser,
      },
    },
  },
  {
    files: ['**/*.ts'],
    languageOptions: {
      parser: tsParser,
    },
  },
  {
    files: ['src/**/*.{js,jsx,ts,tsx,vue}'],
  },
  {
    // Custom rule to disallow Chinese characters except in i18n locale files.
    //
    // Scope: this rule covers JS/TS string & template literals, comments, and
    // (for .vue) template text plus static attribute values via
    // vue-eslint-parser. CSS files are intentionally NOT covered — ESLint's
    // CSS handling is out of scope here; keep Chinese strings in styles out of
    // the codebase or route them through i18n. If CSS coverage is needed, add a
    // stylelint rule (e.g. stylelint-declaration-block) separately.
    files: ['src/**/*.{js,jsx,ts,tsx,vue}'],
    ignores: [
      'src/core/i18n/locales/**',
      'src/components/settings/SettingsPanel.vue',
      'src/components/common/WelcomeScreen.vue',
    ],
    plugins: {
      'custom-i18n': {
        rules: {
          'no-chinese-content': {
            create(context) {
              const chineseRegex = /[\u4e00-\u9fa5]/;
              const sourceCode = context.sourceCode;
              const reportChinese = (node, detail) => {
                context.report({
                  node: node || undefined,
                  loc: node && node.loc ? node.loc : undefined,
                  message:
                    'Chinese characters are not allowed in ' +
                    detail +
                    '. Please use i18n features.',
                });
              };
              // Template-body visitors: Vue template text and static
              // attribute values (handled via vue-eslint-parser). Dynamic
              // bindings (e.g. :title="t('x')") flow through the AST as
              // expressions and are already covered by the script visitors.
              return {
                // Vue template text, e.g. {{ '中文' }} or raw text nodes.
                VText(node) {
                  if (chineseRegex.test(node.value)) {
                    reportChinese(node, 'template text');
                  }
                },
                // Static attribute values: <div title="...">. Skip directives
                // (v-bind/v-if etc.) — those are expressions, not text.
                VAttribute(node) {
                  if (node.directive) return;
                  const value = node.value;
                  if (
                    value &&
                    value.type === 'VLiteral' &&
                    chineseRegex.test(value.value)
                  ) {
                    reportChinese(value, 'static attribute values');
                  }
                },
                Literal(node) {
                  if (
                    typeof node.value === 'string' &&
                    chineseRegex.test(node.value)
                  ) {
                    reportChinese(node, 'string literals');
                  }
                },
                TemplateElement(node) {
                  if (chineseRegex.test(node.value.raw)) {
                    reportChinese(node, 'template literals');
                  }
                },
                Program() {
                  const comments = sourceCode.getAllComments();
                  comments.forEach(comment => {
                    if (chineseRegex.test(comment.value)) {
                      context.report({
                        loc: comment.loc,
                        message:
                          'Chinese characters are not allowed in comments. Please use English.',
                      });
                    }
                  });
                },
              };
            },
          },
        },
      },
    },
    rules: {
      'custom-i18n/no-chinese-content': 'error',
    },
  },
  prettier,
];
