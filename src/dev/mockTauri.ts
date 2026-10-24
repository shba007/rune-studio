import { mockIPC } from '@tauri-apps/api/mocks'

import {
  drawnShot,
  flightCalls,
  flightEvents,
  flightHeader,
  flightRuns,
  flightSteps,
} from './flightFixture'

const monitor = () => ({
  id: 1,
  x: 0,
  y: 0,
  width: 1920,
  height: 1080,
  scaleFactor: 1,
  primary: true,
})

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

function analysisFixture() {
  return {
    status: 'done',
    id: '120000-1-analysis',
    question: 'What is on this screen, and what could the user do next?',
    answer:
      'A settings dialog with two text fields, a notes area and two buttons. The user can fill in the fields and confirm with the red Delete button or cancel.',
    findings: [
      {
        n: 1,
        label: 'Title bar',
        kind: 'region',
        box: [125, 111, 875, 178],
        description: 'The dialog is called Settings.',
        evidence: 'dark bar across the top of the window',
      },
      {
        n: 2,
        label: 'Name field',
        kind: 'control',
        box: [156, 250, 563, 306],
        description: 'A text field for a name.',
        evidence: 'light grey input with an empty value',
      },
      {
        n: 3,
        label: 'Delete',
        kind: 'issue',
        box: [703, 778, 844, 845],
        description: 'A red button that probably deletes the entry.',
        evidence: 'red fill, bottom right of the dialog',
      },
      {
        n: 4,
        label: 'Footer note',
        kind: 'text',
        box: null,
        description: 'Small print that could not be placed.',
        evidence: 'mentioned at the bottom, too small to locate',
      },
    ],
    model: 'gemma-4-12b-it',
    latencyMs: 4200,
    error: null,
    createdUtc: new Date().toISOString(),
    monitor: { id: 1, x: 0, y: 0, width: 1920, height: 1080, scaleFactor: 1, primary: true },
    stale: false,
    boxesRejected: 0,
    selected: null,
  }
}

export function installTauriMock() {
  const videoParam = new URLSearchParams(location.search).get('video')
  const withVideo = videoParam !== null
  const agentParam = new URLSearchParams(location.search).get('agent')
  mockIPC(
    (command, args) => {
      const given = (args ?? {}) as Record<string, unknown>
      switch (command) {
        case 'external_agent':
          return agentParam
            ? {
                agent: agentParam,
                id: agentParam.toLowerCase(),
                known: false,
                verified: false,
                declared: agentParam,
              }
            : null
        case 'flight_runs':
          return flightRuns
        case 'flight_run':
          return flightRuns.some((run) => run.id === given.id)
            ? flightHeader(String(given.id), withVideo)
            : null
        case 'flight_steps':
          return flightSteps
        case 'flight_events':
          return flightEvents
        case 'flight_calls':
          return flightCalls.filter((item) => given.step == null || item.step === given.step)
        case 'flight_call_body':
          return [
            '{"messages":[{"role":"user","content":"typed Buy milk and call Sam"}]}',
            '{"choices":[{"message":{"reasoning_content":"The menu is open, so type the name."}}]}',
          ]
        case 'flight_file': {
          const file = String(given.relative)
          if (file.endsWith('.mp4'))
            return videoParam === 'webm' ? '/dev/flight.webm' : '/dev/flight.mp4'
          const n = Number(/(\d+)\.jpg$/.exec(file)?.[1] ?? 0)
          return n > 0 ? drawnShot(n) : file
        }
        case 'flight_reveal':
        case 'open_flight':
          return null
        case 'get_settings':
          return {
            baseUrl: 'http://127.0.0.1:8080',
            model: 'gemma-4-12b-it',
            models: {
              agent: 'gemma-4-12b-it',
              verifier: '',
              analysis: '',
              voice: '',
              decision: 'liquid-d1-3b',
              advisor: '',
            },
            maxSteps: 30,
            toggleHotkey: 'Alt+Q',
            stopHotkey: 'Escape',
            externalAgent: false,
            externalAgentSince: 0,
            showThinking: false,
            askEveryAction: false,
            developerMode: false,
            fastChecks: false,
            decisionModel: 'liquid-d1-3b',
            decisionUrl: '',
            archiveKeepDays: 0,
            verifyMode: 'background',
            demoMode: true,
            recordSessionVideo: 'off',
            offerReview: 'always',
            reflectionEnabled: true,
            idleMinutes: 10,
          }
        case 'bridge_info':
          return {
            available: true,
            url: 'http://127.0.0.1:47600/mcp',
            command:
              'claude mcp add --scope user rune-studio -- "C:/Rune Studio/rune-studio-cli.exe" mcp --agent claude',
          }
        case 'get_control_label':
          return null
        case 'check_backend':
          return {
            reachable: true,
            authOk: true,
            missingModels: [],
            modelCapabilities: [],
            latencyMs: 12,
            error: null,
          }
        case 'analysis_state':
          return analysisFixture()
        case 'screen_insight_targets':
          return [
            { target: { kind: 'screen', id: 1 }, label: 'Screen 1 — 1920×1080 (primary)' },
            { target: { kind: 'window', id: 2 }, label: 'Window — Project notes' },
          ]
        case 'overlay_info':
          return { mode: 'none', primary: true, width: 1920, height: 1080, scaleFactor: 1 }
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
        case 'plugin:app|version':
          return '0.0.3'
        case 'plugin:updater|check':
          return new Promise((resolve) =>
            setTimeout(
              () =>
                resolve({
                  rid: 1,
                  currentVersion: '0.0.3',
                  version: '0.0.4',
                  date: null,
                  body: 'Preview update',
                  rawJson: {},
                }),
              800,
            ),
          )
        case 'plugin:updater|download_and_install':
          return undefined
        default:
          return undefined
      }
    },
    { shouldMockEvents: true },
  )
}