import { mockIPC } from '@tauri-apps/api/mocks'

/**
 * Development only: lets the UI render in a plain browser (design review, screenshots) by
 * answering the Tauri commands it calls at startup. Never bundled into a production build.
 */
export function installTauriMock() {
  mockIPC(
    (command) => {
      switch (command) {
        case 'get_settings':
          return {
            baseUrl: 'http://127.0.0.1:8080',
            visionModel: 'qwen3.8-27b',
            asrModel: 'parakeet-tdt-0.6b-v3',
            maxSteps: 30,
            toggleHotkey: 'Alt+Q',
            stopHotkey: 'Ctrl+Alt+Escape',
          }
        case 'check_backend':
          return { reachable: true, authOk: true, missingModels: [], latencyMs: 12, error: null }
        case 'has_api_key':
          return true
        default:
          return undefined
      }
    },
    { shouldMockEvents: true },
  )
}