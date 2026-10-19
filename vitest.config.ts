import { fileURLToPath } from 'node:url'
import { mergeConfig, defineConfig, configDefaults } from 'vite-plus'
import viteConfig from './vite.config'

export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: 'jsdom',
      // scripts/ holds the Node test-runner suites (`bun run test:eval`, `test:m15`); vitest cannot load them.
      exclude: [...configDefaults.exclude, 'e2e/**', 'scripts/**'],
      root: fileURLToPath(new URL('./', import.meta.url)),
    },
  }),
)