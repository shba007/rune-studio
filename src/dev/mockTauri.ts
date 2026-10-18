import { mockIPC } from '@tauri-apps/api/mocks'

/**
 * Development only: lets the UI render in a plain browser (design review, screenshots) by
 * answering the Tauri commands it calls at startup. Never bundled into a production build.
 */
const monitor = () => ({
  id: 1,
  x: 0,
  y: 0,
  width: 1920,
  height: 1080,
  scaleFactor: 1,
  primary: true,
})

/** A small gradient standing in for a real screenshot. */
function placeholderThumbnail(): string {
  const canvas = document.createElement('canvas')
  canvas.width = 320
  canvas.height = 180
  const ctx = canvas.getContext('2d')
  if (!ctx) return ''
  const gradient = ctx.createLinearGradient(0, 0, 320, 180)
  gradient.addColorStop(0, '#2a2a2a')
  gradient.addColorStop(1, '#6a6a6a')
  ctx.fillStyle = gradient
  ctx.fillRect(0, 0, 320, 180)
  return canvas.toDataURL('image/jpeg', 0.7)
}

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
            externalAgent: false,
          }
        case 'bridge_info':
          return {
            available: true,
            url: 'http://127.0.0.1:47600/mcp',
            token: 'demo',
            command:
              'claude mcp add --transport http rune-studio http://127.0.0.1:47600/mcp --header "Authorization: Bearer demo"',
          }
        case 'get_control_label':
          return null
        case 'check_backend':
          return { reachable: true, authOk: true, missingModels: [], latencyMs: 12, error: null }
        case 'has_api_key':
          return true
        case 'debug_capture':
          return {
            monitor: monitor(),
            width: 1280,
            height: 720,
            bytes: 110_592,
            millis: 71,
            thumbnail: placeholderThumbnail(),
          }
        case 'debug_click':
          return { monitor: monitor(), pixelX: 960, pixelY: 540, cursorX: 960, cursorY: 540 }
        default:
          return undefined
      }
    },
    { shouldMockEvents: true },
  )
}