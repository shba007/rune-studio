import { fileURLToPath, URL } from 'node:url'

import { defineConfig, lazyPlugins } from 'vite-plus'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'
import Icons from 'unplugin-icons/vite'
import { FileSystemIconLoader } from 'unplugin-icons/loaders'
import Unfonts from 'unplugin-fonts/vite'

// https://vite.dev/config/
export default defineConfig({
  staged: {
    '*': 'vp check --fix',
  },
  fmt: {
    semi: false,
    singleQuote: true,
  },
  lint: {
    plugins: ['typescript', 'unicorn', 'oxc', 'vue', 'vitest'],
    categories: {
      correctness: 'error',
    },
    env: {
      browser: true,
      builtin: true,
    },
    ignorePatterns: ['**/dist/**', '**/dist-ssr/**', '**/coverage/**'],
    rules: {
      'no-array-constructor': 'error',
      'typescript/ban-ts-comment': 'error',
      'typescript/no-empty-object-type': 'error',
      'typescript/no-explicit-any': 'error',
      'typescript/no-namespace': 'error',
      'typescript/no-require-imports': 'error',
      'typescript/no-unnecessary-type-constraint': 'error',
      'typescript/no-unsafe-function-type': 'error',
      'vite-plus/prefer-vite-plus-imports': 'error',
      // Icons are requested by meaning through <AppIcon>; only src/icons may name an icon set.
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            {
              group: ['~icons/**'],
              message:
                'Do not import icons directly. Add the icon to src/icons/index.ts and use <AppIcon name="..." />.',
            },
          ],
        },
      ],
    },
    overrides: [
      {
        files: ['src/icons/**'],
        rules: { 'no-restricted-imports': 'off' },
      },
      {
        files: ['**/*.ts', '**/*.tsx', '**/*.mts', '**/*.cts', '**/*.vue'],
        rules: {
          'constructor-super': 'off',
          'getter-return': 'off',
          'no-class-assign': 'off',
          'no-const-assign': 'off',
          'no-dupe-class-members': 'off',
          'no-dupe-keys': 'off',
          'no-func-assign': 'off',
          'no-import-assign': 'off',
          'no-new-native-nonconstructor': 'off',
          'no-obj-calls': 'off',
          'no-redeclare': 'off',
          'no-setter-return': 'off',
          'no-this-before-super': 'off',
          'no-undef': 'off',
          'no-unreachable': 'off',
          'no-unsafe-negation': 'off',
          'no-var': 'error',
          'no-with': 'off',
          'prefer-const': 'error',
          'prefer-rest-params': 'error',
          'prefer-spread': 'error',
        },
      },
      {
        files: ['src/**/__tests__/*'],
        rules: {
          'vitest/expect-expect': 'error',
          'vitest/no-commented-out-tests': 'error',
          'vitest/no-conditional-expect': 'error',
          'vitest/no-disabled-tests': 'warn',
          'vitest/no-focused-tests': 'error',
          'vitest/no-identical-title': 'error',
          'vitest/no-import-node-test': 'error',
          'vitest/no-interpolation-in-snapshots': 'error',
          'vitest/no-mocks-import': 'error',
          'vitest/no-standalone-expect': 'error',
          'vitest/no-unneeded-async-expect-function': 'error',
          'vitest/prefer-called-exactly-once-with': 'error',
          'vitest/require-local-test-context-for-concurrent-snapshots': 'error',
          'vitest/valid-describe-callback': 'error',
          'vitest/valid-expect': 'error',
          'vitest/valid-expect-in-promise': 'error',
          'vitest/valid-title': 'error',
        },
      },
    ],
    options: {
      typeAware: true,
      typeCheck: true,
    },
    jsPlugins: [
      {
        name: 'vite-plus',
        specifier: 'vite-plus/oxlint-plugin',
      },
    ],
  },
  plugins: lazyPlugins(() => [
    vue(),
    tailwindcss(),
    // Ready-made icons come from Phosphor Fill ('~icons/ph/<name>-fill') and hand-made SVGs from
    // src/assets/icons ('~icons/custom/<file-name>'). Both are imported only in src/icons/index.ts.
    Icons({
      compiler: 'vue3',
      customCollections: { custom: FileSystemIconLoader('./src/assets/icons') },
    }),
    // Oxanium comes from its @fontsource-variable package, so it is bundled, never fetched.
    Unfonts({
      fontsource: { families: [{ name: 'Oxanium', variable: { wght: true }, subset: 'latin' }] },
    }),
  ]),
  server: {
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
})